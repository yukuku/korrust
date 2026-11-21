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
FIXTURES = ROOT / 'tests/static_jvm_fields'
APP = Path('benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/verses/VersesUiModel.kt')


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--app-classpaths', type=Path, default=Path('/workspace/work/deps33/classpaths.json'))
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-static-jvm-fields'
    directory = (args.output or ROOT / 'target/checks' / stamp).absolute()
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / stamp).absolute()
    directory.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-no-stdlib', '-no-reflect']
    app_paths = list(map(Path, json.loads(args.app_classpaths.read_text())['cases'][str(APP)]['classpath']))
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/static_jvm_fields.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', ROOT / APP]
    external = [args.app_classpaths, *app_paths, java.parent / 'jmods/java.base.jmod']
    if args.baseline:
        external.append(args.baseline)
    frozen = {relative(path): sha(path) for path in [*inputs, *external, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar']}
    shutil.copy2(args.app_classpaths, evidence / 'application-classpaths.json')
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    reproduction = ['python3', relative(__file__), '--candidate', relative(args.candidate), '--app-classpaths', str(args.app_classpaths), '--output', relative(directory), '--evidence-directory', relative(evidence)]
    if args.baseline:
        reproduction.extend(['--baseline', str(args.baseline)])
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), scope='Static JvmField on ordinary class companions and named objects: outer/object storage, initialization order, field-only metadata, absent generated accessors, source reads and writes, Java and Kotlin consumers, unchanged VersesUiModel', limitations=['Enum and interface companions, internal visibility, collisions with instance backing fields or singleton names retain explicit unsupported boundaries', 'Effectful object receiver expressions retain an explicit boundary; pinned compiler evaluation discrepancy is recorded separately', 'Existing value-class, volatile and accessor-annotation restrictions remain; no full Android module or performance claim', 'Compiled Kotlin static field consumption by Korrust is not claimed; independent official Kotlin consumers exercise emitted metadata'], effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'), application_classpath=[relative(standard), *map(str, app_paths)], reproduction_command=reproduction, cases=[], commands=[])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        command = list(map(str, command))
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

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def signatures(label, output, classes):
        rows = []
        for owner in classes:
            lines = run(label + ' ABI ' + owner, [java / 'javap', '-p', '-s', '-v', '-classpath', output, owner]).stdout.splitlines()
            rows.extend((owner, lines[index - 1].strip(), line.strip()) for index, line in enumerate(lines) if line.strip().startswith('descriptor:') and 'ACC_SYNTHETIC' not in lines[index + 1] and lines[index - 1].strip().startswith(('public ', 'protected ')))
        return sorted(rows)

    save()
    dependency_cp = str(standard)
    models = [FIXTURES / name for name in ['Models.kt', 'Read.kt', 'Extra.kt', 'Shadow.kt']]
    providers = [('official', directory / 'official'), ('candidate', directory / 'candidate')]

    def compile_models():
        run('official static field fixtures', [official, *models, '-cp', standard, '-d', providers[0][1], *options])
        run('candidate static field fixtures', [candidate, *models, '-cp', standard, '-d', providers[1][1]])
        return 'Identical source module and stdlib inputs; aliases and unrelated shadow annotation'
    case('semantic_fixture_compile', compile_models)

    def runtime():
        caller = directory / 'caller'
        run('fixed Java caller against official only', [java / 'javac', '-cp', cp(providers[0][1], standard), '-d', caller, FIXTURES / 'Calls.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        for name, output in providers:
            result = run('verified Java assertions on ' + name, [java / 'java', '-Xverify:all', '-cp', cp(caller, output, standard), 'staticprobe.Calls'])
            assert result.stdout == 'OK\n', result.stdout
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        return dict(caller_sha256=hashes, assertions='Static owner/public/final flags, initialization order and interleaved ordinary properties, direct reads/writes, nullable/wide/generic fields, deferred lambda reads, explicit same-name method, annotation aliases/shadows and absent generated accessors')
    case('fixed_java_caller_both_providers', runtime)

    def abi():
        owners = ['staticprobe.' + name for name in ['Host', 'Host$Companion', 'Global', 'Cache', 'Cache$Companion', 'Aliased', 'Carrier', 'shadow.Shadowed']]
        rows = [signatures(name, output, owners) for name, output in providers]
        assert rows[0] == rows[1], str(rows)
        return rows[0]
    case('public_protected_abi', abi)

    for name, output in providers:
        def consumer(name=name, output=output):
            target = directory / ('consumer-' + name)
            run('official Kotlin consumer against ' + name, [official, FIXTURES / 'Consumer.kt', '-cp', cp(output, standard), '-d', target, *options])
            for runtime_name, runtime_out in providers:
                result = run('Kotlin consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', cp(target, runtime_out, standard), 'staticconsumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin field-property metadata consumer, cross-run with both providers'
        case('kotlin_metadata_consumer_' + name, consumer)

    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            classpath = cp(providers[0][1], standard)
            run('official invalid ' + source.stem, [official, source, '-cp', classpath, '-d', directory / ('official-' + source.stem), *options], accepted=(1,))
            result = run('candidate invalid ' + source.stem, [candidate, source, '-cp', classpath, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            return dict(outcome='invalid_rejected', diagnostic=result.stderr)
        case(source.stem, invalid)

    for source in sorted(FIXTURES.glob('Unsupported*.kt')):
        def boundary(source=source):
            run('official valid boundary ' + source.stem, [official, source, '-cp', standard, '-d', directory / ('official-' + source.stem), *options])
            result = run('candidate boundary ' + source.stem, [candidate, source, '-cp', standard, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            diagnostic = json.loads(result.stderr)
            assert diagnostic['category'] == 'unsupported_language', diagnostic
            return dict(outcome='unsupported', diagnostic=diagnostic)
        case(source.stem, boundary)

    def receiver_oracle():
        target = directory / 'oracle-receiver'
        run('official receiver evaluation observation', [official, FIXTURES / 'OracleReceiver.kt', '-cp', standard, '-d', target, *options])
        caller = directory / 'oracle-caller'
        run('official receiver oracle Java caller', [java / 'javac', '-cp', cp(target, standard), '-d', caller, FIXTURES / 'OracleCalls.java'])
        result = run('observe receiver omission in pinned compiler', [java / 'java', '-Xverify:all', '-cp', cp(caller, target, standard), 'OracleCalls'])
        assert result.stdout == 'RV14\n', result.stdout
        return dict(outcome='oracle_discrepancy', observed=result.stdout, resolution='Pinned compiler evaluates the assignment receiver but drops evaluation of the read receiver for static object fields. Korrust rejects nontrivial receivers explicitly rather than silently adopting receiver omission.', specification='references/kotlin-spec/docs/src/md/kotlin.core/expressions.md: function-call and assignment evaluation')
    case('oracle_static_object_receiver_evaluation', receiver_oracle)

    app_cp = cp(standard, *app_paths)
    app_official, app_candidate = directory / 'app-official', directory / 'app-candidate'

    def application():
        run('official unchanged VersesUiModel with genuine captured dependencies', [official, ROOT / APP, '-cp', app_cp, '-d', app_official, *options])
        run('candidate unchanged VersesUiModel with identical dependencies', [candidate, ROOT / APP, '-cp', app_cp, '-d', app_candidate])
        caller = directory / 'app-caller'
        run('fixed VersesCheck Java caller against official only', [java / 'javac', '-cp', cp(app_official, app_cp), '-d', caller, FIXTURES / 'VersesCheck.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        for name, output in [('official', app_official), ('candidate', app_candidate)]:
            run('verified fixed VersesCheck on ' + name, [java / 'java', '-Xverify:all', '-cp', cp(caller, output, app_cp), 'VersesCheck'])
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        owner = 'yuku.alkitab.base.verses.VersesUiModel'
        a, b = [signatures('app-' + name, output, [owner]) for name, output in [('official', app_official), ('candidate', app_candidate)]]
        assert a == b, (a, b)
        return dict(caller_sha256=hashes, source_sha256=sha(ROOT / APP), assertions='Fixed original official Java caller checks EMPTY defaults, ReadingGuide and enum identity, Float/Set fields, data copy/components/equality; both compilers use identical genuine dependencies')
    case('app/VersesUiModel_fixed_java_and_abi', application)

    for name, output in [('official', app_official), ('candidate', app_candidate)]:
        def app_consumer(name=name, output=output):
            target = directory / ('app-consumer-' + name)
            run('official Kotlin VersesUiModel consumer against ' + name, [official, FIXTURES / 'VersesConsumer.kt', '-cp', cp(output, app_cp), '-d', target, *options])
            for runtime_name, runtime_out in [('official', app_official), ('candidate', app_candidate)]:
                result = run('VersesUiModel consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', cp(target, runtime_out, app_cp), 'versesconsumer.VersesConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Actual unchanged app class consumed through ordinary Kotlin field-property metadata'
        case('app_kotlin_consumer_' + name, app_consumer)

    if args.baseline:
        def baseline():
            result = run('pre-feature baseline rejects companion static fields', [args.baseline, *models, '-cp', standard, '-d', directory / 'baseline', '--diagnostic-format', 'json'], accepted=(1,))
            assert 'JVM_FIELD_STATIC_UNSUPPORTED' in result.stderr, result.stderr
            return dict(outcome='baseline_unsupported', diagnostic=result.stderr, compiler_sha256=sha(args.baseline))
        case('baseline_static_field_boundary', baseline)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
