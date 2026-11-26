#!/usr/bin/env python3
"""Frozen differential evidence for Kotlin nullable toString builtin."""
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
FIXTURES = ROOT / 'tests/nullable_to_string'


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
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-nullable-to-string'
    work = ROOT / 'target/checks' / ident
    logs = ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True)
    logs.mkdir(parents=True)
    def rel(p):
        return str(p.relative_to(ROOT)) if p.is_relative_to(ROOT) else str(p)
    inputs = [*source_root.glob('src/**/*.rs'), *FIXTURES.rglob('*'), ROOT / 'tests/nullable_to_string.rs', Path(__file__), ROOT / 'tools/check_nov21_generics.py', *[ROOT/path for path in ['references/kotlin/libraries/stdlib/jvm/builtins/Library.kt','references/kotlin/compiler/ir/backend.common/src/org/jetbrains/kotlin/backend/common/lower/FlattenStringConcatenationLowering.kt','references/kotlin/compiler/ir/backend.jvm/src/org/jetbrains/kotlin/backend/jvm/JvmSymbols.kt','references/kotlin/compiler/ir/backend.jvm/codegen/src/org/jetbrains/kotlin/backend/jvm/intrinsics/IrIntrinsicMethods.kt']], source_root / 'Cargo.toml', source_root / 'Cargo.lock', reference_root / 'references/compiler.json']
    frozen = {rel(p):sha(p) for p in sorted(inputs) if p.is_file()}
    bundle=logs/'frozen-inputs.zip'
    with zipfile.ZipFile(bundle,'w',zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            if path.is_file(): archive.writestr(rel(path),path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler=dict(binary=rel(candidate), binary_sha256=sha(candidate), source_revision=source_revision), source_archive=dict(path=rel(bundle),sha256=sha(bundle)), reference_compiler=dict(version=pin['version'],archive_sha256=pin['archive_sha256'],compiler_jar_sha256=sha(reference / 'lib/kotlin-compiler.jar')), inputs=dict(files_sha256=frozen,stdlib_sha256=sha(stdlib)), commands=[],cases=[], evidence=[], reproduction_commands=[['python3','tools/check_nullable_to_string.py','--candidate',str(candidate),'--reference-root',str(reference_root),'--source-root',str(source_root)]], limitations=['The canonical kotlin.Any?.toString builtin declaration is validated from pinned Kotlin builtin metadata and mapped to its public String.valueOf(Object) JVM target through normal extension selection. Source, import and alias precedence remain in ordinary resolution. No Android runtime or full-app claim.'])
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
        sources = list(source) if isinstance(source,list) else [source]
        command = [kotlin,*sources,'-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','nullable_to_string'] if provider=='official' else [candidate,*sources,'--module-name','nullable_to_string']
        return run('compile '+provider+' '+'/'.join(s.name for s in sources),[*command,'-cp',cp(*deps,stdlib),'-d',output])
    try:
        dependency=work/'dependencies'
        run('compile imported Kotlin shadow extension',[kotlin,FIXTURES/'Library.kt','-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-d',dependency])
        run('compile Java override provider',[jdk/'javac','-d',dependency,*sorted((FIXTURES/'java').rglob('*.java'))])
        outputs={p:work/p for p in ['official','candidate']}
        for provider, output in outputs.items():
            compile(provider,[FIXTURES/(name+'.kt') for name in ['Effects','Local','Imported','Alias','Member']],output,[dependency])
        callers=work/'java'
        run('compile fixed Java caller against official',[jdk/'javac','-cp',cp(outputs['official'],dependency,stdlib),'-d',callers,FIXTURES/'Calls.java'])
        signatures={}
        for provider,output in outputs.items():
            result=run('verified fixed Java caller '+provider,[jdk/'java','-Xverify:all','-cp',cp(callers,output,dependency,stdlib),'Calls'])
            case('fixed-java/'+provider,result.stdout=='OK\n')
            signatures[provider]={}
            for class_name in ['EffectsKt','localshadow.LocalKt','importedshadow.ImportedKt','aliasshadow.AliasKt','membershadow.MemberKt']:
                sig=run('JVM callable signatures '+provider+'/'+class_name,[jdk/'javap','-p','-s','-v','-cp',output,class_name])
                signatures[provider][class_name]=callable_signatures(sig.stdout)
            for consumer in ['official','candidate']:
                consumer_out=work/(provider+'-'+consumer)
                compile(consumer,FIXTURES/'Consumer.kt',consumer_out,[output,dependency])
                runner=work/(provider+'-'+consumer+'-runner')
                run('compile consumer runner',[jdk/'javac','-cp',cp(output,consumer_out,dependency,stdlib),'-d',runner,FIXTURES/'ConsumeCalls.java'])
                result=run('verified Kotlin consumer '+provider+'/'+consumer,[jdk/'java','-Xverify:all','-cp',cp(runner,consumer_out,output,dependency,stdlib),'ConsumeCalls'])
                case('kotlin-consumer/'+provider+'/'+consumer,result.stdout=='OK\n')
        for provider, output in outputs.items():
            for item in json.loads((FIXTURES/'ConsumerRejections.json').read_text()):
                source=work/('consumer-negative-'+item['id']+'.kt');source.write_text(item['source'])
                for consumer in ['official','candidate']:
                    command=[kotlin,source] if consumer=='official' else [candidate,source,'--diagnostic-format','json']
                    result=run('consumer negative '+provider+'/'+consumer+'/'+item['id'],[*command,'-cp',cp(output,dependency,stdlib),'-d',work/('negative-consumer-'+consumer)],(1,))
                    case('consumer-negative/'+provider+'/'+consumer+'/'+item['id'],result.returncode==1)
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
