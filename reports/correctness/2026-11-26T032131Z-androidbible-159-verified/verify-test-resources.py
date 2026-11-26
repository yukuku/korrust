import hashlib,json,os,re,subprocess,sys,tempfile,zipfile
from pathlib import Path

run,model_root,destination = map(Path,sys.argv[1:])
destination.mkdir(parents=True,exist_ok=True)
models={}
for f in model_root.glob('Alkitab*.json'):
    models.update(json.loads(f.read_text()))
rows=[]
for f in sorted((run/'tests/measured').glob('*.json')):
    row=json.loads(f.read_text())
    attempt,=row['attempts']
    if attempt['outcome']!='compiled':continue
    uses_runner = '@RunWith(RobolectricTestRunner' in (Path('/workspace/korrust/benchmarks/sources/androidbible')/row['path']).read_text()
    if uses_runner != False:continue
    if f.stem not in ('387','388'):continue
    model=dict(models[row['group']])
    test_resources=Path('/workspace/korrust/benchmarks/sources/androidbible')/row['group'].split('/')[0]/'src/test/resources'
    model['cp']=[str(test_resources),*model['cp']]
    actual=run/'tests/measured-output'/str(int(f.stem))/'resolved'
    for name,data in attempt['owned_outputs'].items():
        assert hashlib.sha256((actual/name).read_bytes()).hexdigest()==data['sha256']
    klass=next((name[:-6].replace('/','.') for name in attempt['owned_outputs'] if name.endswith('Test.class') and '$' not in name),None)
    entry={'source':row['path'],'source_sha256':row['source_sha256'],'class':klass}
    rows.append(entry)
    if klass is None:continue
    with tempfile.TemporaryDirectory() as tmp:
        siblings=Path(tmp)/'siblings.jar'
        excluded=set(row['excluded_classes'])
        with zipfile.ZipFile(siblings,'w') as z:
            written=set()
            for output in map(Path,model['outputs']):
                for path in sorted(output.rglob('*')):
                    name=path.relative_to(output).as_posix()
                    if path.is_file() and name not in excluded and name not in written:
                        z.write(path,name)
                        written.add(name)
        for compiler,paths in [('kotlin',model['outputs']),('korrust',[str(actual),str(siblings)])]:
            proc=subprocess.run(['java','-Xverify:all','-cp',os.pathsep.join([*paths,*model['cp']]),'org.junit.runner.JUnitCore',klass],capture_output=True,text=True,timeout=180)
            log=proc.stdout+proc.stderr
            (destination/f'{f.stem}-{compiler}.log').write_text(log)
            match=re.search(r'^OK \((\d+) tests?\)$',log,re.M)
            entry[compiler]={'exit_code':proc.returncode,'passed_tests':int(match[1]) if match else None}
    print(entry,flush=True)
(destination/'junit.json').write_text(json.dumps(rows,indent=2)+'\n')
