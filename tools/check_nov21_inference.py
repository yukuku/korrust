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
FIXTURES = ROOT / 'tests/nov21_inference'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--baseline', type=Path, default=Path('/workspace/korrust/target/next10-final-v3/korrust'))
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    parser.add_argument('--supersedes', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-nov21-inference'
    directory = (args.output or ROOT / 'target/checks' / stamp).absolute()
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / stamp).absolute()
    directory.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    (evidence / 'commands').mkdir()
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/QueryTokenizer.kt'
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/nov21_inference.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', app]
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
        supersedes=relative(args.supersedes) if args.supersedes else None,
        scope='Expected results through explicit return expressions, block and try/catch tails, contextual generic reference-array construction, canonical StringArray constraints, direct interparameter bounds, compiled extension result contexts and nullable functional lambda/reference contexts; imported metadata conditional contract facts for stable immutable locals and parameters',
        limitations=['No source contract DSL authoring', 'Contract facts apply to compiled static/top-level and extension calls; instance-member contracts are not applied', 'returns(null) facts are decoded but not applied; pinned compiler also rejects the tested external null-return implication consumer', 'No mutable/captured-cell contract smartcasts', 'No disjunction or negative-type implications', 'No calls-in-place analysis or returns-not-null/unconditional-return effect support', 'General expected-result propagation through when and Elvis remains outside this slice', 'Nullable Unit type-argument errors are currently swallowed by speculative call parsing; the direct reified Unit case has the structured classifier diagnostic', 'No full application build or performance claim'],
        evidence=dict(work_directory=relative(directory), logs=relative(evidence / 'commands')), cases=[], commands=[],
        reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--baseline', relative(args.baseline), '--output', relative(directory), '--evidence-directory', relative(evidence)])

    if args.supersedes:
        report['reproduction_command'].extend(['--supersedes', relative(args.supersedes)])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        command = [str(part) for part in command]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / 'commands' / f'{len(report["commands"]):03d}.txt'
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
    run('official arbitrary contract and generic providers', [official, FIXTURES / 'Dependency.kt', '-d', dependency, *options])
    cp = os.pathsep.join(map(str, [dependency, standard]))
    official_out, candidate_out = directory / 'official', directory / 'candidate'
    providers = [('official', official_out), ('candidate', candidate_out)]
    callers = {}

    def model():
        run('official semantic fixtures', [official, FIXTURES / 'Models.kt', '-cp', cp, '-d', official_out, *options])
        run('candidate semantic fixtures', [candidate, FIXTURES / 'Models.kt', '-cp', cp, '-d', candidate_out])
        return 'Matching sources and classpath; arbitrary metadata contract names and generic bounds, standard-library guard and contextual array elements'
    case('semantic_fixture_compile', model)

    def runtime():
        java_out = directory / 'java'
        run('fixed independent Java caller compiled against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [official_out, dependency, standard])), '-d', java_out, FIXTURES / 'Calls.java'])
        callers.update({relative(path): sha(path) for path in java_out.rglob('*.class')})
        for provider, output in providers:
            result = run(provider + ' independent runtime assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, dependency, standard])), 'inference.Calls'])
            assert result.stdout == 'OK\n', result.stdout
        assert callers == {relative(path): sha(path) for path in java_out.rglob('*.class')}
        return dict(caller_sha256=callers, assertions='Null/empty/UTF-16 guards, true/false/nullability polarity, overload selection, type implications, named conjunction arguments, return arrays, Java platform array elements, nullable generic bound joins, contextual compiled extensions and nullable function parameters with lambda/reference/null arguments')
    case('fixed_java_caller_both_providers', runtime)

    def signatures():
        listings = []
        for provider, output in providers:
            result = run(provider + ' JVM descriptors', [java / 'javap', '-s', '-p', '-classpath', output, 'inference.ModelsKt'])
            lines = result.stdout.splitlines()
            listings.append([(lines[index - 1].split('(')[0].split()[-1], line.strip()) for index, line in enumerate(lines)
                             if line.strip().startswith('descriptor:') and lines[index - 1].strip().startswith('public static')])
        assert listings[0] == listings[1], listings
        return listings[0]
    case('jvm_descriptors', signatures)

    for name, output in providers:
        def consumer(name=name, output=output):
            consumer_out = directory / ('consumer-' + name)
            run('official Kotlin metadata consumer against ' + name, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [output, dependency, standard])), '-d', consumer_out, *options])
            for runtime_name, runtime_out in providers:
                result = run('Kotlin consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_out, dependency, standard])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin consumer reads emitted ordinary function metadata; cross-run against both providers'
        case('kotlin_consumer_' + name, consumer)

    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            run('official invalid ' + source.stem, [official, source, '-cp', cp, '-d', directory / ('oracle-' + source.stem), *options], accepted=(1,))
            result = run('candidate invalid ' + source.stem, [candidate, source, '-cp', cp, '-d', directory / source.stem, '--diagnostic-format', 'json'], accepted=(1,))
            detail = dict(outcome='invalid_rejected', diagnostic=result.stderr)
            if source.stem == 'InvalidChain' and args.supersedes:
                previous = json.loads(args.supersedes.read_text())
                previous_binary = Path(previous['compiler']['path'])
                if not previous_binary.is_absolute():
                    previous_binary = ROOT / previous_binary
                assert sha(previous_binary) == previous['compiler']['sha256']
                run('historical chained-call wrong acceptance from superseded binary', [previous_binary, source, '-cp', cp, '-d', directory / 'historical-chain'], accepted=(0,))
                detail['historical_wrong_acceptance'] = dict(report=relative(args.supersedes), compiler_sha256=sha(previous_binary), reason='Nested receiver and outer source call share an offset; only the outer call may publish facts')
            return detail
        case(source.stem, invalid)

    for source in sorted(FIXTURES.glob('Unsupported*.kt')):
        def unsupported(source=source):
            run('official valid contract boundary ' + source.stem, [official, source, '-cp', cp, '-d', directory / ('official-' + source.stem), *options])
            if source.stem == 'UnsupportedTrailing':
                run('candidate trailing lambda after defaults', [candidate, source, '-cp', cp, '-d', directory / ('candidate-' + source.stem)])
                return dict(outcome='pass', reason='Outside-parentheses argument placement is retained and mapped to the last parameter')
            result = run('candidate contract boundary ' + source.stem, [candidate, source, '-cp', cp, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            reason = 'Reified Unit and nullable Unit classifier representation remains unsupported' if 'Reified' in source.stem else 'Contracts currently refine immutable local JVM slots and parameters only; captured values and mutable bindings are excluded'
            if source.stem == 'UnsupportedReifiedNullableUnit':
                reason = 'Nullable Unit is rejected by the existing parser, whose speculative generic-call path currently swallows its structured unsupported diagnostic'
            if source.stem == 'UnsupportedReifiedUnit':
                diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
                assert diagnostics and diagnostics[0]['category'] == 'unsupported_language' and diagnostics[0]['code'] == 'REIFIED_ARGUMENT_CLASSIFIER_UNSUPPORTED', diagnostics
            return dict(outcome='unsupported_language', reason=reason, diagnostic=result.stderr)
        case(source.stem, unsupported)

    def null_return_boundary():
        source = FIXTURES / 'NullReturn.kt'
        oracle = run('pinned null-return implication consumer boundary', [official, source, '-cp', cp, '-d', directory / 'official-null-return', *options], accepted=(1,))
        candidate_result = run('candidate deferred null-return implication consumer', [candidate, source, '-cp', cp, '-d', directory / 'candidate-null-return', '--diagnostic-format', 'json'], accepted=(1,))
        return dict(outcome='unsupported_language', reason='Metadata decoder retains returns(null), but flow application is deferred after pinned2.3.21 rejects this external smartcast consumer', reference_diagnostic=oracle.stderr, candidate_diagnostic=candidate_result.stderr)
    case('null_return_contract_boundary', null_return_boundary)

    for source in sorted(FIXTURES.glob('Baseline*.kt')):
        def baseline(source=source):
            assert args.baseline.is_file(), str(args.baseline)
            run('official isolated baseline feature ' + source.stem, [official, source, '-cp', cp, '-d', directory / ('official-' + source.stem), *options])
            result = run('pre-milestone baseline rejects ' + source.stem, [args.baseline, source, '-cp', cp, '-d', directory / source.stem, '--diagnostic-format', 'json'], accepted=(1,))
            return dict(outcome='baseline_rejected', sha256=sha(args.baseline), diagnostic=result.stderr)
        case(source.stem, baseline)

    def app_probe():
        run('official unchanged QueryTokenizer', [official, app, '-cp', standard, '-d', directory / 'official-query', *options])
        result = run('candidate unchanged QueryTokenizer', [candidate, app, '-cp', standard, '-d', directory / 'candidate-query', '--diagnostic-format', 'json'], accepted=(0, 1))
        if result.returncode:
            return dict(outcome='unsupported_language', source_sha256=sha(app), diagnostic=result.stderr, reason='Isolated worker records the next remaining implementation boundary; integrated app evidence is recorded separately')
        return dict(outcome='pass', source_sha256=sha(app), claim='Unchanged source compiles; no standalone app runtime claim in this harness')
    case('unchanged_query_tokenizer_compile', app_probe)

    changed = [path for path in [*inputs, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar'] if sha(path) != frozen[relative(path)]]
    assert not changed, 'Inputs changed during frozen run: ' + str(changed)
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    if not args.skip_html:
        subprocess.run(['python3', str(ROOT / 'tools/reports.py')], cwd=ROOT, check=True)
    print(report_path)
    return 1 if report['status'] == 'failed' else 0


if __name__ == '__main__':
    raise SystemExit(main())
