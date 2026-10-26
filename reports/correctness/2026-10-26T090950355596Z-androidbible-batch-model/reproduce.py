import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

ROOT = Path('/workspace/korrust')

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def tree(path):
    return {str(item.relative_to(path)): sha(item) for item in sorted(Path(path).rglob('*'))
            if item.is_file() and '.korrust' not in item.relative_to(path).parts}

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--snapshot',type=Path,default=ROOT/'target/checkpoint-ab-snapshot')
    args=parser.parse_args()
    snapshot=args.snapshot.resolve()
    now=datetime.now(timezone.utc)
    ident=now.strftime('%Y-%m-%dT%H%M%S%fZ')+'-androidbible-batch-model'
    evidence=ROOT/'reports/correctness'/ident
    evidence.mkdir(parents=True)
    target=snapshot/'target'/('androidbible-model-checkpoint-b-'+ident.split('-androidbible')[0])
    artifacts=target.with_name(target.name+'-artifacts')
    compiler=snapshot/'target/release/korrust'
    tool=snapshot/'tools/build_androidbible.py'
    gradle=Path('/workspace/korrust-setup/gradle-9.7.1/bin/gradle')
    sdk=Path('/workspace/korrust-setup/android-sdk')
    env=os.environ.copy()
    env.update(GRADLE_USER_HOME='/workspace/korrust-setup/gradle-home',ANDROID_USER_HOME='/workspace/korrust-setup/android-user',JAVA_TOOL_OPTIONS='-Djavax.net.ssl.trustStore=/etc/ssl/certs/java/cacerts -Dhttps.proxyHost=proxy -Dhttps.proxyPort=8080 -Dhttp.proxyHost=proxy -Dhttp.proxyPort=8080')
    report=dict(schema_version=1,kind='correctness',created_at=now.isoformat(),status='running',compiler={},reference_compiler={},inputs={},commands=[],evidence=[],cases=[],selection='Checkpoint B task10 AlkitabModel batch-integration revalidation: real clean official/candidate Gradle release AAR and debug unit-test task graphs, ordered resolved inputs and source ownership, fixed official-compiled Java callers, official Kotlin consumers, public signatures, declaration-removal rebuild against independent clean candidate/official output, and rejected-batch output preservation. This is existing module coverage, not new module support or task29 completion. Model has no original unit tests; no D8/device/full-app/performance claim.')
    def rel(p):
        try:return str(Path(p).relative_to(ROOT))
        except ValueError:return str(p)
    def save(path,name=None):
        destination=evidence/(name or Path(path).name)
        shutil.copy2(path,destination)
        report['evidence'].append(rel(destination))
        return destination
    def case(name,passed,**details):
        report['cases'].append(dict(id=name,outcome='pass' if passed else 'fail',**details))
        if not passed:raise AssertionError(name)
    def run(label,command,cwd=ROOT,environment=env,allowed=(0,)):
        command=list(map(str,command))
        print(label,flush=True)
        process=subprocess.run(command,cwd=cwd,env=environment,capture_output=True,text=True)
        index=len(report['commands'])
        entry=dict(label=label,argv=command,cwd=str(cwd),exit_code=process.returncode)
        for stream,value in [('stdout',process.stdout),('stderr',process.stderr)]:
            file=evidence/(f'{index:03d}-{stream}.txt');file.write_text(value);entry[stream]=rel(file);report['evidence'].append(rel(file))
        report['commands'].append(entry)
        if process.returncode not in allowed:raise RuntimeError(label+': '+process.stderr[-1500:]+process.stdout[-1500:])
        return process
    def cp(*paths):return os.pathsep.join(map(str,paths))
    def bundle(model,label):
        aar=save(model['aar'],label+'.aar')
        with zipfile.ZipFile(aar) as archive:payload=archive.read('classes.jar')
        jar=evidence/(label+'-classes.jar');jar.write_bytes(payload);report['evidence'].append(rel(jar))
        with zipfile.ZipFile(io.BytesIO(payload)) as archive:names=sorted(name for name in archive.namelist() if name.endswith('.class'))
        return jar,names
    def signatures(label,jar,names):
        output=run(label,['javap','-public','-s','-classpath',jar,*[name[:-6].replace('/','.') for name in names]]).stdout
        result={};owner=None
        for line in output.splitlines():
            if line.startswith('Compiled from') or not line.strip():continue
            if not line.startswith(' ') and line.endswith('{'):
                owner=line;result[owner]=[]
            elif line.strip()!='}':result[owner].append(line.strip())
        return {owner:sorted(members) for owner,members in result.items()}
    try:
        pin=json.loads((snapshot/'references/compiler.json').read_text())
        kotlinc=snapshot/pin['path']/'bin/kotlinc';stdlib=snapshot/pin['path']/'lib/kotlin-stdlib.jar'
        frozen=[snapshot/'Cargo.toml',snapshot/'Cargo.lock',*sorted((snapshot/'src').rglob('*.rs')),tool,snapshot/'tools/classpath_policy.py']
        report['compiler']=dict(path=rel(compiler),binary_sha256=sha(compiler),source_files_sha256={rel(path):sha(path) for path in frozen},version=run('Frozen candidate identity',[compiler,'--version']).stdout.strip(),jvm_target_scope='Official Gradle target21; candidate emits supported older classfile versions without a target-selection option.')
        report['reference_compiler']=dict(version=pin['version'],manifest_sha256=sha(snapshot/'references/compiler.json'),distribution_sha256={rel(p):sha(p) for p in sorted((snapshot/pin['path']/'lib').glob('*.jar'))})
        save(__file__,'reproduce.py');save(tool,'build_androidbible.py');save(snapshot/'tools/classpath_policy.py','classpath_policy.py')
        report['reproduction_commands']=[['python3',rel(evidence/'reproduce.py'),'--snapshot',str(snapshot)]]
        report['environment']=dict(java=run('JDK identity',['java','-version']).stderr.strip(),sdk=str(sdk),gradle=str(gradle),proxy_preserved=True,gradle_user_home=env['GRADLE_USER_HOME'])
        models={};builds={};jars={};names={}
        for provider in ['official','korrust']:
            command=[sys.executable,tool,'--mode',provider,'--modules','AlkitabModel','--gradle',gradle,'--sdk',sdk,'--compiler',compiler,'--work-dir',target,'--artifact-dir',artifacts]
            run('Clean real Gradle '+provider+' Model build',command)
            build=json.loads((artifacts/provider/'build.json').read_text());builds[provider]=build;models[provider]=build['modules']['AlkitabModel']
            module=artifacts/provider/'modules/AlkitabModel'
            save(artifacts/provider/'build.json',provider+'-build.json');save(artifacts/provider/'gradle.log',provider+'-gradle.log');save(module/'gradle-model.json',provider+'-gradle-model.json')
            if provider=='korrust':
                for task in ['compileDebugKotlin','compileReleaseKotlin']:
                    for suffix in ['.json','-request.json']:save(module/(task+suffix),'candidate-'+task+suffix)
                    trace=json.loads((module/(task+'.json')).read_text());request=json.loads((module/(task+'-request.json')).read_text())
                    case('one_batch/'+task,len(trace['invocations'])==1 and trace['invocations'][0]['sources']==request['sources'] and '--module-name' in trace['invocations'][0]['command'] and not trace['fallback'],source_hashes=trace['sources'])
            case('gradle/'+provider,build['status']=='passed' and build['sources_unchanged'] and not build['production_kotlin_fallback'],module='AlkitabModel',test_scope='Original Model has no tests; testDebugUnitTest graph completed with NO-SOURCE test tasks.',source_hashes=build['source_files_sha256'],missing_required_inputs=build.get('missing_required_inputs',{}))
            jars[provider],names[provider]=bundle(models[provider],provider)
        report['inputs']['source_files_sha256']=builds['official']['source_files_sha256']
        case('aar_class_inventory',names['official']==names['korrust'],classes=names['official'])
        case('public_signatures',signatures('Official public signatures',jars['official'],names['official'])==signatures('Candidate public signatures',jars['korrust'],names['korrust']))
        java=evidence/'Caller.java';java.write_text('''import yuku.alkitab.model.SingleChapterVerses;
public class Caller {
 public static void main(String[] args) {
  SingleChapterVerses empty=SingleChapterVerses.Companion.getEMPTY();
  if(empty.getVerseCount()!=0) throw new AssertionError("count");
  try { empty.getVerse(0); throw new AssertionError("missing failure"); }
  catch (IllegalArgumentException e) { if(!e.getMessage().equals("@@getVerse requesting verse_0=0 but verseCount=0")) throw new AssertionError(e); }
  System.out.println("PASS");
 }
}
''');report['evidence'].append(rel(java));drivers=target.parent/(target.name+'-java')
        run('Compile fixed Java caller against official only',['javac','-cp',jars['official'],'-d',drivers,java])
        report['inputs']['fixed_java_bytecode_sha256']=tree(drivers)
        kotlin=evidence/'Consumer.kt';kotlin.write_text('''import yuku.alkitab.model.SingleChapterVerses
fun main() {
 check(SingleChapterVerses.EMPTY.verseCount == 0)
 val verses = object : SingleChapterVerses {
  override val verseCount: Int get() = 50
  override fun getVerse(verse_0: Int): String = "verse"
 }
 check(verses.getVerseNumberText(0) == "1")
 check(verses.getVerseNumberText(38) == "39")
 check(verses.getVerseNumberText(39) == "40")
 println("PASS")
}
''');report['evidence'].append(rel(kotlin))
        for provider,jar in jars.items():
            case('java_runtime/'+provider,run('Fixed Java caller '+provider,['java','-Xverify:all','-cp',cp(drivers,jar,stdlib),'Caller']).stdout=='PASS\n')
            output=target.parent/(target.name+'-consumer-'+provider)
            run('Kotlin metadata consumer compile '+provider,[kotlinc,kotlin,'-cp',jar,'-d',output])
            case('kotlin_runtime/'+provider,run('Kotlin consumer '+provider,['java','-Xverify:all','-cp',cp(output,jar,stdlib),'ConsumerKt']).stdout=='PASS\n')
        source=target/'AlkitabModel/src/main/java/yuku/alkitab/model/SingleChapterVerses.kt'
        old=source.read_text();removed='''    interface WithTextSizeMult : SingleChapterVerses {
        fun getTextSizeMult(verse_0: Int): Float
    }
'''
        assert removed in old
        changed=old.replace(removed,'').replace('interface SingleChapterVerses {','interface SingleChapterVerses {\n    fun revisionMarker(): Int = 17\n')
        source.write_text(changed);save(source,'SingleChapterVerses-changed.kt')
        report['inputs']['deliberate_edit']=dict(original_sha256=hashlib.sha256(old.encode()).hexdigest(),changed_sha256=sha(source),scope='Only disposable app copy. Remove nested WithTextSizeMult declaration and add revisionMarker returning17.')
        sys.path.insert(0,str(snapshot/'tools'))
        import build_androidbible as helper
        def direct(label,work,mode,clean):
            out=artifacts/label;module=out/'modules/AlkitabModel';module.mkdir(parents=True)
            init=out/'adapter.init.gradle';init.write_text(helper.INIT_SCRIPT)
            local=env.copy();local.update(KORRUST_BUILD_MODE=mode,KORRUST_BUILD_MODULES=json.dumps(['AlkitabModel']),KORRUST_BUILD_MODULE_ARTIFACTS=json.dumps({'AlkitabModel':str(module)}),KORRUST_BUILD_COMPILER=str(compiler),KORRUST_BUILD_PYTHON=sys.executable,KORRUST_BUILD_TOOL=str(tool),KORRUST_BUILD_ARTIFACTS=str(out),KORRUST_BUILD_MODEL=str(module/'gradle-model.json'))
            command=[gradle,'--init-script',init,'--no-configuration-cache','--no-build-cache','--max-workers=4','--console=plain','--stacktrace']
            if clean:command+=[':AlkitabModel:clean']
            command+=[':AlkitabModel:bundleReleaseAar',':AlkitabModel:testDebugUnitTest',':AlkitabModel:korrustBuildModel']
            result=run('Changed real Gradle '+label,command,cwd=work,environment=local)
            log=out/'gradle.log';log.write_text(result.stdout+result.stderr);save(log,label+'-gradle.log');save(module/'gradle-model.json',label+'-model.json')
            model=json.loads((module/'gradle-model.json').read_text())
            jar,inventory=bundle(model,label)
            classes=Path(model['kotlin_classes']);class_hashes=tree(classes)
            save(init,label+'-adapter.init.gradle')
            if mode=='korrust':
                for task in ['compileReleaseKotlin','compileDebugKotlin']:
                    save(module/(task+'.json'),label+'-'+task+'.json')
            return model,jar,inventory,class_hashes
        incremental,incjar,incnames,inchashes=direct('candidate-changed-incremental',target,'korrust',False)
        cleanwork=target.with_name(target.name+'-independent-clean')
        shutil.copytree(target,cleanwork,symlinks=True,ignore=shutil.ignore_patterns('.gradle','build'))
        clean,cleanjar,cleannames,cleanhashes=direct('candidate-changed-clean',cleanwork,'korrust',True)
        official,officialjar,officialnames,_=direct('official-changed-clean',cleanwork,'official',True)
        case('declaration_change/artifacts_match_clean',inchashes==cleanhashes,incremental=inchashes,independent_clean=cleanhashes)
        case('declaration_change/stale_nested_classes_removed',not any('WithTextSizeMult' in name for name in incnames) and incnames==officialnames,classes=incnames)
        case('declaration_change/public_signatures',signatures('Official changed signatures',officialjar,officialnames)==signatures('Candidate incremental changed signatures',incjar,incnames))
        mutation=evidence/'ChangedConsumer.kt';mutation.write_text('''import yuku.alkitab.model.SingleChapterVerses
fun main() {
 check(SingleChapterVerses.EMPTY.revisionMarker() == 17)
 println("CHANGED-17")
}
''');report['evidence'].append(rel(mutation));consumer=target.parent/(target.name+'-changed-consumer')
        run('Compile fixed changed Kotlin consumer official',[kotlinc,mutation,'-cp',officialjar,'-d',consumer])
        for provider,jar in [('official',officialjar),('incremental',incjar),('clean',cleanjar)]:
            case('declaration_change/runtime/'+provider,run('Fixed changed consumer '+provider,['java','-Xverify:all','-cp',cp(consumer,jar,stdlib),'ChangedConsumerKt']).stdout=='CHANGED-17\n')
        request=json.loads((artifacts/'candidate-changed-incremental/modules/AlkitabModel/compileReleaseKotlin-request.json').read_text())
        before=tree(Path(request['destination']))
        source.write_text('package yuku.alkitab.model\ninterface SingleChapterVerses { invalid syntax }\n')
        requestpath=artifacts/'invalid-request.json';requestpath.write_text(json.dumps(request,indent=2)+'\n');save(requestpath)
        rejected=run('Reject invalid complete batch without publishing',[sys.executable,tool,'--compile-request',requestpath],allowed=(1,))
        case('invalid_batch/prior_output_preserved',tree(Path(request['destination']))==before,stderr=rejected.stderr[-2000:])
        source.write_text(changed)
        case('frozen_inputs_unchanged',sha(compiler)==report['compiler']['binary_sha256'] and all(sha(ROOT/path)==checksum for path,checksum in report['compiler']['source_files_sha256'].items()))
    except Exception as error:
        report['cases'].append(dict(id='harness_failure',outcome='fail',diagnostic=repr(error)))
        print(repr(error),file=sys.stderr)
    report['counts']=dict(sorted(Counter(case['outcome'] for case in report['cases']).items()))
    report['status']='passed' if report['cases'] and all(case['outcome']=='pass' for case in report['cases']) else 'failed'
    report['evidence_files_sha256']={path:sha(ROOT/path) for path in report['evidence']}
    reportpath=evidence.with_suffix('.json');reportpath.write_text(json.dumps(report,indent=2)+'\n')
    mirror=ROOT/'target/androidbible-model-checkpoint-b-evidence'/ident;mirror.mkdir(parents=True);shutil.copy2(reportpath,mirror/reportpath.name)
    print(str(reportpath),report['status'],report['counts'],flush=True)
    raise SystemExit(report['status']!='passed')

if __name__=='__main__':main()
