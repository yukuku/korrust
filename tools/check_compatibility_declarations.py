#!/usr/bin/env python3
"""Verify checkpoint D declaration semantics and unchanged LogInterface."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import sys

ROOT_HINT = Path(os.environ.get('KORRUST_ROOT', Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(ROOT_HINT / 'tools'))
from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_androidbible_simple import Harness as LoggedHarness
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / 'tests/fixtures/compatibility_declarations'
APP_SOURCE = 'Alkitab/src/main/java/yuku/alkitab/datatransfer/process/LogInterface.kt'
FEATURES = [
    {'id': 'backticks', 'tasks': [16], 'sources': ['Backticks.kt'], 'owners': ['DescriptiveTest', 'BackticksKt']},
    {'id': 'discards', 'tasks': [17], 'sources': ['Discards.kt'], 'owners': ['Components', 'DiscardsKt']},
    {'id': 'functional_interfaces', 'tasks': [18, 19], 'sources': ['FunctionalInterfaces.kt'], 'owners': ['MapperParent', 'Mapper', 'Mapper$DefaultImpls', 'Sink', 'FunctionalInterfacesKt']},
    {'id': 'log_interface', 'tasks': [19], 'sources': ['LogCalls.kt'], 'owners': ['LogCallsKt'], 'app_owners': ['LogInterface', 'LogInterface$DefaultImpls']},
    {'id': 'type_aliases', 'tasks': [20], 'sources': ['Aliases.kt', 'AliasUses.kt'], 'owners': ['GotoTab', 'AliasesKt', 'AliasUsesKt']},
]


def java_driver_source(fixtures, ident):
    source = (fixtures / 'DeclarationsCheck.java').read_text()
    cases = list(re.finditer(r'^            case "([^\"]+)":', source, re.MULTILINE))
    selected = next(index for index, match in enumerate(cases) if match[1] == ident)
    start = cases[selected].start()
    end = cases[selected + 1].start() if selected + 1 < len(cases) else source.index('            default:')
    source = source[:cases[0].start()] + source[start:end] + source[source.index('            default:'):]
    return source.replace('public final class DeclarationsCheck {', 'public final class DeclarationsCheck_' + ident + ' {')


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        self.fixtures = args.fixtures.resolve()
        self.now = datetime.now(timezone.utc)
        self.run_id = self.now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-compatibility-declarations'
        self.work = ROOT / 'target/checks' / self.run_id
        self.evidence = self.work / 'evidence' if args.preflight else ROOT / 'reports/correctness' / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix('.json')
        self.report = {'schema_version': 1, 'kind': 'correctness', 'created_at': self.now.isoformat(), 'status': 'running', 'compiler': {}, 'reference_compiler': {}, 'inputs': {}, 'commands': [], 'evidence': [], 'cases': [], 'selection': 'Checkpoint D tasks16–20: escaped JVM identifiers and actual JUnit test annotation; skipped underscore component effects; Kotlin functional-interface identity/inherited SAM/default helpers/captures; unchanged full LogInterface runtime; concrete/function alias chains, cross-file and compiled-metadata consumers. No complete app, Android runtime or performance claim.'}
        self.java, self.javac, self.javap = [str(Path(os.environ['JAVA_HOME']) / 'bin' / name) if os.environ.get('JAVA_HOME') else shutil.which(name) for name in ['java', 'javac', 'javap']]
        self.outputs = {}

    def outcome(self, ident, passed, **details):
        self.report['cases'].append({'id': ident, 'outcome': 'pass' if passed else 'fail', **details})

    def sources(self, item):
        sources = [self.fixtures / name for name in item['sources']]
        if item['id'] == 'log_interface':
            sources.insert(0, SNAPSHOT / APP_SOURCE)
        return sources

    def prepare(self):
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        self.compiler = self.args.compiler.resolve()
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-jvm-default=enable']
        self.reference_hashes = file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar'))
        frozen_paths = [Path(__file__), self.args.junit_jar, SNAPSHOT / APP_SOURCE, *[path for path in self.fixtures.rglob('*') if path.is_file()]]
        self.frozen = {str(path): sha(path) for path in frozen_paths}
        identity = self.run('Pinned official identity', [self.kotlinc, '-version'])
        if 'kotlinc-jvm ' + pin['version'] + ' ' not in identity.stdout + identity.stderr:
            raise AssertionError('Pinned official compiler identity mismatch')
        self.report['reference_compiler'] = {'version': pin['version'], 'options': self.options, 'distribution_libs_sha256': self.reference_hashes, 'jvm_default_scope': 'enable: interface default bodies plus DefaultImpls compatibility helpers, matching the established candidate interface ABI. Abstract versus concrete default methods remain part of exact public ABI and independent reflection assertions.'}
        if self.args.preflight != 'official':
            self.compiler_sha = sha(self.compiler)
            source_fingerprints = file_hashes([ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', *sorted((ROOT / 'src').rglob('*.rs'))])
            self.report['compiler'] = {'source_files_sha256': source_fingerprints, 'version': self.run('Candidate identity', [self.compiler, '--version']).stdout.strip(), 'binary_sha256': self.compiler_sha, 'jvm_target_scope': 'Official JVM1.8; candidate output target is not configurable; runtime/ABI/metadata evidence only.'}
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        if sha(SNAPSHOT / APP_SOURCE) != inventory[APP_SOURCE]:
            raise AssertionError('Original LogInterface source changed')
        self.report['reproduction_commands'] = [[sys.executable, *sys.argv]]
        self.report['inputs'] = {'fixtures_sha256': {relative(path): sha(path) for path in self.fixtures.rglob('*') if path.is_file()}, 'unchanged_app_sources_sha256': {APP_SOURCE: inventory[APP_SOURCE]}, 'junit_jar': str(self.args.junit_jar), 'junit_jar_sha256': sha(self.args.junit_jar), 'scope_boundary': 'Alias OnGotoFinished preserves the original four-argument function type with a concrete fixture GotoTab class; enum integration is task23, not claimed here.', 'reference_policy': ['references/kotlin-spec/docs/src/md/kotlin.core/syntax.md#escaped-identifiers', 'references/kotlin-spec/docs/src/md/kotlin.core/operators.md#destructuring-declarations', 'references/kotlin-spec/docs/src/md/kotlin.core/declarations.md#functional-interface-declaration', 'references/kotlin-spec/docs/src/md/kotlin.core/declarations.md#type-alias'], 'candidate_owns': 'Every positive Kotlin fixture including original LogInterface; only Java Effects/JUnit and pinned stdlib are external.'}
        self.dependencies = self.work / 'dependencies'
        self.run('Compile independent Java effect recorder', [self.javac, '-d', self.dependencies, self.fixtures / 'Effects.java'])
        self.cp = classpath(self.dependencies, self.args.junit_jar, self.stdlib)
        self.boundaries = json.loads((self.fixtures / 'rejections.json').read_text())
        for item in FEATURES:
            ident = item['id']
            module = 'compatibility_declarations_' + ident
            official = self.work / 'official' / ident
            self.run('Compile official ' + ident, [self.kotlinc, *self.sources(item), *self.options, '-module-name', module, '-cp', self.cp, '-d', official])
            self.outputs[ident] = {'official': official}
            driver = self.work / ('DeclarationsCheck_' + ident + '.java')
            driver.write_text(java_driver_source(self.fixtures, ident))
            self.archive(driver, driver.name)
            callers = self.work / 'java-callers' / ident
            self.run('Compile fixed Java caller against official ' + ident, [self.javac, '-cp', classpath(official, self.cp), '-d', callers, driver, ROOT / 'tests/fixtures/ModuleClassVerification.java'])
            result = self.run('Independent official assertions ' + ident, [self.java, '-Xverify:all', '-cp', classpath(callers, official, self.cp), 'DeclarationsCheck_' + ident, ident])
            self.case('oracle/' + ident, runtime_checks=json.loads(result.stdout)['checks'], result=result.stdout)
            self.outputs[ident]['java_callers'] = callers
            self.outputs[ident]['expected'] = result.stdout
            self.consumer(ident, 'official', official)

    def consumer(self, ident, provider, output):
        destination = self.work / (provider + '-consumer') / ident
        compile_result = self.run('Official Kotlin metadata consumer ' + provider + '/' + ident, [self.kotlinc, self.fixtures / ('Consumer_' + ident + '.kt'), *self.options, '-module-name', 'external_' + ident, '-cp', classpath(output, self.cp), '-d', destination], allowed=(0, 1))
        if compile_result.returncode:
            self.outcome('kotlin_consumer/' + provider + '/' + ident, False, diagnostics=compile_result.stderr)
            return
        observed = self.run('Run official Kotlin consumer ' + provider + '/' + ident, [self.java, '-Xverify:all', '-cp', classpath(destination, output, self.cp), 'compatibility.declarations.consumer.Consumer_' + ident + 'Kt'], allowed=(0, 1))
        self.outcome('kotlin_consumer/' + provider + '/' + ident, observed.returncode == 0 and observed.stdout == 'OK\n')

    def inspect(self, item, candidate):
        ident = item['id']
        official = self.outputs[ident]['official']
        owners = ['compatibility.declarations.' + name for name in item['owners']]
        owners += ['yuku.alkitab.datatransfer.process.' + name for name in item.get('app_owners', [])]
        passed = True
        for owner in owners:
            expected = self.run('Official public ABI ' + owner, [self.javap, '-protected', '-s', '-classpath', official, owner])
            actual = self.run('Candidate public ABI ' + owner, [self.javap, '-protected', '-s', '-classpath', candidate, owner], allowed=(0, 1))
            passed = passed and actual.returncode == 0 and source_abi(expected.stdout) == source_abi(actual.stdout)
        self.outcome('source_abi/' + ident, passed, owners=owners, scope='Explicit source/default-helper public and protected descriptors and modifiers, including abstract versus concrete Java default methods; generated lambda class names are not ABI claims.')
        names = [name[:-6].replace('/', '.') for name in tree_hashes(candidate, '.class')]
        verified = self.run('Verify all candidate class bodies ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.outputs[ident]['java_callers'], candidate, self.cp), 'ModuleClassVerification', *names], allowed=(0, 1))
        self.outcome('class_verification/' + ident, verified.returncode == 0, classes=len(names))

    def compile_candidate(self):
        for item in FEATURES:
            ident = item['id']
            output = self.work / 'candidate' / ident
            result = self.run('Compile candidate ' + ident, [self.compiler, *self.sources(item), '--module-name', 'compatibility_declarations_' + ident, '-cp', self.cp, '-d', output], allowed=(0, 1))
            emitted = tree_hashes(output, '.class')
            self.outcome('compile/' + ident, result.returncode == 0 and bool(emitted), emitted_classes_sha256=emitted, diagnostics=result.stderr)
            if result.returncode or not emitted:
                continue
            self.outputs[ident]['candidate'] = output
            actual = self.run('Fixed official Java bytecodes against candidate ' + ident, [self.java, '-Xverify:all', '-cp', classpath(self.outputs[ident]['java_callers'], output, self.cp), 'DeclarationsCheck_' + ident, ident], allowed=(0, 1))
            self.outcome('java/' + ident, actual.returncode == 0 and actual.stdout == self.outputs[ident]['expected'], scope='Same Java caller bytecodes compiled once against official output; independent expected values and effect traces.')
            self.inspect(item, output)
            self.consumer(ident, 'candidate', output)
            if ident == 'type_aliases':
                reversed_output = self.work / 'candidate-reversed-aliases'
                self.run('Compile alias batch in reversed order', [self.compiler, *reversed(self.sources(item)), '--module-name', 'compatibility_declarations_' + ident, '-cp', self.cp, '-d', reversed_output])
                self.consumer(ident, 'candidate_reversed', reversed_output)

    def negatives(self):
        for boundary in self.boundaries:
            ident = boundary['id']
            if boundary.get('consumer'):
                for provider in ['official', 'candidate']:
                    if provider == 'candidate' and self.args.preflight == 'official':
                        continue
                    output = self.outputs['type_aliases'].get(provider)
                    if output is None:
                        self.report['cases'].append({'id': 'boundary/' + provider + '/' + ident, 'outcome': 'not_run', 'reason': 'Alias provider did not compile.'})
                        continue
                    source = self.work / ('ConsumerBoundary_' + ident + '.kt')
                    source.write_text(boundary['consumer'] + '\n')
                    result = self.run('Reject official consumer ' + provider + '/' + ident, [self.kotlinc, source, *self.options, '-cp', classpath(output, self.cp), '-d', self.work / ('rejected-' + provider + '-' + ident)], allowed=(0, 1))
                    self.outcome('boundary/' + provider + '/' + ident, result.returncode == 1 and bool(re.search(boundary['candidate_diagnostic'], result.stderr, re.I)), diagnostics=result.stderr)
                continue
            sources = []
            for index, text in enumerate(boundary['sources']):
                source = self.work / ('Boundary_' + ident + '_' + str(index) + '.kt')
                source.write_text(text + '\n')
                sources.append(source)
            expected = self.run('Official boundary ' + ident, [self.kotlinc, *sources, *self.options, '-cp', self.cp, '-d', self.work / ('official-boundary-' + ident)], allowed=(0, 1))
            oracle_valid = expected.returncode == (1 if boundary['invalid'] else 0)
            self.outcome('oracle_boundary/' + ident, oracle_valid, diagnostics=expected.stderr)
            if self.args.preflight == 'official':
                continue
            output = self.work / ('candidate-boundary-' + ident)
            actual = self.run('Candidate boundary ' + ident, [self.compiler, *sources, '-cp', self.cp, '-d', output], allowed=(0, 1))
            rejected = actual.returncode == 1 and not tree_hashes(output, '.class') and bool(re.search(boundary['candidate_diagnostic'], actual.stderr, re.I))
            if not boundary['invalid'] and oracle_valid and rejected:
                self.report['cases'].append({'id': 'boundary/' + ident, 'outcome': 'unsupported_language', 'rejection_verified': True, 'diagnostics': actual.stderr})
            else:
                self.outcome('boundary/' + ident, oracle_valid and rejected, diagnostics=actual.stderr)

    def execute(self):
        try:
            self.prepare()
            if self.args.preflight != 'official':
                self.compile_candidate()
            self.negatives()
            if any(sha(Path(path)) != checksum for path, checksum in self.frozen.items()):
                raise AssertionError('Frozen harness/fixture/dependency/app source changed')
            if any(sha(ROOT / path) != checksum for path, checksum in self.reference_hashes.items()):
                raise AssertionError('Pinned official compiler distribution changed')
            if self.args.preflight != 'official' and sha(self.compiler) != self.compiler_sha:
                raise AssertionError('Frozen candidate binary changed')
            self.case('frozen_inputs', files=len(self.frozen))
            self.report['status'] = 'completed' if not any(case['outcome'] in ['fail', 'timeout'] for case in self.report['cases']) else 'failed'
        except Exception as error:
            self.report['status'] = 'failed'
            self.report['cases'].append({'id': 'harness_failure', 'outcome': 'timeout' if isinstance(error, TimeoutError) else 'fail', 'message': str(error)})
            print(str(error), file=sys.stderr)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['finished_at'] = datetime.now(timezone.utc).isoformat()
        self.report_path.write_text(json.dumps(self.report, indent=2) + '\n')
        print('Report: ' + relative(self.report_path), flush=True)
        return 0 if self.report['status'] == 'completed' else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--fixtures', type=Path, default=FIXTURES)
    parser.add_argument('--junit-jar', type=Path, required=True)
    parser.add_argument('--preflight', choices=['official', 'candidate'])
    return Harness(parser.parse_args()).execute()


if __name__ == '__main__':
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)
