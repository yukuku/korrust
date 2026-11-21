import json,sys,subprocess,time,resource,hashlib
from pathlib import Path
mode,binary,config,stem,timeout=sys.argv[1:];w=Path('/workspace/work/archive33-evidence');root=Path('/workspace/korrust')
x=json.load(open(config))['cases'];id,spec=next((i,s) for i,s in x.items() if Path(i).stem==stem)
cp=[root/'references/kotlinc/lib/kotlin-stdlib.jar',*map(Path,spec['classpath'])]
cmd=[binary,str(root/id),'-cp',':'.join(map(str,cp)),'-d',str(w/(mode+'-'+stem)),'--diagnostic-format','json']
before=resource.getrusage(resource.RUSAGE_CHILDREN);start=time.perf_counter()
try:
 p=subprocess.run(cmd,cwd=root,capture_output=True,timeout=float(timeout));rc=p.returncode;out=p.stdout+p.stderr;status='completed'
except subprocess.TimeoutExpired as e:
 rc=None;out=(e.stdout or b'')+(e.stderr or b'');status='timeout'
after=resource.getrusage(resource.RUSAGE_CHILDREN)
(w/(mode+'-'+stem+'.log')).write_bytes(out)
r={'mode':mode,'case':id,'status':status,'returncode':rc,'wall_seconds':time.perf_counter()-start,'user_seconds':after.ru_utime-before.ru_utime,'sys_seconds':after.ru_stime-before.ru_stime,'command':cmd,'compiler_sha256':hashlib.sha256(Path(binary).read_bytes()).hexdigest(),'source_sha256':hashlib.sha256((root/id).read_bytes()).hexdigest(),'classpath':list(map(str,cp))}
(w/(mode+'-'+stem+'.json')).write_text(json.dumps(r,indent=2)+'\n');print(json.dumps({k:r[k] for k in ['mode','case','status','returncode','wall_seconds','user_seconds','sys_seconds']}),flush=True)
