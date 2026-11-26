#!/usr/bin/env python3
"""Frozen evidence for inferred typealias construction and JVM consumers."""
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
FIXTURES = ROOT / 'tests/typealias_constructors'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/debug/korrust')
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-typealias-constructor'
    work = ROOT / 'target/checks' / ident
    logs = ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True)
    logs.mkdir(parents=True)
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    def rel(path):
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
    inputs = [*sorted(ROOT.glob('src/**/*.rs')), *sorted(FIXTURES.glob('*')), ROOT / 'tests/typealias_constructors.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json']
    contents = {rel(path): path.read_bytes() for path in inputs}
    with zipfile.ZipFile(logs / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path, data in contents.items():
            archive.writestr(path, data)
    frozen = {path: hashlib.sha256(data).hexdigest() for path, data in contents.items()}
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), inputs=dict(files_sha256=frozen, bundle=rel(logs / 'inputs.zip'), bundle_sha256=sha(logs / 'inputs.zip'), stdlib_sha256=sha(standard), jdk_release_sha256=sha(jdk.parent / 'release'), jdk_module_sha256={path.name: sha(path) for path in sorted((jdk.parent / 'jmods').glob('*.jmod'))}), commands=[], cases=[], limitations=['Inferred construction through concrete Java and Kotlin generic class aliases, including reordered, fixed, repeated and nested arguments; nonstatic inner and SAM alias construction are outside this evidence', 'Constructor matching precedes expected result constraints; Java named arguments and constructor visibility remain checked', 'Aliases of classes with constructor-owned generic parameters remain explicitly unsupported; the pinned Kotlin compiler rejects the tested Java generic constructor alias calls although corresponding direct class calls are accepted, including distinct parameter names. Direct generic constructors remain covered separately.', 'No full Android app or benchmark claim'], reproduction_commands=[['python3', 'tools/check_typealias_constructors.py', '--candidate', str(args.candidate)]])
    def run(label, command, accepted=(0,)):
        command = list(map(str, command))
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, timeout=180)
        path = logs / f'{len(report["commands"]):03d}.txt'
        path.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=rel(path)))
        assert result.returncode in accepted, label + '\n' + result.stdout + result.stderr
        return result
    def case(ident, condition):
        report['cases'].append(dict(id=ident, outcome='pass' if condition else 'correctness_failure'))
        assert condition, ident
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    try:
        java = work / 'java'
        run('compile Java dependencies', [jdk / 'javac', '-d', java, FIXTURES / 'JavaTypes.java'])
        library = work / 'dependencies'
        options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
        run('compile official Kotlin dependencies', [kotlin, FIXTURES / 'Dependencies.kt', *options, '-cp', cp(java, standard), '-d', library])
        dependencies = cp(java, library, standard)
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, output in outputs.items():
            command = [kotlin, *options] if provider == 'official' else [candidate]
            run('compile ' + provider, [*command, FIXTURES / 'Constructors.kt', '-cp', dependencies, '-d', output])
            case('typealias-constructor/' + provider, True)
        callers = work / 'callers'
        run('fixed Java caller against official', [jdk / 'javac', '-cp', cp(outputs['official'], dependencies), '-d', callers, FIXTURES / 'Calls.java'])
        signatures = []
        consumers = {}
        for provider, output in outputs.items():
            result = run('verified fixed Java caller ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(callers, output, dependencies), 'aliases.consumer.Calls'])
            case('fixed-java/' + provider, result.stdout == 'typealias-constructor-ok\n')
            result = run('JVM signatures ' + provider, [jdk / 'javap', '-public', '-s', '-cp', output, 'aliases.consumer.ConstructorsKt'])
            signatures.append(result.stdout)
            consumers[provider] = work / (provider + '-consumer')
            run('official Kotlin metadata consumer ' + provider, [kotlin, FIXTURES / 'Consumer.kt', *options, '-cp', cp(output, dependencies), '-d', consumers[provider]])
        case('JVM-public-signatures', signatures[0] == signatures[1])
        runner = work / 'consumer-runner'
        run('fixed consumer runner against official', [jdk / 'javac', '-cp', cp(consumers['official'], standard), '-d', runner, FIXTURES / 'ConsumerCalls.java'])
        for provider, output in outputs.items():
            result = run('verified metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(runner, consumers[provider], output, dependencies), 'ConsumerCalls'])
            case('Kotlin-metadata-consumer/' + provider, result.stdout == 'consumer-ok\n')
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
            source = work / (item['id'] + '.kt')
            source.write_text(item['source'])
            for provider in outputs:
                command = [kotlin, *options] if provider == 'official' else [candidate, '--diagnostic-format', 'json']
                result = run('reject ' + provider + '/' + item['id'], [*command, source, '-cp', cp(outputs['official'], dependencies), '-d', work / ('invalid-' + provider)], (1,))
                if item.get('unsupported'):
                    if provider == 'candidate':
                        diagnostic = json.loads(result.stderr.strip().splitlines()[-1])
                        assert diagnostic['code'] == 'TYPE_ALIAS_GENERIC_CONSTRUCTOR_UNSUPPORTED', diagnostic
                    report['cases'].append(dict(id='boundary/' + provider + '/' + item['id'], outcome='unsupported_language' if provider == 'candidate' else 'oracle_rejection'))
                else:
                    case('invalid/' + provider + '/' + item['id'], result.returncode == 1)
        case('frozen-fixtures', all(sha(path) == frozen[rel(path)] for path in [*FIXTURES.glob('*'), ROOT / 'tests/typealias_constructors.rs', Path(__file__)]))
        report['status'] = 'passed'
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
        raise
    finally:
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        path = logs.with_suffix('.json')
        path.write_text(json.dumps(report, indent=2) + '\n')
        print(path)
        subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
