#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.request
p = argparse.ArgumentParser()
p.add_argument('--root', type=Path, required=True)
p.add_argument('--work', type=Path, required=True)
a = p.parse_args()
root, work = a.root.resolve(), a.work.resolve()
work.mkdir(parents=True, exist_ok=True)
def sha(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def run(command):
    result = subprocess.run([str(x) for x in command], text=True, capture_output=True, env=os.environ, timeout=360)
    if result.returncode: raise RuntimeError(result.stdout + result.stderr)
    return dict(argv=[str(x) for x in command], exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
deps = []
for artifact in ['kotlinx-serialization-core-jvm', 'kotlinx-serialization-json-jvm']:
    filename = artifact + '-1.5.1.jar'
    url = 'https://repo.maven.apache.org/maven2/org/jetbrains/kotlinx/' + artifact + '/1.5.1/' + filename
    path = work / 'deps' / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists(): urllib.request.urlretrieve(url, path)
    deps.append(dict(path=str(path), url=url, sha256=sha(path)))
base = root / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/songs/newdoc'
sources = [base / 'SongDocument.kt', base / 'SongDocumentSerializers.kt']
plugin = root / 'references/kotlinc/lib/kotlinx-serialization-compiler-plugin.jar'
stdlib = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
command = [root / 'references/kotlinc/bin/kotlinc', *sources, '-Xplugin=' + str(plugin), '-cp', os.pathsep.join(dep['path'] for dep in deps), '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-d', work / 'model']
result = run(command)
receipt = dict(schema_version=1, kind='dependency_replay', status='complete', official_compiler=json.loads((root / 'references/compiler.json').read_text()), sources={str(source):sha(source) for source in sources}, plugin=dict(path=str(plugin),sha256=sha(plugin)), stdlib=dict(path=str(stdlib),sha256=sha(stdlib)), dependencies=deps, output=str(work / 'model'), output_hashes={str(path.relative_to(work / 'model')):sha(path) for path in (work / 'model').rglob('*') if path.is_file()}, commands=[result])
path = work / 'dependency-receipt.json'
path.write_text(json.dumps(receipt,indent=2)+'\n')
print(path)
