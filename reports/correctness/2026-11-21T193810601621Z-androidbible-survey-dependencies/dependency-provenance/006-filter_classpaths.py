import hashlib,json,re,struct,zipfile
from pathlib import Path
ROOT=Path('/workspace/korrust'); WORK=Path('/workspace/work/deps33')
TARGETS={'AppServices','InternalDbTxWrapper','SyncUtils','VersesUiModel','MediaStateListener'}

def sha(b):return hashlib.sha256(b).hexdigest()
def sourcefile(b):
    pos=8
    def u2():
        nonlocal pos
        v=struct.unpack_from('>H',b,pos)[0];pos+=2;return v
    def u4():
        nonlocal pos
        v=struct.unpack_from('>I',b,pos)[0];pos+=4;return v
    n=u2();cp={};i=1
    while i<n:
        tag=b[pos];pos+=1
        if tag==1:
            size=u2();cp[i]=b[pos:pos+size].decode('utf-8',errors='replace');pos+=size
        elif tag in (3,4):pos+=4
        elif tag in (5,6):pos+=8;i+=1
        elif tag in (7,8,16,19,20):pos+=2
        elif tag in (9,10,11,12,17,18):pos+=4
        elif tag==15:pos+=3
        else:raise ValueError(tag)
        i+=1
    pos+=6;count=u2();pos+=2*count
    for _ in range(2):
        for member in range(u2()):
            pos+=6
            for attr in range(u2()):
                u2();size=u4();pos+=size
    for attr in range(u2()):
        name=cp[u2()];size=u4()
        if name=='SourceFile':return cp[u2()]
        pos+=size

m=json.loads((WORK/'official-model.json').read_text())
cases=json.loads((ROOT/'reports/correctness/2026-11-21T072519326520Z-next10-app-assessment.json').read_text())['cases']
config={'schema_version':1,'cases':{}}
for case in cases:
    id=case['id'];stem=Path(id).stem
    if stem not in TARGETS:continue
    dest=WORK/'filtered'/stem;dest.mkdir(parents=True,exist_ok=True)
    source=ROOT/id; package=re.search(r'^package\s+(\S+)',source.read_text(),re.M).group(1).replace('.','/')
    removed=[];copied={}
    jar=dest/'official-siblings.jar'
    with zipfile.ZipFile(jar,'w',compression=zipfile.ZIP_DEFLATED) as out:
        for output in (m['kotlin_output'],m['java_output']):
            for p in sorted(Path(output).rglob('*')):
                if not p.is_file():continue
                name=p.relative_to(output).as_posix();b=p.read_bytes()
                own=name.startswith(package+'/') and name.endswith('.class') and (sourcefile(b)==source.name or name==package+'/'+stem+'.class' or name.startswith(package+'/'+stem+'$'))
                metadata=name.endswith('.kotlin_module') and (stem.encode() in b or (stem+'Kt').encode() in b)
                if own or metadata:
                    removed.append({'path':str(p),'entry':name,'sha256':sha(b),'reason':'target source' if own else 'module package-parts metadata omitted from filtered sibling output'})
                elif name not in copied:
                    out.writestr(name,b);copied[name]=sha(b)
                elif copied[name]!=sha(b):raise ValueError('conflicting duplicate '+name)
    omitted=[p for p in m['kotlin_classpath'] if not Path(p).exists()]
    cp=[*[p for p in m['kotlin_classpath'] if Path(p).exists()],str(jar)]
    for p in cp:
        if not Path(p).exists():raise ValueError('missing classpath '+p)
    manifest={'nonexistent_empty_generated_classpath_entries_omitted':omitted,'provider':'official untouched AndroidBible Gradle build','source':str(source),'source_sha256':sha(source.read_bytes()),'jar':str(jar),'jar_sha256':sha(jar.read_bytes()),'copied_classes_sha256':copied,'excluded_entries':removed,'official_model':str(WORK/'official-model.json'),'build_log':str(WORK/'official-build.log'),'source_compilation_policy':'Target bytecode excluded. Original Kotlin module metadata retained unless its package-part payload references target facade; every remaining sibling byte copied unmodified from official build outputs.'}
    (dest/'provenance.json').write_text(json.dumps(manifest,indent=2)+'\n')
    (dest/'classpath.txt').write_text(':'.join(cp)+'\n')
    config['cases'][id]={'classpath':cp,'friend_paths':[],'sources':[],'provenance':[{'kind':'filtered official unchanged sibling classes','path':str(dest/'provenance.json'),'sha256':sha((dest/'provenance.json').read_bytes())},*[{'kind':'official dependency replay input','path':str(WORK/n),'sha256':sha((WORK/n).read_bytes())} for n in ['setup-dependencies.sh','reproduce-build.sh','capture.init.gradle','filter_classpaths.py','source-sha256.json','official-model.json','official-build.log','official-build-initial.log']]],'reproduction_commands':[['bash',str(WORK/'reproduce-build.sh')]]}
(WORK/'classpaths.json').write_text(json.dumps(config,indent=2)+'\n')
print(json.dumps({'cases':len(config['cases']),'output':str(WORK/'classpaths.json')}))
