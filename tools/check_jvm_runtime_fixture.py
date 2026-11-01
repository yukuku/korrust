import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import zipfile

from upstream_jvm import Commands, ROOT, relative, sha


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', required=True, type=Path)
    parser.add_argument('--fixture', required=True, type=Path)
    parser.add_argument('--main-class', required=True)
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    (directory / 'commands').mkdir()
    candidate, fixture = args.candidate.resolve(), args.fixture.resolve()
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    kotlin_sources, java_sources = sorted(fixture.rglob('*.kt')), sorted(fixture.rglob('*.java'))
    if not kotlin_sources or not java_sources:
        raise ValueError('A fixture requires Kotlin providers and Java assertions')
    inputs = [*ROOT.glob('src/**/*.rs'), *kotlin_sources, *java_sources, Path(__file__), ROOT / 'tools/upstream_jvm.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', candidate, standard]
    frozen = {relative(path): sha(path) for path in inputs}
    with zipfile.ZipFile(directory / 'frozen-sources.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            if path != candidate and path != standard:
                archive.write(path, relative(path))
    commands = Commands(directory / 'commands', dict(os.environ))
    cases = []
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler={'path': relative(candidate), 'sha256': sha(candidate)}, reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), frozen_input_hashes=frozen, cases=cases, commands=commands.entries, source_archive={'path': relative(directory / 'frozen-sources.zip'), 'sha256': sha(directory / 'frozen-sources.zip')}, limitations=['Fixed Java caller runtime assertions only; this report alone establishes neither complete ABI nor Kotlin metadata compatibility.'])

    def run(label, argv):
        result = commands.run(label, argv)
        assert result is not None and result.returncode == 0, label + ': ' + (result.stderr if result else 'timeout')
        return result

    try:
        official, actual = directory / 'official', directory / 'candidate'
        run('official provider', [kotlin, *kotlin_sources, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-cp', standard, '-d', official])
        run('candidate provider', [candidate, *kotlin_sources, '-cp', standard, '-d', actual])
        callers = directory / 'callers'
        run('fixed Java caller', [java / 'javac', '-cp', os.pathsep.join(map(str, [official, standard])), '-d', callers, *java_sources])
        caller_hashes = {relative(path): sha(path) for path in callers.rglob('*.class')}
        for provider, output in [('official', official), ('candidate', actual)]:
            result = run('verified runtime ' + provider, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, standard])), args.main_class])
            assert result.stdout == 'OK\n', result.stdout
            cases.append({'id': 'verified_' + provider, 'outcome': 'pass'})
        assert caller_hashes == {relative(path): sha(path) for path in callers.rglob('*.class')}
        report['fixed_java_caller_hashes'] = caller_hashes
    except Exception as error:
        cases.append({'id': 'validation_failure', 'outcome': 'correctness_failure', 'error': str(error)})
    changed = [path for path, digest in frozen.items() if sha(ROOT / path) != digest]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    report['counts'] = dict(Counter(case['outcome'] for case in cases))
    report['status'] = 'completed' if not changed and all(case['outcome'] == 'pass' for case in cases) else 'failed'
    report['reproduction_command'] = ['python3', relative(__file__), '--candidate', relative(candidate), '--fixture', relative(fixture), '--main-class', args.main_class, '--output', 'target/runtime-fixture-fresh']
    (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(report['status'], report['counts'])
    return int(report['status'] != 'completed')


if __name__ == '__main__':
    raise SystemExit(main())
