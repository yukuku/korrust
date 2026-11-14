#!/usr/bin/env python3
"""Verify nullable result annotations merge with ordinary and JVM method annotations."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


class Reader:
    def __init__(self, data):
        self.data, self.offset = data, 0

    def take(self, count):
        result = self.data[self.offset:self.offset + count]
        assert len(result) == count
        self.offset += count
        return result

    def number(self, count):
        return int.from_bytes(self.take(count), 'big')


def class_contract(path):
    reader = Reader(path.read_bytes())
    assert reader.number(4) == 0xcafebabe
    reader.take(4)
    pool = [None] * reader.number(2)
    index = 1
    while index < len(pool):
        tag = reader.number(1)
        if tag == 1:
            pool[index] = reader.take(reader.number(2)).decode('utf-8', errors='replace')
        elif tag in [3, 4]:
            reader.take(4)
        elif tag in [5, 6]:
            reader.take(8)
            index += 1
        elif tag in [7, 8, 16, 19, 20]:
            reader.take(2)
        elif tag in [9, 10, 11, 12, 17, 18]:
            reader.take(4)
        elif tag == 15:
            reader.take(3)
        else:
            raise AssertionError('Unknown constant tag ' + str(tag))
        index += 1
    def attributes():
        result = {}
        for _ in range(reader.number(2)):
            name = pool[reader.number(2)]
            result.setdefault(name, []).append(reader.take(reader.number(4)))
        for name in ['RuntimeVisibleAnnotations', 'RuntimeInvisibleAnnotations', 'RuntimeVisibleParameterAnnotations', 'RuntimeInvisibleParameterAnnotations']:
            assert len(result.get(name, [])) <= 1, (str(path), name)
        return result
    def annotation(source):
        name = pool[source.number(2)]
        values = []
        for _ in range(source.number(2)):
            key = pool[source.number(2)]
            tag = chr(source.number(1))
            assert tag == 's', (name, tag)
            values.append((key, pool[source.number(2)]))
        return name, tuple(values)
    def annotations(payloads):
        if not payloads:
            return []
        source = Reader(payloads[0])
        result = [annotation(source) for _ in range(source.number(2))]
        assert source.offset == len(source.data)
        return sorted(result)
    reader.take(6)
    reader.take(2 * reader.number(2))
    for _ in range(reader.number(2)):
        reader.take(6)
        attributes()
    methods = {}
    for _ in range(reader.number(2)):
        access, name, descriptor = reader.number(2), pool[reader.number(2)], pool[reader.number(2)]
        attrs = attributes()
        if access & 1 and access & 0x1000 == 0 and descriptor.endswith(')Ljava/lang/String;'):
            visible = annotations(attrs.get('RuntimeVisibleAnnotations', []))
            invisible = annotations(attrs.get('RuntimeInvisibleAnnotations', []))
            all_names = [entry[0] for entry in visible + invisible]
            assert all_names.count('Lorg/jetbrains/annotations/Nullable;') == 1, (str(path), name, descriptor, all_names)
            assert all_names.count('Lnullableannotations/RuntimeMark;') == 1
            assert all_names.count('Lnullableannotations/BinaryMark;') == 1
            assert 'Lnullableannotations/RuntimeMark;' in [entry[0] for entry in visible]
            assert 'Lnullableannotations/BinaryMark;' in [entry[0] for entry in invisible]
            methods[name + descriptor] = {'access': access, 'visible': visible, 'invisible': invisible}
    attributes()
    assert reader.offset == len(reader.data)
    return methods


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    candidate, source_root = args.candidate.resolve(), args.source_root.resolve()
    ident = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ') + '-nullable-annotation-merge'
    work, logs = ROOT / 'target/checks' / ident, ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True)
    logs.mkdir(parents=True)
    fixture = ROOT / 'tests/nullable_annotations'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    oracle = ROOT / 'references/kotlinc/bin/kotlinc'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    files = [*source_root.glob('src/**/*.rs'), source_root / 'Cargo.toml', source_root / 'Cargo.lock', *fixture.glob('*'), Path(__file__), ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
    frozen = {str(path): sha(path) for path in files + [candidate, standard] if path.is_file()}
    def relative(path):
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
    with zipfile.ZipFile(logs / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            if path.is_file():
                archive.write(path, relative(path))
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'compiler': {'path': str(candidate), 'sha256': sha(candidate), 'source_root': str(source_root)}, 'reference_compiler': json.loads((ROOT / 'references/compiler.json').read_text()), 'frozen_input_hashes': frozen, 'selection': 'Nullable String result methods with RUNTIME/CLASS Java markers and JvmOverloads/JvmStatic across top-level, class, object and companion emitters. Independent classfile parsing checks annotation attribute uniqueness, descriptors, payloads and retention; a fixed Java caller compiles once against official output and runs against both providers with -Xverify:all.', 'limitations': ['Focused generic annotation attribute merging regression; no broader annotation, app or performance claim.'], 'commands': [], 'cases': [], 'evidence': [relative(logs / 'source.zip')], 'reproduction_commands': [['python3', 'tools/check_nullable_annotations.py', '--candidate', str(candidate), '--source-root', str(source_root), '--skip-html']]}
    def run(label, command):
        result = subprocess.run(list(map(str, command)), cwd=ROOT, capture_output=True, text=True, timeout=300)
        index = len(report['commands'])
        entry = {'label': label, 'argv': list(map(str, command)), 'returncode': result.returncode}
        for stream in ['stdout', 'stderr']:
            path = logs / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
        report['commands'].append(entry)
        if result.returncode:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result.stdout
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def case(name, **detail):
        report['cases'].append({'id': name, 'outcome': 'pass', **detail})
    try:
        dependencies = work / 'dependencies'
        run('Compile common annotation definitions', [java / 'javac', '-d', dependencies, fixture / 'RuntimeMark.java', fixture / 'BinaryMark.java'])
        run('Compile official nullable annotation methods', [oracle, fixture / 'Methods.kt', '-cp', dependencies, '-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3', '-module-name', 'nullable_annotations', '-d', work / 'official'])
        run('Compile candidate nullable annotation methods', [candidate, fixture / 'Methods.kt', '-cp', cp(dependencies, standard), '--module-name', 'nullable_annotations', '-d', work / 'candidate'])
        case('matching-provider-compilations')
        run('Compile Java caller once against official output', [java / 'javac', '-cp', cp(work / 'official', dependencies, standard), '-d', work / 'caller', fixture / 'Calls.java'])
        caller_hashes = {str(path): sha(path) for path in (work / 'caller').rglob('*.class')}
        contracts, outputs = {}, {}
        for provider in ['official', 'candidate']:
            output = work / provider
            outputs[provider] = run('Verify fixed caller against ' + provider, [java / 'java', '-Xverify:all', '-cp', cp(work / 'caller', output, dependencies, standard), 'nullableannotations.Calls'])
            assert outputs[provider] == 'OK:77\n', outputs[provider]
            case('runtime/' + provider, independent_assertions=77)
            contracts[provider] = {str(path.relative_to(output)): class_contract(path) for path in output.rglob('*.class')}
            case('annotation-attribute-uniqueness-retention/' + provider, checked_source_methods=sum(len(methods) for methods in contracts[provider].values()))
        assert contracts['official'] == contracts['candidate'], {'official': contracts['official'], 'candidate': contracts['candidate']}
        case('matching-signatures-and-annotation-contracts', contracts=contracts['official'])
        assert caller_hashes == {str(path): sha(path) for path in (work / 'caller').rglob('*.class')}
        case('fixed-java-caller-integrity', class_hashes=caller_hashes)
        report['status'] = 'pass'
    except Exception as error:
        report['status'] = 'fail'
        report['cases'].append({'id': 'harness', 'outcome': 'fail', 'error': str(error)})
    changed = [path for path, digest in frozen.items() if sha(Path(path)) != digest]
    if changed:
        report['status'] = 'fail'
        report['cases'].append({'id': 'frozen-inputs', 'outcome': 'fail', 'paths': changed})
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    destination = logs.with_suffix('.json')
    destination.write_text(json.dumps(report, indent=2) + '\n')
    if not args.skip_html:
        subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(destination)
    print(json.dumps(report['counts'], sort_keys=True))
    return report['status'] != 'pass'


if __name__ == '__main__':
    raise SystemExit(main())
