from pathlib import Path
from datetime import datetime, timezone
from collections import Counter
import json,hashlib,sys
r=Path('/Users/yuku/f/git/korrust');d=r/'target/resume-app-inputs';sys.path.insert(0,str(r/'tools'));import assess_androidbible as a
model_path=r/'target/final-androidbible-debug-model/model.json';m=json.loads(model_path.read_text());raw_path=r/m['raw_capture_model'];raw=json.loads(raw_path.read_text());provenance_path=r/m['normalization_provenance'];provenance=json.loads(provenance_path.read_text());cache={}
def inspect(name):
 if name in cache:return cache[name]
 p=Path(name)
 if not p.exists():result={'path':name,'availability':'missing'}
 elif p.is_file():result={'path':name,'availability':'present','kind':'file','sha256':a.sha(p)}
 else:
  files=a.tree_hashes(p);result={'path':name,'availability':'present','kind':'directory','file_count':len(files),'class_count':sum(path.endswith('.class') for path in files),'tree_manifest_sha256':hashlib.sha256(json.dumps(files,sort_keys=True).encode()).hexdigest()}
 cache[name]=result;return result
cases=[]
for module,entry in m['modules'].items():
 original=raw['modules'].get(module,{})
 for task_name,task in entry.get('compilation_tasks',{}).items():
  before=original.get('compilation_tasks',{}).get(task_name,{})
  groups={key:[inspect(path) for path in task.get(key,[])] for key in ['compile_classpath','compiler_plugin_classpath','friend_paths','generated_source_roots','source_roots']}
  groups['compiler_runtime']=[inspect(path) for path in entry.get('resolved_kotlin_compiler_classpath',[])]
  bindings=[inspect(path) for path in task.get('derived_assessment_scope',{}).get('source_binding_outputs',[])]
  captures=provenance.get('modules',{}).get(module,{}).get('original_capture',{})
  missing_actual=[item for key,items in groups.items() if key not in ['source_roots','generated_source_roots'] for item in items if item['availability']=='missing']
  missing_roots=[item for item in groups['source_roots'] if item['availability']=='missing']
  before_missing=[inspect(path) for path in before.get('compile_classpath',[]) if not Path(path).exists()]
  runtime_drift=[path for path,digest in entry.get('resolved_kotlin_compiler_files_sha256',{}).items() if not Path(path).is_file() or a.sha(path)!=digest]
  cases.append({'id':module+'/'+task_name,'outcome':'inputs_available' if not missing_actual and not runtime_drift else 'setup_failure','original_source_counts':dict(Counter(Path(path).suffix for path in task.get('sources',[]))),'original_captured_source_count':len(before.get('sources',[])),'derived_source_count':len(task.get('sources',[])),'input_groups':groups,'precompiled_source_bindings':bindings,'raw_capture_missing_classpath':before_missing,'original_capture_setup_failures':captures.get('setup_failures',[]),'optional_empty_omissions':captures.get('omitted_optional_empty_classpath',[]),'derived_setup_failures':a.resolved_setup_errors(entry,task_name,task),'runtime_hash_drift':runtime_drift,'missing_declared_source_roots':missing_roots,'source_root_policy':'An absent conventional source-set directory is not a missing declared source file or a Kotlin language failure. Generated source/provider availability remains distinct from original-source counts.','scope':'No compiler invocation. Existing app Kotlin/Java sibling and generated class outputs are real precompiled dependencies for isolated probes, not fresh source-batch/full-app compilation.'})
report={'schema_version':1,'kind':'correctness','created_at':datetime.now(timezone.utc).isoformat(),'status':'completed','selection':'Read-only availability audit of normalized selected debug production inputs against original Gradle capture and normalization provenance. No app corpus compilation.','inputs':{'model':str(model_path.relative_to(r)),'model_sha256':a.sha(model_path),'raw_capture_model':str(raw_path.relative_to(r)),'raw_capture_sha256':a.sha(raw_path),'normalization_provenance':str(provenance_path.relative_to(r)),'normalization_provenance_sha256':a.sha(provenance_path),'raw_capture_status':raw.get('status'),'derived_model_status':m.get('status')},'cases':cases,'counts':dict(Counter(case['outcome'] for case in cases)),'evidence':[str(model_path.relative_to(r)),str(raw_path.relative_to(r)),str(provenance_path.relative_to(r))],'limitations':['Availability is not compilation, runtime, Android or source-batch compatibility.','Raw unrelated release/test capture failures remain preserved and are not promoted to debug task language failures.','Derived model omits only documented optional-empty inputs and includes the explicitly listed real precompiled source bindings.']}
p=r/'reports/correctness/2026-11-02-app-input-availability.json';assert not p.exists();p.write_text(json.dumps(report,indent=2)+'\n')
for case in cases:print(case['id'],case['outcome'],'source',case['original_source_counts'],'classpath',len(case['input_groups']['compile_classpath']),'plugins',len(case['input_groups']['compiler_plugin_classpath']),'runtime',len(case['input_groups']['compiler_runtime']),'bindings',[(x.get('class_count'),x['availability']) for x in case['precompiled_source_bindings']],'raw missing',len(case['raw_capture_missing_classpath']))
