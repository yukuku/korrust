#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import zipfile

from upstream_jvm import Commands, classify_candidate, normalize_signature, relative, run_case, sha

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/new_families/primitive_arrays'
INVALID = {
    'boxed_to_primitive': 'fun bad(value: Array<Long>): LongArray = value',
    'primitive_to_boxed': 'fun bad(value: LongArray): Array<Long> = value',
    'boolean_to_byte': 'fun bad(value: BooleanArray): ByteArray = value',
    'wrong_store': 'fun bad(value: DoubleArray) { value[0] = 1 }',
    'nonliteral_long_store': 'fun bad(value: LongArray, item: Int) { value[0] = item }',
    'boolean_store': 'fun bad(value: BooleanArray) { value[0] = 1 }',
    'nullable_index': 'fun bad(value: DoubleArray?): Double = value[0]',
    'nonnull_type_use_argument': 'import newfamilies.arrays.TypeUses\nfun bad(value: DoubleArray?) = TypeUses.nonnull(value)',
    'nullable_type_use_result': 'import newfamilies.arrays.TypeUses\nfun bad(): DoubleArray = TypeUses.nullable(null)',
    'wrong_spread': 'fun pack(vararg value: Long): LongArray = value; fun bad(value: IntArray): LongArray = pack(*value)',
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    work = args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    jdk_home = subprocess.check_output(['/usr/libexec/java_home', '-v', '21'], text=True).strip()
    environment = {**os.environ, 'JAVA_HOME': jdk_home}
    jdk = Path(jdk_home) / 'bin'
    reference = ROOT / 'references/kotlinc'
    oracle = reference / 'bin/kotlinc'
    stdlib = reference / 'lib/kotlin-stdlib.jar'
    compiler = work.parent / (work.name + '-korrust-frozen')
    shutil.copy2(args.compiler.resolve(), compiler)
    logs = work / 'commands'
    logs.mkdir()
    commands = Commands(logs, environment)
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running',
        'selection': 'LongArray, BooleanArray, FloatArray and DoubleArray as one representation family; construction/initialization/indexing/store/iteration/vararg/spread, structural builtin factories, ordinary Java calls, Kotlin metadata and unchanged AndroidBible AudioHighlightColor with real dependencies. Byte/Int/Char arrays are regressions. ShortArray remains unsupported.',
        'compiler': {'path': relative(compiler), 'sha256': sha(compiler)},
        'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(reference / 'lib/kotlin-compiler.jar'), 'stdlib_sha256': sha(stdlib)},
        'environment': {'JAVA_HOME': jdk_home}, 'inputs': {}, 'commands': [], 'cases': [],
        'limitations': ['ShortArray is explicitly unsupported. Scalar Short is validated separately.', 'Effectful initializer-producing expressions retain the existing specification/oracle evaluation-order rejection boundary.', 'No source inline expansion, complete stdlib array API, Android device/full-app or performance claim.', 'JVM method return/formal parameter type-use nullability supports empty type paths only; unsupported targets/nested paths are preserved as unsupported.']}
    upstream = [ROOT / ('references/kotlin/compiler/testData/codegen/box/arrays/forEach' + name + 'Array.kt') for name in ['Long', 'Boolean', 'Float', 'Double', 'Int', 'Byte', 'Char']]
    frozen_paths = [ROOT / 'tests/fixtures/compatibility_enums_numeric/UnsupportedDoubleArray.kt', ROOT / 'tests/fixtures/compatibility_enums_numeric/rejections.json', ROOT / 'tests/primitive_arrays.rs', *sorted((ROOT / 'src').rglob('*.rs')), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), ROOT / 'tools/upstream_jvm.py', *sorted(FIXTURES.glob('*')), *upstream, compiler, oracle, stdlib, reference / 'lib/kotlin-compiler.jar']
    frozen = {relative(path): sha(path) for path in frozen_paths}
    report['inputs']['files_sha256'] = frozen
    report_path = work / 'report.json'

    def save():
        report['commands'] = commands.entries
        report['counts'] = dict(sorted(Counter(case['outcome'] for case in report['cases']).items()))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def case(ident, outcome='pass', **details):
        report['cases'].append({'id': ident, 'outcome': outcome, **details})
        save()

    def run(label, argv):
        print(label, flush=True)
        result = commands.run(label, argv, timeout=300)
        if result is None:
            raise TimeoutError(label)
        return result

    def require(label, argv):
        result = run(label, argv)
        if result.returncode:
            raise AssertionError(label + ': ' + result.stderr[-2500:])
        return result

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def compile(provider, sources, output, dependencies):
        tool = oracle if provider == 'official' else compiler
        options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_arrays'] if provider == 'official' else ['--module-name', 'new_arrays', '--diagnostic-format', 'json']
        return run('Compile ' + provider + '/' + output.name, [tool, *sources, *options, '-cp', cp(*dependencies, stdlib), '-d', output])

    try:
        version = require('Pinned oracle identity', [oracle, '-version'])
        assert 'kotlinc-jvm 2.3.21 ' in version.stdout + version.stderr
        require('JDK identity', [jdk / 'java', '-version'])
        helpers = work / 'helpers'
        jspecify = next(path for path in (Path.home() / '.gradle/caches/modules-2/files-2.1/org.jspecify/jspecify/1.0.0').rglob('*.jar') if not path.name.endswith(('-sources.jar', '-javadoc.jar')))
        annotations = reference / 'lib/annotations-13.0.jar'
        frozen.update({str(path): sha(path) for path in [jspecify, annotations]})
        report['inputs']['type_use_annotations'] = [{'path': str(path), 'sha256': sha(path)} for path in [jspecify, annotations]]
        require('Compile real Java effect/type-use helpers', [jdk / 'javac', '-cp', cp(jspecify, annotations), '-d', helpers, FIXTURES / 'ArrayEffects.java', FIXTURES / 'TypeUses.java', FIXTURES / 'Unknown.java'])
        sources = [FIXTURES / name for name in ['Arrays.kt', 'AliasedFactory.kt', 'Shadowed.kt', 'TypeUseCalls.kt']] + [ROOT / 'tests/fixtures/compatibility_enums_numeric/UnsupportedDoubleArray.kt']
        outputs = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            actual = compile(provider, sources, output, [helpers, jspecify, annotations])
            assert actual.returncode == 0, actual.stderr
            outputs[provider] = output
            case('compile/' + provider)
        callers = work / 'callers'
        require('Compile fixed Java assertions only against official output', [jdk / 'javac', '-cp', cp(outputs['official'], helpers, stdlib), '-d', callers, FIXTURES / 'ArraysCheck.java'])
        hashes = {str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}
        report['inputs']['fixed_java_callers_sha256'] = hashes
        expected = len(re.findall(r'\brequire\(', (FIXTURES / 'ArraysCheck.java').read_text())) - 1
        for provider, output in outputs.items():
            result = require('Verified fixed Java assertions ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, output, helpers, stdlib), 'newfamilies.arrays.ArraysCheck'])
            assert result.stdout == 'checks=' + str(expected) + '\n', result.stdout
            assert hashes == {str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}
            case('runtime/' + provider, independent_assertions=expected, fixed_official_java_caller=True, jvm_verification='all')
            consumer = work / ('consumer-' + provider)
            require('Compile official Kotlin metadata consumer ' + provider, [oracle, FIXTURES / 'Consumer.kt', '-cp', cp(output, helpers, stdlib), '-d', consumer])
            actual = require('Verified Kotlin metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, helpers, stdlib), 'arrayconsumer.ConsumerKt'])
            assert actual.stdout == 'OK\n'
            case('metadata/' + provider, scope='Primitive-array result/parameter/nullability/vararg metadata, ordinary functions and aliased builtin factories')
        for owner in ['newfamilies.arrays.ArraysKt', 'newfamilies.arrays.ArrayHolder', 'newfamilies.arrays.AliasedFactoryKt', 'newfamilies.arrays.TypeUseCallsKt', 'newfamilies.arrays.shadow.ShadowedKt', 'compatibility.unsupported.UnsupportedDoubleArrayKt']:
            signatures = [normalize_signature(require('JVM signatures ' + provider + '/' + owner, [jdk / 'javap', '-protected', '-s', '-classpath', output, owner]).stdout) for provider, output in outputs.items()]
            assert signatures[0] == signatures[1], str(signatures)
            case('signature/' + owner)
        for name, source in INVALID.items():
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            for provider in ['official', 'candidate']:
                output = work / ('invalid-' + provider + '-' + name)
                actual = compile(provider, [path], output, [helpers, jspecify, annotations])
                assert actual.returncode and not list(output.rglob('*.class')), actual.stderr
            case('invalid/' + name, outcome='invalid_kotlin')
        for name, source in {
            'short_type': 'fun bad(value: ShortArray): Int = 1',
            'builtin_classifier_shadow': 'class DoubleArray',
            'short_array': 'fun bad() = ShortArray(2)',
            'effectful_initializer': 'fun init(): (Int) -> Double = { it.toDouble() }; fun bad() = DoubleArray(2, init())',
        }.items():
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            official = compile('official', [path], work / ('boundary-official-' + name), [])
            actual = compile('candidate', [path], work / ('boundary-candidate-' + name), [])
            assert official.returncode == 0 and classify_candidate(actual) == 'unsupported_language', actual.stderr
            case('boundary/' + name, outcome='unsupported_language', diagnostic=actual.stderr)
        for name, source in {'nested_type_use': 'import newfamilies.arrays.TypeUses\nfun bad(value: Array<String>): Array<String> = TypeUses.nested(value)', 'unknown_type_use': 'import newfamilies.arrays.TypeUses\nfun bad(value: DoubleArray): DoubleArray = TypeUses.unknown(value)'}.items():
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            official = compile('official', [path], work / ('boundary-official-' + name), [helpers, jspecify, annotations])
            actual = compile('candidate', [path], work / ('boundary-candidate-' + name), [helpers, jspecify, annotations])
            assert official.returncode == 0 and classify_candidate(actual) == 'unsupported_language', actual.stderr
            case('boundary/' + name, outcome='unsupported_language', diagnostic=actual.stderr)
        context = {'directory': work, 'commands': commands, 'candidate': compiler, 'official': oracle, 'stdlib': stdlib, 'testlib': reference / 'lib/kotlin-test.jar', **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        for source in upstream:
            entry = {'id': 'original/' + source.stem, 'family': 'primitive_arrays', 'root': 'box', 'tier': 'original', 'kind': 'box', 'path': str(source.relative_to(ROOT)), 'sha256': sha(source)}
            result = run_case(entry, context)
            report['cases'].append(result)
            save()
        deps = ROOT / 'target/dependencies'
        deps.mkdir(parents=True, exist_ok=True)
        aar = next((Path.home() / '.gradle/caches/modules-2/files-2.1/androidx.core/core/1.17.0').rglob('*.aar'))
        core = deps / 'core-1.17.0.jar'
        with zipfile.ZipFile(aar) as archive:
            core.write_bytes(archive.read('classes.jar'))
        sdk = Path.home() / 'Library/Android/sdk/platforms/android-35/android.jar'
        runtime = next((Path.home() / '.gradle/caches/modules-2/files-2.1/org.robolectric/android-all/15-robolectric-12650502').rglob('*.jar'))
        app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/ui/AudioHighlightColor.kt'
        frozen.update({str(path): sha(path) for path in [aar, core, sdk, runtime, app]})
        report['inputs']['real_app_dependencies'] = [{'path': str(path), 'sha256': sha(path)} for path in [aar, core, sdk, runtime]]
        report['inputs']['unchanged_app'] = {'path': str(app), 'sha256': sha(app)}
        app_outputs = {}
        for provider in ['official', 'candidate']:
            output = work / ('app-' + provider)
            actual = compile(provider, [app], output, [sdk, core])
            if actual.returncode:
                case('app_compile/' + provider, outcome=classify_candidate(actual) if provider == 'candidate' else 'oracle_failure', diagnostic=actual.stderr, unchanged_original=True)
            else:
                app_outputs[provider] = output
                case('app_compile/' + provider, unchanged_original=True)
        if len(app_outputs) == 2:
            driver = work / 'app-caller'
            require('Compile app assertions against official output and real Android signatures', [jdk / 'javac', '-cp', cp(app_outputs['official'], sdk, core, stdlib), '-d', driver, FIXTURES / 'AudioCheck.java'])
            for provider, output in app_outputs.items():
                actual = require('Run unchanged app with actual Android/core implementations ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(driver, output, core, runtime, stdlib), 'AudioCheck'])
                assert actual.stdout == 'audio-checks=20\n', actual.stdout
                case('app_runtime/' + provider, independent_assertions=20, jvm_verification='all', runtime='Actual android-all15 + AndroidX Core1.17, plain JVM; no Android device or Robolectric shadows')
        report['status'] = 'complete_with_recorded_failures' if any(item['outcome'] in {'correctness_failure', 'oracle_failure'} for item in report['cases']) else 'complete_with_recorded_boundaries'
    except Exception as error:
        report['status'] = 'failed'
        case('harness_failure', outcome='correctness_failure', error=str(error))
    finally:
        changed = [path for path, value in frozen.items() if not Path(path if Path(path).is_absolute() else ROOT / path).is_file() or sha(Path(path) if Path(path).is_absolute() else ROOT / path) != value]
        report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
        if changed:
            report['status'] = 'invalidated'
        save()
        print(relative(report_path), report['status'], report['counts'])


if __name__ == '__main__':
    main()
