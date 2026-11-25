#!/usr/bin/env python3
"""Frozen differential and fixed-caller evidence for generic vararg captures."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_nov21_generics import callable_signatures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/big_generic_vararg_captures'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/debug/korrust')
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    args = parser.parse_args()
    candidate = args.candidate.resolve()
    reference_root = args.reference_root.resolve()
    pin = json.loads((reference_root / 'references/compiler.json').read_text())
    reference = reference_root / pin['path']
    kotlin = reference / 'bin/kotlinc'
    stdlib = reference / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-big-generic-vararg-captures'
    work = ROOT / 'target/checks' / ident
    logs = ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True)
    logs.mkdir(parents=True)
    def rel(p):
        return str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p)
    inputs = [*ROOT.glob('src/**/*.rs'), *FIXTURES.glob('*'), ROOT / 'tests/big_generic_vararg_captures.rs', Path(__file__), ROOT / 'tools/check_nov21_generics.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', reference_root / 'references/compiler.json']
    frozen = {rel(p):sha(p) for p in sorted(inputs) if p.is_file()}
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), binary_sha256=sha(candidate), source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()), reference_compiler=dict(version=pin['version'],archive_sha256=pin['archive_sha256'],compiler_jar_sha256=sha(reference / 'lib/kotlin-compiler.jar')), inputs=dict(files_sha256=frozen,stdlib_sha256=sha(stdlib)), commands=[],cases=[], evidence=[], reproduction_commands=[['python3','tools/check_big_generic_vararg_captures.py','--candidate',str(candidate),'--reference-root',str(reference_root)]], limitations=['Tests cover generic reference vararg arrays captured in class methods with classifier and independent method parameters, interface bounds, boxed Int calls and imported metadata. Write and invariant-array escape checks remain active through closure captures and aliases. No full-app or Android compatibility claim.'])
    def run(label, command, accepted=(0,)):
        result = subprocess.run(list(map(str,command)),cwd=ROOT,text=True,capture_output=True,timeout=180)
        item = dict(label=label,argv=list(map(str,command)),exit_code=result.returncode)
        for stream in ['stdout','stderr']:
            path=logs / f'{len(report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result,stream)); item[stream]=rel(path)
        report['commands'].append(item)
        if result.returncode not in accepted:
            raise AssertionError(label+'\n'+result.stdout+result.stderr)
        return result
    def case(ident, passed):
        report['cases'].append(dict(id=ident,outcome='pass' if passed else 'correctness_failure'))
        if not passed:
            raise AssertionError(ident)
    def cp(*paths):
        return os.pathsep.join(map(str,paths))
    def compile(provider, source, output, deps=()):
        command = [kotlin,source,'-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','generic_capture'] if provider=='official' else [candidate,source,'--module-name','generic_capture']
        return run('compile '+provider+' '+source.name,[*command,'-cp',cp(*deps,stdlib),'-d',output])
    try:
        outputs={p:work/p for p in ['official','candidate']}
        for provider, output in outputs.items():
            compile(provider,FIXTURES/'Captured.kt',output)
        callers=work/'java'
        run('compile fixed Java caller against official',[jdk/'javac','-cp',cp(outputs['official'],stdlib),'-d',callers,FIXTURES/'Calls.java'])
        signatures={}
        for provider,output in outputs.items():
            result=run('verified fixed Java caller '+provider,[jdk/'java','-Xverify:all','-cp',cp(callers,output,stdlib),'Calls'])
            case('fixed-java/'+provider,result.stdout=='OK\n')
            sig=run('JVM callable signatures '+provider,[jdk/'javap','-p','-s','-v','-cp',output,'capturedvarargs.Box'])
            signatures[provider]=callable_signatures(sig.stdout)
            for consumer in ['official','candidate']:
                consumer_out=work/(provider+'-'+consumer)
                compile(consumer,FIXTURES/'Consumer.kt',consumer_out,[output])
                runner=work/(provider+'-'+consumer+'-runner')
                run('compile consumer runner',[jdk/'javac','-cp',cp(output,consumer_out,stdlib),'-d',runner,FIXTURES/'ConsumeCalls.java'])
                result=run('verified Kotlin consumer '+provider+'/'+consumer,[jdk/'java','-Xverify:all','-cp',cp(runner,consumer_out,output,stdlib),'ConsumeCalls'])
                case('kotlin-consumer/'+provider+'/'+consumer,result.stdout=='OK\n')
        case('JVM-public-callable-signatures',signatures['official']==signatures['candidate'])
        for item in json.loads((FIXTURES/'Rejections.json').read_text()):
            source=work/(item['id']+'.kt');source.write_text(item['source'])
            for provider in ['official','candidate']:
                command=[kotlin,source] if provider=='official' else [candidate,source,'--diagnostic-format','json']
                result=run('negative '+provider+'/'+item['id'],[*command,'-cp',stdlib,'-d',work/('negative-'+provider)],(1,))
                case('negative/'+provider+'/'+item['id'],result.returncode==1)
        case('frozen-inputs',all(sha(Path(p) if Path(p).is_absolute() else ROOT/p)==value for p,value in frozen.items()))
        report['status']='passed'
    except Exception as error:
        report['status']='failed';report['error']=str(error)
        raise
    finally:
        report['counts']=dict(Counter(case['outcome'] for case in report['cases']))
        path=logs.with_suffix('.json');path.write_text(json.dumps(report,indent=2)+'\n')
        print(path)
        reports=ROOT/'tools/reports.py'
        if reports.exists():
            subprocess.run(['python3',reports],cwd=ROOT,check=True)


if __name__=='__main__':
    main()
