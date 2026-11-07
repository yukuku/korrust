#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/suspend_foundation'


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
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    (directory / 'commands').mkdir()
    java_home = Path(os.environ['JAVA_HOME'])
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    compiler = ROOT / 'references/kotlinc/lib/kotlin-compiler.jar'
    candidate = args.candidate.resolve()
    source_paths = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), Path(__file__).resolve(), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', ROOT / 'references/kotlin/libraries/stdlib/jvm/src/kotlin/coroutines/jvm/internal/boxing.kt']
    matrix = ROOT / 'tools/upstream_suspend_foundation.json'
    if matrix.is_file():
        source_paths.append(matrix)
    frozen_paths = [*source_paths, candidate, standard, compiler]
    frozen = {relative(path): sha(path) for path in frozen_paths}
    with zipfile.ZipFile(directory / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in source_paths:
            archive.writestr(str(path.relative_to(ROOT)), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running',
                  compiler=dict(path=relative(candidate), sha256=sha(candidate)), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
                  frozen_input_hashes=frozen, input_bundle=dict(path=relative(directory / 'inputs.zip'), sha256=sha(directory / 'inputs.zip')),
                  cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(candidate), '--output', relative(directory)],
                  runtime_boxing_oracle=dict(source='references/kotlin/libraries/stdlib/jvm/src/kotlin/coroutines/jvm/internal/boxing.kt', source_sha256=sha(ROOT / 'references/kotlin/libraries/stdlib/jvm/src/kotlin/coroutines/jvm/internal/boxing.kt'), evidence='Java asserts coroutine Int input/result wrappers, boxed Any local, listOf element and nullable-call input are fresh, while a separate ordinary boxing function retains Integer.valueOf cache identity'),
                  scope='Bounded suspend functions with real delayed continuation resumption, saved state, ordinary source/compiled suspend calls, function types, public JVM ABI and logical Kotlin metadata',
                  limitations=['Source suspend lambdas/references/conversions, suspend member/extension/generic/default/vararg/inline declarations and nested suspension control flow are unsupported', 'Imported suspend inline/default-mask/member calls are unsupported', 'Nullable Unit and Unit-valued suspend callback parameters are unsupported; no source generic kotlin.Result/value-class support or frame DebugMetadata/debugger/stacktrace equivalence claim', 'Public logical function ABI and metadata are compared; generated frame names and spill layouts need not match', 'Authored dependency is an identical external library classpath for both providers; unchanged upstream CoroutineHelpers.kt must be compiled separately by the upstream adapter'])
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']

    def save():
        report['counts'] = dict(Counter(item['outcome'] for item in report['cases']))
        (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        number = len(report['commands'])
        command = [str(part) for part in command]
        try:
            result = subprocess.run(command, capture_output=True, text=True, timeout=180, cwd=ROOT)
        except subprocess.TimeoutExpired as error:
            raise AssertionError(label + ': timeout') from error
        log = directory / 'commands' / f'{number:03d}.txt'
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
    run('authored external suspend dependency compiled once with pinned Kotlin', [official, FIXTURES / 'Dependency.kt', '-cp', standard, '-d', dependency, *options])
    dependency_hashes = {relative(path): sha(path) for path in dependency.rglob('*.class')}
    cp = os.pathsep.join(map(str, [standard, dependency]))
    official_out, candidate_out = directory / 'official', directory / 'candidate'
    run('official model', [official, FIXTURES / 'Models.kt', '-cp', cp, '-d', official_out, *options])
    run('candidate model', [candidate, FIXTURES / 'Models.kt', '-cp', cp, '-d', candidate_out])
    java_out = directory / 'java'
    run('Java driver and metadata decoder against official output only', [java_home / 'bin/javac', '-cp', os.pathsep.join(map(str, [official_out, standard, compiler])), '-d', java_out, FIXTURES / 'Calls.java', FIXTURES / 'MetadataCheck.java'])
    callers = {relative(path): sha(path) for path in java_out.rglob('*.class')}
    providers = [('official', official_out), ('candidate', candidate_out)]
    for group in ['delayed', 'scalars', 'failures', 'private', 'boxing']:
        def runtime(group=group):
            for provider, output in providers:
                result = run(provider + ' independent ' + group, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, dependency, standard])), 'suspension.Calls', group])
                assert result.stdout == 'OK\n', result.stdout
                if group == 'boxing':
                    bytecode = run(provider + ' boxing positions bytecode', [java_home / 'bin/javap', '-c', '-p', '-classpath', output, 'suspension.ModelsKt']).stdout
                    ordinary = bytecode.split(' ordinaryBox(', 1)[1].split('\n  public ', 1)[0]
                    coroutine = bytecode.split(' boxingPositions(', 1)[1].split('\n  public ', 1)[0]
                    assert 'java/lang/Integer.valueOf' in ordinary, ordinary
                    assert 'kotlin/coroutines/jvm/internal/Boxing.boxInt' in coroutine, coroutine
                    assert 'java/lang/Integer.valueOf' not in coroutine, coroutine
            return 'Identical Java caller bytecode compiled against official only; real delayed resumption and independent assertions under -Xverify:all'
        case('runtime_' + group, runtime)

    for compile_provider, compile_output in providers:
        def kotlin_consumer(compile_provider=compile_provider, compile_output=compile_output):
            consumer_out = directory / ('consumer-' + compile_provider)
            run('Kotlin consumer against ' + compile_provider, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [compile_output, dependency, standard])), '-d', consumer_out, *options])
            for runtime_provider, runtime_output in providers:
                result = run('Kotlin consumer ' + compile_provider + ' on ' + runtime_provider, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_output, dependency, standard])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin consumer compiles against ' + compile_provider + ', suspends twice using standard suspendCoroutine, then resumes and completes on both outputs'
        case('kotlin_consumer_' + compile_provider, kotlin_consumer)

    def metadata():
        results = [run(provider + ' decoded logical suspend metadata', [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, dependency, standard, compiler])), 'suspension.MetadataCheck']).stdout for provider, output in providers]
        assert results[0] == results[1], '\nOFFICIAL\n' + results[0] + '\nCANDIDATE\n' + results[1]
        return 'Function visibility/suspend flags, logical return and parameter types, names, nested suspend type flags/projections/nullability and resolved JVM signatures match pinned Kotlin metadata'
    case('logical_metadata', metadata)

    def abi():
        selected = set(re.findall(r'(?:suspend fun|private suspend fun|fun)\s+(\w+)\s*\(', (FIXTURES / 'Models.kt').read_text())) | {'getTrace', 'setTrace'}
        selected.discard('hidden')
        results = []
        for provider, output in providers:
            result = run(provider + ' public logical JVM signatures', [java_home / 'bin/javap', '-public', '-s', '-classpath', output, 'suspension.ModelsKt'])
            lines = result.stdout.splitlines()
            declarations = []
            for index, line in enumerate(lines):
                if '(' not in line:
                    continue
                name = line.split('(', 1)[0].split()[-1]
                if name in selected:
                    declarations.append((name, line.strip(), lines[index + 1].strip()))
            results.append(sorted(declarations))
        assert results[0] == results[1], str(results)
        assert {item[0] for item in results[0]} == selected, results[0]
        return 'Public Kotlin declarations have exact JVM descriptors and generic signatures, including Continuation/Object and runtime FunctionN continuation parameters; generated synthetic bridges/frames excluded explicitly'
    case('public_jvm_abi', abi)

    invalid = [
        'fun broken(step: suspend () -> String): String = step()',
        'suspend fun broken(step: suspend () -> Int): String = step()',
        'suspend fun broken(step: suspend (Int) -> Int): Int = step("wrong")',
        'class Holder { fun broken(step: suspend () -> String): String = step() }',
        'object Holder { fun broken(step: suspend () -> String): String = step() }',
    ]
    for index, source in enumerate(invalid):
        def rejection(index=index, source=source):
            path = directory / f'Invalid{index}.kt'
            path.write_text(source)
            run('oracle invalid ' + str(index), [official, path, '-cp', standard, '-d', directory / f'invalid-official-{index}', *options], accepted=(1,))
            result = run('candidate invalid ' + str(index), [candidate, path, '-cp', standard, '-d', directory / f'invalid-candidate-{index}', '--diagnostic-format', 'json'], accepted=(1,))
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            assert diagnostics and diagnostics[0]['category'] == 'invalid_kotlin', diagnostics
            return dict(outcome='invalid_rejected', diagnostics=diagnostics)
        case('invalid_' + str(index), rejection)

    unsupported = [
        'suspend fun bounded(step: suspend () -> String): String = if (true) step() else "x"',
        'suspend fun bounded(step: suspend () -> String): String { while (true) { step() }; return "x" }',
        'suspend fun bounded(step: suspend () -> String): String = try { step() } finally { }',
        'suspend fun <T> bounded(value: T): T = value',
        'suspend fun bounded(value: Int = 1): Int = value',
        'class Member { private suspend fun bounded(): String = "x" }',
        'suspend fun bounded(step: suspend () -> String): String { val value: suspend () -> String = { "x" }; return step() }',
        'suspend fun original(): String = "x"\nfun bounded(): suspend () -> String = ::original',
        'suspend fun bounded(step: suspend () -> String): String = step() + step()',
        'suspend fun bounded(step: suspend (Unit) -> String): String = step(Unit)',
        'suspend fun bounded(): Unit? = null',
        '@JvmInline value class V(val raw: Int)\nsuspend fun bounded(value: V): V = value',
        '@JvmInline value class V(val raw: Int)\nsuspend fun bounded(step: suspend (V) -> Int): Int = step(V(1))',
        '@JvmInline value class V(val raw: Int)\nsuspend fun bounded(step: suspend () -> Int): Int { val saved = V(1); val result = step(); return saved.raw + result }',
    ]
    for index, source in enumerate(unsupported):
        def boundary(index=index, source=source):
            path = directory / f'Unsupported{index}.kt'
            path.write_text(source)
            run('oracle accepts boundary ' + str(index), [official, path, '-cp', standard, '-d', directory / f'unsupported-official-{index}', *options])
            result = run('candidate unsupported ' + str(index), [candidate, path, '-cp', standard, '-d', directory / f'unsupported-candidate-{index}', '--diagnostic-format', 'json'], accepted=(1,))
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            assert diagnostics and diagnostics[0]['category'] == 'unsupported_language', diagnostics
            if '@JvmInline value class V' in source:
                assert diagnostics[0]['code'] == 'SUSPEND_VALUE_CLASS_INTERACTION_UNSUPPORTED', diagnostics
            return dict(outcome='unsupported_language', oracle='valid Kotlin', diagnostics=diagnostics)
        case('unsupported_' + str(index), boundary)

    assert callers == {relative(path): sha(path) for path in java_out.rglob('*.class')}
    assert dependency_hashes == {relative(path): sha(path) for path in dependency.rglob('*.class')}
    changed = [path for path, value in frozen.items() if sha(ROOT / path) != value]
    report['input_integrity'] = dict(status='changed' if changed else 'unchanged', changed_paths=changed)
    report['java_caller_hashes'] = callers
    report['external_dependency_hashes'] = dependency_hashes
    report['status'] = 'complete' if not changed and all(item['outcome'] in {'pass', 'invalid_rejected', 'unsupported_language'} for item in report['cases']) else 'failed'
    save()
    print(json.dumps(dict(status=report['status'], counts=report['counts'])))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
