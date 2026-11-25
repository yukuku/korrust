from pathlib import Path
import concurrent.futures, json, hashlib, os, subprocess, zipfile, sys
ROOT=Path('/workspace/korrust')
sys.path.insert(0,str(ROOT/'tools'))
from check_androidbible_survey import class_info
from check_androidbible_class_listener import ownership
W=Path('/workspace/work/all400-nov25')
SNAP=ROOT/'benchmarks/sources/androidbible'
STD=str(ROOT/'references/kotlinc/lib/kotlin-stdlib.jar')
BIN=str(Path(sys.argv[1]).resolve())
mode=sys.argv[2]
assert mode in ('tests','production')
OUT=Path(os.environ.get('RECHECK_ROOT','/workspace/work/mr-const'))/mode
OUT.mkdir(exist_ok=True,parents=True)
sources=sorted(SNAP/p for p in subprocess.check_output(['git','ls-files','--','*.kt'],cwd=SNAP,text=True).splitlines())
assert len(sources)==400
pins=json.loads((ROOT/'benchmarks/source-files.sha256.json').read_text())['androidbible']
models={}
for f in W.glob('Alkitab*.json'):models.update(json.loads(f.read_text()))
known=json.loads(Path('/workspace/work/survey33/classpaths.json').read_text())['cases']
def varint(n):
 a=bytearray()
 while n>=128:a.append((n&127)|128);n>>=7
 return bytes(a)+bytes([n])
def fields(data):
 p=0
 def num():
  nonlocal p
  n=shift=0
  while True:
   b=data[p];p+=1;n|=(b&127)<<shift
   if b<128:return n
   shift+=7
 while p<len(data):
  start=p;t=num();assert t%8==2,t
  n=num();v=data[p:p+n];p+=n;yield t,v,data[start:p]
def module_index(b,excluded):
 end=4+4*int.from_bytes(b[:4],'big')+4
 out=bytearray(b[:end])
 for t,payload,raw in fields(b[end:]):
  if t!=10:out.extend(raw);continue
  parts=list(fields(payload));assert all(t2 in [10,18] for t2,_,_ in parts)
  pkg=next(v.decode().replace('.','/') for t2,v,_ in parts if t2==10)
  retained=b''.join(r for t2,v,r in parts if t2!=18 or pkg+'/'+v.decode()+'.class' not in excluded)
  out.extend(varint(t)+varint(len(retained))+retained)
 return bytes(out)
def original(key):
 m=models[key];entries={};owners={}
 for d in m['outputs']:
  for f in sorted(Path(d).rglob('*')):
   if not f.is_file():continue
   n=f.relative_to(d).as_posix();b=f.read_bytes()
   if n in entries:assert entries[n]==b;continue
   entries[n]=b
   if n.endswith('.class'):owners.setdefault(class_info(b)['source_file'],set()).add(n)
 return entries,owners
library={key:original(key) for key in models}
def run_job(item):
 i,f=item
 rel=f.relative_to(SNAP).as_posix();module,_,source_set,*_=Path(rel).parts
 key='tools' if module=='tools' else module+'/'+source_set
 src_hash=hashlib.sha256(f.read_bytes()).hexdigest();assert pins[rel]==src_hash
 row={'path':rel,'group':key,'source_sha256':src_hash,'attempts':[]}
 sib=OUT/'temp-jars'/f'{i}.jar'
 if key=='tools':
  cp=[str(W/'converter'),*models['Alkitab/main']['cp']];friends=[];excluded=set()
 else:
  m=models[key];cp=m['cp'];entries,owners=library[key];excluded=owners.get(f.name,set())
  with zipfile.ZipFile(sib,'w') as z:
   for n,b in entries.items():
    if n not in excluded:z.writestr(n,module_index(b,excluded) if n.endswith('.kotlin_module') else b)
  friends=[q for q in cp if key.endswith('/test') and ('/compile_app_classes_jar/' in q or '/compile_library_classes_jar/' in q) and '/project/'+module+'/' in q]
 cp=list(dict.fromkeys([q for q in cp if Path(q).exists()]+[STD]))
 row['excluded_classes']=sorted(excluded)
 cp_cases=[('resolved',cp+([str(sib)] if key!='tools' else []),friends+([str(sib)] if key!='tools' else []))]
 earlier=known.get(str(f.relative_to(ROOT)))
 if earlier:
  cp_cases.append(('survey',[str(Path(p) if Path(p).is_absolute() else ROOT/p) for p in earlier['classpath']]+[STD],earlier.get('friend_paths',[])))
 cp_cases.extend([('dependencies',cp,friends),('stdlib',[STD],[])])
 try:
  for label,paths,fr in cp_cases[:1]:
   dest=OUT/'measured-output'/str(i)/label
   args=[BIN,str(f),'-cp',os.pathsep.join(paths),'-d',str(dest),'--diagnostic-format','json']
   if fr:args+=['--friend-paths',os.pathsep.join(fr)]
   attempt={'configuration':label}
   try:
    proc=subprocess.run(args,capture_output=True,text=True,timeout=int(os.environ.get('RECHECK_TIMEOUT','300')))
    attempt.update(exit_code=proc.returncode,diagnostic=proc.stderr.strip())
    if proc.returncode==0:
     rec=ownership(dest,f);assert rec
     classes=[n for n in rec if n.endswith('.class')]
     for n in classes:assert class_info((dest/n).read_bytes())['source_file']==f.name
     assert classes or any(n.endswith('.kotlin_module') for n in rec)
     attempt.update(outcome='compiled',class_count=len(classes),owned_outputs=rec)
    else:attempt['outcome']='rejected' if proc.returncode==1 else 'compiler_error'
   except subprocess.TimeoutExpired:attempt.update(outcome='timeout')
   except Exception as e:attempt.update(outcome='verification_error',diagnostic=repr(e))
   row['attempts'].append(attempt)
   if attempt['outcome']=='compiled':break
 finally:sib.unlink(missing_ok=True)
 (OUT/'measured'/f'{i:03}.json').write_text(json.dumps(row))
for d in ['temp-jars','measured']:(OUT/d).mkdir(exist_ok=True)
def selected(i,f):
 if os.environ.get('RECHECK_IDS') and i not in {int(v) for v in os.environ['RECHECK_IDS'].split(',')}:return False
 before=json.loads((W/'measured'/f'{i:03}.json').read_text())
 diag=before['attempts'][0]['diagnostic']
 if mode=='tests':return 'multi-release JAR class overrides' in diag
 if 'Expected a class member declaration' not in diag:return False
 return 'const val ' in f.read_text().splitlines()[json.loads(diag)['line']-1]
jobs=[(i,f) for i,f in enumerate(sources) if selected(i,f) and not (OUT/'measured'/f'{i:03}.json').exists()]
print(len(jobs),mode,flush=True)
with concurrent.futures.ThreadPoolExecutor(4) as pool:list(pool.map(run_job,jobs))
