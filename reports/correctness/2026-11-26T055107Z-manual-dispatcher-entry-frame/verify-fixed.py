from pathlib import Path
import json,subprocess,os,zipfile,hashlib,re
root=Path('/workspace/work/targets159')
dest=Path('/dev/shm/k159-manual-final-runtime');dest.mkdir(exist_ok=True)
repo=Path('/workspace/korrust'); manifest=json.loads((repo/'benchmarks/source-files.sha256.json').read_text())['androidbible']
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
cases=[]; native=[]
for run,idx in [('entry-frame',292),('wave12-fixed',291)]:
 p=root/run/'tests/measured'/f'{idx:03}.json';r=json.loads(p.read_text());a,=r['attempts'];assert a['outcome']=='compiled'
 assert sha(repo/'benchmarks/sources/androidbible'/r['path'])==manifest[r['path']]==r['source_sha256']
 output=root/run/'tests/measured-output'/str(idx)/'resolved'
 for name,info in a['owned_outputs'].items():assert sha(output/name)==info['sha256']
 cases.append(r);native.append(output)
models={}
for f in Path('/workspace/work/all400-nov25').glob('Alkitab*.json'):models.update(json.loads(f.read_text()))
m=models[cases[0]['group']];excluded=set().union(*(c['excluded_classes'] for c in cases));siblings=dest/'remaining.jar';written=set();removed=set()
with zipfile.ZipFile(siblings,'w',zipfile.ZIP_DEFLATED) as z:
 for output in map(Path,m['outputs']):
  for f in sorted(output.rglob('*')):
   n=f.relative_to(output).as_posix()
   if not f.is_file():continue
   if n in excluded:removed.add(n);continue
   if n not in written:z.write(f,n);written.add(n)
assert removed==excluded
result={}
for compiler,cp in [('kotlin',m['outputs']),('korrust',[*native,siblings])]:
 r=subprocess.run(['java','-Xverify:all','-cp',os.pathsep.join(map(str,[*cp,*m['cp']])),'org.junit.runner.JUnitCore','yuku.alkitab.base.devotion.DevotionDownloaderTest'],capture_output=True,text=True,timeout=180)
 log=r.stdout+r.stderr;(dest/(compiler+'.log')).write_text(log)
 ok=re.search(r'^OK \((\d+) tests?\)$',log,re.M);result[compiler]=dict(exit_code=r.returncode,passed_tests=int(ok[1]) if ok else None)
(dest/'outcomes.json').write_text(json.dumps(dict(cases=cases,removed=sorted(removed),result=result),indent=2)+'\n')
print(result)
