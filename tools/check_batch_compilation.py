#!/usr/bin/env python3
"""Compare bounded module compilation, metadata consumers and rebuild cleanup."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import sys

from check_androidbible_module import ROOT, classpath, relative, sha
from check_androidbible_simple import Harness as LoggedHarness
from check_library_interop import frozen_files
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / 'tests/fixtures/batch_compilation'
MODULE = 'batch_fixture'


def invalid_rejection(returncode, stderr, outputs, pattern):
    diagnostics = '\n'.join(line.split('error:', 1)[1] for line in stderr.splitlines() if 'error:' in line)
    return (returncode != 0 and not outputs and bool(re.search(pattern, diagnostics, re.I))
            and not re.search(r'\bunsupported\b|\bnot (?:yet )?supported\b', diagnostics, re.I))


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-batch-compilation'
        self.work = ROOT / 'target/checks' / self.run_id
        self.evidence = self.work / 'evidence' if args.preflight else ROOT / 'reports/correctness' / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix('.json')
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
                           compiler={}, reference_compiler={}, inputs={}, commands=[], evidence=[], cases=[],
                           selection='Independent bounded multi-source module fixtures, cross-file inference and explicit recursion, overloaded/default/generic calls, companion construction, package scopes, file-private/module-internal visibility, fixed official-compiled Java callers with independent assertions, official Kotlin consumers, public ABI, input permutations and complete-module deletion rebuilds. JVM/source evidence only; no Android, full-app, arbitrary Kotlin-language or performance claim.',
                           reproduction_commands=[['python3', 'tools/check_batch_compilation.py', '--compiler', str(args.compiler)] + (['--preflight', 'official'] if args.preflight else [])])
        self.compiler = args.compiler.resolve()
        self.java, self.javac, self.javap = [str(Path(os.environ['JAVA_HOME']) / 'bin' / name) if os.environ.get('JAVA_HOME') else shutil.which(name) for name in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', MODULE]

    def outcome(self, ident, passed, **details):
        self.report['cases'].append(dict(id=ident, outcome='pass' if passed else 'fail', **details))

    def prepare(self):
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        self.frozen = file_hashes([*FIXTURES.rglob('*.*'), Path(__file__)]) if self.args.preflight else frozen_files()
        self.reference_hashes = file_hashes((ROOT / pin['path'] / 'lib').glob('*.jar'))
        version = self.run('Pinned official compiler identity', [self.kotlinc, '-version'])
        if 'kotlinc-jvm ' + pin['version'] + ' ' not in version.stdout + version.stderr:
            raise AssertionError('Pinned compiler version mismatch')
        self.report['reference_compiler'] = dict(version=pin['version'], options=self.options, distribution_libs_sha256=self.reference_hashes)
        self.report['inputs'] = dict(fixtures_sha256=file_hashes(FIXTURES.rglob('*.*')), frozen_files_sha256=self.frozen,
                                     reference_manifest_sha256=sha(ROOT / 'references/compiler.json'),
                                     repository_revision=self.run('Repository revision', ['git', 'rev-parse', 'HEAD']).stdout.strip())
        self.report['environment'] = dict(java=self.run('JDK identity', [self.java, '-version']).stderr.strip())
        if not self.args.preflight:
            self.compiler_sha = sha(self.compiler)
            self.report['compiler'] = dict(binary_sha256=self.compiler_sha, version=self.run('Candidate identity', [self.compiler, '--version']).stdout.strip(), source_files_sha256=self.frozen,
                                           jvm_target_scope='Official sources/consumers target JVM 1.8. Candidate has no target option and may emit classfile49/52; behavior and ABI are compared without claiming bytecode-target equivalence.')

    def compile(self, label, sources, destination, *, candidate=False, dependencies=(), reject=False, module=MODULE):
        dependencies = [*dependencies, self.stdlib]
        if candidate:
            argv = [self.compiler, *sources, '-d', destination, '-cp', classpath(*dependencies), '--module-name', module]
        else:
            options = self.options[:-1] + [module]
            argv = [self.kotlinc, *sources, *options, '-cp', classpath(*dependencies), '-d', destination]
        return self.run(label, argv, allowed=(0, 1) if reject else (0,))

    def runtime(self, label, output, drivers, main, expected):
        result = self.run(label, [self.java, '-Xverify:all', '-cp', classpath(drivers, output, self.stdlib), main])
        self.outcome(label, result.stdout.strip() == expected, expected_stdout=expected, actual_stdout=result.stdout.strip())

    def verify(self, label, output, drivers):
        names = [name[:-6].replace('/', '.') for name in tree_hashes(output, '.class')]
        result = self.run(label, [self.java, '-Xverify:all', '-cp', classpath(drivers, output, self.stdlib), 'ModuleClassVerification', *names])
        self.outcome(label, bool(names) and result.stdout.strip() == f'Verified {len(names)} module classes', classes=len(names))

    def abi(self, label, expected, actual):
        compared = []
        for name in tree_hashes(expected, '.class'):
            if re.search(r'\$[0-9]|\$DefaultImpls\.class$|\$WhenMappings\.class$', name):
                continue
            name = name[:-6].replace('/', '.')
            left = self.run('Inspect expected ' + name, [self.javap, '-protected', '-s', '-classpath', expected, name])
            right = self.run('Inspect actual ' + name, [self.javap, '-protected', '-s', '-classpath', actual, name], allowed=(0, 1))
            compared.append(dict(class_name=name, matched=right.returncode == 0 and source_abi(left.stdout) == source_abi(right.stdout)))
        self.outcome(label, bool(compared) and all(item['matched'] for item in compared), classes=compared)

    def consumers(self, label, output):
        destination = self.work / (label + '-consumer')
        self.compile('Compile official Kotlin consumer against ' + label, [FIXTURES / 'Consumer.kt'], destination, dependencies=[output], module='external_consumer')
        self.runtime('kotlin_consumer/' + label, output, destination, 'batch.consumer.ConsumerKt', 'consumer-ok:30')

    def boundaries(self, official, candidate=None):
        for ident, pattern in [('private', r'private|access|visible|unresolved|applicable'),
                               ('duplicate', r'duplicate|conflict|redeclar|overload'),
                               ('cycle', r'cycl|recurs|infer')]:
            sources = sorted((FIXTURES / 'negative' / ident).glob('*.kt'))
            for reverse in [False, True]:
                ordered = list(reversed(sources)) if reverse else sources
                suffix = '/reversed' if reverse else '/forward'
                for kind in ['official'] + (['candidate'] if candidate else []):
                    output = self.work / (ident + suffix.replace('/', '-') + '-' + kind)
                    result = self.compile('Reject ' + ident + suffix + ' ' + kind, ordered, output, candidate=kind == 'candidate', reject=True)
                    diagnostics = result.stderr
                    self.outcome('boundary/' + ident + suffix + '/' + kind,
                                 invalid_rejection(result.returncode, diagnostics, tree_hashes(output), pattern),
                                 diagnostic=diagnostics.strip(), expected_classification='invalid_kotlin')
        source = FIXTURES / 'negative/internal/External.kt'
        for kind, output in [('official', official)] + ([('candidate', candidate)] if candidate else []):
            for consumer in ['official'] + (['candidate'] if candidate else []):
                destination = self.work / ('external-' + kind + '-' + consumer)
                result = self.compile('Reject external internal call ' + kind + '/' + consumer, [source], destination,
                                      candidate=consumer == 'candidate', dependencies=[output], reject=True, module='external_consumer')
                self.outcome('boundary/internal/' + kind + '/' + consumer,
                             invalid_rejection(result.returncode, result.stderr, tree_hashes(destination), r'internal|access|visible|unresolved|applicable'),
                             diagnostic=result.stderr.strip(), expected_classification='invalid_kotlin')

    def rebuild(self, *, candidate):
        inputs = self.work / 'rebuild-inputs'
        shutil.copytree(FIXTURES / 'rebuild/v1', inputs)
        sources = sorted(inputs.glob('*.kt'))
        official_initial = self.work / 'rebuild-official-initial'
        self.compile('Compile official initial rebuild module', sources, official_initial)
        incremental = self.work / 'rebuild-incremental'
        if candidate:
            self.compile('Compile candidate initial rebuild module', sources, incremental, candidate=True)
        (inputs / 'Removed.kt').unlink()
        shutil.copy2(FIXTURES / 'rebuild/v2/Primary.kt', inputs / 'Primary.kt')
        sources = sorted(inputs.glob('*.kt'))
        official = self.work / 'rebuild-official-clean'
        self.compile('Compile official changed rebuild module clean', sources, official)
        drivers = self.work / 'rebuild-java-callers'
        self.run('Compile fixed rebuild Java caller against official changed module', [self.javac, '-cp', classpath(official, self.stdlib), '-d', drivers, FIXTURES / 'RebuildCheck.java'])
        self.runtime('rebuild/official_java', official, drivers, 'RebuildCheck', 'rebuild-ok:11')
        self.report['inputs']['rebuild_fixed_java_callers_sha256'] = tree_hashes(drivers)
        if candidate:
            self.compile('Recompile changed module with removed source into existing output', sources, incremental, candidate=True)
            clean = self.work / 'rebuild-candidate-clean'
            self.compile('Compile changed module into independent clean output', sources, clean, candidate=True)
            expected = tree_hashes(clean)
            self.outcome('rebuild/artifacts_match_clean', tree_hashes(incremental) == expected, incremental=tree_hashes(incremental), independent_clean=expected)
            self.abi('rebuild/abi', official, incremental)
            for label, output in [('incremental', incremental), ('clean', clean)]:
                self.runtime('rebuild/' + label + '_java', output, drivers, 'RebuildCheck', 'rebuild-ok:11')
            self.compile('Publish empty complete module', [], incremental, candidate=True)
            self.outcome('rebuild/last_source_removed', tree_hashes(incremental) == {}, remaining_artifacts=tree_hashes(incremental))

    def execute(self):
        self.prepare()
        sources = sorted((FIXTURES / 'positive').rglob('*.kt'))
        official = self.work / 'official'
        reversed_official = self.work / 'official-reversed'
        self.compile('Compile official independent batch', sources, official)
        self.compile('Compile official reversed batch', list(reversed(sources)), reversed_official)
        self.abi('order/official_abi', official, reversed_official)
        drivers = self.work / 'java-callers'
        self.run('Compile fixed Java caller only against official output', [self.javac, '-cp', classpath(official, self.stdlib), '-d', drivers, FIXTURES / 'BatchCheck.java', ROOT / 'tests/fixtures/ModuleClassVerification.java'])
        self.report['inputs']['fixed_java_callers_sha256'] = tree_hashes(drivers)
        self.report['inputs']['official_outputs_sha256'] = tree_hashes(official)
        for label, output in [('official', official), ('official_reversed', reversed_official)]:
            self.runtime('java/' + label, output, drivers, 'BatchCheck', 'batch-ok:30')
            self.verify('verification/' + label, output, drivers)
            self.consumers(label, output)
        candidate = None
        if not self.args.preflight:
            candidate = self.work / 'candidate'
            reversed_candidate = self.work / 'candidate-reversed'
            self.compile('Compile candidate independent batch', sources, candidate, candidate=True)
            self.compile('Compile candidate reversed batch', list(reversed(sources)), reversed_candidate, candidate=True)
            self.outcome('order/candidate_artifacts', tree_hashes(candidate) == tree_hashes(reversed_candidate), forward=tree_hashes(candidate), reversed=tree_hashes(reversed_candidate))
            self.abi('candidate/abi', official, candidate)
            for label, output in [('candidate', candidate), ('candidate_reversed', reversed_candidate)]:
                self.runtime('java/' + label, output, drivers, 'BatchCheck', 'batch-ok:30')
                self.verify('verification/' + label, output, drivers)
                self.consumers(label, output)
        self.boundaries(official, candidate)
        self.rebuild(candidate=not self.args.preflight)
        frozen = all(sha(ROOT / name) == digest for name, digest in self.frozen.items())
        frozen = frozen and all(sha(ROOT / name) == digest for name, digest in self.reference_hashes.items())
        if not self.args.preflight:
            frozen = frozen and sha(self.compiler) == self.compiler_sha
        self.outcome('frozen_inputs', frozen)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--preflight', choices=['official'])
    args = parser.parse_args()
    harness = Harness(args)
    try:
        harness.execute()
    except Exception as error:
        harness.outcome('harness_failure', False, message=str(error))
    harness.report['counts'] = dict(Counter(case['outcome'] for case in harness.report['cases']))
    harness.report['status'] = 'completed' if all(case['outcome'] == 'pass' for case in harness.report['cases']) else 'failed'
    harness.report['finished_at'] = datetime.now(timezone.utc).isoformat()
    harness.report_path.write_text(json.dumps(harness.report, indent=2) + '\n')
    print('Report: ' + relative(harness.report_path), flush=True)
    if not args.preflight:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)
    return 0 if harness.report['status'] == 'completed' else 1


if __name__ == '__main__':
    raise SystemExit(main())
