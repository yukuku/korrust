from pathlib import Path
import hashlib,json,os,subprocess
r=Path('/workspace/korrust');w=Path('/workspace/work/targets159/suspend-constructor-final');w.mkdir(exist_ok=True)
spec=r/'tests/suspend_call_operands/Rejections.json';case=json.loads(spec.read_text())[0];src=w/'Boundary.kt';src.write_text(case['source'])
cp=os.pathsep.join(map(str,[r/'target/checks/2026-11-26T055530070785Z-suspend-receiver-functions/library',r/'references/kotlinc/lib/kotlin-stdlib.jar',r/'target/reference-libs/kotlinx-coroutines-core-jvm-1.9.0.jar']))
commands=[]
for provider,exe,extra in [('official',r/'references/kotlinc/bin/kotlinc',[]),('candidate',Path('/tmp/targets159-suspend-committed/debug/korrust'),['--diagnostic-format','json'])]:
 command=list(map(str,[exe,src,*extra,'-cp',cp,'-d',w/provider]));p=subprocess.run(command,text=True,capture_output=True,timeout=180,env={**os.environ,'JAVA_OPTS':'-Xmx256m -Xms32m'});commands.append(dict(provider=provider,argv=command,exit_code=p.returncode))
 (w/(provider+'-stdout.log')).write_text(p.stdout);(w/(provider+'-stderr.log')).write_text(p.stderr)
 assert p.returncode==(0 if provider=='official' else 1),(provider,p.stdout,p.stderr)
 if provider=='candidate':
  d=[json.loads(x) for x in p.stderr.splitlines() if x.startswith('{')];assert d[0]['category']=='unsupported_language';commands[-1]['diagnostics']=d
j=dict(id=case['id'],status='passed',source_sha256=hashlib.sha256(src.read_bytes()).hexdigest(),fixture_sha256=hashlib.sha256(spec.read_bytes()).hexdigest(),binary_sha256=hashlib.sha256(Path('/tmp/targets159-suspend-committed/debug/korrust').read_bytes()).hexdigest(),commands=commands)
(w/'result.json').write_text(json.dumps(j,indent=2)+'\n');print(j['status'])
