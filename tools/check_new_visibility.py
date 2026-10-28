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

from upstream_jvm import Commands, classify_candidate, normalize_signature, relative, run_case, sha

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/new_families/visibility'
INVALID = {
    'getter_visibility': 'class Bad { var value: Int = 0\n private get }',
    'setter_visibility': 'class Bad { private var value: Int = 0\n public set }',
    'internal_protected_setter': 'class Bad { internal var value: Int = 0\n protected set }',
    'protected_internal_setter': 'open class Bad { protected var value: Int = 0\n internal set }',
    'open_private_setter': 'open class Bad { open var value: Int = 0\n private set }',
    'val_setter': 'class Bad { val value: Int = 0\n private set }',
    'top_protected_setter': 'var value: Int = 0\n protected set',
    'object_protected_setter': 'object Bad { var value: Int = 0\n protected set }',
    'private_write': 'import visibilityprobe.PrivateDefault\nfun bad(value: PrivateDefault) { value.value = 1 }',
    'private_safe_write': 'import visibilityprobe.PrivateDefault\nfun bad(value: PrivateDefault?) { value?.value = 1 }',
    'known_null_private_write': 'import visibilityprobe.PrivateDefault\nfun bad() { val value: PrivateDefault? = null; value?.value = 1 }',
    'private_custom_write': 'import visibilityprobe.CustomPrivate\nfun bad(value: CustomPrivate) { value.value = 1 }',
    'protected_write': 'import visibilityprobe.ProtectedDefault\nfun bad(value: ProtectedDefault) { value.value = 1 }',
    'safe_rhs_type': 'import visibilityprobe.SafeTarget\nfun bad(value: SafeTarget?) { value?.value = "bad" }',
    'source_accessor_call': 'class Bad { var value: Int = 0\n private set\nfun bad() { this.setValue(1) } }',
    'simple_renamed_type': 'import java.util.UUID as Token\nfun bad(value: UUID): Int = 1',
    'java_companion_name_is_not_kotlin_companion': 'import visibilityprobe.FakeCompanion\nfun bad() { FakeCompanion.value = 1 }',
    'user_jvm_field_name': 'package usermarker\nclass JvmField\nclass Subject { @JvmField var value: Int = 0\n private set }',
    'aliased_jvm_field_restricted': 'import kotlin.jvm.JvmField as RealField\nclass Bad { @RealField var value: Int = 0\n private set }',
    'jvm_field_restricted': 'class Bad { @JvmField var value: Int = 0\n private set }',
}
BOUNDARIES = {
    'lateinit': 'class Boundary { lateinit var value: String\n private set }',
    'constructor_property': 'class Boundary(internal val value: Int)',
    'override_property': 'interface Contract { val value: Int }; class Boundary : Contract { override val value: Int = 1\n public get }',
    'enum_property': 'enum class Boundary { ONE; var value: Int = 0\n private set }',
    'extension_property': 'internal val String.visibility: Int get() = length',
    'jvm_static': 'object Boundary { @JvmStatic var value: Int = 0\n private set }',
    'internal_jvm_field': 'object Boundary { @JvmField internal var value: Int = 0 }',
    'setter_annotation': 'class Boundary { @set:Deprecated("setter") var value: Int = 0\n private set }',
    'enclosing_qualified_private_companion': 'class Boundary { companion object { var value: Int = 0\n private set }; fun change() { Boundary.value = 1 } }',
    'safe_compound': 'class Target { var value: Int = 0 }; fun change(value: Target?) { value?.value += 1 }',
    'specialized_generic_private_storage': 'class Cell<T>(initial: T) { var value: T = initial\n private set\nfun setInt(other: Cell<Int>, value: Int) { other.value = value } }',
}
NONFRIEND = {
    'member_read': 'import visibilityprobe.InternalDefault\nfun bad(value: InternalDefault): Int = value.value',
    'member_write': 'import visibilityprobe.InternalDefault\nfun bad(value: InternalDefault) { value.value = 1 }',
    'partial_write': 'import visibilityprobe.PublicInternalSetter\nfun bad(value: PublicInternalSetter) { value.value = 1 }',
    'object_read': 'import visibilityprobe.State\nfun bad(): Int = State.hidden',
    'class_companion_read': 'import visibilityprobe.Host\nfun bad(): Int = Host.hidden',
    'interface_companion_read': 'import visibilityprobe.Contract\nfun bad(): Int = Contract.hidden',
    'top_read': 'import visibilityprobe.top\nfun bad(): Int = top',
}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    work = args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    jdk_home = subprocess.check_output(['/usr/libexec/java_home', '-v', '21'], text=True).strip()
    jdk = Path(jdk_home) / 'bin'
    environment = {**os.environ, 'JAVA_HOME': jdk_home}
    reference = ROOT / 'references/kotlinc'
    oracle = reference / 'bin/kotlinc'
    stdlib = reference / 'lib/kotlin-stdlib.jar'
    reflection = reference / 'lib/kotlin-reflect.jar'
    compiler = work.parent / (work.name + '-korrust-frozen')
    shutil.copy2(args.compiler.resolve(), compiler)
    (work / 'commands').mkdir()
    commands = Commands(work / 'commands', environment)
    original = ROOT / 'references/kotlin/compiler/testData/codegen/box/properties/accessToPrivateSetter.kt'
    app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/BackForwardList.kt'
    frozen_paths = [*sorted((ROOT / 'src').rglob('*.rs')), ROOT / 'tests/property_visibility.rs', ROOT / 'tests/unsupported_syntax_diagnostics.rs', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), ROOT / 'tools/upstream_jvm.py', *sorted(FIXTURES.glob('*')), original, app, oracle, stdlib, reflection, reference / 'lib/kotlin-compiler.jar', compiler]
    frozen = {relative(path): sha(path) for path in frozen_paths}
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running',
              'selection': 'Independent property/accessor visibility, default private-setter omission, custom private setters, protected setters and internal accessor names/metadata across ordinary class/object/companion/package scopes. Safe-call assignment and canonical qualified names are reusable prerequisites. Existing internal functions/private bridges are regressions, not new feature counts.',
              'compiler': {'path': relative(compiler), 'sha256': sha(compiler)},
              'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(reference / 'lib/kotlin-compiler.jar'), 'stdlib_sha256': sha(stdlib), 'reflection_sha256': sha(reflection)},
              'environment': {'JAVA_HOME': jdk_home}, 'inputs': {'files_sha256': frozen}, 'cases': [], 'commands': [],
              'limitations': ['New accessor/internal visibility combined with lateinit, delegates, overriding declarations, constructor properties, enum/anonymous members, extension properties or accessor annotations remains bounded; ordinary existing forms retain their existing support.', 'New property visibility combined with valid JVM ABI marker forms remains explicitly unsupported in this snapshot; restricted/custom JvmField accessors reject as invalid.', 'Only simple safe-call assignment is implemented; compound safe assignments remain unsupported. Specialized private default accessors with differing logical/physical field descriptors are explicitly unsupported.', 'Class-qualified private companion setter access from enclosing declarations remains unsupported; own-companion uses and imported public/internal setters are checked separately.', 'The unchanged BackForwardList assessment is preserved separately; no full-app, Android runtime or performance claim.']}
    report_path = work / 'report.json'

    def save():
        report['commands'] = commands.entries
        report['counts'] = dict(sorted(Counter(c['outcome'] for c in report['cases']).items()))
        report['feature_counts'] = dict(sorted(Counter(c['outcome'] for c in report['cases'] if c.get('scope') != 'application_assessment').items()))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def case(ident, outcome='pass', **details):
        report['cases'].append({'id': ident, 'outcome': outcome, **details})
        save()

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def run(label, argv):
        print(label, flush=True)
        result = commands.run(label, argv, timeout=300)
        if result is None:
            raise TimeoutError(label)
        return result

    def require(label, argv):
        result = run(label, argv)
        assert result.returncode == 0, label + ': ' + result.stderr[-3000:]
        return result

    def compile(provider, sources, output, dependencies, module='visibility-feature', friend=None):
        tool = oracle if provider == 'official' else compiler
        flags = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', module] if provider == 'official' else ['--module-name', module, '--diagnostic-format', 'json']
        if friend is not None:
            flags += ['-Xfriend-paths=' + str(friend)] if provider == 'official' else ['--friend-paths', friend]
        return run('Compile ' + provider + '/' + output.name, [tool, *sources, *flags, '-cp', cp(*dependencies, stdlib), '-d', output])

    def diagnostics(result):
        values = []
        for line in result.stderr.splitlines():
            try:
                values.append(json.loads(line))
            except json.JSONDecodeError:
                pass
        return values

    try:
        result = require('Pinned oracle identity', [oracle, '-version'])
        assert 'kotlinc-jvm 2.3.21 ' in result.stdout + result.stderr
        require('JDK identity', [jdk / 'java', '-version'])
        helpers = work / 'helpers'
        require('Compile real Java effect helper', [jdk / 'javac', '-d', helpers, FIXTURES / 'Effects.java', FIXTURES / 'JvmField.java'])
        sources = [FIXTURES / name for name in ['Visibility.kt', 'Qualified.kt', 'AliasedExtension.kt', 'DirectShadow.kt', 'AliasedShadow.kt', 'GenericStorage.kt']]
        outputs = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, sources, output, [helpers])
            assert result.returncode == 0, result.stderr
            outputs[provider] = output
            case('compile/' + provider)
        generic_callers = work / 'generic-storage-callers'
        require('Compile fixed generic private storage caller against official', [jdk / 'javac', '-cp', cp(outputs['official'], stdlib), '-d', generic_callers, FIXTURES / 'GenericStorageCheck.java'])
        generic_hashes = {str(path.relative_to(generic_callers)): sha(path) for path in generic_callers.rglob('*.class')}
        generic_signatures = {}
        for provider, output in outputs.items():
            observed = require('Run fixed generic private storage caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(generic_callers, output, stdlib), 'visibilitygeneric.GenericStorageCheck'])
            assert observed.stdout == 'checks=3\n', observed.stdout
            assert generic_hashes == {str(path.relative_to(generic_callers)): sha(path) for path in generic_callers.rglob('*.class')}
            inspected = require('Inspect generic storage signatures ' + provider, [jdk / 'javap', '-protected', '-s', '-classpath', output, 'visibilitygeneric.Dual'])
            generic_signatures[provider] = normalize_signature(inspected.stdout)
            generic_consumer = work / ('generic-storage-consumer-' + provider)
            require('Compile generic storage Kotlin consumer ' + provider, [oracle, FIXTURES / 'GenericStorageConsumer.kt', '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-cp', cp(output, stdlib), '-d', generic_consumer])
            observed = require('Run generic storage Kotlin consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(generic_consumer, output, stdlib), 'visibilitygenericconsumer.GenericStorageConsumerKt'])
            assert observed.stdout == 'OK\n'
            case('generic_storage/' + provider, independent_java_assertions=3, fixed_official_java_callers_sha256=generic_hashes, jvm_verification='all', kotlin_consumer='compiled and executed')
        assert generic_signatures['official'] == generic_signatures['candidate'], str(generic_signatures)
        require('Compile ordinary Java field named Companion against official provider', [jdk / 'javac', '-cp', outputs['official'], '-d', helpers, FIXTURES / 'FakeCompanion.java'])
        explicit_outputs = {}
        for provider, output in outputs.items():
            explicit = work / ('explicit-field-' + provider)
            result = compile(provider, [FIXTURES / 'ExplicitField.kt'], explicit, [output, helpers], 'explicit_field')
            assert result.returncode == 0, result.stderr
            explicit_outputs[provider] = explicit
            case('compile_explicit_field/' + provider)
        callers = work / 'callers'
        require('Compile fixed Java caller against official provider only', [jdk / 'javac', '-cp', cp(outputs['official'], explicit_outputs['official'], helpers, stdlib), '-d', callers, FIXTURES / 'VisibilityCheck.java'])
        caller_hashes = {str(p.relative_to(callers)): sha(p) for p in callers.rglob('*.class')}
        report['inputs']['fixed_java_callers_sha256'] = caller_hashes
        expected = None
        for provider, output in outputs.items():
            result = require('Verified fixed Java assertions ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, explicit_outputs[provider], output, helpers, stdlib), 'visibilityprobe.VisibilityCheck'])
            count = int(re.fullmatch(r'checks=(\d+)\n', result.stdout).group(1))
            if expected is None:
                expected = count
            assert count == expected and caller_hashes == {str(p.relative_to(callers)): sha(p) for p in callers.rglob('*.class')}
            case('runtime/' + provider, independent_assertions=count, fixed_official_java_caller=True, jvm_verification='all')
            consumer = work / ('metadata-' + provider)
            result = compile('official', [FIXTURES / 'Friend.kt'], consumer, [output, helpers], 'official_friend', output)
            assert result.returncode == 0, result.stderr
            result = require('Verified official Kotlin friend metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, helpers, stdlib), 'visibilityconsumer.FriendKt'])
            assert result.stdout == 'OK\n'
            case('metadata_friend/' + provider, scope='Official Kotlin consumer, exact output friend path and distinct consumer module')
        reflected = work / 'reflection-caller'
        result = compile('official', [FIXTURES / 'Reflection.kt'], reflected, [outputs['official'], reflection], 'reflection_consumer')
        assert result.returncode == 0, result.stderr
        reflection_hashes = {str(p.relative_to(reflected)): sha(p) for p in reflected.rglob('*.class')}
        report['inputs']['fixed_reflection_consumer_sha256'] = reflection_hashes
        for provider, output in outputs.items():
            result = require('Verified Kotlin reflection visibility and absent setter ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(reflected, output, helpers, stdlib, reflection), 'visibilityreflection.ReflectionKt'])
            assert result.stdout == 'OK\n' and reflection_hashes == {str(p.relative_to(reflected)): sha(p) for p in reflected.rglob('*.class')}
            case('reflection/' + provider, scope='Property/setter visibility, internal physical JVM name, absent default private setter and present custom private setter; identical official-compiled consumer')
        friend_outputs = {}
        for provider, output in outputs.items():
            for consumer_compiler in ['official', 'candidate']:
                consumer = work / ('friend-' + consumer_compiler + '-' + provider)
                result = compile(consumer_compiler, [FIXTURES / 'CandidateFriend.kt'], consumer, [output, helpers], 'friend_calls', output)
                assert result.returncode == 0, result.stderr
                friend_outputs[consumer_compiler, provider] = consumer
        friend_caller = work / 'friend-caller'
        require('Compile fixed imported-property Java caller only against official consumer/provider', [jdk / 'javac', '-cp', cp(friend_outputs['official', 'official'], outputs['official'], stdlib), '-d', friend_caller, FIXTURES / 'FriendCalls.java'])
        friend_hashes = {str(p.relative_to(friend_caller)): sha(p) for p in friend_caller.rglob('*.class')}
        report['inputs']['fixed_friend_caller_sha256'] = friend_hashes
        for (consumer_compiler, provider), consumer in friend_outputs.items():
            result = require('Verified imported property caller ' + consumer_compiler + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(friend_caller, consumer, outputs[provider], helpers, stdlib), 'FriendCalls'])
            assert result.stdout == 'friend-checks=11\n' and friend_hashes == {str(p.relative_to(friend_caller)): sha(p) for p in friend_caller.rglob('*.class')}
            case('compiled_friend/' + consumer_compiler + '/' + provider, independent_assertions=11, fixed_official_java_caller=True, jvm_verification='all')
        owners = ['PrivateDefault', 'CustomPrivate', 'ProtectedDefault', 'InternalDefault', 'PublicInternalSetter', 'ExplicitGetter', 'State', 'Host', 'Host$Companion', 'OtherTarget', 'CustomOther', 'SetterName', 'SafeTarget', 'WideTarget', 'NullableTarget', 'Contract', 'Contract$Companion', 'VisibilityKt', 'AliasedExtensionKt']
        owners = ['visibilityprobe.' + name for name in owners] + ['visibilityqualified.Outer', 'visibilityqualified.Outer$Entry', 'visibilityqualified.QualifiedKt', 'visibilityshadow.Direct', 'visibilityshadowuse.Aliased']
        for owner in owners:
            values = [normalize_signature(require('Protected JVM signatures ' + provider + '/' + owner, [jdk / 'javap', '-protected', '-s', '-classpath', output, owner]).stdout) for provider, output in outputs.items()]
            assert values[0] == values[1], owner + ': ' + str(values)
            case('signature/' + owner)
        for name, source in INVALID.items():
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            actuals = {}
            for provider in ['official', 'candidate']:
                output = work / ('invalid-' + provider + '-' + name)
                result = compile(provider, [path], output, [outputs[provider], helpers], 'outside')
                assert result.returncode != 0 and not list(output.rglob('*.class')), result.stderr
                actuals[provider] = {'returncode': result.returncode, 'diagnostics': diagnostics(result) if provider == 'candidate' else result.stderr}
                if provider == 'candidate' and name == 'user_jvm_field_name':
                    assert all(item.get('code') not in {'JVM_FIELD_RESTRICTED_ACCESSOR', 'PROPERTY_VISIBILITY_ABI_ANNOTATION_UNSUPPORTED'} for item in diagnostics(result)), result.stderr
                if provider == 'candidate' and name == 'aliased_jvm_field_restricted':
                    assert any(item.get('code') == 'JVM_FIELD_RESTRICTED_ACCESSOR' for item in diagnostics(result)), result.stderr
            case('invalid/' + name, outcome='invalid_kotlin', actuals=actuals)
        for name, source in BOUNDARIES.items():
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            result = compile('official', [path], work / ('boundary-official-' + name), [], 'boundary')
            assert result.returncode == 0, result.stderr
            result = compile('candidate', [path], work / ('boundary-candidate-' + name), [], 'boundary')
            assert classify_candidate(result) == 'unsupported_language', result.stderr
            case('boundary/' + name, outcome='unsupported_language', diagnostics=diagnostics(result))
        for name, source in NONFRIEND.items():
            path = work / ('nonfriend-' + name + '.kt')
            path.write_text(source + '\n')
            for provider in ['official', 'candidate']:
                for consumer_compiler in ['official', 'candidate']:
                    output = work / ('nonfriend-' + consumer_compiler + '-' + provider + '-' + name)
                    result = compile(consumer_compiler, [path], output, [outputs[provider]], 'visibility-feature')
                    assert result.returncode != 0 and not list(output.rglob('*.class')), result.stderr
            case('nonfriend/' + name, outcome='invalid_kotlin', scope='Both consumer compilers reject both providers, including matching module name without friend path')
        wrong_path = work / 'wrong-friend.kt'
        wrong_path.write_text(NONFRIEND['member_read'] + '\n')
        for consumer_compiler in ['official', 'candidate']:
            result = compile(consumer_compiler, [wrong_path], work / ('wrong-friend-' + consumer_compiler), [outputs['candidate']], 'other', outputs['official'])
            assert result.returncode != 0, result.stderr
        case('nonfriend/wrong_output_path', outcome='invalid_kotlin')
        context = {'directory': work, 'commands': commands, 'candidate': compiler, 'official': oracle, 'stdlib': stdlib, 'testlib': reference / 'lib/kotlin-test.jar', **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        report['cases'].append(run_case({'id': 'original/accessToPrivateSetter', 'family': 'property_visibility', 'root': 'box', 'kind': 'box', 'tier': 'original', 'path': str(original.relative_to(ROOT)), 'sha256': sha(original)}, context))
        save()
        app_outputs = {}
        for provider in ['official', 'candidate']:
            output = work / ('app-' + provider)
            result = compile(provider, [app], output, [], 'visibility-feature')
            if result.returncode:
                case('app/' + provider, outcome=classify_candidate(result) if provider == 'candidate' else 'oracle_failure', scope='application_assessment', diagnostic=result.stderr, source_sha256=sha(app), unchanged_original=True)
            else:
                app_outputs[provider] = output
                case('app/' + provider, scope='application_assessment', source_sha256=sha(app), unchanged_original=True)
        report['application_assessment'] = {'original': str(app), 'status': 'compiled_both' if len(app_outputs) == 2 else 'blocked', 'note': 'No runtime success is claimed unless both original providers compile and a fixed caller executes. The current remaining mapped-collection blocker is preserved in the exact candidate diagnostic.'}
        failures = [c for c in report['cases'] if c['outcome'] in {'correctness_failure', 'oracle_failure'} and c.get('scope') != 'application_assessment']
        report['status'] = 'failed' if failures else 'complete_with_application_assessment'
    except Exception as error:
        report['status'] = 'failed'
        case('harness_failure', outcome='correctness_failure', error=str(error))
    finally:
        changed = [name for name, expected in frozen.items() if not (Path(name) if Path(name).is_absolute() else ROOT / name).is_file() or sha(Path(name) if Path(name).is_absolute() else ROOT / name) != expected]
        report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
        if changed:
            report['status'] = 'invalidated'
        save()
        print(relative(report_path), report['status'], report['counts'])


if __name__ == '__main__':
    main()
