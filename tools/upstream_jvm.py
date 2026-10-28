#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DIRECTIVE = re.compile(r'^\s*//\s*([A-Z][A-Z0-9_]+)(?::\s*(.*))?\s*$')
STRUCTURE = re.compile(r'^\s*//\s*(FILE|MODULE):\s*(.*?)\s*$')
MARKER = re.compile(r'<!>|<!([^\n]*?)!>')
SUPPORTED = {
    'WITH_STDLIB', 'FULL_JDK', 'WITH_RUNTIME', 'TARGET_BACKEND', 'JVM_TARGET',
    'LANGUAGE', 'LANGUAGE_VERSION', 'API_VERSION', 'FILE', 'MODULE',
}
EXCLUDED = {'IGNORE_BACKEND', 'IGNORE_BACKEND_K1', 'IGNORE_BACKEND_K2'}
METADATA = {'FIR_IDENTICAL', 'RUN_PIPELINE_TILL'}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    try:
        return str(Path(path).resolve().relative_to(ROOT))
    except ValueError:
        return str(Path(path).resolve())


def strip_markers(text, origins):
    clean = []
    mapping = []
    expected = []
    stack = []
    cursor = 0
    for marker in MARKER.finditer(text):
        clean.append(text[cursor:marker.start()])
        mapping.extend(origins[cursor:marker.start()])
        offset = len(mapping)
        if marker.group() == '<!>':
            if not stack:
                raise ValueError('Unmatched diagnostic closing marker')
            names, start, original = stack.pop()
            expected.append({'names': names, 'start': start, 'end': offset,
                             'original_start': original})
        else:
            names = re.findall(r'(?:^|,\s*)([A-Z][A-Z0-9_]*)', marker.group(1))
            if not names:
                raise ValueError('Unsupported diagnostic marker: ' + marker.group())
            stack.append((names, offset, origins[marker.start()]))
        cursor = marker.end()
    clean.append(text[cursor:])
    mapping.extend(origins[cursor:])
    if stack:
        raise ValueError('Unclosed diagnostic marker')
    return ''.join(clean), mapping, expected


def module_header(value):
    match = re.fullmatch(r'([\w-]+)((?:\([^()]*\)){0,3})', value)
    if not match:
        raise ValueError('Unsupported MODULE syntax: ' + value)
    groups = re.findall(r'\(([^()]*)\)', match.group(2))
    lists = [[name.strip() for name in group.split(',') if name.strip()] for group in groups]
    lists.extend([[] for _ in range(3 - len(lists))])
    return {'name': match.group(1), 'dependencies': lists[0], 'friends': lists[1],
            'depends_on': lists[2], 'files': [], 'directives': []}


def parse_fixture(path):
    path = Path(path)
    original = path.read_text()
    modules = []
    module = module_header('main')
    file_name = path.name
    parts = []
    origins = []
    global_directives = []
    seen_structure = False
    offset = 0

    def flush():
        nonlocal parts, origins
        if parts and re.sub(r'/\*.*?\*/|^\s*//[^\n]*', '', ''.join(parts), flags=re.DOTALL | re.MULTILINE).strip():
            text, mapping, expected = strip_markers(''.join(parts), origins)
            module['files'].append({'name': file_name, 'text': text,
                                    'original_char_offsets': mapping, 'expected_diagnostics': expected})
        parts, origins = [], []

    for line in original.splitlines(keepends=True):
        structure = STRUCTURE.match(line)
        directive = DIRECTIVE.match(line.rstrip('\r\n'))
        if structure:
            kind, value = structure.groups()
            if kind == 'MODULE':
                flush()
                if module['files']:
                    modules.append(module)
                module = module_header(value)
                file_name = path.name
            else:
                flush()
                file_name = value
                if Path(value).is_absolute() or '..' in Path(value).parts:
                    raise ValueError('Unsafe FILE name: ' + value)
            seen_structure = True
        else:
            if directive:
                item = {'name': directive.group(1), 'value': (directive.group(2) or '').strip(),
                        'original_line': original.count('\n', 0, offset) + 1}
                (module['directives'] if seen_structure else global_directives).append(item)
            parts.append(line)
            origins.extend(range(offset, offset + len(line)))
        offset += len(line)
    flush()
    if module['files']:
        modules.append(module)
    if not modules:
        raise ValueError('No source files')
    if len({item['name'] for item in modules}) != len(modules):
        raise ValueError('Duplicate module names')
    for item in modules:
        item['directives'] = global_directives + item['directives']
        if len({file['name'] for file in item['files']}) != len(item['files']):
            raise ValueError('Duplicate FILE names in module ' + item['name'])
    return {'path': relative(path), 'sha256': sha(path), 'original': original, 'modules': modules}


def assess(fixture, kind):
    reasons = []
    excluded = []
    names = {module['name'] for module in fixture['modules']}
    for module in fixture['modules']:
        if module['depends_on']:
            reasons.append('dependsOn module refinement requires multiplatform harness')
        for dependency in module['dependencies'] + module['friends']:
            if dependency not in names:
                reasons.append('Unknown module dependency: ' + dependency)
        for directive in module['directives']:
            name, value = directive['name'], directive['value']
            if name in EXCLUDED:
                if any(backend in {'JVM', 'JVM_IR', 'ANY'} for backend in re.split(r'[,\s]+', value)):
                    excluded.append(name + ': ' + value)
            elif name == 'TARGET_BACKEND':
                if not any(backend in {'JVM', 'JVM_IR', 'ANY'} for backend in re.split(r'[,\s]+', value)):
                    excluded.append(name + ': ' + value)
            elif name == 'RUN_PIPELINE_TILL' and value not in {'BACKEND', 'FRONTEND'}:
                reasons.append('Unsupported pipeline: ' + value)
            elif name not in SUPPORTED | METADATA:
                reasons.append('Unsupported directive ' + name + (': ' + value if value else ''))
        for file in module['files']:
            if Path(file['name']).suffix not in {'.kt', '.java'}:
                reasons.append('Unsupported source extension: ' + file['name'])
    if kind == 'incremental':
        reasons.append('Incremental scenario dependency snapshots/edit scripts are not implemented')
    if excluded:
        return 'excluded_target', sorted(set(excluded))
    return ('harness_unsupported', sorted(set(reasons))) if reasons else ('ready', [])


def source_location(fixture, file, clean_char):
    mapping = file['original_char_offsets']
    clean_char = min(max(clean_char, 0), len(mapping))
    origin = mapping[clean_char] if clean_char < len(mapping) else (mapping[-1] + 1 if mapping else 0)
    original = fixture['original']
    before = original[:origin]
    return {'path': fixture['path'], 'char_offset': origin,
            'byte_offset': len(before.encode()), 'line': before.count('\n') + 1,
            'column': len(before.rsplit('\n', 1)[-1]) + 1}


def options_for(module):
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    for item in module['directives']:
        name, value = item['name'], item['value']
        if name in {'LANGUAGE_VERSION', 'API_VERSION', 'JVM_TARGET'}:
            flag = {'LANGUAGE_VERSION': '-language-version', 'API_VERSION': '-api-version',
                    'JVM_TARGET': '-jvm-target'}[name]
            options[options.index(flag) + 1] = value
        elif name == 'LANGUAGE':
            options.extend('-XXLanguage:' + feature for feature in re.split(r'[,\s]+', value) if feature)
    return options


class Commands:
    def __init__(self, directory, environment):
        self.directory = directory
        self.environment = environment
        self.entries = []

    def run(self, label, args, timeout=120):
        number = len(self.entries)
        entry = {'label': label, 'argv': [str(arg) for arg in args], 'cwd': relative(ROOT),
                 'timeout_seconds': timeout}
        self.entries.append(entry)
        try:
            result = subprocess.run(entry['argv'], cwd=ROOT, env=self.environment,
                                    capture_output=True, text=True, timeout=timeout)
            entry['exit_code'] = result.returncode
            stdout, stderr = result.stdout, result.stderr
        except subprocess.TimeoutExpired as error:
            entry['timed_out'] = True
            stdout, stderr = error.stdout or '', error.stderr or ''
            stdout = stdout.decode(errors='replace') if isinstance(stdout, bytes) else stdout
            stderr = stderr.decode(errors='replace') if isinstance(stderr, bytes) else stderr
            result = None
        for stream, content in [('stdout', stdout), ('stderr', stderr)]:
            output = self.directory / f'{number:04d}-{stream}.txt'
            output.write_text(content)
            entry[stream] = relative(output)
        return result


def normalize_signature(text):
    return sorted(line.strip() for line in text.splitlines()
                  if line.strip() and not line.startswith('Compiled from'))


def classify_candidate(result):
    if result is None:
        return 'timeout'
    if result.returncode == 0:
        return 'compiled'
    diagnostics = []
    for line in result.stderr.splitlines():
        try:
            diagnostics.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    categories = {item.get('category') for item in diagnostics}
    if categories == {'unsupported_language'}:
        return 'unsupported_language'
    if categories == {'missing_dependency'}:
        return 'dependency_failure'
    if categories and categories <= {'setup', 'io'}:
        return 'setup_failure'
    if not diagnostics and re.search(r'unsupported|not yet supported|not supported', result.stderr, re.IGNORECASE):
        return 'unsupported_language'
    return 'correctness_failure'


def write_sources(fixture, directory):
    modules = {}
    for module in fixture['modules']:
        paths = []
        mappings = []
        for source in module['files']:
            path = directory / module['name'] / source['name']
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(source['text'])
            paths.append(path)
            mapping_path = path.with_suffix(path.suffix + '.source-map.json')
            mapping_path.write_text(json.dumps({'original': fixture['path'], 'original_sha256': fixture['sha256'],
                'materialized': relative(path), 'sha256': sha(path),
                'original_char_offsets': source['original_char_offsets'],
                'expected_diagnostics': source['expected_diagnostics']}, indent=2) + '\n')
            mappings.append(relative(mapping_path))
        modules[module['name']] = {'paths': paths, 'source_maps': mappings}
    return modules


def mapped_diagnostics(result, fixture, sources):
    if result is None:
        return []
    by_path = {str(path.resolve()): file for module in fixture['modules']
               for path, file in zip(sources[module['name']]['paths'], module['files'])}
    found = []
    for line in result.stderr.splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        file = by_path.get(str(Path(item.get('path') or item.get('source_identity') or '').resolve()))
        if file is not None:
            offset = item.get('offset', 0)
            clean_char = len(file['text'].encode()[:offset].decode(errors='ignore'))
            item['upstream_location'] = source_location(fixture, file, clean_char)
        found.append(item)
    return found


def run_case(entry, context):
    case = {'id': entry['id'], 'family': entry['family'], 'root': entry['root'],
            'tier': entry['tier'], 'kind': entry['kind'], 'outcome': 'not_run'}
    directory = context['directory'] / entry['id']
    directory.mkdir(parents=True)
    try:
        fixture = parse_fixture(ROOT / entry['path'])
    except (ValueError, OSError) as error:
        case.update(outcome='harness_unsupported', reasons=[str(error)])
        return case
    if fixture['sha256'] != entry['sha256']:
        case.update(outcome='harness_unsupported', reasons=['Original fixture differs from frozen matrix hash'])
        return case
    case['source'] = {'path': fixture['path'], 'sha256': fixture['sha256']}
    case['modules'] = [{key: value for key, value in module.items() if key != 'files'}
                       for module in fixture['modules']]
    outcome, reasons = assess(fixture, entry['kind'])
    if outcome != 'ready':
        case.update(outcome=outcome, reasons=reasons)
        return case
    sources = write_sources(fixture, directory / 'sources')
    case['source_maps'] = [item for module in sources.values() for item in module['source_maps']]
    stdlib, testlib = context['stdlib'], context['testlib']
    support = [stdlib, testlib]
    helper_entries = entry.get('helpers', [])
    if helper_entries:
        case['helpers'] = [{'path': helper, 'sha256': sha(ROOT / helper)} for helper in helper_entries]
        case.update(outcome='harness_unsupported', reasons=['Explicit helper compilation is not implemented'])
        return case
    java_files = [path for source in sources.values() for path in source['paths'] if path.suffix == '.java']
    if entry['kind'] in {'java_consumer', 'kotlin_consumer'}:
        sibling = (ROOT / entry['path']).with_suffix('.java')
        if not sibling.is_file():
            case.update(outcome='harness_unsupported', reasons=['No explicit paired Java source'])
            return case
        destination = directory / 'sources' / sibling.name
        shutil.copyfile(sibling, destination)
        java_files.append(destination)
        case['paired_java'] = {'path': relative(sibling), 'sha256': sha(sibling)}
    commands = context['commands']
    outputs = {'official': {}, 'candidate': {}}
    candidate_failures = []
    for provider in ['official', 'candidate']:
        pending = list(fixture['modules'])
        while pending:
            available = next((module for module in pending
                              if all(name in outputs[provider] for name in module['dependencies'] + module['friends'])), None)
            if available is None:
                case.update(outcome='harness_unsupported', reasons=['Cyclic module graph'])
                return case
            pending.remove(available)
            module = available
            name = module['name']
            out = directory / provider / name
            out.mkdir(parents=True)
            dependencies = [outputs[provider][dep] for dep in dict.fromkeys(module['dependencies'] + module['friends'])]
            cp = [*dependencies, *support]
            local_java = [path for path in sources[name]['paths'] if path.suffix == '.java']
            if entry['kind'] == 'kotlin_consumer':
                local_java = java_files
            if local_java:
                java_out = directory / 'java-dependencies' / name
                if provider == 'official':
                    compile_java = commands.run(entry['id'] + '/javac-dependency',
                        [context['javac'], '-cp', os.pathsep.join(map(str, cp)), '-d', java_out, *local_java])
                    if compile_java is None or compile_java.returncode:
                        case.update(outcome='harness_unsupported', reasons=['Java dependency cannot compile independently; mixed cyclic Java/Kotlin requires adapter support'])
                        return case
                cp.append(java_out)
            kotlin = [path for path in sources[name]['paths'] if path.suffix == '.kt']
            if not kotlin:
                outputs[provider][name] = java_out
                continue
            common = [*kotlin, '-module-name' if provider == 'official' else '--module-name', name,
                      '-cp', os.pathsep.join(map(str, cp)), '-d', out]
            friends = [outputs[provider][friend] for friend in module['friends']]
            if friends:
                common += [('' if provider == 'official' else '-') + '-friend-paths', os.pathsep.join(map(str, friends))]
            if provider == 'official':
                args = [context['official'], *common, *options_for(module), '-Xrender-internal-diagnostic-names']
                if friends:
                    args = [arg for arg in args if arg != '-friend-paths']
                    friend_index = args.index(os.pathsep.join(map(str, friends)))
                    args[friend_index] = '-Xfriend-paths=' + args[friend_index]
            else:
                unsupported_options = [item for item in module['directives']
                                       if item['name'] in {'LANGUAGE', 'LANGUAGE_VERSION', 'API_VERSION', 'JVM_TARGET'}
                                       and (item['name'] == 'LANGUAGE' or item['value'] not in {'2.3', '1.8'})]
                if unsupported_options:
                    case.update(outcome='harness_unsupported', reasons=['Candidate CLI cannot match requested language/JVM settings'], settings=unsupported_options)
                    return case
                args = [context['candidate'], *common, '--diagnostic-format', 'json']
            result = commands.run(entry['id'] + '/' + provider + '/' + name, args)
            if provider == 'official' and (result is None or result.returncode):
                if entry['kind'] == 'diagnostics' and result is not None and result.returncode:
                    case.update(outcome='harness_unsupported', reasons=['Diagnostic sources and marker ranges mapped; diagnostic category/range equivalence is not implemented'])
                else:
                    case.update(outcome='timeout' if result is None else 'oracle_failure', reasons=['Official compiler did not accept selected materialized source'])
                return case
            if provider == 'candidate' and (result is None or result.returncode):
                candidate_failures.append(classify_candidate(result))
                case['candidate_diagnostics'] = mapped_diagnostics(result, fixture, sources)
                break
            outputs[provider][name] = out
    if candidate_failures:
        case['outcome'] = candidate_failures[0]
        return case
    if entry['kind'] == 'diagnostics':
        case.update(outcome='harness_unsupported', reasons=['Positive diagnostics case compiled; diagnostic warning/category equivalence is not implemented'])
        return case
    classpaths = {}
    for provider in outputs:
        classpaths[provider] = [*outputs[provider].values(), *support,
                               *[directory / 'java-dependencies' / name for name in sources
                                 if (directory / 'java-dependencies' / name).exists()]]
    official_cp = os.pathsep.join(map(str, classpaths['official']))
    if entry['kind'] == 'java_consumer':
        consumer = directory / 'java-consumer'
        result = commands.run(entry['id'] + '/compile-upstream-java-against-official',
            [context['javac'], '-cp', official_cp, '-d', consumer, *java_files])
        if result is None or result.returncode:
            case.update(outcome='oracle_failure', reasons=['Upstream Java caller failed against official output'])
            return case
        case['java_caller_class_hashes'] = {relative(path): sha(path) for path in consumer.rglob('*.class')}
    if entry['kind'] in {'box', 'inline', 'spec_box'}:
        boxes = [(module, source) for module in fixture['modules'] for source in module['files']
                 if re.search(r'\bfun\s+box\s*\(\s*\)', source['text'])]
        if len(boxes) != 1:
            case.update(outcome='harness_unsupported', reasons=['Expected one top-level parameterless box entry point'])
            return case
        module, source = boxes[0]
        package = re.search(r'^\s*package\s+([\w.]+)', source['text'], re.MULTILINE)
        facade_annotation = re.search(r'@file:JvmName\("([^"]+)"\)', source['text'])
        stem = Path(source['name']).stem
        facade = (facade_annotation.group(1) if facade_annotation else stem[:1].upper() + stem[1:] + 'Kt')
        if not re.match(r'[A-Za-z_$]', facade):
            facade = '_' + facade
        facade = (package.group(1) + '.' if package else '') + facade
        driver = directory / 'UpstreamBox.java'
        driver.write_text('public final class UpstreamBox { public static void main(String[] args) throws Exception { '
            + 'String actual = ' + facade + '.box(); '
            + 'if (!"OK".equals(actual)) throw new AssertionError(actual); System.out.println("OK"); } }\n')
        driver_out = directory / 'caller'
        result = commands.run(entry['id'] + '/compile-box-caller-against-official',
                              [context['javac'], '-cp', official_cp, '-d', driver_out, driver])
        if result is None or result.returncode:
            case.update(outcome='harness_unsupported', reasons=['Cannot compile direct Java box caller'])
            return case
        hashes = {relative(path): sha(path) for path in driver_out.rglob('*.class')}
        case['java_caller_class_hashes'] = hashes
        for provider, cp in classpaths.items():
            result = commands.run(entry['id'] + '/verify-box/' + provider,
                [context['java'], '-Xverify:all', '-cp', os.pathsep.join(map(str, [driver_out, *cp])), 'UpstreamBox'])
            if result is None or result.returncode or result.stdout != 'OK\n':
                case.update(outcome='timeout' if result is None else ('oracle_failure' if provider == 'official' else 'correctness_failure'),
                            reasons=['box assertion or JVM verification failed for ' + provider])
                return case
        if any(sha(ROOT / path) != value for path, value in hashes.items()):
            raise AssertionError('Official-compiled Java caller changed')
        case['runtime'] = 'Original upstream box() asserts OK with -Xverify:all; identical Java caller bytecode reused'
        consumer = directory / 'UpstreamConsumer.kt'
        import_name = (package.group(1) + '.' if package else '') + 'box'
        consumer.write_text('package korrust.consumer\nimport ' + import_name + '\nfun main() { check(box() == "OK") }\n')
        consumer_outcomes = {}
        for provider, cp in classpaths.items():
            consumer_out = directory / ('kotlin-consumer-' + provider)
            result = commands.run(entry['id'] + '/official-kotlin-consumer-compile/' + provider,
                [context['official'], consumer, '-cp', os.pathsep.join(map(str, cp)), '-d', consumer_out,
                 '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8'])
            if result is not None and result.returncode == 0:
                result = commands.run(entry['id'] + '/official-kotlin-consumer-verify/' + provider,
                    [context['java'], '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, *cp])),
                     'korrust.consumer.UpstreamConsumerKt'])
            if result is None or result.returncode:
                case.update(outcome='timeout' if result is None else ('oracle_failure' if provider == 'official' else 'correctness_failure'),
                            reasons=['Official Kotlin consumer compile or runtime assertion failed for ' + provider])
                return case
            consumer_outcomes[provider] = 'compiled with pinned Kotlin and executed with JVM verification'
        case['official_kotlin_consumers'] = consumer_outcomes
        case['kotlin_consumer_scope'] = 'Top-level box declaration metadata/name resolution and execution only; no complete metadata equivalence claim'

    signature_differences = []
    for name, official_out in outputs['official'].items():
        candidate_out = outputs['candidate'][name]
        for classfile in sorted(official_out.rglob('*.class')):
            class_name = str(classfile.relative_to(official_out).with_suffix('')).replace(os.sep, '.')
            if '$' in class_name:
                continue
            results = [commands.run(entry['id'] + '/signatures/' + provider + '/' + class_name,
                [context['javap'], '-protected', '-s', '-classpath', os.pathsep.join(map(str, classpaths[provider])), class_name])
                       for provider in ['official', 'candidate']]
            if any(result is None or result.returncode for result in results) or normalize_signature(results[0].stdout) != normalize_signature(results[1].stdout):
                signature_differences.append(class_name)
    if signature_differences:
        case.update(outcome='correctness_failure', reasons=['Public/protected JVM descriptors differ'], signature_differences=signature_differences)
        return case
    if entry['kind'] == 'java_consumer':
        case.update(outcome='harness_unsupported', reasons=['Original Java caller compiled once against official output and JVM signatures compared; no upstream executable assertion entry point'])
        return case
    case['outcome'] = 'pass'
    case['signature_scope'] = 'Top-level public/protected JVM declarations and descriptors; nested and generated class signatures are unassessed'
    case['scope'] = 'Original source execution and top-level JVM descriptors' if 'runtime' in case else 'Compilation and top-level JVM descriptors only; no runtime assertion'
    return case


def publish_report(report, directory, destination):
    artifact = destination.with_suffix('')
    if destination.exists() or artifact.exists():
        raise RuntimeError('Refusing to overwrite historical report: ' + str(destination))
    artifact.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(directory, artifact, ignore=shutil.ignore_patterns('report.json', '*.class', '*.kotlin_module'))
    old_prefix, new_prefix = relative(directory) + '/', relative(artifact) + '/'

    def archived(value, key):
        if key == 'argv':
            return value
        if isinstance(value, str) and value.startswith(old_prefix):
            return new_prefix + value[len(old_prefix):]
        if isinstance(value, list):
            return [archived(item, key) for item in value]
        if isinstance(value, dict):
            return {item_key: archived(item, item_key) for item_key, item in value.items()}
        return value

    for path in artifact.rglob('*.source-map.json'):
        path.write_text(json.dumps(archived(json.loads(path.read_text()), ''), indent=2) + '\n')
    archived_report = archived(report, '')
    archived_report['archived_from'] = relative(directory)
    archived_report['reproduction_command'] = ['python3', 'tools/upstream_jvm.py', '--candidate',
        report.get('compiler', {}).get('path', 'target/release/korrust'), '--output',
        'target/upstream-fresh-run']
    destination.write_text(json.dumps(archived_report, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--matrix', type=Path, default=ROOT / 'tools/upstream_feature_matrix.json')
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--select', action='append', default=[])
    parser.add_argument('--inventory-only', action='store_true')
    parser.add_argument('--publish', type=Path)
    args = parser.parse_args()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    matrix = json.loads(args.matrix.read_text())
    entries = [entry for entry in matrix['cases'] if not args.select or entry['id'] in args.select or entry['family'] in args.select]
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(),
              'status': 'running', 'selection': matrix['selection'], 'matrix_sha256': sha(args.matrix),
              'limitations': matrix['limitations'], 'cases': [], 'commands': []}
    environment = dict(os.environ)
    java_home = environment.get('JAVA_HOME')
    if not java_home and Path('/usr/libexec/java_home').is_file():
        result = subprocess.run(['/usr/libexec/java_home', '-v', '21'], text=True, capture_output=True)
        if result.returncode == 0:
            java_home = result.stdout.strip()
            environment['JAVA_HOME'] = java_home
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    reference = ROOT / pin['path']
    command_dir = directory / 'commands'
    command_dir.mkdir()
    commands = Commands(command_dir, environment)
    context = {'directory': directory, 'commands': commands, 'candidate': args.candidate.resolve(),
               'official': reference / 'bin/kotlinc', 'stdlib': reference / 'lib/kotlin-stdlib.jar',
               'testlib': reference / 'lib/kotlin-test.jar',
               **{name: str(Path(java_home) / 'bin' / name) if java_home else name for name in ['java', 'javac', 'javap']}}
    report['inputs'] = {'reference_source_manifest_sha256': sha(ROOT / 'references/sources.json'),
                        'compiler_pin_sha256': sha(ROOT / 'references/compiler.json'),
                        'adapter_sha256': sha(Path(__file__)), 'matrix_sha256': sha(args.matrix)}
    if not args.inventory_only:
        report['compiler'] = {'path': relative(context['candidate']), 'sha256': sha(context['candidate'])}
        report['reference_compiler'] = {'version': pin['version'], 'archive_sha256': pin['archive_sha256'],
            'compiler_jar_sha256': sha(reference / 'lib/kotlin-compiler.jar'), 'stdlib_sha256': sha(context['stdlib']), 'kotlin_test_sha256': sha(context['testlib'])}
        java_version = commands.run('JDK identity', [context['java'], '-version'])
        report['environment'] = {'JAVA_HOME': java_home, 'java_version': (java_version.stdout + java_version.stderr).strip() if java_version else None}
        version = commands.run('Official compiler identity', [context['official'], '-version'])
        if version is None or version.returncode or ('kotlinc-jvm ' + pin['version'] + ' ') not in version.stderr + version.stdout:
            raise RuntimeError('Reference compiler version mismatch')
    frozen_paths = {args.matrix.resolve(), Path(__file__).resolve()}
    if not args.inventory_only:
        frozen_paths.update({context['candidate'], reference / 'lib/kotlin-compiler.jar', context['stdlib'], context['testlib']})
    for entry in entries:
        frozen_paths.add((ROOT / entry['path']).resolve())
        sibling = (ROOT / entry['path']).with_suffix('.java')
        if entry['kind'] in {'java_consumer', 'kotlin_consumer'} and sibling.is_file():
            frozen_paths.add(sibling.resolve())
        frozen_paths.update((ROOT / helper).resolve() for helper in entry.get('helpers', []))
    frozen = {str(path): sha(path) for path in sorted(frozen_paths)}
    report['frozen_input_hashes'] = {relative(path): value for path, value in frozen.items()}
    for entry in entries:
        if args.inventory_only:
            fixture = parse_fixture(ROOT / entry['path'])
            if fixture['sha256'] != entry['sha256']:
                raise RuntimeError('Original fixture differs from frozen matrix: ' + entry['path'])
            outcome, reasons = assess(fixture, entry['kind'])
            case = {**entry, 'outcome': 'not_run' if outcome == 'ready' else outcome,
                    'reasons': reasons, 'sha256': fixture['sha256'],
                    'modules': [{key: value for key, value in module.items() if key != 'files'} for module in fixture['modules']]}
        else:
            print(entry['id'], flush=True)
            case = run_case(entry, context)
        report['cases'].append(case)
        report['commands'] = commands.entries
        report['counts'] = dict(sorted(Counter(item['outcome'] for item in report['cases']).items()))
        (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    report['status'] = 'complete' if args.inventory_only or all(case['outcome'] not in {'correctness_failure', 'oracle_failure', 'timeout', 'dependency_failure', 'setup_failure'} for case in report['cases']) else 'failed'
    changed = [relative(path) for path, value in frozen.items() if not Path(path).is_file() or sha(path) != value]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    if changed:
        report['status'] = 'invalidated'
    report['claims'] = 'Selected upstream fixture outcomes only; no compatibility percentage, full-suite, Android, or performance claim'
    (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    if args.publish:
        publish_report(report, directory, args.publish.resolve())
    print(json.dumps(report['counts'], sort_keys=True))


if __name__ == '__main__':
    main()
