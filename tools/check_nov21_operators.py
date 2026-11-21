#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
import subprocess
from pathlib import Path
import zipfile
from upstream_jvm import Commands

ROOT = Path(__file__).resolve().parents[1]

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    root, candidate, work = args.source_root.resolve(), args.candidate.resolve(), args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    report_path = args.report.resolve() if args.report else work / 'report.json'
    evidence = report_path.with_suffix('') if args.report else work
    if evidence != work:
        evidence.mkdir(parents=True, exist_ok=False)
    (evidence / 'commands').mkdir()
    commands = Commands(evidence / 'commands', dict(os.environ))
    fixtures = root / 'tests/nov21_operators'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    lib = root / 'references/kotlinc/lib'
    standard, oracle = lib / 'kotlin-stdlib.jar', root / 'references/kotlinc/bin/kotlinc'
    inputs = sorted([path for directory in ['src', 'tests/nov21_operators'] for path in (root / directory).rglob('*') if path.is_file()] + [root / 'tests/nov21_operators.rs', root / 'tools/check_nov21_operators.py', root / 'tools/upstream_jvm.py', root / 'Cargo.toml', root / 'Cargo.lock'])
    frozen = {str(path): sha(path) for path in [*inputs, candidate, standard, lib / 'kotlin-compiler.jar', root / 'references/compiler.json', root / 'references/sources.json']}
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'cases': [], 'commands': commands.entries,
              'compiler': {'path': str(candidate), 'sha256': sha(candidate)}, 'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(lib / 'kotlin-compiler.jar'), 'stdlib_sha256': sha(standard)},
              'environment': {'JAVA_HOME': os.environ['JAVA_HOME']}, 'input_revision': {'base_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(), 'working_tree': 'frozen source archive'}, 'reference_inputs': {'compiler_manifest': str(root / 'references/compiler.json'), 'source_manifest': str(root / 'references/sources.json')}, 'frozen_input_hashes': frozen,
              'reproduction_commands': [f'python3 tools/check_nov21_operators.py --candidate {candidate} --source-root {root} --output NEW_DIRECTORY'],
              'claims': 'Source/compiled custom inc/dec prefix/postfix updates and compound assignments on saved property/index lvalues. Existing local compound operators are regression coverage, not a new feature.',
              'limitations': ['Prefix updates of property/custom-index targets are explicitly rejected because the pinned compiler rereads the getter after setting, contrary to the specification expansion; see the recorded oracle discrepancy.', 'Nullable update receivers and safe-call update/compound targets remain unsupported.', 'Parameterized inc/dec declarations, including defaulted or vararg parameters, are explicitly unsupported; pinned Kotlin accepts some such declarations.', 'Index expressions keep their standalone inferred types; literal indices requiring contextual narrowing are rejected in agreement with pinned Kotlin.', 'The existing ordinary overload-resolution, generic, inline and primitive compound-operator boundaries apply.', 'Android and full-application compatibility remain unassessed.'],
              'reference_rules': ['references/kotlin-spec/docs/src/md/kotlin.core/expressions.md', 'references/kotlin-spec/docs/src/md/kotlin.core/operators.md']}
    archive = evidence / 'source.zip'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as target:
        for path in inputs:
            target.write(path, path.relative_to(root))
    def relative(path):
        return str(path.relative_to(root)) if path.is_relative_to(root) else str(path)
    report['evidence'] = {'directory': relative(evidence), 'commands': relative(evidence / 'commands'), 'source_archive': {'path': relative(archive), 'sha256': sha(archive)}}
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def run(label, argv):
        result = commands.run(label, argv, timeout=600)
        if result is None or result.returncode:
            raise RuntimeError(label + ': ' + (result.stderr if result else 'timeout'))
        return result.stdout.strip()
    def case(name, **values):
        report['cases'].append({'id': name, 'outcome': 'pass', **values})
    def compile(provider, sources, dest, dependencies):
        argv = [oracle if provider == 'official' else candidate, *sources, '-cp', cp(*dependencies), '-d', dest]
        argv += ['-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'] if provider == 'official' else ['--diagnostic-format', 'json']
        run('Compile ' + provider + '/' + dest.name, argv)
    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Official compiler identity', [oracle, '-version'])
        dependency = work / 'dependency'
        compile('official', [fixtures / 'dependency/Library.kt'], dependency, [standard])
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, dest in outputs.items():
            compile(provider, [fixtures / 'Operators.kt', fixtures / 'Imported.kt'], dest, [dependency, standard])
            case('compile/' + provider)
        java = work / 'java'
        run('Compile fixed Java caller against official output', [jdk / 'javac', '-cp', cp(outputs['official'], dependency, standard), '-d', java, fixtures / 'Check.java'])
        caller_hashes = {str(path): sha(path) for path in java.rglob('*.class')}
        observations = {}
        for provider, dest in outputs.items():
            observations[provider] = run('Run fixed Java caller/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(java, dest, dependency, standard), 'Check'])
            if not observations[provider].startswith('OK '):
                raise RuntimeError(observations[provider])
            case('runtime/' + provider, assertions=int(observations[provider].split()[1]), output=observations[provider])
        if len(set(observations.values())) != 1:
            raise RuntimeError('Runtime observations differ')
        for owner in ['updates.Counter', 'updates.Holder', 'updates.Indexed', 'updates.Base', 'updates.Child', 'updates.OperatorsKt', 'updates.ImportedKt']:
            signatures = {provider: run('Inspect ABI/' + provider + '/' + owner, [jdk / 'javap', '-public', '-s', '-classpath', dest, owner]) for provider, dest in outputs.items()}
            def normalized(value):
                return sorted(line.strip() for line in value.splitlines() if line.strip() and not line.startswith('Compiled from'))
            if normalized(signatures['official']) != normalized(signatures['candidate']):
                raise RuntimeError('Public ABI differs: ' + owner)
            case('abi/' + owner)
        for against, dependency_output in outputs.items():
            consumer = work / ('consumer-against-' + against)
            compile('official', [fixtures / 'Consumer.kt'], consumer, [dependency_output, dependency, standard])
            for provider, dest in outputs.items():
                observation = run('Run Kotlin consumer compiled against ' + against + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, dest, dependency, standard), 'consumer.ConsumerKt'])
                if observation != 'OK':
                    raise RuntimeError('Kotlin consumer: ' + observation)
                case('kotlin_consumer/' + against + '/' + provider, output=observation)
        discrepancy = work / 'prefix-oracle'
        compile('official', [fixtures / 'PrefixOracle.kt'], discrepancy, [standard])
        observed = run('Observe pinned prefix-property getter discrepancy', [jdk / 'java', '-Xverify:all', '-cp', cp(discrepancy, standard), 'discrepancy.PrefixOracleKt'])
        if observed != 'g+sg':
            raise RuntimeError('Pinned prefix discrepancy changed: ' + observed)
        checked = commands.run('Reject prefix-property discrepancy/candidate', [candidate, fixtures / 'PrefixOracle.kt', '-cp', standard, '-d', work / 'prefix-candidate', '--diagnostic-format', 'json'], timeout=600)
        if checked is None or checked.returncode == 0 or 'PREFIX_UPDATE_ORACLE_DISCREPANCY' not in checked.stderr:
            raise RuntimeError('Prefix discrepancy was not rejected')
        report['cases'].append({'id': 'prefix_property_oracle_discrepancy', 'outcome': 'unsupported_language', 'official_trace': observed, 'specification_expansion_trace': 'g+s', 'rule': 'expressions.md:814', 'diagnostic': checked.stderr.strip()})
        for boundary in json.loads((fixtures / 'boundaries/cases.json').read_text()):
            name, valid = boundary['name'], boundary['official_accepts']
            path = fixtures / 'boundaries' / (name + '.kt')
            official = commands.run('Boundary official/' + name, [oracle, path, '-cp', standard, '-d', work / ('official-' + name)], timeout=600)
            checked = commands.run('Boundary candidate/' + name, [candidate, path, '-cp', standard, '-d', work / ('candidate-' + name), '--diagnostic-format', 'json'], timeout=600)
            if official is None or (official.returncode == 0) != valid:
                raise RuntimeError(name + ': unexpected official classification ' + (official.stderr if official else 'timeout'))
            if checked is None or checked.returncode == 0 or boundary['code'] not in checked.stderr:
                raise RuntimeError(name + ': wrong candidate rejection ' + (checked.stderr if checked else 'timeout'))
            report['cases'].append({'id': 'boundary/' + name, 'outcome': 'unsupported_language' if valid else 'invalid_kotlin', 'official_accepts': valid, 'source_sha256': sha(path), 'diagnostic': checked.stderr.strip()})
        if caller_hashes != {str(path): sha(path) for path in java.rglob('*.class')}:
            raise RuntimeError('Fixed Java caller changed')
        case('fixed_caller_integrity', class_hashes=caller_hashes)
        report['status'] = 'complete'
    except Exception as error:
        report['status'], report['error'] = 'failed', str(error)
    changed = [path for path, value in frozen.items() if not Path(path).is_file() or sha(path) != value]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    if changed:
        report['status'] = 'invalidated'
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'counts': report['counts'], 'error': report.get('error'), 'report': str(report_path)}))
    return 0 if report['status'] == 'complete' else 1

if __name__ == '__main__':
    try:
        raise SystemExit(main())
    finally:
        subprocess.run(['python3', str(ROOT / 'tools/reports.py')], cwd=ROOT, check=False)
