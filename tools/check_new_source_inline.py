#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / 'tests/fixtures/new_families/source_inline19'
UPSTREAM = [
    'classLiteral/primitiveClassEquality.kt',
    'reified/instanceof.kt',
    'reified/checkcast.kt',
    'reified/safecast.kt',
]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    output = (args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-source-inline')).resolve()
    output.mkdir(parents=True, exist_ok=False)
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', cases=[], commands=[], limits='Bounded static source inline bodies, real JVM reification, parameter modes, direct lambda expansion and caller-side cleanup. Source body exception tables, captured/generated inline closures, receivers, defaults, varargs, complex reified bounds/classifiers remain explicit boundaries. Scalar Short is a prerequisite; ShortArray remains unsupported. No Android/full app or speed claim.')
    reference = ROOT / json.loads((ROOT / 'references/compiler.json').read_text())['path']
    stdlib = reference / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    compiler = output / 'korrust-frozen'
    shutil.copy2(args.compiler.resolve(), compiler)
    frozen = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', *sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.rglob('*')), Path(__file__).resolve(), ROOT / 'tools/upstream_jvm.py', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', ROOT / 'tests/batch_frontend.rs']
    frozen += [ROOT / 'references/kotlin/compiler/testData/codegen/box' / name for name in UPSTREAM]
    frozen = [path for path in frozen if path.is_file()]
    hashes = {str(path.relative_to(ROOT)): sha(path) for path in frozen}
    report['resource_limits'] = dict(source_inline_dependency_depth=64, command_seconds=180)
    report['inputs'] = dict(files_sha256=hashes, compiler_sha256=sha(compiler), stdlib_sha256=sha(stdlib), compiler_jar_sha256=sha(reference / 'lib/kotlin-compiler.jar'))
    with zipfile.ZipFile(output / 'source-only.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in frozen:
            archive.write(path, path.relative_to(ROOT))
    commands = output / 'commands'
    commands.mkdir()

    def run(label, command, success=True):
        index = len(report['commands'])
        completed = subprocess.run([str(value) for value in command], cwd=ROOT, text=True, capture_output=True, timeout=180)
        log = commands / f'{index:03d}.txt'
        log.write_text(completed.stdout + completed.stderr)
        report['commands'].append(dict(label=label, argv=[str(value) for value in command], exit_code=completed.returncode, log=str(log.relative_to(output))))
        if success and completed.returncode:
            raise RuntimeError(label + ': ' + (completed.stdout + completed.stderr)[-2000:])
        return completed

    def case(name, outcome, **evidence):
        report['cases'].append(dict(id=name, outcome=outcome, **evidence))

    providers = [FIXTURES / name for name in ['Reified.kt', 'Effects.kt', 'Forward.kt', 'Shorts.kt', 'Ordering.kt']]
    try:
        official = output / 'official'
        candidate = output / 'candidate'
        for name, destination, command in [('official', official, [reference / 'bin/kotlinc']), ('candidate', candidate, [compiler])]:
            run(name + ' provider', [*command, *providers, '-cp', stdlib, '-d', destination])
            case(name + '_provider', 'pass')
        java = output / 'java'
        run('Fixed Java caller compiled against official', [jdk / 'javac', '-cp', str(official) + os.pathsep + str(stdlib), '-d', java, FIXTURES / 'JavaCheck.java'])
        outputs = []
        for name, destination in [('official', official), ('candidate', candidate)]:
            result = run(name + ' Java verification', [jdk / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [java, destination, stdlib])), 'verify.JavaCheck'])
            outputs.append(result.stdout)
            case(name + '_java_runtime', 'pass', assertions=result.stdout.strip())
            consumer = output / (name + '-consumer')
            run(name + ' independent Kotlin consumer compilation', [reference / 'bin/kotlinc', FIXTURES / 'KotlinConsumer.kt', '-cp', destination, '-d', consumer])
            result = run(name + ' independent Kotlin consumer runtime', [jdk / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer, destination, stdlib])), 'consumer.KotlinConsumerKt'])
            if result.stdout != 'checks=14\n':
                raise RuntimeError('Unexpected Kotlin assertion count')
            case(name + '_kotlin_consumer', 'pass', assertions=14)
        if outputs[0] != outputs[1]:
            raise RuntimeError('Java outputs differ')
        for classifier in ['probe.ReifiedKt', 'lambdas.EffectsKt', 'forward.ForwardKt', 'shorts.ShortsKt', 'ordering.OrderingKt']:
            signatures = []
            for name, destination in [('official', official), ('candidate', candidate)]:
                result = run(name + ' signatures ' + classifier, [jdk / 'javap', '-v', '-s', '-public', '-cp', destination, classifier])
                lines = result.stdout.splitlines()
                signatures.append(sorted((line.strip(), lines[index+1].strip()) for index, line in enumerate(lines[:-1]) if line.strip().endswith(';') and '(' in line and lines[index+1].strip().startswith('descriptor:') and not (' access$' in line and 'ACC_SYNTHETIC' in lines[index+2])))
            if signatures[0] != signatures[1]:
                raise RuntimeError('Signature mismatch ' + classifier + ': ' + repr(signatures))
            case('signatures_' + classifier, 'pass')
        inline_names = ['twice', 'test', 'cast', 'safe', 'token', 'physical', 'apply', 'runCross', 'returnNo', 'genericApply', 'privateTwice', 'publicProperty', 'setPublicProperty', 'publicOverload', 'relay', 'outer', 'inner', 'combine', 'divide', 'fail']
        for classifier in ['probe.ReifiedKt', 'lambdas.EffectsKt', 'forward.ForwardKt', 'ordering.OrderingKt']:
            result = run('Actual callsite expansion ' + classifier, [jdk / 'javap', '-c', '-p', '-cp', candidate, classifier])
            for line in result.stdout.splitlines():
                if re.search(r'\binvokestatic\b', line) and any(re.search(r'(?:[./]|Method )' + name + r':', line) for name in inline_names):
                    raise RuntimeError('Ordinary source inline invocation remains: ' + line)
            case('expanded_' + classifier, 'pass')
        for folder, expected in [('invalid', 'invalid_kotlin'), ('boundary', 'unsupported_language')]:
            for source in sorted((FIXTURES / folder).glob('*.kt')):
                name = folder + '_' + source.stem
                run(name + ' official', [reference / 'bin/kotlinc', source, '-cp', stdlib, '-d', output / (name + '-official')], success=False)
                oracle = report['commands'][-1]['exit_code']
                if (folder == 'invalid') != (oracle != 0):
                    raise RuntimeError(name + ': oracle classification differs')
                destination = output / (name + '-candidate')
                result = run(name + ' candidate', [compiler, source, '-cp', stdlib, '-d', destination, '--diagnostic-format', 'json'], success=False)
                diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
                if result.returncode == 0 or not diagnostics or diagnostics[0]['category'] != expected or list(destination.rglob('*.class')):
                    raise RuntimeError(name + ': candidate classification differs: ' + result.stdout + result.stderr)
                case(name, 'invalid_rejected' if folder == 'invalid' else 'unsupported_language', diagnostics=diagnostics)
        matrix = dict(schema_version=1, selection='Unchanged pinned source inline/reified upstream originals', limitations=[], cases=[])
        for index, path in enumerate(UPSTREAM):
            source = ROOT / 'references/kotlin/compiler/testData/codegen/box' / path
            matrix['cases'].append(dict(id='upstream_' + str(index), family='source_inline_reified', root='box', path=str(source.relative_to(ROOT)), tier='acceptance', kind='box', sha256=sha(source)))
        matrix_path = output / 'upstream-matrix.json'
        matrix_path.write_text(json.dumps(matrix, indent=2) + '\n')
        run('Unchanged upstream originals', [sys.executable, ROOT / 'tools/upstream_jvm.py', '--matrix', matrix_path, '--candidate', compiler, '--output', output / 'upstream'])
        upstream = json.loads((output / 'upstream/report.json').read_text())
        for item in upstream['cases']:
            case(item['id'], item['outcome'], source=item['source'], report='upstream/report.json')
        if any(item['outcome'] != 'pass' for item in upstream['cases']):
            raise RuntimeError('An upstream acceptance original did not pass')
        report['status'] = 'pass'
    except (RuntimeError, subprocess.TimeoutExpired, OSError) as error:
        report['status'] = 'fail'
        report['failure'] = str(error)
    finally:
        changed = [path for path, digest in hashes.items() if sha(ROOT / path) != digest]
        report['input_integrity'] = dict(status='changed' if changed else 'unchanged', changed=changed)
        if changed:
            report['status'] = 'fail'
        report['counts'] = dict(Counter(item['outcome'] for item in report['cases']))
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(output / 'report.json')
        print(json.dumps({key: report.get(key) for key in ['status', 'counts', 'failure', 'input_integrity']}))
    return 0 if report['status'] == 'pass' else 1


if __name__ == '__main__':
    sys.exit(main())
