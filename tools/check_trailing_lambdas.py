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
FIXTURES = ROOT / 'tests/trailing_lambdas'


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
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-trailing-lambdas'
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
    app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/AudioHttpErrorBody.kt'
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/trailing_lambdas.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', app]
    frozen = {relative(path): sha(path) for path in [*inputs, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar']}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
        frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')),
        effective_options=dict(jvm_target='1.8', language_version='2.3', api_version='2.3'),
        scope='Outside-parentheses trailing lambdas mapped to the last parameter after omitted defaults and named arguments; ordinary source and compiled generic, nullable function, member, extension and overload resolution, labels, inline returns and evaluation order',
        limitations=['Existing bounded language and interop restrictions still apply', 'General nonfinal vararg parameters remain outside this slice', 'No full Android module/application or performance claim'],
        cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--output', relative(directory), '--evidence-directory', relative(evidence)])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        command = [str(part) for part in command]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        assert result.returncode in accepted, label + ': ' + result.stdout + result.stderr
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
    run('official generic providers', [official, FIXTURES / 'Dependency.kt', '-d', dependency, *options])
    cp = os.pathsep.join(map(str, [dependency, standard]))
    providers = [('official', directory / 'official'), ('candidate', directory / 'candidate')]

    def models():
        run('official semantic fixtures', [official, FIXTURES / 'Models.kt', '-cp', cp, '-d', providers[0][1], *options])
        run('candidate semantic fixtures', [candidate, FIXTURES / 'Models.kt', '-cp', cp, '-d', providers[1][1]])
        return 'Same unmodified fixtures, dependency and standard library'
    case('semantic_fixture_compile', models)

    def runtime():
        caller = directory / 'caller'
        run('fixed Java caller compiled against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [providers[0][1], dependency, standard])), '-d', caller, FIXTURES / 'Calls.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        for name, output in providers:
            result = run(name + ' independent assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [caller, output, dependency, standard])), 'trailing.Calls'])
            assert result.stdout == 'OK\n', result.stdout
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        return dict(caller_sha256=hashes, assertions='Source and compiled default dispatch, generic and nullable lambda inference, overloads, labels, nonlocal returns, reordered argument evaluation, Byte boxing/formatting and array initialization')
    case('fixed_java_caller_both_providers', runtime)

    def signatures():
        listings = []
        for name, output in providers:
            result = run(name + ' public JVM descriptors', [java / 'javap', '-public', '-s', '-v', '-classpath', output, 'trailing.ModelsKt'])
            lines = result.stdout.splitlines()
            listings.append([(lines[index - 1].strip(), line.strip()) for index, line in enumerate(lines) if line.strip().startswith('descriptor:') and 'ACC_SYNTHETIC' not in lines[index + 1]])
        assert listings[0] == listings[1], listings
        return listings[0]
    case('public_jvm_descriptors', signatures)

    for name, output in providers:
        def consumer(name=name, output=output):
            consumer_out = directory / ('consumer-' + name)
            run('official Kotlin consumer against ' + name, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [output, dependency, standard])), '-d', consumer_out, *options])
            for runtime_name, runtime_out in providers:
                result = run('consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_out, dependency, standard])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official consumers read emitted Kotlin metadata and run against both providers'
        case('kotlin_consumer_' + name, consumer)

    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            run('official invalid ' + source.stem, [official, source, '-cp', cp, '-d', directory / ('official-' + source.stem), *options], accepted=(1,))
            result = run('candidate invalid ' + source.stem, [candidate, source, '-cp', cp, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            return dict(outcome='invalid_rejected', diagnostic=result.stderr)
        case(source.stem, invalid)

    def application():
        run('official unchanged AudioHttpErrorBody', [official, app, '-d', directory / 'app-official', *options])
        result = run('candidate unchanged AudioHttpErrorBody', [candidate, app, '-cp', standard, '-d', directory / 'app-candidate', '--diagnostic-format', 'json'], accepted=(0, 1))
        if result.returncode:
            return dict(outcome='unsupported', diagnostic=result.stderr, reason='Next remaining blocker after trailing-lambda argument mapping')
        caller = directory / 'audio-caller'
        run('fixed Audio Java caller compiled against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [directory / 'app-official', standard])), '-d', caller, FIXTURES / 'AudioCheck.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        outputs = []
        signatures = []
        for provider in ['official', 'candidate']:
            result = run('verified unchanged Audio on ' + provider, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [caller, directory / ('app-' + provider), standard])), 'AudioCheck'])
            assert result.stdout == 'audio-checks=267\n', result.stdout
            outputs.append(result.stdout)
            listing = run('Audio public JVM descriptors ' + provider, [java / 'javap', '-public', '-s', '-v', '-classpath', directory / ('app-' + provider), 'yuku.alkitab.base.audio.AudioHttpErrorBodyKt']).stdout.splitlines()
            signatures.append([(listing[index - 1].strip(), line.strip()) for index, line in enumerate(listing) if line.strip().startswith('descriptor:') and 'ACC_SYNTHETIC' not in listing[index + 1]])
        assert signatures[0] == signatures[1], signatures
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        return dict(caller_sha256=hashes, stdout=outputs, assertions='267 independent cases: all 256 byte classifications, null/whitespace handling, line folding, 15/16/17 byte hex truncation, 1023/1024/1025 byte limits and input immutability')
    case('app/AudioHttpErrorBody', application)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
