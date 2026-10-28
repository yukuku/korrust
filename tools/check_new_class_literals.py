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

from upstream_jvm import Commands, run_case

ROOT = Path(__file__).resolve().parents[1]
BOUNDARIES = [
    ('constant_class_literal', False, 'const val TOKEN = String::class', 'INVALID_CONSTANT_EXPRESSION'),
    ('nullable_value', False, 'fun bad(value: String?) = value::class', 'INVALID_CLASS_LITERAL'),
    ('nullable_value_shadow', False, 'val String: Any = 7\nfun bad() = String?::class', 'INVALID_CLASS_LITERAL'),
    ('generic_value_shadow', False, 'val List: Any = 8\nfun bad() = List<String>::class', 'INVALID_CLASS_LITERAL'),
    ('nullable_type', False, 'fun bad() = String?::class', 'INVALID_CLASS_LITERAL'),
    ('null_literal', False, 'fun bad() = null::class', 'INVALID_CLASS_LITERAL'),
    ('generic_class', False, 'fun bad() = List<String>::class', 'INVALID_CLASS_LITERAL'),
    ('generic_array_component', False, 'fun bad() = Array<List<String>>::class', 'INVALID_CLASS_LITERAL'),
    ('function_array_component', False, 'fun bad() = Array<(Int) -> Unit>::class', 'INVALID_CLASS_LITERAL'),
    ('nested_star_array', False, 'fun bad() = Array<Array<*>>::class', 'INVALID_CLASS_LITERAL'),
    ('nested_projected_array', True, 'fun boundary() = Array<Array<out String>>::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('jvm_binary_name', False, 'fun bad(): kotlin.reflect.KClass<*> = java.util.`Map$Entry`::class', 'CLASSIFIER_DOLLAR_NAME_UNSUPPORTED'),
    ('array_arity', False, 'fun bad() = Array<String, Int>::class', 'INVALID_CLASS_LITERAL'),
    ('nullable_alias', False, 'typealias Nullable = String?\nfun bad(): kotlin.reflect.KClass<*> = Nullable::class', 'INVALID_CLASS_LITERAL'),
    ('fixed_alias', True, 'typealias Fixed = List<String>\nfun boundary() = Fixed::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('bare_generic_alias', True, 'typealias Generic<T> = List<T>\nfun boundary() = Generic::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('star_array', False, 'fun bad() = Array<*>::class', 'INVALID_CLASS_LITERAL'),
    ('unreified_type', False, 'fun <T: Any> bad() = T::class', 'INVALID_CLASS_LITERAL'),
    ('unreified_array', False, 'fun <T: Any> bad() = Array<T>::class', 'INVALID_CLASS_LITERAL'),
    ('nullable_parameter', False, 'fun <T> bad(value: T) = value::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('indirect_nullable_parameter', False, 'fun <U, T: U> bad(value: T): kotlin.reflect.KClass<*> = value::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('definitely_nonnull_parameter', True, 'fun <T> boundary(value: T): kotlin.reflect.KClass<*> = value!!::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('smartcast_parameter', True, 'fun <T> boundary(value: T): kotlin.reflect.KClass<*> { if (value != null) return value::class; return String::class }', 'CLASS_LITERAL_UNSUPPORTED'),
    ('internal_library', False, 'fun bad() = newfamilies.dependency.InternalKotlinType::class', 'INVALID_CLASS_LITERAL'),
    ('private_array', False, 'fun bad(): kotlin.reflect.KClass<*> = Array<newfamilies.dependency.PrivateKotlinType>::class', 'INVALID_CLASS_LITERAL'),
    ('internal_array', False, 'fun bad(): kotlin.reflect.KClass<*> = Array<newfamilies.dependency.InternalKotlinType>::class', 'INVALID_CLASS_LITERAL'),
    ('public_java_nested', True, 'fun boundary(): kotlin.reflect.KClass<*> = java.util.Map.Entry::class', 'CLASS_LITERAL_VISIBILITY_UNSUPPORTED'),
    ('source_internal', True, 'internal class Hidden\nfun boundary(): kotlin.reflect.KClass<*> = Hidden::class', 'CLASS_LITERAL_VISIBILITY_UNSUPPORTED'),
    ('private_library', False, 'fun bad() = newfamilies.dependency.PrivateKotlinType::class', 'INVALID_CLASS_LITERAL'),
    ('internal_outer', False, 'fun bad() = newfamilies.dependency.InternalOuter.PublicNested::class', 'INVALID_CLASS_LITERAL'),
    ('private_nested', False, 'fun bad() = newfamilies.dependency.PublicOuter.PrivateNested::class', 'INVALID_CLASS_LITERAL'),
    ('protected_nested', False, 'fun bad() = newfamilies.dependency.PublicOuter.ProtectedNested::class', 'INVALID_CLASS_LITERAL'),
    ('bare_array', True, 'fun boundary() = Array::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('projected_array', True, 'fun boundary() = Array<out String>::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('short', True, 'fun boundary() = Short::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('unit', True, 'fun boundary() = Unit::class', 'CLASS_LITERAL_UNSUPPORTED'),
    ('star_mapping', True, 'fun boundary(value: kotlin.reflect.KClass<*>) = value.java', 'PROJECTED_EXTENSION_CAPTURE_UNSUPPORTED'),
    ('nested_capture', True, 'import newfamilies.dependency.nested\nfun boundary(value: Any) = value::class.nested', 'PROJECTED_EXTENSION_CAPTURE_UNSUPPORTED'),
    ('immediate_lambda', True, 'fun boundary() = ({42})()::class.java', 'CLASS_LITERAL_REPRESENTATION_UNSUPPORTED'),
    ('annotation_class', True, 'import newfamilies.dependency.ClassMark\n@ClassMark(String::class) fun boundary() {}', 'CLASS_LITERAL_ANNOTATION_UNSUPPORTED'),
    ('file_annotation', True, '@file:Suppress("unused")\npackage marker\nfun boundary() = String::class', 'FILE_ANNOTATION_UNSUPPORTED'),
    ('unsigned_literal', True, 'fun boundary(): UInt = 1u', 'UNSIGNED_LITERAL_UNSUPPORTED'),
    ('mapped_java_string', True, 'fun boundary(value: java.lang.String) = value.bytes', 'JAVA_SYNTHETIC_RECEIVER_UNSUPPORTED'),
]
UPSTREAM = [
    'classLiteral/bound/simple.kt',
    'classLiteral/bound/smartCast.kt',
    'classLiteral/bound/sideEffect.kt',
    'classLiteral/bound/javaIntrinsicWithSideEffect.kt',
    'classLiteral/bareArray.kt',
    'classLiteral/primitiveClassEquality.kt',
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(path):
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def inventory(path):
    if path.is_file():
        return {str(path): sha(path)}
    return {str(item): sha(item) for item in sorted(path.rglob('*')) if item.is_file()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--source-snapshot', type=Path, default=ROOT)
    parser.add_argument('--app-capture', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    source = args.source_snapshot.resolve()
    fixtures = source / 'tests/fixtures/new_families/class_literals'
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-new-class-literals')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', cases=[], commands=[], inputs={}, evidence=[], selection='Source type/bound class literals create real KClass via the pinned runtime; ordinary generic extension-property metadata resolves actual Java mapping and unrelated relay getters. Physical JVM result descriptors determine boxed versus primitive class identity; direct immediate-lambda/compiled-inline representations remain explicit unsupported boundaries. No full reflection or full Android app claim. App probes use unchanged files with actual captured external dependencies and official sibling classes; source compilation only, not Android execution.')
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
    stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    compiler = work / 'korrust-frozen'
    shutil.copy2(args.compiler.resolve(), compiler)
    original = ROOT / 'references/kotlin/compiler/testData/codegen/box'
    upstream = [original / item for item in UPSTREAM]
    frozen = [source / 'Cargo.toml', source / 'Cargo.lock', source / 'tests/unsupported_syntax_diagnostics.rs', *sorted((source / 'src').rglob('*.rs')), *sorted(fixtures.glob('*')), Path(__file__).resolve(), source / 'tools/upstream_jvm.py', source / 'selection.json', *upstream]
    hashes = {relative(path): sha(path) for path in frozen}
    report['inputs']['files_sha256'] = hashes
    report['inputs']['reference_sources_manifest_sha256'] = sha(ROOT / 'references/sources.json')
    report['inputs']['lowering_sources_sha256'] = {name: sha(ROOT / name) for name in ['references/kotlin/compiler/fir/checkers/src/org/jetbrains/kotlin/fir/analysis/checkers/expression/FirClassLiteralChecker.kt', 'references/kotlin/compiler/ir/backend.jvm/codegen/src/org/jetbrains/kotlin/backend/jvm/codegen/ExpressionCodegen.kt', 'references/kotlin/compiler/ir/backend.jvm/codegen/src/org/jetbrains/kotlin/backend/jvm/intrinsics/JavaClassProperty.kt']}
    report['inputs']['spec_sha256'] = sha(ROOT / 'references/kotlin-spec/docs/src/md/kotlin.core/expressions.md')
    report['limits'] = dict(type_parameter_bound_steps=64, containing_class_steps=64, array_classifier_steps=64, ordinary_command_seconds=120, app_command_seconds=600)
    report['compiler'] = dict(path=relative(compiler), binary_sha256=sha(compiler))
    report['reference_compiler'] = dict(version=pin['version'], pin_sha256=sha(ROOT / 'references/compiler.json'), stdlib_sha256=sha(stdlib))
    report['reference_compiler']['testlib_sha256'] = sha(ROOT / pin['path'] / 'lib/kotlin-test.jar')
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_class_literals']
    report['reference_compiler']['options'] = options

    def run(label, command, timeout=120):
        print(label, flush=True)
        result = subprocess.run(list(map(str, command)), cwd=ROOT, text=True, capture_output=True, timeout=timeout)
        entry = dict(label=label, argv=list(map(str, command)), exit_code=result.returncode, timeout_seconds=timeout)
        for stream in ['stdout', 'stderr']:
            path = work / f'{len(report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
            report['evidence'].append(relative(path))
        report['commands'].append(entry)
        return result

    def require(result):
        if result.returncode:
            raise AssertionError(result.stderr[-3500:])
        return result

    def case(ident, outcome='pass', **details):
        report['cases'].append(dict(id=ident, outcome=outcome, **details))

    def cp(*items):
        return os.pathsep.join(map(str, items))

    dependency = work / 'dependencies'

    def compile_provider(provider, sources, destination, classpath=None):
        command = [kotlinc, *sources, *options] if provider == 'official' else [compiler, *sources, '--module-name', 'new_class_literals', '--diagnostic-format', 'json']
        return run('Compile ' + provider + ' ' + destination.name, [*command, '-cp', classpath or cp(dependency, stdlib), '-d', destination], timeout=600 if destination.name.startswith('app-') else 120)

    try:
        identity = require(run('Official identity', [kotlinc, '-version']))
        assert 'kotlinc-jvm ' + pin['version'] + ' ' in identity.stdout + identity.stderr
        require(run('JDK identity', [jdk / 'java', '-version']))
        require(run('Compile actual generic Java dependency', [jdk / 'javac', '-d', dependency, fixtures / 'UnsafeGenerics.java', fixtures / 'InitializationTrace.java', fixtures / 'InitializationProbe.java', fixtures / 'ClassMark.java']))
        require(run('Compile generic Kotlin property and visibility dependencies', [kotlinc, fixtures / 'GenericProperties.kt', fixtures / 'KotlinClassLibrary.kt', *options, '-module-name', 'class_literal_library', '-d', dependency]))
        report['inputs']['dependency_classes_sha256'] = inventory(dependency)
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, destination in outputs.items():
            require(compile_provider(provider, [fixtures / 'Classes.kt'], destination))
            case('compile/' + provider)
        driver = work / 'driver'
        require(run('Compile fixed Java caller against official classes', [jdk / 'javac', '-cp', cp(outputs['official'], dependency, stdlib), '-d', driver, fixtures / 'ClassesCheck.java']))
        report['inputs']['fixed_java_classes_sha256'] = inventory(driver)
        for provider, destination in outputs.items():
            result = require(run('Execute fixed Java caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(driver, destination, dependency, stdlib), 'newfamilies.classes.ClassesCheck']))
            assert result.stdout.strip() == 'checks=82', result.stdout
            case('runtime/' + provider, independent_assertions=82, jvm_verification='all')
        signatures = []
        for provider, destination in outputs.items():
            signatures.append([require(run('Public signature ' + provider + ' ' + name, [jdk / 'javap', '-public', '-s', '-classpath', destination, name])).stdout for name in ['newfamilies.classes.ClassesKt', 'newfamilies.classes.FixtureType', 'newfamilies.classes.Outer', 'newfamilies.classes.Outer$Nested']])
            require(run('Metadata attributes ' + provider, [jdk / 'javap', '-v', '-classpath', destination, 'newfamilies.classes.ClassesKt']))
        def normalized(text):
            return text.splitlines()[1], sorted(re.findall(r'^  public[^\n]*\n    descriptor: [^\n]*', text, re.M))
        assert list(map(normalized, signatures[0])) == list(map(normalized, signatures[1])), 'Public generic signatures differ'
        case('public_signatures')
        consumers = {}
        for provider, destination in outputs.items():
            consumers[provider] = work / ('consumer-' + provider)
            require(run('Compile independent official Kotlin consumer ' + provider, [kotlinc, fixtures / 'Consumer.kt', *options, '-module-name', 'class_literal_consumer', '-cp', cp(destination, dependency, stdlib), '-d', consumers[provider]]))
        consumer_driver = work / 'consumer-driver'
        require(run('Compile fixed Kotlin consumer launcher', [jdk / 'javac', '-cp', cp(consumers['official'], stdlib), '-d', consumer_driver, fixtures / 'ConsumerLauncher.java']))
        report['inputs']['fixed_consumer_driver_classes_sha256'] = inventory(consumer_driver)
        for provider, destination in outputs.items():
            result = require(run('Execute Kotlin metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer_driver, consumers[provider], destination, dependency, stdlib), 'ConsumerLauncher']))
            assert result.stdout.strip() == 'OK', result.stdout
            case('kotlin_consumer/' + provider, independently_compiled_against_provider=True, jvm_verification='all')
        integrated = {provider: work / ('integrated-' + provider) for provider in ['official', 'candidate']}
        for provider, destination in integrated.items():
            require(compile_provider(provider, [fixtures / 'Integration.kt'], destination))
        integrated_driver = work / 'integrated-driver'
        require(run('Compile fixed array/generic integration caller', [jdk / 'javac', '-cp', cp(integrated['official'], stdlib), '-d', integrated_driver, fixtures / 'IntegrationCheck.java']))
        report['inputs']['fixed_integration_java_classes_sha256'] = inventory(integrated_driver)
        integrated_signatures = []
        for provider, destination in integrated.items():
            result = require(run('Execute fixed array/generic integration caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(integrated_driver, destination, stdlib), 'newfamilies.integration.IntegrationCheck']))
            assert result.stdout.strip() == 'checks=19', result.stdout
            case('integration/runtime/' + provider, independent_assertions=19, jvm_verification='all')
            integrated_signatures.append([require(run('Integration signature ' + provider + ' ' + name, [jdk / 'javap', '-public', '-s', '-classpath', destination, name])).stdout for name in ['newfamilies.integration.IntegrationKt', 'newfamilies.integration.SourceContract', 'newfamilies.integration.SourceScalar']])
        assert list(map(normalized, integrated_signatures[0])) == list(map(normalized, integrated_signatures[1])), 'Integrated generic signatures differ'
        case('integration/public_signatures')
        integrated_consumers = {}
        for provider, destination in integrated.items():
            integrated_consumers[provider] = work / ('integrated-consumer-' + provider)
            require(run('Compile official integration consumer ' + provider, [kotlinc, fixtures / 'IntegrationConsumer.kt', *options, '-cp', cp(destination, stdlib), '-d', integrated_consumers[provider]]))
        integrated_launcher = work / 'integrated-launcher'
        require(run('Compile fixed integration consumer launcher', [jdk / 'javac', '-cp', cp(integrated_consumers['official'], stdlib), '-d', integrated_launcher, fixtures / 'IntegrationLauncher.java']))
        report['inputs']['fixed_integration_launcher_classes_sha256'] = inventory(integrated_launcher)
        for provider, destination in integrated.items():
            result = require(run('Execute integration consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(integrated_launcher, integrated_consumers[provider], destination, stdlib), 'IntegrationLauncher']))
            assert result.stdout.strip() == 'OK', result.stdout
            case('integration/kotlin_consumer/' + provider, jvm_verification='all')
        known_invalid = work / 'ConfusingBooleanWhen.kt'
        known_invalid.write_text('fun existing(value: Any?): Int = when (false) { value !is String -> value.length; else -> 0 }\n')
        known_oracle = compile_provider('official', [known_invalid], work / 'known-invalid-official')
        known_candidate = compile_provider('candidate', [known_invalid], work / 'known-invalid-candidate')
        assert known_oracle.returncode, 'Pinned Boolean-when diagnostic changed'
        case('existing_assessment/confusing_boolean_when', outcome='correctness_failure' if known_candidate.returncode == 0 else 'invalid_kotlin',
             scope='existing_behavior_assessment', official_accepts=False,
             candidate_accepts=known_candidate.returncode == 0,
             note='Existing unparenthesized logical branch syntax acceptance; outside the class-literal feature claims. Valid regression fixtures retain required parentheses.')
        generated = {relative(known_invalid): sha(known_invalid)}
        for name, valid, text, code in BOUNDARIES:
            path = work / (name + '.kt')
            path.write_text(text + '\n')
            generated[relative(path)] = sha(path)
            oracle = compile_provider('official', [path], work / ('official-' + name))
            destination = work / ('candidate-' + name)
            actual = compile_provider('candidate', [path], destination)
            assert bool(oracle.returncode) != valid, name + ': oracle classification changed'
            assert actual.returncode and not list(destination.rglob('*.class')), name + ': must reject without classes'
            assert code in actual.stderr, name + ': ' + actual.stderr
            category = 'unsupported_language' if valid or name in {'nullable_parameter', 'indirect_nullable_parameter', 'jvm_binary_name'} else 'invalid_kotlin'
            assert category in actual.stderr, name + ': ' + actual.stderr
            case('boundary/' + name, outcome='unsupported_language' if valid else 'invalid_kotlin', official_accepts=valid, candidate_category=category, candidate_code=code, classification_gap=not valid and category == 'unsupported_language')
        report['inputs']['generated_sources_sha256'] = generated
        report['selected_status'] = 'pass'
        adapter_logs = work / 'upstream-commands'
        adapter_logs.mkdir()
        adapter_commands = Commands(adapter_logs, os.environ.copy())
        adapter_context = {'directory': work / 'upstream-originals', 'commands': adapter_commands,
                           'candidate': compiler, 'official': kotlinc, 'stdlib': stdlib,
                           'testlib': ROOT / pin['path'] / 'lib/kotlin-test.jar',
                           **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        for path in upstream:
            ident = str(path.relative_to(original))
            entry = {'id': 'upstream/' + ident.replace('/', '__'), 'family': 'class_literals',
                     'root': 'box', 'kind': 'box', 'tier': 'original',
                     'path': str(path.relative_to(ROOT)), 'sha256': sha(path)}
            result = run_case(entry, adapter_context)
            result['original_fixture'] = ident
            report['cases'].append(result)
        report['commands'].extend(adapter_commands.entries)
        for command in adapter_commands.entries:
            report['evidence'].extend([command['stdout'], command['stderr']])
        capture = json.loads(args.app_capture.read_text())
        task = capture['modules']['Alkitab']['compilation_tasks']['compilePlainDebugKotlin']
        absent = [item for item in task['compile_classpath'] if not Path(item).exists()]
        assert all(item.endswith('/generated/ksp/plainDebug/classes') for item in absent), absent
        external = [Path(item) for item in task['compile_classpath'] if Path(item).exists()]
        app_build = args.app_capture.parent / 'modules/Alkitab/module-build'
        siblings = [app_build / 'intermediates/built_in_kotlinc/plainDebug/compilePlainDebugKotlin/classes', app_build / 'intermediates/javac/plainDebug/compilePlainDebugJavaWithJavac/classes']
        assert all(item.is_dir() for item in siblings)
        app_classpath = external + siblings
        app_hashes = {key: value for item in app_classpath for key, value in inventory(item).items()}
        report['inputs']['app_probe'] = dict(capture=str(args.app_capture), capture_sha256=sha(args.app_capture), capture_status=capture['status'], actual_task=task['task'], task_compiler_options=task['compiler_options'], per_file_options=options, classpath=list(map(str, app_classpath)), absent_optional_captured_paths=absent, absent_policy='Same existing dependency paths supplied to both per-file compilers; original task tolerated this absent generated directory. This is not a replay of the full production task.', dependency_files_sha256=app_hashes, scope='Actual external Android dependencies plus official compiled application siblings; unchanged single source file compilation, no Android execution')
        app_source = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/br/SecretSettingsReceiver.kt'
        report['inputs']['app_probe']['source_sha256'] = sha(app_source)
        require(compile_provider('official', [app_source], work / 'app-official', cp(*app_classpath)))
        actual = compile_provider('candidate', [app_source], work / 'app-candidate', cp(*app_classpath))
        if actual.returncode:
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            outcome = 'unsupported_language' if diagnostics and all(item['category'] == 'unsupported_language' for item in diagnostics) else 'correctness_failure'
            case('app/SecretSettingsReceiver.kt', outcome=outcome, diagnostics=diagnostics, original_source_unchanged=True, compiled=False)
        else:
            abi = [require(run('Inspect actual app source signatures ' + provider, [jdk / 'javap', '-public', '-s', '-classpath', work / ('app-' + provider), 'yuku.alkitab.base.br.SecretSettingsReceiver'])).stdout for provider in outputs]
            assert abi[0] == abi[1]
            case('app/SecretSettingsReceiver.kt', original_source_unchanged=True, compiled=True, signatures_match=True, runtime='Not executed; real Android dependencies and official application siblings supply per-file compile context')
        after_app = {key: value for item in app_classpath for key, value in inventory(item).items()}
        assert after_app == app_hashes, 'Actual application dependency inputs changed during run'
        assert hashes == {relative(path): sha(path) for path in frozen}, 'Frozen compiler/fixture/harness inputs changed'
        report['input_integrity'] = {'status': 'unchanged', 'compiler_binary_sha256': sha(compiler)}
        report['status'] = 'complete_with_external_assessment_failure' if any(item['outcome'] in {'correctness_failure', 'oracle_failure', 'timeout', 'dependency_failure', 'setup_failure'} for item in report['cases']) else 'pass'
    except Exception as error:
        report['status'] = 'fail'
        report['error'] = str(error)
        print(str(error), flush=True)
    report['summary'] = dict(Counter(item['outcome'] for item in report['cases']))
    output = work / 'report.json'
    output.write_text(json.dumps(report, indent=2) + '\n')
    print(output, flush=True)
    raise SystemExit(0 if report['status'] == 'pass' else 1)


if __name__ == '__main__':
    main()
