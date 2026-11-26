#!/usr/bin/env python3
"""Compare binary Compose callback lowering against the pinned Kotlin plugin."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/compose_callbacks'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--compiler-source', type=Path, default=ROOT)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--no-report', action='store_true')
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-compose-callbacks'
    work = Path(tempfile.mkdtemp(prefix=ident + '-'))
    logs = work / 'logs' if args.no_report else ROOT / 'reports/correctness' / ident
    logs.mkdir(parents=True)
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    runtime = ROOT / 'references/compose/runtime.jar'
    annotations = ROOT / 'references/compose/annotations.jar'
    plugin = ROOT / 'references/kotlinc/lib/compose-compiler-plugin.jar'
    compiler_jar = ROOT / 'references/kotlinc/lib/kotlin-compiler.jar'
    pin = json.loads((ROOT / 'references/compose-runtime.json').read_text())
    assert runtime.is_file(), 'Run python3 tools/setup_compose_reference.py first'
    assert sha(runtime) == pin['classes_sha256'], 'Compose runtime checksum mismatch'
    assert annotations.is_file() and sha(annotations) == pin['annotations']['classes_sha256'], 'Run python3 tools/setup_compose_reference.py for pinned stability annotations'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'

    def rel(path):
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)

    inputs = {str(p.relative_to(args.compiler_source)): p.read_bytes() for p in sorted(args.compiler_source.glob('src/**/*.rs'))}
    for path in [*sorted(FIXTURES.glob('*')), ROOT / 'tests/compose_callbacks.rs', Path(__file__), ROOT / 'tools/setup_compose_reference.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/compose-runtime.json']:
        inputs[rel(path)] = path.read_bytes()
    with zipfile.ZipFile(logs / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path, data in inputs.items():
            archive.writestr(path, data)
    frozen = {path: hashlib.sha256(data).hexdigest() for path, data in inputs.items()}
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), compose_runtime=pin, inputs=dict(files_sha256=frozen, bundle=rel(logs / 'inputs.zip'), bundle_sha256=sha(logs / 'inputs.zip'), stdlib_sha256=sha(standard), compose_plugin_sha256=sha(plugin), jdk_release_sha256=sha(jdk.parent / 'release')), commands=[], cases=[], limitations=['Only literal zero-argument Unit Compose callbacks passed to supported noninline, nongeneric binary methods are lowered', 'Source composable declarations and receiver/argument/result/suspend/nullable/generic composable callback shapes remain unsupported; binary composable calls accept only supported static noninline nongeneric Unit file-facade methods and straight-line groups; nested ordinary Unit functions support up to one parameter and stable scalar/String or annotation-free Kotlin interface captures; annotated or unknown class stability, mutable cells and conditional memoization remain explicit unsupported boundaries', 'Runtime protocol is pinned to AndroidX Compose Runtime 1.11.0 and the Kotlin 2.3.21 bundled plugin; keys and generated class names are compiler-specific', 'Composer and RecomposeScope are controlled proxies; actual pinned composableLambdaInstance wrappers and generated callback bodies execute, without a real Composition/Recomposer or Android execution claim'], reproduction_commands=[['python3', 'tools/setup_compose_reference.py'], ['python3', 'tools/check_compose_callbacks.py', '--candidate', str(args.candidate)]])

    report['reproduction_commands'][-1].extend(['--compiler-source', str(args.compiler_source)])
    if args.baseline:
        report['reproduction_commands'][-1].extend(['--baseline', str(args.baseline)])

    def run(label, command, accepted=(0,)):
        command = list(map(str, command))
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, timeout=180)
        path = logs / f'{len(report["commands"]):03d}.txt'
        path.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=rel(path)))
        assert result.returncode in accepted, label + '\n' + result.stdout + result.stderr
        return result

    def case(name, condition, **details):
        report['cases'].append(dict(id=name, outcome='pass' if condition else 'correctness_failure', **details))
        assert condition, name

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    options = ['-no-stdlib', '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-Xlambdas=class', '-Xplugin=' + str(plugin), '-P', 'plugin:androidx.compose.compiler.plugins.kotlin:sourceInformation=false']
    try:
        dependencies = work / 'dependency'
        run('compile Java lexical superclass', [jdk / 'javac', '-d', dependencies, FIXTURES / 'LexicalBase.java'])
        run('compile hidden-argument recorder', [jdk / 'javac', '-cp', cp(runtime, standard), '-d', dependencies, FIXTURES / 'Capture.java'])
        library_sources = [FIXTURES / name for name in ['Library.kt', 'Lookalike.kt', 'LexicalLibrary.kt', 'Binary.kt', 'Memo.kt']]
        run('compile transformed binary dependency', [kotlin, *library_sources, *options, '-cp', cp(runtime, annotations, standard), '-d', dependencies])
        instrumenter = work / 'instrumenter'
        run('compile descriptor-preserving entry probe', [jdk / 'javac', '-cp', compiler_jar, '-d', instrumenter, FIXTURES / 'Instrument.java'])
        run('instrument pinned binary method arguments', [jdk / 'java', '-cp', cp(instrumenter, compiler_jar), 'Instrument', dependencies / 'composeprobe/BinaryKt.class'])
        run('probe pinned memo callee ABI', [jdk / 'java', '-cp', cp(instrumenter, compiler_jar), 'Instrument', dependencies / 'composeprobe/MemoKt.class'])
        dependency_cp = cp(dependencies, runtime, annotations, standard)
        if args.baseline:
            baseline = work / 'baseline-frozen'
            shutil.copy2(args.baseline.resolve(), baseline)
            report['baseline_compiler'] = dict(binary=rel(baseline), sha256=sha(baseline))
            result = run('controlled lexical-dispatch baseline', [baseline, '--diagnostic-format', 'json', FIXTURES / 'Consumer.kt', '-cp', dependency_cp, '-d', work / 'baseline'], (1,))
            diagnostic = json.loads(result.stderr or result.stdout)
            case('lexical-dispatch-baseline', diagnostic['message'] == 'Unresolved member or imported extension: apply', diagnostic=diagnostic)
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, output in outputs.items():
            command = [kotlin, *options] if provider == 'official' else [candidate]
            run('compile callback consumer ' + provider, [*command, FIXTURES / 'Consumer.kt', '-cp', dependency_cp, '-d', output])
            case('compile/' + provider, True)
        callers = work / 'callers'
        run('fixed Java caller against official', [jdk / 'javac', '-cp', cp(outputs['official'], dependency_cp), '-d', callers, FIXTURES / 'ProtocolCheck.java', FIXTURES / 'BinaryProtocol.java', FIXTURES / 'MemoProtocol.java'])
        signatures = []
        for provider, output in outputs.items():
            result = run('verified runtime protocol ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, output, dependency_cp), 'composeprobe.ProtocolCheck'])
            case('runtime/' + provider, result.stdout == 'memo-protocol-ok\ncompose-protocol-ok:128\n', protocol_scenarios=128, additional_assertions=['interface and top-level hosts', 'mutable captures', 'nullable reference and wide captures', 'local return', 'disabled tracing', 'abrupt exception', 'lexical dispatch and initialization', 'nested callback remembered identity and invalidation', 'primitive and String value comparisons versus unannotated interface identity', 'nullable host and wide captures', 'captureless literal and callable reference caching', 'deferred runtime side effects and source markers', 'exact binary callee flags and defaults', 'nearer and nested receiver precedence', 'labeled and nearest this'])
            signatures.append(run('public JVM signatures ' + provider, [jdk / 'javap', '-public', '-s', '-cp', output, 'composeprobe.ConsumerKt']).stdout)
        case('public-JVM-signatures', signatures[0] == signatures[1])
        boundaries = json.loads((FIXTURES / 'Rejections.json').read_text())
        for item in boundaries:
            source = work / (item['id'] + '.kt')
            source.write_text(item['source'].replace('fun unsupported', 'fun boundary_' + item['id'].replace('-', '_')))
        oracle_sources = [work / (item['id'] + '.kt') for item in boundaries if item['oracle'] == 'accept']
        run('oracle valid unsupported boundaries', [kotlin, *oracle_sources, *options, '-cp', dependency_cp, '-d', work / 'boundary-official'])
        for item in boundaries:
            source = work / (item['id'] + '.kt')
            if item['oracle'] != 'accept':
                run('oracle invalid boundary/' + item['id'], [kotlin, source, *options, '-cp', dependency_cp, '-d', work / ('boundary-official-' + item['id'])], (1,))
            case('oracle-boundary/' + item['id'], True)
            result = run('candidate boundary/' + item['id'], [candidate, '--diagnostic-format', 'json', source, '-cp', dependency_cp, '-d', work / ('boundary-candidate-' + item['id'])], (1,))
            diagnostic = json.loads(result.stderr or result.stdout)
            case('candidate-boundary/' + item['id'], not item.get('code') or diagnostic['code'] == item['code'], diagnostic=diagnostic)
        source = work / 'missing-runtime.kt'
        source.write_text('fun unsupported(s: composeprobe.Registry) { s.accept {} }')
        result = run('missing runtime ABI', [candidate, '--diagnostic-format', 'json', source, '-cp', cp(dependencies, standard), '-d', work / 'missing-runtime'], (1,))
        diagnostic = json.loads(result.stderr or result.stdout)
        case('missing-runtime', diagnostic['code'] == 'COMPOSE_RUNTIME_ABI_UNSUPPORTED', diagnostic=diagnostic)
        untransformed = work / 'untransformed'
        plain_options = ['-no-stdlib', '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
        run('untransformed binary dependency', [kotlin, *library_sources, *plain_options, '-cp', cp(runtime, annotations, standard), '-d', untransformed])
        result = run('reject untransformed callback ABI', [candidate, '--diagnostic-format', 'json', source, '-cp', cp(untransformed, runtime, standard), '-d', work / 'untransformed-candidate'], (1,))
        diagnostic = json.loads(result.stderr or result.stdout)
        case('untransformed-callback', diagnostic['code'] == 'LIBRARY_DECLARATION_UNSUPPORTED', diagnostic=diagnostic)
        fake_source = work / 'Composer.java'
        fake_source.write_text('package androidx.compose.runtime; public interface Composer { boolean shouldExecute(boolean changed, int flags); }')
        fake_runtime = work / 'fake-runtime'
        run('incompatible runtime class', [jdk / 'javac', '-d', fake_runtime, fake_source])
        result = run('reject incompatible runtime ABI', [candidate, '--diagnostic-format', 'json', source, '-cp', cp(dependencies, fake_runtime, runtime, standard), '-d', work / 'fake-runtime-candidate'], (1,))
        diagnostic = json.loads(result.stderr or result.stdout)
        case('incompatible-runtime', diagnostic['code'] == 'COMPOSE_RUNTIME_ABI_UNSUPPORTED', diagnostic=diagnostic)
        case('frozen-fixtures', all(sha(path) == frozen[rel(path)] for path in [*FIXTURES.glob('*'), ROOT / 'tests/compose_callbacks.rs', Path(__file__)]))
        report['status'] = 'passed'
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
        raise
    finally:
        report['counts'] = dict(Counter(item['outcome'] for item in report['cases']))
        path = logs.with_suffix('.json')
        path.write_text(json.dumps(report, indent=2) + '\n')
        print(path)
        if not args.no_report and not args.skip_html:
            subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    if args.no_report:
        shutil.rmtree(work)


if __name__ == '__main__':
    main()
