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
from upstream_jvm import parse_fixture, assess, options_for

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/class_delegation_new'
UPSTREAM_ROOT = ROOT / 'references/kotlin/compiler/testData/codegen/box/delegation'
UPSTREAMS = [('delegationDifferentModule.kt',True),('delegationDifferentModule2.kt',True),('inClassDeclaration.kt',False),('kt30102_comparable.kt',False)]
APP = ROOT/'benchmarks/sources/androidbible'


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def relative(path):
    path=Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
def cp(*paths): return os.pathsep.join(map(str,paths))
def signatures(output):
    result=[]
    for line in output.splitlines():
        line=line.strip()
        if line.startswith(('public ','protected ','descriptor:')) and ' #' not in line and (';' in line or line.startswith('descriptor:') or line.endswith('{')): result.append(line)
        elif line.startswith('Signature:') and '// ' in line: result.append('Signature: '+line.split('// ',1)[1])
    return sorted(result)


class Check:
    def __init__(self,args):
        self.args=args;self.compiler=args.compiler.resolve();self.now=datetime.now(timezone.utc)
        self.ident=self.now.strftime('%Y-%m-%dT%H%M%S%fZ')+'-new-class-delegation'
        self.work=ROOT/'target/checks'/self.ident;self.logs=ROOT/'reports/correctness'/self.ident
        self.work.mkdir(parents=True);self.logs.mkdir(parents=True)
        self.pin=json.loads((ROOT/'references/compiler.json').read_text())
        self.kotlin=ROOT/self.pin['path']/'bin/kotlinc';self.stdlib=ROOT/self.pin['path']/'lib/kotlin-stdlib.jar';self.compiler_jar=ROOT/self.pin['path']/'lib/kotlin-compiler.jar'
        home=Path(os.environ['JAVA_HOME']);self.java,self.javac,self.javap=[home/'bin'/n for n in ['java','javac','javap']]
        self.options=['-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','class_delegation']
        frozen=[*args.source_root.glob('src/**/*.rs'),args.source_root/'Cargo.toml',args.source_root/'Cargo.lock',*FIXTURES.glob('*'),Path(__file__),ROOT/'tools/upstream_jvm.py',ROOT/'base-manifest.json',*[UPSTREAM_ROOT/n for n,_ in UPSTREAMS],*APP.rglob('*.kt')]
        self.frozen={relative(p):sha(p) for p in sorted(frozen) if p.is_file()}
        self.report=dict(schema_version=1,kind='correctness',created_at=self.now.isoformat(),status='running',compiler={'binary':relative(self.compiler),'binary_sha256':sha(self.compiler),'frozen_source_root':relative(args.source_root)},reference_compiler={'version':self.pin['version'],'archive_sha256':self.pin['archive_sha256'],'options':self.options,'compiler_jar_sha256':sha(self.compiler_jar)},inputs={'files_sha256':self.frozen,'stdlib_sha256':sha(self.stdlib)},commands=[],cases=[],evidence=[],reproduction_commands=[['python3','tools/check_new_class_delegation.py','--compiler',relative(self.compiler),'--source-root',relative(args.source_root)]],limitations=['One new family: class-header interface delegation. Existing property delegation, source classifiers/generics and ordinary interface implementation remain prerequisites. Saved private-final-synthetic receivers, one evaluation after super construction and before property/init execution, disjoint multiple delegates, explicit overrides, interface getter/setter forwarding, generic substitution/erased bridges and delegated Kotlin member metadata are covered.','Public top-level interfaces and ordinary top-level delegators; invariant Kotlin interface parameters and nongeneric Java scalar/reference contracts. Default bodies, internal/nested/dollar-named classifier/type shapes, delegating objects/data classes/enums/secondary constructors/explicit superclass combinations, decorated/generic/extension/renamed/vararg/default-argument members, complex overlapping erased signatures and Java generic/raw/reference-array member types remain explicit boundaries. Extra private saved fields are permitted; final constructor backing-field reuse is not an optimization goal.','Fixed Java assertions compile once against official and execute against both outputs with verification. Separate official Kotlin consumers and metadata flag/ABI checks. Original upstream multi-module files/directives are preserved. Pinned PSI inventory establishes zero AndroidBible header delegation uses; no new app-file, APK/device/full-app, performance or complete Kotlin claim.'])

    def run(self,label,args,accepted=(0,),timeout=300):
        started=time.monotonic();index=len(self.report['commands'])
        result=subprocess.run(list(map(str,args)),cwd=ROOT,capture_output=True,text=True,timeout=timeout)
        entry={'label':label,'argv':list(map(str,args)),'cwd':'.','exit_code':result.returncode,'wall_time_seconds':time.monotonic()-started}
        for stream in ['stdout','stderr']:
            path=self.logs/f'{index:03d}-{stream}.txt';path.write_text(getattr(result,stream));entry[stream]=relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted: raise AssertionError(label+'\n'+result.stdout+result.stderr)
        return result

    def case(self,ident,passed,**fields):
        self.report['cases'].append({'id':ident,'outcome':'pass' if passed else 'correctness_failure',**fields})

    def boundary(self,item,dependency):
        directory=self.work/'boundaries'/item['id'];directory.mkdir(parents=True);source=directory/'Boundary.kt';source.write_text(item['source'])
        oracle=self.run('Official boundary '+item['id'],[self.kotlin,source,*self.options,'-cp',cp(dependency,self.stdlib),'-d',directory/'official'],accepted=(0,1))
        actual=self.run('Candidate boundary '+item['id'],[self.compiler,source,'--diagnostic-format','json','-cp',cp(dependency,self.stdlib),'-d',directory/'candidate'],accepted=(0,1))
        diagnostics=[json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
        category='invalid_kotlin' if item['official']=='reject' else 'unsupported_language'
        valid=(oracle.returncode==0)==(item['official']=='accept') and actual.returncode!=0 and any(d.get('code')==item['code'] and d.get('category')==category for d in diagnostics)
        self.report['cases'].append({'id':'boundary/'+item['id'],'outcome':('invalid_rejected' if item['official']=='reject' else 'unsupported_language') if valid else 'correctness_failure','official_accepted':oracle.returncode==0,'diagnostics':diagnostics,'source_sha256':sha(source)})

    def upstream(self,name,supported):
        fixture=parse_fixture(UPSTREAM_ROOT/name);state,reasons=assess(fixture,'box')
        if state!='ready':
            self.report['cases'].append({'id':'upstream/'+name,'outcome':state,'reasons':reasons});return
        directory=self.work/'upstream'/Path(name).stem;directory.mkdir(parents=True)
        sources={};mapping=[]
        for module in fixture['modules']:
            paths=[]
            for file in module['files']:
                path=directory/'sources'/module['name']/file['name'];path.parent.mkdir(parents=True,exist_ok=True);path.write_text(file['text']);paths.append(path)
                mapping.append({'module':module['name'],'materialized':relative(path),'sha256':sha(path),'original_char_offsets':file['original_char_offsets']})
            sources[module['name']]=paths
        self.report.setdefault('upstream_materializations',[]).append({'original':relative(UPSTREAM_ROOT/name),'sha256':fixture['sha256'],'modules':[{k:v for k,v in m.items() if k!='files'} for m in fixture['modules']],'files':mapping})
        outputs={p:{} for p in ['official','candidate']};java_outputs={};failure=None
        for provider in outputs:
            for module in fixture['modules']:
                module_name=module['name'];dependency=[outputs[provider][dep] for dep in dict.fromkeys(module['dependencies']+module['friends'])]
                java_files=[p for p in sources[module_name] if p.suffix=='.java'];kotlin_files=[p for p in sources[module_name] if p.suffix=='.kt']
                if java_files:
                    if provider=='official':
                        java_outputs[module_name]=directory/'java'/module_name
                        self.run('Original Java dependency '+name,[self.javac,'-cp',cp(*dependency,self.stdlib),'-d',java_outputs[module_name],*java_files])
                    dependency.append(java_outputs[module_name])
                output=directory/provider/module_name
                if provider=='official': command=[self.kotlin,*kotlin_files,*options_for(module),'-module-name',module_name,'-cp',cp(*dependency,self.stdlib),'-d',output]
                else: command=[self.compiler,*kotlin_files,'--module-name',module_name,'--diagnostic-format','json','-cp',cp(*dependency,self.stdlib),'-d',output]
                actual=self.run('Unchanged original '+name+'/'+provider+'/'+module_name,command,accepted=(0,1))
                if provider=='official' and actual.returncode: raise AssertionError('Official original rejected '+name)
                if actual.returncode:
                    diagnostics=[json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
                    failure=diagnostics
                    self.report['cases'].append({'id':'upstream/'+name+'/boundary','outcome':'unsupported_language' if not supported and diagnostics and diagnostics[0].get('category')=='unsupported_language' else 'correctness_failure','original':fixture['path'],'sha256':fixture['sha256'],'diagnostics':diagnostics})
                    break
                outputs[provider][module_name]=output
        if failure is not None:return
        if not supported:
            self.case('upstream/'+name+'/unexpected_acceptance',False);return
        box_file=next((m,f) for m in fixture['modules'] for f in m['files'] if re.search(r'\bfun\s+box\s*\(',f['text']))
        package=re.search(r'^\s*package\s+([\w.]+)',box_file[1]['text'],re.MULTILINE)
        owner=((package.group(1)+'.') if package else '')+Path(box_file[1]['name']).stem[:1].upper()+Path(box_file[1]['name']).stem[1:]+'Kt'
        driver=directory/'UpstreamCalls.java';driver.write_text('public class UpstreamCalls { public static void main(String[] args) { if(!'+owner+'.box().equals("OK"))throw new AssertionError();System.out.println("OK"); }}')
        callers=directory/'callers';self.run('Compile fixed original assertions '+name,[self.javac,'-cp',cp(*outputs['official'].values(),*java_outputs.values(),self.stdlib),'-d',callers,driver])
        for provider in outputs:
            result=self.run('Execute unchanged original '+name+'/'+provider,[self.java,'-Xverify:all','-cp',cp(callers,*outputs[provider].values(),*java_outputs.values(),self.stdlib),'UpstreamCalls'])
            self.case('upstream/'+name+'/'+provider,result.stdout=='OK\n',original=fixture['path'],sha256=fixture['sha256'],reused_assertion_bytecodes=True)

    def execute(self):
        dependency=self.work/'dependency'
        self.run('Compile independent Java effects and contracts',[self.javac,'-d',dependency,*[FIXTURES/name for name in ['Events.java','JavaText.java','JavaGeneric.java','JavaDefaults.java','JavaArrays.java','Tag.java']]])
        outputs={p:self.work/p for p in ['official','candidate']}
        self.run('Official delegation declarations',[self.kotlin,FIXTURES/'Use.kt',*self.options,'-cp',dependency,'-d',outputs['official']])
        self.run('Candidate delegation declarations',[self.compiler,FIXTURES/'Use.kt','--module-name','class_delegation','-cp',cp(dependency,self.stdlib),'-d',outputs['candidate']])
        callers=self.work/'callers'
        self.run('Compile fixed Java assertions once against official',[self.javac,'-cp',cp(outputs['official'],dependency,self.stdlib),'-d',callers,FIXTURES/'Calls.java'])
        self.run('Compile same Java assertions against candidate signatures',[self.javac,'-cp',cp(outputs['candidate'],dependency,self.stdlib),'-d',self.work/'candidate-signature-consumer',FIXTURES/'Calls.java'])
        for provider,output in outputs.items():
            result=self.run('Execute identical Java bytecodes '+provider,[self.java,'-Xverify:all','-cp',cp(callers,output,dependency,self.stdlib),'Calls'])
            self.case('java/'+provider,result.stdout=='OK\n',reused_assertion_bytecodes=True,scope='Saved receivers, multiple delegates, init/property order, overrides, interface getters/setters, wide values, boxed null/identity, scalar/nullable/numeric bridges and pre-effect failures, UTF16, platform Java contracts, caught/escaping initializer exceptions and source inheritance')
            for consumer_kind in ['official','candidate']:
                consumer=self.work/(provider+'-'+consumer_kind+'-consumer')
                command=[self.kotlin,FIXTURES/'Consumer.kt',*self.options,'-cp',cp(output,dependency),'-d',consumer] if consumer_kind=='official' else [self.compiler,FIXTURES/'Consumer.kt','-cp',cp(output,dependency,self.stdlib),'-d',consumer]
                self.run(consumer_kind+' Kotlin metadata consumer '+provider,command)
                launcher=self.work/(provider+'-'+consumer_kind+'-launcher')
                self.run('Compile Kotlin consumer assertions '+provider+'/'+consumer_kind,[self.javac,'-cp',cp(consumer,output,dependency,callers,self.stdlib),'-d',launcher,FIXTURES/'ConsumerCalls.java'])
                result=self.run('Execute Kotlin metadata consumer '+provider+'/'+consumer_kind,[self.java,'-Xverify:all','-cp',cp(launcher,consumer,output,dependency,callers,self.stdlib),'ConsumerCalls'])
                self.case('metadata/'+provider+'/'+consumer_kind,result.stdout=='OK\n')
        for path in sorted(outputs['official'].rglob('*.class')):
            owner=str(path.relative_to(outputs['official']).with_suffix('')).replace(os.sep,'.')
            expected=self.run('Official public ABI '+owner,[self.javap,'-protected','-s','-v','-classpath',outputs['official'],owner]).stdout
            actual=self.run('Candidate public ABI '+owner,[self.javap,'-protected','-s','-v','-classpath',outputs['candidate'],owner]).stdout
            self.case('abi/'+owner,signatures(expected)==signatures(actual),expected=signatures(expected),actual=signatures(actual))
        metadata_tool=self.work/'metadata-tool';self.run('Compile pinned metadata declaration inspector',[self.javac,'-cp',cp(self.compiler_jar,self.stdlib),'-d',metadata_tool,FIXTURES/'MetadataDump.java'])
        owners=['delegated.'+n for n in ['Forward','IntForward','PlainIntForward','LongForward','FloatForward','DoubleForward','ByteForward','BooleanForward','CharForward','NullableForward','RefForward','Saved','Ordered','Inherited','Shared','JavaForward','DiamondForward']]
        inspected={}
        for provider,output in outputs.items():
            result=self.run('Inspect delegated metadata kind/modality '+provider,[self.java,'-cp',cp(metadata_tool,output,dependency,self.compiler_jar,self.stdlib),'MetadataDump',*owners])
            inspected[provider]=sorted(result.stdout.splitlines())
        self.case('metadata/delegated_member_flags',inspected['official']==inspected['candidate'],expected=inspected['official'],actual=inspected['candidate'])
        for item in json.loads((FIXTURES/'Rejections.json').read_text()):self.boundary(item,dependency)
        annotated=self.work/'Annotated.kt';annotated.write_text('package decorated\nimport delegated.Tag\ninterface AnnotatedMember { @Tag fun f():Int }\ninterface AnnotatedParameter { fun f(@Tag value:Int):Int }')
        annotation_output=self.work/'annotated-dependency';self.run('Official decorated interface dependencies',[self.kotlin,annotated,*self.options,'-cp',dependency,'-d',annotation_output])
        self.report.setdefault('auxiliary_sources_sha256',{})[relative(annotated)]=sha(annotated)
        for classifier in ['AnnotatedMember','AnnotatedParameter']:
            self.boundary({'id':'imported_'+classifier,'source':'import decorated.'+classifier+'\nclass F(d:'+classifier+'):'+classifier+' by d','official':'accept','code':'INTERFACE_DELEGATION_ANNOTATION_UNSUPPORTED'},cp(dependency,annotation_output))
        for name,supported in UPSTREAMS:self.upstream(name,supported)
        inventory_tool=self.work/'inventory-tool';self.run('Compile pinned PSI inventory',[self.javac,'-cp',cp(self.compiler_jar,self.stdlib),'-d',inventory_tool,FIXTURES/'Inventory.java'])
        inventory=self.run('Inventory actual unchanged AndroidBible classifier delegation',[self.java,'-cp',cp(inventory_tool,self.compiler_jar,self.stdlib),'Inventory',APP])
        count=len(list(APP.rglob('*.kt')))
        self.case('androidbible/header_delegation_inventory',inventory.stdout==f'FILES={count} HEADER_DELEGATIONS=0\n',acceptance_scope='application_inventory_only',files=count,header_delegations=0,new_app_coverage_claim=False)
        self.report['emitted_classes_sha256']={p:{relative(f):sha(f) for f in output.rglob('*.class')} for p,output in outputs.items()}

    def finish(self):
        changed=[p for p,h in self.frozen.items() if sha(ROOT/p)!=h]
        self.case('frozen_inputs',not changed and sha(self.compiler)==self.report['compiler']['binary_sha256'],changed=changed)
        self.report['counts']=dict(Counter(c['outcome'] for c in self.report['cases']))
        selected=[c for c in self.report['cases'] if c.get('acceptance_scope')!='application_inventory_only'];counts=dict(Counter(c['outcome'] for c in selected));failed=sum(v for k,v in counts.items() if k not in ['pass','invalid_rejected','unsupported_language'])
        self.report['feature_acceptance']={'family':'class_header_interface_delegation','status':'failed' if failed else 'passed','derived_counts':counts,'excluded_assessment_cases':[c['id'] for c in self.report['cases'] if c.get('acceptance_scope')=='application_inventory_only']}
        self.report['status']='failed' if failed else 'completed';path=ROOT/'reports/correctness'/(self.ident+'.json');path.write_text(json.dumps(self.report,indent=2)+'\n')
        subprocess.run(['python3','tools/reports.py'],cwd=ROOT,check=True,capture_output=True,text=True)
        print(relative(path));print(json.dumps(self.report['counts'],sort_keys=True));return 1 if failed else 0


def main():
    p=argparse.ArgumentParser();p.add_argument('--compiler',type=Path,required=True);p.add_argument('--source-root',type=Path,default=ROOT);args=p.parse_args();args.source_root=args.source_root.resolve();check=Check(args)
    try:check.execute()
    except Exception as error:check.report['cases'].append({'id':'harness','outcome':'correctness_failure','error':str(error)})
    return check.finish()

if __name__=='__main__':raise SystemExit(main())
