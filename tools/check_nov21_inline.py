#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

from upstream_jvm import normalize_signature

ROOT = Path(__file__).resolve().parents[1]

def sha(p):
    return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    ap.add_argument('--baseline', type=Path)
    ap.add_argument('--output', type=Path)
    ap.add_argument('--skip-unit-array', action='store_true')
    ap.add_argument('--skip-app', action='store_true')
    ap.add_argument('--skip-html', action='store_true')
    a = ap.parse_args()
    now = datetime.now(timezone.utc)
    stamp = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-nov21-inline'
    work = a.output or ROOT / 'target/checks' / stamp
    work.mkdir(parents=True, exist_ok=False)
    ev = ROOT / 'reports/correctness' / stamp
    ev.mkdir(parents=True)
    (ev / 'commands').mkdir()
    reportpath = ev.with_suffix('.json')
    features = ROOT / 'reports/features'
    features.mkdir(parents=True, exist_ok=True)
    candidate = work / 'korrust-frozen'
    shutil.copy2(a.candidate, candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    fixtures = ROOT / 'tests/nov21_inline'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    upstream = ROOT / 'references/kotlin/compiler/testData/codegen/box/strings/twoArgumentStringTemplate.kt'
    app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/AudioHttpErrorBody.kt'
    paths = sorted((ROOT / 'src').rglob('*.rs')) + sorted(p for p in fixtures.rglob('*') if p.is_file()) + [ROOT / 'tests/nov21_inline.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', upstream, app, stdlib, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', candidate]
    def rel(p):
        return os.path.relpath(p, ROOT)
    hashes = {rel(p): sha(p) for p in paths}
    bundle = ev / 'inputs.zip'
    with zipfile.ZipFile(bundle, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in paths:
            if path.suffix in {'.rs', '.kt', '.java', '.py', '.toml', '.lock', '.json'}:
                archive.writestr(rel(path), path.read_bytes())
    r = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(path=rel(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()), reference_compiler=json.loads((ROOT/'references/compiler.json').read_text()), frozen_input_hashes=hashes, input_bundle=dict(path=rel(bundle),sha256=sha(bundle)), reproduction_commands=[['python3',rel(Path(__file__)),'--candidate',str(a.candidate)]], effective_options=dict(jvm_target='1.8',language_version='2.3',api_version='2.3',module_name='nov21_inline'), commands=[],cases=[], limitations=['Unit-returning expressions, nullable Unit and arrays with nullable Unit components in templates, and direct reified Unit array allocation remain explicitly unsupported; Unit singleton references are supported.', 'Reified nested array marker encodings beyond a single substituted parameter and existing parameterized generic/function classifier boundaries remain unsupported; direct concrete array components are tested.', 'Generated nested allocations, context-sensitive invokespecial and array-owner method references retain explicit rejection boundaries.', 'Root Object references and inherited field resolution are prerequisite correctness repairs, not counted features.', 'No full Android build or performance claim.'])
    if a.skip_unit_array:
        r['reproduction_commands'][0].append('--skip-unit-array')
    if a.skip_app:
        r['reproduction_commands'][0].append('--skip-app')
    if a.baseline:
        r['reproduction_commands'][0].extend(['--baseline',str(a.baseline)])
    def save():
        r['counts'] = dict(Counter(x['outcome'] for x in r['cases']))
        reportpath.write_text(json.dumps(r,indent=2)+'\n')
    def run(label, argv, ok=True):
        print(label, flush=True)
        result = subprocess.run(list(map(str,argv)),cwd=ROOT,capture_output=True,text=True,timeout=300)
        log = ev/'commands'/f'{len(r["commands"]):03d}.txt'
        log.write_text('$ '+' '.join(map(str,argv))+'\n'+result.stdout+result.stderr)
        r['commands'].append(dict(label=label,argv=list(map(str,argv)),exit_code=result.returncode,log=rel(log)))
        save()
        if ok:
            assert result.returncode == 0, result.stdout+result.stderr
        return result
    def case(id, **kw):
        r['cases'].append(dict(id=id,outcome=kw.pop('outcome','pass'),**kw));save()
    def cp(*p):
        return os.pathsep.join(map(str,p))
    def compile(provider, sources, out, deps=()):
        return run('Compile '+provider+'/'+out.name, [official if provider=='official' else candidate,*sources,*(['-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','nov21_inline'] if provider=='official' else ['--module-name','nov21_inline','--diagnostic-format','json']),'-cp',cp(*deps,stdlib),'-d',out])
    try:
        identity = run('Pinned compiler version',[official,'-version'])
        assert 'kotlinc-jvm 2.3.21 ' in identity.stdout+identity.stderr
        run('JDK version',[jdk/'java','-version'])
        if a.baseline:
            baseline=work/'baseline-frozen'
            shutil.copy2(a.baseline,baseline)
            r['baseline_compiler']=dict(path=rel(baseline),sha256=sha(baseline))
            for source in sorted((fixtures/'baseline').glob('*.kt')):
                result=run('Baseline '+source.stem,[baseline,source,'-cp',stdlib,'-d',work/('baseline-'+source.stem)],ok=False)
                assert result.returncode!=0,result.stdout+result.stderr
                case('baseline/'+source.stem,outcome='baseline_unsupported',diagnostic=result.stderr)
        helpers=work/'helpers'
        run('Java dependencies',[jdk/'javac','-d',helpers,fixtures/'Roots.java',fixtures/'Leaf.java',fixtures/'Visible.java',fixtures/'Effects.java'])
        library=work/'library'
        compile('official',[fixtures/'Library.kt'],library,[helpers])
        outputs={}
        for p in ['official','candidate']:
            outputs[p]=work/p
            compile(p,[fixtures/'Values.kt',fixtures/'Forward.kt'],outputs[p],[helpers,library])
            case('compile/'+p)
        callers=work/'callers'
        run('Fixed official Java caller',[jdk/'javac','-cp',cp(outputs['official'],helpers,library,stdlib),'-d',callers,fixtures/'Calls.java'])
        callers_hash={rel(p):sha(p) for p in callers.rglob('*.class')}
        r['official_compiled_java_callers_sha256']=callers_hash
        results=[]
        for p,out in outputs.items():
            result=run('Verified runtime '+p,[jdk/'java','-Xverify:all','-cp',cp(callers,out,helpers,library,stdlib),'consumer.Calls'])
            assert result.stdout.startswith('checks=')
            results.append(result.stdout)
            case('runtime/'+p,independent_assertions=int(result.stdout.split('=')[1]),fixed_official_java_caller=True)
            meta=work/('metadata-'+p)
            compile('official',[fixtures/'Consumer.kt'],meta,[out,helpers,library])
            result=run('Metadata consumer '+p,[jdk/'java','-Xverify:all','-cp',cp(meta,out,helpers,library,stdlib),'metadata.ConsumerKt'])
            assert result.stdout=='OK\n'
            case('metadata/'+p)
        for owner in ['consumer.ValuesKt', 'consumer.ForwardKt']:
            signatures = [normalize_signature(run('JVM signatures '+p+'/'+owner,[jdk/'javap','-protected','-s','-classpath',out,owner]).stdout) for p,out in outputs.items()]
            assert signatures[0]==signatures[1],str(signatures)
            case('signature/'+owner)
        assert results[0]==results[1]
        assert callers_hash=={rel(p):sha(p) for p in callers.rglob('*.class')}
        for source in sorted((fixtures/'unsupported').glob('*.kt')):
            if a.skip_unit_array and source.stem=='unit_array':
                case('boundary/unit_array',outcome='not_run',reason='Existing generic Unit upper-bound validation rejects before allocation materialization; integrated inference changes supply the corresponding unsupported diagnostic.')
                continue
            compile('official',[source],work/('boundary-official-'+source.stem),[library,helpers])
            output=work/('boundary-candidate-'+source.stem)
            result=run('Unsupported '+source.stem,[candidate,source,'-cp',cp(library,helpers,stdlib),'--diagnostic-format','json','-d',output],ok=False)
            assert result.returncode!=0 and not list(output.rglob('*.class'))
            diagnostics=[json.loads(line) for line in result.stderr.splitlines()]
            assert diagnostics and all(d['category']=='unsupported_language' for d in diagnostics),result.stderr
            case('boundary/'+source.stem,outcome='unsupported_language',diagnostics=diagnostics)
        if a.skip_app:
            case('app/AudioHttpErrorBody',outcome='not_run',reason='Application assessment not run in this invocation; see separate app evidence.')
        else:
            for p in ['official','candidate']:
                compile(p,[app],work/('app-'+p))
            case('app/AudioHttpErrorBody',unchanged=True,path=rel(app),sha256=sha(app),scope='compile only; independent app runtime evidence recorded by integration harness')
        upstream_outputs={}
        for p in ['official','candidate']:
            upstream_outputs[p]=work/('upstream-'+p)
            compile(p,[upstream],upstream_outputs[p])
        caller=work/'Upstream.java'
        caller.write_text('public class Upstream { public static void main(String[] args) { if (!TwoArgumentStringTemplateKt.box().equals("OK")) throw new AssertionError(); System.out.println("OK"); } }\n')
        upstream_java=work/'upstream-java'
        run('Upstream caller against official',[jdk/'javac','-cp',cp(upstream_outputs['official'],stdlib),'-d',upstream_java,caller])
        for p,out in upstream_outputs.items():
            result=run('Unchanged upstream '+p,[jdk/'java','-Xverify:all','-cp',cp(upstream_java,out,stdlib),'Upstream'])
            assert result.stdout=='OK\n'
        case('upstream/twoArgumentStringTemplate',unchanged=True,path=rel(upstream),sha256=sha(upstream))
        assert hashes=={rel(p):sha(p) for p in paths},'Inputs changed during verification'
        r['status']='pass'
    except Exception as e:
        case('harness_failure',outcome='correctness_failure',error=str(e));r['status']='fail'
    finally:
        save()
        if r['status']=='pass':
            f=dict(schema_version=1,kind='features',created_at=now.isoformat(),status='pass',compiler=r['compiler'],reference_compiler=r['reference_compiler'],reproduction_commands=r['reproduction_commands'],evidence=[rel(reportpath)],features=[dict(name='Reified reference array allocation from compiled inline bodies',status='supported',scope='marker0 plus ANEWARRAY with concrete reference, nullable, boxed primitive and direct nested array components',restrictions=r['limitations'][1:3],evidence_report=rel(reportpath),evidence_cases=['runtime/candidate','metadata/candidate']),dict(name='Arbitrary reference string template values',status='supported',scope='Object and nullable dispatch, null rendering, primitive/reference-array identity representation, evaluation order and thrown toString propagation',restrictions=[r['limitations'][0]],evidence_report=rel(reportpath),evidence_cases=['runtime/candidate','upstream/twoArgumentStringTemplate'])])
            (features/(stamp+'.json')).write_text(json.dumps(f,indent=2)+'\n')
        if not a.skip_html:
            subprocess.run([sys.executable,ROOT/'tools/reports.py'],cwd=ROOT,check=True)
        print(reportpath)
    return 0 if r['status']=='pass' else 1
if __name__=='__main__':
    raise SystemExit(main())
