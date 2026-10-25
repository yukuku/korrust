#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import re
import shutil
import sys

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_androidbible_simple import Harness as LoggedHarness
from check_library_interop import boundary_outcome, frozen_files, rejection_outcome
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / 'tests/fixtures/declarations_twenty'
FEATURES = json.loads((FIXTURES / 'cases.json').read_text())
BOOK_COLOR_SOURCE = 'Alkitab/src/main/java/yuku/alkitab/base/util/BookColorUtil.kt'
READING_SOURCE = 'Alkitab/src/main/java/yuku/alkitab/base/util/ReadingPassage.kt'
LEVENSHTEIN_SOURCE = 'Alkitab/src/main/java/yuku/alkitab/base/util/Levenshtein.kt'
PARSER_SOURCE = 'Alkitab/src/main/java/yuku/alkitabconverter/util/DesktopVerseParser.kt'
READING_JAVA_SOURCES = [
    'AlkitabModel/src/main/java/yuku/alkitab/model/Book.java',
    'AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java',
    'AlkitabModel/src/main/java/yuku/alkitab/util/Ari.java',
]
READING_STUBS = [FIXTURES / 'android/os/Parcel.java', ROOT / 'tests/fixtures/interop/android/os/Parcelable.java', ROOT / 'tests/fixtures/interop/androidx/annotation/NonNull.java']
BOUNDARIES = [
    dict(id='contains_operator_modifier', invalid=True, source='import declarations.library.OrdinaryContains\nfun bad(value: OrdinaryContains): Boolean = 1 in value', candidate_diagnostic_regex='operator|contains'),
    dict(id='get_operator_modifier', invalid=True, source='import declarations.library.OrdinaryIndex\nfun bad(value: OrdinaryIndex): Int = value[0]', candidate_diagnostic_regex='operator|get'),
    dict(id='set_operator_modifier', invalid=True, source='import declarations.library.OrdinaryIndex\nfun bad(value: OrdinaryIndex) { value[0] = 1 }', candidate_diagnostic_regex='operator|set'),
    dict(id='generic_contains_type', invalid=True, source='import declarations.library.Bag\nfun bad(value: Bag<String>): Boolean = 1 in value', candidate_diagnostic_regex='type|argument|String|Int'),
    dict(id='generic_set_type', invalid=True, source='import declarations.library.Slots\nfun bad(value: Slots<String>) { value[0] = 1 }', candidate_diagnostic_regex='type|argument|String|Int|No applicable member overload'),
    dict(id='array_element_type', invalid=True, source='fun bad(value: Array<String>) { value[0] = 1 }', candidate_diagnostic_regex='type|String|Int'),
    dict(id='when_subject_scope', invalid=True, source='fun bad(): Int { when(val value = 1) { else -> value }; return value }', candidate_diagnostic_regex='unresolved|scope|value'),
    dict(id='lateinit_primitive', invalid=True, source='class Bad { lateinit var value: Int }', candidate_diagnostic_regex='lateinit|primitive'),
    dict(id='lateinit_val', invalid=True, source='class Bad { lateinit val value: String }', candidate_diagnostic_regex='lateinit|mutable|val'),
    dict(id='unknown_label', invalid=True, source='fun bad() { while(true) { break@missing } }', candidate_diagnostic_regex='label|missing'),
    dict(id='final_override', invalid=True, source='open class Parent { fun value(): Int = 1 }\nclass Child: Parent() { override fun value(): Int = 2 }', candidate_diagnostic_regex='final|override'),
    dict(id='immutable_backing_field', invalid=True, source='class Bad { val value: Int = 1\nget() { field = 2; return field } }', candidate_diagnostic_regex='field|immutable|val|assign'),
    dict(id='lateinit_initializer', invalid=True, source='class Bad { lateinit var value: String = "x" }', candidate_diagnostic_regex='lateinit|initializer'),
    dict(id='private_access', invalid=True, source='class Secret { private fun value(): Int = 1 }\nfun bad(): Int = Secret().value()', candidate_diagnostic_regex='private|access|visibility|Unresolved member.*value'),
    dict(id='protected_access', invalid=True, source='open class Secret { protected fun value(): Int = 1 }\nfun bad(): Int = Secret().value()', candidate_diagnostic_regex='protected|access|visibility|Unresolved member.*value'),
    dict(id='final_class', invalid=True, source='class Parent\nclass Child: Parent()', candidate_diagnostic_regex='final|inherit'),
    dict(id='duplicate_function', invalid=True, source='fun value(): Int = 1\nfun value(): Int = 2', candidate_diagnostic_regex='duplicate|conflict|overload'),
    dict(id='local_function_scope', invalid=True, source='fun outer() { fun hidden(): Int = 1 }\nfun bad(): Int = hidden()', candidate_diagnostic_regex='unresolved|scope|hidden|No applicable function overload'),
    dict(id='local_capture_scope', invalid=True, source='fun bad(): Int {\n fun capture(): Int = later\n val later = 1\n return capture()\n}', candidate_diagnostic_regex='unresolved|scope|later|capture'),
    dict(id='break_across_lambda', invalid=True, source='import declarations.library.invokeInt\nfun bad() { while(true) { invokeInt { break; 1 } } }', candidate_diagnostic_regex='break|loop|lambda'),
    dict(id='private_inline_import', invalid=True, source='import declarations.library.concealed\nfun bad(): Int = concealed(1)', candidate_diagnostic_regex='private|access|concealed|No applicable function'),
    dict(id='protected_inline_call', invalid=True, source='import declarations.library.ProtectedInline\nfun bad(value: ProtectedInline): Int = value.guarded(1)', candidate_diagnostic_regex='protected|access|guarded|Unresolved member'),
    dict(id='continue_across_lambda', invalid=True, source='import declarations.library.invokeInt\nfun bad() { while(true) { invokeInt { continue; 1 } } }', candidate_diagnostic_regex='continue|loop|lambda'),
]
RESTRICTIONS = {
    item['id']: 'Only the concrete signatures, declaration forms and runtime boundaries in the recorded fixture; no unrestricted Kotlin-language compatibility claim.'
    for item in FEATURES
}
RESTRICTIONS.update(
    operator_contains='Concrete source and compiled generic member contains; requires operator modifier and compatible element type. No arbitrary overload-resolution equivalence claim.',
    operator_index='Concrete source and compiled generic get/set; ordinary named get/set without operator must reject. Recorded single-index and two-index signatures only.',
    delegated_properties='Generic compiled getValue/setValue delegate with actual KProperty.name, instance mutable property and independent per-instance state; actual provideDelegate once per instance and owner identity; no lazy-specific substitution or local-delegate claim.',
    generic_arrays='Recorded Array<String>, Array<String?>, Array<Int> and Array<ArrayItem> construction/iteration with literal initializer lambdas and fixed nonnegative sizes; object arrays use real JVM component types. Primitive arrays are prior coverage; no generic reified type-parameter construction claim.',
    try_expressions='Recorded Int/reference try results, typed reference local joins, catch selection, normal/exception/return finally execution and overriding return; no coroutine or asynchronous exception behavior claim.',
)
NEGATIVE = {
    'local_functions': ['local_function_scope', 'local_capture_scope'],
    'mixed_declarations': ['duplicate_function'],
    'uniform_bodies': ['private_access', 'protected_access'],
    'operator_contains': ['contains_operator_modifier', 'generic_contains_type'],
    'operator_index': ['get_operator_modifier', 'set_operator_modifier', 'generic_set_type'],
    'generic_arrays': ['array_element_type'], 'when_subject_variable': ['when_subject_scope'],
    'lateinit_properties': ['lateinit_primitive', 'lateinit_val', 'lateinit_initializer'], 'labeled_control': ['unknown_label', 'break_across_lambda', 'continue_across_lambda'],
    'inheritance_super': ['final_override', 'final_class'], 'custom_accessors': ['immutable_backing_field'],
}


def java_driver_source(ident):
    source = (FIXTURES / 'DeclarationsCheck.java').read_text()
    lines = [line for line in source.splitlines() if not line.strip().startswith('case "') or line.strip().startswith('case "' + ident + '":')]
    return '\n'.join(lines).replace('public final class DeclarationsCheck {', 'public final class DeclarationsCheck_' + ident + ' {') + '\n'


def effectful_initializer_outcome(returncode, stderr, emitted):
    if not returncode or emitted or not re.search(r'initializer|effectful', stderr, re.I):
        return 'fail'
    return 'unsupported_language' if rejection_outcome(stderr) == 'unsupported_language' else 'fail'


def feature_status(cases, ident):
    outcomes = {case['id']: case['outcome'] for case in cases}
    required = [kind + '/' + ident for kind in ['oracle', 'compile', 'java', 'source_abi', 'class_verification', 'kotlin_consumer']]
    required += ['frozen_inputs'] + ['boundary/' + name for name in NEGATIVE.get(ident, [])]
    expected = {name: 'pass' for name in required}
    if ident == 'generic_arrays':
        expected['array_order/oracle'] = 'pass'
        expected['array_order/effectful_initializer'] = 'unsupported_language'
    if all(outcomes.get(name) == value for name, value in expected.items()):
        return 'supported'
    if outcomes.get('compile/' + ident) == 'unsupported_language':
        return 'unsupported'
    return 'partial' if any(outcomes.get(name) == 'pass' for name in required) else 'unassessed'


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-declarations-twenty'
        self.work = ROOT / 'target/checks' / self.run_id
        self.evidence = self.work / 'evidence' if args.preflight else ROOT / 'reports/correctness' / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix('.json')
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={}, reference_compiler={}, inputs={}, commands=[], evidence=[], cases=[], environment=dict(os=platform.platform(), machine=platform.machine()), selection='Twenty isolated declaration/control-flow/protocol fixtures; fixed Java bytecodes compiled against official output, independent expectations, JVM verification, public ABI and per-fixture official Kotlin metadata consumers. Unchanged BookColorUtil and Levenshtein integrations. ReadingPassage source assessment uses original dependencies and explicit Android type stubs; source-only compilation is not a compatibility pass. No Android/device/full-app/performance claim.', reproduction_commands=[['python3', 'tools/check_declarations_twenty.py', '--compiler', str(args.compiler)] + (['--preflight', args.preflight] if args.preflight else [])])
        self.java, self.javac, self.javap = [str(Path(os.environ['JAVA_HOME']) / 'bin' / name) if os.environ.get('JAVA_HOME') else shutil.which(name) for name in ['java', 'javac', 'javap']]
        self.compiled = []

    def outcome(self, ident, passed, **details):
        self.report['cases'].append(dict(id=ident, outcome='pass' if passed else 'fail', **details))

    def prepare(self):
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        self.compiler = self.args.compiler.resolve()
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'declarations_twenty']
        self.frozen = frozen_files()
        self.reference_hashes = file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar'))
        identity = self.run('Pinned official compiler identity', [self.kotlinc, '-version'])
        if 'kotlinc-jvm ' + pin['version'] + ' ' not in identity.stdout + identity.stderr:
            raise AssertionError('Pinned compiler version mismatch')
        self.report['reference_compiler'] = dict(version=pin['version'], options=self.options, distribution_libs_sha256=self.reference_hashes)
        self.report['environment']['java'] = self.run('JDK identity', [self.java, '-version']).stderr.strip()
        self.report['inputs'].update(fixtures_sha256=file_hashes(FIXTURES.rglob('*.*')), reference_manifest_sha256=sha(ROOT / 'references/compiler.json'), reference_sources_manifest_sha256=sha(ROOT / 'references/sources.json'), repository_revision=self.run('Repository revision', ['git', 'rev-parse', 'HEAD']).stdout.strip())
        if self.args.preflight != 'official':
            self.compiler_sha = sha(self.compiler)
            self.report['compiler'] = dict(version=self.run('Candidate identity', [self.compiler, '--version']).stdout.strip(), binary_sha256=self.compiler_sha, source_files_sha256=self.frozen)
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        self.original_hashes = {name: sha(SNAPSHOT / name) for name in [BOOK_COLOR_SOURCE, READING_SOURCE, LEVENSHTEIN_SOURCE, PARSER_SOURCE, *READING_JAVA_SOURCES]}
        if any(checksum != inventory[name] for name, checksum in self.original_hashes.items()):
            raise AssertionError('Original app snapshot changed')
        self.report['inputs'].update(androidbible_unchanged_sources_sha256=self.original_hashes, androidbible_source_manifest_sha256=sha(ROOT / 'benchmarks/source-files.sha256.json'), android_type_stubs_sha256=file_hashes(READING_STUBS), androidbible_commit=json.loads((ROOT / 'benchmarks/sources.json').read_text())['sources']['androidbible']['commit'])
        self.dependencies = self.work / 'dependencies'
        self.run('Compile independent Java trace', [self.javac, '-d', self.dependencies, FIXTURES / 'Trace.java', FIXTURES / 'JavaBase.java'])
        self.run('Compile generic protocol dependency', [self.kotlinc, FIXTURES / 'Protocols.kt', *self.options, '-cp', self.dependencies, '-d', self.dependencies])
        self.official, self.candidate = self.work / 'official', self.work / 'candidate'
        self.run('Compile all official fixtures', [self.kotlinc, *[FIXTURES / item['fixture'] for item in FEATURES], *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', self.official])
        self.drivers = self.work / 'java-callers'
        driver_sources = []
        for item in FEATURES:
            source = self.work / ('DeclarationsCheck_' + item['id'] + '.java')
            source.write_text(java_driver_source(item['id']))
            self.archive(source, source.name)
            driver_sources.append(source)
        self.run('Compile isolated fixed Java callers against official output', [self.javac, '-cp', classpath(self.official, self.dependencies, self.stdlib), '-d', self.drivers, *driver_sources, ROOT / 'tests/fixtures/ModuleClassVerification.java'])
        self.report['inputs'].update(fixed_java_callers_classes_sha256=tree_hashes(self.drivers, '.class'), official_fixture_outputs_sha256=tree_hashes(self.official), protocol_dependency_outputs_sha256=tree_hashes(self.dependencies))
        self.case('official_class_verification', **self.verify(self.official))

    def verify(self, output):
        names = [name[:-6].replace('/', '.') for name in tree_hashes(output, '.class')]
        result = self.run('Verify every emitted class in ' + output.name, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, output, self.dependencies, self.stdlib), 'ModuleClassVerification', *names], allowed=(0, 1), timeout=15)
        if result.returncode or not names:
            raise AssertionError('Class verification failed in ' + str(output))
        return dict(classes=len(names), jvm_verification='all')

    def inspect(self, ident, official, candidate):
        expected_names = [name[:-6].replace('/', '.') for name in tree_hashes(official, '.class')
                          if not re.search(r'\$[0-9]|\$DefaultImpls\.class$|\$WhenMappings\.class$', name)]
        compared = []
        passed = True
        for name in expected_names:
            expected = self.run('Inspect official ABI ' + name, [self.javap, '-protected', '-s', '-classpath', official, name])
            actual = self.run('Inspect candidate ABI ' + name, [self.javap, '-protected', '-s', '-classpath', candidate, name], allowed=(0, 1))
            matched = actual.returncode == 0 and source_abi(expected.stdout) == source_abi(actual.stdout)
            compared.append(dict(class_name=name, matched=matched))
            passed = passed and matched
        self.outcome('source_abi/' + ident, passed and bool(compared), classes=compared, scope='Public/protected declarations and descriptors; member order normalized, implementation closure classes excluded')
        try:
            details = self.verify(candidate)
            self.case('class_verification/' + ident, **details)
        except AssertionError as error:
            self.outcome('class_verification/' + ident, False, message=str(error))

    def consumer(self, item, official, candidate=None):
        stem = Path(item['fixture']).stem
        observations = []
        providers = [('official', official)] + ([('candidate', candidate)] if candidate is not None else [])
        for provider, output in providers:
            destination = self.work / ('consumer-' + provider) / item['id']
            result = self.run('Compile Kotlin metadata consumer ' + provider + '/' + item['id'], [self.kotlinc, FIXTURES / ('Consumer' + stem + '.kt'), *self.options, '-cp', classpath(output, self.dependencies, self.stdlib), '-d', destination], allowed=(0, 1))
            if result.returncode:
                observations.append(None)
                continue
            run = self.run('Run Kotlin metadata consumer ' + provider + '/' + item['id'], [self.java, '-Xverify:all', '-cp', classpath(destination, output, self.dependencies, self.stdlib), 'declarations.consumer.Consumer' + stem + 'Kt'], allowed=(0, 1), timeout=15)
            observations.append(run.stdout if run.returncode == 0 else None)
        if candidate is not None:
            self.outcome('kotlin_consumer/' + item['id'], observations == ['OK\n', 'OK\n'])
        elif observations != ['OK\n']:
            raise AssertionError('Official Kotlin consumer failed: ' + item['id'])

    def probes(self):
        for item in FEATURES:
            ident = item['id']
            expected = self.run('Official independent assertions ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, self.official, self.dependencies, self.stdlib), 'DeclarationsCheck_' + ident, ident], timeout=15)
            self.case('oracle/' + ident, runtime_checks=json.loads(expected.stdout)['checks'])
            if self.args.preflight == 'official':
                self.consumer(item, self.official)
                continue
            destination = self.candidate / ident
            result = self.run('Compile candidate ' + ident, [self.compiler, FIXTURES / item['fixture'], '-cp', classpath(self.dependencies, self.stdlib), '-d', destination], allowed=(0, 1))
            if result.returncode:
                self.report['cases'].append(dict(id='compile/' + ident, outcome=rejection_outcome(result.stderr) if not tree_hashes(destination, '.class') else 'fail', official_outcome='accepted', diagnostics=result.stderr, source_sha256=sha(FIXTURES / item['fixture'])))
                continue
            self.compiled.append(ident)
            self.case('compile/' + ident, emitted_classes_sha256=tree_hashes(destination, '.class'), source_sha256=sha(FIXTURES / item['fixture']))
            actual = self.run('Candidate independent assertions ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, destination, self.dependencies, self.stdlib), 'DeclarationsCheck_' + ident, ident], allowed=(0, 1), timeout=15)
            self.outcome('java/' + ident, actual.returncode == 0 and actual.stdout == expected.stdout, runtime_checks=json.loads(expected.stdout)['checks'], oracle='Identical Java bytecodes compiled only against official output; independent expectations')
            official = self.work / 'official-isolated' / ident
            self.run('Compile isolated official ABI fixture ' + ident, [self.kotlinc, FIXTURES / item['fixture'], *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', official])
            self.inspect(ident, official, destination)
            self.consumer(item, official, destination)

    def boundaries(self):
        for boundary in BOUNDARIES:
            source = self.work / ('Boundary_' + boundary['id'] + '.kt')
            source.write_text(boundary['source'] + '\n')
            self.archive(source, source.name)
            self.run('Official rejection ' + boundary['id'], [self.kotlinc, source, *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', self.work / ('official-boundary-' + boundary['id'])], allowed=(1,))
            self.case('oracle_boundary/' + boundary['id'], official_outcome='rejected', source_sha256=sha(source))
            if self.args.preflight == 'official':
                continue
            destination = self.work / ('candidate-boundary-' + boundary['id'])
            result = self.run('Candidate rejection ' + boundary['id'], [self.compiler, source, '-cp', classpath(self.dependencies, self.stdlib), '-d', destination], allowed=(0, 1))
            self.report['cases'].append(dict(id='boundary/' + boundary['id'], outcome=boundary_outcome(boundary, result.returncode, result.stderr, tree_hashes(destination, '.class')), diagnostics=result.stderr, official_outcome='rejected', source_sha256=sha(source)))

    def array_order_boundary(self):
        official = self.work / 'array-order-official'
        source = FIXTURES / 'ArrayFactoryOrder.kt'
        self.run('Compile official effectful array initializer probe', [self.kotlinc, source, *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', official])
        drivers = self.work / 'array-order-drivers'
        self.run('Compile fixed array order Java oracle caller', [self.javac, '-cp', classpath(official, self.dependencies, self.stdlib), '-d', drivers, FIXTURES / 'ArrayFactoryCheck.java'])
        observed = self.run('Observe pinned official array allocation/factory order', [self.java, '-Xverify:all', '-cp', classpath(drivers, official, self.dependencies, self.stdlib), 'ArrayFactoryCheck'], timeout=15)
        self.case('array_order/oracle', **json.loads(observed.stdout))
        self.run('Inspect pinned official array allocation ordering bytecode', [self.javap, '-c', '-p', '-classpath', official, 'declarations.boundary.ArrayFactoryOrderKt'])
        specs = [dict(source='references/kotlin-spec/docs/src/md/kotlin.core/builtins.md', line=218), dict(source='references/kotlin-spec/docs/src/md/kotlin.core/expressions.md', line=1131)]
        for spec in specs:
            spec['source_sha256'] = sha(ROOT / spec['source'])
        self.report['reference_discrepancies'] = [dict(id='array_initializer_allocation_order', status='suspected', source=relative(source), source_sha256=sha(source), specification=specs, evidence_cases=['array_order/oracle', 'array_order/effectful_initializer'], observation='Pinned compiler allocates Array before evaluating initializer(): negative size skips the factory; zero size evaluates it once, producing event9.', interpretation='The specification describes an inline Array constructor and ordinary call arguments evaluated left-to-right before invocation. Whether the special inline constructor permits earlier allocation needs clarification; this report does not resolve the discrepancy.', resolution='Effectful initializer-producing expressions are explicitly unsupported. Recorded positive array support covers literal lambdas at fixed nonnegative sizes; no broader runtime-order claim.')]
        if self.args.preflight != 'official':
            candidate = self.work / 'array-order-candidate'
            result = self.run('Reject effectful array initializer pending specification resolution', [self.compiler, source, '-cp', classpath(self.dependencies, self.stdlib), '-d', candidate], allowed=(0, 1))
            self.report['cases'].append(dict(id='array_order/effectful_initializer', outcome=effectful_initializer_outcome(result.returncode, result.stderr, tree_hashes(candidate, '.class')), expected_outcome='unsupported_language', official_outcome='accepted', diagnostics=result.stderr, reason='Explicit bounded rejection pending recorded specification/oracle order discrepancy; not a correctness pass against the oracle.'))

    def androidbible(self):
        self.case('androidbible/unchanged_sources', files=len(self.original_hashes))
        self.archive(SNAPSHOT / BOOK_COLOR_SOURCE, 'original-BookColorUtil.kt')
        official = self.work / 'app-official'
        self.run('Compile unchanged BookColorUtil official', [self.kotlinc, SNAPSHOT / BOOK_COLOR_SOURCE, *self.options, '-d', official])
        drivers = self.work / 'app-drivers'
        self.run('Compile fixed app Java caller', [self.javac, '-cp', classpath(official, self.stdlib), '-d', drivers, FIXTURES / 'AppCheck.java'])
        expected = self.run('Official unchanged app assertions', [self.java, '-Xverify:all', '-cp', classpath(drivers, official, self.stdlib), 'AppCheck'], timeout=15)
        self.case('androidbible/book_color_oracle', runtime_checks=json.loads(expected.stdout)['checks'])
        if self.args.preflight != 'official':
            candidate = self.work / 'app-candidate'
            result = self.run('Compile unchanged BookColorUtil candidate', [self.compiler, SNAPSHOT / BOOK_COLOR_SOURCE, '-cp', self.stdlib, '-d', candidate], allowed=(0, 1))
            if result.returncode:
                self.report['cases'].append(dict(id='androidbible/book_color_compile', outcome=rejection_outcome(result.stderr) if not tree_hashes(candidate, '.class') else 'fail', diagnostics=result.stderr))
            else:
                self.case('androidbible/book_color_compile', emitted_classes_sha256=tree_hashes(candidate, '.class'))
                actual = self.run('Candidate unchanged app assertions', [self.java, '-Xverify:all', '-cp', classpath(drivers, candidate, self.stdlib), 'AppCheck'], allowed=(0, 1), timeout=15)
                self.outcome('androidbible/book_color_java', actual.returncode == 0 and actual.stdout == expected.stdout, runtime_checks=24)
                self.inspect('androidbible_book_color', official, candidate)
        self.archive(SNAPSHOT / LEVENSHTEIN_SOURCE, 'original-Levenshtein.kt')
        official = self.work / 'levenshtein-official'
        self.run('Compile unchanged Levenshtein official', [self.kotlinc, SNAPSHOT / LEVENSHTEIN_SOURCE, *self.options, '-d', official])
        drivers = self.work / 'levenshtein-drivers'
        self.run('Compile fixed Levenshtein Java caller', [self.javac, '-cp', classpath(official, self.stdlib), '-d', drivers, FIXTURES / 'LevenshteinCheck.java'])
        expected = self.run('Official Levenshtein assertions', [self.java, '-Xverify:all', '-cp', classpath(drivers, official, self.stdlib), 'LevenshteinCheck'], timeout=15)
        self.case('androidbible/levenshtein_oracle', runtime_checks=json.loads(expected.stdout)['checks'])
        if self.args.preflight != 'official':
            candidate = self.work / 'levenshtein-candidate'
            result = self.run('Compile unchanged Levenshtein candidate', [self.compiler, SNAPSHOT / LEVENSHTEIN_SOURCE, '-cp', self.stdlib, '-d', candidate], allowed=(0, 1))
            if result.returncode:
                self.report['cases'].append(dict(id='androidbible/levenshtein_compile', outcome=rejection_outcome(result.stderr) if not tree_hashes(candidate, '.class') else 'fail', diagnostics=result.stderr))
            else:
                self.case('androidbible/levenshtein_compile', emitted_classes_sha256=tree_hashes(candidate, '.class'))
                actual = self.run('Candidate Levenshtein assertions', [self.java, '-Xverify:all', '-cp', classpath(drivers, candidate, self.stdlib), 'LevenshteinCheck'], allowed=(0, 1), timeout=15)
                self.outcome('androidbible/levenshtein_java', actual.returncode == 0 and actual.stdout == expected.stdout, runtime_checks=9)
                self.inspect('androidbible_levenshtein', official, candidate)
        self.reading_passage()

    def reading_passage(self):
        dependencies = self.work / 'reading-dependencies'
        self.run('Compile original ReadingPassage Java dependencies with explicit Android type stubs', [self.javac, *READING_STUBS, *[SNAPSHOT / name for name in READING_JAVA_SOURCES], '-d', dependencies])
        self.run('Compile original actual app DesktopVerseParser dependency official', [self.kotlinc, SNAPSHOT / PARSER_SOURCE, *self.options, '-cp', classpath(dependencies, self.stdlib), '-d', dependencies])
        oracle = self.run('Compile unchanged ReadingPassage official', [self.kotlinc, SNAPSHOT / READING_SOURCE, *self.options, '-cp', classpath(dependencies, self.stdlib), '-d', self.work / 'reading-official'], allowed=(0, 1))
        details = dict(source=relative(SNAPSHOT / READING_SOURCE), source_sha256=sha(SNAPSHOT / READING_SOURCE), official_exit_code=oracle.returncode, dependency_sources=[relative(SNAPSHOT / name) for name in [PARSER_SOURCE, *READING_JAVA_SOURCES]], android_type_stubs=[relative(path) for path in READING_STUBS], reason='Source compilation assessment only; original Book/Ari/IntArrayList and official original app DesktopVerseParser dependency, explicit throwing Parcel/Parcelable/NonNull type stubs. No runtime, metadata, Android or full-app compatibility claim.')
        if self.args.preflight != 'official':
            candidate = self.work / 'reading-candidate'
            result = self.run('Assess unchanged ReadingPassage candidate with real dependency closure', [self.compiler, SNAPSHOT / READING_SOURCE, '-cp', classpath(dependencies, self.stdlib), '-d', candidate], allowed=(0, 1))
            details.update(candidate_exit_code=result.returncode, diagnostics=result.stderr, emitted_classes_sha256=tree_hashes(candidate, '.class'))
        self.report['cases'].append(dict(id='androidbible/reading_passage', outcome='not_assessed', **details))

    def features(self):
        report = {key: self.report[key] for key in ['schema_version', 'created_at', 'compiler', 'reference_compiler', 'inputs']}
        report.update(kind='features', status='assessed', commands=self.report['reproduction_commands'], evidence=[relative(self.report_path)], features=[])
        for item in FEATURES:
            ident = item['id']
            related = [case['id'] for case in self.report['cases'] if case['id'].endswith('/' + ident) or case['id'] == 'frozen_inputs' or case['id'] in ['boundary/' + name for name in NEGATIVE.get(ident, [])] or (ident == 'generic_arrays' and case['id'].startswith('array_order/'))]
            report['features'].append(dict(name=ident, status=feature_status(self.report['cases'], ident), scope=item['scope'], evidence_cases=related, restrictions=[RESTRICTIONS[ident], 'Evidence limited to recorded fixtures/dependencies; no full-app, Android runtime or compilation-performance claim.']))
        (ROOT / 'reports/features' / (self.run_id + '.json')).write_text(json.dumps(report, indent=2) + '\n')

    def execute(self):
        try:
            self.prepare()
            self.probes()
            self.boundaries()
            self.array_order_boundary()
            self.androidbible()
            if not self.args.preflight:
                pin = json.loads((ROOT / 'references/compiler.json').read_text())
                if self.frozen != frozen_files() or self.reference_hashes != file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar')) or sha(self.compiler) != self.compiler_sha:
                    raise AssertionError('Compiler, fixtures, tools or pinned JARs changed during frozen run')
                if any(sha(SNAPSHOT / name) != checksum for name, checksum in self.original_hashes.items()):
                    raise AssertionError('Original app sources changed')
                self.case('frozen_inputs', compiler_binary_sha256=self.compiler_sha, source_files=len(self.frozen))
            self.report['status'] = 'failed' if any(case['outcome'] in ['fail', 'timeout'] for case in self.report['cases']) else 'completed'
        except Exception as error:
            self.report['status'] = 'failed'
            self.report['cases'].append(dict(id='harness_failure', outcome='timeout' if isinstance(error, TimeoutError) else 'fail', message=str(error)))
            print(str(error), file=sys.stderr)
        finally:
            self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
            self.report['finished_at'] = datetime.now(timezone.utc).isoformat()
            self.report_path.write_text(json.dumps(self.report, indent=2) + '\n')
            if not self.args.preflight:
                self.features()
            print('Report: ' + relative(self.report_path), flush=True)
        return 0 if self.report['status'] == 'completed' else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--preflight', choices=['official', 'candidate'])
    args = parser.parse_args()
    try:
        return Harness(args).execute()
    finally:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)


if __name__ == '__main__':
    sys.exit(main())
