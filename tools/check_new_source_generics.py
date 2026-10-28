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

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/source_generics_new'
APP = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/AlphanumComparator.kt'
APP_TEST = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/test/java/yuku/alkitab/base/util/AlphanumComparatorTest.kt'
UPSTREAMS = [ROOT / ('references/kotlin/compiler/testData/codegen/box/bridges/' + name) for name in ['simpleReturnType.kt','kt2920.kt']]


def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
def cp(*paths): return os.pathsep.join(map(str,paths))
def signatures(output):
    lines=[]
    for line in output.splitlines():
        line=line.strip()
        if line.startswith(('public ','protected ','private ')) and (';' in line or line.endswith('{')): lines.append(line)
        elif line.startswith('descriptor:'): lines.append(line)
        elif line.startswith('Signature:'): lines.append('Signature: '+line.split('//',1)[-1].strip())
    return sorted(lines)


class Check:
    def __init__(self,args):
        self.args=args; self.compiler=args.compiler.resolve()
        self.now=datetime.now(timezone.utc); self.ident=self.now.strftime('%Y-%m-%dT%H%M%S%fZ')+'-new-source-generics'
        self.work=ROOT/'target/checks'/self.ident; self.logs=ROOT/'reports/correctness'/self.ident
        self.work.mkdir(parents=True);self.logs.mkdir(parents=True)
        self.pin=json.loads((ROOT/'references/compiler.json').read_text())
        self.kotlin=ROOT/self.pin['path']/'bin/kotlinc';self.stdlib=ROOT/self.pin['path']/'lib/kotlin-stdlib.jar'
        home=Path(os.environ['JAVA_HOME']);self.java,self.javac,self.javap=[home/'bin'/n for n in ['java','javac','javap']]
        self.options=['-language-version','2.3','-api-version','2.3','-jvm-target','1.8','-module-name','new_source_generics']
        frozen=[*args.source_root.glob('src/**/*.rs'),args.source_root/'Cargo.toml',args.source_root/'Cargo.lock',*FIXTURES.glob('*'),Path(__file__),APP,APP_TEST,*UPSTREAMS,ROOT/'base-manifest.json']
        self.frozen={relative(p):sha(p) for p in sorted(frozen) if p.is_file()}
        self.report=dict(schema_version=1,kind='correctness',created_at=self.now.isoformat(),status='running',compiler={'binary':relative(self.compiler),'binary_sha256':sha(self.compiler),'frozen_source_root':relative(args.source_root)},reference_compiler={'version':self.pin['version'],'archive_sha256':self.pin['archive_sha256'],'options':self.options},inputs={'files_sha256':self.frozen,'stdlib_sha256':sha(self.stdlib),'junit_sha256':sha(args.junit_jar),'hamcrest_sha256':sha(args.hamcrest_jar)},commands=[],cases=[],evidence=[],reproduction_commands=[['python3','tools/check_new_source_generics.py','--compiler',relative(self.compiler),'--source-root',relative(args.source_root),'--junit-jar',str(args.junit_jar),'--hamcrest-jar',str(args.hamcrest_jar)]],limitations=['Invariant source generic classes/interfaces with explicit construction, class bounds, typed fields/members, transitive parameterized inheritance and required erased bridges. Existing generic functions/imported generic classes are prerequisites and excluded as new support.','Declaration-site variance, dependent/F-bounds, interface bounds, classifier generic member functions, generic interface defaults, nested generic classifiers/companions, generic classifier lambda capture and type-parameter shadowing remain explicit unsupported boundaries. Constructor type inference requires explicit arguments.','One actual unchanged AndroidBible source and two unchanged pinned upstream originals; fixed Java bytecodes, class verification, generic JVM signatures and official Kotlin metadata consumers. No full-app, device, performance or exhaustive-language claim.'])
    def run(self, label, args, accepted=(0,)):
        index = len(self.report['commands'])
        result = subprocess.run(list(map(str, args)), cwd=ROOT, capture_output=True, text=True, timeout=180)
        entry = {'label': label, 'argv': list(map(str, args)), 'cwd': '.', 'exit_code': result.returncode}
        for stream in ['stdout', 'stderr']:
            path = self.logs / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(self, ident, passed, **fields):
        self.report['cases'].append({'id': ident, 'outcome': 'pass' if passed else 'correctness_failure', **fields})

    def execute(self):
        dependency=self.work/'dependency'
        self.run('Compile independent effects dependency',[self.javac,'-d',dependency,FIXTURES/'Probe.java'])
        official,candidate=self.work/'official',self.work/'candidate'
        self.run('Official source generic declarations',[self.kotlin,FIXTURES/'Use.kt',*self.options,'-cp',dependency,'-d',official])
        self.run('Candidate source generic declarations',[self.compiler,FIXTURES/'Use.kt','--module-name','new_source_generics','-cp',cp(dependency,self.stdlib),'-d',candidate])
        callers=self.work/'callers'
        self.run('Compile fixed Java assertions once against official',[self.javac,'-cp',cp(official,dependency,self.stdlib),'-d',callers,FIXTURES/'Calls.java'])
        self.run('Compile identical Java source against candidate generic signatures',[self.javac,'-cp',cp(candidate,dependency,self.stdlib),'-d',self.work/'candidate-java-signature-consumer',FIXTURES/'Calls.java'])
        for provider,output in [('official',official),('candidate',candidate)]:
            observed=self.run('Execute identical fixed Java assertions '+provider,[self.java,'-Xverify:all','-cp',cp(callers,output,dependency,self.stdlib),'Calls'])
            self.case('java/'+provider,observed.stdout=='OK\n',scope='Generic source fields/construction/bounds; primitive/ref/null/UTF16; erased interfaces/superclasses; wide slots; bridge casts; evaluation effects; tail when prerequisite')
            consumer=self.work/(provider+'-consumer')
            self.run('Official Kotlin metadata consumer '+provider,[self.kotlin,FIXTURES/'Consumer.kt',*self.options,'-cp',cp(output,dependency),'-d',consumer])
            launcher=self.work/(provider+'-consumer-launcher')
            self.run('Metadata consumer launcher '+provider,[self.javac,'-cp',cp(consumer,output,dependency,self.stdlib),'-d',launcher,FIXTURES/'ConsumerCalls.java'])
            observed=self.run('Execute official Kotlin consumer '+provider,[self.java,'-Xverify:all','-cp',cp(launcher,consumer,output,dependency,self.stdlib),'ConsumerCalls'])
            self.case('metadata/'+provider,observed.stdout=='OK\n')
        for path in sorted(official.rglob('*.class')):
            owner=str(path.relative_to(official).with_suffix('')).replace(os.sep,'.')
            expected=self.run('Official generic signatures '+owner,[self.javap,'-p','-s','-v','-classpath',official,owner])
            actual=self.run('Candidate generic signatures '+owner,[self.javap,'-p','-s','-v','-classpath',candidate,owner])
            self.case('abi/'+owner,signatures(expected.stdout)==signatures(actual.stdout),expected=signatures(expected.stdout),actual=signatures(actual.stdout))
        for item in json.loads((FIXTURES/'Rejections.json').read_text()):
            source=self.work/'boundaries'/item['id']/'Boundary.kt';source.parent.mkdir(parents=True);source.write_text(item['source'])
            oracle=self.run('Official boundary '+item['id'],[self.kotlin,source,*self.options,'-d',source.parent/'official'],accepted=(0,1))
            actual=self.run('Candidate boundary '+item['id'],[self.compiler,source,'--diagnostic-format','json','-cp',self.stdlib,'-d',source.parent/'candidate'],accepted=(0,1))
            diagnostics=[json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            valid=(oracle.returncode==0)==(item['official']=='accept') and actual.returncode!=0 and item['diagnostic'].lower() in actual.stderr.lower()
            if item.get('code'): valid=valid and any(d.get('code')==item['code'] and d.get('category')==item['category'] for d in diagnostics)
            outcome='correctness_failure' if not valid else 'invalid_rejected' if item['official']=='reject' else 'unsupported_language' if diagnostics and diagnostics[0].get('category')=='unsupported_language' else 'unclassified_compilation_failure'
            self.report['cases'].append({'id':'boundary/'+item['id'],'outcome':outcome,'diagnostics':diagnostics,'official_accepted':oracle.returncode==0,'source_sha256':sha(source)})
        for original in UPSTREAMS:
            outputs={p:self.work/('upstream-'+original.stem+'-'+p) for p in ['official','candidate']}
            self.run('Unchanged upstream official '+original.stem,[self.kotlin,original,*self.options,'-d',outputs['official']])
            self.run('Unchanged upstream candidate '+original.stem,[self.compiler,original,'-cp',self.stdlib,'-d',outputs['candidate']])
            owner=next(p.stem for p in outputs['official'].glob('*Kt.class'))
            driver=self.work/('upstream-'+original.stem)/'UpstreamCalls.java';driver.parent.mkdir();driver.write_text('public final class UpstreamCalls { public static void main(String[] args) { if (!'+owner+'.box().equals("OK")) throw new AssertionError(); System.out.println("OK"); } }')
            caller=driver.parent/'callers';self.run('Compile fixed upstream caller '+original.stem,[self.javac,'-cp',cp(outputs['official'],self.stdlib),'-d',caller,driver])
            for provider,output in outputs.items():
                observed=self.run('Execute unchanged upstream '+original.stem+' '+provider,[self.java,'-Xverify:all','-cp',cp(caller,output,self.stdlib),'UpstreamCalls'])
                self.case('upstream/'+original.stem+'/'+provider,observed.stdout=='OK\n',original=relative(original),source_sha256=sha(original))
        app_official,app_candidate=self.work/'app-official',self.work/'app-candidate'
        self.run('Compile unchanged AndroidBible oracle',[self.kotlin,APP,*self.options,'-d',app_official])
        self.run('Compile unchanged AndroidBible candidate',[self.compiler,APP,'-cp',self.stdlib,'-d',app_candidate])
        app_callers=self.work/'app-callers'
        self.run('Compile fixed independent app Java assertions once',[self.javac,'-cp',cp(app_official,self.stdlib),'-d',app_callers,FIXTURES/'AppCalls.java'])
        test_output=self.work/'app-original-tests'
        test_cp=cp(app_official,self.args.junit_jar,self.args.hamcrest_jar,self.stdlib)
        self.run('Compile unchanged original AndroidBible JUnit test once',[self.kotlin,APP_TEST,*self.options,'-cp',test_cp,'-d',test_output])
        for provider,output in [('official',app_official),('candidate',app_candidate)]:
            observed=self.run('Execute fixed app Java assertions '+provider,[self.java,'-Xverify:all','-cp',cp(app_callers,output,self.stdlib),'AppCalls'])
            self.case('app/'+provider,observed.stdout=='OK\n',original=relative(APP),source_sha256=sha(APP))
            observed=self.run('Execute unchanged original app tests '+provider,[self.java,'-Xverify:all','-cp',cp(test_output,output,self.args.junit_jar,self.args.hamcrest_jar,self.stdlib),'org.junit.runner.JUnitCore','yuku.alkitab.base.util.AlphanumComparatorTest'])
            self.case('app_original_tests/'+provider,'OK (12 tests)' in observed.stdout,original=relative(APP_TEST),source_sha256=sha(APP_TEST),reused_assertion_bytecodes=True)
            self.run('Official Kotlin app metadata consumer '+provider,[self.kotlin,FIXTURES/'AppConsumer.kt',*self.options,'-cp',output,'-d',self.work/('app-consumer-'+provider)])
            self.case('app_metadata/'+provider,True)
        self.report['emitted_classes_sha256']={provider:{relative(p):sha(p) for p in output.rglob('*.class')} for provider,output in [('official',official),('candidate',candidate)]}
    def finish(self):
        changed = [path for path, checksum in self.frozen.items() if sha(ROOT / path) != checksum]
        self.case('frozen_inputs', not changed and sha(self.compiler) == self.report['compiler']['binary_sha256'], changed=changed)
        self.report['counts'] = dict(Counter(case['outcome'] for case in self.report['cases']))
        selected = [case for case in self.report['cases'] if case.get('acceptance_scope') != 'external_app_assessment']
        counts = dict(Counter(case['outcome'] for case in selected))
        failed = counts.get('correctness_failure', 0)
        self.report['feature_acceptance'] = {'family': 'source_generic_classifiers_and_parameterized_supertypes', 'status': 'failed' if failed else 'passed', 'derived_counts': counts, 'excluded_assessment_cases': [case['id'] for case in self.report['cases'] if case.get('acceptance_scope') == 'external_app_assessment']}
        self.report['status'] = 'failed' if failed else 'completed'
        path = ROOT / 'reports/correctness' / (self.ident + '.json')
        path.write_text(json.dumps(self.report, indent=2) + '\n')
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True, text=True)
        print(relative(path))
        print(json.dumps(self.report['counts'], sort_keys=True))
        return 1 if failed else 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--junit-jar', type=Path, required=True)
    parser.add_argument('--hamcrest-jar', type=Path, required=True)
    args = parser.parse_args()
    args.source_root = args.source_root.resolve()
    check = Check(args)
    try:
        check.execute()
    except Exception as error:
        check.report['cases'].append({'id': 'harness', 'outcome': 'correctness_failure', 'error': str(error)})
    return check.finish()


if __name__ == '__main__':
    raise SystemExit(main())
