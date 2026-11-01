#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import zipfile
from upstream_jvm import Commands, run_case

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    work = args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    (work / 'commands').mkdir()
    commands = Commands(work / 'commands', dict(os.environ))
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    compiler = args.candidate.resolve()
    lib = ROOT / 'references/kotlinc/lib'
    standard = lib / 'kotlin-stdlib.jar'
    oracle = ROOT / 'references/kotlinc/bin/kotlinc'
    fixtures = ROOT / 'tests/value_classes'
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(),
              'status': 'running', 'cases': [], 'commands': commands.entries,
              'compiler': {'path': str(compiler), 'sha256': sha(compiler)},
              'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(lib / 'kotlin-compiler.jar'), 'stdlib_sha256': sha(standard)},
              'environment': {'JAVA_HOME': os.environ['JAVA_HOME']},
              'claims': 'Bounded single-field value-class source and imported JVM ABI with boxed internal temporaries; no optimization, full reflection, Android runtime or full-app claim.',
              'abi_scope': 'Exact reflection signatures/modifiers/generic signatures of public/protected nonsynthetic methods, carrier fields, required box/unbox/constructor/default helpers and marker constructors. Additional private synthetic implementation members are excluded and disclosed.',
              'limitations': ['Source carriers Int, Long, String and nullable String; nongeneric declarations and nongeneric interfaces.',
                              'Generic or nested source value classifiers, secondary/default/explicit constructor visibility, companions, parameterized supertypes and decorated/inline/generic/vararg member shapes are explicit boundaries.',
                              'Source interface/object/companion signatures and enum storage containing value classes remain explicit boundaries. Physical carrier calls in those emitter contexts are rejected; direct boxed value construction/getters returning ordinary types are exercised. Package value properties remain unsupported; generic collections and ordinary class/data-class consumers are exercised.',
                              'Inner/enum/defaulted/secondary value-consuming constructors remain explicit boundaries. Unsigned source semantics are a separate unaccepted family; imported value carrier metadata supplies a reusable foundation.'],
              'reference_discrepancy': {'spec': 'references/kotlin-spec/docs/src/md/kotlin.core/declarations.md:710-739',
                                        'observed': 'Pinned JVM compiler default toString uses Wrapper(property=value), permits nonpublic carrier properties and pure box-impl does not rerun init. Older spec text describes property-delegating toString, public carriers and possible primary-constructor boxing. Match pinned JVM behavior and record the discrepancy.'}}
    frozen_files = sorted([path for directory in ['src', 'tests/value_classes'] for path in (ROOT / directory).rglob('*') if path.is_file()] + [Path(__file__), ROOT / 'tools/upstream_jvm.py', ROOT / 'tests/value_classes.rs', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock'])
    upstream = ['kt54656.kt', 'kt51353.kt', 'kt51672.kt', 'kt51254.kt', 'boxImplDoesNotExecuteInitBlock.kt']
    originals = [ROOT / 'references/kotlin/compiler/testData/codegen/box/inlineClasses' / name for name in upstream]
    app_sources = [ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/datatransfer/model' / name for name in ['Gid.kt', 'Rpp.kt']]
    frozen = {str(path): sha(path) for path in [*frozen_files, *originals, *app_sources, ROOT / 'references/compiler.json', ROOT / 'references/sources.json', ROOT / 'references/kotlin/compiler/tests-common-new/testFixtures/org/jetbrains/kotlin/test/preprocessors/JvmInlineSourceTransformer.kt', compiler, standard, lib / 'kotlin-compiler.jar']}
    report['frozen_input_hashes'] = frozen
    with zipfile.ZipFile(work / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in frozen_files:
            archive.write(path, str(path.relative_to(ROOT)))
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def run(label, argv, timeout=240):
        result = commands.run(label, argv, timeout=timeout)
        if result is None or result.returncode:
            raise RuntimeError(label + ': ' + (result.stderr if result else 'timeout'))
        return result.stdout.strip()
    def case(name, **values):
        report['cases'].append({'id': name, 'outcome': 'pass', **values})
    def compile_sources(label, sources, dest, dependencies):
        provider = oracle if label == 'official' else compiler
        argv = [provider, *sources, '-cp', cp(*dependencies), '-d', dest]
        argv += ['-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'] if label == 'official' else ['--diagnostic-format', 'json']
        run('Compile ' + label + '/' + dest.name, argv)
    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Official compiler identity', [oracle, '-version'])
        destinations = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, dest in destinations.items():
            compile_sources(provider, [fixtures / 'Values.kt'], dest, [standard])
            case('compile/' + provider)
        java = work / 'java'
        run('Compile fixed Java caller against official output', [jdk / 'javac', '-cp', cp(destinations['official'], standard), '-d', java, fixtures / 'Check.java'])
        java_hashes = {str(path): sha(path) for path in java.rglob('*.class')}
        for provider, dest in destinations.items():
            output = run('Run fixed caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(java, dest, standard), 'Check'])
            if output != 'OK 139':
                raise RuntimeError(output)
            case('runtime/' + provider, independent_assertions=139, output=output)
        if java_hashes != {str(path): sha(path) for path in java.rglob('*.class')}:
            raise RuntimeError('Fixed Java caller changed')
        case('fixed_caller_integrity', class_hashes=java_hashes)
        metadata = work / 'metadata'
        run('Compile metadata and ABI readers', [jdk / 'javac', '-cp', cp(standard, lib / 'kotlin-compiler.jar'), '-d', metadata, fixtures / 'MetadataDump.java', fixtures / 'AbiDump.java'])
        value_owners = ['valuecases.' + name for name in ['Id', 'Wide', 'Text', 'MaybeText', 'Tracked', 'Secret']]
        owners = value_owners + ['valuecases.Holder', 'valuecases.Pair', 'valuecases.Mixed', 'valuecases.ValuesKt']
        for kind, main, selected in [('metadata', 'MetadataDump', value_owners), ('abi', 'AbiDump', owners)]:
            snapshots = {provider: run('Read ' + kind + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(metadata, dest, standard, lib / 'kotlin-compiler.jar'), main, *selected]) for provider, dest in destinations.items()}
            if snapshots['official'] != snapshots['candidate']:
                raise RuntimeError(kind + ' mismatch: ' + repr(snapshots))
            case(kind, owners=selected, snapshot=snapshots['official'])
        for against, dependency in destinations.items():
            consumer = work / ('consumer-' + against)
            compile_sources('official', [fixtures / 'Consumer.kt'], consumer, [dependency, standard])
            for provider, dest in destinations.items():
                output = run('Run Kotlin consumer compiled against ' + against + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, dest, standard), 'valueconsumer.ConsumerKt'])
                if output != 'OK':
                    raise RuntimeError(output)
                case('kotlin_consumer/' + against + '/' + provider)
        imported = {provider: work / ('imported-' + provider) for provider in ['official', 'candidate']}
        for provider, dest in imported.items():
            compile_sources(provider, [fixtures / 'Imported.kt'], dest, [destinations['official'], standard])
            case('imported_compile/' + provider)
        imported_java = work / 'imported-java'
        run('Compile imported fixed Java caller', [jdk / 'javac', '-cp', cp(imported['official'], destinations['official'], standard), '-d', imported_java, fixtures / 'ImportedCheck.java'])
        for provider, dest in imported.items():
            output = run('Run imported calls ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(imported_java, dest, destinations['official'], standard), 'ImportedCheck'])
            if output != 'OK 5':
                raise RuntimeError(output)
            case('imported_runtime/' + provider, independent_assertions=5)
        app = {provider: work / ('app-' + provider) for provider in ['official', 'candidate']}
        for provider, dest in app.items():
            compile_sources(provider, app_sources, dest, [standard])
            case('unchanged_app_compile/' + provider, sources=[str(path) for path in app_sources])
        app_consumer = work / 'app-consumer'
        app_source = fixtures / 'AppConsumer.kt'
        for against, dependency in app.items():
            consumer = app_consumer / against
            compile_sources('official', [app_source], consumer, [dependency, standard])
            for provider, dest in app.items():
                output = run('Run unchanged app consumer ' + against + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, dest, standard), 'AppConsumerKt'])
                if output != 'OK':
                    raise RuntimeError(output)
                case('unchanged_app_consumer/' + against + '/' + provider)
        direct_contexts = [
            ('object', 'object O { fun body(): Int { val value = V(7); return value.raw } }', 'directcontext.O.INSTANCE.body()', 7),
            ('companion', 'class Host { companion object { fun body(): Int { val value = V(11); return value.raw } } }', 'directcontext.Host.Companion.body()', 11),
            ('enum_method', 'enum class E { A; fun body(): Int { val value = V(13); return value.raw } }', 'directcontext.E.A.body()', 13),
            ('enum_initializer', 'enum class E(val code: Int) { A(V(17).raw) }', 'directcontext.E.A.getCode()', 17),
            ('interface_default', 'interface I { fun body(): Int { val value = V(19); return value.raw } }\nclass Impl : I', 'new directcontext.Impl().body()', 19),
        ]
        for name, body, invocation, expected in direct_contexts:
            directory = work / ('direct-context-' + name)
            directory.mkdir()
            source = directory / 'Context.kt'
            source.write_text('package directcontext\n@JvmInline value class V(val raw: Int)\n' + body + '\n')
            driver = directory / 'ContextCheck.java'
            driver.write_text('public final class ContextCheck { public static void main(String[] args) { if (' + invocation + ' != ' + str(expected) + ') throw new AssertionError(); } }\n')
            outputs = {provider: directory / provider for provider in ['official', 'candidate']}
            for provider, output in outputs.items():
                compile_sources(provider, [source], output, [standard])
            callers = directory / 'callers'
            run('Compile direct-context fixed caller/' + name, [jdk / 'javac', '-cp', cp(outputs['official'], standard), '-d', callers, driver])
            caller_hash = sha(callers / 'ContextCheck.class')
            for provider, output in outputs.items():
                run('Run direct-context fixed caller/' + name + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, output, standard), 'ContextCheck'])
                case('direct_context/' + name + '/' + provider, source_sha256=sha(source), expected_result=expected, fixed_java_caller_sha256=caller_hash)
            if sha(callers / 'ContextCheck.class') != caller_hash:
                raise RuntimeError('Direct-context Java caller changed: ' + name)
        boundaries = [
            ('object_physical_call', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nobject O { fun body(): Int = read(V(1)) }', 'VALUE_OBJECT_MEMBER_UNSUPPORTED'),
            ('object_physical_initializer', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nobject O { val code: Int = read(V(1)) }', 'VALUE_OBJECT_MEMBER_UNSUPPORTED'),
            ('companion_physical_call', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nclass Host { companion object { fun body(): Int = read(V(1)) } }', 'VALUE_COMPANION_MEMBER_UNSUPPORTED'),
            ('companion_physical_initializer', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nclass Host { companion object { val code: Int = read(V(1)) } }', 'VALUE_COMPANION_MEMBER_UNSUPPORTED'),
            ('interface_physical_call', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\ninterface I { fun body(): Int = read(V(1)) }', 'VALUE_INTERFACE_SIGNATURE_UNSUPPORTED'),
            ('interface_companion_physical_initializer', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\ninterface I { companion object { val code: Int = read(V(1)) } }', 'VALUE_INTERFACE_SIGNATURE_UNSUPPORTED'),
            ('enum_physical_call', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nenum class E { A; fun body(): Int = read(V(1)) }', 'VALUE_ENUM_CONTEXT_UNSUPPORTED'),
            ('enum_physical_initializer', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nenum class E(val code: Int) { A(read(V(1))) }', 'VALUE_ENUM_CONTEXT_UNSUPPORTED'),
            ('enum_physical_default', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nenum class E(val code: Int = read(V(1))) { A }', 'VALUE_ENUM_CONTEXT_UNSUPPORTED'),
            ('enum_entry_physical_call', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nenum class E { A { override fun body(): Int = read(V(1)) }; abstract fun body(): Int }', 'VALUE_ENUM_CONTEXT_UNSUPPORTED'),
            ('enum_companion_physical_closure', True, '@JvmInline value class V(val raw: Int)\nfun read(value: V): Int = value.raw\nenum class E { A; companion object { fun body(): Int { val result: () -> Int = { read(V(1)) }; return result() } } }', 'VALUE_ENUM_CONTEXT_UNSUPPORTED'),
            ('inner_value_modifier', False, 'class Outer { @JvmInline inner value class Bad(val raw: Int) }', 'VALUE_MODIFIERS_INVALID'),
            ('inner_consumer', True, '@JvmInline value class V(val raw: Int)\nclass Outer { inner class Holder(val value: V) }', 'VALUE_CONSUMER_CONSTRUCTOR_SHAPE_UNSUPPORTED'),
            ('enum_consumer', True, '@JvmInline value class V(val raw: Int)\nenum class E(val value: V) { A(V(1)) }', 'VALUE_CONSUMER_CONSTRUCTOR_SHAPE_UNSUPPORTED'),
            ('missing_marker', False, 'value class Bad(val raw: Int)', 'VALUE_JVM_INLINE_REQUIRED'),
            ('marker_on_ordinary', False, '@JvmInline class Bad(val raw: Int)', 'VALUE_JVM_INLINE_PLACEMENT_INVALID'),
            ('shadow_marker', False, 'annotation class JvmInline\n@JvmInline value class Bad(val raw:Int)', 'VALUE_JVM_INLINE_REQUIRED'),
            ('mutable_carrier', False, '@JvmInline value class Bad(var raw: Int)', 'VALUE_CARRIER_INVALID'),
            ('plain_parameter', False, '@JvmInline value class Bad(raw: Int)', 'VALUE_CARRIER_INVALID'),
            ('two_carriers', False, '@JvmInline value class Bad(val raw: Int, val second: Int)', 'VALUE_CARRIER_INVALID'),
            ('data_modifier', False, '@JvmInline data value class Bad(val raw: Int)', 'VALUE_MODIFIERS_INVALID'),
            ('backing_field', False, '@JvmInline value class Bad(val raw: Int) { val other = 2 }', 'VALUE_STORAGE_INVALID'),
            ('reserved_hash', False, '@JvmInline value class Bad(val raw: Int) { override fun hashCode(): Int = 1 }', 'VALUE_RESERVED_METHOD'),
            ('identity', False, '@JvmInline value class Bad(val raw: Int)\nfun bad(a: Bad,b: Bad): Boolean = a === b', 'VALUE_IDENTITY_EQUALITY_INVALID'),
            ('generic_carrier', True, '@JvmInline value class Boundary<T>(val raw: T)', 'VALUE_DECLARATION_SHAPE_UNSUPPORTED'),
            ('default_carrier', True, '@JvmInline value class Boundary(val raw: Int = 1)', 'VALUE_DECLARATION_SHAPE_UNSUPPORTED'),
            ('secondary', True, '@JvmInline value class Boundary(val raw: Int) { constructor(): this(1) }', 'VALUE_DECLARATION_SHAPE_UNSUPPORTED'),
            ('companion', True, '@JvmInline value class Boundary(val raw: Int) { companion object {} }', 'VALUE_DECLARATION_SHAPE_UNSUPPORTED'),
            ('array_carrier', True, '@JvmInline value class Boundary(val raw: IntArray)', 'VALUE_CARRIER_UNSUPPORTED'),
            ('float_carrier', True, '@JvmInline value class Boundary(val raw: Float)', 'VALUE_CARRIER_UNSUPPORTED'),
            ('nullable_int_carrier', True, '@JvmInline value class Boundary(val raw: Int?)', 'VALUE_CARRIER_UNSUPPORTED'),
            ('private_constructor', True, '@JvmInline value class Boundary private constructor(val raw: Int)', 'VALUE_CONSTRUCTOR_VISIBILITY_UNSUPPORTED'),
            ('inline_member', True, '@JvmInline value class Boundary(val raw: Int) { inline fun read(): Int = raw }', 'SOURCE_INLINE_CONTEXT_UNSUPPORTED'),
            ('generic_interface', True, 'interface I<T>\n@JvmInline value class Boundary(val raw: Int): I<String>', 'VALUE_GENERIC_SUPERTYPE_UNSUPPORTED'),
            ('interface_signature', True, '@JvmInline value class V(val raw: Int)\ninterface I { fun f(value: V): V }', 'VALUE_INTERFACE_SIGNATURE_UNSUPPORTED'),
            ('object_signature', True, '@JvmInline value class V(val raw: Int)\nobject O { val value: V = V(1) }', 'VALUE_OBJECT_MEMBER_UNSUPPORTED'),
            ('top_property', True, '@JvmInline value class V(val raw: Int)\nval value: V = V(1)', 'VALUE_PACKAGE_PROPERTY_UNSUPPORTED'),
            ('nested_classifier', True, 'class Outer { @JvmInline value class V(val raw: Int) }', 'VALUE_NESTED_DECLARATION_UNSUPPORTED'),
            ('generic_value_bound', True, '@JvmInline value class V(val raw: Int)\nfun <T: V> identity(value:T):T = value', 'VALUE_GENERIC_BOUND_UNSUPPORTED'),
            ('consumer_defaults', True, '@JvmInline value class V(val raw: Int)\nclass Holder(val value: V = V(1))', 'VALUE_CONSUMER_CONSTRUCTOR_SHAPE_UNSUPPORTED'),
        ]
        for name, valid, source, code in boundaries:
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            official = commands.run('Boundary official/' + name, [oracle, path, '-cp', standard, '-d', work / ('oracle-' + name)])
            candidate = commands.run('Boundary candidate/' + name, [compiler, path, '-cp', standard, '-d', work / ('candidate-' + name), '--diagnostic-format', 'json'])
            if official is None or (official.returncode == 0) != valid:
                raise RuntimeError(name + ': unexpected official classification')
            category = 'unsupported_language' if valid else 'invalid_kotlin'
            if candidate is None or candidate.returncode == 0 or category not in candidate.stderr or code not in candidate.stderr:
                raise RuntimeError(name + ': wrong candidate classification ' + (candidate.stderr if candidate else 'timeout'))
            report['cases'].append({'id': 'boundary/' + name, 'outcome': category, 'official_accepts': valid, 'candidate_code': code, 'source': str(path), 'source_sha256': sha(path)})
        context = {'directory': work / 'upstream', 'commands': commands, 'candidate': compiler, 'official': oracle, 'stdlib': standard, 'testlib': lib / 'kotlin-test.jar', **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        for path in originals:
            entry = {'id': path.stem, 'family': 'single_field_values', 'root': 'box', 'tier': 'selected', 'kind': 'box', 'path': str(path.relative_to(ROOT)), 'sha256': sha(path)}
            result = run_case(entry, context)
            result['acceptance_scope'] = 'unchanged_original_assessment'
            report['cases'].append(result)
        rejected_outcomes = [case['id'] for case in report['cases'] if case['outcome'] not in ['pass', 'invalid_kotlin', 'unsupported_language', 'harness_unsupported', 'excluded_target']]
        report['status'] = 'failed' if rejected_outcomes else 'complete'
        if rejected_outcomes:
            report['error'] = 'Unaccepted correctness outcomes: ' + ', '.join(rejected_outcomes)
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
    changed = [path for path, value in frozen.items() if not Path(path).is_file() or sha(path) != value]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    if changed:
        report['status'] = 'invalidated'
    report['counts'] = dict(sorted(Counter(case['outcome'] for case in report['cases']).items()))
    (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'counts': report['counts'], 'error': report.get('error', '')[:1200]}, indent=2))
    if report['status'] != 'complete':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
