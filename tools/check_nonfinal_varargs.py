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
FIXTURES = ROOT / 'tests/nonfinal_varargs'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-compiled-nonfinal-varargs'
    directory = (args.output or ROOT / 'target/checks' / stamp).absolute()
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / stamp).absolute()
    directory.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/nonfinal_varargs.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
    frozen = {relative(path): sha(path) for path in [*inputs, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar']}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
        frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')),
        scope='Compiled Kotlin nonfinal varargs: empty arrays, named/default arguments, trailing lambdas, spread arguments, generic elements, member functions, constructors, extensions, fixed-arity overload priority and left-to-right evaluation order',
        limitations=['Source declarations still require final varargs; this suite tests ordinary calls to compiled Kotlin declarations', 'Defaults on the vararg parameter itself remain unsupported', 'Existing compiled inline expansion boundaries apply; no full MockK or application execution claim'],
        effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'),
        cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--output', relative(directory), '--evidence-directory', relative(evidence)])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command):
        command = [str(part) for part in command]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        assert result.returncode == 0, label + ': ' + result.stdout + result.stderr
        return result

    def case(name, operation):
        try:
            detail = operation()
            outcome = detail.pop('outcome', 'pass') if isinstance(detail, dict) else 'pass'
            report['cases'].append(dict(id=name, outcome=outcome, detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        save()

    save()
    dependency = directory / 'dependency'
    run('official dependency with metadata 1.7', [official, FIXTURES / 'Library.kt', '-d', dependency, '-Xmetadata-version=1.7.0', *options])
    cp = os.pathsep.join(map(str, [dependency, standard]))
    providers = [('official', directory / 'official'), ('candidate', directory / 'candidate')]
    for name, output in providers:
        def compile_consumer(name=name, output=output):
            run(name + ' consumer', [(official if name == 'official' else candidate), FIXTURES / 'Consumer.kt', '-cp', cp, '-d', output, *(options if name == 'official' else [])])
            return 'Same unmodified source, compiled dependency and standard library'
        case(name + '_compile', compile_consumer)
    def runtime():
        caller = directory / 'caller'
        run('fixed Java caller against official', [java / 'javac', '-cp', os.pathsep.join(map(str, [providers[0][1], dependency, standard])), '-d', caller, FIXTURES / 'Calls.java'])
        hashes = {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
        for name, output in providers:
            result = run(name + ' independent runtime assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [caller, output, dependency, standard])), 'nonfinal.consumer.Calls'])
            assert result.stdout == 'nonfinal-varargs-ok\n', result.stdout
        return dict(caller_sha256=hashes, assertions='Empty/default result 102; filled result 322 with order 456; named result 117 with order 8451; spread 218; generic element and empty generic inference; ordinary members and constructors return 6')
    case('fixed_java_caller_both_providers', runtime)
    def signatures():
        listings = []
        for name, output in providers:
            result = run(name + ' public JVM signatures', [java / 'javap', '-public', '-s', '-classpath', output, 'nonfinal.consumer.ConsumerKt'])
            listings.append(result.stdout)
        assert listings[0] == listings[1], listings
        return listings[0]
    case('public_jvm_signatures', signatures)
    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            results = []
            for name, compiler in [('official', official), ('candidate', candidate)]:
                command = [str(part) for part in [compiler, source, '-cp', cp, '-d', directory / (name + '-' + source.stem)]]
                result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
                log = evidence / f'{len(report["commands"]):03d}.txt'
                log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
                report['commands'].append(dict(label=name + ' ' + source.stem, argv=command, exit_code=result.returncode, log=relative(log)))
                save()
                assert result.returncode == 1, name + ': ' + result.stdout + result.stderr
                results.append(result.stderr)
            return dict(outcome='invalid_rejected', rejections=results)
        case(source.stem, invalid)
    def unsupported_default():
        source = FIXTURES / 'UnsupportedDefault.kt'
        run('official vararg default parameter boundary', [official, source, '-cp', cp, '-d', directory / 'official-unsupported-default', *options])
        command = [str(part) for part in [candidate, source, '-cp', cp, '-d', directory / 'candidate-unsupported-default', '--diagnostic-format', 'json']]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label='candidate vararg default parameter boundary', argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        assert result.returncode == 1, result.stdout + result.stderr
        diagnostic = json.loads(result.stderr)
        assert diagnostic['category'] == 'unsupported_language', diagnostic
        return dict(outcome='unsupported_language', diagnostic=diagnostic)
    case('vararg_parameter_default_boundary', unsupported_default)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
