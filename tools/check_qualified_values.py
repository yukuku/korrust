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
FIXTURES = ROOT / 'tests/qualified_values'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-qualified-values')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    binary = work / 'korrust-frozen'
    shutil.copy2(args.compiler, binary)
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    kotlinc = ROOT / 'references/kotlinc/bin/kotlinc'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    inputs = {str(path.relative_to(ROOT)): sha(path) for folder in ['src', 'tests/qualified_values'] for path in sorted((ROOT / folder).rglob('*')) if path.is_file()}
    inputs.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock', 'tests/qualified_values.rs', 'tools/check_qualified_values.py']})
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': created.isoformat(), 'status': 'running', 'selection': 'Qualified Java static fields and Kotlin object/companion properties, assignments and updates; qualified calls; lexical, captured, inherited and imported field/property alias shadowing; same-package compiled nested constructors. Fixed official Java caller executed against both outputs with JVM verification. No full-app or performance claim.', 'compiler': {'binary_sha256': sha(binary)}, 'reference_compiler': {'pin': json.loads((ROOT / 'references/compiler.json').read_text())}, 'inputs': {'files_sha256': inputs}, 'dependencies': {str(stdlib): sha(stdlib)}, 'commands': [], 'cases': []}

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
        return run(provider + '/' + output.name, [binary, *sources, '-cp', cp(*dependencies, stdlib), '--module-name', 'qualifiedvalues', '--diagnostic-format', 'json', '-d', output])

    try:
        dependency = work / 'dependency'
        result = run('java-dependency', [jdk / 'javac', '--release', '8', '-d', dependency, *[FIXTURES / name for name in ['State.java', 'Root.java', 'Segment.java', 'Holder.java']]])
        case('java-dependency', result.returncode == 0)
        result = compile('official', [FIXTURES / 'Definitions.kt', FIXTURES / 'Nested.kt'], dependency, [dependency])
        case('kotlin-dependency', result.returncode == 0)
        providers = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, [FIXTURES / name for name in ['Uses.kt', 'Alias.kt', 'PropertyAlias.kt']], output, [dependency])
            case(provider + '/compile', result.returncode == 0)
            if result.returncode == 0:
                providers[provider] = output
        if 'official' not in providers:
            return 1
        caller = work / 'caller'
        result = run('fixed-java-caller', [jdk / 'javac', '--release', '8', '-cp', cp(providers['official'], dependency, stdlib), '-d', caller, FIXTURES / 'Calls.java'])
        case('caller/compile-official', result.returncode == 0)
        report['fixed_java_caller_hashes'] = {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
        for provider, output in providers.items():
            result = run(provider + '/runtime', [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, dependency, stdlib), 'qualifiedcheck.Calls'])
            case(provider + '/runtime', result.returncode == 0 and result.stdout.strip() == 'qualified-values-ok', observed=result.stdout.strip())
            result = run(provider + '/signatures', [jdk / 'javap', '-protected', '-s', '-classpath', cp(output, dependency), 'qualifiedconsumer.UsesKt', 'qualifiedconsumer.MemberShadow', 'qualifiedconsumer.ShadowBase', 'qualifiedconsumer.InheritedShadow', 'qualifiedalias.AliasKt', 'qualifiedpropertyalias.PropertyAliasKt'])
            case(provider + '/signatures-read', result.returncode == 0)
            report[provider + '_signatures'] = result.stdout
        if 'candidate_signatures' in report:
            case('signatures/match', report['official_signatures'] == report['candidate_signatures'])
        for source in sorted(FIXTURES.glob('Invalid*.kt')):
            results = {provider: compile(provider, [source], work / source.stem / provider, [dependency]) for provider in ['official', 'candidate']}
            diagnostics = [json.loads(line) for line in results['candidate'].stderr.splitlines() if line.startswith('{')]
            case(source.stem, all(result.returncode != 0 for result in results.values()) and bool(diagnostics) and not list((work / source.stem / 'candidate').rglob('*.class')), outcome='invalid_rejected', diagnostics=diagnostics)
    except Exception as error:
        case('harness', False, reason=f'{type(error).__name__}: {error}')
    finally:
        report['input_integrity'] = {'status': 'unchanged' if all(sha(ROOT / path) == digest for path, digest in inputs.items()) and sha(stdlib) == report['dependencies'][str(stdlib)] else 'changed'}
        report['counts'] = dict(Counter(row['outcome'] for row in report['cases']))
        report['status'] = 'pass' if report['cases'] and all(row['outcome'] in {'pass', 'invalid_rejected'} for row in report['cases']) and report['input_integrity']['status'] == 'unchanged' else 'failed'
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work / 'report.json', report['status'], report['counts'], flush=True)
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
