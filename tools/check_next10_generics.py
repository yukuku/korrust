#!/usr/bin/env python3
"""Differential evidence for bounded source classifier variance and interface bounds."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/next10_generics'
REFERENCES = [ROOT / 'references/kotlin-spec/docs/src/md/kotlin.core/type-system.md', *[ROOT / ('references/kotlin/compiler/testData/diagnostics/tests/variance/' + name) for name in ['PrimaryConstructor.kt', 'FunctionTypes.kt', 'VarProperty.kt']]]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


class Check:
    def __init__(self, args):
        self.args = args
        self.candidate = args.candidate.resolve()
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-next10-generics'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        reference = args.reference_root.resolve() / pin['path']
        self.kotlin = reference / 'bin/kotlinc'
        self.stdlib = reference / 'lib/kotlin-stdlib.jar'
        self.java, self.javac, self.javap = [Path(os.environ['JAVA_HOME']) / 'bin' / n for n in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'next10_generics']
        paths = [*args.source_root.glob('src/**/*.rs'), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *FIXTURES.glob('*'), *REFERENCES, Path(__file__), ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
        self.frozen = {relative(p): sha(p) for p in sorted(paths) if p.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary':relative(self.candidate), 'binary_sha256':sha(self.candidate), 'source_root':relative(args.source_root)}, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'options':self.options, 'launcher_sha256':sha(self.kotlin), 'compiler_jar_sha256':sha(reference / 'lib/kotlin-compiler.jar')}, inputs={'files_sha256':self.frozen, 'stdlib_sha256':sha(self.stdlib)}, reproduction_commands=[['python3', 'tools/check_next10_generics.py', '--candidate', relative(self.candidate), '--source-root', str(args.source_root), '--reference-root', str(args.reference_root)]], commands=[], cases=[], evidence=[], limitations=['Declaration-site in/out on ordinary supported source generic classes/interfaces; nested generic and function-type polarity, constructor properties, supertypes, source and compiled substitution. Private-to-this variance exceptions remain explicitly unsupported.', 'One independent non-null class/interface parameter upper bound, including interface method/property resolution and parameterized interface bounds. Nullable interface bounds, dependent/recursive/multiple bounds, generic data classes, generic member functions, generic interface bodies, function-type bounds and parameter shadowing remain unsupported.', 'Official Java callers are compiled once and reused against both outputs with -Xverify:all; official Kotlin consumers check metadata. Exposed inputs requiring contextual declaration-variance JVM wildcards are explicitly unsupported; exact final out arguments, Any contravariant inputs, local subtyping and ordinary returned variant types are covered. Projected exposed variant signatures are unsupported. No whole-app or general-generics claim.'])

    def run(self, label, argv, accepted=(0,)):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=240)
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
        command = [self.kotlin, source, *self.options] if provider == 'official' else [self.candidate, source, '--module-name', 'next10_generics']
        return self.run('Compile ' + provider + ' ' + source.name, [*command, '-cp', cp(*dependencies, self.stdlib), '-d', output])

    def execute(self):
        outputs = {p:self.work / p for p in ['official', 'candidate']}
        for provider, output in outputs.items():
            self.compile(provider, FIXTURES / 'Use.kt', output)
        callers = self.work / 'callers'
        self.run('Compile Java caller against official output', [self.javac, '-cp', cp(outputs['official'], self.stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        for provider, output in outputs.items():
            self.run('Java generic signature compile ' + provider, [self.javac, '-cp', cp(output, self.stdlib), '-d', self.work / ('java-' + provider), FIXTURES / 'Calls.java'])
            result = self.run('Verified fixed Java caller ' + provider, [self.java, '-Xverify:all', '-cp', cp(callers, output, self.stdlib), 'Calls'])
            self.case('runtime/' + provider, result.stdout == 'OK\n')
            for consumer_compiler in ['official', 'candidate']:
                consumer = self.work / (provider + '-' + consumer_compiler + '-consumer')
                self.compile(consumer_compiler, FIXTURES / 'Consumer.kt', consumer, [output])
                launcher = self.work / (provider + '-' + consumer_compiler + '-launcher')
                self.run('Compile consumer launcher', [self.javac, '-cp', cp(consumer, output, self.stdlib), '-d', launcher, FIXTURES / 'ConsumerCalls.java'])
                result = self.run('Verified metadata consumer ' + provider + '/' + consumer_compiler, [self.java, '-Xverify:all', '-cp', cp(launcher, consumer, output, self.stdlib), 'ConsumerCalls'])
                self.case('consumer/' + provider + '/' + consumer_compiler, result.stdout == 'OK\n')
            for name in ['Holder', 'Bounded', 'Chars']:
                result = self.run('Class bound signature ' + provider + '/' + name, [self.javap, '-p', '-s', '-v', '-classpath', output, 'generics.' + name])
                bound = 'Ljava/lang/CharSequence;' if name == 'Chars' else 'Lgenerics/Label;'
                self.case('signature/' + provider + '/' + name, '<T::' + bound + '>' in result.stdout, expected_bound=bound)
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = self.work / 'boundaries' / item['id'] / 'Boundary.kt'
            source.parent.mkdir(parents=True)
            source.write_text(item['source'])
            oracle = self.run('Official boundary ' + item['id'], [self.kotlin, source, *self.options, '-d', source.parent / 'official'], (0, 1))
            actual = self.run('Candidate boundary ' + item['id'], [self.candidate, source, '--diagnostic-format', 'json', '-cp', self.stdlib, '-d', source.parent / 'candidate'], (0, 1))
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            accepted = item['official'] == 'accept'
            category = 'unsupported_language' if accepted else 'invalid_kotlin'
            valid = (oracle.returncode == 0) == accepted and actual.returncode != 0
            if item.get('code'):
                valid = valid and any(d.get('code') == item['code'] and d.get('category') == category for d in diagnostics)
            if item.get('diagnostic'):
                valid = valid and item['diagnostic'].lower() in actual.stderr.lower()
            self.report['cases'].append(dict(id='boundary/' + item['id'], outcome=('unsupported_language' if accepted else 'invalid_rejected') if valid else 'correctness_failure', source_sha256=sha(source), diagnostics=diagnostics, official_accepted=oracle.returncode == 0))
        self.report['emitted_classes_sha256'] = {provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider, output in outputs.items()}

    def finish(self):
        changed = [p for p, checksum in self.frozen.items() if sha(ROOT / p) != checksum]
        self.case('frozen_inputs', not changed and sha(self.candidate) == self.report['compiler']['binary_sha256'], changed=changed)
        counts = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['counts'] = counts
        failed = counts.get('correctness_failure', 0)
        self.report['status'] = 'failed' if failed else 'completed'
        self.report['feature_acceptance'] = {'family':'source_classifier_variance_and_interface_bounds', 'status':'failed' if failed else 'passed', 'derived_counts':counts}
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
