import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

root = Path('/workspace/korrust')
parser = argparse.ArgumentParser()
parser.add_argument('--candidate', type=Path, required=True)
parser.add_argument('--snapshot', type=Path, required=True)
args = parser.parse_args()
binary = args.candidate.resolve()
ident = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ') + '-next10-formatted-verse-text'
evidence = root / 'reports/correctness' / ident
work = root / 'target/checks' / ident
evidence.mkdir()
work.mkdir(parents=True)
source = root / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/FormattedVerseText.kt'
stdlib = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
oracle = root / 'references/kotlinc/bin/kotlinc'
jdk = Path(os.environ['JAVA_HOME'])
caller = evidence / 'FormattedVerseTextCaller.java'
caller_source = Path(__file__).with_name('FormattedVerseTextCaller.java')
shutil.copy2(caller_source, caller)
shutil.copy2(Path(__file__), evidence / 'replay.py')
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
inputs = [source, binary, stdlib, root / 'references/kotlinc/lib/kotlin-compiler.jar', args.snapshot.resolve(), jdk / 'jmods/java.base.jmod', caller, evidence / 'replay.py']
frozen = {str(path.relative_to(root)) if path.is_relative_to(root) else str(path): sha(path) for path in inputs}
report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'compiler': {'path': str(binary), 'sha256': sha(binary)}, 'reference_compiler': json.loads((root / 'references/compiler.json').read_text()), 'frozen_input_hashes': frozen, 'cases': [], 'commands': [], 'evidence': [], 'selection': 'Unchanged AndroidBible FormattedVerseText.kt, newly accepted in the next-ten milestone. Fixed Java caller compiles against official output and runs against both providers with JVM verification and 18 independent assertions covering nulls, empty input, identity, force/default overloads, marker removal, whitespace, UTF-16, and original trailing-marker exception behavior.', 'limitations': ['Single unchanged file and tested behavior only; no full Android module/application or performance claim.'], 'reproduction_commands': [['python3', str((evidence / 'replay.py').relative_to(root)), '--candidate', str(binary), '--snapshot', str(args.snapshot.resolve())]]}
def run(label, command):
    index = len(report['commands'])
    result = subprocess.run([str(item) for item in command], cwd=root, capture_output=True, text=True, timeout=600)
    out = evidence / f'{index:02d}-stdout.txt'
    err = evidence / f'{index:02d}-stderr.txt'
    out.write_text(result.stdout)
    err.write_text(result.stderr)
    report['commands'].append({'label': label, 'argv': [str(item) for item in command], 'exit_code': result.returncode, 'stdout': str(out.relative_to(root)), 'stderr': str(err.relative_to(root))})
    report['evidence'].extend([str(out.relative_to(root)), str(err.relative_to(root))])
    if result.returncode:
        raise RuntimeError(label + ': ' + result.stderr)
    return result.stdout
try:
    run('Compile unchanged official source', [oracle, source, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'next10_formatter', '-d', work / 'official'])
    report['cases'].append({'id': 'official_compile', 'outcome': 'pass'})
    run('Compile unchanged candidate source', [binary, source, '-cp', stdlib, '--module-name', 'next10_formatter', '-d', work / 'candidate'])
    report['cases'].append({'id': 'candidate_compile', 'outcome': 'pass'})
    run('Compile fixed Java caller against official output', [jdk / 'bin/javac', '-cp', f'{work}/official:{stdlib}', '-d', work / 'caller', caller])
    caller_hashes = {str(path.relative_to(work)): sha(path) for path in (work / 'caller').rglob('*.class')}
    outputs = {}
    signatures = {}
    for provider in ('official', 'candidate'):
        outputs[provider] = run('Verify Java caller against ' + provider, [jdk / 'bin/java', '-Xverify:all', '-cp', f'{work}/caller:{work}/{provider}:{stdlib}', 'FormattedVerseTextCaller'])
        if not outputs[provider].strip().endswith('OK:18'):
            raise RuntimeError('Missing independent assertions')
        report['cases'].append({'id': 'runtime/' + provider, 'outcome': 'pass', 'assertions': 18, 'stdout': outputs[provider]})
        text = run('Inspect public ABI ' + provider, [jdk / 'bin/javap', '-public', '-s', '-classpath', work / provider, 'yuku.alkitab.base.util.FormattedVerseText'])
        lines = text.splitlines()
        signatures[provider] = sorted((lines[i - 1].strip(), line.strip()) for i, line in enumerate(lines) if line.strip().startswith('descriptor:'))
    if outputs['official'] != outputs['candidate'] or signatures['official'] != signatures['candidate']:
        raise RuntimeError('Differential runtime or public ABI mismatch')
    report['cases'].append({'id': 'runtime_and_public_abi_agree', 'outcome': 'pass'})
    if caller_hashes != {str(path.relative_to(work)): sha(path) for path in (work / 'caller').rglob('*.class')}:
        raise RuntimeError('Fixed caller changed')
    report['cases'].append({'id': 'fixed_caller_integrity', 'outcome': 'pass', 'class_hashes': caller_hashes})
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
