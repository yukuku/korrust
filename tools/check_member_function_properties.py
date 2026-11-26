#!/usr/bin/env python3
"""Frozen differential and fixed-caller evidence for function-valued instance properties."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
from check_nov21_generics import callable_signatures
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/member_function_properties'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def public_signatures(output):
    result = callable_signatures(output)
    current = descriptor = None
    for line in output.splitlines():
        if line.startswith(('  public ', '  protected ')):
            current = line.strip().split('(', 1)[0].rsplit(' ', 1)[-1] if '(' in line else None
            descriptor = None
        elif line.startswith(('  private ', '  static ')):
            current = None
        elif current and line.strip().startswith('descriptor:'):
            descriptor = line.strip().removeprefix('descriptor:').strip()
        elif current and descriptor and line.strip().startswith('flags:') and 'ACC_SYNTHETIC' in line:
            result.pop(current + descriptor, None)
        elif line == '}':
            current = None
    return result


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
    pin = json.loads((reference_root / 'references/compiler.json').read_text())
    reference = reference_root / pin['path']
    kotlin = reference / 'bin/kotlinc'
    stdlib = reference / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-member-function-properties'
    work = ROOT / 'target/checks' / ident
    logs = ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True)
    logs.mkdir(parents=True)
    def rel(p):
        return str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p)
    inputs = [*source_root.glob('src/**/*.rs'), *FIXTURES.glob('*'), ROOT / 'tests/member_function_properties.rs', Path(__file__), ROOT / 'tools/check_nov21_generics.py', source_root / 'Cargo.toml', source_root / 'Cargo.lock', reference_root / 'references/compiler.json']
    frozen = {rel(p):sha(p) for p in sorted(inputs) if p.is_file()}
    bundle=logs/'frozen-inputs.zip'
    with zipfile.ZipFile(bundle,'w',zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            if path.is_file(): archive.writestr(rel(path),path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), binary_sha256=sha(candidate), source_revision=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip() if source_root==ROOT else None,source_root=rel(source_root)), reference_compiler=dict(version=pin['version'],archive_sha256=pin['archive_sha256'],compiler_jar_sha256=sha(reference / 'lib/kotlin-compiler.jar')), inputs=dict(files_sha256=frozen,stdlib_sha256=sha(stdlib)),source_archive=dict(path=rel(bundle),sha256=sha(bundle)), commands=[],cases=[], evidence=[], reproduction_commands=[['python3','tools/check_member_function_properties.py','--candidate',str(candidate),'--reference-root',str(reference_root),'--source-root',str(source_root)]], limitations=['Ordinary function-typed instance properties are read once before argument evaluation, using ordinary getter, generic receiver specialization and accessibility rules. Applicable methods precede property invocation. Private enclosing receiver capture and mutable callback replacement are verified. Four Kotlin provider/frontend combinations and public nonsynthetic JVM signatures are compared. No Android runtime or full-application claim.'])
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
        command = [kotlin,source,'-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','member_function_properties'] if provider=='official' else [candidate,source,'--module-name','member_function_properties']
        return run('compile '+provider+' '+source.name,[*command,'-cp',cp(*deps,stdlib),'-d',output])
    try:
        dependency=work/'dependencies'
        dependency.mkdir()
        outputs={p:work/p for p in ['official','candidate']}
        for provider, output in outputs.items():
            compile(provider,FIXTURES/'Values.kt',output,[dependency])
        callers=work/'java'
        run('compile fixed Java caller against official',[jdk/'javac','-cp',cp(outputs['official'],dependency,stdlib),'-d',callers,FIXTURES/'Check.java'])
        signatures={}
        for provider,output in outputs.items():
            result=run('verified fixed Java caller '+provider,[jdk/'java','-Xverify:all','-cp',cp(callers,output,dependency,stdlib),'Check'])
            case('fixed-java/'+provider,result.stdout=='OK\n')
            signatures[provider]={}
            for class_name in ['memberfunctions.ValuesKt','memberfunctions.Request','memberfunctions.Captured','memberfunctions.Target','memberfunctions.Both','memberfunctions.Dispatch','memberfunctions.Multiplied','memberfunctions.Generic','memberfunctions.ExtensionHolder']:
                sig=run('JVM callable signatures '+provider+'/'+class_name,[jdk/'javap','-p','-s','-v','-cp',cp(output,dependency),class_name])
                signatures[provider][class_name]=public_signatures(sig.stdout)
        case('JVM-public-nonsynthetic-callable-signatures',signatures['official']==signatures['candidate'])
        launcher=None
        for provider,output in outputs.items():
            for frontend in ['official','candidate']:
                target=work/(provider+'-'+frontend+'-consumer')
                compile(frontend,FIXTURES/'Consumer.kt',target,[output,dependency])
                if launcher is None:
                    launcher=work/'consumer-java'
                    run('compile fixed consumer runner against official',[jdk/'javac','-cp',cp(target,output,dependency,stdlib),'-d',launcher,FIXTURES/'ConsumerCheck.java'])
                result=run('verified Kotlin consumer '+provider+'/'+frontend,[jdk/'java','-Xverify:all','-cp',cp(launcher,target,output,dependency,stdlib),'ConsumerCheck'])
                case('kotlin-consumer/'+provider+'/'+frontend,result.stdout=='OK\n')
        for item in json.loads((FIXTURES/'Rejections.json').read_text()):
            source=work/(item['id']+'.kt');source.write_text(item['source'])
            oracle=run('boundary official/'+item['id'],[kotlin,source,'-cp',cp(dependency,stdlib),'-d',work/('boundary-official-'+item['id'])],(0,1))
            result=run('boundary candidate/'+item['id'],[candidate,source,'--diagnostic-format','json','-cp',cp(dependency,stdlib),'-d',work/('boundary-candidate-'+item['id'])],(1,))
            diagnostics=[json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            expected='unsupported_language' if item.get('unsupported') else 'invalid_kotlin'
            assert oracle.returncode==(0 if item.get('unsupported') else 1)
            assert diagnostics[0]['category']==expected if item.get('unsupported') else diagnostics[0]['category'] in ['invalid_kotlin','unclassified']
            report['cases'].append(dict(id='boundary/'+item['id'],outcome='unsupported_language' if item.get('unsupported') else 'invalid_rejected',oracle_exit_code=oracle.returncode,candidate_exit_code=result.returncode,diagnostic=diagnostics,rejection_check='pass'))
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
