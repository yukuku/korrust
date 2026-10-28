#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

from upstream_jvm import parse_fixture, write_sources

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/java_interop_new'
SNAPSHOT = ROOT / 'benchmarks/sources/androidbible'
APP = SNAPSHOT / 'AlkitabYes2/src/test/java/yuku/alkitab/yes2/SnappyStreamRoundTripTest.kt'
UPSTREAM = ROOT / 'references/kotlin/compiler/testData/codegen/box/javaVisibility/package/inheritedPackageStaticFunction.kt'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


def signatures(output):
    result = []
    declaration = None
    for line in map(str.strip, output.splitlines()):
        if line.startswith(('public ', 'protected ')) and line.endswith(';'):
            declaration = line
        elif line.startswith('descriptor:') and declaration:
            result.append(declaration + ' ' + line)
            declaration = None
    return sorted(result)


class Check:
    def __init__(self, args):
        self.args = args
        self.compiler = args.compiler.resolve()
        self.now = datetime.now(timezone.utc)
        self.ident = self.now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-new-java-interop'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlin = ROOT / self.pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / self.pin['path'] / 'lib/kotlin-stdlib.jar'
        java_home = Path(os.environ['JAVA_HOME'])
        self.java, self.javac, self.javap = [java_home / 'bin' / name for name in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_java_interop']
        self.app_java = [*SNAPSHOT.glob('Snappy/src/main/java/**/*.java'), *SNAPSHOT.glob('Bintex/src/main/java/**/*.java'), *[SNAPSHOT / ('AlkitabYes2/src/main/java/yuku/alkitab/yes2/' + path) for path in ['compress/SnappyInputStream.java', 'compress/SnappyOutputStream.java', 'io/RandomInputStream.java', 'io/RandomAccessFileRandomInputStream.java']]]
        frozen = [*args.source_root.glob('src/**/*.rs'), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *FIXTURES.glob('*'), Path(__file__), ROOT / 'tools/upstream_jvm.py', APP, UPSTREAM, *self.app_java]
        self.frozen = {relative(path): sha(path) for path in sorted(frozen) if path.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=self.now.isoformat(), status='running', compiler={'binary': relative(self.compiler), 'binary_sha256': sha(self.compiler), 'frozen_source_root': relative(args.source_root)}, reference_compiler={'version': self.pin['version'], 'archive_sha256': self.pin['archive_sha256'], 'options': self.options}, inputs={'files_sha256': self.frozen, 'stdlib_sha256': sha(self.stdlib), 'junit_sha256': sha(args.junit_jar), 'hamcrest_sha256': sha(args.hamcrest_jar), 'reference_commit': json.loads((ROOT / 'references/sources.json').read_text())['kotlin']['commit']}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_new_java_interop.py', '--compiler', relative(self.compiler), '--source-root', relative(args.source_root), '--junit-jar', str(args.junit_jar), '--hamcrest-jar', str(args.hamcrest_jar)]], limitations=['Explicit Java public static imports, aliases, inherited overloads/fields and ordinary calls; static-member wildcard and static-interface method imports remain explicit unsupported boundaries.', 'Synthetic getter/setter properties on ordinary concrete Java-class receivers, including generic Java classes and inherited accessibility bridges. Interface, enum, Kotlin and mapped java.lang receiver/origin cases remain explicitly unsupported where source classifier provenance is required.', 'Public non-generic builtin-class functions are decoded by declaration, retaining parameter names, nullability and JVM signatures. Raw Object calls are limited to matching kotlin.Any declarations; actual JVM implementations execute normally.', 'String platform concatenation uses the existing String concatenation operation; Unit operands explicitly reject. Nullable String receiver extension operators remain a separate existing boundary.', 'Selected unchanged upstream original and AndroidBible app test progress are separate from whole-app compatibility. Java callers compile once against official output and reuse identical bytecodes; official Kotlin consumers compile against each output.', 'No Android-device execution, full-app, production, performance or exhaustive upstream-suite claim.'])

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
        dependency = self.work / 'dependency'
        java_sources = [path for path in FIXTURES.glob('*.java') if path.name not in ['Calls.java', 'ConsumerCalls.java']]
        self.run('Compile original independent Java API fixtures', [self.javac, '-d', dependency, *java_sources])
        official, candidate = self.work / 'official', self.work / 'candidate'
        self.run('Compile official interop clients', [self.kotlin, FIXTURES / 'Use.kt', *self.options, '-cp', dependency, '-d', official])
        self.run('Compile candidate interop clients', [self.compiler, FIXTURES / 'Use.kt', '--module-name', 'new_java_interop', '-cp', cp(dependency, self.stdlib), '-d', candidate])
        callers = self.work / 'callers'
        self.run('Compile fixed independent Java assertions once against official', [self.javac, '-cp', cp(official, dependency, self.stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        for provider, output in [('official', official), ('candidate', candidate)]:
            observed = self.run('Run identical Java assertion bytecodes ' + provider, [self.java, '-Xverify:all', '-cp', cp(callers, output, dependency, self.stdlib), 'newinterop.Calls'])
            self.case('java/' + provider, observed.stdout == 'OK\n', scope='Static aliases, inherited overloads/fields, varargs; getters/setters/nulls; inherited virtual dispatch/accessibility bridges; platform concatenation; Any; effect order; generic JavaBean substitution')
            consumer = self.work / (provider + '-consumer')
            self.run('Official Kotlin metadata consumer ' + provider, [self.kotlin, FIXTURES / 'Consumer.kt', *self.options, '-cp', cp(output, dependency, self.stdlib), '-d', consumer])
            launcher = self.work / (provider + '-consumer-launcher')
            self.run('Compile metadata consumer launcher ' + provider, [self.javac, '-cp', cp(consumer, output, dependency, self.stdlib), '-d', launcher, FIXTURES / 'ConsumerCalls.java'])
            observed = self.run('Run metadata consumer ' + provider, [self.java, '-Xverify:all', '-cp', cp(launcher, consumer, output, dependency, self.stdlib), 'ConsumerCalls'])
            self.case('metadata/' + provider, observed.stdout == 'OK\n')
        expected = self.run('Official public JVM signatures', [self.javap, '-protected', '-s', '-classpath', official, 'newinterop.use.UseKt'])
        actual = self.run('Candidate public JVM signatures', [self.javap, '-protected', '-s', '-classpath', candidate, 'newinterop.use.UseKt'])
        self.case('abi', signatures(expected.stdout) == signatures(actual.stdout), expected=signatures(expected.stdout), actual=signatures(actual.stdout))
        self.report['emitted_classes_sha256'] = {provider: {relative(path): sha(path) for path in output.rglob('*.class')} for provider, output in [('official', official), ('candidate', candidate)]}
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = self.work / 'boundaries' / item['id'] / 'Boundary.kt'
            source.parent.mkdir(parents=True)
            source.write_text(item['source'])
            oracle = self.run('Official boundary ' + item['id'], [self.kotlin, source, *self.options, '-cp', dependency, '-d', source.parent / 'official'], accepted=(0, 1))
            actual = self.run('Candidate boundary ' + item['id'], [self.compiler, source, '--diagnostic-format', 'json', '-cp', cp(dependency, self.stdlib), '-d', source.parent / 'candidate'], accepted=(0, 1))
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            valid = (oracle.returncode == 0) == (item['official'] == 'accept') and actual.returncode != 0 and item['diagnostic'] in actual.stderr
            if item.get('code'):
                valid = valid and any(diagnostic.get('code') == item['code'] and diagnostic.get('category') == item['category'] for diagnostic in diagnostics)
            if not valid:
                outcome = 'correctness_failure'
            elif item['official'] == 'accept':
                outcome = 'unsupported_language' if diagnostics and diagnostics[0].get('category') == 'unsupported_language' else 'unclassified_compilation_failure'
            else:
                outcome = 'invalid_rejected'
            self.report['cases'].append({'id': 'boundary/' + item['id'], 'outcome': outcome, 'source_sha256': sha(source), 'diagnostics': diagnostics, 'official_accepted': oracle.returncode == 0})
        fixture = parse_fixture(UPSTREAM)
        modules = write_sources(fixture, self.work / 'upstream-sources')
        self.report['inputs']['upstream_directives'] = [module['directives'] for module in fixture['modules']]
        lib = self.work / 'upstream-lib'
        self.run('Compile unchanged upstream Java dependency', [self.javac, '-d', lib, *modules['lib']['paths']])
        upstream_official, upstream_candidate = self.work / 'upstream-official', self.work / 'upstream-candidate'
        self.run('Compile unchanged upstream official Kotlin', [self.kotlin, *modules['main']['paths'], *self.options, '-cp', lib, '-d', upstream_official])
        owners = [str(path.relative_to(upstream_official).with_suffix('')).replace(os.sep, '.') for path in upstream_official.rglob('*.class')]
        owner = next(owner for owner in owners if '$' not in owner)
        driver = self.work / 'UpstreamCalls.java'
        driver.write_text('public final class UpstreamCalls { public static void main(String[] args) { String value = ' + owner + '.box(); if (!value.equals("OK")) throw new AssertionError(value); System.out.println("OK"); } }\n')
        upstream_callers = self.work / 'upstream-callers'
        self.run('Compile fixed upstream Java caller against official', [self.javac, '-cp', cp(upstream_official, lib, self.stdlib), '-d', upstream_callers, driver])
        self.run('Compile unchanged upstream candidate Kotlin', [self.compiler, *modules['main']['paths'], '-cp', cp(lib, self.stdlib), '-d', upstream_candidate])
        for provider, output in [('official', upstream_official), ('candidate', upstream_candidate)]:
            observed = self.run('Run unchanged upstream ' + provider, [self.java, '-Xverify:all', '-cp', cp(upstream_callers, output, lib, self.stdlib), 'UpstreamCalls'])
            self.case('upstream/' + provider, observed.stdout == 'OK\n', original=relative(UPSTREAM), source_sha256=sha(UPSTREAM), scope='Original MODULE/FILE source text; inaccessible Java declaring class/public qualifier; ordinary String concatenation of platform results')
        app_dependency = self.work / 'app-dependency'
        self.run('Compile unchanged original Snappy/Bintex/stream Java closure', [self.javac, '-d', app_dependency, *self.app_java])
        app_official, app_candidate = self.work / 'app-official', self.work / 'app-candidate'
        app_cp = cp(app_dependency, self.args.junit_jar, self.args.hamcrest_jar, self.stdlib)
        self.run('Compile unchanged AndroidBible app test oracle', [self.kotlin, APP, *self.options, '-cp', app_cp, '-d', app_official])
        observed = self.run('Run original AndroidBible JUnit oracle', [self.java, '-Xverify:all', '-cp', cp(app_official, app_cp), 'org.junit.runner.JUnitCore', 'yuku.alkitab.yes2.SnappyStreamRoundTripTest'])
        self.case('app/official', 'OK (4 tests)' in observed.stdout, acceptance_scope='external_app_assessment')
        actual = self.run('Assess original AndroidBible test candidate', [self.compiler, APP, '--diagnostic-format', 'json', '-cp', app_cp, '-d', app_candidate], accepted=(0, 1))
        diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
        if actual.returncode:
            category = diagnostics[0].get('category') if diagnostics else None
            outcome = 'unsupported_language' if category == 'unsupported_language' else 'unclassified_compilation_failure' if category == 'unclassified' else 'correctness_failure'
            self.report['cases'].append({'id': 'app/candidate', 'outcome': outcome, 'acceptance_scope': 'external_app_assessment', 'diagnostics': diagnostics, 'source_sha256': sha(APP), 'scope': 'Matched original real dependency closure; first diagnostic probe, no full app compatibility claim'})
        else:
            observed = self.run('Run original AndroidBible JUnit candidate', [self.java, '-Xverify:all', '-cp', cp(app_candidate, app_cp), 'org.junit.runner.JUnitCore', 'yuku.alkitab.yes2.SnappyStreamRoundTripTest'])
            self.case('app/candidate', 'OK (4 tests)' in observed.stdout, acceptance_scope='external_app_assessment')

    def finish(self):
        changed = [path for path, checksum in self.frozen.items() if sha(ROOT / path) != checksum]
        self.case('frozen_inputs', not changed and sha(self.compiler) == self.report['compiler']['binary_sha256'], changed=changed)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        selected = [case for case in self.report['cases'] if case.get('acceptance_scope') != 'external_app_assessment']
        counts = dict(Counter(case['outcome'] for case in selected))
        failed = counts.get('correctness_failure', 0)
        self.report['feature_acceptance'] = {'family': 'java_static_imports_and_bounded_synthetic_properties', 'status': 'failed' if failed else 'passed', 'derived_counts': counts, 'excluded_assessment_cases': [case['id'] for case in self.report['cases'] if case.get('acceptance_scope') == 'external_app_assessment']}
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
    parser.add_argument('--junit-jar', type=Path, required=True)
    parser.add_argument('--hamcrest-jar', type=Path, required=True)
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
