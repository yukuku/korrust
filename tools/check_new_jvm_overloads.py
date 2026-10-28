#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

from upstream_jvm import parse_fixture, write_sources

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/jvm_overloads'
APP = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/FormattedVerseText.kt'
UPSTREAM = ROOT / 'references/kotlin/compiler/testData/codegen/box/jvmOverloads/simpleJavaCall.kt'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


def signatures(output):
    declarations = []
    previous = None
    for line in output.splitlines():
        line = line.strip()
        if line.startswith(('public ', 'protected ')) and line.endswith(';'):
            previous = line
        elif line.startswith('descriptor:') and previous:
            declarations.append(previous + ' ' + line)
            previous = None
    return sorted(declarations)


class Check:
    def __init__(self, compiler, source_root):
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-new-jvm-overloads'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.compiler = compiler.resolve()
        self.pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlin = ROOT / self.pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / self.pin['path'] / 'lib/kotlin-stdlib.jar'
        self.java_home = Path(os.environ['JAVA_HOME'])
        self.java = self.java_home / 'bin/java'
        self.javac = self.java_home / 'bin/javac'
        self.javap = self.java_home / 'bin/javap'
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_jvm_overloads']
        frozen = [*source_root.glob('src/**/*.rs'), *FIXTURES.glob('*'), Path(__file__), ROOT / 'tools/upstream_jvm.py', source_root / 'Cargo.toml', source_root / 'Cargo.lock', APP, UPSTREAM]
        self.frozen = {relative(path): sha(path) for path in sorted(frozen) if path.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary': relative(self.compiler), 'binary_sha256': sha(self.compiler), 'source_files_sha256': self.frozen, 'frozen_source_root': relative(source_root)}, reference_compiler={'version': self.pin['version'], 'archive_sha256': self.pin['archive_sha256'], 'options': self.options}, inputs={'files_sha256': self.frozen, 'stdlib_sha256': sha(self.stdlib), 'kotlin_source_commit': json.loads((ROOT / 'references/sources.json').read_text())['kotlin']['commit']}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_new_jvm_overloads.py', '--compiler', relative(self.compiler), '--source-root', relative(source_root)]], limitations=['Function overload lowering within existing source-default limit of 32 parameters. Constructor annotations, extension defaults, qualified/aliased object JvmStatic annotations and source-interface methods reject; source generic classes remain a separate family.', 'Java source compiled once against official Kotlin output; identical caller bytecodes reused against both outputs. Kotlin metadata consumers compile with the official compiler against each output separately.', 'The unchanged FormattedVerseText app target is assessed separately and still fails compilation. An independent StringBuilder indexing reduction is recorded with actual structured diagnostic categories; no app compatibility or supported builtin-member mapping claim.', 'No Android-device, full-app, incremental-performance or complete upstream-suite claim.'])

    def run(self, label, args, accepted=(0,)):
        index = len(self.report['commands'])
        result = subprocess.run(list(map(str, args)), cwd=ROOT, capture_output=True, text=True, timeout=180)
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
        self.run('Compile independent effect recorder', [self.javac, '-d', dependencies, FIXTURES / 'Effects.java', FIXTURES / 'Marker.java'])
        base = cp(dependencies, self.stdlib)
        official, candidate = self.work / 'official', self.work / 'candidate'
        self.run('Compile official functions', [self.kotlin, FIXTURES / 'Functions.kt', *self.options, '-cp', base, '-d', official])
        self.run('Compile candidate functions', [self.compiler, FIXTURES / 'Functions.kt', '--module-name', 'new_jvm_overloads', '-cp', base, '-d', candidate])
        callers = self.work / 'callers'
        self.run('Compile fixed Java calls against official', [self.javac, '-cp', cp(official, dependencies, self.stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        for provider, output in [('official', official), ('candidate', candidate)]:
            result = self.run('Run fixed Java caller ' + provider, [self.java, '-Xverify:all', '-cp', cp(callers, output, dependencies, self.stdlib), 'overloads.Calls'])
            self.case('java/' + provider, result.stdout == 'OK\n', scope='Independent values, effects, null-before-default ordering, exceptions, generic signatures, varargs, nontrailing defaults, signed bit-31 mask, object/static/companion dispatch')
            consumer = self.work / (provider + '-consumer')
            self.run('Official Kotlin metadata consumer ' + provider, [self.kotlin, FIXTURES / 'Consumer.kt', *self.options, '-cp', cp(output, dependencies, self.stdlib), '-d', consumer])
            consumer_callers = self.work / (provider + '-consumer-callers')
            self.run('Compile metadata consumer Java launcher ' + provider, [self.javac, '-cp', cp(consumer, output, dependencies, self.stdlib), '-d', consumer_callers, FIXTURES / 'ConsumerCalls.java'])
            result = self.run('Run metadata consumer ' + provider, [self.java, '-Xverify:all', '-cp', cp(consumer_callers, consumer, output, dependencies, self.stdlib), 'ConsumerCalls'])
            self.case('metadata/' + provider, result.stdout == 'OK\n')
        for owner in ['overloads.FunctionsKt', 'overloads.Member', 'overloads.Member$Companion', 'overloads.Singleton', 'overloads.Annotations', 'overloads.Annotations$Companion', 'overloads.Exceptional', 'overloads.Stateful', 'overloads.DefaultState', 'overloads.CompanionState', 'overloads.CompanionState$Companion', 'overloads.VirtualBase', 'overloads.VirtualDerived']:
            expected = self.run('Official descriptors ' + owner, [self.javap, '-protected', '-s', '-classpath', official, owner])
            actual = self.run('Candidate descriptors ' + owner, [self.javap, '-protected', '-s', '-classpath', candidate, owner])
            self.case('abi/' + owner, signatures(expected.stdout) == signatures(actual.stdout), expected=signatures(expected.stdout), actual=signatures(actual.stdout))
        self.report['emitted_classes_sha256'] = {provider: {relative(path): sha(path) for path in output.rglob('*.class')} for provider, output in [('official', official), ('candidate', candidate)]}
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = self.work / 'rejections' / item['id'] / 'Rejected.kt'
            source.parent.mkdir(parents=True)
            source.write_text(item['source'])
            oracle = self.run('Official rejection ' + item['id'], [self.kotlin, source, *self.options, '-cp', self.stdlib, '-d', source.parent / 'official'], accepted=(0, 1))
            actual = self.run('Candidate rejection ' + item['id'], [self.compiler, source, '--diagnostic-format', 'json', '-cp', self.stdlib, '-d', source.parent / 'candidate'], accepted=(0, 1))
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            valid = (oracle.returncode == 0) == (item['official'] == 'accept') and actual.returncode != 0 and item['diagnostic'] in actual.stderr
            if item.get('code'):
                valid = valid and any(diagnostic.get('code') == item['code'] and diagnostic.get('category') == item.get('category', 'invalid_kotlin') for diagnostic in diagnostics)
            if valid and item['official'] == 'accept':
                outcome = 'unsupported_language' if diagnostics and diagnostics[0].get('category') == 'unsupported_language' else 'unclassified_compilation_failure'
            else:
                outcome = 'invalid_rejected' if valid else 'correctness_failure'
            self.report['cases'].append({'id': 'boundary/' + item['id'], 'outcome': outcome, 'source_sha256': sha(source), 'official_accepted': oracle.returncode == 0, 'candidate_diagnostic': actual.stderr, 'diagnostics': diagnostics})
        fixture = parse_fixture(UPSTREAM)
        paths = write_sources(fixture, self.work / 'upstream')['main']['paths']
        self.report['inputs']['upstream_directives'] = fixture['modules'][0]['directives']
        kotlin_paths = [path for path in paths if path.suffix == '.kt']
        java_paths = [path for path in paths if path.suffix == '.java']
        upstream_official, upstream_candidate = self.work / 'upstream-official', self.work / 'upstream-candidate'
        self.run('Compile unchanged extracted upstream Kotlin and Java signatures', [self.kotlin, *paths, *self.options, '-cp', self.stdlib, '-d', upstream_official])
        upstream_callers = self.work / 'upstream-callers'
        self.run('Compile original upstream Java caller once against official', [self.javac, '-cp', cp(upstream_official, self.stdlib), '-d', upstream_callers, *java_paths, FIXTURES / 'SimpleUpstreamCalls.java'])
        self.run('Compile original upstream Kotlin with fixed Java caller class', [self.compiler, *kotlin_paths, '-cp', cp(upstream_callers, self.stdlib), '-d', upstream_candidate])
        for provider, output in [('official', upstream_official), ('candidate', upstream_candidate)]:
            result = self.run('Run unchanged original upstream ' + provider, [self.java, '-Xverify:all', '-cp', cp(upstream_callers, output, self.stdlib), 'SimpleUpstreamCalls'])
            self.case('upstream/simpleJavaCall/' + provider, result.stdout == 'OK\n', original=relative(UPSTREAM), source_sha256=sha(UPSTREAM), scope='Original FILE directives split without source rewriting; Java bytecode fixed from official output')
        app_official, app_candidate = self.work / 'app-official', self.work / 'app-candidate'
        self.run('Compile unchanged FormattedVerseText official', [self.kotlin, APP, *self.options, '-cp', self.stdlib, '-d', app_official])
        app_callers = self.work / 'app-callers'
        self.run('Compile independent unchanged app assertions', [self.javac, '-cp', cp(app_official, self.stdlib), '-d', app_callers, FIXTURES / 'AppCalls.java'])
        result = self.run('Run independent app oracle', [self.java, '-Xverify:all', '-cp', cp(app_callers, app_official, self.stdlib), 'AppCalls'])
        self.case('app/official', result.stdout == 'OK\n', source=relative(APP), source_sha256=sha(APP))
        for ident, source in [('app/candidate', APP), ('app/indexing_reduction', FIXTURES / 'IndexingBoundary.kt')]:
            if source != APP:
                self.run('Official indexing reduction', [self.kotlin, source, *self.options, '-cp', self.stdlib, '-d', self.work / 'indexing-official'])
            actual = self.run('Assess ' + ident, [self.compiler, source, '--diagnostic-format', 'json', '-cp', self.stdlib, '-d', app_candidate if source == APP else self.work / 'indexing-candidate'], accepted=(0, 1))
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            category = diagnostics[0].get('category') if diagnostics else None
            outcome = 'unsupported_language' if actual.returncode and category == 'unsupported_language' else 'unclassified_compilation_failure' if actual.returncode and category == 'unclassified' else 'correctness_failure' if actual.returncode else 'not_assessed'
            self.report['cases'].append({'id': ident, 'outcome': outcome, 'acceptance_scope': 'external_app_assessment', 'source': relative(source), 'source_sha256': sha(source), 'diagnostics': diagnostics, 'reason': 'App progress probe, outside positive family acceptance. JvmOverloads annotation no longer blocks the unchanged source; StringBuilder indexing is independently reduced, without assigning a stronger diagnostic category.'})

    def finish(self):
        changed = [path for path, checksum in self.frozen.items() if sha(ROOT / path) != checksum]
        self.case('frozen_inputs', not changed and sha(self.compiler) == self.report['compiler']['binary_sha256'], changed=changed)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        selected = [case for case in self.report['cases'] if case.get('acceptance_scope') != 'external_app_assessment']
        selected_counts = dict(Counter(case['outcome'] for case in selected))
        failed = selected_counts.get('correctness_failure', 0)
        self.report['feature_acceptance'] = {'family': 'jvm_overloads_function_lowering', 'status': 'failed' if failed else 'passed', 'derived_counts': selected_counts, 'excluded_assessment_cases': [case['id'] for case in self.report['cases'] if case.get('acceptance_scope') == 'external_app_assessment']}
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
    args = parser.parse_args()
    check = Check(args.compiler, args.source_root.resolve())
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append({'id': 'harness', 'outcome': 'correctness_failure', 'error': str(error)})
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
