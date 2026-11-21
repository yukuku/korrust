import hashlib,json,re,struct,zipfile
from pathlib import Path
import argparse
HERE=Path(__file__).resolve().parent
p=argparse.ArgumentParser();p.add_argument('--repo-root',type=Path,required=True);p.add_argument('--work-dir',type=Path,required=True)
args=p.parse_args();ROOT=args.repo_root.resolve();WORK=args.work_dir.resolve()/'replay-artifacts'
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
targets=json.loads((HERE/'targets.json').read_text())
config={'schema_version':1,'cases':{}}
for id,expected_hash in targets.items():
    source=ROOT/id;stem=source.stem
    assert sha(source.read_bytes())==expected_hash
    package=re.search(r'^package\s+(\S+)',source.read_text(),re.M).group(1).replace('.','/')
    dest=WORK/'filtered'/stem;dest.mkdir(parents=True,exist_ok=True)
    removed=[];copied={};jar=dest/'official-siblings.jar'
    with zipfile.ZipFile(jar,'w',compression=zipfile.ZIP_DEFLATED) as out:
        for output in (m['kotlin_output'],m['java_output']):
            for p in sorted(Path(output).rglob('*')):
                if not p.is_file():continue
                name=p.relative_to(output).as_posix();b=p.read_bytes()
                own=name.startswith(package+'/') and name.endswith('.class') and (sourcefile(b)==source.name or name==package+'/'+stem+'.class' or name.startswith(package+'/'+stem+'$'))
                metadata=name.endswith('.kotlin_module') and stem.encode() in b
                if own or metadata:
                    removed.append({'entry':name,'sha256':sha(b),'reason':'target source' if own else 'module metadata references target facade'})
                elif name not in copied:
                    info=zipfile.ZipInfo(name,date_time=(1980,1,1,0,0,0));info.compress_type=zipfile.ZIP_DEFLATED
                    out.writestr(info,b);copied[name]=sha(b)
                elif copied[name]!=sha(b):raise ValueError('conflicting duplicate '+name)
    omitted=[p for p in m['kotlin_classpath'] if not Path(p).exists()]
    if any(not p.endswith('/generated/ksp/plainDebug/classes') for p in omitted):
        raise ValueError('Unexpected missing classpath inputs: '+repr(omitted))
    cp=[*[p for p in m['kotlin_classpath'] if Path(p).exists()],str(jar)]
    manifest={'provider':'official untouched AndroidBible Gradle build',
              'source':str(source),'source_sha256':expected_hash,'jar':str(jar),'jar_sha256':sha(jar.read_bytes()),
              'copied_classes_sha256':copied,'excluded_entries':removed,
              'nonexistent_empty_generated_classpath_entries_omitted':omitted,
              'policy':'Every retained byte is copied unchanged from official output; target class family is absent.'}
    receipt=dest/'provenance.json';receipt.write_text(json.dumps(manifest,indent=2)+'\n')
    (dest/'classpath.txt').write_text(':'.join(cp)+'\n')
    config['cases'][id]={'classpath':cp,'friend_paths':[],'sources':[],
        'provenance':[{'path':str(receipt),'sha256':sha(receipt.read_bytes())}],
        'reproduction_commands':[['python3',str(HERE/'replay.py'),'--repo-root',str(ROOT),'--work-dir',str(args.work_dir.resolve())]]}
(WORK/'classpaths.json').write_text(json.dumps(config,indent=2)+'\n')
print(json.dumps({'cases':len(config['cases']),'output':str(WORK/'classpaths.json')}))
