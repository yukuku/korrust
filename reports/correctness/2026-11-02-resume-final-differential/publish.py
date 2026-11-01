import hashlib
import json
from pathlib import Path
import shutil
import zipfile
from collections import Counter
from datetime import datetime, timezone

ROOT = Path.cwd()
WORK = ROOT / 'target/resume-final-differential'
DESTINATION = ROOT / 'reports/correctness/2026-11-02-resume-final-differential'
PREFIX = 'reports/correctness/2026-11-02-resume-final-differential'
EXPECTED = '362e12fd88ea493a1132d85a618a656f916242ef7ba4f48d002ea64fb054105b'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relocate(value, old, new, key=''):
    if key in ['argv', 'reproduction_command']:
        return value
    if isinstance(value, str):
        for prefix in [str(ROOT / old) + '/', old + '/']:
            if value.startswith(prefix):
                return new + '/' + value[len(prefix):]
        return value
    if isinstance(value, list):
        return [relocate(item, old, new, key) for item in value]
    if isinstance(value, dict):
        return {name: relocate(item, old, new, name) for name, item in value.items()}
    return value


def main():
    if DESTINATION.exists() or DESTINATION.with_suffix('.json').exists():
        raise RuntimeError('Historical destination exists')
    freeze = json.loads((WORK / 'source-freeze.json').read_text())
    changed = [name for name, expected in freeze['files'].items() if not (ROOT / name).is_file() or sha(ROOT / name) != expected]
    current = {str(path.relative_to(ROOT)) for directory in ['src', 'tests', 'tools'] for path in (ROOT / directory).rglob('*') if path.is_file() and '__pycache__' not in path.parts and path.suffix in ['.rs', '.py', '.json', '.kt', '.java']}
    added = sorted(current - freeze['files'].keys())
    binary = ROOT / freeze['compiler']['path']
    if sha(binary) != EXPECTED:
        changed.append(freeze['compiler']['path'])
    reports = {}
    for suite in ['upstream-full36', 'value-classes', 'suspend-foundation']:
        path = WORK / suite / 'report.json'
        report = json.loads(path.read_text())
        if report['status'] == 'running':
            raise RuntimeError(suite + ' is still running')
        if report['compiler']['sha256'] != EXPECTED:
            raise RuntimeError(suite + ' compiler mismatch')
        reports[suite] = report
    DESTINATION.mkdir(parents=True)
    for name in ['source-freeze.json', 'source.zip', 'publish.py', 'observations.json', 'javaann-dependency.txt']:
        shutil.copy2(WORK / name, DESTINATION / name)
    cases, suites = [], []
    for suite, report in reports.items():
        source = WORK / suite
        target = DESTINATION / suite
        target.mkdir()
        for path in sorted(source.rglob('*')):
            if not path.is_file() or path.name == 'report.json':
                continue
            if path.suffix not in ['.json', '.txt', '.kt', '.java', '.log'] and path.name not in ['source.zip', 'inputs.zip']:
                continue
            relative = path.relative_to(source)
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(path, destination)
            if path.name.endswith('.source-map.json'):
                destination.write_text(json.dumps(relocate(json.loads(path.read_text()), str(source.relative_to(ROOT)), str(target.relative_to(ROOT))), indent=2) + '\n')
        shutil.copy2(source / 'report.json', target / 'raw-report.json')
        published = relocate(report, str(source.relative_to(ROOT)), str(target.relative_to(ROOT)))
        published['archived_from'] = str(source.relative_to(ROOT))
        (DESTINATION / (suite + '.json')).write_text(json.dumps(published, indent=2) + '\n')
        transcript = WORK / (suite + '.log')
        if transcript.exists():
            shutil.copy2(transcript, DESTINATION / transcript.name)
        for case in published['cases']:
            cases.append({**case, 'id': suite + '/' + case['id'], 'source_suite': suite, 'suite_case_id': case['id']})
        suites.append({'id': suite, 'status': report['status'], 'counts': dict(Counter(case['outcome'] for case in report['cases'])), 'input_integrity': report.get('input_integrity'), 'report': PREFIX + '/' + suite + '.json', 'raw_report': PREFIX + '/' + suite + '/raw-report.json', 'raw_report_sha256': sha(source / 'report.json'), 'error': report.get('error')})
    rejected = [{'id': case['id'], 'outcome': case['outcome']} for case in cases if case['outcome'] not in ['pass', 'invalid_kotlin', 'invalid_rejected', 'unsupported_language', 'harness_unsupported', 'excluded_target']]
    integrity = {'status': 'changed' if changed or added else 'unchanged', 'changed_paths': changed, 'added_source_paths': added}
    summary = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'invalidated' if changed or added else 'failed' if rejected or any(report['status'] != 'complete' for report in reports.values()) else 'complete', 'compiler': freeze['compiler'], 'environment': {'JAVA_HOME': freeze['java_home']}, 'scope': 'Final exact-binary regression: complete unchanged 36-case upstream feature matrix plus value-class and suspend-foundation suites. No source edits, full-app, Android runtime or performance claim.', 'input_integrity': integrity, 'source_freeze': PREFIX + '/source-freeze.json', 'source_bundle': {'path': PREFIX + '/source.zip', 'sha256': sha(WORK / 'source.zip')}, 'root_source_matches_frozen_compiler_checkout': freeze['root_source_matches_frozen_compiler_checkout'], 'observations': json.loads((WORK / 'observations.json').read_text()), 'suites': suites, 'cases': cases, 'counts': dict(Counter(case['outcome'] for case in cases)), 'unaccepted_outcomes': rejected, 'limitations': ['Passes are executable case/group outcomes, not a language-conformance percentage.', 'Original unsupported-language, harness, excluded-target and oracle/failure classifications are preserved.', 'Published files omit generated class/JAR/compiler binaries; source bundles, command logs and fixed-caller/dependency hashes preserve provenance.', 'Exact executed command arguments remain unchanged in archived report copies; relocated evidence links refer to published artifacts.'], 'reproduction_commands': [['env', 'JAVA_HOME=' + freeze['java_home'], 'python3', 'tools/upstream_jvm.py', '--matrix', 'tools/upstream_feature_matrix.json', '--candidate', freeze['compiler']['path'], '--output', 'target/resume-final-differential-replay/upstream-full36'], ['env', 'JAVA_HOME=' + freeze['java_home'], 'python3', 'tools/check_value_classes.py', '--candidate', freeze['compiler']['path'], '--output', 'target/resume-final-differential-replay/value-classes'], ['env', 'JAVA_HOME=' + freeze['java_home'], 'python3', 'tools/check_suspend_foundation.py', '--candidate', freeze['compiler']['path'], '--output', 'target/resume-final-differential-replay/suspend-foundation']]}
    for archive in DESTINATION.rglob('*.zip'):
        with zipfile.ZipFile(archive) as bundle:
            assert not any(Path(name).suffix in ['.class', '.jar', '.kotlin_module'] for name in bundle.namelist()), archive
    evidence = [{'path': str(path.relative_to(ROOT)), 'sha256': sha(path), 'bytes': path.stat().st_size} for path in sorted(DESTINATION.rglob('*')) if path.is_file()]
    (DESTINATION / 'artifact-index.json').write_text(json.dumps({'schema_version': 1, 'files': evidence}, indent=2) + '\n')
    DESTINATION.with_suffix('.json').write_text(json.dumps(summary, indent=2) + '\n')
    (WORK / 'publication.json').write_text(json.dumps({'report': str(DESTINATION.with_suffix('.json').relative_to(ROOT)), 'status': summary['status'], 'counts': summary['counts'], 'integrity': integrity}, indent=2) + '\n')
    print(json.dumps({'status': summary['status'], 'counts': summary['counts'], 'suites': suites, 'integrity': integrity, 'files': len(evidence)}, indent=2))


if __name__ == '__main__':
    main()
