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
FIXTURES = ROOT / 'tests/reified_reference_arrays'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def signature_contract(output):
    classes = {}
    owner = None
    member = None
    for line in output.splitlines():
        text = line.strip()
        if text.endswith('{'):
            owner = text
            classes[owner] = []
        elif text.startswith('descriptor:'):
            classes[owner].append((member, text))
            member = None
        elif text and text != '}' and not text.startswith('Compiled from'):
            member = text
    return {owner: sorted(members) for owner, members in classes.items()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-reified-reference-arrays')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    binary = work / 'korrust-frozen'
    shutil.copy2(args.compiler, binary)
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    kotlinc = ROOT / 'references/kotlinc/bin/kotlinc'
    annotations = ROOT / 'references/kotlinc/lib/annotations-13.0.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    inputs = {str(path.relative_to(ROOT)): sha(path) for folder in ['src', 'tests/reified_reference_arrays'] for path in sorted((ROOT / folder).rglob('*')) if path.is_file()}
    inputs.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock', 'tests/reified_reference_arrays.rs', 'tools/check_reified_reference_arrays.py']})
    inputs.update({name: sha(ROOT / name) for name in ['references/kotlin/compiler/backend/src/org/jetbrains/kotlin/codegen/inline/ReifiedTypeInliner.kt', 'references/kotlin-spec/docs/src/md/kotlin.core/rtti.md']})
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': created.isoformat(), 'status': 'running', 'selection': 'Erased parameterized reference classifiers for allocation-only reified uses and metadata-validated builtin array factories; Class<T>, Class<*>, Box<String>, List<String> and nested nonreified Class<U>; runtime component types and array stores; generic is/mixed-operation boundaries. Fixed official Java caller executed against both outputs with JVM verification.', 'compiler': {'binary_sha256': sha(binary)}, 'reference_compiler': {'pin': json.loads((ROOT / 'references/compiler.json').read_text())}, 'inputs': {'files_sha256': inputs}, 'dependencies': {str(stdlib): sha(stdlib), str(annotations): sha(annotations)}, 'commands': [], 'cases': []}

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
            return run(provider + '/' + output.name, [kotlinc, *sources, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-classpath', cp(*dependencies, stdlib, annotations), '-d', output])
        return run(provider + '/' + output.name, [binary, *sources, '-cp', cp(*dependencies, stdlib, annotations), '--module-name', 'reifiedarrays', '--diagnostic-format', 'json', '-d', output])

    try:
        dependency = work / 'dependency'
        result = compile('official', [FIXTURES / 'Library.kt'], dependency, [])
        case('kotlin-dependency', result.returncode == 0)
        providers = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, [FIXTURES / 'Uses.kt'], output, [dependency])
            case(provider + '/compile', result.returncode == 0)
            if result.returncode == 0:
                providers[provider] = output
        if 'official' not in providers:
            return 1
        caller = work / 'caller'
        result = run('fixed-java-caller', [jdk / 'javac', '--release', '8', '-cp', cp(providers['official'], dependency, stdlib, annotations), '-d', caller, FIXTURES / 'Calls.java'])
        case('caller/compile-official', result.returncode == 0)
        report['fixed_java_caller_hashes'] = {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
        for provider, output in providers.items():
            result = run(provider + '/runtime', [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, dependency, stdlib, annotations), 'erasedcheck.Calls'])
            case(provider + '/runtime', result.returncode == 0 and result.stdout.strip() == 'reified-reference-arrays-ok', observed=result.stdout.strip())
            result = run(provider + '/signatures', [jdk / 'javap', '-protected', '-s', '-classpath', cp(output, dependency), 'erasedconsumer.UsesKt'])
            case(provider + '/signatures-read', result.returncode == 0)
            report[provider + '_signatures'] = result.stdout
        if 'candidate_signatures' in report:
            report['signature_contracts'] = {provider: signature_contract(report[provider + '_signatures']) for provider in ['official', 'candidate']}
            case('signatures/match', report['signature_contracts']['official'] == report['signature_contracts']['candidate'])
        for source in sorted([*FIXTURES.glob('Invalid*.kt'), *FIXTURES.glob('Unsupported*.kt')]):
            results = {provider: compile(provider, [source], work / source.stem / provider, [dependency]) for provider in ['official', 'candidate']}
            diagnostics = [json.loads(line) for line in results['candidate'].stderr.splitlines() if line.startswith('{')]
            if source.name.startswith('Unsupported'):
                passed = results['official'].returncode == 0 and results['candidate'].returncode != 0 and bool(diagnostics) and all(d.get('category') == 'unsupported_language' and d.get('code') == 'REIFIED_ARGUMENT_CLASSIFIER_UNSUPPORTED' for d in diagnostics)
                outcome = 'unsupported_language'
            else:
                passed = all(result.returncode != 0 for result in results.values()) and bool(diagnostics)
                outcome = 'invalid_rejected'
            case(source.stem, passed and not list((work / source.stem / 'candidate').rglob('*.class')), outcome=outcome, diagnostics=diagnostics)
    except Exception as error:
        case('harness', False, reason=f'{type(error).__name__}: {error}')
    finally:
        report['input_integrity'] = {'status': 'unchanged' if all(sha(ROOT / path) == digest for path, digest in inputs.items()) and all(sha(path) == digest for path, digest in report['dependencies'].items()) else 'changed'}
        report['counts'] = dict(Counter(row['outcome'] for row in report['cases']))
        report['status'] = 'pass' if report['cases'] and all(row['outcome'] in {'pass', 'invalid_rejected', 'unsupported_language'} for row in report['cases']) and report['input_integrity']['status'] == 'unchanged' else 'failed'
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work / 'report.json', report['status'], report['counts'], flush=True)
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
