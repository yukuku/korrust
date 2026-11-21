#!/usr/bin/env python3
"""Compile unchanged InternalDbTxWrapper against source-filtered genuine app outputs."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path('benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/storage/InternalDbTxWrapper.kt')
FIXTURES = ROOT / 'tests/internaldb_wrapper'
OWNER = 'yuku/alkitab/base/storage/InternalDbTxWrapper'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


def signatures(output):
    result, declaration = {}, None
    for line in output.splitlines():
        if line.startswith('  public ') or line.startswith('  protected '):
            declaration = line.strip()
        elif declaration and line.strip().startswith('descriptor:'):
            result[declaration] = line.strip().removeprefix('descriptor:').strip()
            declaration = None
    return result


class Check:
    def __init__(self, args):
        self.args = args
        self.reference = args.reference_root.resolve()
        config = json.loads(args.classpaths.read_text())
        matching = [(name, case) for name, case in config['cases'].items() if 'InternalDbTxWrapper' in name]
        if len(matching) != 1:
            raise ValueError('Expected exactly one InternalDbTxWrapper classpath case')
        self.case_id, config = matching[0]
        self.dependencies = [Path(p).resolve() for p in config['classpath']]
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-internaldb-wrapper-compile'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.candidate = self.work / 'korrust-frozen'
        shutil.copy2(args.candidate, self.candidate)
        pin = json.loads((self.reference / 'references/compiler.json').read_text())
        kotlin_root = self.reference / pin['path']
        self.kotlin = kotlin_root / 'bin/kotlinc'
        self.stdlib = kotlin_root / 'lib/kotlin-stdlib.jar'
        self.jdk = Path(os.environ['JAVA_HOME']) / 'bin'
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'internaldb_proof']
        files = [self.reference / SOURCE, *FIXTURES.glob('*'), Path(__file__), args.classpaths, self.candidate, self.stdlib, kotlin_root / 'lib/kotlin-compiler.jar', self.reference / 'references/compiler.json', self.reference / 'benchmarks/sources.json', args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *(args.source_root / 'src').rglob('*.rs')]
        for dep in self.dependencies:
            files.extend(dep.rglob('*.class') if dep.is_dir() else [dep])
        self.frozen = {relative(p):sha(p) for p in sorted(set(files)) if p.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary':relative(self.candidate), 'binary_sha256':sha(self.candidate), 'source_revision':subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=args.source_root, text=True).strip()}, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'options':self.options}, inputs={'files_sha256':self.frozen, 'source':str(SOURCE), 'classpaths_manifest':relative(args.classpaths), 'classpath_case':self.case_id, 'classpath':list(map(str, self.dependencies)), 'classpath_config':config}, reproduction_commands=[['python3', 'tools/check_internaldb_wrapper.py', '--candidate', str(args.candidate), '--source-root', str(args.source_root), '--reference-root', str(self.reference), '--classpaths', str(args.classpaths)]], commands=[], cases=[], evidence=[], limitations=[
            'Existing language-support proof for unchanged InternalDbTxWrapper.kt against genuine source-filtered official Gradle app outputs and captured production dependencies. No synthetic dependency classes or source rewrites are used.',
            'Public JVM declaration/descriptor signatures, official Kotlin consumers and Java source compatibility are checked for both providers. This compile report does not claim Android framework execution, transaction runtime behavior, or a full Korrust app build.'
        ])

    def run(self, label, argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=600)
        entry = dict(label=label, argv=list(map(str, argv)), exit_code=result.returncode)
        for stream in ['stdout', 'stderr']:
            path = self.logs / f'{len(self.report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(self, ident, passed, **fields):
        self.report['cases'].append(dict(id=ident, outcome='pass' if passed else 'correctness_failure', **fields))

    def execute(self):
        leaks = []
        for dep in self.dependencies:
            names = [str(p.relative_to(dep)) for p in dep.rglob('*.class')] if dep.is_dir() else zipfile.ZipFile(dep).namelist()
            leaks.extend(str(dep) + ':' + n for n in names if n.startswith(OWNER) and n.endswith('.class'))
        self.case('source-output-excluded-from-classpath', not leaks, leaked_classes=leaks)
        if leaks:
            raise AssertionError('Dependency classpath contains tested-source output')
        classpath = cp(*self.dependencies, self.stdlib)
        outputs = {provider:self.work / provider for provider in ['official', 'candidate']}
        public = {}
        for provider, output in outputs.items():
            argv = [self.kotlin, self.reference / SOURCE, *self.options] if provider == 'official' else [self.candidate, self.reference / SOURCE, '--module-name', 'internaldb_proof']
            self.run('Compile unchanged source ' + provider, [*argv, '-cp', classpath, '-d', output])
            self.case('unchanged-source/' + provider, True, source_sha256=sha(self.reference / SOURCE))
            self.run('Official Kotlin metadata consumer ' + provider, [self.kotlin, FIXTURES / 'Consumer.kt', *self.options, '-cp', cp(output, classpath), '-d', self.work / ('consumer-' + provider)])
            self.case('kotlin-metadata-consumer/' + provider, True)
            self.run('Java source caller ' + provider, [self.jdk / 'javac', '-cp', cp(output, classpath), '-d', self.work / ('java-' + provider), FIXTURES / 'AbiCalls.java'])
            self.case('java-source-compatibility/' + provider, True)
            public[provider] = {}
            for suffix in ['', '$TxHandle']:
                name = (OWNER + suffix).replace('/', '.')
                result = self.run('Public JVM signatures ' + provider + '/' + name, [self.jdk / 'javap', '-public', '-s', '-classpath', cp(output, classpath), name])
                public[provider][name] = signatures(result.stdout)
        self.report['public_jvm_signatures'] = public
        for name in public['official']:
            self.case('public-jvm-signatures/' + name, public['official'][name] == public['candidate'][name])
        self.report['outputs'] = {p:str(o.resolve()) for p, o in outputs.items()}
        self.report['emitted_classes_sha256'] = {provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider, output in outputs.items()}

    def finish(self):
        changed = [p for p, checksum in self.frozen.items() if sha(ROOT / p) != checksum]
        self.case('frozen-inputs', not changed, changed=changed)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['status'] = 'failed' if self.report['counts'].get('correctness_failure') else 'completed'
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        if not self.args.skip_html:
            subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True)
        print(relative(path))
        print(json.dumps(self.report['counts'], sort_keys=True))
        return int(self.report['status'] == 'failed')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--classpaths', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    for key in ['candidate', 'classpaths', 'source_root', 'reference_root']:
        setattr(args, key, getattr(args, key).resolve())
    check = Check(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append(dict(id='harness', outcome='correctness_failure', error=str(error)))
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
