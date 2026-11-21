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
FIXTURES = ROOT / 'tests/enum_field_annotations'

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
    parser.add_argument('--supersedes', type=Path)
    parser.add_argument('--historical', type=Path)
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    stamp = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-enum-field-annotations'
    work = ROOT / 'target/checks' / stamp
    evidence = ROOT / 'reports/correctness' / stamp
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *[p for p in sorted(FIXTURES.iterdir()) if p.is_file()], ROOT / 'tests/enum_field_annotations.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', candidate, stdlib]
    frozen = {relative(path): sha(path) for path in inputs}
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=dict(version='2.3.21', artifact_sha256=sha(ROOT / 'references/kotlinc/lib/kotlin-compiler.jar')),
        frozen_input_hashes=frozen, effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'),
        supersedes=relative(args.supersedes) if args.supersedes else None,
        scope='Passive declaration annotations with payloads on imported enum constant fields, retaining actual runtime field reads and annotation data',
        limitations=['Exact public/static/final enum constants with self-typed descriptors only', 'No recovery for ordinary annotated fields, field signatures, type-use annotations, synthetic fields, nullability/type-qualifier annotations or Kotlin semantic annotations and opt-in meta-annotations', 'Passive annotation inspection checks structural class metadata and qualifier meta-annotations without requiring annotation implementation Kotlin metadata to be at the compiler pin', 'Pinned 2.3.21 accepts the tested HIDDEN enum entry from another module; candidate conservatively rejects Kotlin semantic field annotations', 'No application build or performance claim'],
        reproduction_command=['python3', 'tools/check_enum_field_annotations.py', '--candidate', relative(args.candidate)], commands=[], cases=[])
    if args.historical:
        report['reproduction_command'].extend(['--historical', relative(args.historical)])
    if args.supersedes:
        report['reproduction_command'].extend(['--supersedes', relative(args.supersedes)])
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
    run('real compiled Kotlin dependency', [official, FIXTURES / 'Dependency.kt', '-Xemit-jvm-type-annotations', '-d', dependency, *options])
    annotations = ROOT / 'references/kotlinc/lib/annotations-13.0.jar'
    run('real compiled Java enum type annotations', [java / 'javac', '-cp', annotations, '-d', dependency, FIXTURES / 'JavaMode.java'])
    cp = os.pathsep.join(map(str, [dependency, stdlib, annotations]))
    outputs = dict(official=work / 'official', candidate=work / 'candidate')
    def compile_models():
        run('official enum consumers', [official, FIXTURES / 'Models.kt', '-cp', cp, '-d', outputs['official'], *options])
        run('candidate enum consumers', [candidate, FIXTURES / 'Models.kt', '-cp', cp, '-d', outputs['candidate']])
        return 'Exact source, actual pinned Kotlin/Java providers, matching dependency classpath'
    case('enum_field_compile', compile_models)
    def runtime():
        callers = work / 'callers'
        run('fixed Java assertions against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs['official'], dependency, stdlib])), '-d', callers, FIXTURES / 'Calls.java'])
        hashes = {relative(path): sha(path) for path in callers.rglob('*.class')}
        for name, output in outputs.items():
            result = run(name + ' fixed caller JVM verification', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, dependency, stdlib])), 'fieldmeta.Calls'])
            assert result.stdout == 'OK\n', result.stdout
        assert hashes == {relative(path): sha(path) for path in callers.rglob('*.class')}
        return dict(caller_sha256=hashes, assertions='Exact enum field identities, exhaustive when routing, runtime annotation scalar/array payloads, binary plus runtime annotations')
    case('fixed_java_caller_both_providers', runtime)
    def signatures():
        listings = [run(name + ' JVM signatures', [java / 'javap', '-s', '-protected', '-classpath', output, 'fieldmeta.ModelsKt']).stdout for name, output in outputs.items()]
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
            run('official rejects ' + source.stem, [official, source, '-cp', cp, '-d', work / ('official-' + source.stem), *options], accepted=(1,))
            result = run('candidate rejects ' + source.stem, [candidate, source, '-cp', cp, '-d', work / ('candidate-' + source.stem)], accepted=(1,))
            return dict(outcome='invalid_rejected', diagnostic=result.stderr)
        case(source.stem, invalid)
    for source in sorted(FIXTURES.glob('Unsupported*.kt')):
        def unsupported(source=source):
            run('official valid boundary ' + source.stem, [official, source, '-cp', cp, '-d', work / ('official-' + source.stem), *options])
            result = run('candidate bounded rejection ' + source.stem, [candidate, source, '-cp', cp, '-d', work / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            if source.stem != 'UnsupportedOrdinary':
                assert 'ENUM_CONSTANT_METADATA_UNSUPPORTED' in result.stderr, result.stderr
            return dict(outcome='unsupported_language', diagnostic=result.stderr)
        case(source.stem, unsupported)
    import importlib.util
    spec = importlib.util.spec_from_file_location('field_mutations', FIXTURES / 'mutate.py')
    mutations = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mutations)
    for mode in ['synthetic', 'type_use', 'duplicate_annotation_table', 'malformed_signature']:
        def mutation(mode=mode):
            patched = work / ('dependency-' + mode)
            shutil.copytree(dependency, patched)
            classfile = patched / 'fieldmeta/dependency/Choice.class'
            classfile.write_bytes(mutations.mutate(classfile.read_bytes(), mode))
            digest = sha(classfile)
            patch_cp = os.pathsep.join(map(str, [patched, stdlib, annotations]))
            oracle = run('official patched class observation ' + mode, [official, FIXTURES / 'PatchRead.kt', '-cp', patch_cp, '-d', work / ('oracle-' + mode), *options], accepted=(0, 1, 2))
            result = run('candidate rejects patched metadata ' + mode, [candidate, FIXTURES / 'PatchRead.kt', '-cp', patch_cp, '-d', work / ('candidate-' + mode), '--diagnostic-format', 'json'], accepted=(1,))
            assert digest == sha(classfile)
            return dict(outcome='unsupported_dependency' if mode in ['synthetic', 'type_use'] else 'malformed_dependency_rejected', classfile_sha256=digest, reference_exit_code=oracle.returncode, reference_diagnostic=oracle.stderr, diagnostic=result.stderr)
        case('patched_' + mode, mutation)
    if args.baseline:
        def baseline():
            result = run('pre-feature compiler rejects annotated enum constants', [args.baseline, FIXTURES / 'Models.kt', '-cp', cp, '-d', work / 'baseline'], accepted=(1,))
            return dict(outcome='baseline_rejected', compiler_sha256=sha(args.baseline), diagnostic=result.stderr)
        case('baseline_missing_feature', baseline)
    if args.historical:
        def historical():
            result = run('historical wrong acceptance of opt-in marker', [args.historical, FIXTURES / 'InvalidOptIn.kt', '-cp', cp, '-d', work / 'historical-opt-in'], accepted=(0,))
            return dict(outcome='historical_wrong_acceptance_confirmed', compiler_sha256=sha(args.historical), correction='Observed RequiresOptIn/Experimental meta-annotations make a custom annotation semantics-bearing; no name-based application allowance')
        case('historical_opt_in_wrong_acceptance', historical)
    assert all(sha(path) == frozen[relative(path)] for path in inputs), 'Frozen inputs changed'
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    if not args.skip_html:
        subprocess.run(['python3', str(ROOT / 'tools/reports.py')], cwd=ROOT, check=True)
    print(report_path)
    return int(report['status'] == 'failed')

if __name__ == '__main__':
    raise SystemExit(main())
