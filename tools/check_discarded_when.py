#!/usr/bin/env python3
"""Frozen differential evidence for discarded when branch effects."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile
from check_nov21_generics import callable_signatures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/discarded_when'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--skip-html', action='store_true')
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/debug/korrust')
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--reference-root', type=Path, default=ROOT)
    args = parser.parse_args()
    candidate = args.candidate.resolve()
    source_root = args.source_root.resolve()
    reference_root = args.reference_root.resolve()
    revision_file = source_root / 'source-revision.txt'
    source_revision = revision_file.read_text().strip() if revision_file.exists() else subprocess.check_output(['git','rev-parse','HEAD'],cwd=source_root,text=True).strip()
    pin = json.loads((reference_root / 'references/compiler.json').read_text())
    reference = reference_root / pin['path']
    kotlin = reference / 'bin/kotlinc'
    stdlib = reference / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-discarded-when'
    work = ROOT / 'target/checks' / ident
    logs = ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True)
    logs.mkdir(parents=True)
    def rel(p):
        return str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p)
    inputs = [*source_root.glob('src/**/*.rs'), *FIXTURES.rglob('*'), ROOT / 'tests/discarded_when.rs', Path(__file__), ROOT / 'tools/check_nov21_generics.py', source_root / 'Cargo.toml', source_root / 'Cargo.lock', reference_root / 'references/compiler.json']
    frozen = {rel(p):sha(p) for p in sorted(inputs) if p.is_file()}
    bundle=logs/'frozen-inputs.zip'
    with zipfile.ZipFile(bundle,'w',zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            if path.is_file(): archive.writestr(rel(path),path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), binary_sha256=sha(candidate), source_revision=source_revision), source_archive=dict(path=rel(bundle),sha256=sha(bundle)), reference_compiler=dict(version=pin['version'],archive_sha256=pin['archive_sha256'],compiler_jar_sha256=sha(reference / 'lib/kotlin-compiler.jar')), inputs=dict(files_sha256=frozen,stdlib_sha256=sha(stdlib)), commands=[],cases=[], evidence=[], reproduction_commands=[['python3','tools/check_discarded_when.py','--candidate',str(candidate),'--reference-root',str(reference_root),'--source-root',str(source_root)]], limitations=['When expressions in discarded statement and Unit callback contexts check branch effects without computing a common value type. Value contexts retain existing result-join rules. No Android runtime or full-app claim.'])
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
        command = [kotlin,source,'-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','discarded_when'] if provider=='official' else [candidate,source,'--module-name','discarded_when']
        return run('compile '+provider+' '+source.name,[*command,'-cp',cp(*deps,stdlib),'-d',output])
    try:
        dependency=work/'dependencies'
        dependency.mkdir()
        outputs={p:work/p for p in ['official','candidate']}
        for provider, output in outputs.items():
            compile(provider,FIXTURES/'Effects.kt',output,[dependency])
        callers=work/'java'
        run('compile fixed Java caller against official',[jdk/'javac','-cp',cp(outputs['official'],dependency,stdlib),'-d',callers,FIXTURES/'Calls.java'])
        signatures={}
        for provider,output in outputs.items():
            result=run('verified fixed Java caller '+provider,[jdk/'java','-Xverify:all','-cp',cp(callers,output,dependency,stdlib),'Calls'])
            case('fixed-java/'+provider,result.stdout=='OK\n')
            sig=run('JVM callable signatures '+provider,[jdk/'javap','-p','-s','-v','-cp',output,'EffectsKt'])
            signatures[provider]=callable_signatures(sig.stdout)
            for consumer in ['official','candidate']:
                consumer_out=work/(provider+'-'+consumer)
                compile(consumer,FIXTURES/'Consumer.kt',consumer_out,[output,dependency])
                runner=work/(provider+'-'+consumer+'-runner')
                run('compile consumer runner',[jdk/'javac','-cp',cp(output,consumer_out,dependency,stdlib),'-d',runner,FIXTURES/'ConsumeCalls.java'])
                result=run('verified Kotlin consumer '+provider+'/'+consumer,[jdk/'java','-Xverify:all','-cp',cp(runner,consumer_out,output,dependency,stdlib),'ConsumeCalls'])
                case('kotlin-consumer/'+provider+'/'+consumer,result.stdout=='OK\n')
        case('JVM-public-callable-signatures',signatures['official']==signatures['candidate'])
        for item in json.loads((FIXTURES/'Rejections.json').read_text()):
            source=work/(item['id']+'.kt');source.write_text(item['source'])
            for provider in ['official','candidate']:
                command=[kotlin,source] if provider=='official' else [candidate,source,'--diagnostic-format','json']
                result=run('negative '+provider+'/'+item['id'],[*command,'-cp',cp(dependency,stdlib),'-d',work/('negative-'+provider)],(1,))
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
        if reports.exists() and not args.skip_html:
            subprocess.run(['python3',reports],cwd=ROOT,check=True)


if __name__=='__main__':
    main()
