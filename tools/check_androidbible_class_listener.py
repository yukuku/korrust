#!/usr/bin/env python3
"""Compare unchanged VersesListeners using genuine AndroidBible build dependencies."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

from check_androidbible_survey import class_entries, class_info, emitted, inventory

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/verses/VersesListeners.kt'
CALLER = ROOT / 'tests/fixtures/androidbible_class_listener/ListenersCheck.java'
RUNTIME_URL = 'https://repo.maven.apache.org/maven2/org/robolectric/android-all/14-robolectric-10818077/android-all-14-robolectric-10818077.jar'
RUNTIME_SHA256 = '6be2218c6a53fe3c57bc22ebdc723edcb7270a8a6f187545708aa5c0ed813977'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def label(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def fnv(data):
    value = 0xcbf29ce484222325
    for byte in data:
        value = ((value ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
    return value


def ownership(output, source):
    index = output / '.korrust/index'
    header, body = index.read_text().split('\n', 1)
    if header != f'korrust-output-v1\t{fnv(body.encode()):016x}':
        raise ValueError('Output ownership checksum mismatch')
    records = {}
    for line in body.splitlines():
        owner, name, digest = line.split('\t')
        owner, name = bytes.fromhex(owner).decode(), bytes.fromhex(name).decode()
        if Path(owner).resolve() != source.resolve() or name in records:
            raise ValueError('Invalid or duplicated source ownership')
        if fnv((output / name).read_bytes()) != int(digest, 16):
            raise ValueError('Owned output content changed')
        records[name] = {'source': label(owner), 'sha256': sha(output / name)}
    files = {p.relative_to(output).as_posix() for p in output.rglob('*') if p.is_file() and '.korrust' not in p.parts}
    if set(records) != files:
        raise ValueError('Missing or extra owned outputs')
    return records


def filter_siblings(model, source, destination):
    copied, excluded = {}, []
    with zipfile.ZipFile(destination, 'w', zipfile.ZIP_DEFLATED) as archive:
        for folder in [Path(model['kotlin_output']), Path(model['java_output'])]:
            for path in sorted(folder.rglob('*')):
                if not path.is_file():
                    continue
                name, data = path.relative_to(folder).as_posix(), path.read_bytes()
                own = name.endswith('.class') and class_info(data)['source_file'] == source.name
                if own:
                    excluded.append({'original': label(path), 'entry': name, 'sha256': sha(path)})
                elif name not in copied:
                    copied[name] = sha(path)
                    archive.writestr(name, data)
                elif copied[name] != sha(path):
                    raise ValueError('Conflicting genuine output entry: ' + name)
    if not excluded:
        raise ValueError('Original target output missing from genuine build')
    return copied, excluded


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--dependencies', type=Path, default=Path('/workspace/work/deps33'))
    parser.add_argument('--android-runtime', type=Path, required=True)
    parser.add_argument('--timeout-seconds', type=int, default=300)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    if args.timeout_seconds <= 0:
        parser.error('--timeout-seconds must be positive')
    if sha(args.android_runtime) != RUNTIME_SHA256:
        parser.error('Android runtime differs from the pinned genuine artifact')
    jdk = Path(os.environ['JAVA_HOME']).resolve()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-androidbible-class-listener'
    evidence = ROOT / 'reports/correctness' / ident
    work = ROOT / 'target/checks' / ident
    evidence.mkdir(parents=True)
    work.mkdir(parents=True)
    output = evidence.with_suffix('.json')
    frozen = {}

    def track(path):
        path = Path(path).resolve()
        frozen.setdefault(label(path), inventory(path))
        return path

    model_path = track(args.dependencies / 'official-model.json')
    model = json.loads(model_path.read_text())
    source_manifest = track(ROOT / 'benchmarks/source-files.sha256.json')
    upstream = json.loads(source_manifest.read_text())['androidbible']
    pinned = upstream[str(SOURCE.relative_to(ROOT / 'benchmarks/sources/androidbible'))]
    build_sources = json.loads(track(args.dependencies / 'source-sha256.json').read_text())
    for name, expected in build_sources.items():
        original = track(args.dependencies / 'project' / name)
        if sha(original) != expected or upstream.get(name) != expected:
            raise ValueError('Genuine build source differs from the unchanged pinned app: ' + name)
    if sha(SOURCE) != pinned:
        raise ValueError('Unchanged AndroidBible source hash mismatch')
    for path in [SOURCE, CALLER, Path(__file__), ROOT / 'tools/check_androidbible_survey.py', ROOT / 'src', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', args.candidate, args.android_runtime, jdk / 'bin/java', jdk / 'bin/javac', jdk / 'bin/javap', jdk / 'jmods', ROOT / 'references/kotlinc/bin/kotlinc', ROOT / 'references/kotlinc/lib']:
        track(path)
    for field in ['kotlin_sources']:
        for path in model[field]:
            if str(path).endswith('.kt') and Path(path).is_file():
                track(path)
    provenance = []
    for name in ['source-sha256.json', 'official-build.log', 'official-build-initial.log', 'capture.init.gradle', 'reproduce-build.sh', 'setup-dependencies.sh']:
        original = track(args.dependencies / name)
        dest = evidence / 'dependency-provenance' / name
        dest.parent.mkdir(exist_ok=True)
        shutil.copy2(original, dest)
        provenance.append({'original': label(original), 'archive': label(dest), 'sha256': sha(dest)})
    for folder in [model['kotlin_output'], model['java_output']]:
        track(folder)
    sibling = work / 'genuine-siblings.jar'
    copied, excluded = filter_siblings(model, SOURCE, sibling)
    track(sibling)
    omitted = [p for p in model['kotlin_classpath'] if not Path(p).exists()]
    classpath = list(dict.fromkeys([track(p) for p in model['kotlin_classpath'] if Path(p).exists()] + [track(ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'), sibling]))
    names = set()
    for path in classpath:
        names.update(class_info(data)['name'] for _, data in class_entries(path))
    binary = work / 'candidate'
    shutil.copy2(args.candidate, binary)
    track(binary)
    archive_path = evidence / 'inputs.zip'
    with zipfile.ZipFile(archive_path, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in [SOURCE, CALLER, Path(__file__), ROOT / 'tools/check_androidbible_survey.py', ROOT / 'references/compiler.json']:
            archive.write(path, path.relative_to(ROOT))
    track(archive_path)
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': now.isoformat(), 'status': 'running',
              'selection': 'One unchanged production source, VersesListeners.kt; every target-owned class is excluded from genuine dependency outputs.',
              'compiler': {'path': label(args.candidate), 'binary_sha256': sha(binary), 'frozen_path': label(binary)},
              'reference_compiler': json.loads((ROOT / 'references/compiler.json').read_text()),
              'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
              'inputs': {'source': label(SOURCE), 'source_sha256': pinned, 'classpath': list(map(label, classpath)), 'files_sha256': frozen,
                         'sibling_entries_sha256': copied, 'excluded_target_classes': excluded, 'omitted_nonexistent_gradle_entries': omitted,
                         'dependency_provenance': provenance, 'input_archive': label(archive_path),
                         'android_runtime': {'path': label(args.android_runtime), 'sha256': RUNTIME_SHA256, 'origin': RUNTIME_URL}},
              'environment': {'JAVA_HOME': str(jdk), 'official_jvm_target': '1.8', 'candidate_jvm_target': 'compiler default; actual classfile versions recorded per class', 'language_version': '2.3', 'api_version': '2.3'},
              'reproduction_commands': [[sys.executable, label(Path(__file__)), *sys.argv[1:]]], 'commands': [], 'cases': [], 'evidence': [label(archive_path)],
              'limitations': ['Single unchanged file with genuine official-built sibling dependencies; no full-module, full-app, Android-device, Kotlin-consumer, or performance claim.',
                              'Java caller is compiled against official output once and reused with both providers under -Xverify:all; plain JVM uses actual android-all framework implementations.']}

    def case(name, outcome='pass', **fields):
        report['cases'].append({'id': name, 'outcome': outcome, **fields})
        save()

    def save():
        report['counts'] = dict(Counter(c['outcome'] for c in report['cases']))
        output.write_text(json.dumps(report, indent=2) + '\n')

    def run(name, command):
        log = evidence / (f'{len(report["commands"]):02d}-' + name + '.log')
        command = list(map(str, command))
        try:
            result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=args.timeout_seconds)
            code, data = result.returncode, result.stdout
        except subprocess.TimeoutExpired as error:
            code, data = None, error.stdout or b''
        log.write_bytes(data)
        report['commands'].append({'argv': command, 'exit_code': code, 'log': label(log), 'timeout_seconds': args.timeout_seconds})
        report['evidence'].append(label(log))
        save()
        return code, data.decode(errors='replace')

    def cp(paths):
        return os.pathsep.join(map(str, paths))

    save()
    try:
        outputs = {}
        for provider in ['official', 'candidate']:
            dest = work / (provider + '-output')
            dest.mkdir()
            command = [ROOT / 'references/kotlinc/bin/kotlinc', '-no-stdlib', '-no-reflect', '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8'] if provider == 'official' else [binary]
            code, diagnostic = run('compile-' + provider, [*command, SOURCE, '-cp', cp(classpath), '-d', dest])
            classes = emitted(dest)
            overlap = sorted({c['name'] for c in classes} & names)
            success = code == 0 and not overlap and bool(classes)
            case('compile/' + provider, 'pass' if success else 'oracle_failure' if provider == 'official' else 'unsupported_language' if code == 1 else 'compiler_failure', exit_code=code, diagnostic=diagnostic, classes=classes, classpath_overlap=overlap, output=label(dest))
            if success:
                outputs[provider] = dest
                if provider == 'candidate':
                    case('ownership/candidate', owned_outputs=ownership(dest, SOURCE))
        if 'official' in outputs:
            caller = work / 'caller'
            code, diagnostic = run('compile-java-caller', [jdk / 'bin/javac', '-cp', cp([outputs['official'], *classpath]), '-d', caller, CALLER])
            case('java_caller/compile', 'pass' if code == 0 else 'failure', diagnostic=diagnostic)
            if code == 0:
                runtime_cp = [p for p in classpath if p.name != 'android.jar']
                for provider, dest in outputs.items():
                    code, observed = run('java-caller-' + provider, [jdk / 'bin/java', '-Xverify:all', '-cp', cp([caller, dest, args.android_runtime, *runtime_cp]), 'ListenersCheck'])
                    case('java_caller/' + provider, 'pass' if code == 0 and observed == 'listener-checks=41\n' else 'failure', observed=observed, independent_assertions=41, jvm_verification='all')
        if len(outputs) == 2:
            signatures = {}
            for provider, dest in outputs.items():
                code, text = run('abi-' + provider, [jdk / 'bin/javap', '-public', '-s', '-classpath', cp([dest, *classpath]), 'yuku.alkitab.base.verses.VersesListeners', 'yuku.alkitab.base.verses.VersesListeners$Companion'])
                if code != 0:
                    raise ValueError('Cannot read public ABI')
                signatures[provider] = sorted(line.strip() for line in text.splitlines() if line.strip() and not line.startswith('Compiled from'))
            case('abi/public_java_signatures', 'pass' if signatures['official'] == signatures['candidate'] else 'failure', signatures=signatures)
    except Exception as error:
        case('harness', 'failure', error=str(error))
    changed = [name for name, before in frozen.items() if inventory(ROOT / name) != before]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_inputs': changed}
    report['status'] = 'invalidated' if changed else 'failed' if any(c['outcome'] != 'pass' for c in report['cases']) else 'complete'
    save()
    if not args.skip_html:
        subprocess.run([sys.executable, str(ROOT / 'tools/reports.py')], cwd=ROOT, check=True)
    print('Report: ' + label(output))
    print(json.dumps({'status': report['status'], 'counts': report['counts']}))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
