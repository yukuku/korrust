#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/sealed_new'
ORIGINAL = ROOT / 'references/kotlin/compiler/testData/codegen/box/sealed/simple.kt'
APPS = [ROOT / ('benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/' + name) for name in ['versionmanager/VersionFileImporter.kt', 'base/audio/ui/AudioBar.kt']]


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
def cp(*paths): return os.pathsep.join(map(str, paths))
def signatures(output):
    return sorted(line.strip() for line in output.splitlines() if line.strip().startswith(('public ', 'protected ', 'private ', 'descriptor:')) and ' #' not in line and (';' in line or line.strip().startswith('descriptor:') or line.endswith('{')))


class Check:
    def __init__(self, args):
        self.args = args
        self.compiler = args.compiler.resolve()
        self.now = datetime.now(timezone.utc)
        self.ident = self.now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-new-sealed'
        self.work = ROOT / 'target/checks' / self.ident
        self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True)
        self.logs.mkdir(parents=True)
        self.pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlin = ROOT / self.pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / self.pin['path'] / 'lib/kotlin-stdlib.jar'
        home = Path(os.environ['JAVA_HOME'])
        self.java, self.javac, self.javap = [home / 'bin' / n for n in ['java', 'javac', 'javap']]
        self.app_prerequisites=json.loads(args.app_prerequisites.read_text())
        self.app_capture_path=args.app_capture
        self.app_capture=json.loads(self.app_capture_path.read_text())
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_sealed']
        frozen = [*args.source_root.glob('src/**/*.rs'), args.source_root/'Cargo.toml', args.source_root/'Cargo.lock', *FIXTURES.glob('*'), Path(__file__), ORIGINAL, *APPS, ROOT/'base-manifest.json',args.app_prerequisites,self.app_capture_path]
        self.frozen = {relative(p): sha(p) for p in sorted(frozen) if p.is_file()}
        self.report = dict(schema_version=1, kind='correctness', created_at=self.now.isoformat(), status='running', compiler={'binary':relative(self.compiler), 'binary_sha256':sha(self.compiler), 'frozen_source_root':relative(args.source_root)}, reference_compiler={'version':self.pin['version'], 'archive_sha256':self.pin['archive_sha256'], 'options':self.options}, inputs={'files_sha256':self.frozen, 'stdlib_sha256':sha(self.stdlib)}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3','tools/check_new_sealed.py','--compiler',relative(self.compiler),'--source-root',relative(args.source_root),'--app-prerequisites',str(args.app_prerequisites),'--app-capture',str(self.app_capture_path)]], limitations=['Named non-generic sealed classes/interfaces in the current source batch and same package; implicit protected source constructors lower to private JVM constructors with public synthetic marker accessors; deterministic direct-child Kotlin metadata. Source generic sealed classifiers and explicit/default/secondary sealed constructors remain unsupported.', 'Exhaustive when supports bounded sealed closures, direct non-sealed leaves, sealed intermediates, objects, enum entries, booleans, nullable subjects, appropriate is/!is cases, branch result joins of equal/subtype/nullable types, returns/throws and definite assignment. Generic/intersection/flow-sensitive closure analysis and result inference for all-abrupt branches remain explicit unsupported boundaries. Uninhabited no-else when is rejected like the pinned compiler.', 'JVM 1.8 fixed Java assertions, official Kotlin consumers, one unchanged pinned upstream original. Actual whole AndroidBible files are separately assessed and remain blocked; no extracted app fragment, Android runtime, full-app JVM21/plugin, performance, or complete Kotlin claim.'])

    def run(self, label, args, accepted=(0,), timeout=300):
        index = len(self.report['commands'])
        started=time.monotonic()
        result = subprocess.run(list(map(str,args)), cwd=ROOT, capture_output=True, text=True, timeout=timeout)
        entry = {'label':label, 'argv':list(map(str,args)), 'cwd':'.', 'exit_code':result.returncode, 'wall_time_seconds':time.monotonic()-started}
        for stream in ['stdout','stderr']:
            path = self.logs / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result,stream))
            entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted: raise AssertionError(label+'\n'+result.stdout+result.stderr)
        return result

    def case(self, ident, passed, **fields):
        self.report['cases'].append({'id':ident,'outcome':'pass' if passed else 'correctness_failure', **fields})

    def boundary(self, ident, sources, official, code, dependencies=()):
        directory = self.work/'boundaries'/ident
        directory.mkdir(parents=True)
        paths=[]
        for name,source in sources.items():
            path=directory/name; path.write_text(source); paths.append(path)
        oracle=self.run('Official boundary '+ident,[self.kotlin,*paths,*self.options,'-cp',cp(self.stdlib,*dependencies),'-d',directory/'official'],accepted=(0,1))
        actual=self.run('Candidate boundary '+ident,[self.compiler,*paths,'--module-name','new_sealed','--diagnostic-format','json','-cp',cp(self.stdlib,*dependencies),'-d',directory/'candidate'],accepted=(0,1))
        diagnostics=[json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
        category='invalid_kotlin' if official=='reject' else 'unsupported_language'
        valid=(oracle.returncode==0)==(official=='accept') and actual.returncode!=0 and any(d.get('code')==code and d.get('category')==category for d in diagnostics)
        self.report['cases'].append({'id':'boundary/'+ident,'outcome':('invalid_rejected' if official=='reject' else 'unsupported_language') if valid else 'correctness_failure','official_accepted':oracle.returncode==0,'diagnostics':diagnostics,'source_sha256':{relative(p):sha(p) for p in paths}})

    def execute(self):
        dependency=self.work/'dependency'
        self.run('Independent observation dependency',[self.javac,'-d',dependency,FIXTURES/'Probe.java'])
        outputs={provider:self.work/provider for provider in ['official','candidate']}
        self.run('Official sealed declarations',[self.kotlin,FIXTURES/'Use.kt',*self.options,'-cp',dependency,'-d',outputs['official']])
        self.run('Candidate sealed declarations',[self.compiler,FIXTURES/'Use.kt','--module-name','new_sealed','-cp',cp(dependency,self.stdlib),'-d',outputs['candidate']])
        callers=self.work/'callers'
        self.run('Compile fixed Java assertions once against official',[self.javac,'-cp',cp(outputs['official'],dependency,self.stdlib),'-d',callers,FIXTURES/'Calls.java'])
        self.run('Compile same Java assertions against candidate signatures',[self.javac,'-cp',cp(outputs['candidate'],dependency,self.stdlib),'-d',self.work/'candidate-signature-consumer',FIXTURES/'Calls.java'])
        for provider,output in outputs.items():
            observed=self.run('Run identical fixed Java bytecodes '+provider,[self.java,'-Xverify:all','-cp',cp(callers,output,dependency,self.stdlib),'Calls'])
            self.case('java/'+provider,observed.stdout=='OK\n',reused_assertion_bytecodes=True,scope='Class/interface closure, nullable booleans, enums/objects, branch joins, exceptions/null guards, unknown Java subclass, wide constructor values, data leaves, evaluation count/order, return/assignment')
            for consumer_kind in ['official','candidate']:
                consumer=self.work/(provider+'-'+consumer_kind+'-consumer')
                if consumer_kind=='official': args=[self.kotlin,FIXTURES/'Consumer.kt',*self.options,'-cp',cp(output,dependency),'-d',consumer]
                else: args=[self.compiler,FIXTURES/'Consumer.kt','-cp',cp(output,dependency,self.stdlib),'-d',consumer]
                self.run(consumer_kind+' Kotlin exhaustive metadata consumer '+provider,args)
                launcher=self.work/(provider+'-'+consumer_kind+'-launcher')
                self.run('Consumer launcher '+provider+'/'+consumer_kind,[self.javac,'-cp',cp(consumer,output,dependency,self.stdlib),'-d',launcher,FIXTURES/'ConsumerCalls.java'])
                observed=self.run('Run exhaustive metadata consumer '+provider+'/'+consumer_kind,[self.java,'-Xverify:all','-cp',cp(launcher,consumer,output,dependency,self.stdlib),'ConsumerCalls'])
                self.case('metadata/'+provider+'/'+consumer_kind,observed.stdout=='OK\n')
        for owner in ['Choice','Packet','Tree','Branch','Signal','Action','Outcome','Operation','AbstractBase']:
            inspected={provider:self.run('Inspect sealed ABI '+provider+'/'+owner,[self.javap,'-p','-s','-v','-classpath',output,'sealednew.'+owner]).stdout for provider,output in outputs.items()}
            self.case('abi/'+owner,signatures(inspected['official'])==signatures(inspected['candidate']),expected=signatures(inspected['official']),actual=signatures(inspected['candidate']))
            if owner in ['Choice','Packet','Tree','Branch','Outcome','AbstractBase']:
                self.case('constructor_flags/'+owner,all(re.search(r'public sealednew\.'+owner+r'\([^\n]*DefaultConstructorMarker\);\s+descriptor:[^\n]+\s+flags: \(0x1001\) ACC_PUBLIC, ACC_SYNTHETIC',text) for text in inspected.values()) and all(re.search(r'private sealednew\.'+owner+r'\([^\n]*\);\s+descriptor:[^\n]+\s+flags: \(0x0002\) ACC_PRIVATE',text) for text in inspected.values()))
        for item in json.loads((FIXTURES/'Rejections.json').read_text()):
            self.boundary(item['id'],{'Boundary.kt':item['source']},item['official'],item['code'])
        self.boundary('source_batch_limit',{'Root.kt':'sealed interface S','First.kt':'\n'.join('class C'+str(n)+':S' for n in range(128)),'Rest.kt':'\n'.join('class C'+str(n)+':S' for n in range(128,257))},'accept','SOURCE_DECLARATION_LIMIT_UNSUPPORTED')
        self.boundary('cross_package',{'Root.kt':'package parent\nsealed interface Root','Child.kt':'package child\nclass Child:parent.Root'},'reject','SEALED_INHERITANCE_INVALID')
        for provider,output in outputs.items():
            self.boundary('external_module_'+provider,{'Child.kt':'package sealednew\nclass Forbidden: Signal'},'reject','SEALED_INHERITANCE_INVALID',(output,dependency))
        batch=self.work/'batch';batch.mkdir()
        root=batch/'Root.kt';root.write_text('package batch\nsealed interface Root')
        child=batch/'Children.kt';child.write_text('package batch\nclass First:Root\nclass Second:Root\nfun selected(v:Root):Int=when(v){is First->1;is Second->2}')
        batch_outputs={p:batch/p for p in outputs}
        self.run('Official same-package same-source-module separate files',[self.kotlin,root,child,*self.options,'-d',batch_outputs['official']])
        self.run('Candidate same-package same-source-module separate files',[self.compiler,root,child,'-cp',self.stdlib,'-d',batch_outputs['candidate']])
        driver=batch/'BatchCalls.java';driver.write_text('public class BatchCalls { public static void main(String[] args) { if(batch.ChildrenKt.selected(new batch.First())!=1 || batch.ChildrenKt.selected(new batch.Second())!=2)throw new AssertionError(); System.out.println("OK"); }}')
        caller=batch/'callers';self.run('Compile batch fixed Java caller',[self.javac,'-cp',cp(batch_outputs['official'],self.stdlib),'-d',caller,driver])
        for provider,output in batch_outputs.items():
            result=self.run('Execute batch '+provider,[self.java,'-Xverify:all','-cp',cp(caller,output,self.stdlib),'BatchCalls'])
            self.case('source_module/'+provider,result.stdout=='OK\n')
        upstream={p:self.work/('upstream-'+p) for p in outputs}
        self.run('Unchanged pinned sealed upstream official',[self.kotlin,ORIGINAL,*self.options,'-d',upstream['official']])
        self.run('Unchanged pinned sealed upstream candidate',[self.compiler,ORIGINAL,'-cp',self.stdlib,'-d',upstream['candidate']])
        driver=self.work/'UpstreamCalls.java';driver.write_text('public class UpstreamCalls { public static void main(String[] args) { if(!SimpleKt.box().equals("OK"))throw new AssertionError(); System.out.println("OK"); }}')
        caller=self.work/'upstream-callers';self.run('Compile fixed unchanged upstream Java caller',[self.javac,'-cp',cp(upstream['official'],self.stdlib),'-d',caller,driver])
        for provider,output in upstream.items():
            result=self.run('Execute unchanged upstream '+provider,[self.java,'-Xverify:all','-cp',cp(caller,output,self.stdlib),'UpstreamCalls'])
            self.case('upstream/simple/'+provider,result.stdout=='OK\n',original=relative(ORIGINAL),source_sha256=sha(ORIGINAL))
        self.report['emitted_classes_sha256']={p:{relative(f):sha(f) for f in output.rglob('*.class')} for p,output in outputs.items()}
        task=self.app_capture['modules']['Alkitab']['compilation_tasks']['compilePlainDebugKotlin']
        declared=[Path(p) for p in task['compile_classpath']]
        available=[p for p in declared if p.exists()]
        self.report['app_prerequisites']={'report':str(self.args.app_prerequisites),'report_sha256':sha(self.args.app_prerequisites),'status':self.app_prerequisites['status'],'capture':str(self.app_capture_path),'capture_sha256':sha(self.app_capture_path),'actual_compiler_options':task['compiler_options'],'actual_plugin_classpath':task['compiler_plugin_classpath'],'declared_classpath':list(map(str,declared)),'unavailable_empty_or_optional_classpath':list(map(str,[p for p in declared if not p.exists()])),'existing_classpath_files_sha256':{str(p):sha(p) for p in available if p.is_file()},'assessment_scope':'Candidate parser-stage first blocker only; original complete production Kotlin+Java compilation is established in the linked report. Captured JVM21+plugins are separate from the JVM1.8 family fixture profile.'}
        for source,code in zip(APPS,['DISCARD_CATCH_BINDING_UNSUPPORTED','DATA_OBJECT_UNSUPPORTED']):
            snapshot_key=str(source).split('/benchmarks/sources/androidbible/',1)[1]
            recorded=self.app_prerequisites['inputs']['source_files_sha256'][snapshot_key]
            if sha(source)!=recorded: raise AssertionError('App prerequisite source mismatch '+str(source))
            actual=self.run('Actual unchanged AndroidBible parser probe with real captured dependencies '+source.name,[self.compiler,source,'--diagnostic-format','json','-cp',cp(*available),'-d',self.work/('app-'+source.stem)],accepted=(0,1))
            diagnostics=[json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            self.report['cases'].append({'id':'app/'+source.stem,'outcome':'unsupported_language' if actual.returncode and diagnostics and diagnostics[0].get('category')=='unsupported_language' and diagnostics[0].get('code')==code else 'unclassified_compilation_failure','acceptance_scope':'external_app_assessment','original':relative(source),'source_sha256':sha(source),'diagnostics':diagnostics,'oracle_scope':'Pinned production source verified separately by captured actual Android build; standalone candidate parser stops before dependency resolution. No wholefile pass claimed.'})

    def finish(self):
        changed=[path for path,checksum in self.frozen.items() if sha(ROOT/path)!=checksum]
        self.case('frozen_inputs',not changed and sha(self.compiler)==self.report['compiler']['binary_sha256'],changed=changed)
        self.report['counts']=dict(Counter(case['outcome'] for case in self.report['cases']))
        selected=[case for case in self.report['cases'] if case.get('acceptance_scope')!='external_app_assessment']
        counts=dict(Counter(case['outcome'] for case in selected))
        failed=counts.get('correctness_failure',0)+counts.get('unclassified_compilation_failure',0)
        self.report['feature_acceptance']={'family':'sealed_classifiers_and_exhaustive_when','status':'failed' if failed else 'passed','derived_counts':counts,'excluded_assessment_cases':[case['id'] for case in self.report['cases'] if case.get('acceptance_scope')=='external_app_assessment']}
        self.report['status']='failed' if failed else 'completed'
        path=ROOT/'reports/correctness'/(self.ident+'.json');path.write_text(json.dumps(self.report,indent=2)+'\n')
        subprocess.run(['python3','tools/reports.py'],cwd=ROOT,check=True,capture_output=True,text=True)
        print(relative(path));print(json.dumps(self.report['counts'],sort_keys=True));return 1 if failed else 0


def main():
    parser=argparse.ArgumentParser();parser.add_argument('--compiler',type=Path,required=True);parser.add_argument('--source-root',type=Path,default=ROOT);parser.add_argument('--app-prerequisites',type=Path,required=True);parser.add_argument('--app-capture',type=Path,required=True)
    args=parser.parse_args();args.source_root=args.source_root.resolve();check=Check(args)
    try: check.execute()
    except Exception as error: check.report['cases'].append({'id':'harness','outcome':'correctness_failure','error':str(error)})
    return check.finish()

if __name__=='__main__': raise SystemExit(main())
