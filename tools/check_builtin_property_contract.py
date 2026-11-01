import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path

from upstream_jvm import Commands, ROOT, relative, sha


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    (directory / 'commands').mkdir()
    fixture = ROOT / 'tests/fixtures/builtin_properties'
    candidate = args.candidate.resolve()
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    inputs = [*ROOT.glob('src/**/*.rs'), *fixture.iterdir(), Path(__file__), ROOT / 'tools/upstream_jvm.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', candidate, standard]
    frozen = {relative(path): sha(path) for path in inputs}
    commands = Commands(directory / 'commands', dict(os.environ))
    cases = []
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler={'path': relative(candidate), 'sha256': sha(candidate)}, frozen_input_hashes=frozen, reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), cases=cases, commands=commands.entries, limitations=['Repair of the pinned Kotlin/JVM property contract; no additional feature family or complete builtin mapping claim.'])

    def run(label, argv, exit_code=0):
        result = commands.run(label, argv)
        assert result is not None and result.returncode == exit_code, label + ': ' + (result.stderr if result else 'timeout')
        return result

    try:
        official, actual = directory / 'official', directory / 'candidate'
        run('official positive', [kotlin, fixture / 'Values.kt', *options, '-cp', standard, '-d', official])
        run('candidate positive', [candidate, fixture / 'Values.kt', '-cp', standard, '-d', actual])
        callers = directory / 'callers'
        run('fixed Java caller', [java / 'javac', '-cp', os.pathsep.join(map(str, [official, standard])), '-d', callers, fixture / 'Calls.java'])
        caller_hashes = {relative(path): sha(path) for path in callers.rglob('*.class')}
        for provider, output in [('official', official), ('candidate', actual)]:
            run('verified runtime ' + provider, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, standard])), 'Calls'])
            cases.append({'id': 'verified_' + provider, 'outcome': 'pass'})
        for index, source in enumerate(['fun f(x: String): Int = x.length()', 'fun f(x: CharSequence): Int = x.length()', 'fun f(x: java.lang.StringBuilder): Int = x.length()']):
            path = directory / f'Invalid{index}.kt'
            path.write_text(source)
            run('official invalid ' + str(index), [kotlin, path, *options, '-cp', standard, '-d', directory / f'invalid-official-{index}'], 1)
            result = run('candidate invalid ' + str(index), [candidate, path, '--diagnostic-format', 'json', '-cp', standard, '-d', directory / f'invalid-candidate-{index}'], 1)
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            assert diagnostics and diagnostics[0]['category'] == 'invalid_kotlin', diagnostics
            cases.append({'id': 'invalid_' + str(index), 'outcome': 'invalid_rejected', 'source': source, 'diagnostics': diagnostics})
        assert caller_hashes == {relative(path): sha(path) for path in callers.rglob('*.class')}
        report['fixed_java_caller_hashes'] = caller_hashes
    except Exception as error:
        cases.append({'id': 'validation_failure', 'outcome': 'correctness_failure', 'error': str(error)})
    changed = [path for path, digest in frozen.items() if sha(ROOT / path) != digest]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    report['counts'] = dict(Counter(case['outcome'] for case in cases))
    report['status'] = 'completed' if not changed and not any(case['outcome'] == 'correctness_failure' for case in cases) else 'failed'
    (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(report['status'], report['counts'])
    return int(report['status'] != 'completed')


if __name__ == '__main__':
    raise SystemExit(main())
