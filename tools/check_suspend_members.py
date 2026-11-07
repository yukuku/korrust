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
FIXTURES = ROOT / 'tests/fixtures/suspend_members'


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
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-suspend-members'
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
    source_paths = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/suspend_members.rs', Path(__file__).resolve(), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
    frozen_paths = [*source_paths, candidate, standard, compiler]
    frozen = {relative(path): sha(path) for path in frozen_paths}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in source_paths:
            archive.writestr(relative(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
                  compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
                  frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), evidence=dict(work_directory=relative(directory), logs=relative(evidence / 'commands')),
                  effective_options=dict(jvm_target='1.8', language_version='2.3', api_version='2.3'), cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--output', relative(directory), '--evidence-directory', relative(evidence)],
                  scope='Bounded source suspend abstract interface members, default interface bodies, final class implementations, source and compiled virtual/interface suspend calls, real saved dispatch receivers and continuation resumption',
                  limitations=['Custom parameter annotations on suspend declarations are unsupported', 'Private, generic, extension, inline, vararg and defaulted suspend members are unsupported', 'Concrete suspend member bodies on open or abstract classes and suspend object declarations are unsupported', 'Existing straight-line suspension limit of eight points and suspend foundation value-class, nullable Unit, source suspend lambda and callback restrictions remain', 'No full AndroidBible AudioHttp.kt claim: its production object and withContext lambda remain unsupported'])
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
    dependency = directory / 'dependency'
    run('official external suspend interface dependency', [official, FIXTURES / 'Dependency.kt', '-cp', standard, '-d', dependency, *options])
    cp = os.pathsep.join(map(str, [standard, dependency]))
    official_out, candidate_out = directory / 'official', directory / 'candidate'
    run('official model', [official, FIXTURES / 'Models.kt', '-cp', cp, '-d', official_out, *options])
    run('candidate model', [candidate, FIXTURES / 'Models.kt', '-cp', cp, '-d', candidate_out])
    java_out = directory / 'java'
    run('Java caller and metadata decoder compiled against official only', [java_home / 'bin/javac', '-cp', os.pathsep.join(map(str, [official_out, dependency, standard, compiler])), '-d', java_out, FIXTURES / 'Calls.java', FIXTURES / 'MetadataCheck.java'])
    callers = {relative(path): sha(path) for path in java_out.rglob('*.class')}
    providers = [('official', official_out), ('candidate', candidate_out)]
    cross_callers = {}
    for group in ['immediate', 'delayed', 'failures', 'scalars', 'dependency']:
        def runtime(group=group):
            for provider, output in providers:
                result = run(provider + ' independent ' + group, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, dependency, standard])), 'members.Calls', group])
                assert result.stdout == 'OK\n', result.stdout
            return 'Same Java caller compiled once against official; independent immediate/delayed/error assertions with JVM verification'
        case('runtime_' + group, runtime)

    for compile_provider, compile_output in providers:
        def kotlin_consumer(compile_provider=compile_provider, compile_output=compile_output):
            consumer_out = directory / ('consumer-' + compile_provider)
            run('Kotlin consumer against ' + compile_provider, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [compile_output, dependency, standard])), '-d', consumer_out, *options])
            for runtime_provider, runtime_output in providers:
                result = run('Kotlin consumer ' + compile_provider + ' on ' + runtime_provider, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_output, dependency, standard])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin consumer reads logical suspend member metadata, constructs suspend callback and fun-interface SAM, and resumes twice against both providers'
        case('kotlin_consumer_' + compile_provider, kotlin_consumer)

    def compiled_interfaces():
        oracle_cross = directory / 'cross-official'
        run('official compiled-interface consumer', [official, FIXTURES / 'Cross.kt', '-cp', os.pathsep.join(map(str, [official_out, standard])), '-d', oracle_cross, *options])
        cross_java = directory / 'cross-java'
        run('Java compiled-interface caller against official only', [java_home / 'bin/javac', '-cp', os.pathsep.join(map(str, [oracle_cross, official_out, standard])), '-d', cross_java, FIXTURES / 'CrossCalls.java'])
        cross_callers.update({relative(path): sha(path) for path in cross_java.rglob('*.class')})
        for compile_provider, compile_output in providers:
            cross_out = directory / ('cross-candidate-' + compile_provider)
            run('candidate source implementation against ' + compile_provider + ' interface metadata', [candidate, FIXTURES / 'Cross.kt', '-cp', os.pathsep.join(map(str, [compile_output, standard])), '-d', cross_out])
            for runtime_provider, runtime_output in providers:
                result = run('compiled-interface consumer ' + compile_provider + ' on ' + runtime_provider, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [cross_out, cross_java, runtime_output, standard])), 'cross.CrossCalls'])
                assert result.stdout == 'OK\n', result.stdout
        return 'Korrust reads official and its own compiled suspend interface metadata; unchanged official Java caller resumes source implementations and inherited default forwarding on both providers'
    case('compiled_interface_providers', compiled_interfaces)

    def metadata():
        results = [run(provider + ' logical suspend member metadata', [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, dependency, standard, compiler])), 'members.MetadataCheck']).stdout for provider, output in providers]
        assert results[0] == results[1], results
        assert 'Boundary.read result=kotlin/String?true' in results[0], results[0]
        assert 'Lkotlin/coroutines/Continuation;)Ljava/lang/Object;' in results[0], results[0]
        return results[0]
    case('logical_metadata', metadata)

    def signatures():
        extracted = []
        for provider, output in providers:
            signatures = []
            for owner in ['members.Boundary', 'members.Reader', 'members.Immediate', 'members.Scalars', 'members.ExternalReader', 'members.AbstractBoundary']:
                listing = run(provider + ' JVM signatures ' + owner, [java_home / 'bin/javap', '-s', '-p', '-classpath', output, owner]).stdout
                lines = listing.splitlines()
                for index, line in enumerate(lines):
                    if 'Continuation' in line and '(' in line and not line.strip().startswith('descriptor:'):
                        name = re.search(r'([\w$]+)\(', line).group(1)
                        signatures.append(owner + '.' + name + ' ' + lines[index + 1].strip())
            extracted.append(sorted(signatures))
        assert extracted[0] == extracted[1], extracted
        return extracted[0]
    case('jvm_signatures', signatures)

    def abstract_annotations():
        for provider, output in providers:
            listing = run(provider + ' abstract suspend parameter annotations', [java_home / 'bin/javap', '-v', '-p', '-classpath', output, 'members.AbstractBoundary']).stdout
            method = listing.split(' read(', 1)[1].split('RuntimeInvisibleParameterAnnotations:', 1)[1]
            assert 'parameter 0:' in method and 'parameter 1:' in method, method
            assert method.count('org.jetbrains.annotations.NotNull') == 2, method
        return 'Abstract suspend declarations annotate both the logical String parameter and physical trailing Continuation'
    case('abstract_continuation_parameter_annotations', abstract_annotations)

    invalid = [
        'interface R { suspend fun read(): String }; fun broken(reader: R): String = reader.read()',
        'class R { suspend fun read(): String = "x"; fun broken(): String = read() }',
        'interface R { suspend fun read(): String; fun broken(): String = read() }',
        'interface R { suspend fun read(): String }; class C: R { override fun read(): String = "x" }',
        'interface R { fun read(): String }; class C: R { override suspend fun read(): String = "x" }',
    ]
    unsupported = [
        'class C { private suspend fun read(): String = "x" }',
        'annotation class Mark; suspend fun read(@Mark value: String): String = value',
        'annotation class Mark; class C { suspend fun read(@Mark value: String): String = value }',
        'annotation class Mark; interface C { suspend fun read(@Mark value: String): String }',
        'annotation class Mark; abstract class C { abstract suspend fun read(@Mark value: String): String }',
        'enum class C { A; suspend fun read(): String = "x" }',
        'interface R { suspend fun read(): String }; class C(next: R): R by next',
        'interface R<T> { suspend fun read(value: T): T }; class C: R<String> { override suspend fun read(value: String): String = value }',
        'open class C { open suspend fun read(): String = "x" }',
        'class C { suspend fun <T> read(value: T): T = value }',
        'class C { suspend fun read(value: String = "x"): String = value }',
        'class C { suspend fun read(step: suspend () -> String): String = if (true) step() else "x" }',
        'interface R { suspend fun read(step: suspend () -> String): String = try { step() } finally { } }',
    ]
    for category, sources in [('invalid_kotlin', invalid), ('unsupported_language', unsupported)]:
        for index, source in enumerate(sources):
            def boundary(category=category, index=index, source=source):
                path = directory / f'{category}{index}.kt'
                path.write_text(source)
                run('oracle boundary ' + category + str(index), [official, path, '-cp', standard, '-d', directory / f'oracle-{category}-{index}', *options], accepted=(1,) if category == 'invalid_kotlin' else (0,))
                result = run('candidate boundary ' + category + str(index), [candidate, path, '-cp', standard, '-d', directory / f'candidate-{category}-{index}', '--diagnostic-format', 'json'], accepted=(1,))
                diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
                assert diagnostics and diagnostics[0]['category'] == category, diagnostics
                return dict(outcome='invalid_rejected' if category == 'invalid_kotlin' else 'unsupported_language', diagnostics=diagnostics)
            case(category + '_' + str(index), boundary)
    assert callers == {relative(path): sha(path) for path in java_out.rglob('*.class')}
    assert frozen == {relative(path): sha(path) for path in frozen_paths}, 'Inputs changed during harness run'
    assert all(sha(ROOT / path) == digest for path, digest in cross_callers.items())
    report['java_callers'] = {**callers, **cross_callers}
    report['status'] = 'fail' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'pass'
    save()
    print(json.dumps(dict(status=report['status'], counts=report['counts'], report=relative(report_path))))
    return int(report['status'] != 'pass')


if __name__ == '__main__':
    raise SystemExit(main())
