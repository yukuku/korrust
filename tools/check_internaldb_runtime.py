#!/usr/bin/env python3
"""Run unchanged transaction-wrapper outputs with genuine Robolectric and native SQLite."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import subprocess
import zipfile

from check_internaldb_wrapper import Check, OWNER, cp, relative, sha

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'tests/internaldb_wrapper/DatabaseSmoke.java'
MARKER = 'TRANSACTION_PROOF: rollback, commit, repeated commit, exception, exception after commit, retained handle'


class RuntimeCheck(Check):
    def __init__(self, args):
        self.args = args
        self.compilation = json.loads(args.compile_report.read_text())
        if self.compilation['status'] != 'completed':
            raise ValueError('Compile proof must complete successfully first')
        receipt = json.loads(args.test_runtime_receipt.read_text())
        self.dependencies = [Path(p) for p in dict.fromkeys([*self.compilation['inputs']['classpath'], *receipt['dependency_classpath']])]
        if any(not p.is_file() for p in self.dependencies):
            raise ValueError('Every runtime classpath artifact must exist as a genuine jar')
        self.outputs = {p:Path(o) for p, o in self.compilation['outputs'].items()}
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-internaldb-wrapper-robolectric'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.jdk = Path(os.environ['JAVA_HOME']) / 'bin'
        files = [Path(__file__), ROOT / 'tools/check_internaldb_wrapper.py', FIXTURE, args.compile_report, args.test_runtime_receipt, *self.dependencies, *args.sdk_directory.glob('*.jar'), *args.sdk_directory.glob('receipt*.json'), self.jdk / 'java', self.jdk / 'javac']
        files.extend(p for output in self.outputs.values() for p in output.rglob('*.class'))
        self.frozen = {relative(p):sha(p) for p in sorted(set(files))}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=self.compilation['compiler'], reference_compiler=self.compilation['reference_compiler'], inputs={'files_sha256':self.frozen, 'compile_report':relative(args.compile_report), 'runtime_receipt':relative(args.test_runtime_receipt), 'robolectric_sdk_directory':relative(args.sdk_directory), 'omitted_original_app_runtime_jar':receipt['omitted_original_app_runtime_jar'], 'classpath':list(map(str, self.dependencies))}, reproduction_commands=[['python3', 'tools/check_internaldb_runtime.py', '--compile-report', str(args.compile_report), '--test-runtime-receipt', str(args.test_runtime_receipt), '--sdk-directory', str(args.sdk_directory)]], commands=[], cases=[], evidence=[], limitations=[
            'A fixed JUnit caller compiled against official wrapper output executes both providers with -Xverify:all under genuine Robolectric4.13, instrumented Android SDK28 and native SQLite. Genuine Android14 resource-SDK and native-runtime dependency artifacts are frozen too. No Android framework or app dependency sources/classes are substituted.',
            'The unchanged original InternalDbHelper opens an existing native SQLite database pre-created at the actual BuildConfig database version. The test creates its own ordinary table; it does not exercise first-install schema creation or unrelated eager Firebase/Crashlytics application bootstrap. Original InternalDb and helper implementations remain in the genuine filtered app sibling jar.',
            'Checks cover rollback without commit, commit, repeated commit calls, callback exception identity and rollback before/after commit, retained handles, closed transactions and committed row values. This is a JVM test-environment proof, not device/emulator execution, a full Korrust app build or a performance claim.'
        ])

    def execute(self):
        leaks = []
        for path in self.dependencies:
            with zipfile.ZipFile(path) as archive:
                leaks.extend(str(path) + ':' + name for name in archive.namelist() if name.startswith(OWNER) and name.endswith('.class'))
        self.case('tested-source-excluded-from-dependencies', not leaks, leaked_classes=leaks)
        if leaks:
            return
        for provider, output in self.outputs.items():
            expected = {path.split('/' + provider + '/', 1)[1]:digest for path, digest in self.compilation['emitted_classes_sha256'][provider].items()}
            actual = {str(p.relative_to(output)):sha(p) for p in output.rglob('*.class')}
            self.case('unchanged-compiled-provider/' + provider, actual == expected)
            if actual != expected:
                return
        caller = self.work / 'fixed-caller'
        self.run('Compile fixed JUnit caller against official output', [self.jdk / 'javac', '-cp', cp(self.outputs['official'], *self.dependencies), '-d', caller, FIXTURE])
        self.case('fixed-official-java-caller', True)
        for provider, output in self.outputs.items():
            try:
                result = self.run('Verified native SQLite transaction proof ' + provider, [self.jdk / 'java', '-Xmx2g', '-Xverify:all', '-Drobolectric.offline=true', '-Drobolectric.dependency.dir=' + str(self.args.sdk_directory), '-Duser.language=en', '-Duser.country=US', '-cp', cp(caller, output, *self.dependencies), 'org.junit.runner.JUnitCore', 'wrapperproof.DatabaseSmoke'])
            except AssertionError as error:
                self.report['cases'].append(dict(id='native-sqlite-runtime/' + provider, outcome='runtime_environment_failure' if provider == 'official' else 'correctness_failure', reason=str(error)))
                if provider == 'official':
                    self.report['cases'].append(dict(id='native-sqlite-runtime/candidate', outcome='not_run', reason='Official provider did not execute successfully'))
                return
            self.case('native-sqlite-runtime/' + provider, MARKER in result.stdout and 'OK (1 test)' in result.stdout)
        self.report['fixed_caller_classes_sha256'] = {relative(p):sha(p) for p in caller.rglob('*.class')}

    def finish(self):
        changed = [p for p, digest in self.frozen.items() if sha(ROOT / p) != digest]
        self.case('frozen-inputs', not changed, changed=changed)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['status'] = 'failed' if any(self.report['counts'].get(k) for k in ['correctness_failure', 'runtime_environment_failure']) else 'completed'
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        if not self.args.skip_html:
            subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True)
        print(relative(path)); print(json.dumps(self.report['counts'], sort_keys=True))
        return int(self.report['status'] == 'failed')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compile-report', type=Path, required=True)
    parser.add_argument('--test-runtime-receipt', type=Path, required=True)
    parser.add_argument('--sdk-directory', type=Path, required=True)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    for key in ['compile_report', 'test_runtime_receipt', 'sdk_directory']:
        setattr(args, key, getattr(args, key).resolve())
    check = RuntimeCheck(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append(dict(id='harness', outcome='runtime_environment_failure', reason=str(error)))
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
