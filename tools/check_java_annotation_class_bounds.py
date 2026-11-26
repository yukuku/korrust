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

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/java_annotation_class_bounds'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--junit', type=Path, required=True)
    parser.add_argument('--robolectric', type=Path)
    parser.add_argument('--android', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if bool(args.robolectric) != bool(args.android):
        parser.error('--robolectric and --android must be provided together')
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-java-annotation-class-bounds')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    frozen = work / 'korrust-frozen'
    shutil.copy2(args.compiler, frozen)
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    kotlinc = ROOT / 'references/kotlinc/bin/kotlinc'
    inputs = {str(path.relative_to(ROOT)): sha(path) for folder in ['src', 'tests/java_annotation_class_bounds'] for path in sorted((ROOT / folder).rglob('*')) if path.is_file()}
    inputs[str(Path(__file__).relative_to(ROOT))] = sha(__file__)
    inputs['tests/java_annotation_class_bounds.rs'] = sha(ROOT / 'tests/java_annotation_class_bounds.rs')
    inputs.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock']})
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': created.isoformat(), 'status': 'running', 'selection': 'Binary Java annotation Class upper, invariant, lower and star bounds, arrays and nested values; defaults and real JUnit 4 Test annotation; checked property reads; invalid arguments and unsupported parameterized bounds.', 'compiler': {'binary_sha256': sha(frozen)}, 'reference_compiler': {'pin': json.loads((ROOT / 'references/compiler.json').read_text())}, 'inputs': {'files_sha256': inputs}, 'dependencies': {str(args.junit): sha(args.junit), str(stdlib): sha(stdlib)}, 'commands': [], 'cases': []}
    if args.robolectric:
        report['selection'] += ' Deprecated Java annotation elements and real Robolectric Config with Android Application bound.'
        report['dependencies'].update({str(path): sha(path) for path in [args.robolectric, args.android]})

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def run(label, argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=240)
        index = len(report['commands'])
        stdout = work / f'{index:03d}-stdout.txt'
        stderr = work / f'{index:03d}-stderr.txt'
        stdout.write_text(result.stdout)
        stderr.write_text(result.stderr)
        report['commands'].append({'id': label, 'argv': list(map(str, argv)), 'exit_code': result.returncode, 'stdout': str(stdout.relative_to(ROOT)), 'stderr': str(stderr.relative_to(ROOT))})
        print(label, result.returncode, result.stderr[:160].strip(), flush=True)
        return result

    def case(label, passed, outcome='pass', **extra):
        report['cases'].append({'id': label, 'outcome': outcome if passed else 'correctness_failure', **extra})

    def compile(provider, sources, output, dependencies):
        if provider == 'official':
            return run(provider + '/' + output.name, [kotlinc, *sources, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-classpath', cp(*dependencies, stdlib), '-d', output])
        return run(provider + '/' + output.name, [frozen, *sources, '-cp', cp(*dependencies, stdlib), '--module-name', 'annotationbounds', '--diagnostic-format', 'json', '-d', output])

    try:
        java = work / 'java'
        result = run('java-library', [jdk / 'javac', '--release', '8', '-d', java, *[FIXTURES / name for name in ['Bounds.java', 'Upper.java', 'Nested.java', 'Complex.java']]])
        case('java-library', result.returncode == 0)
        providers = {}
        accepted = [FIXTURES / 'Accepted.kt', FIXTURES / 'Junit.kt']
        dependencies = [java, args.junit]
        callers = [FIXTURES / 'Check.java', FIXTURES / 'JunitCheck.java']
        checks = [('Check', 'OK:bounds'), ('JunitCheck', 'OK:junit')]
        if args.robolectric:
            accepted.append(FIXTURES / 'Robolectric.kt')
            dependencies.extend([args.robolectric, args.android])
            callers.append(FIXTURES / 'RobolectricCheck.java')
            checks.append(('RobolectricCheck', 'OK:robolectric'))
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, accepted, output, dependencies)
            case(provider + '/compile', result.returncode == 0)
            if result.returncode == 0:
                providers[provider] = output
        if 'official' not in providers:
            return 1
        caller = work / 'caller'
        result = run('fixed-java-callers', [jdk / 'javac', '--release', '8', '-cp', cp(providers['official'], *dependencies, stdlib), '-d', caller, *callers])
        case('callers/compile-official', result.returncode == 0)
        report['fixed_java_caller_hashes'] = {str(p.relative_to(caller)): sha(p) for p in caller.rglob('*.class')}
        for provider, output in providers.items():
            for name, expected in checks:
                result = run(provider + '/' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, *dependencies, stdlib), 'annotationboundscheck.' + name])
                case(provider + '/' + name, result.returncode == 0 and result.stdout.strip() == expected, observed=result.stdout.strip())
            result = run(provider + '/signatures', [jdk / 'javap', '-protected', '-s', '-classpath', cp(output, java), 'annotationboundsuses.AcceptedKt', 'annotationboundsuses.Junit'])
            case(provider + '/signatures-read', result.returncode == 0)
            report[provider + '_signatures'] = result.stdout
        if 'candidate_signatures' in report:
            case('signatures/match', report['candidate_signatures'] == report['official_signatures'])
        for source in sorted(FIXTURES.glob('Wrong*.kt')):
            results = {}
            for provider in ['official', 'candidate']:
                output = work / source.stem / provider
                results[provider] = compile(provider, [source], output, [java, args.junit])
            diagnostics = [json.loads(line) for line in results['candidate'].stderr.splitlines() if line.startswith('{')]
            case(source.stem, all(result.returncode != 0 for result in results.values()) and bool(diagnostics) and all(d.get('category') == 'invalid_kotlin' for d in diagnostics) and not list((work / source.stem / 'candidate').rglob('*.class')), outcome='invalid_rejected', diagnostics=diagnostics)
        results = {}
        for provider in ['official', 'candidate']:
            results[provider] = compile(provider, [FIXTURES / 'Unsupported.kt'], work / 'unsupported' / provider, [java])
        diagnostics = [json.loads(line) for line in results['candidate'].stderr.splitlines() if line.startswith('{')]
        case('parameterized-bound', results['official'].returncode == 0 and results['candidate'].returncode != 0 and bool(diagnostics) and all(d.get('category') == 'unsupported_language' and d.get('code') == 'ANNOTATION_DEFINITION_UNSUPPORTED' for d in diagnostics), outcome='unsupported_language', diagnostics=diagnostics)
    except Exception as error:
        case('harness', False, reason=f'{type(error).__name__}: {error}')
    finally:
        report['input_integrity'] = {'status': 'unchanged' if all(sha(ROOT / path) == digest for path, digest in inputs.items()) and all(sha(path) == digest for path, digest in report['dependencies'].items()) else 'changed'}
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['status'] = 'pass' if report['cases'] and all(case['outcome'] in {'pass', 'invalid_rejected', 'unsupported_language'} for case in report['cases']) and report['input_integrity']['status'] == 'unchanged' else 'failed'
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work / 'report.json', report['status'], report['counts'], flush=True)
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
