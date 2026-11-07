#!/usr/bin/env python3
"""Compare bounded primary-constructor overloads with the pinned Kotlin compiler."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/constructor_overloads'
APP_ROOT = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab'
APPS = [APP_ROOT / 'base/verses/EmptyableRecyclerView.kt', APP_ROOT / 'base/widget/ColorPreference.kt', APP_ROOT / 'songs/SongInfo.kt', APP_ROOT / 'base/verses/VerseTextCompose.kt', APP_ROOT / 'base/verses/VerseItemCompose.kt']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


def signatures(output):
    result, previous = [], None
    for line in output.splitlines():
        line = line.strip()
        if line.startswith(('public ', 'protected ')) and line.endswith(';'):
            previous = line
        elif line.startswith('descriptor:') and previous:
            result.append(previous + ' ' + line)
            previous = None
    return sorted(result)


class Check:
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-constructor-overloads'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.compiler = args.compiler.resolve()
        self.pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlin = ROOT / self.pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / self.pin['path'] / 'lib/kotlin-stdlib.jar'
        self.reflect = ROOT / self.pin['path'] / 'lib/kotlin-reflect.jar'
        self.parcelize = ROOT / self.pin['path'] / 'lib/parcelize-runtime.jar'
        jdk = Path(os.environ['JAVA_HOME']) / 'bin'
        self.java, self.javac, self.javap = [jdk / name for name in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'constructor_overloads']
        frozen = [*args.source_root.glob('src/**/*.rs'), *FIXTURES.glob('*'), ROOT / 'tests/constructor_overloads.rs', Path(__file__), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *APPS]
        self.frozen = {str(path.resolve()): sha(path) for path in sorted(frozen) if path.is_file()}
        self.dependency_hashes = {str(path.resolve()): sha(path) for path in [self.stdlib, self.reflect, self.parcelize]}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary': relative(self.compiler), 'binary_sha256': sha(self.compiler), 'source_files_sha256': {relative(path): value for path, value in self.frozen.items()}, 'frozen_source_root': relative(args.source_root)}, reference_compiler={'version': self.pin['version'], 'archive_sha256': self.pin['archive_sha256'], 'options': self.options}, inputs={'files_sha256': {relative(path): value for path, value in self.frozen.items()}, 'stdlib_sha256': sha(self.stdlib), 'reflect_sha256': sha(self.reflect), 'parcelize_runtime_sha256': sha(self.parcelize), 'kotlin_source_commit': json.loads((ROOT / 'references/sources.json').read_text())['kotlin']['commit']}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_constructor_overloads.py', '--compiler', relative(self.compiler), '--source-root', relative(args.source_root)] + (['--app-classpath', args.app_classpath] if args.app_classpath else [])], limitations=['Public primary constructors of ordinary nongeneric classes. Explicit nonpublic constructor visibility, inner, value, sealed and enum combinations remain unsupported. Defaults retain the existing 32-parameter mask limit.', 'Java callers compile exactly once against official output and identical caller classfiles run against both providers with -Xverify:all. Official Kotlin consumers compile against each provider separately.', 'Android source assessments are separate from feature acceptance; no Android, full-app, or performance claim. Parcelize transformations remain explicitly unsupported.'])

    def run(self, label, args, accepted=(0,)):
        result = subprocess.run(list(map(str, args)), cwd=ROOT, capture_output=True, text=True, timeout=240)
        index = len(self.report['commands'])
        entry = {'label': label, 'argv': list(map(str, args)), 'cwd': '.', 'exit_code': result.returncode}
        for stream in ['stdout', 'stderr']:
            path = self.logs / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(self, ident, passed, **fields):
        self.report['cases'].append({'id': ident, 'outcome': 'pass' if passed else 'correctness_failure', **fields})

    def execute(self):
        dependencies = self.work / 'dependencies'
        self.run('Compile independent recorder and runtime annotations', [self.javac, '-d', dependencies, FIXTURES / 'Effects.java', FIXTURES / 'Mark.java'])
        sources = [FIXTURES / 'Constructors.kt', FIXTURES / 'Lookalike.kt']
        official, candidate = self.work / 'official', self.work / 'candidate'
        base = cp(dependencies, self.stdlib)
        self.run('Compile official constructors', [self.kotlin, *sources, *self.options, '-cp', base, '-d', official])
        self.run('Compile candidate constructors', [self.compiler, *sources, '--module-name', 'constructor_overloads', '-cp', base, '-d', candidate])
        callers = self.work / 'callers'
        self.run('Compile Java callers once against official', [self.javac, '-cp', cp(official, dependencies, self.stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        caller_hashes = {relative(path): sha(path) for path in sorted(callers.rglob('*.class'))}
        self.report['fixed_java_callers_sha256'] = caller_hashes
        for provider, output in [('official', official), ('candidate', candidate)]:
            result = self.run('Run fixed Java caller ' + provider, [self.java, '-Xverify:all', '-cp', cp(callers, output, dependencies, self.stdlib), 'constructors.Calls'])
            self.case('java/' + provider, result.stdout == 'OK\n', scope='Interleaved required parameters, dependent defaults, evaluation order, exceptions, pre-default null checks, 32-bit signed mask, data/internal/open classes, runtime constructor and parameter annotations, canonical aliases, user lookalike')
            consumer = self.work / (provider + '-consumer')
            self.run('Compile official Kotlin consumer ' + provider, [self.kotlin, FIXTURES / 'Consumer.kt', *self.options, '-cp', cp(output, dependencies, self.stdlib), '-d', consumer])
            launcher = self.work / (provider + '-consumer-caller')
            self.run('Compile consumer launcher ' + provider, [self.javac, '-cp', cp(consumer, output, dependencies, self.stdlib), '-d', launcher, FIXTURES / 'ConsumerCalls.java'])
            result = self.run('Run metadata consumer ' + provider, [self.java, '-Xverify:all', '-cp', cp(launcher, consumer, output, dependencies, self.stdlib, self.reflect), 'ConsumerCalls'])
            self.case('metadata/' + provider, result.stdout == 'OK\n')
        self.case('fixed_java_callers_unchanged', caller_hashes == {relative(path): sha(path) for path in sorted(callers.rglob('*.class'))})
        for owner in ['Interleaved', 'Dependent', 'Exceptional', 'Decorated', 'Parent', 'Child', 'Record', 'Internal', 'NoDefaults', 'Explicit', 'Mask32', 'Wide', 'Nullable', 'Automatic', 'custom.Lookalike']:
            owner = owner if '.' in owner else 'constructors.' + owner
            expected = self.run('Official ABI ' + owner, [self.javap, '-protected', '-s', '-classpath', official, owner])
            actual = self.run('Candidate ABI ' + owner, [self.javap, '-protected', '-s', '-classpath', candidate, owner])
            self.case('abi/' + owner, signatures(expected.stdout) == signatures(actual.stdout), expected=signatures(expected.stdout), actual=signatures(actual.stdout))
        self.report['emitted_classes_sha256'] = {provider: {relative(path): sha(path) for path in output.rglob('*.class')} for provider, output in [('official', official), ('candidate', candidate)]}
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = self.work / 'rejections' / item['id'] / 'Rejected.kt'
            source.parent.mkdir(parents=True)
            source.write_text(item['source'])
            libraries = cp(self.stdlib, *([self.parcelize] if item.get('parcelize_runtime') else []))
            oracle = self.run('Official boundary ' + item['id'], [self.kotlin, source, *self.options, '-cp', libraries, '-d', source.parent / 'official'], accepted=(0, 1))
            result = self.run('Candidate boundary ' + item['id'], [self.compiler, source, '--module-name', 'constructor_overloads', '--diagnostic-format', 'json', '-cp', libraries, '-d', source.parent / 'candidate'], accepted=(0, 1))
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            valid = (oracle.returncode == 0) == item['official_accepts'] and result.returncode != 0
            if 'code' in item:
                valid = valid and any(value.get('code') == item['code'] and value.get('category') == item['category'] for value in diagnostics)
            elif 'message' in item:
                valid = valid and item['message'] in result.stderr
            outcome = ('unsupported_language' if item.get('category') == 'unsupported_language' else 'invalid_rejected') if valid else 'correctness_failure'
            self.report['cases'].append({'id': 'boundary/' + item['id'], 'outcome': outcome, 'source_sha256': sha(source), 'official_accepted': oracle.returncode == 0, 'diagnostics': diagnostics})
        for source in APPS:
            if not self.args.app_classpath:
                self.report['cases'].append({'id': 'app/' + source.stem, 'outcome': 'not_assessed', 'acceptance_scope': 'external_app_assessment', 'source': relative(source), 'source_sha256': sha(source), 'reason': 'Actual Android/AndroidX and app dependency classpath unavailable; source remains unchanged. Supply --app-classpath to assess.'})
                continue
            base = cp(self.args.app_classpath, self.stdlib)
            for provider in ['official', 'candidate']:
                args = [self.kotlin, source, *self.options] if provider == 'official' else [self.compiler, source, '--module-name', 'constructor_overloads', '--diagnostic-format', 'json']
                result = self.run('Assess unchanged app ' + source.stem + ' ' + provider, [*args, '-cp', base, '-d', self.work / ('app-' + source.stem + '-' + provider)], accepted=(0, 1))
                self.report['cases'].append({'id': 'app/' + source.stem + '/' + provider, 'outcome': 'compilation_accepted' if result.returncode == 0 else 'compilation_rejected', 'acceptance_scope': 'external_app_assessment', 'source': relative(source), 'source_sha256': sha(source)})

    def finish(self):
        changed = [relative(path) for path, checksum in self.frozen.items() if sha(path) != checksum]
        changed_dependencies = [relative(path) for path, checksum in self.dependency_hashes.items() if sha(path) != checksum]
        self.case('frozen_dependencies', not changed_dependencies, changed=changed_dependencies)
        self.case('frozen_inputs', not changed and sha(self.compiler) == self.report['compiler']['binary_sha256'], changed=changed)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        selected = [case for case in self.report['cases'] if case.get('acceptance_scope') != 'external_app_assessment']
        counts = dict(Counter(case['outcome'] for case in selected))
        failed = counts.get('correctness_failure', 0)
        self.report['feature_acceptance'] = {'family': 'primary_constructor_jvm_overloads', 'status': 'failed' if failed else 'passed', 'derived_counts': counts}
        self.report['status'] = 'failed' if failed else 'completed'
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True, text=True)
        print(relative(path))
        print(json.dumps(self.report['counts'], sort_keys=True))
        return 1 if failed else 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--app-classpath')
    args = parser.parse_args()
    args.source_root = args.source_root.resolve()
    check = Check(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append({'id': 'harness', 'outcome': 'correctness_failure', 'error': str(error)})
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
