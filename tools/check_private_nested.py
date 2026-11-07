#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import zipfile
from check_inner_classes import signatures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/private_nested'
APP_PATHS = [
    'Alkitab/src/main/java/yuku/alkitab/base/util/BackForwardListController.kt',
    'Alkitab/src/main/java/yuku/alkitab/songs/newdoc/LegacyParcelDecoder.kt',
]
OWNERS = ['privateprobe.Vault$Entry', 'privateprobe.Vault$Record', 'privateprobe.Vault$Reader', 'privateprobe.ObjectOwner$Reader', 'privateprobe.Depth$Hidden']


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def cp(*paths):
    return os.pathsep.join(map(str, paths))


class Check:
    def __init__(self, args):
        self.compiler = args.compiler.resolve()
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-private-nested'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlin = ROOT / pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        self.compiler_jar = ROOT / pin['path'] / 'lib/kotlin-compiler.jar'
        self.java, self.javac, self.javap = [Path(os.environ['JAVA_HOME']) / 'bin' / name for name in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'private_nested']
        self.frozen_paths = sorted({*args.source_root.glob('src/**/*.rs'), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *FIXTURES.glob('*'), ROOT / 'tests/private_nested.rs', ROOT / 'tests/test_check_private_nested.py', Path(__file__), ROOT / 'tools/check_inner_classes.py', ROOT / 'tools/upstream_jvm.py', ROOT / 'references/compiler.json', self.compiler, self.stdlib, self.compiler_jar, *[ROOT / 'benchmarks/sources/androidbible' / name for name in APP_PATHS]})
        self.frozen = {relative(path): sha(path) for path in self.frozen_paths}
        archive = self.logs / 'frozen-sources.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as output:
            for path in self.frozen_paths:
                if path not in (self.compiler, self.stdlib, self.compiler_jar):
                    output.write(path, relative(path))
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', environment={'platform': platform.platform(), 'java_home': os.environ['JAVA_HOME']}, compiler={'binary': relative(self.compiler), 'binary_sha256': sha(self.compiler), 'frozen_source_root': relative(args.source_root)}, reference_compiler={'version': pin['version'], 'archive_sha256': pin['archive_sha256'], 'compiler_jar_sha256': sha(self.compiler_jar), 'options': self.options}, inputs={'files_sha256': self.frozen}, source_archive={'path': relative(archive), 'sha256': sha(archive)}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_private_nested.py', '--compiler', relative(self.compiler), '--source-root', relative(args.source_root)]], limitations=['One bounded feature family: private static nested final ordinary/data classes. Existing public nested declarations are prerequisites and are not a new feature count.', 'Private inner, open, abstract, sealed and value classes, generic enclosing classes, escaped dollar classifier names and top-level private classes remain unsupported. Individual members retain existing visibility limitations; this feature does not add access to private sibling members.', 'No unchanged whole AndroidBible source, Android build, APK/device, or full-app compatibility claim. Both reviewed files retain independent blockers.'])

    def run(self, label, command, accepted=(0,)):
        result = subprocess.run(list(map(str, command)), cwd=ROOT, capture_output=True, text=True, timeout=300)
        entry = {'label': label, 'argv': list(map(str, command)), 'cwd': '.', 'exit_code': result.returncode}
        for stream in ['stdout', 'stderr']:
            path = self.logs / f'{len(self.report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(self, ident, passed, **fields):
        self.report['cases'].append({'id': ident, 'outcome': 'pass' if passed else 'correctness_failure', **fields})

    def compile(self, label, source, output, provider, dependencies=(), accepted=(0,)):
        if provider == 'official':
            command = [self.kotlin, *source, *self.options, '-cp', cp(*dependencies, self.stdlib), '-d', output]
        else:
            command = [self.compiler, *source, '--module-name', 'private_nested', '--diagnostic-format', 'json', '-cp', cp(*dependencies, self.stdlib), '-d', output]
        return self.run(label, command, accepted)

    def execute(self):
        self.run('java-version', [self.java, '-version'])
        outputs = {provider: self.work / provider for provider in ['official', 'candidate']}
        sources = [FIXTURES / 'Vault.kt', FIXTURES / 'ObjectOwner.kt']
        for provider, output in outputs.items():
            self.compile('library/' + provider, sources, output, provider)
        callers = self.work / 'fixed-java'
        self.run('fixed-java/compile-once-official', [self.javac, '-cp', cp(outputs['official'], self.stdlib), '-d', callers, FIXTURES / 'Driver.java'])
        hashes = {relative(file): sha(file) for file in callers.rglob('*.class')}
        self.report['fixed_java_callers_sha256'] = hashes
        runs = {}
        for provider, output in outputs.items():
            runs[provider] = self.run('runtime/' + provider, [self.java, '-Xverify:all', '-cp', cp(callers, output, self.stdlib), 'Driver']).stdout
        expected = 'Record(id=9, text=ok):42\n13\n11\n17\n31\n23\n23\nEntry:private static final\nRecord:private static final\nReader:private static final\n'
        self.case('runtime/independent_expected', runs['official'] == expected, expected=expected, actual=runs['official'])
        self.case('runtime/same_java_bytecode', runs['official'] == runs['candidate'], expected=runs['official'], actual=runs['candidate'], verified=True)
        self.case('runtime/fixed_java_unchanged', hashes == {relative(file): sha(file) for file in callers.rglob('*.class')})
        for file in sorted(outputs['official'].rglob('*.class')):
            owner = str(file.relative_to(outputs['official']).with_suffix('')).replace(os.sep, '.')
            if '$lambda$' in owner or owner.endswith('$closure$1'):
                continue
            abi = {}
            for provider, output in outputs.items():
                result = self.run('abi/' + provider + '/' + owner, [self.javap, '-protected', '-s', '-v', '-classpath', output, owner])
                abi[provider] = signatures(result.stdout)
                self.run('raw-all-members/' + provider + '/' + owner, [self.javap, '-p', '-s', '-v', '-classpath', output, owner])
            self.case('abi/' + owner, abi['official'] == abi['candidate'], expected=abi['official'], actual=abi['candidate'])
        tool = self.work / 'metadata-tool'
        self.run('metadata/compile-decoder', [self.javac, '-cp', cp(self.compiler_jar, self.stdlib), '-d', tool, FIXTURES / 'MetadataDump.java'])
        metadata = {}
        for provider, output in outputs.items():
            metadata[provider] = self.run('metadata/' + provider, [self.java, '-cp', cp(tool, output, self.compiler_jar, self.stdlib), 'MetadataDump', *OWNERS]).stdout
        self.case('metadata/private_data_and_constructor_flags', metadata['official'] == metadata['candidate'], expected=metadata['official'], actual=metadata['candidate'])
        for provider, output in outputs.items():
            for compiler in ['official', 'candidate']:
                consumer = self.work / f'consumer-{provider}-{compiler}'
                self.compile('consumer/' + provider + '/' + compiler, [FIXTURES / 'Consumer.kt'], consumer, compiler, [output])
                if provider == 'official' and compiler == 'official':
                    self.run('consumer/fixed-java/compile-once-official', [self.javac, '-cp', cp(consumer, output, self.stdlib), '-d', callers, FIXTURES / 'ConsumerDriver.java'])
                result = self.run('consumer/verified/' + provider + '/' + compiler, [self.java, '-Xverify:all', '-cp', cp(callers, consumer, output, self.stdlib), 'ConsumerDriver'])
                self.case('consumer/' + provider + '/' + compiler, result.stdout == 'Record(id=9, text=ok):42:11\n')
        for item in json.loads((FIXTURES / 'Boundaries.json').read_text()):
            directory = self.work / 'boundary' / item['id']
            directory.mkdir(parents=True)
            source = directory / 'Boundary.kt'
            source.write_text(item['source'])
            results = {provider: self.compile('boundary/' + item['id'] + '/' + provider, [source], directory / provider, provider, accepted=(0, 1)) for provider in outputs}
            diagnostics = [json.loads(line) for line in results['candidate'].stderr.splitlines() if line.startswith('{')]
            category = 'invalid_kotlin' if item['official'] == 'reject' else 'unsupported_language'
            valid = (results['official'].returncode == 0) == (item['official'] == 'accept') and results['candidate'].returncode != 0 and any(d.get('category') == category and d.get('code') == item['code'] for d in diagnostics)
            self.report['cases'].append({'id': 'boundary/' + item['id'], 'outcome': ('invalid_rejected' if item['official'] == 'reject' else 'unsupported_language') if valid else 'correctness_failure', 'official_accepted': results['official'].returncode == 0, 'diagnostics': diagnostics, 'source_sha256': sha(source)})
        for provider, output in outputs.items():
            for ident, text in [('constructor', 'fun bad(): Int = privateprobe.Vault.Entry(1, null).index'), ('type', 'fun bad(value: privateprobe.Vault.Entry): Int = 1'), ('import', 'import privateprobe.Vault.Entry\nfun bad(): Int = Entry(1, null).index'), ('nested_public', 'fun bad(): Int = privateprobe.Depth.Hidden.Public(1).x')]:
                directory = self.work / 'invalid-consumer' / provider / ident
                directory.mkdir(parents=True)
                source = directory / 'Invalid.kt'
                source.write_text(text)
                for compiler in ['official', 'candidate']:
                    result = self.compile('invalid-consumer/' + provider + '/' + ident + '/' + compiler, [source], directory / compiler, compiler, [output], accepted=(0, 1))
                    diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
                    valid = result.returncode != 0 and (compiler == 'official' or any(d.get('category') == 'invalid_kotlin' for d in diagnostics))
                    self.report['cases'].append({'id': 'invalid-consumer/' + provider + '/' + ident + '/' + compiler, 'outcome': 'invalid_rejected' if valid else 'correctness_failure', 'diagnostics': diagnostics, 'source_sha256': sha(source)})
        self.report['inputs']['androidbible_sources_reviewed'] = [{'path': 'benchmarks/sources/androidbible/' + name, 'sha256': sha(ROOT / 'benchmarks/sources/androidbible' / name), 'whole_file_status': 'not_assessed', 'new_unchanged_file_claim': False, 'reason': reason} for name, reason in zip(APP_PATHS, ['DisplayEntry private nested class shape is covered by Vault.Entry. Generic enclosing instance capture and Android/UI dependencies retain independent blockers.', 'Reader private nested class shape is covered by ObjectOwner.Reader. Private enum Format, Android/app model dependencies and decoder constructs retain independent blockers.'])]
        self.report['emitted_classes_sha256'] = {provider: {relative(file): sha(file) for file in output.rglob('*.class')} for provider, output in outputs.items()}

    def finish(self):
        changed = [relative(path) for path in self.frozen_paths if sha(path) != self.frozen[relative(path)]]
        self.case('frozen_inputs', not changed, changed=changed)
        counts = dict(Counter(case['outcome'] for case in self.report['cases']))
        failed = counts.get('correctness_failure', 0)
        self.report['counts'] = counts
        self.report['status'] = 'failed' if failed else 'completed'
        self.report['feature_acceptance'] = {'family': 'private_static_nested_classes', 'status': 'failed' if failed else 'passed', 'derived_counts': counts}
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True, text=True)
        print(relative(path))
        print(json.dumps(counts, sort_keys=True))
        return int(bool(failed))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
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
