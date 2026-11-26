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
FIXTURES = ROOT / 'tests/static_wildcard'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-java-static-wildcard-imports'
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
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/static_wildcard.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
    frozen = {relative(path): sha(path) for path in [*inputs, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', *([args.baseline] if args.baseline else [])]}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
        frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')),
        effective_options=dict(jvm_target='1.8', language_version='2.3', api_version='2.3'),
        scope='Java class wildcard imports: merged overload candidates, explicit/renamed/local precedence, deduplication, visibility, nested constructor values, inherited static members through accessible qualifiers, mutable fields and callable references',
        limitations=['Wildcard owners are bounded to supported public Java classes; Kotlin classes/objects and Java interfaces remain explicitly unsupported', 'No full application or performance claim'],
        cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--output', relative(directory), '--evidence-directory', relative(evidence)])

    if args.baseline:
        report['reproduction_command'].extend(['--baseline', str(args.baseline)])

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
    run('Java static providers', [java / 'javac', '-d', dependency, *[FIXTURES / name for name in ['First.java', 'Second.java', 'Derived.java', 'DerivedAgain.java']]])
    cp = os.pathsep.join(map(str, [dependency, standard]))
    providers = [('official', directory / 'official'), ('candidate', directory / 'candidate')]

    models_sources = [FIXTURES / name for name in ['Values.kt', 'Overloads.kt', 'Explicit.kt', 'Alias.kt', 'Duplicate.kt', 'Inherited.kt', 'InheritedDuplicate.kt', 'SourcePriority.kt']]

    def models():
        run('official semantic fixtures', [official, *models_sources, '-cp', cp, '-d', providers[0][1], *options])
        run('candidate semantic fixtures', [candidate, *models_sources, '-cp', cp, '-d', providers[1][1]])
        return 'Same unmodified fixtures, dependency and standard library'
    case('static_wildcard_fixture_compile', models)

    def runtime():
        caller = directory / 'caller'
        run('fixed Java caller compiled against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [providers[0][1], dependency, standard])), '-d', caller, FIXTURES / 'Calls.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        for name, output in providers:
            result = run(name + ' independent assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [caller, output, dependency, standard])), 'star.consumer.Calls'])
            assert result.stdout == 'static-wildcard-ok\n', result.stdout
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        return dict(caller_sha256=hashes, assertions='Overloaded static calls, mutable field reads/writes, nested construction, inherited methods/fields, explicit/renamed/source priority, duplicate imports and callable capture/reference results')
    case('fixed_java_caller_both_providers', runtime)

    def signatures():
        listings = []
        classes = ['star.consumer.' + name + 'Kt' for name in ['Values', 'Overloads', 'Explicit', 'Alias', 'Duplicate', 'Inherited', 'InheritedDuplicate']] + ['star.local.SourcePriorityKt']
        for name, output in providers:
            declarations = []
            for owner in classes:
                result = run(name + ' public JVM descriptors ' + owner, [java / 'javap', '-public', '-s', '-v', '-classpath', output, owner])
                lines = result.stdout.splitlines()
                declarations.extend((owner, lines[index - 1].strip(), line.strip()) for index, line in enumerate(lines) if line.strip().startswith('descriptor:') and 'ACC_SYNTHETIC' not in lines[index + 1])
            listings.append(sorted(declarations))
        assert listings[0] == listings[1], listings
        return listings[0]
    case('public_jvm_descriptors', signatures)

    for name, output in providers:
        def consumer(name=name, output=output):
            consumer_out = directory / ('consumer-' + name)
            run('official Kotlin consumer against ' + name, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [output, dependency, standard])), '-d', consumer_out, *options])
            for runtime_name, runtime_out in providers:
                result = run('consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_out, dependency, standard])), 'star.reader.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official consumers read emitted Kotlin metadata and run against both providers'
        case('kotlin_consumer_' + name, consumer)

    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            run('official invalid ' + source.stem, [official, source, '-cp', cp, '-d', directory / ('official-' + source.stem), *options], accepted=(1,))
            result = run('candidate invalid ' + source.stem, [candidate, source, '-cp', cp, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            diagnostic = json.loads(result.stderr)
            assert diagnostic['category'] == 'invalid_kotlin', diagnostic
            return dict(outcome='invalid_rejected', diagnostic=diagnostic)
        case(source.stem, invalid)

    if args.baseline:
        def baseline():
            result = run('baseline valid wildcard imports fail resolution', [args.baseline, *models_sources, '-cp', cp, '-d', directory / 'baseline'], accepted=(1,))
            return dict(outcome='baseline_resolution_failure', diagnostic=result.stderr, compiler_sha256=sha(args.baseline))
        case('baseline_valid_import_resolution_failure', baseline)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
