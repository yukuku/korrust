#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import subprocess

from upstream_jvm import Commands, classify_candidate, normalize_signature, publish_report, relative, sha

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/new_families/aliases'
UPSTREAM = ROOT / 'references/kotlin/compiler/testData/codegen/box/typealias'
APP = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/compose/goto/GotoTab.kt'
INVALID = {
    'missing_arguments': 'typealias Rows<T> = List<T>\nfun bad(value: Rows): Int = 1',
    'extra_arguments': 'typealias Rows<T> = List<T>\nfun bad(value: Rows<String, Int>): Int = 1',
    'duplicate_parameters': 'typealias Rows<T, T> = List<T>',
    'variance': 'typealias Rows<out T> = List<T>',
    'reified': 'typealias Rows<reified T> = List<T>',
    'explicit_bound': 'typealias Rows<T : Any> = List<T>',
    'parameter_rhs': 'typealias Identity<T> = T',
    'nullable_parameter_rhs': 'typealias Identity<T> = T?',
    'recursive': 'typealias Cycle<T> = List<Cycle<T>>',
    'mutual_recursive': 'typealias Left<T> = List<Right<T>>\ntypealias Right<T> = List<Left<T>>',
    'parameter_scope': 'typealias Rows<T> = List<T>\nfun bad(value: T): Int = 1',
    'implicit_bound': 'import aliasdep.Bounded\ntypealias Constrained<T> = Bounded<T>\nfun bad(value: Constrained<Int>): Int = 1',
    'nullable_bound': 'import aliasdep.Bounded\ntypealias Constrained<T> = Bounded<T>\nfun bad(value: Constrained<String?>): Int = 1',
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/alias-build/release/korrust')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--publish', type=Path)
    parser.add_argument('--source-snapshot', type=Path, default=ROOT)
    args = parser.parse_args()
    work = args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    now = datetime.now(timezone.utc)
    environment = dict(os.environ)
    if not environment.get('JAVA_HOME'):
        environment['JAVA_HOME'] = subprocess.check_output(['/usr/libexec/java_home', '-v', '21'], text=True).strip()
    java_home = Path(environment['JAVA_HOME'])
    if not (java_home / 'jmods/java.base.jmod').is_file():
        settings = subprocess.run([java_home / 'bin/java', '-XshowSettings:properties', '-version'], text=True, capture_output=True, check=True)
        actual_home = re.search(r'^\s*java.home\s*=\s*(.+)$', settings.stderr, re.MULTILINE)
        if actual_home is None:
            raise RuntimeError('Cannot determine actual JDK home')
        environment['JAVA_HOME'] = actual_home.group(1).strip()
    jdk = Path(environment['JAVA_HOME']) / 'bin'
    compiler = work.parent / (work.name + '-korrust-frozen')
    shutil.copy2(args.compiler.resolve(), compiler)
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
    stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
    upstream_files = [UPSTREAM / name for name in ['typeAliasFunction.kt', 'simple.kt', 'genericTypeAliasConstructor2.kt']]
    snapshot = args.source_snapshot.resolve()
    files = [snapshot / 'Cargo.toml', snapshot / 'Cargo.lock', *sorted((snapshot / 'src').rglob('*.rs')),
             ROOT / 'tests/generic_aliases.rs', Path(__file__), ROOT / 'tools/upstream_jvm.py',
             *sorted(FIXTURES.glob('*')), *upstream_files, APP,
             ROOT / 'references/compiler.json', ROOT / 'references/sources.json', compiler, stdlib, ROOT / pin['path'] / 'lib/kotlin-compiler.jar']
    frozen = {relative(path): sha(path) for path in files}
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': now.isoformat(), 'status': 'running',
              'selection': 'Source generic aliases in type positions: substitution, nullable/function/receiver/list/array/classifier aliases, public/private cross-file scope, arity/cycles/implicit target bounds and exported metadata. Existing nongeneric AndroidBible OnGotoFinished is a regression; no new app target or source generic-class support is claimed.',
              'compiler': {'path': relative(compiler), 'sha256': sha(compiler), 'source_snapshot': relative(snapshot)},
              'reference_compiler': {'version': pin['version'], 'archive_sha256': pin['archive_sha256'], 'compiler_jar_sha256': sha(ROOT / pin['path'] / 'lib/kotlin-compiler.jar'), 'stdlib_sha256': sha(stdlib)},
              'inputs': {'files_sha256': frozen}, 'environment': {'JAVA_HOME': environment['JAVA_HOME']}, 'commands': [], 'cases': [],
              'limitations': ['Generic library-property getter result substitution and alias constructor resolution remain independently recorded failing boundaries.', 'Projected alias arguments at concrete nested positions are explicitly unsupported.', 'Source generic classes, full Kotlin metadata parity, full-app compatibility and performance are not claimed.', 'Read-only List parameter wildcards are verified for known nested list/function/type-parameter forms; arbitrary classifier finality remains unassessed.']}
    logs = work / 'commands'
    logs.mkdir()
    commands = Commands(logs, environment)
    report_path = work / 'report.json'

    def record(ident, outcome='pass', **details):
        report['cases'].append({'id': ident, 'outcome': outcome, **details})
        report['commands'] = commands.entries
        report['counts'] = dict(sorted(Counter(case['outcome'] for case in report['cases']).items()))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, argv):
        print(label, flush=True)
        result = commands.run(label, argv)
        if result is None:
            raise TimeoutError(label)
        return result

    def require(label, argv):
        result = run(label, argv)
        if result.returncode:
            raise AssertionError(label + ': ' + result.stderr[-3500:])
        return result

    def cp(*entries):
        return os.pathsep.join(map(str, entries))

    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_aliases']
    dependencies = work / 'dependencies'

    def compile(provider, sources, output, extra):
        if provider == 'official':
            argv = [kotlinc, *sources, *options]
        else:
            argv = [compiler, *sources, '--module-name', 'new_aliases', '--diagnostic-format', 'json']
        return run('Compile ' + provider + ' ' + output.name, [*argv, '-cp', cp(*extra, stdlib), '-d', output])

    try:
        version = require('Pinned oracle identity', [kotlinc, '-version'])
        assert 'kotlinc-jvm ' + pin['version'] + ' ' in version.stdout + version.stderr
        require('JDK identity', [jdk / 'java', '-version'])
        require('Compile ordinary generic dependency with pinned Kotlin', [kotlinc, FIXTURES / 'Dependency.kt', *options, '-d', dependencies])
        record('dependency', scope='Official-built generic dependency; excluded from source compiler feature count')
        sources = [FIXTURES / 'Definitions.kt', FIXTURES / 'Uses.kt']
        outputs = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            actual = compile(provider, sources, output, [dependencies])
            assert actual.returncode == 0, actual.stderr
            outputs[provider] = output
            record('compile/' + provider)
        reversed_output = work / 'candidate-reversed'
        actual = compile('candidate', list(reversed(sources)), reversed_output, [dependencies])
        assert actual.returncode == 0, actual.stderr
        outputs['candidate-reversed'] = reversed_output
        record('cross_file_order')
        caller = work / 'java-caller'
        require('Compile fixed Java assertions against official output', [jdk / 'javac', '-cp', cp(outputs['official'], dependencies, stdlib), '-d', caller, FIXTURES / 'AliasesCheck.java'])
        caller_hashes = {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
        report['inputs']['fixed_java_caller_sha256'] = caller_hashes
        expected_checks = len(re.findall(r'\brequire\(', (FIXTURES / 'AliasesCheck.java').read_text())) - 1
        for provider, output in outputs.items():
            result = require('Verified fixed Java assertions ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, dependencies, stdlib), 'newfamilies.aliases.AliasesCheck'])
            assert result.stdout == 'checks=' + str(expected_checks) + '\n', result.stdout
            assert caller_hashes == {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
            record('runtime/' + provider, independent_assertions=expected_checks, fixed_official_java_caller=True, jvm_verification='all')
        for owner in ['DefinitionsKt', 'UsesKt', 'Holder']:
            signatures = {}
            for provider, output in outputs.items():
                result = require('JVM signature ' + provider + '/' + owner, [jdk / 'javap', '-protected', '-s', '-classpath', output, 'newfamilies.aliases.' + owner])
                signatures[provider] = normalize_signature(result.stdout)
            assert len({tuple(signature) for signature in signatures.values()}) == 1, str(signatures)
            record('signature/' + owner)
        for provider, output in outputs.items():
            consumer = work / ('consumer-' + provider)
            require('Official Kotlin exported generic alias consumer ' + provider, [kotlinc, FIXTURES / 'Consumer.kt', *options, '-cp', cp(output, dependencies, stdlib), '-d', consumer])
            result = require('Verified exported alias consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, dependencies, stdlib), 'aliasconsumer.ConsumerKt'])
            assert result.stdout == 'OK\n'
            record('kotlin_metadata/' + provider, scope='Exported generic alias type parameters, nested/function/receiver/nullable/array types, dependent generic bounds and ordinary class properties')
            for name, source in {
                'private_alias': 'import newfamilies.aliases.PrivateRows\nfun bad(value: PrivateRows<Int>) {}',
                'alias_bound': 'import newfamilies.aliases.Constrained\nfun bad(value: Constrained<Int>) {}',
            }.items():
                path = work / ('ConsumerBoundary_' + name + '.kt')
                path.write_text(source + '\n')
                actual = compile('official', [path], work / ('consumer-rejected-' + provider + '-' + name), [output, dependencies])
                assert actual.returncode and not list((work / ('consumer-rejected-' + provider + '-' + name)).rglob('*.class')), actual.stderr
                record('metadata_rejection/' + provider + '/' + name, outcome='invalid_kotlin')
        for name, source in INVALID.items():
            path = work / ('Boundary_' + name + '.kt')
            path.write_text(source + '\n')
            classifications = {}
            for provider in ['official', 'candidate']:
                output = work / ('invalid-' + provider + '-' + name)
                actual = compile(provider, [path], output, [dependencies])
                assert actual.returncode and not list(output.rglob('*.class')), name + ': ' + actual.stderr
                classifications[provider] = actual.stderr
            record('invalid/' + name, outcome='invalid_kotlin', diagnostics=classifications)
        private = work / 'PrivateDefinitions.kt'
        use = work / 'PrivateUse.kt'
        private.write_text('package privatealiases\nprivate typealias Rows<T> = List<T>\n')
        use.write_text('package privatealiases\nfun bad(value: Rows<String>): Int = 1\n')
        for provider in ['official', 'candidate']:
            output = work / ('private-cross-file-' + provider)
            actual = compile(provider, [private, use], output, [dependencies])
            assert actual.returncode and not list(output.rglob('*.class'))
        record('invalid/private_cross_file', outcome='invalid_kotlin')
        projection = work / 'ProjectedAlias.kt'
        projection.write_text('typealias Rows<T> = List<T>\nfun boundary(value: Rows<out String>): Int = 1\n')
        oracle = compile('official', [projection], work / 'projected-official', [dependencies])
        actual = compile('candidate', [projection], work / 'projected-candidate', [dependencies])
        assert oracle.returncode == 0 and classify_candidate(actual) == 'unsupported_language'
        record('boundary/projected_nested', outcome='unsupported_language', diagnostic=actual.stderr)
        for name in ['UnsupportedGenericGetter.kt', 'UnsupportedAliasConstructor.kt']:
            source = FIXTURES / name
            oracle = compile('official', [source], work / ('boundary-official-' + source.stem), [dependencies])
            actual = compile('candidate', [source], work / ('boundary-candidate-' + source.stem), [dependencies])
            assert oracle.returncode == 0
            outcome = classify_candidate(actual)
            assert actual.returncode, 'Boundary unexpectedly compiles; requires runtime assertions before claim'
            record('boundary/' + source.stem, outcome=outcome, source=relative(source), sha256=sha(source), diagnostics=actual.stderr, relation_to_scope='Existing constructor/getter resolution boundary; not accepted generic alias behavior')
        for source in upstream_files:
            outputs_upstream = {}
            for provider in ['official', 'candidate']:
                output = work / ('upstream-' + provider + '-' + source.stem)
                actual = compile(provider, [source], output, [dependencies])
                if provider == 'official':
                    assert actual.returncode == 0, actual.stderr
                elif actual.returncode:
                    record('upstream/' + source.stem, outcome=classify_candidate(actual), source=relative(source), sha256=sha(source), diagnostics=actual.stderr, directives_preserved=True)
                    break
                outputs_upstream[provider] = output
            else:
                facade = source.stem[0].upper() + source.stem[1:] + 'Kt'
                driver = work / ('Box_' + source.stem + '.java')
                driver.write_text('public final class Box_' + source.stem + ' { public static void main(String[] args) { if (!"OK".equals(' + facade + '.box())) throw new AssertionError(); System.out.println("OK"); } }\n')
                driver_out = work / ('upstream-caller-' + source.stem)
                require('Compile fixed upstream box caller ' + source.stem, [jdk / 'javac', '-cp', cp(outputs_upstream['official'], stdlib), '-d', driver_out, driver])
                hashes = {str(path.relative_to(driver_out)): sha(path) for path in driver_out.rglob('*.class')}
                for provider, output in outputs_upstream.items():
                    result = require('Verified unchanged upstream ' + provider + '/' + source.stem, [jdk / 'java', '-Xverify:all', '-cp', cp(driver_out, output, stdlib), 'Box_' + source.stem])
                    assert result.stdout == 'OK\n'
                record('upstream/' + source.stem, source=relative(source), sha256=sha(source), unchanged_upstream=True, fixed_official_java_caller_hashes=hashes, directives_preserved=True,
                       backend_note='WASM_FAILS_IN is read only by upstream WasmBoxRunner/WasiBoxRunner and does not constrain JVM execution' if source.name == 'simple.kt' else None)
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())
        assert sha(APP) == inventory['androidbible'][str(APP.relative_to(ROOT / 'benchmarks/sources/androidbible'))]
        for provider in ['official', 'candidate']:
            output = work / ('goto-' + provider)
            actual = compile(provider, [APP], output, [])
            assert actual.returncode == 0, actual.stderr
            consumer = work / ('goto-consumer-' + provider)
            require('Compile existing AndroidBible alias consumer ' + provider, [kotlinc, FIXTURES / 'GotoRegression.kt', *options, '-cp', cp(output, stdlib), '-d', consumer])
            result = require('Verified existing AndroidBible alias regression ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, stdlib), 'aliasregression.GotoRegressionKt'])
            assert result.stdout == 'OK\n'
            record('app_regression/' + provider, source=relative(APP), sha256=sha(APP), new_app_target=False, scope='Unchanged nongeneric OnGotoFinished/GotoTab regression only')
        report['status'] = 'complete_with_recorded_boundaries'
    except Exception as error:
        report['status'] = 'failed'
        record('harness_failure', outcome='correctness_failure', error=str(error))
    finally:
        changed = [path for path, value in frozen.items() if not (ROOT / path).is_file() or sha(ROOT / path) != value]
        report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
        if changed:
            report['status'] = 'invalidated'
        report['commands'] = commands.entries
        report['counts'] = dict(sorted(Counter(case['outcome'] for case in report['cases']).items()))
        report_path.write_text(json.dumps(report, indent=2) + '\n')
        if args.publish:
            destination = args.publish.resolve()
            publish_report(report, work, destination)
            published = json.loads(destination.read_text())
            published['reproduction_command'] = ['python3', 'tools/check_new_aliases.py', '--compiler', report['compiler']['path'], '--source-snapshot', report['compiler']['source_snapshot'], '--output', 'target/source-generic-aliases-fresh']
            destination.write_text(json.dumps(published, indent=2) + '\n')
        print(relative(report_path), report['status'], json.dumps(report['counts'], sort_keys=True))


if __name__ == '__main__':
    main()
