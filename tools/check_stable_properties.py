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

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/stable_properties'

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    stamp = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-stable-properties'
    work = ROOT / 'target/checks' / stamp
    evidence = ROOT / 'reports/correctness' / stamp
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/stable_properties.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', candidate, stdlib]
    frozen = {relative(path): sha(path) for path in inputs}
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=dict(version='2.3.21', artifact_sha256=sha(ROOT / 'references/kotlinc/lib/kotlin-compiler.jar')),
        frozen_input_hashes=frozen, effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'),
        scope='Compiled Kotlin final default val property smartcasts after null guards, with stable immutable local receiver identity and chains, constrained to explicit friend dependencies',
        limitations=['Source property headers do not yet expose stability', 'Mutable local receivers and captured receivers remain unsupported', 'No open, delegated, custom getter, mutable property, extension property or ordinary external module smartcasts', 'No application build or performance claim'],
        reproduction_command=['python3', 'tools/check_stable_properties.py', '--candidate', relative(args.candidate)], commands=[], cases=[])
    if args.baseline:
        report['reproduction_command'].extend(['--baseline', relative(args.baseline)])
    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')
    def run(label, command, accepted=(0,)):
        command = [str(part) for part in command]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.log'
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
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    dependency = work / 'dependency'
    run('real compiled Kotlin dependency', [official, FIXTURES / 'Dependency.kt', '-d', dependency, *options])
    cp = os.pathsep.join(map(str, [dependency, stdlib]))
    outputs = dict(official=work / 'official', candidate=work / 'candidate')
    def compile_models():
        run('official matching friend compilation', [official, FIXTURES / 'Models.kt', '-cp', cp, '-Xfriend-paths=' + str(dependency), '-d', outputs['official'], *options])
        run('candidate matching friend compilation', [candidate, FIXTURES / 'Models.kt', '-cp', cp, '--friend-paths', dependency, '-d', outputs['candidate']])
        return 'Exact source and dependencies, matching explicit friend origin'
    case('guarded_property_compile', compile_models)
    def runtime():
        callers = work / 'callers'
        run('fixed Java assertions against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs['official'], dependency, stdlib])), '-d', callers, FIXTURES / 'Calls.java'])
        hashes = {relative(path): sha(path) for path in callers.rglob('*.class')}
        for name, output in outputs.items():
            result = run(name + ' fixed caller JVM verification', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, dependency, stdlib])), 'stable.Calls'])
            assert result.stdout == 'OK\n', result.stdout
        assert hashes == {relative(path): sha(path) for path in callers.rglob('*.class')}
        return dict(caller_sha256=hashes, assertions='Null/nonnull UTF-16 lengths, reverse/null polarity, early-return control flow, conjunction, chained getters, distinct and shadowed receivers')
    case('fixed_java_caller_both_providers', runtime)
    def signatures():
        listings = [run(name + ' JVM signatures', [java / 'javap', '-s', '-protected', '-classpath', output, 'stable.ModelsKt']).stdout for name, output in outputs.items()]
        assert listings[0] == listings[1], listings
        return listings[0]
    case('jvm_signatures', signatures)
    for provider, output in outputs.items():
        def consumer(provider=provider, output=output):
            consumer_out = work / ('consumer-' + provider)
            run('official Kotlin metadata consumer against ' + provider, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [output, dependency, stdlib])), '-d', consumer_out, *options])
            for runtime_provider, runtime_out in outputs.items():
                result = run('Kotlin consumer ' + provider + ' on ' + runtime_provider, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_out, dependency, stdlib])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin consumer reads emitted ordinary function metadata and cross-runs against both providers'
        case('kotlin_consumer_' + provider, consumer)
    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            run('official rejects ' + source.stem, [official, source, '-cp', cp, '-Xfriend-paths=' + str(dependency), '-d', work / ('official-' + source.stem), *options], accepted=(1,))
            result = run('candidate rejects ' + source.stem, [candidate, source, '-cp', cp, '--friend-paths', dependency, '-d', work / ('candidate-' + source.stem)], accepted=(1,))
            return dict(outcome='invalid_rejected', diagnostic=result.stderr)
        case(source.stem, invalid)
    def cross_module():
        source = FIXTURES / 'CrossModule.kt'
        run('official rejects external public property smartcast', [official, source, '-cp', cp, '-d', work / 'external-official', *options], accepted=(1,))
        run('candidate rejects external public property smartcast', [candidate, source, '-cp', cp, '-d', work / 'external-candidate'], accepted=(1,))
        return dict(outcome='invalid_rejected', reason='No explicit friend origin')
    case('ordinary_external_module', cross_module)
    if args.baseline:
        def baseline():
            result = run('pre-feature compiler rejects valid friend smartcasts', [args.baseline, FIXTURES / 'Models.kt', '-cp', cp, '--friend-paths', dependency, '-d', work / 'baseline'], accepted=(1,))
            return dict(outcome='baseline_rejected', compiler_sha256=sha(args.baseline), diagnostic=result.stderr)
        case('baseline_missing_feature', baseline)
    assert all(sha(path) == frozen[relative(path)] for path in inputs), 'Frozen inputs changed'
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    if not args.skip_html:
        subprocess.run(['python3', str(ROOT / 'tools/reports.py')], cwd=ROOT, check=True)
    print(report_path)
    return int(report['status'] == 'failed')

if __name__ == '__main__':
    raise SystemExit(main())
