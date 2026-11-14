import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[3]
parser = argparse.ArgumentParser()
parser.add_argument('--candidate', type=Path, required=True)
parser.add_argument('--provider-report', type=Path, required=True)
args = parser.parse_args()
logs = Path(__file__).resolve().parent
source = logs / 'JvmStaticReflectionCaller.java'
provider_report = args.provider_report.resolve()
provider_data = json.loads(provider_report.read_text())
base = ROOT / 'target/checks' / provider_report.stem
work = ROOT / 'target/checks' / logs.name
work.mkdir(parents=True)
standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
candidate = args.candidate.resolve()
java = Path(os.environ['JAVA_HOME']) / 'bin'
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
relative = lambda path: str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
inputs = [candidate, standard, provider_report, source, Path(__file__).resolve()]
for provider in ['official', 'candidate', 'dependencies']:
    inputs += sorted((base / provider).rglob('*.class'))
frozen = {str(path): sha(path) for path in inputs}
report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'compiler': {'path': relative(candidate), 'sha256': sha(candidate)}, 'reference_compiler': provider_data['reference_compiler'], 'selection': 'Supplementary exact reflection counts for canonical RUNTIME JvmStatic on all 21 annotated nullable fixture methods: nine methods must have exactly one runtime JvmStatic, twelve must have none. Java caller compiles once against official output and is reused unchanged with verification against both frozen providers.', 'frozen_input_hashes': {relative(Path(path)): value for path, value in frozen.items()}, 'provider_report': relative(provider_report), 'cases': [], 'commands': [], 'evidence': [relative(source), relative(Path(__file__).resolve())], 'reproduction_commands': [['python3', relative(Path(__file__).resolve()), '--candidate', relative(candidate), '--provider-report', relative(provider_report)]]}
def cp(*paths):
    return os.pathsep.join(map(str, paths))
def run(label, command):
    result = subprocess.run(list(map(str, command)), cwd=ROOT, capture_output=True, text=True, timeout=180)
    index = len(report['commands'])
    entry = {'label': label, 'argv': list(map(str, command)), 'returncode': result.returncode}
    for stream in ['stdout', 'stderr']:
        path = logs / f'{index:02d}-{stream}.txt'
        path.write_text(getattr(result, stream))
        entry[stream] = relative(path)
    report['commands'].append(entry)
    if result.returncode:
        raise RuntimeError(label + ': ' + result.stderr)
    return result.stdout
try:
    assert sha(candidate) == provider_data['compiler']['sha256']
    run('JDK identity', [java / 'java', '-version'])
    run('Compile reflection caller once against official provider', [java / 'javac', '-cp', cp(base / 'official', base / 'dependencies', standard), '-d', work / 'caller', source])
    callers = {str(path): sha(path) for path in (work / 'caller').rglob('*.class')}
    report['cases'].append({'id': 'fixed_official_compilation', 'outcome': 'pass', 'source_sha256': sha(source), 'caller_class_hashes': {relative(Path(path)): value for path, value in callers.items()}})
    for provider in ['official', 'candidate']:
        output = run('Exact JvmStatic reflection count against ' + provider, [java / 'java', '-Xverify:all', '-cp', cp(work / 'caller', base / provider, base / 'dependencies', standard), 'nullableannotations.JvmStaticReflectionCaller'])
        assert output == 'OK:21:9\n', output
        report['cases'].append({'id': 'reflection/' + provider, 'outcome': 'pass', 'checked_methods': 21, 'exactly_one_runtime_jvmstatic': 9, 'no_runtime_jvmstatic': 12})
    assert callers == {str(path): sha(path) for path in (work / 'caller').rglob('*.class')}
    report['cases'].append({'id': 'fixed_caller_hashes_preserved', 'outcome': 'pass'})
    report['status'] = 'pass'
except Exception as error:
    report['status'] = 'fail'
    report['cases'].append({'id': 'harness', 'outcome': 'fail', 'error': str(error)})
changed = [path for path, value in frozen.items() if sha(Path(path)) != value]
report['cases'].append({'id': 'frozen_inputs', 'outcome': 'fail' if changed else 'pass', 'changed': changed})
if changed:
    report['status'] = 'fail'
report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
path = logs.with_suffix('.json')
path.write_text(json.dumps(report, indent=2) + '\n')
print(path)
print(json.dumps(report['counts']))
raise SystemExit(report['status'] != 'pass')
