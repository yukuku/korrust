#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

from upstream_jvm import normalize_signature

DEFAULT_ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=DEFAULT_ROOT / 'target/release/korrust')
    parser.add_argument('--source-root', type=Path, default=DEFAULT_ROOT)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    root = args.source_root.resolve()
    fixtures = root / 'tests/next10_shorts'
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-next10-shorts'
    directory = (args.output or root / 'target/checks' / stamp).resolve()
    directory.mkdir(parents=True, exist_ok=False)
    evidence = root / 'reports/correctness' / stamp
    evidence.mkdir(parents=True, exist_ok=False)
    (evidence / 'commands').mkdir()
    report_path = evidence.with_suffix('.json')
    official = root / 'references/kotlinc/bin/kotlinc'
    stdlib = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    paths = [*sorted((root / 'src').rglob('*.rs')), *sorted(path for path in fixtures.rglob('*') if path.is_file()), root / 'tests/next10_shorts.rs', root / 'tests/primitive_arrays.rs', root / 'Cargo.toml', root / 'Cargo.lock', root / 'references/compiler.json', root / 'references/sources.json', root / 'references/kotlin/libraries/stdlib/jvm/builtins/Primitives.kt', root / 'references/kotlin/compiler/testData/codegen/box/arrays/forEachShortArray.kt', root / 'references/kotlin-spec/docs/src/md/kotlin.core/builtins.md', Path(__file__).resolve(), root / 'tools/upstream_jvm.py', candidate, stdlib, root / 'references/kotlinc/lib/kotlin-compiler.jar']

    def relative(path):
        try:
            return str(Path(path).resolve().relative_to(root))
        except ValueError:
            return str(Path(path).resolve())

    frozen = {relative(path): sha(path) for path in paths}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in paths:
            if path.is_relative_to(root) and path.suffix in {'.rs', '.kt', '.java', '.py', '.toml', '.lock', '.json', '.md'}:
                archive.writestr(relative(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()), reference_compiler=json.loads((root / 'references/compiler.json').read_text()), frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), commands=[], cases=[], evidence=dict(work_directory=relative(directory), logs=relative(evidence / 'commands')), effective_options=dict(jvm_target='1.8', language_version='2.3', api_version='2.3', module_name='next10_shorts'), scope='Integral arithmetic and ordering promotion across Byte/Short/Int/Long, unary Byte/Short promotion; ShortArray allocation, stable initializer construction, indexing, writes, iteration, vararg/spread, factories, JVM signatures and source/compiled Kotlin metadata', limitations=['Short source types, contextual literals, conversions and boxing predate this milestone and are regression checks, not new feature claims.', 'Integral equality retains Kotlin operand-type restrictions; bitwise/shift operators do not gain implicit narrowing or Short receivers.', 'Effectful initializer-producing expressions retain the existing explicit specification/oracle evaluation-order rejection boundary.', 'Inherited import-resolution boundary: an explicit alias of a default-imported function and its original implicit name do not freely coexist in one source file; positive alias and original-name checks use separate source files.', 'Short annotation elements, const declarations, complete standard library APIs, Android/full-app compatibility and performance are not claimed.'], reproduction_command=['python3', relative(Path(__file__)), '--candidate', relative(args.candidate), '--source-root', str(root)])
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'next10_shorts']

    def save():
        report['counts'] = dict(sorted(Counter(case['outcome'] for case in report['cases']).items()))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, argv):
        argv = list(map(str, argv))
        print(label, flush=True)
        result = subprocess.run(argv, cwd=root, capture_output=True, text=True, timeout=300)
        log = evidence / 'commands' / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(argv) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=argv, exit_code=result.returncode, log=relative(log)))
        save()
        return result

    def require(label, argv):
        result = run(label, argv)
        assert result.returncode == 0, result.stdout + result.stderr
        return result

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def case(ident, outcome='pass', **details):
        report['cases'].append(dict(id=ident, outcome=outcome, **details))
        save()

    def compile(provider, sources, output, dependencies=()):
        return run('Compile ' + provider + '/' + output.name, [official if provider == 'official' else candidate, *sources, *(options if provider == 'official' else ['--module-name', 'next10_shorts', '--diagnostic-format', 'json']), '-cp', cp(*dependencies, stdlib), '-d', output])

    try:
        identity = require('Pinned compiler identity', [official, '-version'])
        assert 'kotlinc-jvm 2.3.21 ' in identity.stdout + identity.stderr
        require('JVM identity', [jdk / 'java', '-version'])
        if args.baseline:
            baseline = directory / 'baseline-frozen'
            shutil.copy2(args.baseline.resolve(), baseline)
            report['baseline_compiler'] = dict(path=relative(baseline), sha256=sha(baseline))
            report['reproduction_command'].extend(['--baseline', relative(args.baseline)])
            for source in sorted((fixtures / 'baseline').glob('*.kt')):
                result = run('Baseline ' + source.stem, [baseline, source, '-cp', stdlib, '--diagnostic-format', 'json', '-d', directory / ('baseline-' + source.stem)])
                assert result.returncode != 0, 'Baseline already supported ' + source.stem
                case('baseline/' + source.stem, outcome='baseline_unsupported', diagnostic=result.stderr)
        helpers = directory / 'helpers'
        require('Compile Java effect helper', [jdk / 'javac', '-d', helpers, fixtures / 'Effects.java'])
        library = directory / 'library'
        actual = compile('official', [fixtures / 'Library.kt'], library)
        assert actual.returncode == 0, actual.stderr
        case('dependency/official')
        outputs = {}
        for provider in ['official', 'candidate']:
            output = directory / provider
            actual = compile(provider, [fixtures / 'Values.kt', fixtures / 'AliasedFactory.kt'], output, [helpers, library])
            assert actual.returncode == 0, actual.stderr
            outputs[provider] = output
            case('compile/' + provider)
        callers = directory / 'callers'
        require('Compile fixed Java caller against official output', [jdk / 'javac', '-cp', cp(outputs['official'], library, helpers, stdlib), '-d', callers, fixtures / 'Calls.java'])
        caller_hashes = {relative(path): sha(path) for path in callers.rglob('*.class')}
        report['official_compiled_java_callers_sha256'] = caller_hashes
        runtime_results = []
        for provider, output in outputs.items():
            actual = require('Verified Java boundaries ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, output, library, helpers, stdlib), 'next10.shorts.Calls'])
            assert actual.stdout.startswith('checks=')
            runtime_results.append(actual.stdout)
            assert caller_hashes == {relative(path): sha(path) for path in callers.rglob('*.class')}
            case('runtime/' + provider, independent_assertions=int(actual.stdout.split('=')[1]), fixed_official_java_caller=True, jvm_verification='all')
            consumer = directory / ('consumer-' + provider)
            actual = compile('official', [fixtures / 'Consumer.kt'], consumer, [output, library, helpers])
            assert actual.returncode == 0, actual.stderr
            actual = require('Verified Kotlin metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, library, helpers, stdlib), 'consumer.ConsumerKt'])
            assert actual.stdout == 'OK\n', actual.stdout
            case('metadata/' + provider)
        assert runtime_results[0] == runtime_results[1]
        for owner in ['next10.shorts.ValuesKt', 'next10.shorts.Holder', 'next10.shorts.AliasedFactoryKt']:
            signatures = [normalize_signature(require('JVM signatures ' + provider + '/' + owner, [jdk / 'javap', '-protected', '-s', '-classpath', output, owner]).stdout) for provider, output in outputs.items()]
            assert signatures[0] == signatures[1], str(signatures)
            case('signature/' + owner)
        for source in sorted((fixtures / 'baseline').glob('*.kt')):
            for provider in ['official', 'candidate']:
                actual = compile(provider, [source], directory / (provider + '-new-' + source.stem))
                assert actual.returncode == 0, actual.stderr
            case('newly_supported/' + source.stem)
        for source in sorted((fixtures / 'invalid').glob('*.kt')):
            details = {}
            for provider in ['official', 'candidate']:
                output = directory / ('invalid-' + provider + '-' + source.stem)
                actual = compile(provider, [source], output)
                assert actual.returncode != 0 and not list(output.rglob('*.class')), actual.stderr
                details[provider] = actual.stderr
            case('invalid/' + source.stem, outcome='invalid_kotlin', diagnostics=details)
        for source in sorted((fixtures / 'unsupported').glob('*.kt')):
            actual = compile('official', [source], directory / ('boundary-official-' + source.stem))
            assert actual.returncode == 0, actual.stderr
            output = directory / ('boundary-candidate-' + source.stem)
            actual = compile('candidate', [source], output)
            assert actual.returncode != 0 and not list(output.rglob('*.class'))
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines()]
            assert diagnostics and all(item['category'] == 'unsupported_language' for item in diagnostics), actual.stderr
            expected_code = 'ANNOTATION_BYTE_SHORT_UNSUPPORTED' if source.stem == 'short_annotation' else 'ARRAY_INITIALIZER_EVALUATION_UNSUPPORTED'
            assert diagnostics[0]['code'] == expected_code
            case('boundary/' + source.stem, outcome='unsupported_language', diagnostics=diagnostics)
        upstream = root / 'references/kotlin/compiler/testData/codegen/box/arrays/forEachShortArray.kt'
        upstream_caller = directory / 'Upstream.java'
        upstream_caller.write_text('public final class Upstream { public static void main(String[] args) { if (!ForEachShortArrayKt.box().equals("OK")) throw new AssertionError(); System.out.println("OK"); } }\n')
        upstream_outputs = {}
        for provider in ['official', 'candidate']:
            output = directory / ('upstream-' + provider)
            actual = compile(provider, [upstream], output)
            assert actual.returncode == 0, actual.stderr
            upstream_outputs[provider] = output
        upstream_java = directory / 'upstream-java'
        require('Compile original fixture Java caller against official', [jdk / 'javac', '-cp', cp(upstream_outputs['official'], stdlib), '-d', upstream_java, upstream_caller])
        for provider, output in upstream_outputs.items():
            actual = require('Original pinned fixture ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(upstream_java, output, stdlib), 'Upstream'])
            assert actual.stdout == 'OK\n'
        case('original/forEachShortArray', path=relative(upstream), sha256=sha(upstream), directives=[], unchanged=True)
        assert frozen == {relative(path): sha(path) for path in paths}, 'Inputs changed during verification'
        report['status'] = 'pass'
    except Exception as error:
        case('harness_failure', outcome='correctness_failure', error=str(error))
        report['status'] = 'fail'
    finally:
        save()
        if report['status'] == 'pass':
            feature = dict(schema_version=1, kind='features', created_at=created.isoformat(), status='pass', compiler=report['compiler'], reference_compiler=report['reference_compiler'], reproduction_commands=[report['reproduction_command']], evidence=[relative(report_path)], features=[dict(name='Integral numeric arithmetic and ordering promotion', status='supported', scope='Byte/Short arithmetic and unary results promote to Int; mixed integral arithmetic and comparisons promote to Long when either operand is Long; existing floating operand promotion is regression-tested', restrictions=['No implicit assignment narrowing or new integral equality/bitwise conversions'], evidence_report=relative(report_path), evidence_cases=['runtime/candidate', 'newly_supported/mixed_integral_arithmetic']), dict(name='ShortArray representation and operations', status='supported', scope='Construction, stable initializer lambdas, indexing, signed 16-bit writes/reads, iteration, primitive varargs and spread, ordinary library factories, Java and Kotlin metadata contracts', restrictions=report['limitations'][2:], evidence_report=relative(report_path), evidence_cases=['runtime/candidate', 'metadata/candidate'])])
            (root / 'reports/features' / (stamp + '.json')).write_text(json.dumps(feature, indent=2) + '\n')
        if not args.skip_html:
            subprocess.run([sys.executable, root / 'tools/reports.py'], cwd=root, check=True)
        print(report_path)
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
