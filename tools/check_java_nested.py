#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import zipfile
import upstream_jvm
from upstream_jvm import Commands, normalize_signature


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root, work, candidate = args.root.resolve(), args.output.resolve(), args.candidate.resolve()
    upstream_jvm.ROOT = root
    work.mkdir(parents=True, exist_ok=False)
    (work / 'commands').mkdir()
    commands = Commands(work / 'commands', dict(os.environ))
    fixtures = root / 'tests/java_nested'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    official = root / 'references/kotlinc/bin/kotlinc'
    standard = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
    original = root / 'references/kotlin/compiler/testData/compileKotlinAgainstJava/StaticNestedClass.kt'
    original_java = original.with_suffix('.java')
    sources = [*sorted((root / 'src').rglob('*.rs')), *sorted(path for path in fixtures.rglob('*') if path.is_file()), Path(__file__).resolve(), root / 'tools/upstream_jvm.py', root / 'Cargo.toml', root / 'Cargo.lock', root / 'references/compiler.json', root / 'references/sources.json', original, original_java]
    frozen = {str(path): sha(path) for path in [*sources, candidate, standard, root / 'references/kotlinc/lib/kotlin-compiler.jar']}
    with zipfile.ZipFile(work / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in sources:
            archive.write(path, str(path.relative_to(root)) if path.is_relative_to(root) else 'harness/' + path.name)
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'compiler': {'path': str(candidate), 'sha256': sha(candidate)}, 'environment': {'JAVA_HOME': os.environ['JAVA_HOME']}, 'frozen_input_hashes': frozen, 'cases': [], 'commands': commands.entries, 'scope': 'Direct Java member-class namespace resolution using InnerClasses metadata, static field reads/calls, actual access flags and existing Kotlin value/import priority. No source-inner expansion or new family claim.', 'limitations': ['Protected Java nested classifiers and nonpublic Java constructor visibility remain explicit boundaries.', 'Inherited member-class lookup and broader Java field/type/construction rules retain existing boundaries.'], 'reproduction_command': ['python3', str(Path(__file__).resolve()), '--root', str(root), '--candidate', str(candidate), '--output', str(work)]}
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def run(label, argv, accepted=(0,)):
        result = commands.run(label, argv, timeout=240)
        if result is None or result.returncode not in accepted:
            raise RuntimeError(label + ': ' + (result.stdout + result.stderr if result else 'timeout'))
        return result
    def case(name, **detail):
        report['cases'].append({'id': name, 'outcome': 'pass', **detail})
    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Pinned Kotlin identity', [official, '-version'])
        dependency = work / 'dependency'
        java_sources = [*sorted((fixtures / 'dependency').glob('*.java')), original_java]
        run('Compile identical Java dependency', [jdk / 'javac', '-d', dependency, *java_sources])
        dependency_hashes = {str(path): sha(path) for path in dependency.rglob('*.class')}
        kotlin_sources = [fixtures / name for name in ['Reads.kt', 'SamePackage.kt', 'AliasPackage.kt', 'DefaultAlias.kt', 'Callees.kt']] + [original]
        destinations = {'official': work / 'official', 'candidate': work / 'candidate'}
        for provider, destination in destinations.items():
            binary = official if provider == 'official' else candidate
            argv = [binary, *kotlin_sources, '-cp', cp(dependency, standard), '-d', destination]
            argv += ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8'] if provider == 'official' else ['--diagnostic-format', 'json']
            run('Compile unchanged original and independent model/' + provider, argv)
            case('compile/' + provider, original_source_sha256=sha(original))
        callers = work / 'java'
        run('Compile fixed Java callers against official only', [jdk / 'javac', '-cp', cp(destinations['official'], dependency, standard), '-d', callers, *[fixtures / name for name in ['Check.java', 'OriginalCheck.java', 'DefaultCheck.java', 'CalleeCheck.java']]])
        caller_hashes = {str(path): sha(path) for path in callers.rglob('*.class')}
        for provider, destination in destinations.items():
            for main, expected, count in [('nestedcases.Check', 'OK 21', 21), ('test.OriginalCheck', 'OK 2', 2), ('DefaultCheck', 'OK 4', 4), ('calleecases.CalleeCheck', 'OK 24', 24)]:
                result = run('Fixed Java runtime/' + provider + '/' + main, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, destination, dependency, standard), main])
                assert result.stdout.strip() == expected, result.stdout
                case('runtime/' + provider + '/' + main, independent_assertions=count)
        owners = ['nestedcases.ReadsKt', 'nestedcases.LocalRoot', 'nestedcases.LocalTools', 'library.SamePackageKt', 'library.AliasPackageKt', 'DefaultAliasKt', 'test.StaticNestedClassKt', 'calleecases.CalleesKt']
        signatures = {provider: normalize_signature(run('JVM signatures/' + provider, [jdk / 'javap', '-public', '-s', '-classpath', cp(destination, dependency), *owners]).stdout) for provider, destination in destinations.items()}
        assert signatures['official'] == signatures['candidate'], 'Public JVM signatures differ'
        case('public_jvm_signatures', owners=owners)
        for entry in json.loads((fixtures / 'boundaries/cases.json').read_text()):
            name, valid, code = entry['id'], entry['valid'], entry['code']
            source = fixtures / 'boundaries' / (name + '.kt')
            oracle = run('Oracle boundary/' + name, [official, source, '-cp', cp(dependency, standard), '-d', work / ('official-' + name)], accepted=(0,) if valid else (1,))
            checked = run('Candidate boundary/' + name, [candidate, source, '-cp', cp(dependency, standard), '-d', work / ('candidate-' + name), '--diagnostic-format', 'json'], accepted=(1,))
            diagnostics = [json.loads(line) for line in checked.stderr.splitlines() if line.startswith('{')]
            assert diagnostics, checked.stderr
            if valid:
                assert diagnostics[0]['category'] == 'unsupported_language', diagnostics
            elif code is not None:
                assert diagnostics[0]['category'] == 'invalid_kotlin', diagnostics
            if code is not None:
                assert diagnostics[0]['code'] == code, diagnostics
            report['cases'].append({'id': 'boundary/' + name, 'outcome': 'unsupported_language' if valid else 'invalid_rejected', 'official_accepts': valid, 'diagnostics': diagnostics, 'source_sha256': sha(source)})
        assert caller_hashes == {str(path): sha(path) for path in callers.rglob('*.class')}
        assert dependency_hashes == {str(path): sha(path) for path in dependency.rglob('*.class')}
        case('fixed_caller_dependency_integrity', caller_hashes=caller_hashes, dependency_hashes=dependency_hashes)
        report['status'] = 'complete'
    except Exception as error:
        report['status'], report['error'] = 'failed', str(error)
    changed = [name for name, expected in frozen.items() if not Path(name).is_file() or sha(Path(name)) != expected]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    if changed:
        report['status'] = 'invalidated'
    report['counts'] = dict(Counter(item['outcome'] for item in report['cases']))
    (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'counts': report['counts'], 'error': report.get('error')}))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
