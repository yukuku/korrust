"""Run pinned AndroidBible tests against two compiler outputs for one unchanged source."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import urllib.request

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('app', choices=('OldVerseTextDecoder', 'AudioHttpErrorBody', 'QueryTokenizer'))
parser.add_argument('--candidate', required=True, type=Path)
args = parser.parse_args()
root = Path.cwd()
jdk = Path(os.environ['JAVA_HOME']) / 'bin'
candidate = args.candidate.resolve()
oracle = root / 'references/kotlinc/bin/kotlinc'
stdlib = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ')
ident = f'{stamp}-androidbible-{args.app.lower()}-junit'
out = root / 'reports/correctness' / ident
work = root / 'target/checks' / ident
out.mkdir()
work.mkdir(parents=True)
shutil.copyfile(__file__, out/'replay.py')
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
def rel(p):
    return str(p.relative_to(root)) if p.is_relative_to(root) else str(p)
android = root / 'benchmarks/sources/androidbible'
def source(name):
    found = list(android.rglob(name))
    assert len(found) == 1, (name,found)
    return found[0]
def cp(*paths):
    return os.pathsep.join(map(str,paths))
cases = []
commands = []
evidence = [rel(out/'replay.py')]
def run(label, *argv):
    result = subprocess.run(list(map(str,argv)), stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, timeout=300)
    log = out/f'{len(commands):02d}-{label}.log'
    log.write_text(result.stdout)
    commands.append(dict(label=label, argv=list(map(str,argv)), exit_code=result.returncode, log=rel(log)))
    evidence.append(rel(log))
    if result.returncode:
        raise RuntimeError(result.stdout[-2000:])
    return result.stdout
deps = work/'deps'
deps.mkdir()
jars=[]
for suffix in ['junit/junit/4.13.2/junit-4.13.2.jar','org/hamcrest/hamcrest-core/1.3/hamcrest-core-1.3.jar']:
    jar=work/Path(suffix).name
    urllib.request.urlretrieve('https://repo.maven.apache.org/maven2/'+suffix, jar)
    jars.append(jar)
test = source(args.app+'Test.kt')
inputs = [source(args.app+'.kt')]
if args.app=='OldVerseTextDecoder':
    inputs.append(source('VerseTextDecoder.kt'))
bintex = sorted((android/'Bintex/src/main/java/yuku/bintex').glob('*.java'))
dependency_sources = [] if args.app=='AudioHttpErrorBody' else [*bintex,source('Utf8Decoder.kt')]
tracked=[out/'replay.py',candidate,stdlib,root/'references/kotlinc/lib/kotlin-compiler.jar',jdk.parent/'jmods/java.base.jmod',*inputs,test,*jars,*dependency_sources]
digests={rel(p):sha(p) for p in tracked}
report=dict(schema_version=1,kind='correctness',created_at=stamp,selection=f'Unchanged AndroidBible {args.app} and original JUnit 4 tests. Kotlin test bytecode built once against official provider, then executed separately with each provider.', compiler=dict(path=rel(candidate),sha256=sha(candidate)),reference_compiler=json.loads((root/'references/compiler.json').read_text()), source_revision=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),frozen_input_hashes=digests,commands=commands,evidence=evidence,cases=cases,reproduction_commands=[['python3',rel(out/'replay.py'),args.app,'--candidate',rel(candidate)]])
try:
    kotlin_options=['-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','app']
    if dependency_sources:
        run('java-dependencies',jdk/'javac','-d',deps,*bintex)
        run('kotlin-dependencies',oracle,source('Utf8Decoder.kt'),*kotlin_options,'-classpath',deps,'-d',deps)
    run('official-compile',oracle,*inputs,*kotlin_options,'-classpath',deps,'-d',work/'official')
    run('candidate-compile',candidate,*inputs,'-cp',cp(stdlib,deps),'--module-name','app','-d',work/'candidate')
    run('fixed-test-compile',oracle,test,*kotlin_options,'-classpath',cp(work/'official',deps,*jars),'-Xfriend-paths='+str(work/'official'),'-d',work/'tests')
    package=test.read_text().splitlines()[0].split()[1]
    for provider in ['official','candidate']:
        output=run(provider+'-junit',jdk/'java','-Xverify:all','-cp',cp(work/'tests',work/provider,deps,*jars,stdlib),'org.junit.runner.JUnitCore',package+'.'+test.stem)
        assert 'OK (' in output
        cases.append(dict(id=provider+'-junit',outcome='pass',summary=output.strip().splitlines()[-1]))
    report['status']='pass'
except Exception as e:
    cases.append(dict(id='execution',outcome='failed',message=str(e)))
    report['status']='failed'
if any(sha(p)!=digests[rel(p)] for p in tracked):
    report['status']='invalidated'
report['counts']=dict(Counter(c['outcome'] for c in cases))
out.with_suffix('.json').write_text(json.dumps(report,indent=2)+'\n')
print(ident,report['status'],flush=True)
raise SystemExit(report['status']!='pass')
