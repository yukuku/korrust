#!/usr/bin/env python3
"""Verify unchanged SyncUtils against a frozen integrated compiler and genuine app dependencies."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

from check_internaldb_wrapper import Check, cp, relative, sha, signatures

ROOT = Path(__file__).resolve().parents[1]
SOURCE = Path('benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/sync/SyncUtils.kt')
CALLER = ROOT / 'tests/syncutils_unchanged/SyncCheck.java'
OWNER = 'yuku/alkitab/base/sync/SyncUtils'


class SyncCheck(Check):
    def __init__(self, args):
        self.args = args
        config = json.loads(args.classpaths.read_text())['cases'][str(SOURCE)]
        self.dependencies = [Path(p).resolve() for p in config['classpath']]
        self.friends = [Path(p).resolve() for p in config.get('friend_paths', [])]
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-unchanged-syncutils'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.candidate = self.work / 'korrust-frozen'
        shutil.copy2(args.candidate, self.candidate)
        self.reference = args.reference_root.resolve()
        pin = json.loads((self.reference / 'references/compiler.json').read_text())
        kotlin = self.reference / pin['path']
        self.kotlin = kotlin / 'bin/kotlinc'
        self.stdlib = kotlin / 'lib/kotlin-stdlib.jar'
        self.jdk = Path(os.environ['JAVA_HOME']) / 'bin'
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'syncutils_proof']
        files = [self.candidate, self.reference / SOURCE, CALLER, Path(__file__), ROOT / 'tools/check_internaldb_wrapper.py', args.classpaths, self.stdlib, kotlin / 'lib/kotlin-compiler.jar', self.reference / 'references/compiler.json', self.reference / 'benchmarks/sources.json', args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *(args.source_root / 'src').rglob('*.rs'), *self.dependencies, *self.friends]
        expanded = []
        for path in files:
            expanded.extend(path.rglob('*.class') if path.is_dir() else [path])
        self.frozen = {relative(p):sha(p) for p in sorted(set(expanded))}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary':relative(self.candidate), 'binary_sha256':sha(self.candidate), 'source_root':relative(args.source_root), 'source_revision':args.source_revision}, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'options':self.options}, inputs={'files_sha256':self.frozen, 'source':str(SOURCE), 'classpath_config':config, 'classpath':list(map(str, self.dependencies)), 'friend_paths':list(map(str, self.friends)), 'caller':relative(CALLER)}, reproduction_commands=[['mkdir', '-p', str(args.source_root)], ['git', '-C', str(self.reference), 'archive', args.source_revision, '--format=tar', '-o', str(args.source_root.parent / 'syncutils-source-snapshot.tar'), 'src', 'Cargo.toml', 'Cargo.lock'], ['tar', '-xf', str(args.source_root.parent / 'syncutils-source-snapshot.tar'), '-C', str(args.source_root)], ['cargo', 'build', '--release', '--offline', '--locked', '--manifest-path', str(args.source_root / 'Cargo.toml'), '--target-dir', str(args.candidate.parent.parent)], ['python3', 'tools/check_syncutils.py', '--candidate', str(args.candidate), '--source-root', str(args.source_root), '--source-revision', args.source_revision, '--reference-root', str(self.reference), '--classpaths', str(args.classpaths)]], commands=[], cases=[], evidence=[], limitations=[
            'Preliminary integrated-revision proof for unchanged SyncUtils.kt with all genuine 163 production classpath entries, including source-filtered original Java/Kotlin app outputs. Tested-source class prefixes are independently checked absent from every dependency jar.',
            'The exact fixed Java caller supplied by the source survey is compiled against official output and reused against both providers under -Xverify:all. Assertions cover nonidentical equal contents, null contents, gid/kind mismatches, first matching entity identity, empty results, callback execution, exception cause/message and declared checked exceptions.',
            'Public JVM declarations, generic signatures and descriptors are compared for the object and nested interface. This does not claim full Kotlin metadata compatibility, Android framework execution, an entire Korrust app build or performance.'
        ])

    def execute(self):
        leaks = []
        for path in self.dependencies:
            with zipfile.ZipFile(path) as archive:
                leaks.extend(str(path) + ':' + name for name in archive.namelist() if name.startswith(OWNER) and name.endswith('.class'))
        self.case('tested-source-excluded-from-classpath', not leaks, leaked_classes=leaks)
        if leaks:
            raise AssertionError('Tested-source output leaked into dependency classpath')
        classpath = cp(*self.dependencies, self.stdlib)
        outputs = {provider:self.work / provider for provider in ['official', 'candidate']}
        public = {}
        for provider, output in outputs.items():
            argv = [self.kotlin, self.reference / SOURCE, *self.options] if provider == 'official' else [self.candidate, self.reference / SOURCE, '--module-name', 'syncutils_proof']
            if self.friends:
                argv.extend(['-Xfriend-paths=' + cp(*self.friends)] if provider == 'official' else ['--friend-paths', cp(*self.friends)])
            self.run('Compile unchanged SyncUtils ' + provider, [*argv, '-cp', classpath, '-d', output])
            self.case('unchanged-source/' + provider, True, source_sha256=sha(self.reference / SOURCE))
            public[provider] = {}
            for suffix in ['', '$ThrowEverythingRunnable']:
                name = (OWNER + suffix).replace('/', '.')
                result = self.run('JVM signatures ' + provider + '/' + name, [self.jdk / 'javap', '-public', '-s', '-classpath', cp(output, classpath), name])
                public[provider][name] = signatures(result.stdout)
        caller = self.work / 'fixed-caller'
        self.run('Compile fixed Java caller against official output', [self.jdk / 'javac', '-cp', cp(outputs['official'], classpath), '-d', caller, CALLER])
        self.case('fixed-official-java-caller', True)
        for provider, output in outputs.items():
            result = self.run('Verified fixed Java caller ' + provider, [self.jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, classpath), 'SyncCheck'])
            self.case('runtime/' + provider, result.stdout == '')
        self.report['public_jvm_signatures'] = public
        for name in public['official']:
            self.case('public-jvm-signatures/' + name, public['official'][name] == public['candidate'][name])
        self.report['emitted_classes_sha256'] = {provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider, output in outputs.items()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--classpaths', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, required=True)
    parser.add_argument('--source-revision', required=True)
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    for key in ['candidate', 'classpaths', 'source_root', 'reference_root']:
        setattr(args, key, getattr(args, key).resolve())
    check = SyncCheck(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append(dict(id='harness', outcome='correctness_failure', error=str(error)))
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
