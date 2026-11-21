"""Run fixed Java callers against the exact verified outputs of the 33-file survey."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument('--survey', type=Path, required=True)
parser.add_argument('--callers', type=Path, required=True)
args = parser.parse_args()
root = Path.cwd()
jdk = Path(os.environ['JAVA_HOME']) / 'bin'
def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

provider = json.loads(args.survey.read_text())
assert provider['status'] == 'complete' and provider['counts'] == {'compiled':33}
cases = {
    'SyncUtils': 'SyncCheck', 'AppServices': 'ServicesCheck',
    'VersesUiModel': 'VersesCheck', 'MediaStateListener': 'MediaCheck',
    'SongDocumentSearch': 'SongCalls', 'SongDocumentText': 'SongCalls',
}
stamp = datetime.now(timezone.utc)
ident = stamp.strftime('%Y-%m-%dT%H%M%S%fZ') + '-androidbible-survey-jvm-callers'
evidence = Path('reports/correctness') / ident
work = Path('target/checks') / ident
evidence.mkdir()
work.mkdir(parents=True)
shutil.copyfile(__file__, evidence / 'replay.py')
record = dict(schema_version=1,kind='correctness',created_at=stamp.isoformat(),status='running',
              selection='Verify the Java callers against official and candidate outputs from the dependency-aware 33-file survey.',
              reference_compiler=provider['reference_compiler'],compiler=provider['compiler'],survey=str(args.survey),
              source_revision=provider['source_revision'],cases=[],commands=[],evidence=[str(args.survey),str(evidence/'replay.py')],input_hashes={str(args.survey):sha(args.survey)},
              reproduction_commands=[['python3',str(evidence/'replay.py'),'--survey',str(args.survey),'--callers',str(evidence)]])
def run(label,argv):
    result = subprocess.run(list(map(str,argv)),capture_output=True,timeout=120)
    path = evidence / (label+'.log')
    path.write_bytes(result.stdout+result.stderr)
    record['commands'].append(dict(label=label,argv=list(map(str,argv)),exit_code=result.returncode,log=str(path)))
    record['evidence'].append(str(path))
    if result.returncode:
        raise RuntimeError(f'{label} exited {result.returncode}, see {path}')

def checked_output(case,name):
    obs = case['providers'][name]
    assert case['outcome']=='compiled' and obs['outcome']=='compiled' and not case['classpath_overlap']
    directory = Path(obs['output'])
    emitted = {str(p.relative_to(directory)):sha(p) for p in directory.rglob('*.class')}
    assert emitted and emitted == {x['path']:x['sha256'] for x in obs['classes']}
    return directory

def verify_inputs():
    for cp in provider['classpath_copies']:
        path = Path(cp['copy'])
        original = provider['input_hashes'][cp['original']]
        hashes = {'sha256':sha(path)} if path.is_file() else {str(p.relative_to(path)):sha(p) for p in path.rglob('*') if p.is_file()}
        assert original == hashes, cp

def save():
    record['counts']=dict(Counter(item['outcome'] for item in record['cases']))
    evidence.with_suffix('.json').write_text(json.dumps(record,indent=2)+'\n')

verify_inputs()
for name in sorted(set(cases.values())):
    source=evidence/(name+'.java')
    shutil.copyfile(args.callers/source.name,source)
    record['input_hashes'][str(source)]=sha(source)
    record['evidence'].append(str(source))
    selected=[c for c in provider['cases'] if cases.get(Path(c['id']).stem)==name]
    assert len(selected)==sum(v==name for v in cases.values())
    common=selected[0]['classpath']
    assert all(c['classpath']==common for c in selected)
    output=work/name
    try:
        providers={p:[checked_output(c,p) for c in selected] for p in ['official','candidate']}
        def cp(parts):
            return os.pathsep.join(map(str,[*parts,*common]))
        run(name+'-compile',[jdk/'javac','-proc:none','-cp',cp(providers['official']),'-d',output,source])
        caller_hashes={str(p.relative_to(output)):sha(p) for p in output.rglob('*.class')}
        assert caller_hashes, 'javac emitted no caller classes'
        record['input_hashes'].update({str(output/p):digest for p,digest in caller_hashes.items()})
        for p,paths in providers.items():
            run(name+'-'+p,[jdk/'java','-Xverify:all','-cp',cp([output,*paths]),name])
            assert caller_hashes=={str(f.relative_to(output)):sha(f) for f in output.rglob('*.class')}, 'Fixed caller classes changed'
            for case in selected:
                checked_output(case,p)
        record['cases'].append(dict(id=name,outcome='pass',sources=[c['id'] for c in selected],caller_class_hashes=caller_hashes))
    except (AssertionError,ValueError,RuntimeError,subprocess.TimeoutExpired) as error:
        record['cases'].append(dict(id=name,outcome='failed',message=str(error)))
    save()
verify_inputs()
record['status']='pass' if all(c['outcome']=='pass' for c in record['cases']) else 'failed'
save()
print(ident,record['status'],record['counts'])
raise SystemExit(record['status']!='pass')
