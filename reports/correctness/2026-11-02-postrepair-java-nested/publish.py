from pathlib import Path
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import shutil
import zipfile

ROOT = Path.cwd()
WORK = ROOT / 'target/resume-postrepair-java-nested'
DEST = ROOT / 'reports/correctness/2026-11-02-postrepair-java-nested'
EXPECTED = '58ee34b1866ca7a9a5cd0ce838730856bef745907e159f93a2a4a8a00249b784'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relocate(value, key=''):
    if key in ['argv', 'reproduction_command']:
        return value
    if isinstance(value, str):
        for prefix in [str(WORK) + '/', str(WORK.relative_to(ROOT)) + '/']:
            if value.startswith(prefix):
                return str(DEST.relative_to(ROOT)) + '/' + value[len(prefix):]
        return value
    if isinstance(value, list):
        return [relocate(item, key) for item in value]
    if isinstance(value, dict):
        return {name: relocate(item, name) for name, item in value.items()}
    return value


def main():
    report = json.loads((WORK / 'report.json').read_text())
    assert report['status'] != 'running'
    assert report['compiler']['sha256'] == EXPECTED
    assert sha(Path(report['compiler']['path'])) == EXPECTED
    changed = [path for path, expected in report['frozen_input_hashes'].items() if not Path(path).is_file() or sha(Path(path)) != expected]
    if changed:
        raise RuntimeError('Frozen inputs changed: ' + str(changed))
    if DEST.exists() or DEST.with_suffix('.json').exists():
        raise RuntimeError('Historical report destination exists')
    DEST.mkdir(parents=True)
    for path in sorted(WORK.rglob('*')):
        if not path.is_file() or path.name == 'report.json' or path.is_relative_to(WORK / '__pycache__'):
            continue
        if path.suffix not in ['.txt', '.log', '.json', '.kt', '.java', '.py'] and path.name != 'source.zip':
            continue
        destination = DEST / path.relative_to(WORK)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, destination)
    shutil.copy2(WORK / 'report.json', DEST / 'raw-report.json')
    published = relocate(report)
    published['archived_from'] = str(WORK.relative_to(ROOT))
    published['created_at'] = datetime.now(timezone.utc).isoformat()
    published['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    published['original_run_created_at'] = report['created_at']
    published['raw_report'] = str((DEST / 'raw-report.json').relative_to(ROOT))
    published['raw_report_sha256'] = sha(WORK / 'report.json')
    published['source_bundle'] = {'path': str((DEST / 'source.zip').relative_to(ROOT)), 'sha256': sha(WORK / 'source.zip')}
    published['historical_evidence'] = ['reports/correctness/2026-11-02-resume-final-differential.json', 'reports/correctness/2026-11-02-resume-final-differential/upstream-full36.json']
    published['history_note'] = 'The earlier exact-binary 362e12fd full matrix recorded kotlin_consumes_nested_java as correctness_failure. This fresh proof uses combined repaired binary 58ee34b1 and preserves all historical reports. The earlier isolated V1 authored constructor-boundary return-exposure failure and successful V2 remain in the isolated repair history.'
    published['publication_scope'] = 'Source bundles and command logs only; generated class/JAR/compiler binaries excluded. Actual command arguments remain unchanged for replay.'
    with zipfile.ZipFile(DEST / 'source.zip') as archive:
        assert not any(Path(name).suffix in ['.class', '.jar', '.kotlin_module'] for name in archive.namelist())
    artifacts = [{'path': str(path.relative_to(ROOT)), 'sha256': sha(path), 'bytes': path.stat().st_size} for path in sorted(DEST.rglob('*')) if path.is_file()]
    (DEST / 'artifact-index.json').write_text(json.dumps({'schema_version': 1, 'files': artifacts}, indent=2) + '\n')
    DEST.with_suffix('.json').write_text(json.dumps(published, indent=2) + '\n')
    print(json.dumps({'report': str(DEST.with_suffix('.json').relative_to(ROOT)), 'status': report['status'], 'counts': published['counts'], 'integrity': report['input_integrity'], 'files': len(artifacts)}, indent=2))


if __name__ == '__main__':
    main()
