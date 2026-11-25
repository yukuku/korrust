#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/anonymous_subclasses'
GENERIC = ROOT / 'tests/anonymous_generics'
TYPEVARS = ROOT / 'tests/anonymous_typevars'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--exposure-only', action='store_true')
    parser.add_argument('--generic-only', action='store_true')
    args = parser.parse_args()
    compiler = args.compiler.resolve()
    stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ') + '-anonymous-subclasses'
    work = ROOT / 'target/checks' / stamp
    evidence = ROOT / 'reports/correctness' / stamp
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    report_path = evidence.with_suffix('.json')
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(path for path in FIXTURES.rglob('*') if path.is_file()), *sorted(path for path in GENERIC.rglob('*') if path.is_file()), *sorted(path for path in TYPEVARS.rglob('*') if path.is_file()), ROOT / 'tests/anonymous_typevars.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json']

    def relative(path):
        return str(Path(path).relative_to(ROOT)) if Path(path).is_relative_to(ROOT) else str(path)

    def sha(path):
        return hashlib.sha256(Path(path).read_bytes()).hexdigest()

    frozen = {str(path): sha(path) for path in [*inputs, compiler, standard]}
    archive = evidence / 'inputs.zip'
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
        for path in inputs:
            bundle.writestr(relative(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler={'path': str(compiler), 'sha256': sha(compiler)}, reference_compiler={**json.loads((ROOT / 'references/compiler.json').read_text()), 'options': options}, inputs={'sha256': {relative(Path(path)): digest for path, digest in frozen.items()}}, source_archive={'path': relative(archive), 'sha256': sha(archive)}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_anonymous_subclasses.py', '--compiler', str(compiler), *(['--exposure-only'] if args.exposure_only else []), *(['--generic-only'] if args.generic_only else [])]], limitations=['Concrete generic and nongeneric class and interface bases with constructor substitution, override specialization, erased bridges and JVM generic superclass/interface signatures. Own members must be override functions; anonymous properties, generic methods and covariant bridges remain diagnosed. Superclass default/reordered arguments and conflicting inherited defaults are outside this evidence.', 'Lexical type variables in anonymous supertypes are covered in generic functions, generic class methods, primary-constructor initializers and generic interface default methods, with EnclosingMethod/InnerClasses and reflective generic declarations, override signatures and captures. Anonymous Kotlin metadata and full anonymous identity in exported APIs remain outside this evidence. Shadowed method/class type-variable names, generic suspend contexts, ordinary-lambda enclosing contexts remain explicit unsupported boundaries.', 'Java callers compile once against official Kotlin output, then execute unchanged with verification against both providers.'])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        command = list(map(str, command))
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=240)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append({'label': label, 'argv': command, 'exit_code': result.returncode, 'log': relative(log)})
        save()
        if result.returncode not in accepted:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def exercise(name, sources, dependencies, caller_file, main_class, facade):
        providers = {provider: work / name / provider for provider in ['official', 'candidate']}
        run(name + '/official', [kotlin, *sources, *options, '-cp', cp(*dependencies, standard), '-d', providers['official']])
        run(name + '/candidate', [compiler, *sources, '-cp', cp(*dependencies, standard), '-d', providers['candidate']])
        caller = work / name / 'caller'
        run(name + '/javac-fixed', ['javac', '-cp', cp(providers['official'], *dependencies, standard), '-d', caller, caller_file])
        fixed = {str(path): sha(path) for path in caller.rglob('*.class')}
        signatures = {}
        for provider, output in providers.items():
            result = run(name + '/verify/' + provider, ['java', '-Xverify:all', '-cp', cp(caller, output, *dependencies, standard), main_class])
            assert result.stdout == 'OK\n', result.stdout
            report['cases'].append({'id': name + '/verify/' + provider, 'outcome': 'pass', 'assertion': 'Provider behavior, dispatch, fresh instances and unchanged Java caller verification'})
            text = run(name + '/abi/' + provider, ['javap', '-s', '-p', '-cp', cp(output, *dependencies, standard), facade]).stdout
            public = []
            active = False
            for line in text.splitlines():
                line = line.strip()
                if line.startswith('descriptor:'):
                    if active:
                        public.append(line)
                elif line.endswith((';', '{')):
                    active = line.startswith('public ')
                    if active:
                        public.append(line)
            signatures[provider] = sorted(public)
        assert signatures['official'] == signatures['candidate'], signatures
        assert fixed == {str(path): sha(path) for path in caller.rglob('*.class')}
        report['cases'].append({'id': name + '/public-abi-and-fixed-caller', 'outcome': 'pass', 'signatures': signatures['candidate'], 'caller_sha256': fixed})
        save()

    try:
        if not args.exposure_only:
            if not args.generic_only:
                exercise('source', [FIXTURES / 'Base.kt', FIXTURES / 'Source.kt'], [], FIXTURES / 'Check.java', 'Check', 'anonymousclasses.SourceKt')
                library = work / 'compiled-base'
                run('compiled-base/library', [kotlin, FIXTURES / 'Base.kt', *options, '-d', library])
                exercise('compiled-base', [FIXTURES / 'Source.kt'], [library], FIXTURES / 'Check.java', 'Check', 'anonymousclasses.SourceKt')
                java_library = work / 'java-base'
                run('java-base/library', ['javac', '-d', java_library, FIXTURES / 'JavaBase.java'])
                exercise('java-base', [FIXTURES / 'Java.kt'], [java_library], FIXTURES / 'JavaCheck.java', 'JavaCheck', 'anonymousjava.JavaKt')
            exercise('generic-source', [GENERIC / 'Base.kt', GENERIC / 'Source.kt'], [], GENERIC / 'Check.java', 'Check', 'anonymousgeneric.SourceKt')
            generic_library = work / 'generic-library'
            run('generic-library/official', [kotlin, GENERIC / 'Base.kt', *options, '-d', generic_library])
            exercise('generic-compiled', [GENERIC / 'Source.kt'], [generic_library], GENERIC / 'Check.java', 'Check', 'anonymousgeneric.SourceKt')
            generic_java = work / 'generic-java-library'
            run('generic-java/library', ['javac', '-d', generic_java, GENERIC / 'JavaBase.java'])
            exercise('generic-java', [GENERIC / 'Java.kt'], [generic_library, generic_java], GENERIC / 'JavaCheck.java', 'JavaCheck', 'anonymousgeneric.JavaKt')
            exercise('generic-enclosing', [TYPEVARS / 'Model.kt'], [], TYPEVARS / 'Driver.java', 'typevars.Driver', 'typevars.ModelKt')
        if not args.generic_only:
            exercise('exposure', [FIXTURES / 'Exposure.kt'], [], FIXTURES / 'ExposureCheck.java', 'ExposureCheck', 'anonymousexposure.ExposureKt')
        for path in sorted([*(FIXTURES / 'boundaries').glob('*.kt'), *(GENERIC / 'boundaries').glob('*.kt'), *(TYPEVARS / 'boundaries').glob('*.kt')]):
            if args.generic_only and not (path.is_relative_to(GENERIC) or path.is_relative_to(TYPEVARS)):
                continue
            if args.exposure_only and not path.stem.startswith('InvalidPublic'):
                continue
            official = run('boundary/' + path.stem + '/official', [kotlin, path, *options, '-cp', standard, '-d', work / path.stem / 'official'], (0, 1))
            if path.stem in {'Generic', 'EnclosingParameter', 'EnclosingNestedParameter'}:
                candidate = run('boundary/' + path.stem + '/candidate', [compiler, path, '-cp', standard, '-d', work / path.stem / 'candidate'])
                assert official.returncode == candidate.returncode == 0
                report['cases'].append({'id': 'boundary/' + path.stem, 'outcome': 'pass'})
                continue
            candidate = run('boundary/' + path.stem + '/candidate', [compiler, path, '--diagnostic-format', 'json', '-cp', standard, '-d', work / path.stem / 'candidate'], (1,))
            diagnostics = [json.loads(line) for line in candidate.stderr.splitlines() if line.startswith('{')]
            category = 'invalid_kotlin' if path.stem.startswith('Invalid') else 'unsupported_language'
            assert diagnostics[0]['category'] == category, diagnostics
            assert official.returncode == (1 if category == 'invalid_kotlin' else 0), official.stderr
            report['cases'].append({'id': 'boundary/' + path.stem, 'outcome': 'invalid_rejected' if category == 'invalid_kotlin' else 'unsupported_language', 'diagnostic': diagnostics})
        assert all(sha(path) == digest for path, digest in frozen.items()), 'Frozen inputs changed'
        report['status'] = 'passed'
    except Exception as error:
        report['status'] = 'failed'
        report['cases'].append({'id': 'harness', 'outcome': 'correctness_failure', 'detail': str(error)})
        raise
    finally:
        save()
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, capture_output=True)
        print(relative(report_path))


if __name__ == '__main__':
    main()
