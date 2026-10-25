#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import platform
import shutil
import sys

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_androidbible_simple import Harness as LoggedHarness
from check_library_interop import boundary_outcome, frozen_files, rejection_outcome
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / 'tests/fixtures/next_twenty'
FEATURES = json.loads((FIXTURES / 'cases.json').read_text())
APP_SOURCE = 'Alkitab/src/main/java/yuku/alkitab/base/widget/VerseNumberPlacement.kt'
BOOK_COLOR_SOURCE = 'Alkitab/src/main/java/yuku/alkitab/base/util/BookColorUtil.kt'
APP_PROBE = 'Alkitab/src/main/java/yuku/alkitab/base/util/ReadingPassage.kt'
CITATIONS = {
    'inclusive_ranges': ('base/util/Levenshtein.kt', 'for (i in 0..m)'),
    'descending_progressions': ('base/verses/VersesDataModel.kt', 'downTo'),
    'stepped_progressions': ('base/util/ReadingPassage.kt', 'indices step 2'),
    'array_iteration': ('base/verses/VersesControllerImpl.kt', 'for (checkedPosition in checkedPositions)'),
    'string_iteration': ('base/widget/VerseRenderer.kt', 'for (c in field.toString())'),
    'list_iteration': ('datatransfer/process/ExportProcess.kt', 'for (entry in'),
    'indices_iteration': ('songs/SongBookUtil.kt', 'for (i in infos.indices)'),
    'float_arithmetic': ('songs/SongComposeContent.kt', 'overflow * widths[i] / unfrozenTotal'),
    'float_comparisons': ('base/util/LabelColorUtil.kt', 'hsl[2] > 0.5f'),
    'float_conversions': ('songs/SongComposeContent.kt', '.toFloat()'),
    'compound_assignments': ('base/util/LabelColorUtil.kt', 'hsl[2] += 0.44f'),
    'decrement': ('base/verses/VersesControllerImpl.kt', 'blockStartPos--'),
    'reference_identity': ('base/util/DownloadMapper.kt', 'currentByKey[row.key] === row'),
    'structural_equality': ('base/storage/room/SongInfoEntity.kt', 'bookName == other.bookName'),
    'type_tests': ('base/util/LabelColorUtil.kt', 'bg is GradientDrawable'),
    'checked_casts': ('base/widget/ReaderToolbarLayout.kt', 'root.parent as FrameLayout'),
    'safe_casts': ('base/widget/ColorPreference.kt', 'defaultValue as? Int'),
    'type_smartcasts': ('base/util/LabelColorUtil.kt', 'if (current is GradientDrawable)'),
    'when_type_conditions': ('songs/newdoc/SongDocumentSearch.kt', 'is RowBlock ->'),
}
RESTRICTIONS = {
    'inclusive_ranges': 'Int range iteration only; no general user-defined range operators or Long/Char progression claim.',
    'descending_progressions': 'Ordinary/infix calls resolve actual compiled extension signatures, including custom stride/downTo; bounded supported reference/primitive signatures, no arbitrary overload-resolution equivalence claim.',
    'stepped_progressions': 'Runtime kotlin.ranges.IntProgression iteration reads first/last/step; constructor semantics come from actual called library methods. No Long/Char progression claim.',
    'array_iteration': 'Recorded IntArray, ByteArray and CharArray foreach only; no arbitrary object-array or multidimensional-array claim.',
    'string_iteration': 'UTF-16 code-unit iteration over supported String receivers; no Unicode code-point iteration claim.',
    'list_iteration': 'Recorded List<String>, List<String?>, List<Long>, Iterable<String>, Iterable<Long?> and a compiled custom Iterable<String> implementation; generic iterator dispatch and mutable-list concurrent modification. Arbitrary iterator/operator protocols remain outside this scope.',
    'indices_iteration': 'Supported compiled extension-property getter metadata and receiver lowering resolve arbitrary imported names, demonstrated by String.weight through an explicit alias and String.mass through a wildcard import, with rejection of the hidden original alias name; IntArray/String/List indices remain iteration examples. No delegated properties, property references or unrestricted generic receiver inference claim.',
    'do_while': 'Ordinary post-test loops with break/continue; labeled loop control and arbitrary return-path analysis are outside this claim.',
    'float_arithmetic': 'Float operands only, including IEEE NaN/infinity/signed zero; Double and general mixed-number promotion are outside this scope.',
    'float_comparisons': 'Statically Float and nullable Float equality/order; boxed Any equality is exercised separately with Float.equals semantics.',
    'float_conversions': 'Float to Int/Long and Int/Long to Float with pinned truncation/saturation; no Double, Short or unsigned conversion claim.',
    'compound_assignments': 'Mutable initialized numeric locals only, with Int/Long/Float arithmetic; property/array lvalues and arbitrary plusAssign operator dispatch remain outside this claim.',
    'decrement': 'Prefix/postfix Int local decrement only; arbitrary lvalues, overloaded dec and other numeric types are outside this scope.',
    'reference_identity': 'Nullable reference identity and ordered operands; primitive identity and boxing-identity inference are outside this scope.',
    'structural_equality': 'Nullable String/Object equals dispatch and boxed Float semantics; ordinary object/ThreadLocal reference equality regressions included.',
    'type_tests': 'Reifiable String, CharSequence and nullable String tests; erased generic type arguments reject.',
    'checked_casts': 'Recorded String/String?/boxed Int casts with null/type exceptions; generic and unchecked erasure casts are outside this scope.',
    'safe_casts': 'Recorded String and boxed Int safe casts; generic and unchecked erasure casts are outside this scope.',
    'type_smartcasts': 'Stable parameter String refinements in if branches and early-return guards; mutable/unstable property flow remains outside this claim.',
    'when_type_conditions': 'Explicit-else String/CharSequence is/!is conditions and stable subject refinement, plus Boolean-subject/subjectless regression checks; sealed exhaustiveness and guards remain outside this scope.',
}

BOUNDARIES = [
    dict(id='aliased_original_property', invalid=True, source='import next.library.*\nimport next.library.weight as measured\nfun bad(value: String): Int = value.weight', candidate_diagnostic_regex='unresolved|unknown|property|weight'),
    dict(id='noninfix_extension', invalid=True, source='import next.library.ordinary\nfun bad(): Int = 1 ordinary 2', candidate_diagnostic_regex='infix|modifier|unsupported'),
    dict(id='immutable_compound', invalid=True, source='fun bad(): Int { val x = 1; x -= 1; return x }', candidate_diagnostic_regex='immutable|mutable|val|assign'),
    dict(id='immutable_decrement', invalid=True, source='fun bad(): Int { val x = 1; x--; return x }', candidate_diagnostic_regex='immutable|mutable|val|decrement'),
    dict(id='nonboolean_do_condition', invalid=True, source='fun bad(): Int { do { } while (1); return 0 }', candidate_diagnostic_regex='Boolean|condition|type'),
    dict(id='erased_generic_test', invalid=True, source='fun bad(x: Any): Boolean = x is List<String>', candidate_diagnostic_regex='erased|generic|type argument|unsupported'),
]


def feature_status(cases, ident):
    outcomes = {case['id']: case['outcome'] for case in cases}
    required = [kind + '/' + ident for kind in ['oracle', 'compile', 'java', 'source_abi', 'class_verification']]
    required += ['kotlin_consumer', 'frozen_inputs']
    negative = {'compound_assignments': 'immutable_compound', 'decrement': 'immutable_decrement', 'do_while': 'nonboolean_do_condition', 'indices_iteration': 'aliased_original_property'}
    if ident in negative:
        required.append('boundary/' + negative[ident])
    if ident == 'descending_progressions':
        required += ['source_abi/source_infix_extension', 'source_abi/source_infix_member', 'boundary/noninfix_extension']
    if ident in ['descending_progressions', 'stepped_progressions']:
        required += ['shadow/' + ('downTo' if ident == 'descending_progressions' else 'step')]
    if all(outcomes.get(name) == 'pass' for name in required):
        return 'supported'
    if outcomes.get('compile/' + ident) == 'unsupported_language':
        return 'unsupported'
    return 'partial' if any(outcomes.get(name) == 'pass' for name in required) else 'unassessed'


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-next-twenty'
        self.work = ROOT / 'target/checks' / self.run_id
        self.evidence = self.work / 'evidence' if args.preflight else ROOT / 'reports/correctness' / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix('.json')
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={}, reference_compiler={}, inputs={}, commands=[], evidence=[], cases=[], environment=dict(os=platform.platform(), machine=platform.machine()), selection='Twenty bounded language/progression-iteration additions. Fixed Java callers compile once against official output and execute against both providers with independent expectations and JVM verification. Official Kotlin metadata consumers and facade ABI checks. Unchanged VerseNumberPlacement is source regression coverage; ReadingPassage is assessed separately without a built dependency closure. No app/device/performance claim.', reproduction_commands=[['python3', 'tools/check_next_twenty.py', '--compiler', str(args.compiler)]])
        self.java, self.javac, self.javap = [str(Path(os.environ['JAVA_HOME']) / 'bin' / name) if os.environ.get('JAVA_HOME') and (Path(os.environ['JAVA_HOME']) / 'bin' / name).is_file() else shutil.which(name) for name in ['java', 'javac', 'javap']]
        if not all([self.java, self.javac, self.javap]):
            raise RuntimeError('JDK tools required')

    def outcome(self, name, passed, **details):
        self.report['cases'].append(dict(id=name, outcome='pass' if passed else 'fail', **details))

    def prepare(self):
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        self.compiler = self.args.compiler.resolve()
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'next_twenty']
        self.frozen = frozen_files()
        self.reference_hashes = file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar'))
        version = self.run('Pinned official compiler identity', [self.kotlinc, '-version'])
        if 'kotlinc-jvm ' + pin['version'] + ' ' not in version.stdout + version.stderr:
            raise AssertionError('Pinned compiler version mismatch')
        self.report['reference_compiler'] = dict(version=pin['version'], options=self.options, distribution_libs_sha256=self.reference_hashes)
        self.report['environment']['java'] = self.run('JDK identity', [self.java, '-version']).stderr.strip()
        self.report['inputs'].update(fixtures_sha256=file_hashes(FIXTURES.rglob('*.*')), reference_manifest_sha256=sha(ROOT / 'references/compiler.json'), reference_sources_manifest_sha256=sha(ROOT / 'references/sources.json'), androidbible_commit=json.loads((ROOT / 'benchmarks/sources.json').read_text())['sources']['androidbible']['commit'], repository_revision=self.run('Repository revision', ['git', 'rev-parse', 'HEAD']).stdout.strip())
        if self.args.preflight != 'official':
            self.compiler_sha = sha(self.compiler)
            self.report['compiler'] = dict(version=self.run('Candidate identity', [self.compiler, '--version']).stdout.strip(), binary_sha256=self.compiler_sha, source_files_sha256=self.frozen, jvm_target_scope='Official JVM1.8 versus supported candidate classfile49/52 subset; source/JVM contracts, not bytecode equivalence')
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        app_files, citations = {APP_SOURCE, APP_PROBE, BOOK_COLOR_SOURCE}, {}
        for ident, (suffix, needle) in CITATIONS.items():
            name = 'Alkitab/src/main/java/yuku/alkitab/' + suffix
            matches = [dict(line=n, text=line.strip()) for n, line in enumerate((SNAPSHOT / name).read_text().splitlines(), 1) if needle in line]
            if not matches:
                raise AssertionError('Source citation no longer matches ' + ident)
            citations[ident] = dict(source=relative(SNAPSHOT / name), source_sha256=sha(SNAPSHOT / name), examples=matches)
            app_files.add(name)
        citations['do_while'] = dict(status='not_assessed', reason='No do-while occurrence found in selected AndroidBible Kotlin snapshot; reusable control flow assessed by differential fixtures')
        self.original_hashes = {name: sha(SNAPSHOT / name) for name in sorted(app_files)}
        if any(checksum != inventory[name] for name, checksum in self.original_hashes.items()):
            raise AssertionError('Original benchmark source changed')
        self.report['inputs'].update(androidbible_unchanged_sources_sha256=self.original_hashes, androidbible_feature_usage=citations, androidbible_source_manifest_sha256=sha(ROOT / 'benchmarks/source-files.sha256.json'))
        self.dependencies = self.work / 'dependencies'
        self.run('Compile independent Java dependency', [self.javac, '-d', self.dependencies, FIXTURES / 'Tracker.java', FIXTURES / 'Words.java'])
        self.run('Compile independent shadowing extension library', [self.kotlinc, FIXTURES / 'ShadowLibrary.kt', *self.options, '-d', self.dependencies])
        self.official, self.candidate = self.work / 'official', self.work / 'candidate'
        self.run('Compile all official feature probes', [self.kotlinc, *[FIXTURES / item['fixture'] for item in FEATURES], FIXTURES / 'ObjectEquality.kt', FIXTURES / 'SourceExtensions.kt', FIXTURES / 'SourceMember.kt', *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', self.official])
        self.drivers = self.work / 'java-callers'
        self.run('Compile fixed Java against official output', [self.javac, '-cp', classpath(self.official, self.dependencies, self.stdlib), '-d', self.drivers, FIXTURES / 'NextTwentyCheck.java', ROOT / 'tests/fixtures/ModuleClassVerification.java'])
        names = [name[:-6].replace('/', '.') for name in tree_hashes(self.official, '.class')]
        self.run('Verify every official emitted probe class', [self.java, '-Xverify:all', '-cp', classpath(self.drivers, self.official, self.dependencies, self.stdlib), 'ModuleClassVerification', *names], timeout=15)
        self.case('official_class_verification', classes=len(names))
        self.compiled = []

    def inspect(self, ident, facade, official, candidate):
        abi = [self.run('Inspect ' + name + ' ABI ' + ident, [self.javap, '-protected', '-s', '-classpath', directory, facade]).stdout for name, directory in [('official', official), ('candidate', candidate)]]
        self.outcome('source_abi/' + ident, source_abi(abi[0]) == source_abi(abi[1]), scope='All public/protected signatures and descriptors; normalize member order only')
        names = [name[:-6].replace('/', '.') for name in tree_hashes(candidate, '.class')]
        verified = self.run('Verify every candidate class ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, candidate, self.dependencies, self.stdlib), 'ModuleClassVerification', *names], allowed=(0, 1), timeout=15)
        self.outcome('class_verification/' + ident, verified.returncode == 0 and bool(names), classes=len(names))

    def probes(self):
        for item in FEATURES:
            ident = item['id']
            expected = self.run('Official independent assertions ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, self.official, self.dependencies, self.stdlib), 'NextTwentyCheck', ident], timeout=15)
            self.case('oracle/' + ident, runtime_checks=json.loads(expected.stdout)['checks'])
            if self.args.preflight == 'official':
                continue
            destination = self.candidate / ident
            if ident == 'descending_progressions':
                for extra in ['SourceExtensions.kt', 'SourceMember.kt']:
                    self.run('Compile source infix declaration ' + extra, [self.compiler, FIXTURES / extra, '-cp', classpath(self.dependencies, self.stdlib), '-d', destination])
            if ident == 'structural_equality':
                self.run('Compile object equality regression', [self.compiler, FIXTURES / 'ObjectEquality.kt', '-cp', classpath(self.dependencies, self.stdlib), '-d', destination])
            result = self.run('Compile candidate ' + ident, [self.compiler, FIXTURES / item['fixture'], '-cp', classpath(*([destination] if destination.exists() else []), self.dependencies, self.stdlib), '-d', destination], allowed=(0, 1))
            if result.returncode:
                outcome = rejection_outcome(result.stderr) if not tree_hashes(destination, '.class') else 'fail'
                self.report['cases'].append(dict(id='compile/' + ident, outcome=outcome, official_outcome='accepted', diagnostics=result.stderr, source_sha256=sha(FIXTURES / item['fixture'])))
                continue
            self.compiled.append(ident)
            self.case('compile/' + ident, emitted_classes_sha256=tree_hashes(destination, '.class'), source_sha256=sha(FIXTURES / item['fixture']))
            actual = self.run('Candidate independent assertions ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.drivers, destination, self.dependencies, self.stdlib), 'NextTwentyCheck', ident], allowed=(0, 1), timeout=15)
            self.outcome('java/' + ident, actual.returncode == 0 and actual.stdout == expected.stdout, runtime_checks=json.loads(expected.stdout)['checks'], jvm_verification='all', oracle='Fixed identical Java bytecodes compiled against official output with independent assertions')
            self.inspect(ident, item['facade'], self.official, destination)
            if ident == 'descending_progressions':
                for extra, facade in [('source_infix_extension', 'next.probes.SourceExtensionsKt'), ('source_infix_member', 'next.probes.InfixBox')]:
                    self.inspect(extra, facade, self.official, destination)

    def consumers(self):
        providers = [('official', [self.official])]
        if self.args.preflight != 'official' and len(self.compiled) == len(FEATURES):
            providers.append(('candidate', [self.candidate / item['id'] for item in FEATURES]))
        observations = []
        for name, outputs in providers:
            destination = self.work / ('consumer-' + name)
            self.run('Compile official Kotlin consumer against ' + name, [self.kotlinc, FIXTURES / 'Consumer.kt', *self.options, '-cp', classpath(*outputs, self.dependencies, self.stdlib), '-d', destination])
            observations.append(self.run('Run Kotlin metadata consumer ' + name, [self.java, '-Xverify:all', '-cp', classpath(destination, *outputs, self.dependencies, self.stdlib), 'next.consumer.ConsumerKt'], timeout=15).stdout)
        if len(providers) == 2:
            self.outcome('kotlin_consumer', observations[0] == observations[1], runtime_checks=26)
        elif self.args.preflight != 'official':
            self.report['cases'].append(dict(id='kotlin_consumer', outcome='not_run', reason='Not all candidate feature fixtures compiled'))

    def boundaries(self):
        for boundary in BOUNDARIES:
            source = self.work / ('Boundary_' + boundary['id'] + '.kt')
            source.write_text(boundary['source'] + '\n')
            self.archive(source, source.name)
            self.run('Official rejection ' + boundary['id'], [self.kotlinc, source, *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', self.work / ('official-boundary-' + boundary['id'])], allowed=(1,))
            if self.args.preflight == 'official':
                continue
            destination = self.work / ('candidate-boundary-' + boundary['id'])
            result = self.run('Candidate rejection ' + boundary['id'], [self.compiler, source, '-cp', classpath(self.dependencies, self.stdlib), '-d', destination], allowed=(0, 1))
            self.report['cases'].append(dict(id='boundary/' + boundary['id'], outcome=boundary_outcome(boundary, result.returncode, result.stderr, tree_hashes(destination, '.class')), official_outcome='rejected', diagnostics=result.stderr, source_sha256=sha(source)))
        shadow_official = self.work / 'shadow-official'
        self.run('Compile official custom progression imports', [self.kotlinc, FIXTURES / 'ShadowDownTo.kt', FIXTURES / 'ShadowStep.kt', *self.options, '-cp', classpath(self.dependencies, self.stdlib), '-d', shadow_official])
        drivers = self.work / 'shadow-drivers'
        self.run('Compile fixed shadow Java caller', [self.javac, '-cp', classpath(shadow_official, self.dependencies, self.stdlib), '-d', drivers, FIXTURES / 'ShadowCheck.java'])
        for name, source in [('downTo', 'ShadowDownTo.kt'), ('step', 'ShadowStep.kt')]:
            expected = self.run('Official shadow assertions ' + name, [self.java, '-Xverify:all', '-cp', classpath(drivers, shadow_official, self.dependencies, self.stdlib), 'ShadowCheck', name], timeout=15)
            if self.args.preflight == 'official':
                continue
            destination = self.work / ('shadow-candidate-' + name)
            result = self.run('Candidate shadow ' + name, [self.compiler, FIXTURES / source, '-cp', classpath(self.dependencies, self.stdlib), '-d', destination], allowed=(0, 1))
            if result.returncode:
                passed = False
                details = dict(behavior='Custom infix must execute its actual declared library body; rejection is not sufficient', diagnostics=result.stderr, custom_extension_support='unsupported')
            else:
                actual = self.run('Candidate shadow assertions ' + name, [self.java, '-Xverify:all', '-cp', classpath(drivers, destination, self.dependencies, self.stdlib), 'ShadowCheck', name], allowed=(0, 1), timeout=15)
                passed = actual.returncode == 0 and actual.stdout == expected.stdout
                details = dict(behavior='Imported custom extension executes, not built-in progression lowering')
            self.outcome('shadow/' + name, passed, **details)

    def androidbible(self):
        self.case('androidbible/unchanged_sources', files=len(self.original_hashes), scope='Cited sources match manifest; VerseNumberPlacement compiled unchanged as regression')
        self.archive(SNAPSHOT / APP_SOURCE, 'original-VerseNumberPlacement.kt')
        official = self.work / 'app-official'
        self.run('Compile unchanged VerseNumberPlacement official', [self.kotlinc, SNAPSHOT / APP_SOURCE, *self.options, '-d', official])
        drivers = self.work / 'app-drivers'
        self.run('Compile fixed app Java against official output', [self.javac, '-cp', classpath(official, self.stdlib), '-d', drivers, FIXTURES / 'AppCheck.java'])
        expected = self.run('Original app official assertions', [self.java, '-Xverify:all', '-cp', classpath(drivers, official, self.stdlib), 'AppCheck'], timeout=15)
        self.case('androidbible/oracle', runtime_checks=json.loads(expected.stdout)['checks'])
        if self.args.preflight == 'official':
            return
        destination = self.work / 'app-candidate'
        result = self.run('Compile unchanged VerseNumberPlacement candidate', [self.compiler, SNAPSHOT / APP_SOURCE, '-cp', self.stdlib, '-d', destination], allowed=(0, 1))
        if result.returncode:
            self.report['cases'].append(dict(id='androidbible/compile', outcome=rejection_outcome(result.stderr), diagnostics=result.stderr))
        else:
            self.case('androidbible/compile', emitted_classes_sha256=tree_hashes(destination, '.class'))
            actual = self.run('Original app candidate assertions', [self.java, '-Xverify:all', '-cp', classpath(drivers, destination, self.stdlib), 'AppCheck'], allowed=(0, 1), timeout=15)
            self.outcome('androidbible/java', actual.returncode == 0 and actual.stdout == expected.stdout, runtime_checks=8)
            self.inspect('androidbible', 'yuku.alkitab.base.widget.VerseNumberPlacementKt', official, destination)
        self.run('Official acceptance of unchanged BookColorUtil assessment', [self.kotlinc, SNAPSHOT / BOOK_COLOR_SOURCE, *self.options, '-d', self.work / 'official-assess-BookColorUtil'])
        self.case('androidbible/book_color_oracle', source_sha256=sha(SNAPSHOT / BOOK_COLOR_SOURCE), scope='Official accepts exact original source; candidate parser boundary assessed separately')
        book_assessment = self.run('Assess original BookColorUtil numeric syntax boundary', [self.compiler, SNAPSHOT / BOOK_COLOR_SOURCE, '-cp', self.stdlib, '-d', self.work / 'assess-BookColorUtil'], allowed=(0, 1))
        self.report['cases'].append(dict(id='androidbible/book_color_assessment', outcome='not_assessed', source=relative(SNAPSHOT / BOOK_COLOR_SOURCE), source_sha256=sha(SNAPSHOT / BOOK_COLOR_SOURCE), reason='Source assessment only; hexadecimal separators are outside this milestone', diagnostics=book_assessment.stderr, exit_code=book_assessment.returncode))
        assessment = self.run('Assess unchanged ReadingPassage', [self.compiler, SNAPSHOT / APP_PROBE, '-cp', self.stdlib, '-d', self.work / 'assess-ReadingPassage'], allowed=(0, 1))
        self.report['cases'].append(dict(id='androidbible/source_assessment', outcome='not_assessed', source=relative(SNAPSHOT / APP_PROBE), source_sha256=sha(SNAPSHOT / APP_PROBE), reason='Dependency closure not built; diagnostics recorded as assessment only', diagnostics=assessment.stderr, exit_code=assessment.returncode))

    def features(self):
        report = {key: self.report[key] for key in ['schema_version', 'created_at', 'compiler', 'reference_compiler', 'inputs']}
        report.update(kind='features', status='assessed', commands=self.report['reproduction_commands'], evidence=[relative(self.report_path)], features=[])
        for item in FEATURES:
            ident = item['id']
            related = [case['id'] for case in self.report['cases'] if case['id'].endswith('/' + ident) or case['id'] in ['kotlin_consumer', 'frozen_inputs', 'official_class_verification'] or case['id'].startswith('boundary/')]
            if ident == 'descending_progressions':
                related += ['source_abi/source_infix_extension', 'source_abi/source_infix_member']
            if ident in ['descending_progressions', 'stepped_progressions']:
                related.append('shadow/' + ('downTo' if ident == 'descending_progressions' else 'step'))
            report['features'].append(dict(name=ident, status=feature_status(self.report['cases'], ident), scope=item['scope'], evidence_cases=related, restrictions=[RESTRICTIONS[ident], 'Evidence is limited to recorded fixtures and dependencies; no full-app, Android-device runtime or compilation-performance claim.']))
        (ROOT / 'reports/features' / (self.run_id + '.json')).write_text(json.dumps(report, indent=2) + '\n')

    def execute(self):
        try:
            self.prepare()
            self.probes()
            self.consumers()
            self.boundaries()
            self.androidbible()
            if not self.args.preflight:
                pin = json.loads((ROOT / 'references/compiler.json').read_text())
                if self.frozen != frozen_files() or self.reference_hashes != file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar')) or sha(self.compiler) != self.compiler_sha:
                    raise AssertionError('Compiler, fixtures, tools or pinned JARs changed during frozen run')
                if any(sha(SNAPSHOT / name) != checksum for name, checksum in self.original_hashes.items()):
                    raise AssertionError('Original cited sources changed')
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
    return Harness(parser.parse_args()).execute()


if __name__ == '__main__':
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)
