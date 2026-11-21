#!/usr/bin/env python3
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

ROOT = Path(__file__).resolve().parents[1]
SELECTION = 'reports/correctness/2026-11-21T072519326520Z-next10-app-assessment.json'


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(path):
    if path.is_file():
        return {'sha256': digest(path)}
    if not path.is_dir():
        raise ValueError(f'Missing input: {path}')
    return {str(p.relative_to(path)): digest(p) for p in sorted(path.rglob('*')) if p.is_file()}


def class_info(data):
    offset = 0

    def take(n):
        nonlocal offset
        value = data[offset:offset + n]
        if len(value) != n:
            raise ValueError('Truncated class file')
        offset += n
        return value

    def number(n):
        return int.from_bytes(take(n), 'big')

    if take(4) != b'\xca\xfe\xba\xbe':
        raise ValueError('Invalid class file')
    number(2)
    major = number(2)
    pool = [None] * number(2)
    index = 1
    while index < len(pool):
        tag = number(1)
        if tag == 1:
            pool[index] = take(number(2)).decode('utf-8', errors='replace')
        elif tag in (7, 8, 16, 19, 20):
            pool[index] = number(2)
        elif tag in (3, 4, 9, 10, 11, 12, 17, 18):
            take(4)
        elif tag in (5, 6):
            take(8)
            index += 1
        elif tag == 15:
            take(3)
        else:
            raise ValueError(f'Unknown constant pool tag {tag}')
        index += 1
    flags, this_class = number(2), number(2)
    take(2)
    take(2 * number(2))

    def attributes():
        values = {}
        for _ in range(number(2)):
            name = pool[number(2)]
            values[name] = take(number(4))
        return values

    for _ in range(2):
        for _ in range(number(2)):
            take(6)
            attributes()
    attrs = attributes()
    source = pool[int.from_bytes(attrs['SourceFile'], 'big')] if 'SourceFile' in attrs else None
    return {'name': pool[pool[this_class]], 'source_file': source, 'class_major_version': major,
            'primary': not flags & 0x1000 and 'EnclosingMethod' not in attrs}


def class_entries(path):
    if path.is_dir():
        for p in sorted(path.rglob('*.class')):
            yield str(p.relative_to(path)), p.read_bytes()
    else:
        with zipfile.ZipFile(path) as archive:
            for name in sorted(archive.namelist()):
                if name.endswith('.class'):
                    yield name, archive.read(name)


def emitted(path):
    result = []
    for name, data in class_entries(path):
        result.append(dict(class_info(data), path=name, sha256=hashlib.sha256(data).hexdigest()))
    return result


def main():
    parser = argparse.ArgumentParser(description='Compile the fixed 33 AndroidBible cases with identical explicit inputs.')
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--dependencies', type=Path)
    parser.add_argument('--case', action='append', default=[], help='Exact case ID from the pinned selection.')
    parser.add_argument('--timeout-seconds', type=int, default=600)
    parser.add_argument('--standalone-oracle-evidence', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    if args.timeout_seconds <= 0:
        parser.error('--timeout-seconds must be positive')
    selection_path = ROOT / SELECTION
    selected = json.loads(selection_path.read_text())['cases']
    if len(selected) != 33 or len({case['id'] for case in selected}) != 33:
        raise ValueError('The pinned selection must contain exactly 33 distinct cases')
    unknown = set(args.case) - {case['id'] for case in selected}
    if unknown:
        parser.error(f'Unknown case IDs: {sorted(unknown)}')
    all_ids = {case['id'] for case in selected}
    for case in selected:
        if digest(ROOT / case['id']) != case['sha256']:
            raise ValueError(f'Pinned source changed: {case["id"]}')
    selected = [case for case in selected if not args.case or case['id'] in args.case]
    config = json.loads(args.dependencies.read_text()) if args.dependencies else {'cases': {}}
    if set(config['cases']) - all_ids:
        raise ValueError('Dependency configuration contains cases outside the pinned selection')
    prior_oracle = None
    if args.standalone_oracle_evidence:
        prior_oracle = json.loads(args.standalone_oracle_evidence.read_text())
        pinned = {c['id']: c['sha256'] for c in json.loads(selection_path.read_text())['cases']}
        if len(prior_oracle) != 33 or {c['source']: c['sha256'] for c in prior_oracle} != pinned:
            raise ValueError('Standalone oracle receipt must identify all 33 pinned cases and hashes')
    if args.dependencies:
        if prior_oracle is None:
            parser.error('--dependencies requires --standalone-oracle-evidence')
        needs_deps = {c['source'] for c in prior_oracle if c['exit'] != 0}
        required = {c['id'] for c in selected} & needs_deps
        if set(config['cases']) - needs_deps or required - set(config['cases']):
            raise ValueError('Dependency mapping must cover selected oracle dependency rejections and cannot add dependencies to standalone successes')
    mode = 'dependencies' if args.dependencies else 'standalone'
    stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ')
    ident = stamp + '-androidbible-survey-' + mode
    evidence = ROOT / 'reports/correctness' / ident
    work = ROOT / 'target/checks' / ident
    evidence.mkdir(parents=True, exist_ok=False)
    work.mkdir(parents=True, exist_ok=False)
    report_path = evidence.with_suffix('.json')

    def label(path):
        path = path.resolve()
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)

    def resolve(path):
        return (ROOT / path).resolve()

    frozen = {}
    copies = {}
    class_names = {}

    def track(path):
        path = path.resolve()
        key = label(path)
        if key not in frozen:
            frozen[key] = inventory(path)
        return path

    def freeze_classpath(path):
        path = track(path)
        if path not in copies:
            dest = work / 'classpath' / f'{len(copies):03d}' / path.name
            dest.parent.mkdir(parents=True)
            if path.is_dir():
                shutil.copytree(path, dest)
            else:
                shutil.copy2(path, dest)
            if inventory(dest) != frozen[label(path)]:
                raise ValueError(f'Input changed during copy: {path}')
            copies[path] = dest
            class_names[dest] = {class_info(data)['name'] for _, data in class_entries(dest)}
        return copies[path]

    binaries = {}
    identities = {}
    for name in ('baseline', 'candidate'):
        original = getattr(args, name)
        if original:
            original = track(original)
            binary = work / 'bin' / name
            binary.parent.mkdir(exist_ok=True)
            shutil.copy2(original, binary)
            binaries[name] = binary
            identities[name] = {'path': label(original), 'sha256': digest(original), 'frozen_path': label(binary)}
    kotlinc = track(ROOT / 'references/kotlinc/bin/kotlinc')
    track(ROOT / 'references/kotlinc/lib')
    java_home = Path(os.environ['JAVA_HOME']).resolve()
    track(java_home / 'jmods')
    track(java_home / 'bin/java')
    track(Path(__file__))
    track(selection_path)
    track(ROOT / 'references/compiler.json')
    if args.dependencies:
        track(args.dependencies)
    prior_archives = []
    if args.standalone_oracle_evidence:
        receipt = track(args.standalone_oracle_evidence)
        archive_dir = evidence / 'prior-standalone-oracle'
        archive_dir.mkdir()
        oracle_files = [receipt, *(receipt.parent / (Path(c['source']).stem + '.log') for c in prior_oracle)]
        recipe = receipt.parent.parent / 'oracle.py'
        if recipe.is_file():
            oracle_files.append(recipe)
        for original in oracle_files:
            original = track(original)
            dest = archive_dir / original.name
            shutil.copy2(original, dest)
            prior_archives.append({'original': label(original), 'archive': label(dest), 'sha256': digest(dest)})
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    plans = []
    provenance_copies = {}
    for case in selected:
        spec = config['cases'].get(case['id'], {})
        extra = spec.get('classpath', [])
        target = spec.get('official_jvm_target', '1.8')
        if target != '1.8' and not spec.get('jvm_target_reason'):
            raise ValueError(f'A higher oracle JVM target requires a dependency reason: {case["id"]}')
        friends = [resolve(p) for p in spec.get('friend_paths', [])]
        original_cp = list(dict.fromkeys([stdlib.resolve(), *(resolve(p) for p in extra)]))
        if set(friends) - set(original_cp):
            raise ValueError(f'Friend paths must be explicit classpath entries: {case["id"]}')
        if extra and not spec.get('provenance'):
            raise ValueError(f'Dependency provenance is required: {case["id"]}')
        for item in spec.get('provenance', []):
            if isinstance(item, dict) and 'path' in item:
                origin = track(resolve(item['path']))
                if not origin.is_file():
                    raise ValueError(f'Provenance must identify a receipt or replay script file: {origin}')
                if origin not in provenance_copies:
                    destination = evidence / 'dependency-provenance' / f'{len(provenance_copies):03d}-{origin.name}'
                    destination.parent.mkdir(exist_ok=True)
                    shutil.copy2(origin, destination)
                    provenance_copies[origin] = destination
        sources = [track(ROOT / case['id']), *(track(resolve(p)) for p in spec.get('sources', []))]
        if len(set(sources)) != len(sources):
            raise ValueError(f'Duplicate source inputs: {case["id"]}')
        plans.append((case, spec, sources, [freeze_classpath(p) for p in original_cp],
                      [freeze_classpath(p) for p in friends]))
    archive_inputs = {Path(__file__).resolve(), selection_path, ROOT / 'references/compiler.json'}
    archive_inputs.update(source for _, _, sources, _, _ in plans for source in sources)
    if args.dependencies:
        archive_inputs.add(args.dependencies.resolve())
    archive_map = {}
    input_archive = evidence / 'inputs.zip'
    with zipfile.ZipFile(input_archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
        for index, original in enumerate(sorted(archive_inputs)):
            member = str(original.relative_to(ROOT)) if original.is_relative_to(ROOT) else f'external/{index:03d}-{original.name}'
            bundle.write(original, member)
            archive_map[label(original)] = {'member': member, 'sha256': digest(original)}
        bundle.writestr('input-map.json', json.dumps(archive_map, indent=2) + '\n')
    input_archive_hash = digest(input_archive)
    report = {
        'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(),
        'status': 'running', 'mode': mode, 'selection_report': SELECTION,
        'assessment_scope': 'full_33' if len(selected) == 33 else 'selected_subset',
        'pinned_case_count': 33, 'selected_case_count': len(selected),
        'input_archive': {'path': label(input_archive), 'sha256': input_archive_hash, 'members': archive_map},
        'selection': 'Exact case IDs and source hashes from the pinned 33-file survey; no source rewriting. '
                     + ('Only JDK and pinned standard library.' if mode == 'standalone' else 'Explicit genuine dependencies and friend paths; recorded separately from standalone acceptance.'),
        'compiler': identities, 'reference_compiler': json.loads((ROOT / 'references/compiler.json').read_text()),
        'source_revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'environment': {'JAVA_HOME': str(java_home), 'official_jvm_target': '1.8 unless the case records a dependency requirement', 'candidate_jvm_target': 'compiler default; emitted class versions recorded per class', 'language_version': '2.3', 'api_version': '2.3'},
        'input_hashes': frozen, 'cases': [], 'commands': [], 'evidence': [],
        'dependency_provenance_copies': [{'original': label(p), 'archive': label(q), 'sha256': digest(q)} for p, q in provenance_copies.items()],
        'classpath_copies': [{'original': label(p), 'copy': label(q)} for p, q in copies.items()],
        'prior_standalone_oracle_archives': prior_archives,
        'prior_standalone_oracle': label(args.standalone_oracle_evidence) if args.standalone_oracle_evidence else None,
        'reproduction_commands': [[sys.executable, label(Path(__file__)), *sys.argv[1:]]],
        'limitations': ['Compile and emitted-class assessment only; this does not establish runtime, whole-module, or Android-device compatibility.',
                        'Oracle-generated synthetic and local classes need not have identical names; primary source classes must be emitted.'],
    }

    def save():
        report['counts'] = dict(Counter(c['outcome'] for c in report['cases']))
        report['provider_counts'] = {name: dict(Counter(c['providers'][name]['outcome'] for c in report['cases'] if name in c['providers']))
                                     for name in ('official', *binaries)}
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(name, sources, cp, friends, index, target):
        destination = work / 'output' / f'{index:02d}' / name
        destination.mkdir(parents=True, exist_ok=False)
        common = [str(p) for p in sources] + ['-cp', os.pathsep.join(map(str, cp)), '-d', str(destination)]
        if name == 'official':
            command = [str(kotlinc), *common, '-no-stdlib', '-no-reflect', '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', target]
            if friends:
                command.append('-Xfriend-paths=' + ','.join(map(str, friends)))
        else:
            command = [str(binaries[name]), *common]
            if friends:
                command += ['--friend-paths', os.pathsep.join(map(str, friends))]
        log = evidence / f'{index:02d}-{name}.log'
        try:
            result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=args.timeout_seconds)
            log.write_bytes(result.stdout)
            crashed = result.returncode not in (0, 1) or b'panicked at' in result.stdout or b'Exception in thread' in result.stdout
            state = {'outcome': 'tool_error' if crashed else 'compiled' if result.returncode == 0 else 'rejected', 'exit_code': result.returncode}
        except subprocess.TimeoutExpired as error:
            log.write_bytes(error.stdout or b'')
            state = {'outcome': 'timeout', 'exit_code': None}
        state.update(log=label(log), output=label(destination), classes=emitted(destination))
        report['commands'].append({'argv': command, 'exit_code': state['exit_code'], 'timeout_seconds': args.timeout_seconds, 'log': label(log)})
        report['evidence'].append(label(log))
        return state

    save()
    for index, (case, spec, sources, cp, friends) in enumerate(plans):
        current = {'id': case['id'], 'sha256': case['sha256'], 'source_inputs': [label(p) for p in sources],
                   'classpath': [label(p) for p in cp], 'friend_paths': [label(p) for p in friends],
                   'dependency_provenance': spec, 'official_jvm_target': spec.get('official_jvm_target', '1.8'), 'providers': {}}
        current['providers']['official'] = oracle = run('official', sources, cp, friends, index, current['official_jvm_target'])
        overlap = sorted({c['name'] for c in oracle['classes']} & set().union(*(class_names[p] for p in cp)))
        current['classpath_overlap'] = overlap
        if overlap:
            current['outcome'] = 'classpath_overlap'
        else:
            for name in binaries:
                current['providers'][name] = run(name, sources, cp, friends, index, current['official_jvm_target'])
            candidate = current['providers']['candidate']
            primary = {c['name'] for c in oracle['classes'] if c['primary'] and c['source_file'] in {p.name for p in sources}}
            surveyed_primary = {c['name'] for c in oracle['classes'] if c['primary'] and c['source_file'] == Path(case['id']).name}
            missing = sorted(primary - {c['name'] for c in candidate['classes']})
            current['surveyed_primary_classes'] = sorted(surveyed_primary)
            current['expected_primary_classes'] = sorted(primary)
            current['missing_primary_classes'] = missing
            for name in binaries:
                observation = current['providers'][name]
                observation['missing_primary_classes'] = sorted(primary - {c['name'] for c in observation['classes']})
                if oracle['outcome'] == 'compiled' and observation['outcome'] == 'compiled' and (not surveyed_primary or observation['missing_primary_classes']):
                    observation['outcome'] = 'emission_mismatch'
            if any(p['outcome'] == 'tool_error' for p in current['providers'].values()):
                current['outcome'] = 'tool_error'
            elif any(p['outcome'] == 'timeout' for p in current['providers'].values()):
                current['outcome'] = 'timeout'
            elif any(p['outcome'] == 'emission_mismatch' for p in current['providers'].values()):
                current['outcome'] = 'emission_mismatch'
            elif oracle['outcome'] != 'compiled':
                current['outcome'] = 'candidate_accepted_oracle_rejected' if candidate['outcome'] == 'compiled' else 'oracle_rejected'
            elif candidate['outcome'] != 'compiled':
                current['outcome'] = 'candidate_rejected'
            elif not surveyed_primary or missing:
                current['outcome'] = 'emission_mismatch'
            else:
                current['outcome'] = 'compiled'
        report['cases'].append(current)
        save()
    changed = [path for path, before in frozen.items() if inventory(resolve(path)) != before]
    changed_copies = [label(copy) for original, copy in copies.items() if inventory(copy) != frozen[label(original)]]
    archive_changed = digest(input_archive) != input_archive_hash
    changed_binaries = [name for name, path in binaries.items() if digest(path) != identities[name]['sha256']]
    report['input_integrity'] = {'changed_originals': changed, 'changed_classpath_copies': changed_copies, 'changed_binary_copies': changed_binaries, 'input_archive_changed': archive_changed}
    bad = [c['id'] for c in report['cases'] if c['outcome'] != 'compiled' and not (mode == 'standalone' and c['outcome'] == 'oracle_rejected')]
    report['status'] = 'invalidated' if changed or changed_copies or changed_binaries or archive_changed else 'failed' if bad else 'complete'
    save()
    if not args.skip_html:
        subprocess.run([sys.executable, str(ROOT / 'tools/reports.py')], cwd=ROOT, check=True)
    print(json.dumps({'report': label(report_path), 'status': report['status'], 'counts': report['counts'], 'provider_counts': report['provider_counts']}))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
