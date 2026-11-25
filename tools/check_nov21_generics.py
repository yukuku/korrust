#!/usr/bin/env python3
"""Differential evidence for contextual variant JVM signatures and independent generic source members."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/nov21_generics'
REFERENCES = [ROOT / path for path in ['references/kotlin-spec/docs/src/md/kotlin.core/type-system.md', 'references/kotlin/compiler/backend/src/org/jetbrains/kotlin/codegen/state/typeMappingUtil.kt', 'references/kotlin/compiler/backend.common.jvm/src/org/jetbrains/kotlin/load/kotlin/TypeMappingModeExtensions.kt', 'references/kotlin/core/compiler.common.jvm/src/org/jetbrains/kotlin/load/kotlin/TypeMappingMode.kt', 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/sync/SyncUtils.kt', 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/datatransfer/process/ExportProcess.kt', 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/songs/newdoc/LegacyParcelDecoder.kt']]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


def callable_signatures(output):
    result, current, descriptor = {}, None, None
    for line in output.splitlines():
        if line.startswith('  public ') or line.startswith('  protected '):
            current = line.strip().split('(', 1)[0].rsplit(' ', 1)[-1] if '(' in line else None
            descriptor = None
        elif line.startswith('  private ') or line.startswith('  static '):
            current = None
        elif current and line.strip().startswith('descriptor:'):
            descriptor = line.strip().removeprefix('descriptor:').strip()
            result[current + descriptor] = {'descriptor': descriptor, 'signature': None}
        elif current and descriptor and line.strip().startswith('Signature:'):
            result[current + descriptor]['signature'] = line.split('//', 1)[1].strip()
        elif line == '}':
            current = None
    return result


class Check:
    def __init__(self, args):
        self.args = args
        self.baseline = args.baseline.resolve() if args.baseline else None
        self.candidate = args.candidate.resolve()
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-nov21-generics'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        reference = args.reference_root.resolve() / pin['path']
        self.kotlin = reference / 'bin/kotlinc'
        self.stdlib = reference / 'lib/kotlin-stdlib.jar'
        self.java, self.javac, self.javap = [Path(os.environ['JAVA_HOME']) / 'bin' / n for n in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'nov21_generics']
        paths = [*args.source_root.glob('src/**/*.rs'), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *FIXTURES.glob('*'), ROOT / 'tests/nov21_generics.rs', ROOT / 'tests/next10_generics.rs', *REFERENCES, Path(__file__), ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
        self.frozen = {relative(p): sha(p) for p in sorted(paths) if p.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary':relative(self.candidate), 'binary_sha256':sha(self.candidate), 'source_root':relative(args.source_root)}, baseline_compiler={'binary':relative(self.baseline), 'binary_sha256':sha(self.baseline)} if self.baseline else None, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'options':self.options, 'launcher_sha256':sha(self.kotlin), 'compiler_jar_sha256':sha(reference / 'lib/kotlin-compiler.jar')}, inputs={'files_sha256':self.frozen, 'stdlib_sha256':sha(self.stdlib)}, reproduction_commands=[['python3', 'tools/check_nov21_generics.py', '--candidate', relative(self.candidate), '--source-root', str(args.source_root), '--reference-root', str(args.reference_root)]], commands=[], cases=[], evidence=[], limitations=[
            'Newly supported: independent method type parameters in supported ordinary generic source classes and abstract generic source interfaces; interface generic-call registration, method substitution/erasure and exact callable JVM/Kotlin metadata are checked. Ordinary nongeneric class/object generic methods already existed and are regression evidence, not a new feature count.',
            'Contextual declaration-variance callable signatures cover final/open/type-parameter arguments, nested final variant classifiers, invariant nesting, projections, input/output/function-type modes, arrays, class constructors and interface/class/object members. Private backing-field generic signatures are not parity claims; projected exposed JVM fields and variant classifier bounds/nested supertypes remain explicit boundaries.',
            'Independent single reference upper bounds; method shadowing, recursive/dependent/multiple bounds, generic inline or suspend members, and generic interface method bodies remain explicit boundaries. Existing generic classifier nested/data restrictions remain.',
            'Fixed Java caller compiled against official output is reused with -Xverify:all. All four official/candidate Kotlin provider/consumer pairs execute. AndroidBible files are relevance evidence only; this report claims no new unchanged app-file or full-app coverage.'
        ])

        self.report['compiler']['source_revision'] = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=args.source_root, capture_output=True, text=True, check=True).stdout.strip()
        if self.baseline:
            self.report['reproduction_commands'][0].extend(['--baseline', str(self.baseline)])

    def run(self, label, argv, accepted=(0,)):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=600)
        entry = dict(label=label, argv=list(map(str, argv)), cwd='.', exit_code=result.returncode)
        for stream in ['stdout', 'stderr']:
            path = self.logs / f'{len(self.report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(self, ident, passed, **fields):
        self.report['cases'].append(dict(id=ident, outcome='pass' if passed else 'correctness_failure', **fields))

    def compile(self, provider, source, output, dependencies=()):
        command = [self.kotlin, source, *self.options] if provider == 'official' else [self.candidate, source, '--module-name', 'nov21_generics']
        return self.run('Compile ' + provider + ' ' + source.name, [*command, '-cp', cp(*dependencies, self.stdlib), '-d', output])

    def execute(self):
        outputs = {p:self.work / p for p in ['official', 'candidate']}
        if self.baseline:
            for name, code in [('BaselineMembers.kt', 'GENERIC_CLASS_MEMBER_TYPE_PARAMETERS_UNSUPPORTED'), ('BaselineWild.kt', 'VARIANT_JVM_SIGNATURE_UNSUPPORTED'), ('BaselineInterface.kt', 'GENERIC_INTERFACE_BODY_UNSUPPORTED')]:
                result = self.run('Baseline rejects new scope ' + name, [self.baseline, FIXTURES / name, '-cp', self.stdlib, '--diagnostic-format', 'json', '-d', self.work / ('baseline-' + name)], (1,))
                diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
                self.case('baseline/' + name, any(d.get('code') == code for d in diagnostics), diagnostics=diagnostics)
        for provider, output in outputs.items():
            self.compile(provider, FIXTURES / 'Use.kt', output)
            for name in ['BaselineMembers.kt', 'BaselineWild.kt', 'BaselineInterface.kt']:
                self.compile(provider, FIXTURES / name, self.work / (provider + '-' + name))
                self.case('new-scope/' + provider + '/' + name, True)
        callers = self.work / 'callers'
        self.run('Compile fixed Java caller against official output', [self.javac, '-cp', cp(outputs['official'], self.stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        signatures = {}
        for provider, output in outputs.items():
            self.run('Java signature source compile ' + provider, [self.javac, '-cp', cp(output, self.stdlib), '-d', self.work / ('java-' + provider), FIXTURES / 'Calls.java'])
            result = self.run('Verified fixed Java caller ' + provider, [self.java, '-Xverify:all', '-cp', cp(callers, output, self.stdlib), 'Calls'])
            self.case('runtime/' + provider, result.stdout == 'OK\n')
            for consumer_compiler in ['official', 'candidate']:
                consumer = self.work / (provider + '-' + consumer_compiler + '-consumer')
                self.compile(consumer_compiler, FIXTURES / 'Consumer.kt', consumer, [output])
                invalid = FIXTURES / 'InvalidConsumer.kt'
                command = [self.kotlin, invalid, *self.options] if consumer_compiler == 'official' else [self.candidate, invalid, '--module-name', 'nov21_generics', '--diagnostic-format', 'json']
                rejected = self.run('Reject imported method bound ' + provider + '/' + consumer_compiler, [*command, '-cp', cp(output, self.stdlib), '-d', self.work / (provider + '-' + consumer_compiler + '-invalid')], (1,))
                self.case('imported-bound/' + provider + '/' + consumer_compiler, 'Label' in rejected.stderr or 'bound' in rejected.stderr.lower())

                launcher = self.work / (provider + '-' + consumer_compiler + '-launcher')
                self.run('Compile consumer launcher', [self.javac, '-cp', cp(consumer, output, self.stdlib), '-d', launcher, FIXTURES / 'ConsumerCalls.java'])
                result = self.run('Verified metadata consumer ' + provider + '/' + consumer_compiler, [self.java, '-Xverify:all', '-cp', cp(launcher, consumer, output, self.stdlib), 'ConsumerCalls'])
                self.case('consumer/' + provider + '/' + consumer_compiler, result.stdout == 'OK\n')
            signatures[provider] = {}
            for name in ['Generic', 'GenericApi', 'Helpers', 'Cov', 'Con', 'Inv', 'UseKt', 'WildMethods', 'WildApi', 'WildDefaults', 'WildDefaults$Companion', 'WildDefaultApi', 'WildDefaultApi$DefaultImpls', 'WildDefaultImpl']:
                result = self.run('javap callable signatures ' + provider + '/' + name, [self.javap, '-p', '-s', '-v', '-classpath', output, 'methodgenerics.' + name])
                signatures[provider][name] = callable_signatures(result.stdout)
        self.report['callable_signatures'] = signatures
        for name, expected in signatures['official'].items():
            self.case('signature/' + name, expected == signatures['candidate'][name], expected=expected, actual=signatures['candidate'][name])
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = self.work / 'boundaries' / item['id'] / 'Boundary.kt'
            source.parent.mkdir(parents=True)
            source.write_text(item['source'])
            oracle = self.run('Official boundary ' + item['id'], [self.kotlin, source, *self.options, '-d', source.parent / 'official'], (0, 1))
            actual = self.run('Candidate boundary ' + item['id'], [self.candidate, source, '--diagnostic-format', 'json', '-cp', self.stdlib, '-d', source.parent / 'candidate'], (0, 1))
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            accepted = item['official'] == 'accept'
            valid = (oracle.returncode == 0) == accepted and actual.returncode != 0
            if item.get('code'):
                valid = valid and any(d.get('code') == item['code'] for d in diagnostics)
            if accepted:
                valid = valid and any(d.get('category') == 'unsupported_language' for d in diagnostics)
            self.report['cases'].append(dict(id='boundary/' + item['id'], outcome=('unsupported_language' if accepted else 'invalid_rejected') if valid else 'correctness_failure', source_sha256=sha(source), diagnostics=diagnostics, official_accepted=oracle.returncode == 0))
        self.report['emitted_classes_sha256'] = {provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider, output in outputs.items()}

    def finish(self):
        changed = [p for p, checksum in self.frozen.items() if sha(ROOT / p) != checksum]
        self.case('frozen_inputs', not changed and sha(self.candidate) == self.report['compiler']['binary_sha256'] and (not self.baseline or sha(self.baseline) == self.report['baseline_compiler']['binary_sha256']), changed=changed)
        counts = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['counts'] = counts
        failed = counts.get('correctness_failure', 0)
        self.report['status'] = 'failed' if failed else 'completed'
        self.report['feature_acceptance'] = {'family':'contextual_variant_signatures_and_generic_members', 'status':'failed' if failed else 'passed', 'derived_counts':counts}
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        if not self.args.skip_html:
            subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True)
        print(relative(path))
        print(json.dumps(counts, sort_keys=True))
        return int(bool(failed))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--skip-html', action='store_true')
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    args = parser.parse_args()
    args.source_root = args.source_root.resolve()
    check = Check(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append(dict(id='harness', outcome='correctness_failure', error=str(error)))
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
