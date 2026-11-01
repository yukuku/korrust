#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import zipfile
from upstream_jvm import Commands, run_case

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def privatize_method(path, target):
    data = bytearray(path.read_bytes())
    cursor = 8
    def word():
        nonlocal cursor
        value = struct.unpack_from('>H', data, cursor)[0]
        cursor += 2
        return value
    names = {}
    count = word()
    index = 1
    while index < count:
        tag = data[cursor]
        cursor += 1
        if tag == 1:
            size = word()
            names[index] = bytes(data[cursor:cursor + size])
            cursor += size
        elif tag in [3, 4, 9, 10, 11, 12, 17, 18]:
            cursor += 4
        elif tag in [5, 6]:
            cursor += 8
            index += 1
        elif tag in [7, 8, 16, 19, 20]:
            cursor += 2
        elif tag == 15:
            cursor += 3
        else:
            raise ValueError('constant tag ' + str(tag))
        index += 1
    if target is None:
        flags = struct.unpack_from('>H', data, cursor)[0]
        struct.pack_into('>H', data, cursor, flags & ~1)
        path.write_bytes(data)
        return
    cursor += 6
    interfaces = word()
    cursor += 2 * interfaces
    def skip_attributes():
        nonlocal cursor
        for _ in range(word()):
            cursor += 2
            size = struct.unpack_from('>I', data, cursor)[0]
            cursor += 4 + size
    for _ in range(word()):
        cursor += 6
        skip_attributes()
    changed = False
    for _ in range(word()):
        flags_offset = cursor
        flags = word()
        name = names[word()]
        word()
        if name == target.encode('utf-8'):
            struct.pack_into('>H', data, flags_offset, (flags & ~7) | 2)
            changed = True
        skip_attributes()
    if not changed:
        raise ValueError('method absent ' + target)
    path.write_bytes(data)


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
    candidate = args.candidate.resolve()
    lib = ROOT / 'references/kotlinc/lib'
    standard = lib / 'kotlin-stdlib.jar'
    oracle = ROOT / 'references/kotlinc/bin/kotlinc'
    fixtures = ROOT / 'tests/unsigned'
    originals = ['boxedUnsignedEqualsZero.kt', 'literalEqualsNullableUnsigned.kt', 'nullableUnsignedEqualsLiteral.kt', 'unsignedLiteralsForMaxLongValue.kt', 'unsignedLongToString.kt', 'unsignedIntToString.kt', 'checkBasicUnsignedLiterals.kt', 'unsignedToSignedConversion.kt', 'generic.kt']
    upstream = [ROOT / 'references/kotlin/compiler/testData/codegen/box/unsignedTypes' / name for name in originals]
    app = ROOT / 'benchmarks/sources/androidbible/AlkitabYes2/src/test/java/yuku/alkitab/yes2/util/UnsignedBinarySearchKtTest.kt'
    frozen_files = sorted([path for directory in ['src', 'tests/unsigned'] for path in (ROOT / directory).rglob('*') if path.is_file()] + [Path(__file__), ROOT / 'tools/upstream_jvm.py', ROOT / 'tests/unsigned.rs', ROOT / 'tests/unsupported_syntax_diagnostics.rs', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock'])
    frozen = {str(path): sha(path) for path in [*frozen_files, *upstream, app, candidate, standard, lib / 'kotlin-compiler.jar', ROOT / 'references/compiler.json', ROOT / 'references/sources.json']}
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'cases': [], 'commands': commands.entries,
              'compiler': {'path': str(candidate), 'sha256': sha(candidate)}, 'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(lib / 'kotlin-compiler.jar'), 'stdlib_sha256': sha(standard)},
              'environment': {'JAVA_HOME': os.environ['JAVA_HOME']}, 'frozen_input_hashes': frozen,
              'claims': 'Bounded UInt and ULong scalar literals, binary arithmetic, comparison, bitwise operators, integral conversions and nullable/generic boxing with real Kotlin value-class metadata and JVM mangling. Internal temporaries remain boxed; no optimization, Android runtime or full-app claim.',
              'limitations': ['UByte/UShort have imported scalar ABI and conversion coverage; contextual literals remain an explicit boundary.', 'Unsigned const evaluation, unsigned arrays/ranges, generic value-class bounds, and previously bounded value-class source contexts remain unsupported.', 'Only scalar compiled static value members are expanded; function/reified parameters and inaccessible nested JVM references are rejected.'],
              'inline_reference_policy': 'Expand the pinned method Code attribute, checking every referenced class/member for JVM accessibility at the call site. No library operation body is substituted by name.',
              'reference_discrepancy': {'spec': 'references/kotlin-spec/docs/src/md/kotlin.core/builtins.md:77-79', 'observed': 'Pinned spec still lists unsigned integer types as a TODO. Kotlin 2.3.21 compiler/stdlib metadata and actual JVM bytecode define the tested version-specific ABI.'},
              'app_assessment': {'path': str(app), 'sha256': sha(app), 'outcome': 'not_covered', 'reason': 'Actual unsigned app usage requires UIntArray and sorted arrays in the JUnit test; those semantics are outside this scalar family. No unchanged AndroidBible file pass is claimed.'}}
    with zipfile.ZipFile(work / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in frozen_files:
            archive.write(path, str(path.relative_to(ROOT)))
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def run(label, argv, timeout=300):
        result = commands.run(label, argv, timeout=timeout)
        if result is None or result.returncode:
            raise RuntimeError(label + ': ' + (result.stderr if result else 'timeout'))
        return result.stdout.strip()
    def case(name, **values):
        report['cases'].append({'id': name, 'outcome': 'pass', **values})
    def compile(provider, sources, dest, dependencies):
        binary = oracle if provider == 'official' else candidate
        argv = [binary, *sources, '-cp', cp(*dependencies), '-d', dest]
        argv += ['-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'] if provider == 'official' else ['--diagnostic-format', 'json']
        run('Compile ' + provider + '/' + dest.name, argv, timeout=600)
    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Official compiler identity', [oracle, '-version'])
        destinations = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, dest in destinations.items():
            compile(provider, [fixtures / 'Scalars.kt'], dest, [standard])
            case('compile/' + provider)
        java = work / 'java'
        run('Compile fixed Java caller against official output', [jdk / 'javac', '-cp', cp(destinations['official'], standard), '-d', java, fixtures / 'Check.java', fixtures / 'AbiDump.java'])
        caller_hashes = {str(path): sha(path) for path in java.rglob('*.class')}
        for provider, dest in destinations.items():
            output = run('Run fixed Java caller/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(java, dest, standard), 'Check'])
            if output != 'OK 950':
                raise RuntimeError(output)
            case('runtime/' + provider, independent_assertions=950, output=output)
        signatures = {provider: run('Read ABI/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(java, dest, standard), 'AbiDump', 'unsignedcases.ScalarsKt', 'unsignedcases.ScalarBox', 'unsignedcases.ScalarPair']) for provider, dest in destinations.items()}
        if signatures['official'] != signatures['candidate']:
            raise RuntimeError('Unsigned physical/generic ABI signatures differ')
        case('abi', owners=3, scope='Public/protected nonsynthetic methods, carrier fields, physical/marker constructors and value default helpers; additional private synthetic implementations disclosed.')
        for against, dependency in destinations.items():
            consumer = work / ('consumer-against-' + against)
            compile('official', [fixtures / 'Consumer.kt'], consumer, [dependency, standard])
            for provider, dest in destinations.items():
                output = run('Run Kotlin consumer compiled against ' + against + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, dest, standard), 'unsignedconsumer.ConsumerKt'])
                if output != 'OK 15':
                    raise RuntimeError(output)
                case('kotlin_consumer/' + against + '/' + provider, assertions=15)
        dependency = work / 'dependency'
        compile('official', [fixtures / 'dependency/Counter.kt'], dependency, [standard])
        run('Inspect arbitrary compiled-inline dependency', [jdk / 'javap', '-p', '-c', '-classpath', dependency, 'externalvalue.Counter', 'externalvalue.CounterKt'])
        imported = {provider: work / ('imported-' + provider) for provider in destinations}
        for provider, dest in imported.items():
            compile(provider, [fixtures / 'Imported.kt'], dest, [dependency, standard])
            case('arbitrary_inline_compile/' + provider)
        imported_java = work / 'imported-java'
        run('Compile arbitrary inline fixed Java caller', [jdk / 'javac', '-cp', cp(imported['official'], dependency, standard), '-d', imported_java, fixtures / 'ImportedCheck.java'])
        for provider, dest in imported.items():
            output = run('Run arbitrary inline caller/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(imported_java, dest, dependency, standard), 'ImportedCheck'])
            if output != 'OK 4':
                raise RuntimeError(output)
            case('arbitrary_inline_runtime/' + provider, independent_assertions=4)
        adversarial = work / 'inaccessible-dependency'
        shutil.copytree(dependency, adversarial)
        patched = adversarial / 'externalvalue/CounterKt.class'
        before = sha(patched)
        privatize_method(patched, 'foldBits')
        result = commands.run('Reject nested inaccessible inline reference', [candidate, fixtures / 'Imported.kt', '-cp', cp(adversarial, standard), '-d', work / 'inaccessible-output', '--diagnostic-format', 'json'])
        if result is None or result.returncode == 0 or 'COMPILED_INLINE_BODY_UNSUPPORTED' not in result.stderr or 'inaccessible JVM reference' not in result.stderr:
            raise RuntimeError('Nested inaccessible reference was not rejected: ' + (result.stderr if result else 'timeout'))
        report['cases'].append({'id': 'nested_inaccessible_reference', 'outcome': 'unsupported_language', 'candidate_code': 'COMPILED_INLINE_BODY_UNSUPPORTED', 'scope': 'Adversarial JVM access-flag mutation of generated dependency; not an ordinary Kotlin oracle-valid source case.', 'original_class_sha256': before, 'mutated_class_sha256': sha(patched)})
        array_adversarial = work / 'inaccessible-array-dependency'
        shutil.copytree(dependency, array_adversarial)
        component = array_adversarial / 'externalvalue/Token.class'
        component_before = sha(component)
        privatize_method(component, None)
        result = commands.run('Reject inaccessible inline array component', [candidate, fixtures / 'Imported.kt', '-cp', cp(array_adversarial, standard), '-d', work / 'inaccessible-array-output', '--diagnostic-format', 'json'])
        if result is None or result.returncode == 0 or 'COMPILED_INLINE_BODY_UNSUPPORTED' not in result.stderr or 'inaccessible JVM reference' not in result.stderr:
            raise RuntimeError('Inaccessible array component was not rejected: ' + (result.stderr if result else 'timeout'))
        report['cases'].append({'id': 'nested_inaccessible_array_component', 'outcome': 'unsupported_language', 'candidate_code': 'COMPILED_INLINE_BODY_UNSUPPORTED', 'scope': 'Adversarial generated dependency access-flag mutation: reference-array class literal component becomes package-private.', 'original_class_sha256': component_before, 'mutated_class_sha256': sha(component)})
        boundaries = [
            ('const', True, 'const val x = 1u', 'UNSIGNED_CONSTANT_UNSUPPORTED'),
            ('defaults', True, 'fun f(a:UInt = 3u):UInt = a', 'VALUE_DEFAULT_ARGUMENT_UNSUPPORTED'),
            ('mixed_signed', True, 'fun f(a:Int,b:Long):Long = a * b', 'MIXED_INTEGER_ARITHMETIC_UNSUPPORTED'),
            ('nullable_operator', True, 'operator fun UInt?.plus(other:UInt):UInt = (this ?: 0u).plus(other)\nfun f(a:UInt?):UInt = a + 1u', 'NOMINAL_NULLABLE_OPERATOR_UNSUPPORTED'),
            ('array', True, 'fun f(x: UIntArray): UIntArray = x', 'UNSIGNED_ARRAY_UNSUPPORTED'),
            ('range', True, 'fun f() = 1u..2u', 'UNSIGNED_RANGE_UNSUPPORTED'),
            ('small', True, 'fun f(): UByte = 255u', 'UNSIGNED_SMALL_LITERAL_UNSUPPORTED'),
            ('overflow', False, 'fun f(): UInt = 18446744073709551615u', 'UNSIGNED_LITERAL_TYPE_MISMATCH'),
            ('forced', False, 'fun f(): UInt = 1uL', 'UNSIGNED_LITERAL_TYPE_MISMATCH'),
            ('overflow64', False, 'fun f() = 18446744073709551616uL', 'UNSIGNED_LITERAL_OUT_OF_RANGE'),
            ('small_overflow', False, 'fun f(): UByte = 256u', 'UNSIGNED_LITERAL_TYPE_MISMATCH'),
            ('signed_operand', False, 'fun f(a:UInt):UInt = a + 1', 'UNSIGNED_OPERATOR_TYPE_MISMATCH'),
            ('identity', False, 'fun f(a:UInt,b:UInt):Boolean = a === b', 'VALUE_IDENTITY_EQUALITY_INVALID'),
            ('bad_separator', False, 'fun f() = 0x_1u', 'UNSIGNED_LITERAL_INVALID'),
        ]
        for name, valid, source, code in boundaries:
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            boundary_cp = str(standard)
            official = commands.run('Boundary official/' + name, [oracle, path, '-cp', boundary_cp, '-d', work / ('oracle-' + name)])
            checked = commands.run('Boundary candidate/' + name, [candidate, path, '-cp', boundary_cp, '-d', work / ('candidate-' + name), '--diagnostic-format', 'json'])
            category = 'unsupported_language' if valid else 'invalid_kotlin'
            if official is None or (official.returncode == 0) != valid:
                raise RuntimeError(name + ': unexpected official classification')
            if checked is None or checked.returncode == 0 or category not in checked.stderr or code not in checked.stderr:
                raise RuntimeError(name + ': wrong candidate classification ' + (checked.stderr if checked else 'timeout'))
            report['cases'].append({'id': 'boundary/' + name, 'outcome': category, 'official_accepts': valid, 'candidate_code': code, 'source_sha256': sha(path)})
        context = {'directory': work / 'upstream', 'commands': commands, 'candidate': candidate, 'official': oracle, 'stdlib': standard, 'testlib': lib / 'kotlin-test.jar', **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        for path in upstream:
            entry = {'id': path.stem, 'family': 'unsigned_scalars', 'root': 'box', 'tier': 'selected', 'kind': 'box', 'path': str(path.relative_to(ROOT)), 'sha256': sha(path)}
            result = run_case(entry, context)
            result['acceptance_scope'] = 'unchanged_original_assessment'
            report['cases'].append(result)
        if caller_hashes != {str(path): sha(path) for path in java.rglob('*.class')}:
            raise RuntimeError('Fixed Java caller changed')
        case('fixed_caller_integrity', class_hashes=caller_hashes)
        report['status'] = 'complete'
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
    changed = [path for path, value in frozen.items() if not Path(path).is_file() or sha(path) != value]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    if changed:
        report['status'] = 'invalidated'
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'counts': report['counts'], 'error': report.get('error'), 'report': str(work / 'report.json')}))
    return 0 if report['status'] == 'complete' else 1


if __name__ == '__main__':
    raise SystemExit(main())
