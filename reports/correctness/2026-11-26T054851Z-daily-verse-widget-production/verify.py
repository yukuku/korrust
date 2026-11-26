import os,json,hashlib,re,subprocess,sys,zipfile
from pathlib import Path
run,model_root,dest=map(Path,sys.argv[1:]);dest.mkdir(parents=True,exist_ok=True)
models={}
for f in model_root.glob('Alkitab*.json'): models.update(json.loads(f.read_text()))
row=json.loads((run/'production/measured/051.json').read_text());attempt,=row['attempts'];assert attempt['outcome']=='compiled'
actual=run/'production/measured-output/51/resolved'
for name,data in attempt['owned_outputs'].items():assert hashlib.sha256((actual/name).read_bytes()).hexdigest()==data['sha256']
deps=Path('/workspace/work/deps33');module=deps/'project/Alkitab';sdk=deps/'robolectric-sdk28'
config=module/'build/intermediates/unit_test_config_directory/plainDebugUnitTest/generatePlainDebugUnitTestConfig/out'
r=module/'build/intermediates/compile_and_runtime_r_class_jar/plainDebug/processPlainDebugResources/R.jar'
app=list(map(Path,models['Alkitab/main']['outputs']));test=list(map(Path,models['Alkitab/test']['outputs']))
runtime=json.loads((deps/'test-runtime-receipt.json').read_text());rest=[*runtime['dependency_classpath'],next(sdk.glob('*14-*-i6.jar'))]
filtered=dest/'app-without-input.jar';excluded=set(row['excluded_classes']);removed=set()
with zipfile.ZipFile(filtered,'w',zipfile.ZIP_DEFLATED) as z:
 seen=set()
 for directory in app:
  assert directory.is_dir()
  for f in sorted(directory.rglob('*')):
   if not f.is_file():continue
   n=f.relative_to(directory).as_posix()
   if n in excluded:removed.add(n);continue
   if n not in seen:z.write(f,n);seen.add(n)
assert excluded==removed
classes=['yuku.alkitab.base.appwidget.DailyVerseClickReceiverTest','yuku.alkitab.base.appwidget.DailyVerseDataTest']
sources=[Path('/workspace/korrust/benchmarks/sources/androidbible/Alkitab/src/test/java')/(klass.replace('.','/')+'.kt') for klass in classes]
rows={}
for label,files in [('kotlin',app),('korrust',[actual,filtered])]:
 env=os.environ.copy()
 cmd=['java','-Xmx1400m','-Xverify:all','-Drobolectric.offline=true','-Drobolectric.dependency.dir='+str(sdk),'-cp',os.pathsep.join(map(str,[*test,config,r,*files,*rest])),'org.junit.runner.JUnitCore',*classes]
 proc=subprocess.run(cmd,cwd=module,env=env,capture_output=True,text=True,timeout=600)
 log=proc.stdout+proc.stderr;(dest/(label+'.log')).write_text(log)
 match=re.search(r'^OK \((\d+) tests?\)$',log,re.M)
 rows[label]=dict(exit_code=proc.returncode,passed_tests=int(match[1]) if match else None)
 print(label,rows[label],flush=True)
(dest/'result.json').write_text(json.dumps(dict(receipt=row,removed=sorted(removed),tests=classes,test_source_sha256={k:hashlib.sha256(f.read_bytes()).hexdigest() for k,f in zip(classes,sources)},results=rows),indent=2)+'\n')
