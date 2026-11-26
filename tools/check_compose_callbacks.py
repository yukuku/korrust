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
    parser.add_argument('--no-report', action='store_true')
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
    plugin = ROOT / 'references/kotlinc/lib/compose-compiler-plugin.jar'
    pin = json.loads((ROOT / 'references/compose-runtime.json').read_text())
    assert runtime.is_file(), 'Run python3 tools/setup_compose_reference.py first'
    assert sha(runtime) == pin['classes_sha256'], 'Compose runtime checksum mismatch'
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
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), compose_runtime=pin, inputs=dict(files_sha256=frozen, bundle=rel(logs / 'inputs.zip'), bundle_sha256=sha(logs / 'inputs.zip'), stdlib_sha256=sha(standard), compose_plugin_sha256=sha(plugin), jdk_release_sha256=sha(jdk.parent / 'release')), commands=[], cases=[], limitations=['Only literal zero-argument Unit Compose callbacks passed to supported noninline, nongeneric binary methods are lowered', 'Source composable declarations, hidden Composer calls inside the body, nested composable creation, receiver/argument/result/suspend/nullable/generic callback shapes remain explicitly unsupported', 'Runtime protocol is pinned to AndroidX Compose Runtime 1.11.0 and the Kotlin 2.3.21 bundled plugin; keys and generated class names are compiler-specific', 'No Android execution, full app compatibility, or performance claim'], reproduction_commands=[['python3', 'tools/setup_compose_reference.py'], ['python3', 'tools/check_compose_callbacks.py', '--candidate', str(args.candidate)]])

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
        library_sources = [FIXTURES / 'Library.kt', FIXTURES / 'Lookalike.kt']
        run('compile transformed binary dependency', [kotlin, *library_sources, *options, '-cp', cp(runtime, standard), '-d', dependencies])
        dependency_cp = cp(dependencies, runtime, standard)
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, output in outputs.items():
            command = [kotlin, *options] if provider == 'official' else [candidate]
            run('compile callback consumer ' + provider, [*command, FIXTURES / 'Consumer.kt', '-cp', dependency_cp, '-d', output])
            case('compile/' + provider, True)
        callers = work / 'callers'
        run('fixed Java caller against official', [jdk / 'javac', '-cp', cp(outputs['official'], dependency_cp), '-d', callers, FIXTURES / 'ProtocolCheck.java'])
        signatures = []
        for provider, output in outputs.items():
            result = run('verified runtime protocol ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, output, dependency_cp), 'composeprobe.ProtocolCheck'])
            case('runtime/' + provider, result.stdout == 'compose-protocol-ok:128\n', protocol_scenarios=128, additional_assertions=['interface and top-level hosts', 'mutable captures', 'nullable reference and wide captures', 'local return', 'disabled tracing', 'abrupt exception'])
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
        run('untransformed binary dependency', [kotlin, *library_sources, *plain_options, '-cp', cp(runtime, standard), '-d', untransformed])
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
        if not args.no_report:
            subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    if args.no_report:
        shutil.rmtree(work)


if __name__ == '__main__':
    main()
