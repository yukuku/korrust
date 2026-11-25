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
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/suspend_references'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    try:
        return str(Path(path).resolve().relative_to(ROOT))
    except ValueError:
        return str(Path(path).resolve())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-suspend-references'
    requested = args.output.resolve() if args.output else None
    report_output = requested if requested and requested.is_relative_to(ROOT / 'reports') else None
    directory = (ROOT / 'target/checks' / (report_output.name if report_output else stamp)) if report_output else (requested or ROOT / 'target/checks' / stamp)
    directory.mkdir(parents=True, exist_ok=False)
    evidence = (args.evidence_directory or report_output or ROOT / 'reports/correctness' / stamp).resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    (evidence / 'commands').mkdir()
    report_path = evidence.with_suffix('.json')
    java_home = Path(os.environ['JAVA_HOME'])
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    compiler = ROOT / 'references/kotlinc/lib/kotlin-compiler.jar'
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    source_paths = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/suspend_references.rs', Path(__file__).resolve(), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
    frozen_paths = [*source_paths, candidate, standard, compiler]
    frozen = {relative(path): sha(path) for path in frozen_paths}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in source_paths:
            archive.writestr(relative(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
                  compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
                  frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), evidence=dict(work_directory=relative(directory), logs=relative(evidence / 'commands')),
                  effective_options=dict(jvm_target='1.8', language_version='2.3', api_version='2.3'), cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--output', relative(directory), '--evidence-directory', relative(evidence)],
                  scope='Expected-type source suspend callable references, top-level and bound/unbound/interface member targets, receiver evaluation, ordinary function adaptation and delayed continuation resumption',
                  limitations=['Callable references require an explicit supported function expected type; inferred reflection types and reflection API are outside scope', 'Existing generic, extension, private member, default, vararg and inline suspend declaration boundaries remain', 'Existing bounded suspend state-machine and callback type restrictions remain'])
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']

    def save():
        report['counts'] = dict(Counter(item['outcome'] for item in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        command = [str(part) for part in command]
        result = subprocess.run(command, capture_output=True, text=True, timeout=360, cwd=ROOT)
        log = evidence / 'commands' / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        if result.returncode not in accepted:
            raise AssertionError(label + ': ' + result.stdout + result.stderr)
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
    official_out, candidate_out = directory / 'official', directory / 'candidate'
    run('official model', [official, FIXTURES / 'Models.kt', '-cp', standard, '-d', official_out, *options])
    run('candidate model', [candidate, FIXTURES / 'Models.kt', '-cp', standard, '-d', candidate_out])
    java_out = directory / 'java'
    run('Java caller compiled against official only', [java_home / 'bin/javac', '-cp', os.pathsep.join(map(str, [official_out, standard])), '-d', java_out, FIXTURES / 'Calls.java'])
    callers = {relative(path): sha(path) for path in java_out.rglob('*.class')}
    providers = [('official', official_out), ('candidate', candidate_out)]
    for group in ['top', 'members', 'failures']:
        def runtime(group=group):
            for provider, output in providers:
                result = run(provider + ' independent ' + group, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, standard])), 'suspendrefs.Calls', group])
                assert result.stdout == 'OK\n', result.stdout
            return 'Fixed Java caller compiled once against official; independent immediate, delayed, receiver and exception identity assertions with JVM verification'
        case('runtime_' + group, runtime)
    for compile_provider, compile_output in providers:
        def kotlin_consumer(compile_provider=compile_provider, compile_output=compile_output):
            consumer_out = directory / ('consumer-' + compile_provider)
            run('Kotlin consumer against ' + compile_provider, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [compile_output, standard])), '-d', consumer_out, *options])
            for runtime_provider, runtime_output in providers:
                result = run('Kotlin consumer ' + compile_provider + ' on ' + runtime_provider, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_output, standard])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin consumer resolves logical suspend return metadata from each provider, invokes returned reference, and resumes twice on both providers'
        case('kotlin_consumer_' + compile_provider, kotlin_consumer)
    def signatures():
        listings = []
        for provider, output in providers:
            listing = run(provider + ' factory signatures', [java_home / 'bin/javap', '-p', '-s', '-classpath', output, 'suspendrefs.ModelsKt']).stdout
            signatures = []
            selected = False
            for line in listing.splitlines():
                line = line.strip()
                if line.startswith(('public ', 'private ', 'protected ')):
                    selected = line.startswith('public static') and ' access$' not in line
                    if selected:
                        signatures.append(line)
                elif selected and line.startswith('descriptor:'):
                    signatures.append(line)
                    selected = False
            listings.append(signatures)
        assert listings[0] == listings[1], listings
        return 'Public factory generic declarations and erased JVM descriptors match official Kotlin'
    case('factory_signatures', signatures)
    invalid = [
        'suspend fun read(): String = "x"; fun broken(): () -> String = ::read',
        'suspend fun read(): String = "x"; fun broken(): String { val block: suspend () -> String = ::read; return block() }',
        'suspend fun read(value: Int): Int = value; fun broken(): suspend (String) -> Int = ::read',
        'suspend fun read(value: Int): Int = value; fun broken(): suspend () -> Int = ::read',
        'suspend fun read(): Int = 1; fun broken(): suspend () -> String = ::read',
        'class Reader { suspend fun read(): String = "x" }; fun broken(): suspend () -> String = Reader::read',
    ]
    for index, source in enumerate(invalid):
        def boundary(index=index, source=source):
            path = directory / f'invalid{index}.kt'
            path.write_text(source)
            run('oracle invalid ' + str(index), [official, path, '-cp', standard, '-d', directory / f'oracle-invalid-{index}', *options], accepted=(1,))
            result = run('candidate invalid ' + str(index), [candidate, path, '-cp', standard, '-d', directory / f'candidate-invalid-{index}', '--diagnostic-format', 'json'], accepted=(1,))
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            assert diagnostics and diagnostics[0]['category'] == 'invalid_kotlin', diagnostics
            return dict(outcome='invalid_rejected', diagnostics=diagnostics)
        case('invalid_' + str(index), boundary)
    assert callers == {relative(path): sha(path) for path in java_out.rglob('*.class')}
    assert frozen == {relative(path): sha(path) for path in frozen_paths}, 'Inputs changed during harness run'
    report['java_callers'] = callers
    report['status'] = 'fail' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'pass'
    save()
    print(json.dumps(dict(status=report['status'], counts=report['counts'], report=relative(report_path))))
    return int(report['status'] != 'pass')


if __name__ == '__main__':
    raise SystemExit(main())
