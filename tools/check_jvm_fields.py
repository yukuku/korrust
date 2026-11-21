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
FIXTURES = ROOT / 'tests/jvm_fields'
APP = Path('benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/services/AppServices.kt')


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
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-instance-jvm-fields'
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
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/jvm_fields.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', ROOT / APP]
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
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), scope='JvmField instance constructor/body properties: public/protected and final/mutable JVM fields, absent generated accessors, field-only Kotlin metadata, initialization order, generic producer fields, source/library resolution and canonical annotation identity', limitations=['Static/object/companion and enum field lowering are outside this instance slice', 'JvmField value-class carrier properties, accessor annotations, volatile modifier combinations and declarations assigned later in init retain explicit unsupported boundaries', 'Generic field reading by Korrust retains existing independent interop bounds; generic field emission is checked through fixed Java/Kotlin consumers', 'No full Android module/application or performance claim'], effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'), application_classpath=[relative(standard), *map(str, app_paths)], reproduction_command=reproduction, cases=[], commands=[])

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
    dependency = directory / 'dependency'
    run('official unrelated annotation provider', [official, FIXTURES / 'Dependency.kt', '-cp', standard, '-d', dependency, *options])
    dependency_cp = cp(dependency, standard)
    models = [FIXTURES / name for name in ['Models.kt', 'Shadow.kt', 'LocalShadow.kt']]
    providers = [('official', directory / 'official'), ('candidate', directory / 'candidate')]

    def compile_models():
        run('official instance field fixtures', [official, *models, '-cp', dependency_cp, '-d', providers[0][1], *options])
        run('candidate instance field fixtures', [candidate, *models, '-cp', dependency_cp, '-d', providers[1][1]])
        return 'Matching sources and dependencies; alias and package-shadow annotation identity'
    case('semantic_fixture_compile', compile_models)

    def runtime():
        caller = directory / 'caller'
        run('fixed Java caller against official only', [java / 'javac', '-cp', cp(providers[0][1], dependency, standard), '-d', caller, FIXTURES / 'Calls.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        for name, output in providers:
            result = run('verified Java assertions on ' + name, [java / 'java', '-Xverify:all', '-cp', cp(caller, output, dependency, standard), 'fieldapi.Calls'])
            assert result.stdout == 'OK\n', result.stdout
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        return dict(caller_sha256=hashes, assertions='Direct reads/writes, final/public/protected/private flags, no generated accessors, explicit same-name method, generic field signature, nullable/wide/Boolean fields, initializer order, unchanged lateinit/accessor properties and annotation shadows')
    case('fixed_java_caller_both_providers', runtime)

    def abi():
        classes = ['fieldapi.' + name for name in ['Capsule', 'Ordered', 'AccessorName', 'GenericBox', 'Protected', 'Shadow', 'Guessed', 'Legacy', 'ModelsKt']] + ['fieldmarker.LocalShadow']
        a, b = [signatures(name, output, classes) for name, output in providers]
        assert a == b, (a, b)
        return a
    case('public_protected_jvm_abi', abi)

    for name, output in providers:
        def consumer(name=name, output=output):
            target = directory / ('consumer-' + name)
            run('official Kotlin consumer against ' + name, [official, FIXTURES / 'Consumer.kt', '-cp', cp(output, dependency, standard), '-d', target, *options])
            for runtime_name, runtime_out in providers:
                result = run('Kotlin consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', cp(target, runtime_out, dependency, standard), 'fieldconsumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Kotlin consumers compile and cross-run using emitted field-only property metadata'
        case('kotlin_consumer_' + name, consumer)

    def library_readers():
        official_reader = directory / 'reader-official'
        run('official library field reader', [official, FIXTURES / 'LibraryReader.kt', '-cp', cp(providers[0][1], dependency, standard), '-d', official_reader, *options])
        caller = directory / 'reader-caller'
        run('fixed library-reader caller against official only', [java / 'javac', '-cp', cp(official_reader, providers[0][1], dependency, standard), '-d', caller, FIXTURES / 'ReaderCalls.java'])
        for name, output in providers:
            reader = directory / ('reader-candidate-' + name)
            run('candidate library reader against ' + name, [candidate, FIXTURES / 'LibraryReader.kt', '-cp', cp(output, dependency, standard), '-d', reader])
            for provider_name, provider_out in providers:
                result = run('candidate reader ' + name + ' on ' + provider_name, [java / 'java', '-Xverify:all', '-cp', cp(caller, reader, provider_out, dependency, standard), 'fieldreader.ReaderCalls'])
                assert result.stdout == 'OK\n', result.stdout
        return 'Korrust consumes both providers through ordinary field resolution; fixed Java reader caller is compiled against official only'
    case('compiled_library_field_reads_writes', library_readers)

    for source in sorted(FIXTURES.glob('Invalid*.kt')):
        def invalid(source=source):
            classpath = cp(providers[0][1], dependency, standard)
            run('official invalid ' + source.stem, [official, source, '-cp', classpath, '-d', directory / ('official-' + source.stem), *options], accepted=(1,))
            result = run('candidate invalid ' + source.stem, [candidate, source, '-cp', classpath, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(1,))
            return dict(outcome='invalid_rejected', diagnostic=result.stderr)
        case(source.stem, invalid)

    for source in sorted(FIXTURES.glob('Unsupported*.kt')):
        def boundary(source=source):
            run('official valid boundary ' + source.stem, [official, source, '-cp', standard, '-d', directory / ('official-' + source.stem), *options])
            result = run('candidate boundary ' + source.stem, [candidate, source, '-cp', standard, '-d', directory / ('candidate-' + source.stem), '--diagnostic-format', 'json'], accepted=(0, 1))
            return dict(outcome='unsupported' if result.returncode else 'pass', diagnostic=result.stderr)
        case(source.stem, boundary)

    def oracle_parameter():
        source = FIXTURES / 'OracleParameter.kt'
        target = directory / 'oracle-parameter'
        run('pinned compiler accepts field annotation on nonproperty parameter', [official, source, '-cp', standard, '-d', target, *options])
        listing = run('pinned output has no corresponding field', [java / 'javap', '-p', '-classpath', target, 'Bad']).stdout
        assert 'java.lang.String value;' not in listing, listing
        result = run('candidate rejects nonexistent property field', [candidate, source, '-cp', standard, '-d', directory / 'candidate-parameter', '--diagnostic-format', 'json'], accepted=(1,))
        return dict(outcome='oracle_discrepancy', official_abi=listing, diagnostic=result.stderr, resolution='The pinned compiler accepts and drops the explicit field annotation on a constructor parameter without val/var. Korrust rejects the nonexistent property backing-field target rather than adopting ignored annotation semantics.', specification='references/kotlin-spec/docs/src/md/kotlin.core/annotations.md: allowed targets include a property backing field')
    case('oracle_nonproperty_parameter_field_target', oracle_parameter)

    app_cp = cp(standard, *app_paths)
    app_official, app_candidate = directory / 'app-official', directory / 'app-candidate'

    def application():
        run('official unchanged AppServices with genuine captured dependencies', [official, ROOT / APP, '-cp', app_cp, '-d', app_official, *options])
        run('candidate unchanged AppServices with identical dependencies', [candidate, ROOT / APP, '-cp', app_cp, '-d', app_candidate])
        caller = directory / 'app-caller'
        run('fixed ServicesCheck Java caller against official only', [java / 'javac', '-cp', cp(app_official, app_cp), '-d', caller, FIXTURES / 'ServicesCheck.java'])
        hashes = {relative(path): sha(path) for path in caller.rglob('*.class')}
        for name, output in [('official', app_official), ('candidate', app_candidate)]:
            run('verified fixed ServicesCheck on ' + name, [java / 'java', '-Xverify:all', '-cp', cp(caller, output, app_cp), 'ServicesCheck'])
        assert hashes == {relative(path): sha(path) for path in caller.rglob('*.class')}
        owner = 'yuku.alkitab.base.services.AppServices'
        a, b = [signatures('app-' + name, output, [owner]) for name, output in [('official', app_official), ('candidate', app_candidate)]]
        assert a == b, (a, b)
        return dict(caller_sha256=hashes, source_sha256=sha(ROOT / APP), assertions='Fixed original Java caller reads public final storage/versions/uiDimensions fields and checks constructor reference identity; genuine dependency artifacts are hashed and shared by both compilers')
    case('app/AppServices_fixed_java_and_abi', application)

    for name, output in [('official', app_official), ('candidate', app_candidate)]:
        def app_consumer(name=name, output=output):
            target = directory / ('app-consumer-' + name)
            run('official Kotlin AppServices consumer against ' + name, [official, FIXTURES / 'ServicesConsumer.kt', '-cp', cp(output, app_cp), '-d', target, *options])
            for runtime_name, runtime_out in [('official', app_official), ('candidate', app_candidate)]:
                result = run('AppServices consumer ' + name + ' on ' + runtime_name, [java / 'java', '-Xverify:all', '-cp', cp(target, runtime_out, app_cp), 'servicesconsumer.ServicesConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Actual unchanged app class consumed through ordinary Kotlin field-property metadata'
        case('app_kotlin_consumer_' + name, app_consumer)

    if args.baseline:
        def baseline():
            target = directory / 'app-baseline'
            run('historical baseline compile unchanged AppServices', [args.baseline, ROOT / APP, '-cp', app_cp, '-d', target])
            result = run('fixed official caller exposes historical wrong private field ABI', [java / 'java', '-Xverify:all', '-cp', cp(directory / 'app-caller', target, app_cp), 'ServicesCheck'], accepted=(1,))
            assert 'IllegalAccessError' in result.stderr and 'AppServices.storage' in result.stderr, result.stderr
            return dict(outcome='baseline_abi_failure', diagnostic=result.stderr, compiler_sha256=sha(args.baseline))
        case('baseline_wrong_app_field_exposure', baseline)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
