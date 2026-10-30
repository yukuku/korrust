#!/usr/bin/env python3
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
FIXTURES = ROOT / 'tests/fixtures/new_families/annotations17'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--core-only', action='store_true')
    parser.add_argument('--inject-jar', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-rich-annotations')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    lib = ROOT / 'references/kotlinc/lib'
    kotlinc = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = lib / 'kotlin-stdlib.jar'
    compilerlib = lib / 'kotlin-compiler.jar'
    reflect = lib / 'kotlin-reflect.jar'
    frozen_binary = work / 'korrust-frozen'
    shutil.copy2(args.compiler, frozen_binary)
    inputs = {str(p.relative_to(ROOT)): sha(p) for folder in ['src', 'tests/fixtures/new_families/annotations17'] for p in sorted((ROOT / folder).rglob('*')) if p.is_file()}
    inputs.update({str(Path(__file__).relative_to(ROOT)): sha(__file__), 'Cargo.toml': sha(ROOT / 'Cargo.toml'), 'Cargo.lock': sha(ROOT / 'Cargo.lock')})
    nia = ROOT / 'benchmarks/sources/nowinandroid/core/common/src/main/kotlin/com/google/samples/apps/nowinandroid/core/common/network/NiaDispatchers.kt'
    inputs[str(nia.relative_to(ROOT))] = sha(nia)
    inject = args.inject_jar or next((Path.home() / '.gradle/caches/modules-2/files-2.1/javax.inject/javax.inject/1').glob('*/javax.inject-1.jar'), None)
    if inject is None or not inject.is_file():
        raise RuntimeError('Real javax.inject-1.jar dependency is required; pass --inject-jar')
    with zipfile.ZipFile(work / 'frozen-inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for name in inputs:
            archive.write(ROOT / name, name)
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': created.isoformat(), 'status': 'running', 'selection': 'Bounded source annotation declarations, recursive JVM defaults and values, class mapping, retention and placement; fixed official Java callers, signatures, pinned metadata decoding, official Kotlin consumers, invalid versus unsupported cases. No compiler plugin, Android runtime, full-app or performance claim.', 'compiler': {'binary_sha256': sha(frozen_binary)}, 'reference_compiler': {'pin': json.loads((ROOT / 'references/compiler.json').read_text())}, 'inputs': {'files_sha256': inputs}, 'commands': [], 'cases': [], 'dependencies': {str(inject): sha(inject)}}

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def run(label, argv):
        index = len(report['commands'])
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=240)
        stdout = work / f'{index:04d}-stdout.txt'
        stderr = work / f'{index:04d}-stderr.txt'
        stdout.write_text(result.stdout)
        stderr.write_text(result.stderr)
        report['commands'].append({'id': label, 'argv': list(map(str, argv)), 'exit_code': result.returncode, 'stdout': str(stdout.relative_to(ROOT)), 'stderr': str(stderr.relative_to(ROOT))})
        print(label, result.returncode, result.stdout[:100].strip(), result.stderr[:150].strip(), flush=True)
        return result

    def case(label, passed, outcome='pass', **extra):
        report['cases'].append({'id': label, 'outcome': outcome if passed else 'correctness_failure', **extra})

    def compile_kotlin(provider, sources, output, dependencies):
        output.mkdir(parents=True, exist_ok=True)
        if provider == 'official':
            return run(provider + '/' + output.name, [kotlinc, *sources, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-classpath', cp(*dependencies, stdlib), '-d', output])
        return run(provider + '/' + output.name, [frozen_binary, *sources, '-cp', cp(*dependencies, stdlib), '--module-name', 'annotations17', '--diagnostic-format', 'json', '-d', output])

    try:
        java = work / 'java'; java.mkdir()
        java_result = run('java-library', [jdk / 'javac', '--release', '8', '-d', java, FIXTURES / 'JavaRich.java', FIXTURES / 'JavaNested.java', FIXTURES / 'JavaBoundClass.java'])
        case('setup/java', java_result.returncode == 0)
        providers = {}
        sources = [FIXTURES / name for name in ['Definitions.kt', 'Uses.kt', 'Shadow.kt', 'JavaUses.kt', 'Reads.kt']]
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile_kotlin(provider, sources, output, [java])
            case('provider/' + provider, result.returncode == 0)
            if result.returncode == 0:
                providers[provider] = output
        if 'official' not in providers:
            return
        caller = work / 'caller'; caller.mkdir()
        result = run('fixed-java-callers', [jdk / 'javac', '--release', '8', '-cp', cp(providers['official'], java, stdlib, compilerlib), '-d', caller, *[FIXTURES / name for name in ['ReflectionCheck.java', 'MetadataCheck.java', 'ReadCheck.java']]])
        case('callers/compile-official', result.returncode == 0)
        report['fixed_java_caller_hashes'] = {str(p.relative_to(caller)): sha(p) for p in caller.rglob('*.class')}
        for provider, output in providers.items():
            for name in ['ReflectionCheck', 'MetadataCheck', 'ReadCheck']:
                result = run(provider + '/' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, java, stdlib, compilerlib), 'annotation17check.' + name])
                case(provider + '/' + name, result.returncode == 0 and result.stdout.startswith('OK:'), observed=result.stdout.strip())
            consumer = work / (provider + '-consumer')
            result = compile_kotlin('official', [FIXTURES / 'KotlinConsumer.kt'], consumer, [output, java, reflect])
            case(provider + '/kotlin-consumer-compile', result.returncode == 0)
            if result.returncode == 0:
                result = run(provider + '/kotlin-consumer-run', [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, java, stdlib, reflect), 'annotation17consumer.KotlinConsumerKt'])
                case(provider + '/kotlin-consumer-run', result.returncode == 0 and result.stdout.strip() == 'OK:21')
            imported = work / (provider + '-imported-reads')
            result = compile_kotlin(provider, [FIXTURES / 'Reads.kt'], imported, [providers['official'], java])
            case(provider + '/imported-annotation-reads-compile', result.returncode == 0)
            if result.returncode == 0:
                result = run(provider + '/imported-annotation-reads-run', [jdk / 'java', '-Xverify:all', '-cp', cp(caller, imported, providers['official'], java, stdlib), 'annotation17check.ReadCheck'])
                case(provider + '/imported-annotation-reads-run', result.returncode == 0 and result.stdout.startswith('OK:'))
            signatures = {}
            for owner in ['annotation17.Rich', 'annotation17.Nested', 'annotation17.Meta', 'annotation17.Pack', 'annotation17.Placed', 'annotation17reads.ReadsKt']:
                result = run(provider + '/signature/' + owner, [jdk / 'javap', '-protected', '-s', '-classpath', cp(output, java), owner])
                members = []
                declaration = None
                for line in result.stdout.splitlines():
                    stripped = line.strip()
                    if stripped.startswith('descriptor:'):
                        members.append((declaration, stripped))
                        declaration = None
                    elif stripped and not line.startswith('Compiled from'):
                        declaration = stripped
                signatures[owner] = sorted(members)
                case(provider + '/signature-read/' + owner, result.returncode == 0)
            report[provider + '_signatures'] = signatures
        if 'candidate_signatures' in report:
            for owner, signature in report['official_signatures'].items():
                case('signature/' + owner, report['candidate_signatures'].get(owner) == signature)
        nia_outputs = {}
        for provider in ['official', 'candidate']:
            output = work / (provider + '-nia')
            result = compile_kotlin(provider, [nia], output, [inject])
            case('nia/' + provider + '/compile-unchanged', result.returncode == 0)
            if result.returncode == 0:
                nia_outputs[provider] = output
        if 'official' in nia_outputs:
            nia_caller = work / 'nia-caller'; nia_caller.mkdir()
            result = run('nia/fixed-java-caller', [jdk / 'javac', '--release', '8', '-cp', cp(nia_outputs['official'], inject, stdlib), '-d', nia_caller, FIXTURES / 'NiaCheck.java'])
            case('nia/fixed-java-caller', result.returncode == 0)
            report['nia_fixed_java_caller_hashes'] = {str(path.relative_to(nia_caller)): sha(path) for path in nia_caller.rglob('*.class')}
            for provider, output in nia_outputs.items():
                result = run('nia/' + provider + '/runtime', [jdk / 'java', '-Xverify:all', '-cp', cp(nia_caller, output, inject, stdlib), 'annotation17check.NiaCheck'])
                case('nia/' + provider + '/runtime', result.returncode == 0 and result.stdout.strip() == 'OK:4')
        report['negative_matrix_scope'] = 'not_run' if args.core_only else 'executed'
        for row in ([] if args.core_only else json.loads((FIXTURES / 'cases.json').read_text())):
            source = FIXTURES / row['path'].removeprefix('fixtures/')
            output = work / 'boundaries' / row['id']
            official = compile_kotlin('official', [source], output / 'official', [providers['official'], java])
            candidate = compile_kotlin('candidate', [source], output / 'candidate', [providers['official'], java])
            expected = row['expected_oracle'] == 'accept'
            if (official.returncode == 0) != expected:
                case(row['id'], False, reason='Authored oracle expectation differs', oracle_expected=row['expected_oracle'])
                continue
            if expected:
                diagnostics = [json.loads(line) for line in candidate.stderr.splitlines() if line.startswith('{')]
                okay = candidate.returncode != 0 and diagnostics and all(d.get('category') == 'unsupported_language' for d in diagnostics)
                case(row['id'], okay, outcome='unsupported_language', diagnostics=diagnostics)
            else:
                diagnostics = [json.loads(line) for line in candidate.stderr.splitlines() if line.startswith('{')]
                case(row['id'], candidate.returncode != 0 and diagnostics and all(d.get('category') == 'invalid_kotlin' for d in diagnostics) and not list((output / 'candidate').rglob('*.class')), outcome='invalid_rejected', diagnostics=diagnostics)
        for row in ([] if args.core_only else json.loads((FIXTURES / 'extra-cases.json').read_text())):
            source = FIXTURES / 'extra' / (row['id'] + '.kt')
            output = work / 'extras' / row['id']
            official = compile_kotlin('official', [source], output / 'official', [providers['official'], java])
            candidate = compile_kotlin('candidate', [source], output / 'candidate', [providers['official'], java])
            expected = row['oracle'] == 'accept'
            case('extra/' + row['id'], (official.returncode == 0) == expected and (candidate.returncode == 0) == expected, outcome='pass' if expected else 'invalid_rejected')
    finally:
        report['input_integrity'] = {'status': 'unchanged' if all(sha(ROOT / name) == digest for name, digest in inputs.items()) and sha(inject) == report['dependencies'][str(inject)] else 'changed'}
        report['counts'] = dict(Counter(row['outcome'] for row in report['cases']))
        report['status'] = 'pass' if report['cases'] and all(row['outcome'] in {'pass', 'invalid_rejected', 'unsupported_language'} for row in report['cases']) and report['input_integrity']['status'] == 'unchanged' else 'failed'
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work / 'report.json', report['status'], report['counts'], flush=True)
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    raise SystemExit(main())
