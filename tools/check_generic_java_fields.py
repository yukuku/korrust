#!/usr/bin/env python3
"""Frozen differential evidence for specialized Java generic instance field reads."""
import argparse
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import shutil
import subprocess

from check_internaldb_wrapper import Check, cp, relative, sha, signatures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/generic_java_fields'


class FieldsCheck(Check):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-generic-java-fields'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.candidate = self.work / 'korrust-frozen'
        shutil.copy2(args.candidate, self.candidate)
        reference = args.reference_root.resolve()
        pin = json.loads((reference / 'references/compiler.json').read_text())
        kotlin_root = reference / pin['path']
        self.kotlin = kotlin_root / 'bin/kotlinc'
        self.stdlib = kotlin_root / 'lib/kotlin-stdlib.jar'
        self.annotations = kotlin_root / 'lib/annotations-13.0.jar'
        self.jdk = Path(os.environ['JAVA_HOME']) / 'bin'
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'generic_java_fields']
        paths = [*FIXTURES.glob('*'), ROOT / 'tests/generic_java_fields.rs', Path(__file__), ROOT / 'tools/check_internaldb_wrapper.py', self.candidate, self.stdlib, self.annotations, kotlin_root / 'lib/kotlin-compiler.jar', reference / 'references/compiler.json', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', *(ROOT / 'src').rglob('*.rs'), reference / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/sync/Sync.java']
        if args.baseline:
            paths.append(args.baseline)
        if args.nullable_parameter_candidate:
            paths.append(args.nullable_parameter_candidate)
        if args.nullable_parameter_source_root:
            paths.extend((args.nullable_parameter_source_root / 'src').rglob('*.rs'))
            paths.extend(args.nullable_parameter_source_root / p for p in ['Cargo.toml', 'Cargo.lock'])
        self.frozen = {relative(p):sha(p) for p in sorted(paths) if p.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'binary':relative(self.candidate), 'binary_sha256':sha(self.candidate), 'source_revision':subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()}, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'options':self.options}, inputs={'files_sha256':self.frozen}, reproduction_commands=[['python3', 'tools/check_generic_java_fields.py', '--candidate', str(args.candidate), '--reference-root', str(reference), *(['--baseline', str(args.baseline)] if args.baseline else [])]], commands=[], cases=[], evidence=[], limitations=[
            'Public generic Java instance field signatures retain declaring-class type parameters, nullable/non-null declaration or root type-use annotations and erased JVM descriptors. Receiver substitution includes inherited generic supertypes, direct out/in/star reads, invariant nested lists/arrays and boxed primitives.',
            'Generic writes and static generic field reads remain explicitly unsupported. Nested parameter-dependent reads through projected receivers, unrecognized field annotations, and non-null field enhancement of nullable source type parameters remain explicit boundaries. This is a bounded JVM language feature; no new unchanged app-file, Android execution, full-app compatibility or speed claim is made.',
            'One fixed Java caller compiled against official Kotlin output runs against both providers under -Xverify:all, with independent value/null/exception/evaluation-count assertions. Public JVM signatures are compared. Original Sync.Entity<C>.content is relevance evidence; its application classpath is not substituted by the focused Java fixture.'
        ])

    def execute(self):
        dependency = self.work / 'dependency'
        self.run('Compile ordinary generic Java fields', [self.jdk / 'javac', '-cp', self.annotations, '-d', dependency, *(FIXTURES / p for p in ['Cell.java', 'StringCell.java', 'NumberCell.java', 'Provider.java'])])
        classpath = cp(dependency, self.stdlib, self.annotations)
        source = FIXTURES / 'Use.kt'
        if self.args.baseline:
            result = subprocess.run([str(self.args.baseline), str(source), '-cp', classpath, '-d', str(self.work / 'baseline')], text=True, capture_output=True)
            for stream in ['stdout', 'stderr']:
                path = self.logs / ('baseline-' + stream + '.txt')
                path.write_text(getattr(result, stream))
            self.case('baseline-missing-support', result.returncode == 1 and 'Java field' in result.stderr, exit_code=result.returncode, stdout=relative(self.logs / 'baseline-stdout.txt'), stderr=relative(self.logs / 'baseline-stderr.txt'))
        outputs = {provider:self.work / provider for provider in ['official', 'candidate']}
        public = {}
        for provider, output in outputs.items():
            argv = [self.kotlin, source, *self.options] if provider == 'official' else [self.candidate, source, '--module-name', 'generic_java_fields']
            self.run('Compile specialized field reads ' + provider, [*argv, '-cp', classpath, '-d', output])
            self.case('compile/' + provider, True)
            result = self.run('Public JVM signatures ' + provider, [self.jdk / 'javap', '-public', '-s', '-classpath', output, 'genericfields.UseKt'])
            public[provider] = signatures(result.stdout)
        self.report['public_jvm_signatures'] = public
        self.case('public-jvm-signatures', public['official'] == public['candidate'])
        caller = self.work / 'fixed-caller'
        self.run('Compile fixed Java caller against official output', [self.jdk / 'javac', '-cp', cp(outputs['official'], classpath), '-d', caller, FIXTURES / 'Driver.java'])
        for provider, output in outputs.items():
            result = self.run('Verified fixed Java caller ' + provider, [self.jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, classpath), 'genericfields.Driver'])
            self.case('runtime/' + provider, result.stdout == 'OK\n')
        if self.args.nullable_parameter_candidate:
            self.report['nullable_parameter_compiler'] = {'binary':relative(self.args.nullable_parameter_candidate), 'binary_sha256':sha(self.args.nullable_parameter_candidate), 'source_root':relative(self.args.nullable_parameter_source_root) if self.args.nullable_parameter_source_root else None, 'dependency':'Includes the separately validated Java-platform generic-bound feature; this interaction is not a second field feature.'}
            self.report['reproduction_commands'][0].extend(['--nullable-parameter-candidate', str(self.args.nullable_parameter_candidate)])
            if self.args.nullable_parameter_source_root:
                self.report['reproduction_commands'][0].extend(['--nullable-parameter-source-root', str(self.args.nullable_parameter_source_root)])
            nullable_outputs = {provider:self.work / ('nullable-' + provider) for provider in ['official', 'candidate']}
            for provider, output in nullable_outputs.items():
                argv = [self.kotlin, FIXTURES / 'NullableParameter.kt', *self.options] if provider == 'official' else [self.args.nullable_parameter_candidate, FIXTURES / 'NullableParameter.kt', '--module-name', 'generic_java_fields']
                self.run('Compile unconstrained field-result parameter ' + provider, [*argv, '-cp', classpath, '-d', output])
                self.case('nullable-parameter-compile/' + provider, True)
            nullable_caller = self.work / 'nullable-caller'
            self.run('Compile fixed nullable-parameter caller against official output', [self.jdk / 'javac', '-cp', cp(nullable_outputs['official'], classpath), '-d', nullable_caller, FIXTURES / 'NullableParameterCalls.java'])
            for provider, output in nullable_outputs.items():
                result = self.run('Verify nullable and nonnull parameter bounds ' + provider, [self.jdk / 'java', '-Xverify:all', '-cp', cp(nullable_caller, output, classpath), 'genericfields.NullableParameterCalls'])
                self.case('nullable-parameter-runtime/' + provider, result.stdout == 'OK\n')
            source = FIXTURES / 'RequiredParameter.kt'
            self.run('Oracle accepts nonnull enhancement of nullable parameter', [self.kotlin, source, *self.options, '-cp', classpath, '-d', self.work / 'required-official'])
            actual = subprocess.run(list(map(str, [self.args.nullable_parameter_candidate, source, '--diagnostic-format', 'json', '-cp', classpath, '-d', self.work / 'required-candidate'])), text=True, capture_output=True, timeout=600)
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            entry = dict(label='Explicit nonnull nullable-parameter boundary', argv=list(map(str, [self.args.nullable_parameter_candidate, source, '--diagnostic-format', 'json', '-cp', classpath, '-d', self.work / 'required-candidate'])), exit_code=actual.returncode)
            for stream in ['stdout', 'stderr']:
                path = self.logs / ('required-parameter-' + stream + '.txt')
                path.write_text(getattr(actual, stream))
                entry[stream] = relative(path)
            self.report['commands'].append(entry)
            valid = actual.returncode == 1 and any(d.get('code') == 'NONNULL_JAVA_FIELD_PARAMETER_UNSUPPORTED' and d.get('category') == 'unsupported_language' for d in diagnostics)
            self.report['cases'].append(dict(id='boundary/nonnull-nullable-parameter', outcome='unsupported_language' if valid else 'correctness_failure', diagnostics=diagnostics, stdout=relative(self.logs / 'required-parameter-stdout.txt'), stderr=relative(self.logs / 'required-parameter-stderr.txt')))
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = self.work / 'boundaries' / item['id'] / 'Boundary.kt'
            source.parent.mkdir(parents=True)
            source.write_text(item['source'])
            outcomes = {}
            for provider in ['official', 'candidate']:
                argv = [self.kotlin, source, *self.options] if provider == 'official' else [self.candidate, source, '--diagnostic-format', 'json']
                result = subprocess.run(list(map(str, [*argv, '-cp', classpath, '-d', source.parent / provider])), text=True, capture_output=True, timeout=600)
                entry = dict(label='Boundary ' + provider + '/' + item['id'], argv=list(map(str, [*argv, '-cp', classpath, '-d', source.parent / provider])), exit_code=result.returncode)
                for stream in ['stdout', 'stderr']:
                    path = self.logs / f'{len(self.report["commands"]):03d}-{stream}.txt'
                    path.write_text(getattr(result, stream)); entry[stream] = relative(path)
                self.report['commands'].append(entry)
                outcomes[provider] = result
            oracle, actual = outcomes['official'], outcomes['candidate']
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            accepted = item['official'] == 'accept'
            valid = (oracle.returncode == 0) == accepted and actual.returncode == 1
            if not accepted:
                valid &= any(d.get('category') == 'invalid_kotlin' for d in diagnostics)
            if item.get('code'):
                valid &= any(d.get('code') == item['code'] and d.get('category') == 'unsupported_language' for d in diagnostics)
            self.report['cases'].append(dict(id='boundary/' + item['id'], outcome=('unsupported_language' if accepted else 'invalid_rejected') if valid else 'correctness_failure', official_accepted=oracle.returncode == 0, diagnostics=diagnostics, source_sha256=sha(source)))
        self.report['emitted_classes_sha256'] = {provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider, output in outputs.items()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    parser.add_argument('--nullable-parameter-candidate', type=Path)
    parser.add_argument('--nullable-parameter-source-root', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    args.candidate = args.candidate.resolve()
    if args.baseline:
        args.baseline = args.baseline.resolve()
    if args.nullable_parameter_candidate:
        args.nullable_parameter_candidate = args.nullable_parameter_candidate.resolve()
    if args.nullable_parameter_source_root:
        args.nullable_parameter_source_root = args.nullable_parameter_source_root.resolve()
    check = FieldsCheck(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append(dict(id='harness', outcome='correctness_failure', error=str(error)))
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
