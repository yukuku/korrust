#!/usr/bin/env python3
import hashlib
import json
import re
import subprocess
import tempfile
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

from reports import generate

ROOT = Path(__file__).resolve().parents[1]
PATTERNS = {
    'imports': r'\bimport\s',
    'classes': r'\bclass\s',
    'data_classes': r'\bdata\s+class\s',
    'interfaces': r'\binterface\s',
    'objects': r'\bobject\b',
    'nullable_type_or_safe_call': r'\?',
    'lambda_or_block_arrow': r'->',
    'when': r'\bwhen\s*[({]',
    'for': r'\bfor\s*\(',
    'suspend': r'\bsuspend\s',
    'internal': r'\binternal\s',
    'String_type_or_name': r'\bString\b',
}
MASK = re.compile(r'//[^\n]*|/\*.*?\*/|""".*?"""|"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'', re.S)


def main():
    now = datetime.now(timezone.utc)
    source_root = ROOT / 'benchmarks/sources/androidbible'
    compiler = ROOT / 'target/release/korrust'
    files = sorted(source_root.rglob('*.kt'))
    observations = {name: [] for name in PATTERNS}
    observations['string_literals'] = []
    cases = []
    with tempfile.TemporaryDirectory(prefix='korrust-corpus-') as directory:
        for index, path in enumerate(files):
            source = path.read_text()
            relative = str(path.relative_to(ROOT))
            masked = MASK.sub(' ', source)
            for name, pattern in PATTERNS.items():
                if re.search(pattern, masked):
                    observations[name].append(relative)
            if any(match.group().startswith('"') for match in MASK.finditer(source)):
                observations['string_literals'].append(relative)
            result = subprocess.run([str(compiler), str(path), '-d', str(Path(directory) / str(index))], text=True, capture_output=True, timeout=10)
            cases.append({'id': relative, 'outcome': 'emitted' if result.returncode == 0 else 'rejected',
                          'source_sha256': hashlib.sha256(path.read_bytes()).hexdigest(), 'diagnostic': result.stderr.strip()})
    report = {
        'schema_version': 1, 'kind': 'corpus', 'created_at': now.isoformat(), 'status': 'completed',
        'selection': 'All .kt files in the pinned AndroidBible snapshot, including tests and tools; excludes .kts and generated sources.',
        'reason': 'Standalone compilation probes with no dependency classpath supplied; imported Java dependencies are unavailable to this scan. Emission is not a correctness pass. Rejection records only the first blocker, not every missing feature. Syntax observations are regex heuristics; nested comments and template expressions are not fully parsed. Categories overlap and are not coverage percentages.',
        'compiler': {'binary_sha256': hashlib.sha256(compiler.read_bytes()).hexdigest()},
        'inputs': {'manifest': 'benchmarks/sources.json'},
        'commands': [['python3', 'tools/corpus.py']],
        'counts': dict(Counter(case['outcome'] for case in cases)),
        'observations': [{'construct': name, 'files': len(paths), 'examples': paths[:5]} for name, paths in observations.items()],
        'cases': cases,
    }
    path = ROOT / 'reports/correctness' / (now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-androidbible-corpus.json')
    path.write_text(json.dumps(report, indent=2) + '\n')
    print(path.relative_to(ROOT))
    print(json.dumps(report['counts']))
    for row in report['observations']:
        print(row['construct'], row['files'])
    generate(ROOT)


if __name__ == '__main__':
    main()
