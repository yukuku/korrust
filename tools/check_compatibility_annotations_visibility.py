#!/usr/bin/env python3
"""Check ordinary annotations, bounded module visibility and diagnostic boundaries."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import struct
import sys

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_androidbible_simple import Harness as LoggedHarness
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / 'tests/fixtures/compatibility_annotations_visibility'
GID = 'Alkitab/src/main/java/yuku/alkitab/datatransfer/model/Gid.kt'
MODULE = 'visibility_fixture'
ANNOTATION_PREFIX = 'Lcompatibility/annotations/'


class Reader:
    def __init__(self, data):
        self.data, self.offset = data, 0

    def take(self, length):
        result = self.data[self.offset:self.offset + length]
        if len(result) != length:
            raise ValueError('Truncated classfile')
        self.offset += length
        return result

    def u1(self):
        return self.take(1)[0]

    def u2(self):
        return int.from_bytes(self.take(2), 'big')

    def u4(self):
        return int.from_bytes(self.take(4), 'big')


def declaration_annotations(path):
    reader = Reader(Path(path).read_bytes())
    if reader.u4() != 0xcafebabe:
        raise ValueError('Invalid classfile magic')
    reader.take(4)
    pool = [None] * reader.u2()
    index = 1
    while index < len(pool):
        tag = reader.u1()
        if tag == 1:
            pool[index] = reader.take(reader.u2()).replace(b'\xc0\x80', b'\0').decode('utf-8', 'surrogatepass')
        elif tag == 3:
            pool[index] = int.from_bytes(reader.take(4), 'big', signed=True)
        elif tag == 4:
            pool[index] = struct.unpack('>f', reader.take(4))[0]
        elif tag in (5, 6):
            reader.take(8)
            index += 1
        elif tag in (7, 8, 16, 19, 20):
            pool[index] = reader.u2()
        elif tag in (9, 10, 11, 12, 17, 18):
            reader.take(4)
        elif tag == 15:
            reader.take(3)
        else:
            raise ValueError('Unsupported classfile constant tag ' + str(tag))
        index += 1
    reader.take(6)
    reader.take(2 * reader.u2())
    result = {}

    def annotation(data):
        name = pool[data.u2()]
        values = {pool[data.u2()]: value(data) for _ in range(data.u2())}
        return {'type': name, 'values': values}

    def value(data):
        kind = chr(data.u1())
        if kind in 'BCDFIJSZs':
            return {'tag': kind, 'value': pool[data.u2()]}
        if kind == 'e':
            return {'tag': kind, 'type': pool[data.u2()], 'value': pool[data.u2()]}
        if kind == 'c':
            return {'tag': kind, 'value': pool[data.u2()]}
        if kind == '@':
            return {'tag': kind, 'value': annotation(data)}
        if kind == '[':
            return {'tag': kind, 'value': [value(data) for _ in range(data.u2())]}
        raise ValueError('Invalid annotation element kind ' + kind)

    def attributes(owner):
        for _ in range(reader.u2()):
            name = pool[reader.u2()]
            data = Reader(reader.take(reader.u4()))
            records = []
            if name in ('RuntimeVisibleAnnotations', 'RuntimeInvisibleAnnotations'):
                records = [(owner, annotation(data)) for _ in range(data.u2())]
            elif name in ('RuntimeVisibleParameterAnnotations', 'RuntimeInvisibleParameterAnnotations'):
                for parameter in range(data.u1()):
                    records += [(owner + '#parameter:' + str(parameter), annotation(data)) for _ in range(data.u2())]
            for target, item in records:
                if item['type'].startswith(ANNOTATION_PREFIX):
                    result.setdefault(target, {}).setdefault(name, []).append(item)
    for kind in ('field', 'method'):
        for _ in range(reader.u2()):
            reader.u2()
            name, descriptor = pool[reader.u2()], pool[reader.u2()]
            attributes(kind + ':' + name + descriptor)
    attributes('class')
    return result


def expected_annotations():
    def mark(text, count, enabled):
        return {'type': ANNOTATION_PREFIX + 'RuntimeMark;', 'values': {
            'text': {'tag': 's', 'value': text}, 'count': {'tag': 'I', 'value': count},
            'enabled': {'tag': 'Z', 'value': int(enabled)}}}
    def binary(text):
        return {'type': ANNOTATION_PREFIX + 'ClassMark;', 'values': {'value': {'tag': 's', 'value': text}}}
    visible = 'RuntimeVisibleAnnotations'
    invisible = 'RuntimeInvisibleAnnotations'
    parameters = 'RuntimeVisibleParameterAnnotations'
    return {
        'class': {visible: [mark('type', 1, True)], invisible: [binary('binary-type')]},
        'field:valueI': {visible: [mark('field', 3, True)], invisible: [binary('binary-field')]},
        'method:getValue()I': {visible: [mark('getter', 4, False)], invisible: [binary('binary-getter')]},
        'method:setValue(I)V': {visible: [mark('setter', 5, True)]},
        'method:plus(I)I': {visible: [mark('method', 6, False)]},
        'method:<init>(I)V#parameter:0': {parameters: [mark('parameter', 2, False)]},
        'method:setValue(I)V#parameter:0': {parameters: [mark('setter-parameter', 7, False)]},
    }


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        self.fixtures = args.fixtures.resolve()
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-compatibility-annotations-visibility'
        self.work = ROOT / 'target/checks' / self.run_id
        self.evidence = self.work / 'evidence' if args.preflight else ROOT / 'reports/correctness' / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix('.json')
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
                           compiler={}, reference_compiler={}, inputs={}, commands=[], evidence=[], cases=[],
                           selection='Tasks26–28 bounded Java-defined ordinary annotations with String/Int/Boolean arguments, RUNTIME/CLASS/SOURCE retention, class/method/field/getter/setter/constructor-parameter/setter-parameter placement; file-private functions/properties and module-internal object/class methods with JVM mangling; official and candidate external/friend visibility consumers; actionable valid-unsupported value-class/enum-body/property/type-use/array-class-enum/plugin boundaries. Java annotation classes are independent compiled dependencies. Friend consumers use one Java launcher compiled against official FriendKt.main()V; automatic Kotlin main(String[]) launcher lowering is not assessed. Kotlin annotation declaration emission, general type-use/property annotation emission, serialization/Compose transformations, Android/full-module/device behavior and performance are not claimed.')
        self.java, self.javac, self.javap = [str(Path(os.environ['JAVA_HOME']) / 'bin' / name) if os.environ.get('JAVA_HOME') else shutil.which(name) for name in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', MODULE]

    def outcome(self, name, passed, **details):
        self.report['cases'].append(dict(id=name, outcome='pass' if passed else 'fail', **details))

    def compile(self, label, sources, destination, *, candidate=False, dependencies=(), module=MODULE, friend=None, reject=False, diagnostic_format=None):
        cp = classpath(*dependencies, self.dependencies, self.stdlib, *self.plugin_jars)
        if candidate:
            command = [self.compiler, *sources, '-d', destination, '-cp', cp, '--module-name', module]
            if friend:
                command += ['--friend-paths', str(friend)]
            if diagnostic_format is not None:
                command += ['--diagnostic-format', diagnostic_format]
        else:
            command = [self.kotlinc, *sources, *self.options[:-1], module, '-cp', cp, '-d', destination]
            if friend:
                command += ['-Xfriend-paths=' + str(friend)]
        return self.run(label, command, allowed=(0, 1) if reject else (0,))

    def prepare(self):
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        self.metadata_compiler = ROOT / pin['path'] / 'lib/kotlin-compiler.jar'
        self.compiler = self.args.compiler.resolve()
        self.plugin_jars = [self.args.serialization_jar.resolve(), self.args.compose_jar.resolve()]
        for jar in self.plugin_jars:
            if not jar.is_file():
                raise FileNotFoundError('Required real plugin-annotation dependency missing: ' + str(jar))
        self.dependencies = self.work / 'dependencies'
        self.frozen = file_hashes([Path(__file__), *[p for p in self.fixtures.rglob('*') if p.is_file()], SNAPSHOT / GID, *self.plugin_jars])
        self.reference_hashes = file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar'))
        identity = self.run('Pinned official compiler identity', [self.kotlinc, '-version'])
        if 'kotlinc-jvm ' + pin['version'] + ' ' not in identity.stdout + identity.stderr:
            raise AssertionError('Pinned official identity mismatch')
        self.report['reference_compiler'] = dict(version=pin['version'], options=self.options, distribution_sha256=self.reference_hashes)
        self.report['inputs'] = dict(fixture_files_sha256=file_hashes(self.fixtures.rglob('*.*')), frozen_files_sha256=self.frozen,
                                     unchanged_app_sources_sha256={GID: sha(SNAPSHOT / GID)},
                                     metadata_flags_oracle=dict(compiler_jar=str(self.metadata_compiler), compiler_jar_sha256=sha(self.metadata_compiler), api='JvmProtoBufUtil.readClassDataFrom + Flags.HAS_ANNOTATIONS; exact Annotated class/plus/primary value/getter/setter/setter-parameter selection. Omitted accessor flags inherit property HAS_ANNOTATIONS/visibility/modality per pinned metadata.proto and MemberDeserializer. Selected effective numeric flags and HAS bits are checked, including setter IS_NOT_DEFAULT.'),
                                     annotation_dependency_jars_sha256={str(jar): sha(jar) for jar in self.plugin_jars},
                                     annotation_dependency_provenance=[
                                         {'artifact': 'org.jetbrains.kotlinx:kotlinx-serialization-core-jvm:1.5.1', 'url': 'https://repo.maven.apache.org/maven2/org/jetbrains/kotlinx/kotlinx-serialization-core-jvm/1.5.1/kotlinx-serialization-core-jvm-1.5.1.jar'},
                                         {'artifact': 'androidx.compose.runtime:runtime-android:1.9.4', 'url': 'https://dl.google.com/dl/android/maven2/androidx/compose/runtime/runtime-android/1.9.4/runtime-android-1.9.4.aar', 'extraction': 'Unchanged classes.jar from the real AAR; only annotation declaration resolution tested, no Compose plugin transformation.'},
                                     ],
                                     external_compilation='javac builds all Java annotation dependencies; pinned official Kotlin builds only PropertyMark annotation dependency. All positive production source files are candidate-owned.',
                                     ordinary_annotation_scope='Existing Java annotation classes with String/Int/Boolean constants. Property/type-use targets and array/class/enum constants are explicit unsupported boundaries.',
                                     task26_original_inventory={'AlkitabYes2/src/test/java/yuku/alkitab/yes2/Yes2RoundTripTest.kt': '@get:Rule', 'AlkitabYes2/src/test/java/yuku/alkitab/yes2/util/UnsignedBinarySearchKtTest.kt': '@file:OptIn(ExperimentalUnsignedTypes::class)', 'AlkitabIo/src/test/java/yuku/alkitab/io/Utf8DecoderTest.java': '@Test(expected = IndexOutOfBoundsException.class)'})
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        if inventory[GID] != sha(SNAPSHOT / GID):
            raise AssertionError('Pinned Gid source changed')
        self.report['reproduction_commands'] = [[sys.executable, *sys.argv]]
        if self.args.preflight != 'official':
            self.compiler_sha = sha(self.compiler)
            source_root = self.args.compiler_root.resolve()
            self.compiler_sources = file_hashes([source_root / 'Cargo.toml', source_root / 'Cargo.lock', *sorted((source_root / 'src').rglob('*.rs'))])
            self.report['compiler'] = dict(path=str(self.compiler), binary_sha256=self.compiler_sha, version=self.run('Candidate identity', [self.compiler, '--version']).stdout.strip(), source_files_sha256=self.compiler_sources, jvm_target_scope='Official target JVM1.8; candidate emits supported classfile versions without a target-selection option.')
        self.run('Compile real independent Java annotation dependency', [self.javac, '-parameters', '-d', self.dependencies, *sorted((self.fixtures / 'dependencies').glob('*.java'))])
        self.compile('Compile official property-target annotation dependency', [self.fixtures / 'dependencies/PropertyMark.kt'], self.dependencies)
        self.report['inputs']['compiled_annotation_dependency_sha256'] = tree_hashes(self.dependencies)
        self.sources = sorted((self.fixtures / 'positive').glob('*.kt'))
        self.outputs = {}
        for provider in ['official'] + ([] if self.args.preflight == 'official' else ['candidate']):
            output = self.work / provider
            self.compile('Compile ' + provider + ' annotation/visibility module', self.sources, output, candidate=provider == 'candidate')
            self.outputs[provider] = output
        self.drivers = self.work / 'fixed-java'
        self.run('Compile fixed Java assertions against official only', [self.javac, '-cp', classpath(self.outputs['official'], self.dependencies, self.stdlib, self.metadata_compiler), '-d', self.drivers, self.fixtures / 'AnnotationsVisibilityCheck.java', self.fixtures / 'MetadataFlagsCheck.java', ROOT / 'tests/fixtures/ModuleClassVerification.java'])
        self.report['inputs']['fixed_java_bytecode_sha256'] = tree_hashes(self.drivers)

    def verify_outputs(self):
        expected = expected_annotations()
        signatures = {}
        metadata_flags = {}
        friend_launcher = None
        for provider, output in self.outputs.items():
            observed = declaration_annotations(output / 'compatibility/annotations/Annotated.class')
            self.outcome('annotations/classfile/' + provider, observed == expected, expected=expected, observed=observed)
            metadata_result = self.run('Pinned official metadata flag decoder ' + provider, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, output, self.dependencies, self.stdlib, self.metadata_compiler), 'MetadataFlagsCheck'])
            metadata_flags[provider] = json.loads(metadata_result.stdout)
            expected_flags = {'class': 7, 'plus': 7, 'constructor_value': 1, 'getter': 7, 'setter': 71, 'setter_parameter': 1,
                              'source_class': 6, 'source_plus': 6, 'source_constructor_value': 0, 'source_property': 1798,
                              'source_getter': 6, 'source_setter': 6, 'source_setter_parameter': 0}
            source_attributes = declaration_annotations(output / 'compatibility/annotations/SourceRetention.class')
            self.outcome('annotations/source_retention/' + provider, source_attributes == {}, observed=source_attributes)
            self.outcome('metadata/annotation_flags/' + provider, metadata_flags[provider] == expected_flags, expected=expected_flags, observed=metadata_flags[provider])
            runtime = self.run('Run fixed runtime/visibility assertions ' + provider, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, output, self.dependencies, self.stdlib), 'AnnotationsVisibilityCheck'])
            self.outcome('runtime/' + provider, runtime.stdout.strip() == 'annotations-visibility-ok:13', observed_stdout=runtime.stdout.strip())
            owners = [name[:-6].replace('/', '.') for name in tree_hashes(output, '.class')]
            verified = self.run('Verify all emitted classes ' + provider, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, output, self.dependencies, self.stdlib), 'ModuleClassVerification', *owners])
            self.outcome('verification/' + provider, verified.stdout.strip() == f'Verified {len(owners)} module classes', classes=owners)
            signatures[provider] = {}
            for owner in ['compatibility.annotations.Annotated', 'compatibility.visibility.PrivateKt', 'compatibility.visibility.SameModuleKt', 'compatibility.visibility.InternalObject', 'compatibility.visibility.InternalClass', 'compatibility.visibility.FilePrivateAKt', 'compatibility.visibility.FilePrivateBKt']:
                result = self.run('Public JVM signatures ' + provider + '/' + owner, [self.javap, '-public', '-s', '-classpath', output, owner])
                signatures[provider][owner] = source_abi(result.stdout)
                if owner.endswith('.InternalObject'):
                    self.outcome('internal_mangling/' + provider, 'decorate$visibility_fixture' in result.stdout and 'decorate(int)' not in result.stdout)
                if owner.endswith('.InternalClass'):
                    self.outcome('internal_class_mangling/' + provider, 'internalValue$visibility_fixture' in result.stdout and 'internalValue(int)' not in result.stdout)
            destination = self.work / ('consumer-' + provider)
            self.compile('Official Kotlin external consumer ' + provider, [self.fixtures / 'consumers/Consumer.kt'], destination, dependencies=[output], module='external_consumer')
            result = self.run('Run external Kotlin consumer ' + provider, [self.java, '-Xverify:all', '-cp', classpath(destination, output, self.dependencies, self.stdlib), 'compatibility.consumer.ConsumerKt'])
            self.outcome('kotlin_consumer/' + provider, result.stdout.strip() == 'consumer-ok')
            for kind in ['official'] + ([] if self.args.preflight == 'official' else ['candidate']):
                friend = self.work / ('friend-' + provider + '-' + kind)
                self.compile('Compile friend consumer ' + provider + '/' + kind, [self.fixtures / 'consumers/Friend.kt'], friend, dependencies=[output], module='friend_consumer', friend=output, candidate=kind == 'candidate')
                if friend_launcher is None:
                    if provider != 'official' or kind != 'official':
                        raise AssertionError('Friend launcher must compile against official consumer output first')
                    friend_launcher = self.work / 'fixed-friend-launcher'
                    self.run('Compile fixed Java friend launcher against official consumer only', [self.javac, '-cp', classpath(friend, output, self.dependencies, self.stdlib), '-d', friend_launcher, self.fixtures / 'FriendConsumerLauncher.java'])
                    self.report['inputs']['fixed_friend_launcher_bytecode_sha256'] = tree_hashes(friend_launcher)
                    self.report['inputs']['friend_entrypoint_scope'] = 'One fixed official-compiled Java launcher invokes FriendKt.main()V for every provider/consumer pair. Original friend assertions and compiler flags are unchanged; automatic Kotlin main(String[]) bridge lowering is outside scope.'
                result = self.run('Run friend consumer ' + provider + '/' + kind, [self.java, '-Xverify:all', '-cp', classpath(friend_launcher, friend, output, self.dependencies, self.stdlib), 'FriendConsumerLauncher'])
                self.outcome('friend_consumer/' + provider + '/' + kind, result.stdout.strip() == 'friend-ok')
        if 'candidate' in signatures:
            self.outcome('metadata/selected_flag_parity', metadata_flags['candidate'] == metadata_flags['official'], official=metadata_flags['official'], candidate=metadata_flags['candidate'])
            self.outcome('public_signatures', signatures['official'] == signatures['candidate'], official=signatures['official'], candidate=signatures['candidate'])

    def boundary(self, ident, sources, classification, pattern=None, *, dependencies=(), module=MODULE, exact_line=None, expected_code=None):
        official = self.work / 'boundaries' / ident / 'official'
        result = self.compile('Official boundary ' + ident, sources, official, dependencies=dependencies, module=module, reject=True)
        expected_valid = classification in ('unsupported_language', 'plugin_required')
        oracle_valid = result.returncode == 0
        self.report['cases'].append(dict(id='boundary/' + ident + '/official', outcome='oracle_accepted' if oracle_valid else classification, boundary_verified=oracle_valid == expected_valid, diagnostic=result.stderr.strip(), expected_language_valid=expected_valid))
        if self.args.preflight == 'official':
            return
        output = self.work / 'boundaries' / ident / 'candidate'
        result = self.compile('Candidate boundary ' + ident, sources, output, dependencies=dependencies, module=module, candidate=True, reject=True, diagnostic_format='json' if expected_code is not None else None)
        if expected_code is not None:
            try:
                payload = json.loads(result.stderr)
            except json.JSONDecodeError:
                payload = None
            location = None
            is_correct = result.returncode != 0 and not tree_hashes(output) and isinstance(payload, dict)
            if isinstance(payload, dict):
                path, line, column = payload.get('path'), payload.get('line'), payload.get('column')
                is_correct = (is_correct and payload.get('category') == classification and payload.get('code') == expected_code
                              and isinstance(path, str) and path in {str(source) for source in sources}
                              and type(line) is int and line >= 1 and type(column) is int and column >= 1)
                if exact_line is not None:
                    is_correct = is_correct and line == exact_line
                if isinstance(path, str) and type(line) is int and type(column) is int:
                    location = {'path': path, 'line': line, 'column': column}
            self.report['cases'].append(dict(id='boundary/' + ident + '/candidate', outcome=classification if is_correct else 'fail', boundary_verified=is_correct,
                                             diagnostic=result.stderr.strip(), diagnostic_payload=payload, expected_diagnostic_code=expected_code,
                                             first_diagnostic_location=location))
            return
        location = re.search(r'([^\n]+\.kt):(\d+):(\d+):', result.stderr)
        is_correct = result.returncode != 0 and not tree_hashes(output) and bool(re.search(pattern, result.stderr, re.I)) and location is not None
        if exact_line is not None:
            is_correct = is_correct and int(location.group(2)) == exact_line if location else False
        if classification == 'invalid_kotlin':
            is_correct = is_correct and not re.search(r'unsupported|not (?:yet )?supported', result.stderr, re.I)
        self.report['cases'].append(dict(id='boundary/' + ident + '/candidate', outcome=classification if is_correct else 'fail', boundary_verified=is_correct, diagnostic=result.stderr.strip(), first_diagnostic_location={'path': location.group(1), 'line': int(location.group(2)), 'column': int(location.group(3))} if location else None))

    def boundaries(self):
        self.boundary('invalid_target', [self.fixtures / 'negative/InvalidTarget.kt'], 'invalid_kotlin', r'target|field|applicab')
        self.boundary('invalid_argument', [self.fixtures / 'negative/InvalidArgument.kt'], 'invalid_kotlin', r'argument|type|Int|mismatch')
        self.boundary('mangling_collision', [self.fixtures / 'negative/ManglingCollision.kt'], 'invalid_kotlin', r'collision|clash|duplicate|conflict')
        self.boundary('cross_file_private', [self.fixtures / 'positive/Private.kt', self.fixtures / 'negative/PrivateAccess.kt'], 'invalid_kotlin', r'private|access|visible|unresolved')
        for provider, output in self.outputs.items():
            self.boundary('external_internal_' + provider, [self.fixtures / 'negative/ExternalInternal.kt'], 'invalid_kotlin', r'internal|access|visible|unresolved', dependencies=[output], module='external_consumer')
            self.boundary('external_class_internal_' + provider, [self.fixtures / 'negative/ExternalClassInternal.kt'], 'invalid_kotlin', r'internal|access|visible|unresolved', dependencies=[output], module='external_consumer')
        for name, pattern in [('AnnotationArguments', r'annotation|argument|array|class|enum'), ('PropertyTarget', r'property|target|annotation'), ('TypeUse', r'type.use|annotation|target'), ('PrivateClass', r'private.*class|class.*private'), ('EnumBody', r'enum.*bod|entry.*bod')]:
            self.boundary(name, [self.fixtures / ('unsupported/' + name + '.kt')], 'unsupported_language', r'(?:unsupported|not (?:yet )?supported).*(' + pattern + ')|(' + pattern + ').*(?:unsupported|not (?:yet )?supported)')
        self.boundary('DollarClassifier', [self.fixtures / 'unsupported/DollarClassifier.kt'], 'unsupported_language', exact_line=3, expected_code='CLASSIFIER_DOLLAR_NAME_UNSUPPORTED')
        self.boundary('InternalProperty', [self.fixtures / 'unsupported/InternalProperty.kt'], 'unsupported_language', exact_line=4, expected_code='INTERNAL_PROPERTY_VISIBILITY_UNSUPPORTED')
        self.boundary('JvmName', [self.fixtures / 'unsupported/JvmName.kt'], 'unsupported_language', r'JvmName.*(?:unsupported|not (?:yet )?supported)|(?:unsupported|not (?:yet )?supported).*JvmName')
        self.boundary('JvmSynthetic', [self.fixtures / 'unsupported/JvmSynthetic.kt'], 'unsupported_language', r'JvmSynthetic.*(?:unsupported|not (?:yet )?supported)|(?:unsupported|not (?:yet )?supported).*JvmSynthetic')
        self.boundary('serialization_plugin', [self.fixtures / 'unsupported/PluginRequired.kt'], 'plugin_required', r'serializ.*plugin|plugin.*serializ')
        self.boundary('compose_plugin', [self.fixtures / 'unsupported/ComposeRequired.kt'], 'plugin_required', r'compos.*plugin|plugin.*compos')
        self.boundary('original_gid', [SNAPSHOT / GID], 'unsupported_language', r'value.class.*(?:unsupported|not (?:yet )?supported)|(?:unsupported|not (?:yet )?supported).*value.class', exact_line=4)
        self.boundary('missing_dependency', [self.fixtures / 'negative/MissingDependency.kt'], 'dependency_setup_failure', r'absent|DoesNotExist|unresolved|missing|unknown', exact_line=3)

    def execute(self):
        try:
            self.prepare()
            self.verify_outputs()
            self.boundaries()
            self.outcome('frozen_inputs', all(sha(ROOT / path) == checksum for path, checksum in self.frozen.items()))
            self.outcome('frozen_reference', all(sha(ROOT / path) == checksum for path, checksum in self.reference_hashes.items()))
            if self.args.preflight != 'official':
                self.outcome('frozen_binary', sha(self.compiler) == self.compiler_sha)
                self.outcome('frozen_compiler_sources', all(sha(ROOT / path) == checksum for path, checksum in self.compiler_sources.items()))
            self.report['status'] = 'completed' if all(case.get('boundary_verified', case['outcome'] != 'fail') for case in self.report['cases']) else 'failed'
        except Exception as error:
            self.report['status'] = 'failed'
            self.report['cases'].append(dict(id='harness_failure', outcome='timeout' if isinstance(error, TimeoutError) else 'fail', message=str(error)))
            print(str(error), file=sys.stderr)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['finished_at'] = datetime.now(timezone.utc).isoformat()
        self.report_path.write_text(json.dumps(self.report, indent=2) + '\n')
        print('Report: ' + relative(self.report_path), flush=True)
        return 0 if self.report['status'] == 'completed' else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--compiler-root', type=Path, default=ROOT)
    parser.add_argument('--fixtures', type=Path, default=FIXTURES)
    parser.add_argument('--serialization-jar', type=Path, default=Path('/workspace/korrust-setup/annotation-dependencies/kotlinx-serialization-core-jvm-1.5.1.jar'))
    parser.add_argument('--compose-jar', type=Path, default=Path('/workspace/korrust-setup/annotation-dependencies/runtime-android-1.9.4-classes.jar'))
    parser.add_argument('--preflight', choices=['official', 'candidate'])
    return Harness(parser.parse_args()).execute()


if __name__ == '__main__':
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)
