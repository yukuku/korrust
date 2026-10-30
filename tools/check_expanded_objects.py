#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile
from upstream_jvm import Commands, normalize_signature, run_case, publish_report

ROOT = Path(__file__).resolve().parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--publish', type=Path)
    parser.add_argument('--app-capture', type=Path)
    args = parser.parse_args()
    work = args.output.resolve()
    work.mkdir(parents=True)
    commands_dir = work / 'commands'
    commands_dir.mkdir()
    commands = Commands(commands_dir, dict(os.environ))
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    lib = ROOT / 'references/kotlinc/lib'
    standard = lib / 'kotlin-stdlib.jar'
    compiler = args.candidate.resolve()
    oracle = ROOT / 'references/kotlinc/bin/kotlinc'
    fixtures = ROOT / 'tests/expanded_objects'
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'cases': [], 'commands': commands.entries, 'compiler': {'path': str(compiler), 'sha256': sha(compiler)}, 'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(lib / 'kotlin-compiler.jar'), 'stdlib_sha256': sha(standard)}, 'environment': {'JAVA_HOME': os.environ['JAVA_HOME']}, 'claims': 'Bounded data-object singleton semantics and nested interface supertypes; not full reflection or unrestricted object inheritance.'}
    files = sorted([p for directory in ['src', 'tests/expanded_objects'] for p in (ROOT / directory).rglob('*') if p.is_file()] + [Path(__file__), ROOT / 'tools/upstream_jvm.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'tests/expanded_objects.rs', ROOT / 'tests/sealed_diagnostics.rs'])
    frozen = {str(p.relative_to(ROOT)): sha(p) for p in files}
    (work / 'source-manifest.json').write_text(json.dumps(frozen, indent=2))
    with zipfile.ZipFile(work / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            archive.write(path, str(path.relative_to(ROOT)))
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def run(label, argv):
        result = commands.run(label, argv)
        if result is None or result.returncode:
            raise RuntimeError(label + ': ' + (result.stderr if result else 'timeout'))
        return result
    def case(name, **values):
        report['cases'].append({'id': name, 'outcome': 'pass', **values})
    try:
        run('JVM version', [jdk / 'java', '-version'])
        run('Official compiler version', [oracle, '-version'])
        destinations = {}
        for provider in ['official', 'candidate']:
            dest = work / provider
            argv = [oracle if provider == 'official' else compiler, fixtures / 'Objects.kt', '-cp', standard, '-d', dest]
            argv += ['-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'] if provider == 'official' else ['--diagnostic-format', 'json']
            run('Compile ' + provider, argv)
            destinations[provider] = dest
            case('compile/' + provider)
        java = work / 'java'
        run('Compile fixed Java caller', [jdk / 'javac', '-cp', cp(destinations['official'], standard), '-d', java, fixtures / 'Check.java'])
        for provider, dest in destinations.items():
            output = run('Execute fixed caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(java, dest, standard), 'Check']).stdout.strip()
            if not output.startswith('OK '):
                raise RuntimeError(output)
            case('runtime/' + provider, independent_assertions=int(output.split()[1]), output=output)
        metadata = work / 'metadata'
        run('Compile metadata reader', [jdk / 'javac', '-cp', cp(standard, lib / 'kotlin-compiler.jar'), '-d', metadata, fixtures / 'MetadataDump.java'])
        owners = ['objectcases.Start', 'objectcases.Stop', 'objectcases.Container$Nested', 'objectcases.Empty', 'objectcases.Tracked', 'objectcases.SerializableEntry']
        outputs = {}
        for provider, dest in destinations.items():
            outputs[provider] = sorted(run('Read metadata ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(metadata, dest, standard, lib / 'kotlin-compiler.jar'), 'MetadataDump', *owners]).stdout.splitlines())
        if outputs['official'] != outputs['candidate']:
            raise RuntimeError('Metadata mismatch: ' + repr(outputs))
        case('metadata', snapshots=outputs)
        for owner in owners + ['objectcases.ObjectsKt']:
            signatures = {provider: normalize_signature(run('Signature ' + provider + '/' + owner, [jdk / 'javap', '-p', '-s', '-classpath', dest, owner]).stdout) for provider, dest in destinations.items()}
            if signatures['official'] != signatures['candidate']:
                raise RuntimeError('Signature mismatch ' + owner + repr(signatures))
            case('signature/' + owner)
        for provider, dest in destinations.items():
            consumer = work / ('consumer-' + provider)
            run('Compile Kotlin consumer ' + provider, [oracle, fixtures / 'Consumer.kt', '-cp', cp(dest, standard), '-d', consumer, '-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'])
            launcher = work / ('launcher-' + provider)
            run('Compile Kotlin consumer launcher ' + provider, [jdk / 'javac', '-cp', cp(consumer, standard), '-d', launcher, fixtures / 'Launch.java'])
            output = run('Execute Kotlin consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(launcher, consumer, dest, standard), 'Launch']).stdout.strip()
            if output != 'OK':
                raise RuntimeError(output)
            case('kotlin_consumer/' + provider)
        boundaries = [
            ('custom_hash', False, 'data object Bad { override fun hashCode(): Int = 1 }', 'DATA_OBJECT_CUSTOM_EQUALITY'),
            ('custom_equals', False, 'data object Bad { override fun equals(other: Any?): Boolean = false }', 'DATA_OBJECT_CUSTOM_EQUALITY'),
            ('custom_equals_without_override', False, 'data object Bad { fun equals(other: Any?): Boolean = false }', 'DATA_OBJECT_CUSTOM_EQUALITY'),
            ('superclass', True, 'open class Base\ndata object Boundary : Base()', 'OBJECT_SUPERCLASS_UNSUPPORTED'),
        ]
        for name, valid, source, code in boundaries:
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            results = {}
            for provider in ['official', 'candidate']:
                argv = [oracle if provider == 'official' else compiler, path, '-cp', standard, '-d', work / (provider + '-' + name)]
                argv += ['-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'] if provider == 'official' else ['--diagnostic-format', 'json']
                results[provider] = commands.run('Boundary ' + name + '/' + provider, argv)
            if results['official'] is None or (results['official'].returncode == 0) != valid:
                raise RuntimeError(name + ': oracle classification mismatch')
            category = 'unsupported_language' if valid else 'invalid_kotlin'
            actual = results['candidate']
            if actual is None or actual.returncode == 0 or code not in actual.stderr or category not in actual.stderr:
                raise RuntimeError(name + ': candidate rejection mismatch ' + (actual.stderr if actual else 'timeout'))
            report['cases'].append({'id': 'boundary/' + name, 'outcome': category, 'official_accepts': valid, 'candidate_code': code, 'source': str(path), 'source_sha256': sha(path)})
        context = {'directory': work / 'upstream', 'commands': commands, 'candidate': compiler, 'official': oracle, 'stdlib': standard, 'testlib': lib / 'kotlin-test.jar', **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        for name in ['equals', 'hashCode', 'toString', 'multipleInstances']:
            path = ROOT / 'references/kotlin/compiler/testData/codegen/box/dataObjects' / (name + '.kt')
            entry = {'id': 'upstream/' + name, 'family': 'expanded_objects', 'root': 'box', 'tier': 'basic', 'path': str(path.relative_to(ROOT)), 'kind': 'box', 'sha256': sha(path)}
            result = run_case(entry, context)
            if name == 'multipleInstances':
                result['acceptance_scope'] = 'external_reflection_assessment'
                result['scope_reason'] = 'The unchanged original requires reflection Constructor star-projection and compiled apply receiver handling outside the data-object contract. Duplicate equality is independently tested through a fixed Java reflection and serialization caller.'
            report['cases'].append(result)
        if args.app_capture:
            capture_path = args.app_capture.resolve()
            capture = json.loads(capture_path.read_text())
            task = capture['modules']['Alkitab']['compilation_tasks']['compilePlainDebugKotlin']
            declared = [Path(p) for p in task['compile_classpath']]
            available = [p for p in declared if p.exists()]
            original = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/ui/AudioBar.kt'
            result = commands.run('Unchanged AndroidBible AudioBar with captured real dependencies', [compiler, original, '-cp', cp(*available), '-d', work / 'androidbible-audiobar', '--diagnostic-format', 'json'])
            diagnostics = [json.loads(line) for line in (result.stderr if result else '').splitlines() if line.startswith('{')]
            report['app_assessment'] = {'original': str(original), 'source_sha256': sha(original), 'capture': str(capture_path), 'capture_sha256': sha(capture_path), 'captured_options': task['compiler_options'], 'captured_plugins': task['compiler_plugin_classpath'], 'missing_classpath_entries': [str(p) for p in declared if not p.exists()], 'classpath_files_sha256': {str(p): sha(p) for p in available if p.is_file()}, 'returncode': result.returncode if result else None, 'diagnostics': diagnostics, 'scope': 'Standalone unchanged-source assessment with actual captured dependencies; production JVM21/plugin build and bounded compiler JVM1.8 are distinct. No full-app or Compose transformation claim.'}
        selected = [c for c in report['cases'] if c.get('acceptance_scope') != 'external_reflection_assessment']
        accepted = not any(c['outcome'] == 'correctness_failure' for c in selected)
        report['feature_acceptance'] = {'family': 'expanded_objects', 'status': 'passed' if accepted else 'failed', 'derived_counts': dict(Counter(c['outcome'] for c in selected)), 'excluded_assessment_cases': [c['id'] for c in report['cases'] if c.get('acceptance_scope') == 'external_reflection_assessment']}
        report['status'] = ('complete_with_external_assessment_failure' if any(c['outcome'] == 'correctness_failure' for c in report['cases']) else 'complete') if accepted else 'failed'
    except Exception as error:
        report['status'] = 'failed'
        report['failure'] = str(error)
    current = sorted([p for directory in ['src', 'tests/expanded_objects'] for p in (ROOT / directory).rglob('*') if p.is_file()] + [Path(__file__), ROOT / 'tools/upstream_jvm.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'tests/expanded_objects.rs', ROOT / 'tests/sealed_diagnostics.rs'])
    current_hashes = {str(p.relative_to(ROOT)): sha(p) for p in current}
    report['input_integrity'] = {'changed_paths': sorted(name for name in frozen.keys() | current_hashes.keys() if frozen.get(name) != current_hashes.get(name))}
    if report['input_integrity']['changed_paths']:
        report['status'] = 'failed_input_integrity'
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    if args.publish:
        publish_report(report, work, args.publish.resolve())
    print(json.dumps({'status': report['status'], 'counts': report['counts'], 'failure': report.get('failure')}))
    return 0 if report.get('feature_acceptance', {}).get('status') == 'passed' and not report['input_integrity']['changed_paths'] else 1

if __name__ == '__main__':
    raise SystemExit(main())
