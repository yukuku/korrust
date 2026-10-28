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

ROOT = Path(__file__).resolve().parents[1]
BOUNDARIES = [
    ('unknown_name', False, ['const val BAD = missing + 1']),
    ('unknown_template_name', False, ['object E { const val BAD = "$missing" }']),
    ('enum_annotation_shadow', False, ['import newfamilies.library.RuntimeConstant\nconst val X = 1\nenum class E { X; @RuntimeConstant(answer = X, text = "x", flag = true) fun bad() {} }']),
    ('property_shadow', False, ['const val X = 1\nobject Holder { val X = 2; const val BAD = X }']),
    ('property_prefix_shadow', False, ['object X { const val VALUE = 1 }\nobject Holder { val X = 2; const val BAD = X.VALUE }']),
    ('annotation_property_shadow', False, ['import newfamilies.library.RuntimeConstant\nconst val X = 1\nobject Holder { val X = 2; @RuntimeConstant(answer = X, text = "x", flag = true) fun bad() {} }']),
    ('annotation_constructor_property_shadow', False, ['import newfamilies.library.RuntimeConstant\nconst val X = 1\nclass Holder(val X: Int) { @RuntimeConstant(answer = X, text = "x", flag = true) fun bad() {} }']),
    ('divide_zero', False, ['const val BAD = 1 / 0']),
    ('remainder_zero', False, ['const val BAD = 1 % 0']),
    ('negative_long_literal_range', False, ['const val BAD = -9223372036854775808L']),
    ('positive_long_literal_range', False, ['const val BAD = 9223372036854775808L']),
    ('wrong_declared_type', False, ['const val BAD: Int = 1L + 2L']),
    ('mixed_equality', False, ['const val BAD = 1 == 1L']),
    ('null_concatenation', False, ['const val BAD = "x" + null']),
    ('null_property', False, ['const val BAD = null']),
    ('runtime_call', False, ['fun runtime(): Int = 3\nconst val BAD = runtime()']),
    ('skipped_runtime_call', False, ['fun runtime(): Boolean = true\nconst val BAD = false && runtime()']),
    ('skipped_wrong_type', False, ['const val BAD = false && 3']),
    ('forward_same_owner', False, ['const val BAD = LATER + 1\nconst val LATER = 3']),
    ('forward_qualified_owner', False, ['object Owner { const val BAD = Owner.LATER + 1; const val LATER = 3 }']),
    ('cross_file_cycle', False, ['const val FIRST = SECOND + 1', 'const val SECOND = FIRST + 1']),
    ('file_private_cross_file', False, ['private const val HIDDEN = 3', 'const val BAD = HIDDEN + 1']),
    ('object_private_outside', False, ['object Owner { private const val HIDDEN = 3 }\nconst val BAD = Owner.HIDDEN + 1']),
    ('java_non_constant', False, ['import newfamilies.library.JavaConstants\nconst val BAD = JavaConstants.NON_CONSTANT + 1']),
    ('java_private', False, ['import newfamilies.library.JavaConstants\nconst val BAD = JavaConstants.PRIVATE + 1']),
    ('java_mutable', False, ['import newfamilies.library.JavaConstants\nconst val BAD = JavaConstants.MUTABLE + 1']),
    ('annotation_divide_zero', False, ['import newfamilies.library.RuntimeConstant\n@RuntimeConstant(answer = 1 / 0, text = "x", flag = true) fun bad() {}']),
    ('kotlin_library_internal', False, ['import newfamilies.kotlinlibrary.INTERNAL\nconst val BAD = INTERNAL + 1']),
    ('long_reference_type', False, ['const val BASE = 1\nconst val BAD: Long = BASE']),
    ('long_reference_expression_type', False, ['const val BASE = 1\nconst val BAD: Long = BASE + 1']),
    ('long_left_literal_reference_type', False, ['const val BASE = 1\nconst val BAD: Long = 1 + BASE']),
    ('byte_expression_type', False, ['const val BAD: Byte = 1 + 2']),
    ('byte_const_literal', True, ['const val BOUNDARY: Byte = 1']),
    ('constexpr_call', True, ['const val BOUNDARY = 1.shl(3)']),
    ('floating_template', True, ['const val BOUNDARY = "${1.0}"']),
    ('kotlin_library_public', True, ['import newfamilies.kotlinlibrary.PUBLIC\nconst val BOUNDARY = PUBLIC + 1']),
]

def depth_chain(count):
    lines = [f'object Node{index} {{ const val VALUE: Int = Node{index + 1}.VALUE + 1 }}' for index in range(count)]
    lines.append(f'object Node{count} {{ const val VALUE: Int = 1 }}')
    return '\n'.join(lines)


def string_growth(count):
    lines = ['const val A0 = "x"']
    lines.extend(f'const val A{index} = A{index - 1} + A{index - 1}' for index in range(1, count + 1))
    return '\n'.join(lines)


BOUNDARIES.append(('cross_owner_depth', True, [depth_chain(80)]))



def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(path):
    return str(Path(path).relative_to(ROOT))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--source-snapshot', type=Path)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    source_root = args.source_snapshot.resolve() if args.source_snapshot else ROOT
    fixtures = source_root / 'tests/fixtures/new_families/constants'
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-new-constants')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', cases=[], commands=[], inputs={}, compiler={}, reference_compiler={}, evidence=[], selection='Typed scalar compile-time expressions, Kotlin source constant references and checked Java ConstantValue dependencies; reflection observes actual provider fields instead of javac-inlined constants, fixed Java callers reused with JVM verification, official Kotlin metadata consumers, runtime annotation values, class-initialization independence, preserved upstream constant-expression fixture and official-invalid versus valid unsupported boundaries. Byte/Short const declarations, constexpr function interpretation and floating-point formatting are outside this bounded evaluator. Compiled Kotlin constant dependencies are conservatively unsupported until Kotlin visibility is represented; Java field access flags alone do not prove Kotlin visibility. Unresolved bare names with no wildcard imports are invalid; unresolved qualified and external imported paths retain explicit unsupported boundaries because the bounded resolver cannot prove declaration absence. No full-app/Android/runtime-performance claim.')
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
    stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    compiler = work / 'korrust-frozen'
    shutil.copy2(args.compiler.resolve(), compiler)
    upstream = ROOT / 'references/kotlin/compiler/testData/codegen/box/constants/literalToLongConversion.kt'
    frozen = [source_root / 'Cargo.toml', source_root / 'Cargo.lock', *sorted((source_root / 'src').rglob('*.rs')), source_root / 'tests/frontend.rs', *sorted(fixtures.glob('*')), Path(__file__).resolve(), upstream]
    report['inputs']['files_sha256'] = {relative(path): digest(path) for path in frozen}
    report['inputs']['source_snapshot'] = relative(source_root)
    report['inputs']['reference_sources_manifest_sha256'] = digest(ROOT / 'references/sources.json')
    report['inputs']['constant_spec_sha256'] = {name: digest(ROOT / name) for name in ['references/kotlin-spec/docs/src/md/kotlin.core/declarations.md', 'references/kotlin-spec/docs/src/md/kotlin.core/expressions.md']}
    report['limits'] = dict(active_constant_declarations=128, combined_expression_reference_evaluation_frames=128, constant_string_modified_utf8_bytes=65535)
    report['reproduction_commands'] = [['python3', 'tools/check_new_constants.py', '--compiler', str(args.compiler.resolve())] + (['--source-snapshot', str(source_root)] if args.source_snapshot else [])]
    report['compiler'] = dict(path=relative(compiler), original_path=str(args.compiler.resolve()), binary_sha256=digest(compiler))
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_constants']
    report['reference_compiler'] = dict(version=pin['version'], options=options, compiler_pin_sha256=digest(ROOT / 'references/compiler.json'), stdlib_sha256=digest(stdlib))

    def run(label, command):
        print(label, flush=True)
        index = len(report['commands'])
        result = subprocess.run(list(map(str, command)), cwd=ROOT, text=True, capture_output=True, timeout=120)
        entry = dict(label=label, argv=list(map(str, command)), exit_code=result.returncode)
        for stream in ['stdout', 'stderr']:
            path = work / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
            report['evidence'].append(relative(path))
        report['commands'].append(entry)
        return result

    def require(result):
        if result.returncode:
            raise AssertionError(result.stderr[-3000:])
        return result

    def case(ident, outcome='pass', **details):
        report['cases'].append(dict(id=ident, outcome=outcome, **details))

    dependency = work / 'dependencies'

    def cp(*items):
        return os.pathsep.join(map(str, items))

    def compile_provider(provider, sources, destination):
        command = [kotlinc, *sources, *options] if provider == 'official' else [compiler, *sources, '--module-name', 'new_constants', '--diagnostic-format', 'json']
        return run('Compile ' + provider + ' ' + destination.name, [*command, '-cp', cp(dependency, stdlib), '-d', destination])

    try:
        identity = require(run('Pinned official compiler identity', [kotlinc, '-version']))
        assert 'kotlinc-jvm ' + pin['version'] + ' ' in identity.stdout + identity.stderr
        require(run('JDK identity', [jdk / 'java', '-version']))
        require(run('Candidate identity', [compiler, '--version']))
        require(run('Compile Java constant and annotation dependencies', [jdk / 'javac', '-d', dependency, fixtures / 'InitializationTrace.java', fixtures / 'JavaConstants.java', fixtures / 'RuntimeConstant.java']))
        require(run('Compile official Kotlin constant dependency', [kotlinc, fixtures / 'KotlinLibrary.kt', *options, '-module-name', 'constant_library', '-d', dependency]))
        report['inputs']['dependency_classes_sha256'] = {str(path.relative_to(dependency)): digest(path) for path in dependency.rglob('*.class')}
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        sources = [fixtures / name for name in ['Shared.kt', 'Constants.kt', 'ImportedConstants.kt', 'DefaultPriority.kt', 'ImportChoice.kt', 'SameFileChoice.kt']]
        for provider, destination in outputs.items():
            require(compile_provider(provider, sources, destination))
            case('compile/' + provider)
        driver = work / 'driver'
        require(run('Compile fixed Java caller against official output', [jdk / 'javac', '-cp', cp(outputs['official'], dependency, stdlib), '-d', driver, fixtures / 'ConstantsCheck.java']))
        report['inputs']['fixed_java_classes_sha256'] = {str(path.relative_to(driver)): digest(path) for path in driver.rglob('*.class')}
        observations = []
        for provider, destination in outputs.items():
            result = require(run('Run reflective fixed Java caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(driver, destination, dependency, stdlib), 'newfamilies.constants.ConstantsCheck']))
            observations.append(result.stdout)
            match = re.fullmatch(r'checks=(\d+)\n?', result.stdout)
            assert match and int(match.group(1)) == 60, result.stdout
            case('runtime/' + provider, independent_assertions=int(match.group(1)), jvm_verification='all', const_field_access='reflection; no compile-time inlining by javac', dependency_clinit='independently asserted not triggered')
        assert observations[0] == observations[1]
        inspected = {}
        for provider, destination in outputs.items():
            inspected[provider] = []
            for name in ['newfamilies.constants.ConstantsKt', 'newfamilies.shared.SharedKt', 'newfamilies.constants.Holder', 'newfamilies.constants.Holder$Nested']:
                text = require(run('Inspect ConstantValue metadata ' + provider + ' ' + name, [jdk / 'javap', '-public', '-constants', '-s', '-classpath', destination, name])).stdout
                inspected[provider].append(text)
                require(run('Inspect class attributes ' + provider + ' ' + name, [jdk / 'javap', '-v', '-classpath', destination, name]))
        assert inspected['official'] == inspected['candidate'], 'Public field/method descriptors or ConstantValue observations differ'
        case('public_signatures_constant_values', scope='javap public descriptors and ConstantValue text compared; full verbose class attributes logged and reflection checks actual field contents')
        consumers = {}
        for provider, destination in outputs.items():
            consumers[provider] = work / ('consumer-' + provider)
            require(run('Compile official Kotlin metadata consumer ' + provider, [kotlinc, fixtures / 'Consumer.kt', *options, '-module-name', 'constant_consumer', '-cp', cp(destination, dependency, stdlib), '-d', consumers[provider]]))
        consumer_driver = work / 'consumer-driver'
        require(run('Compile fixed metadata launcher', [jdk / 'javac', '-cp', cp(consumers['official'], outputs['official'], stdlib), '-d', consumer_driver, fixtures / 'ConsumerLauncher.java']))
        report['inputs']['fixed_consumer_launcher_classes_sha256'] = {str(path.relative_to(consumer_driver)): digest(path) for path in consumer_driver.rglob('*.class')}
        for provider, destination in outputs.items():
            result = require(run('Run Kotlin metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer_driver, consumers[provider], destination, dependency, stdlib), 'ConsumerLauncher']))
            assert result.stdout.strip() == 'OK'
            case('kotlin_consumer/' + provider, independently_compiled_against_provider=True, jvm_verification='all')
            for name in ['PrivateConsumer', 'InternalConsumer']:
                rejection = run('Reject Kotlin metadata visibility ' + provider + ' ' + name, [kotlinc, fixtures / (name + '.kt'), *options, '-module-name', 'constant_consumer', '-cp', cp(destination, dependency, stdlib), '-d', work / (provider + '-' + name)])
                assert rejection.returncode and re.search('private|internal|invisible|access', rejection.stderr, re.I), rejection.stderr
                case('metadata_visibility/' + provider + '/' + name, outcome='invalid_kotlin', official_consumer_rejects=True)
        generated_sources = {}
        for name, valid, source_texts in BOUNDARIES:
            sources = []
            for index, text in enumerate(source_texts):
                path = work / (name + '_' + str(index) + '.kt')
                path.write_text(text + '\n')
                generated_sources[relative(path)] = digest(path)
                report['evidence'].append(relative(path))
                sources.append(path)
            oracle = compile_provider('official', sources, work / ('official-' + name))
            destination = work / ('candidate-' + name)
            actual = compile_provider('candidate', sources, destination)
            assert bool(oracle.returncode) != valid, name + ': official classification changed'
            assert actual.returncode and not list(destination.rglob('*.class')), name + ': must reject without output classes'
            if valid:
                assert re.search('unsupported|not supported|support only', actual.stderr, re.I), actual.stderr
                case('boundary/' + name, outcome='unsupported_language', official_accepts=True, candidate_rejects=True)
            else:
                if name in {'unknown_name', 'unknown_template_name'}:
                    assert 'INVALID_CONSTANT_EXPRESSION' in actual.stderr and 'invalid_kotlin' in actual.stderr, actual.stderr
                case('boundary/' + name, outcome='invalid_kotlin', official_accepts=False, candidate_rejects=True)
        string_limit = work / 'computed_string_limit.kt'
        string_limit.write_text(string_growth(16) + '\n')
        generated_sources[relative(string_limit)] = digest(string_limit)
        report['evidence'].append(relative(string_limit))
        oracle_limit = compile_provider('official', [string_limit], work / 'official-string-limit')
        actual_limit_dir = work / 'candidate-string-limit'
        actual_limit = compile_provider('candidate', [string_limit], actual_limit_dir)
        assert oracle_limit.returncode and re.search('UTF8|UTF-8|string too large', oracle_limit.stderr, re.I), oracle_limit.stderr
        assert actual_limit.returncode and 'CONSTANT_STRING_LIMIT_UNSUPPORTED' in actual_limit.stderr and not list(actual_limit_dir.rglob('*.class')), actual_limit.stderr
        case('resource/computed_string_limit', outcome='unsupported_language', official_outcome='JVM target rejected oversized constant', candidate_rejects=True, limit_modified_utf8_bytes=65535, scope='Language-valid scalar string growth exceeds JVM ConstantValue UTF-8 storage; classified as a target/resource boundary, not invalid Kotlin syntax')
        report['inputs']['generated_boundary_sources_sha256'] = generated_sources
        upstream_outputs = {provider: work / ('upstream-' + provider) for provider in outputs}
        for provider, destination in upstream_outputs.items():
            require(compile_provider(provider, [upstream], destination))
        java = work / 'UpstreamConstantCheck.java'
        java.write_text('public final class UpstreamConstantCheck { public static void main(String[] args) { String value = LiteralToLongConversionKt.box(); if (!"OK".equals(value)) throw new AssertionError(value); System.out.println(value); } }\n')
        upstream_driver = work / 'upstream-driver'
        require(run('Compile fixed upstream Java caller', [jdk / 'javac', '-cp', cp(upstream_outputs['official'], stdlib), '-d', upstream_driver, java]))
        report['inputs']['fixed_upstream_driver_classes_sha256'] = {str(path.relative_to(upstream_driver)): digest(path) for path in upstream_driver.rglob('*.class')}
        for provider, destination in upstream_outputs.items():
            result = require(run('Run unchanged upstream constant fixture ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(upstream_driver, destination, stdlib), 'UpstreamConstantCheck']))
            assert result.stdout.strip() == 'OK'
        case('upstream/literalToLongConversion', unchanged_upstream=True, source=relative(upstream), source_sha256=digest(upstream), directives_preserved=True, fixed_java_caller=True, jvm_verification='all')
        assert report['inputs']['files_sha256'] == {relative(path): digest(path) for path in frozen}
        assert generated_sources == {name: digest(ROOT / name) for name in generated_sources}
        assert report['compiler']['binary_sha256'] == digest(compiler)
        case('frozen_inputs')
        report['status'] = 'pass'
    except Exception as error:
        case('harness', 'fail', error=str(error))
        report['status'] = 'fail'
        raise
    finally:
        report['summary'] = dict(Counter(case['outcome'] for case in report['cases']))
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(relative(work / 'report.json'), flush=True)


if __name__ == '__main__':
    main()
