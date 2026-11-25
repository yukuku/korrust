from concurrent.futures import ThreadPoolExecutor
import subprocess, pathlib, json, hashlib
root=pathlib.Path('/workspace/korrust')
source_report=root/'reports/correctness/2026-11-21T072519326520Z-next10-app-assessment.json'
out=pathlib.Path('/workspace/work/survey33/standalone')
out.mkdir(exist_ok=True)
def compile(case):
    src=root/case['id']
    assert hashlib.sha256(src.read_bytes()).hexdigest()==case['sha256']
    dest=out/src.stem
    cmd=[str(root/'references/kotlinc/bin/kotlinc'),str(src),'-no-stdlib','-no-reflect','-cp',str(root/'references/kotlinc/lib/kotlin-stdlib.jar'),'-jvm-target','21','-d',str(dest)]
    p=subprocess.run(cmd,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (out/(src.stem+'.log')).write_bytes(p.stdout)
    return {'source':case['id'],'sha256':case['sha256'],'exit':p.returncode,'classes':sorted(str(c.relative_to(dest)) for c in dest.rglob('*.class')),'command':cmd}
with ThreadPoolExecutor(max_workers=2) as pool:
    result=list(pool.map(compile,json.loads(source_report.read_text())['cases']))
(out/'results.json').write_text(json.dumps(result,indent=2)+'\n')
print(len(result),sum(r['exit']==0 for r in result),[r['source'].split('/')[-1] for r in result if r['exit']])
