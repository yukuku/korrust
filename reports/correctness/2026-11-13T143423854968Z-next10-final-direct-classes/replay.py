from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

root = Path('/workspace/korrust')
old_id = '2026-11-13T143013272305Z-next10-generics'
old = json.loads((root / 'reports/correctness' / (old_id + '-direct-classes.json')).read_text())
stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ')
ident = stamp + '-next10-final-direct-classes'
evidence = root / 'reports/correctness' / ident
work = root / 'target/checks' / ident
evidence.mkdir()
work.mkdir(parents=True)
binary = root / 'target/release/korrust'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
inputs = [binary, root / 'reports/correctness/2026-11-13-next-ten-final/frozen-inputs.json', root / 'references/kotlinc/lib/kotlin-stdlib.jar', root / 'references/kotlinc/lib/kotlin-compiler.jar']
for name in ('Direct.kt', 'Consumer.kt', 'Calls.java'):
    shutil.copy2(root / 'reports/correctness' / old_id / name, evidence / name)
    inputs.append(evidence / name)
shutil.copy2(Path(__file__), evidence / 'replay.py')
inputs.append(evidence / 'replay.py')
frozen = {str(path.relative_to(root)): sha(path) for path in inputs}
report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'compiler': {'path': str(binary.relative_to(root)), 'sha256': sha(binary)}, 'reference_compiler': old['reference_compiler'], 'frozen_input_hashes': frozen, 'commands': [], 'cases': [], 'evidence': [], 'scope': 'Final combined compiler replay of direct source-class declaration variance and compiled class metadata proof. Fixed Java caller asserts source and Kotlin consumer results, compiled once against official output; all executions use -Xverify:all.', 'limitations': old['limitations'], 'reproduction_commands': [['python3', str((evidence / 'replay.py').relative_to(root))]]}
old_root = '/workspace/work/next10/generics'
old_binary = old['compiler']['binary']
try:
    for index, command in enumerate(old['commands']):
        args = [argument.replace(old_root + '/reports/correctness/' + old_id, str(evidence)).replace(old_root + '/target/checks/' + old_id, str(work)).replace(old_binary, str(binary)).replace(old_root, str(root)) for argument in command['argv']]
        result = subprocess.run(args, cwd=root, text=True, capture_output=True, timeout=600)
        out = evidence / f'{index:03d}-stdout.txt'
        err = evidence / f'{index:03d}-stderr.txt'
        out.write_text(result.stdout)
        err.write_text(result.stderr)
        report['commands'].append({'label': command['label'], 'argv': args, 'exit_code': result.returncode, 'stdout': str(out.relative_to(root)), 'stderr': str(err.relative_to(root))})
        report['evidence'].extend([str(out.relative_to(root)), str(err.relative_to(root))])
        if result.returncode != 0:
            raise RuntimeError(command['label'] + ': ' + result.stderr)
        if '-Xverify:all' in args:
            if result.stdout.strip() != 'OK':
                raise RuntimeError('Unexpected verification output: ' + result.stdout)
            report['cases'].append({'id': command['label'], 'outcome': 'pass', 'stdout': result.stdout.strip()})
    report['status'] = 'complete'
except Exception as error:
    report['status'] = 'failed'
    report['cases'].append({'id': 'harness_failure', 'outcome': 'correctness_failure', 'error': str(error)})
changed = [name for name, digest in frozen.items() if sha(root / name) != digest]
report['cases'].append({'id': 'frozen_inputs', 'outcome': 'correctness_failure' if changed else 'pass', 'changed': changed})
if changed:
    report['status'] = 'invalidated'
report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
path = evidence.with_suffix('.json')
path.write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'report': str(path.relative_to(root)), 'status': report['status'], 'counts': report['counts']}))
raise SystemExit(report['status'] != 'complete')
