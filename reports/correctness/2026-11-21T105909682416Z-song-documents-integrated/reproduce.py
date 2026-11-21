#!/usr/bin/env python3
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
import tarfile
import zipfile

REVISION = '000b87640b34b9d166fda80bf91f9ab87b070ee4'
BASE = 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/songs/newdoc/'
NAMES = [BASE + name for name in ['SongDocument.kt', 'SongDocumentSerializers.kt', 'SongDocumentSearch.kt', 'SongDocumentText.kt']]
DRIVER = 'tests/stable_properties/SongCalls.java'

def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def hashes(path): return {str(p.relative_to(path)): sha(p) for p in sorted(path.rglob('*')) if p.is_file()}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--reference-root', type=Path, default=Path('/workspace/korrust'))
    parser.add_argument('--report-root', type=Path, default=Path('/workspace/work/song33'))
    parser.add_argument('--evidence', type=Path)
    parser.add_argument('--work', type=Path, default=Path('/workspace/work/song33-proof'))
    parser.add_argument('--runtime-dependencies', type=Path, default=Path('/workspace/work/song33-probe/deps'))
    args = parser.parse_args()
    root, report_root, work = args.reference_root.resolve(), args.report_root.resolve(), args.work.resolve()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-song-documents-integrated'
    evidence = args.evidence.resolve() if args.evidence else report_root / 'reports/correctness' / stamp
    evidence.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=False)
    snapshot = work / 'inputs'
    snapshot.mkdir()
    members = ['src', 'Cargo.toml', 'Cargo.lock', 'references/compiler.json', 'references/sources.json', 'benchmarks/sources.json', 'benchmarks/source-files.sha256.json', *NAMES, DRIVER]
    archive = subprocess.check_output(['git', '-C', str(root), 'archive', REVISION, *members])
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        source.extractall(snapshot, filter='data')
    consumer = snapshot / 'probes/SongConsumer.kt'
    consumer.parent.mkdir()
    consumer.write_text('''package songconsumer
import yuku.alkitab.songs.newdoc.*
fun main() {
    val document = SongDocument("0", Meta(null, null), emptyList())
    check(SongDocumentSearch.searchableTexts(document) == listOf("0"))
    check(SongDocumentSearch.lyricLines(document).isEmpty())
    check(SongDocumentText.render(document, null, null, { index -> "Version $index" }, "R") == "0. \\n\\n\\n\\n")
    println("OK")
}
''')
    with zipfile.ZipFile(evidence / 'source-inputs.zip', 'w', zipfile.ZIP_DEFLATED) as bundle:
        for path in snapshot.rglob('*'):
            if path.is_file(): bundle.write(path, path.relative_to(snapshot))
    fixed_inputs = hashes(snapshot)
    libraries = work / 'jars'
    libraries.mkdir()
    jars = []
    for name in ['kotlinx-serialization-core-jvm-1.5.1.jar', 'kotlinx-serialization-json-jvm-1.5.1.jar']:
        source = args.runtime_dependencies / name
        target = libraries / name
        shutil.copy2(source, target)
        jars.append(target)
    for name in ['kotlin-stdlib.jar', 'kotlin-compiler.jar', 'kotlinx-serialization-compiler-plugin.jar']:
        source = root / 'references/kotlinc/lib' / name
        shutil.copy2(source, libraries / name)
    library_hashes = hashes(libraries)
    official = root / 'references/kotlinc/bin/kotlinc'
    stdlib = libraries / 'kotlin-stdlib.jar'
    plugin = libraries / 'kotlinx-serialization-compiler-plugin.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    candidate = work / 'compiler-release/korrust'
    def relative(path):
        path = Path(path).absolute()
        return str(path.relative_to(report_root)) if path.is_relative_to(report_root) else str(path)
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
        compiler=dict(source_revision=REVISION, frozen_source_hashes=fixed_inputs, path=str(candidate)),
        reference_compiler=json.loads((snapshot / 'references/compiler.json').read_text()),
        source_input_bundle=dict(path=relative(evidence / 'source-inputs.zip'), sha256=sha(evidence / 'source-inputs.zip')),
        unchanged_app_sources={name: fixed_inputs[name] for name in NAMES},
        frozen_library_hashes=library_hashes,
        options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8', friend_dependency='Actual SongDocument and SongDocumentSerializers outputs from pinned official compiler and serialization plugin'),
        scope='Early integrated proof for both unchanged portable Song app objects against actual model/serialization dependencies; genuine JAR inputs, matching classpaths and explicit friend paths, fixed official Java caller, JVM verification, signatures and official Kotlin metadata consumers',
        limitations=['No full Android application build', 'No compilation or runtime performance claim'],
        reproduction_command=['python3', relative(Path(__file__)), '--reference-root', str(root), '--report-root', str(report_root), '--work', str(work)], cases=[], commands=[])
    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')
    def run(label, command, *, cwd=root, env=None):
        command = [str(x) for x in command]
        result = subprocess.run(command, cwd=cwd, env=env, capture_output=True, text=True, timeout=600)
        path = evidence / f'{len(report["commands"]):03d}.log'
        path.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, cwd=str(cwd), exit_code=result.returncode, log=relative(path)))
        save()
        assert result.returncode == 0, label + ': ' + result.stdout + result.stderr
        return result
    def case(name, operation):
        try:
            detail = operation()
            report['cases'].append(dict(id=name, outcome='pass', detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        save()
    save()
    def build():
        build_env = os.environ.copy()
        build_env['CARGO_TARGET_DIR'] = '/workspace/korrust/target/song33-proof-build'
        run('Build exact frozen integrated source snapshot in release mode', ['cargo', 'build', '--release', '--offline', '--locked'], cwd=snapshot, env=build_env)
        candidate.parent.mkdir()
        shutil.copy2(Path(build_env['CARGO_TARGET_DIR']) / 'release/korrust', candidate)
        report['compiler']['sha256'] = sha(candidate)
        return dict(source_revision=REVISION, sha256=sha(candidate))
    case('frozen_integrated_release_build', build)
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    model = work / 'actual-model'
    cp = os.pathsep.join(map(str, [model, stdlib, *jars]))
    def dependencies():
        run('Pinned official actual Song model and serializer dependency compile', [official, snapshot / NAMES[0], snapshot / NAMES[1], '-Xplugin=' + str(plugin), '-cp', os.pathsep.join(map(str, jars)), '-d', model, *options])
        report['model_hashes'] = hashes(model)
        return dict(sources={name:fixed_inputs[name] for name in NAMES[:2]}, output_hashes=report['model_hashes'])
    case('actual_dependency_compile', dependencies)
    outputs = dict(official=work / 'official', candidate=work / 'candidate')
    def apps():
        sources = [snapshot / name for name in NAMES[2:]]
        run('Official two unchanged app source module', [official, *sources, '-cp', cp, '-Xfriend-paths=' + str(model), '-module-name', 'songproof', '-d', outputs['official'], *options])
        run('Candidate two unchanged app source module with genuine JARs', [candidate, *sources, '-cp', cp, '--friend-paths', model, '--module-name', 'songproof', '-d', outputs['candidate']])
        model_classes = set(model.rglob('*.class'))
        names = {str(p.relative_to(model)) for p in model_classes}
        assert not names.intersection(str(p.relative_to(outputs['candidate'])) for p in outputs['candidate'].rglob('*.class'))
        report['application_output_hashes'] = {name:hashes(path) for name,path in outputs.items()}
        return dict(source_hashes={name:fixed_inputs[name] for name in NAMES[2:]}, classpath=[str(model), str(stdlib), *map(str,jars)], friend_paths=[str(model)], dependency_output_overlap=False)
    case('both_unchanged_app_sources_compile', apps)
    def runtime():
        callers = work / 'fixed-official-java'
        run('Compile fixed Java SongCalls once against official output', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs['official'], model, stdlib, *jars])), '-d', callers, snapshot / DRIVER])
        caller_hashes = hashes(callers)
        for provider, output in outputs.items():
            result = run(provider + ' full independent Song assertions with JVM verification', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, model, stdlib, *jars])), 'SongCalls'])
            assert result.stdout == 'OK\n', result.stdout
        assert hashes(callers) == caller_hashes
        return dict(caller_source_sha256=fixed_inputs[DRIVER], caller_class_hashes=caller_hashes, assertions='Search field inclusion and exclusion, styled/wrapped lyric extraction, exact render text, Turkish default Locale independence, title/original title/authors/tune/musical/scripture placement, NORMAL/REFRAIN/TEXT formatting, caption fallback callback argument and count, multiple lyric blocks, empty/null fields')
    case('fixed_java_caller_both_outputs', runtime)
    def signatures():
        names = ['yuku.alkitab.songs.newdoc.SongDocumentSearch', 'yuku.alkitab.songs.newdoc.SongDocumentText']
        listings = [run(provider + ' Song JVM signatures', [java / 'javap', '-s', '-protected', '-classpath', output, *names]).stdout for provider, output in outputs.items()]
        assert listings[0] == listings[1], listings
        return listings[0]
    case('matching_jvm_signatures', signatures)
    for provider, output in outputs.items():
        def consumer_case(provider=provider, output=output):
            consumer_output = work / ('consumer-' + provider)
            run('Official Kotlin metadata consumer against ' + provider, [official, consumer, '-cp', os.pathsep.join(map(str, [output, model, stdlib, *jars])), '-d', consumer_output, *options])
            for runtime_provider, runtime_output in outputs.items():
                result = run('Kotlin consumer ' + provider + ' on ' + runtime_provider, [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_output, runtime_output, model, stdlib, *jars])), 'songconsumer.SongConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Pinned Kotlin consumers compile against each provider and cross-run against both outputs'
        case('kotlin_consumer_' + provider, consumer_case)
    assert hashes(snapshot) == fixed_inputs, 'Frozen source snapshot changed'
    assert hashes(libraries) == library_hashes, 'Frozen library artifacts changed'
    assert hashes(model) == report.get('model_hashes'), 'Dependency outputs changed'
    report['status'] = 'failed' if any(case['outcome']=='correctness_failure' for case in report['cases']) else 'complete'
    save()
    run('Generate readable HTML reports', ['python3', report_root / 'tools/reports.py'], cwd=report_root)
    assert (report_root / 'target/reports-html/index.html').is_file()
    print(report_path)
    return int(report['status']=='failed')
if __name__ == '__main__': raise SystemExit(main())
