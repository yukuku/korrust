#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/file_annotations'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-file-annotations')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-file-annotations')).resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    report_path = evidence.with_suffix('.json')
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    kotlinc = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    binary = work / 'korrust-frozen'
    shutil.copy2(args.compiler, binary)
    inputs = {str(path.relative_to(ROOT)): sha(path) for folder in ['src', 'tests/file_annotations'] for path in sorted((ROOT / folder).rglob('*')) if path.is_file()}
    inputs.update({str(Path(__file__).relative_to(ROOT)): sha(__file__), 'Cargo.toml': sha(ROOT / 'Cargo.toml'), 'Cargo.lock': sha(ROOT / 'Cargo.lock'), 'tests/file_annotations.rs': sha(ROOT / 'tests/file_annotations.rs')})
    app_paths = [ROOT / 'benchmarks/sources/androidbible' / name for name in ['Alkitab/src/main/java/yuku/alkitab/songs/newdoc/SongDocumentSerializers.kt', 'AlkitabYes2/src/test/java/yuku/alkitab/yes2/util/UnsignedBinarySearchKtTest.kt']]
    inputs.update({str(path.relative_to(ROOT)): sha(path) for path in app_paths})
    with zipfile.ZipFile(evidence / 'frozen-inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for name in inputs:
            archive.write(ROOT / name, name)
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': created.isoformat(), 'status': 'running', 'selection': 'Ordinary @file annotations: Kotlin FILE target, arguments, retention, source facade isolation, class-only files, fixed Java caller, official Kotlin consumers, signatures, invalid and unsupported boundaries. OptIn, Suppress and JVM file lowering remain unsupported; actual AndroidBible source checks record their first unsupported blocker, not compiled app coverage.', 'compiler': {'binary_sha256': sha(binary), 'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()}, 'effective_options': {'jvm_target': '1.8', 'language_version': '2.3', 'api_version': '2.3', 'module_name': 'file_annotations'}, 'reference_compiler': {'pin': json.loads((ROOT / 'references/compiler.json').read_text())}, 'inputs': {'files_sha256': inputs}, 'dependencies': {str(stdlib.relative_to(ROOT)): sha(stdlib)}, 'reproduction_commands': ['python3 tools/check_file_annotations.py --compiler ' + shlex.quote(str(args.compiler))], 'commands': [], 'cases': [], 'evidence': {'frozen_inputs': str((evidence / 'frozen-inputs.zip').relative_to(ROOT)), 'work_directory': str(work.relative_to(ROOT))}}

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def run(label, argv):
        index = len(report['commands'])
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=240)
        stdout = evidence / f'{index:04d}-stdout.txt'
        stderr = evidence / f'{index:04d}-stderr.txt'
        stdout.write_text(result.stdout)
        stderr.write_text(result.stderr)
        report['commands'].append({'id': label, 'argv': list(map(str, argv)), 'exit_code': result.returncode, 'stdout': str(stdout.relative_to(ROOT)), 'stderr': str(stderr.relative_to(ROOT))})
        print(label, result.returncode, result.stdout[:70].strip(), result.stderr[:100].strip(), flush=True)
        return result

    def case(label, passed, outcome='pass', **extra):
        report['cases'].append({'id': label, 'outcome': outcome if passed else 'correctness_failure', **extra})

    def compile(provider, sources, output, dependencies):
        output.mkdir(parents=True, exist_ok=True)
        common = [*sources, '-cp', cp(*dependencies, stdlib), '-d', output]
        if provider == 'official':
            return run(provider + '/' + output.name, [kotlinc, *common, '-module-name', 'file_annotations', '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8'])
        return run(provider + '/' + output.name, [binary, *common, '--module-name', 'file_annotations', '--diagnostic-format', 'json'])

    try:
        outputs = {}
        sources = [FIXTURES / name for name in ['Definitions.kt', 'Uses.kt', 'ClassFile.kt', 'Other.kt', 'SourceOnly.kt', 'Shadow.kt']]
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, sources, output, [])
            case(provider + '/compile', result.returncode == 0)
            if result.returncode == 0:
                outputs[provider] = output
        if 'official' not in outputs:
            return 1
        caller = work / 'caller'
        caller.mkdir()
        result = run('fixed-java-caller', [jdk / 'javac', '--release', '8', '-cp', cp(outputs['official'], stdlib), '-d', caller, FIXTURES / 'Check.java'])
        case('fixed-java-caller', result.returncode == 0)
        report['fixed_java_caller_hashes'] = {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
        signatures = {}
        for provider, output in outputs.items():
            result = run(provider + '/java-runtime', [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, stdlib), 'filecheck.Check'])
            case(provider + '/java-runtime', result.returncode == 0 and result.stdout.strip() == 'OK:42:hello:7', observed=result.stdout.strip())
            consumer = work / (provider + '-consumer')
            result = compile('official', [FIXTURES / 'Consumer.kt'], consumer, [output])
            case(provider + '/kotlin-consumer-compile', result.returncode == 0)
            if result.returncode == 0:
                result = run(provider + '/kotlin-consumer-run', [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, stdlib), 'fileconsumer.ConsumerKt'])
                case(provider + '/kotlin-consumer-run', result.returncode == 0 and result.stdout.strip() == 'OK:consumer')
            result = run(provider + '/facade-attributes', [jdk / 'javap', '-v', '-classpath', output, 'fileuses.UsesKt'])
            text = result.stdout
            visible = text.rsplit('RuntimeVisibleAnnotations:', 1)[-1]
            invisible = text.rsplit('RuntimeInvisibleAnnotations:', 1)[-1]
            case(provider + '/retention', result.returncode == 0 and 'filemarks.Visible(' in visible and 'filemarks.Binary(' in invisible and 'filemarks.Source' not in text)
            emitted = sorted(str(path.relative_to(output)) for path in output.rglob('*.class'))
            report[provider + '_classes'] = emitted
            case(provider + '/no-class-only-facade', 'fileuses/ClassFileKt.class' not in emitted)
            signatures[provider] = {}
            for owner in ['fileuses.UsesKt', 'fileuses.OtherKt', 'fileuses.SourceOnlyKt', 'fileuses.Neighbor', 'fileuses.Standalone', 'filemarks.Visible', 'filemarks.Binary', 'filemarks.Source']:
                result = run(provider + '/signature/' + owner, [jdk / 'javap', '-protected', '-s', '-classpath', output, owner])
                members = [line.strip() for line in result.stdout.splitlines() if line.strip() and not line.startswith('Compiled from')]
                signatures[provider][owner] = members
                case(provider + '/signature-read/' + owner, result.returncode == 0)
            imported = work / (provider + '-imported')
            result = compile(provider, [FIXTURES / 'Uses.kt', FIXTURES / 'ClassFile.kt', FIXTURES / 'Other.kt', FIXTURES / 'SourceOnly.kt'], imported, [outputs['official']])
            case(provider + '/imported-annotations-compile', result.returncode == 0)
            if result.returncode == 0:
                result = run(provider + '/imported-annotations-runtime', [jdk / 'java', '-Xverify:all', '-cp', cp(caller, imported, outputs['official'], stdlib), 'filecheck.Check'])
                case(provider + '/imported-annotations-runtime', result.returncode == 0 and result.stdout.strip() == 'OK:42:hello:7')
        report['signatures'] = signatures
        if 'candidate' in signatures:
            case('abi/signatures', signatures['official'] == signatures['candidate'])
            case('abi/emitted-class-set', report['official_classes'] == report['candidate_classes'])
        for row in json.loads((FIXTURES / 'cases.json').read_text()):
            source = FIXTURES / row['path']
            label = source.stem
            official = compile('official', [source], work / 'boundaries' / label / 'official', [outputs['official']])
            candidate_output = work / 'boundaries' / label / 'candidate'
            candidate = compile('candidate', [source], candidate_output, [outputs['official']])
            diagnostics = [json.loads(line) for line in candidate.stderr.splitlines() if line.startswith('{')]
            okay = (official.returncode == 0) == (row['oracle'] == 'accept') and candidate.returncode != 0 and diagnostics and all(d.get('category') == row['category'] and ('code' not in row or d.get('code') == row['code']) for d in diagnostics) and not list(candidate_output.rglob('*.class'))
            case('boundary/' + label, okay, outcome='unsupported_language' if row['category'] == 'unsupported_language' else 'invalid_rejected', diagnostics=diagnostics, official_accepted=official.returncode == 0, oracle_expected=row['oracle'], limitation=row.get('limitation'))
        for source in app_paths:
            output = work / 'app-boundaries' / source.stem
            candidate = compile('candidate', [source], output, [])
            diagnostics = [json.loads(line) for line in candidate.stderr.splitlines() if line.startswith('{')]
            okay = candidate.returncode != 0 and diagnostics and diagnostics[0].get('category') == 'unsupported_language' and not list(output.rglob('*.class'))
            case('app-boundary/' + source.stem, okay, outcome='unsupported_language', diagnostics=diagnostics, first_blocker=diagnostics[0].get('code') if diagnostics else None, oracle='not_run: app dependencies and additional language features outside this bounded harness')
        case('abi/fixed-java-caller-unchanged', report['fixed_java_caller_hashes'] == {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')})
    except Exception as error:
        case('harness-error', False, reason=str(error))
        raise
    finally:
        report['input_integrity'] = {'status': 'unchanged' if all(sha(ROOT / name) == digest for name, digest in inputs.items()) and sha(binary) == report['compiler']['binary_sha256'] and sha(stdlib) == report['dependencies'][str(stdlib.relative_to(ROOT))] else 'changed'}
        report['counts'] = dict(Counter(row['outcome'] for row in report['cases']))
        report['status'] = 'pass' if report['cases'] and all(row['outcome'] in {'pass', 'invalid_rejected', 'unsupported_language'} for row in report['cases']) and report['input_integrity']['status'] == 'unchanged' else 'failed'
        report_path.write_text(json.dumps(report, indent=2) + '\n')
        print(report_path, report['status'], report['counts'], flush=True)
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
