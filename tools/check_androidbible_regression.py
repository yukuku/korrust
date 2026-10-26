#!/usr/bin/env python3
"""Run the preserved AndroidBible checks selected by tests/androidbible_regression.json."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / 'tests/androidbible_regression.json'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def validate_manifest(manifest):
    inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
    historical = json.loads((ROOT / manifest['historical_assessment']).read_text())
    required = {case['id'] for case in historical['cases'] if case['outcome'] == 'emitted'}
    required.add('benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/PlaybackState.kt')
    actual = [case['source'] for case in manifest['cases']]
    if len(actual) != len(set(actual)) or set(actual) != required:
        raise ValueError('Manifest must contain exactly the historical emission sources plus PlaybackState')
    if len({case['id'] for case in manifest['cases']}) != len(actual):
        raise ValueError('Duplicate regression case identity')
    for case in manifest['cases']:
        for entry in [{'source': case['source'], 'sha256': case['source_sha256']}, *case['dependency_sources']]:
            path = entry['source']
            pinned = inventory[path.removeprefix('benchmarks/sources/androidbible/')]
            if entry['sha256'] != pinned or sha(ROOT / path) != pinned:
                raise ValueError('Pinned source changed: ' + path)
        if case['dependency_setup'] not in manifest['dependency_setups']:
            raise ValueError('Missing dependency setup: ' + case['id'])
        for kind in ['emission', 'behavior', 'abi', 'kotlin_consumer']:
            if not case['checks'][kind] and kind not in case['gaps']:
                raise ValueError('Unexplained check gap: ' + case['id'] + '/' + kind)
            for check in case['checks'][kind]:
                if check['suite'] not in manifest['suites']:
                    raise ValueError('Unknown suite: ' + check['suite'])


def compile_evidence(report, source, compiler):
    expected_source = (ROOT / source).resolve()
    for command in report.get('commands', []):
        argv = command.get('argv', command.get('command', []))
        if not argv or command.get('exit_code') != 0:
            continue
        if Path(argv[0]).resolve() != Path(compiler).resolve():
            continue
        if any(Path(arg).resolve() == expected_source for arg in argv[1:] if str(arg).endswith('.kt')):
            return True
    return False


def aggregate(manifest, suites, compiler):
    cases = []
    for target in manifest['cases']:
        checks = {}
        for kind, requirements in target['checks'].items():
            observations = []
            for requirement in requirements:
                suite = suites.get(requirement['suite'], {})
                report = suite.get('report', {})
                if 'compile_source' in requirement:
                    passed = compile_evidence(report, requirement['compile_source'], compiler)
                    observations.append({**requirement, 'outcome': 'pass' if passed else 'fail'})
                else:
                    matches = [case for case in report.get('cases', []) if case['id'] == requirement['case']]
                    passed = len(matches) == 1 and matches[0]['outcome'] == 'pass'
                    observations.append({**requirement, 'outcome': 'pass' if passed else 'fail', 'observed': [case['outcome'] for case in matches]})
            checks[kind] = {'outcome': ('pass' if all(item['outcome'] == 'pass' for item in observations) else 'fail') if observations else 'not_run', 'evidence_cases': observations}
            if not observations:
                checks[kind]['reason'] = target['gaps'][kind]
        failed = any(check['outcome'] == 'fail' for check in checks.values())
        cases.append({'id': target['source'], 'source_sha256': target['source_sha256'], 'outcome': 'fail' if failed else 'pass', 'checks': checks, 'dependency_setup': target['dependency_setup'], 'dependency_sources': target['dependency_sources'], 'scope': target['scope']})
    suite_failures = [name for name, suite in suites.items() if suite.get('outcome') != 'pass']
    if suite_failures:
        cases.append({'id': 'required_suites', 'outcome': 'fail', 'suites': suite_failures})
    return cases


def assess_suite(report, exit_code, compiler_sha):
    return ('pass' if exit_code == 0 and report.get('status') == 'completed'
            and report.get('compiler', {}).get('binary_sha256') == compiler_sha
            and not any(case['outcome'] in ['fail', 'failure', 'timeout', 'compiler_failure', 'setup_failure', 'dependency_failure', 'error'] for case in report.get('cases', [])) else 'fail')


def suite_command(name, suite, args):
    command = [sys.executable, str(ROOT / suite['tool'])]
    if name != 'baseline':
        command += ['--compiler', str(args.compiler)]
    if name in ['simple', 'ten', 'twenty']:
        command += ['--android-jar', str(args.android_jar), '--d8-jar', str(args.d8_jar)]
    if name == 'interop':
        command += ['--android-jar', str(args.android_jar), '--androidx-annotations', str(args.androidx_annotations)]
    return command


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest', type=Path, default=MANIFEST)
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--android-jar', type=Path, required=True)
    parser.add_argument('--d8-jar', type=Path, required=True)
    parser.add_argument('--androidx-annotations', type=Path, required=True)
    parser.add_argument('--timeout', type=float, default=7200, help='Maximum seconds per preserved suite')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    args.compiler = args.compiler.resolve()
    if args.compiler != (ROOT / 'target/release/korrust').resolve():
        parser.error('The preserved baseline builds target/release/korrust; select that binary for matched regression evidence')
    now = datetime.now(timezone.utc)
    output = args.output or ROOT / 'reports/correctness' / (now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-androidbible-regression.json')
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists():
        parser.error('Refusing to overwrite existing evidence: ' + str(output))
    evidence_dir = output.with_suffix('')
    evidence_dir.mkdir(parents=True)
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': now.isoformat(), 'status': 'running', 'selection': 'Required exact AndroidBible source regression selection; per-dimension gaps are not passes.', 'compiler': {}, 'reference_compiler': json.loads((ROOT / 'references/compiler.json').read_text()), 'inputs': {}, 'commands': [], 'evidence': [], 'cases': []}
    suites = {}
    try:
        manifest = json.loads(args.manifest.read_text())
        validate_manifest(manifest)
        frozen = {relative(path): sha(path) for folder in ['src', 'tests', 'tools'] for path in (ROOT / folder).rglob('*') if path.is_file() and '__pycache__' not in path.parts}
        frozen.update({relative(path): sha(path) for path in [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', args.manifest, ROOT / manifest['historical_assessment']]})
        compiler_sha = sha(args.compiler)
        report['compiler'] = {'binary_sha256': compiler_sha, 'source_files_sha256': frozen}
        report['inputs'] = {'manifest': relative(args.manifest), 'manifest_sha256': sha(args.manifest), 'dependency_setups': manifest['dependency_setups'], 'external_dependencies_sha256': {str(path): sha(path) for path in [args.android_jar, args.d8_jar, args.androidx_annotations]}}
        report['evidence'].append(relative(args.manifest))
        for name, suite in manifest['suites'].items():
            command = suite_command(name, suite, args)
            print('Required suite: ' + name, flush=True)
            try:
                result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=args.timeout)
                stdout, stderr, code = result.stdout, result.stderr, result.returncode
            except subprocess.TimeoutExpired as error:
                stdout, stderr, code = error.stdout or b'', error.stderr or b'', None
            stdout = stdout.decode(errors='replace') if isinstance(stdout, bytes) else stdout
            stderr = stderr.decode(errors='replace') if isinstance(stderr, bytes) else stderr
            entry = {'argv': command, 'exit_code': code, 'timeout_seconds': args.timeout}
            for stream, value in [('stdout', stdout), ('stderr', stderr)]:
                path = evidence_dir / (name + '-' + stream + '.txt')
                path.write_text(value)
                entry[stream] = relative(path)
                report['evidence'].append(relative(path))
            report['commands'].append(entry)
            matches = re.findall(r'^Report: (.+\.json)$', stdout, re.MULTILINE)
            child = {}
            if len(matches) == 1:
                child_path = (ROOT / matches[0]).resolve()
                if child_path.is_relative_to(ROOT / 'reports/correctness') and child_path.is_file():
                    child = json.loads(child_path.read_text())
                    report['evidence'].append(relative(child_path))
            suites[name] = {'outcome': assess_suite(child, code, compiler_sha), 'report': child, 'report_path': matches[0] if len(matches) == 1 else None}
        report['cases'] = aggregate(manifest, suites, args.compiler)
        report['suites'] = {name: {key: value for key, value in suite.items() if key != 'report'} for name, suite in suites.items()}
        if any(sha(Path(path)) != checksum for path, checksum in report['inputs']['external_dependencies_sha256'].items()):
            raise AssertionError('External dependency changed during regression run')
        if sha(args.compiler) != compiler_sha or any(sha(ROOT / path) != value for path, value in frozen.items()):
            raise AssertionError('Frozen compiler or inputs changed during regression run')
        report['status'] = 'completed' if all(case['outcome'] == 'pass' for case in report['cases']) else 'failed'
    except Exception as error:
        report['status'] = 'failed'
        report['cases'].append({'id': 'harness_failure', 'outcome': 'fail', 'message': str(error)})
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    report['check_counts'] = {kind: dict(Counter(case['checks'][kind]['outcome'] for case in report['cases'] if 'checks' in case)) for kind in ['emission', 'behavior', 'abi', 'kotlin_consumer']}
    report['finished_at'] = datetime.now(timezone.utc).isoformat()
    output.write_text(json.dumps(report, indent=2) + '\n')
    print('Report: ' + relative(output), flush=True)
    return 0 if report['status'] == 'completed' else 1


if __name__ == '__main__':
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)
