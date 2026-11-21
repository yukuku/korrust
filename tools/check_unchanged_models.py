#!/usr/bin/env python3
"""Differential proof for unchanged AndroidBible models using original dependencies."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/unchanged_models'
APP = Path('benchmarks/sources/androidbible/Alkitab/src/main/java')
SOURCES = [APP / p for p in ['yuku/alkitab/base/audio/PlaybackState.kt', 'yuku/alkitab/datatransfer/model/Rpp.kt']]
DEPENDENCIES = [APP / p for p in ['yuku/alkitab/base/audio/AudioLogEntry.kt', 'yuku/alkitab/datatransfer/model/Gid.kt']]
CLASSES = ['yuku.alkitab.base.audio.PlaybackState', 'yuku.alkitab.base.audio.PlaybackState$Companion', 'yuku.alkitab.datatransfer.model.Rpp']


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
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-unchanged-models'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.reference = args.reference_root.resolve()
        pin = json.loads((self.reference / 'references/compiler.json').read_text())
        kotlin_root = self.reference / pin['path']
        self.kotlin = kotlin_root / 'bin/kotlinc'
        self.stdlib = kotlin_root / 'lib/kotlin-stdlib.jar'
        self.java, self.javac, self.javap = [Path(os.environ['JAVA_HOME']) / 'bin' / n for n in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'unchanged_models']
        paths = [*(self.reference / p for p in SOURCES + DEPENDENCIES), *FIXTURES.glob('*'), Path(__file__), *(args.source_root / 'src').glob('**/*.rs'), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', self.reference / 'references/compiler.json', self.reference / 'benchmarks/sources.json']
        self.frozen = {relative(p): sha(p) for p in sorted(paths) if p.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary':relative(args.candidate), 'binary_sha256':sha(args.candidate), 'source_root':relative(args.source_root), 'source_revision':subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=args.source_root, text=True).strip()}, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'compiler_jar_sha256':sha(kotlin_root / 'lib/kotlin-compiler.jar'), 'options':self.options}, inputs={'files_sha256':self.frozen, 'stdlib_sha256':sha(self.stdlib), 'jdk_java_base_jmod_sha256':sha(Path(os.environ['JAVA_HOME']) / 'jmods/java.base.jmod')}, reproduction_commands=[['python3', 'tools/check_unchanged_models.py', '--candidate', str(args.candidate), '--source-root', str(args.source_root), '--reference-root', str(self.reference)]], commands=[], cases=[], evidence=[], limitations=[
            'Evidence for existing language support, not a new compiler feature. The original app sources are unchanged. Original AudioLogEntry and Gid dependencies are compiled separately by the pinned official compiler; no dependency stubs or rewritten Kotlin inputs are used.',
            'The fixed Java caller and Kotlin consumer are compiled against official provider output and the same bytecode executes with both providers under -Xverify:all. A second official Kotlin consumer compile against candidate output checks exposed Kotlin metadata.',
            'Claims are scoped to the two unchanged model files, their public JVM signatures and the asserted runtime behavior; this is not an Android, full-module or full-application build.'
        ])

    def run(self, label, argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=600)
        entry = dict(label=label, argv=list(map(str, argv)), cwd=relative(ROOT), exit_code=result.returncode)
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

    def compile(self, provider, sources, output, dependencies=()):
        command = [self.kotlin, *sources, *self.options] if provider == 'official' else [self.args.candidate, *sources, '--module-name', 'unchanged_models']
        return self.run('Compile ' + provider + ' ' + ','.join(p.name for p in sources), [*command, '-cp', cp(*dependencies, self.stdlib), '-d', output])

    def execute(self):
        deps = self.work / 'original-dependencies'
        self.compile('official', [self.reference / p for p in DEPENDENCIES], deps)
        self.case('original-dependencies/official', True, source_paths=list(map(str, DEPENDENCIES)), classes_sha256={relative(p):sha(p) for p in deps.rglob('*.class')})
        outputs = {provider:self.work / provider for provider in ['official', 'candidate']}
        for provider, output in outputs.items():
            self.compile(provider, [self.reference / p for p in SOURCES], output, [deps])
            for source in SOURCES:
                self.case('unchanged-source/' + provider + '/' + source.stem, True, source=str(source), source_sha256=sha(self.reference / source))
        consumer = self.work / 'fixed-kotlin-consumer'
        self.compile('official', [FIXTURES / 'Consumer.kt'], consumer, [outputs['official'], deps])
        caller = self.work / 'fixed-java-caller'
        self.run('Compile fixed Java caller against official provider', [self.javac, '-cp', cp(outputs['official'], deps, consumer, self.stdlib), '-d', caller, FIXTURES / 'Calls.java'])
        public = {}
        for provider, output in outputs.items():
            if provider == 'candidate':
                self.compile('official', [FIXTURES / 'Consumer.kt'], self.work / ('metadata-' + provider), [output, deps])
            self.case('kotlin-metadata-consumer/' + provider, True)
            result = self.run('Verified fixed Java and Kotlin bytecode ' + provider, [self.java, '-Xverify:all', '-cp', cp(caller, consumer, output, deps, self.stdlib), 'Calls'])
            self.case('fixed-bytecode-runtime/' + provider, result.stdout == 'OK\n')
            self.run('Java source compatibility ' + provider, [self.javac, '-cp', cp(output, deps, consumer, self.stdlib), '-d', self.work / ('java-' + provider), FIXTURES / 'Calls.java'])
            self.case('java-source-compatibility/' + provider, True)
            public[provider] = {}
            for name in CLASSES:
                result = self.run('Public JVM signatures ' + provider + '/' + name, [self.javap, '-public', '-s', '-classpath', cp(output, deps), name])
                public[provider][name] = signatures(result.stdout)
        self.report['public_jvm_signatures'] = public
        for name in CLASSES:
            self.case('public-jvm-signatures/' + name, public['official'][name] == public['candidate'][name])
        self.report['emitted_classes_sha256'] = {provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider, output in outputs.items()}

    def finish(self):
        changed = [p for p, checksum in self.frozen.items() if sha(ROOT / p) != checksum]
        self.case('frozen-inputs', not changed and sha(self.args.candidate) == self.report['compiler']['binary_sha256'], changed=changed)
        counts = dict(Counter(case['outcome'] for case in self.report['cases']))
        self.report['counts'] = counts
        self.report['status'] = 'failed' if counts.get('correctness_failure') else 'completed'
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        if not self.args.skip_html:
            subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True)
        print(relative(path))
        print(json.dumps(counts, sort_keys=True))
        return int(self.report['status'] == 'failed')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    args.candidate = args.candidate.resolve()
    args.source_root = args.source_root.resolve()
    check = Check(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append(dict(id='harness', outcome='correctness_failure', error=str(error)))
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
