#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=Path(__file__).resolve().parents[1])
    args = parser.parse_args()
    root = args.source_root.resolve()
    binaries = {name: getattr(args, name).resolve() for name in ('baseline', 'candidate')}
    stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ')
    ident = stamp + '-next10-app-assessment'
    evidence = root / 'reports/correctness' / ident
    work = root / 'target/checks' / ident
    evidence.mkdir(parents=True)
    work.mkdir(parents=True)
    stdlib = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
    jdk_base = Path(os.environ['JAVA_HOME']) / 'jmods/java.base.jmod'
    sources = []
    for path in sorted((root / 'benchmarks/sources/androidbible').rglob('*.kt')):
        lines = path.read_text().splitlines()
        imports = [line for line in lines if line.startswith('import ')]
        if len(lines) <= 180 and '/test/' not in str(path) and all(
            line.startswith(('import kotlin.', 'import java.', 'import javax.')) for line in imports
        ):
            sources.append(path)
    inputs = [*sources, stdlib, jdk_base, root / 'tools/check_next10_app.py', *binaries.values()]
    frozen = {str(path.relative_to(root)) if path.is_relative_to(root) else str(path): sha(path) for path in inputs}
    report = {
        'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(),
        'status': 'running', 'selection': 'Unchanged AndroidBible production Kotlin files with at most 180 lines and only Kotlin/JDK explicit imports, compiled individually with the pinned standard library and JDK. Same-package/application dependencies are not supplied. Compile acceptance only; rejection is not classified as invalid Kotlin or a correctness failure.',
        'compiler': {name: {'path': str(path), 'sha256': sha(path)} for name, path in binaries.items()},
        'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
        'environment': {'JAVA_HOME': os.environ.get('JAVA_HOME')}, 'frozen_input_hashes': frozen,
        'cases': [], 'commands': [], 'evidence': [],
        'limitations': ['This survey does not establish runtime, Kotlin metadata, Android-device, full-module or whole-application compatibility. QueryTokenizer runtime and fixed Java caller evidence is recorded separately by the operators harness.'],
        'reproduction_commands': [['python3', 'tools/check_next10_app.py', '--baseline', str(binaries['baseline']), '--candidate', str(binaries['candidate'])]],
    }
    for index, source in enumerate(sources):
        case = {'id': str(source.relative_to(root)), 'sha256': sha(source), 'providers': {}}
        for provider, binary in binaries.items():
            command = [str(binary), str(source), '-cp', str(stdlib), '-d', str(work / provider / str(index))]
            log = evidence / f'{index:02d}-{provider}.log'
            try:
                result = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
                log.write_text(result.stdout)
                observation = {'outcome': 'compiled' if result.returncode == 0 else 'rejected', 'exit_code': result.returncode}
            except subprocess.TimeoutExpired as error:
                output = error.stdout or b''
                log.write_text(output.decode(errors='replace') if isinstance(output, bytes) else output)
                observation = {'outcome': 'timeout', 'exit_code': None}
            relative = str(log.relative_to(root))
            observation['log'] = relative
            case['providers'][provider] = observation
            report['commands'].append({'argv': command, 'exit_code': observation['exit_code'], 'log': relative})
            report['evidence'].append(relative)
        old, new = (case['providers'][provider]['outcome'] for provider in ('baseline', 'candidate'))
        case['outcome'] = 'regression' if old == 'compiled' and new != 'compiled' else 'newly_compiled' if old != 'compiled' and new == 'compiled' else 'unchanged'
        report['cases'].append(case)
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    report['provider_counts'] = {provider: dict(Counter(case['providers'][provider]['outcome'] for case in report['cases'])) for provider in binaries}
    changed = [name for name, digest in frozen.items() if sha(root / name) != digest]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    report['status'] = 'invalidated' if changed else 'failed' if report['counts'].get('regression') or any(case['providers'][provider]['outcome'] == 'timeout' for case in report['cases'] for provider in binaries) else 'complete'
    report_path = evidence.with_suffix('.json')
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    subprocess.run(['python3', str(root / 'tools/reports.py')], cwd=root, check=True)
    print(json.dumps({'report': str(report_path), 'status': report['status'], 'counts': report['counts'], 'provider_counts': report['provider_counts']}))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
