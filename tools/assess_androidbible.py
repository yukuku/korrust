#!/usr/bin/env python3
"""Assess unchanged AndroidBible sources using historical or resolved Gradle inputs.

Examples:
  python3 tools/assess_androidbible.py --mode limited
  python3 tools/assess_androidbible.py --mode resolved --model target/.../build.json

Each file is an isolated first-blocker probe, not a complete-module compilation.
"""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import threading
import zipfile

from classpath_policy import validate_classpath

ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / 'benchmarks/sources/androidbible'
HISTORICAL = ROOT / 'reports/correctness/2026-10-26T073039Z-androidbible-source-assessment.json'
DIAGNOSTIC_CATEGORIES = {'unclassified', 'unsupported_language', 'missing_dependency', 'invalid_kotlin', 'setup', 'io'}
UNSUPPORTED_DIAGNOSTIC_CODES = {'VALUE_CLASS_JVM_BOXING_UNSUPPORTED', 'ENUM_ENTRY_BODY_UNSUPPORTED', 'COMPILER_PLUGIN_UNSUPPORTED', 'ANNOTATION_JVM_LOWERING_UNSUPPORTED'}

LIMITED_POLICY = 'Historical limited classpath: pinned stdlib base archive entries excluding META-INF/versions; original IntArrayList/Bintex Java; test-only Android stubs; candidate SingleChapterVerses and AudioLogEntry. Preserved intentionally for comparison, separate from resolved Gradle inputs.'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def source_group(source):
    parts = Path(source).parts
    if any(part.lower() in ['test', 'tests', 'androidtest', 'testfixtures'] for part in parts):
        return 'tests'
    if 'src' in parts and any(part in ['main', 'debug', 'release'] for part in parts):
        return 'production'
    return 'tools'


def structured_diagnostic(text):
    try:
        value = json.loads(text)
    except (TypeError, ValueError):
        return None
    if not isinstance(value, dict) or not {'source_identity', 'offset', 'category', 'code', 'message'} <= value.keys():
        return None
    if value['source_identity'] is not None and not isinstance(value['source_identity'], str):
        return None
    if type(value['offset']) is not int or value['offset'] < 0 or not isinstance(value['category'], str) or value['category'] not in DIAGNOSTIC_CATEGORIES:
        return None
    if not isinstance(value['code'], str) or not value['code'] or not isinstance(value['message'], str):
        return None
    if 'path' in value and not isinstance(value['path'], str):
        return None
    if any(key in value and (type(value[key]) is not int or value[key] < 1) for key in ['line', 'column']):
        return None
    return {key: value[key] for key in ['source_identity', 'offset', 'category', 'code', 'message', 'path', 'line', 'column'] if key in value}


def candidate_diagnostic(candidate):
    return candidate.get('diagnostic') or structured_diagnostic(candidate.get('stderr', ''))


def diagnostic_category(diagnostic):
    if diagnostic['code'] in UNSUPPORTED_DIAGNOSTIC_CODES:
        return 'unsupported_language'
    return diagnostic['category']


def first_diagnostic(text):
    structured = structured_diagnostic(text)
    if structured is not None:
        return {'path': structured.get('path', structured['source_identity']), 'line': structured.get('line'), 'column': structured.get('column'), 'message': structured['message'], 'source_identity': structured['source_identity'], 'offset': structured['offset'], 'category': structured['category'], 'code': structured['code']}
    lines = text.splitlines()
    for line in lines:
        match = re.search(r'(?:file://)?(.+?\.kt):(\d+):(\d+):\s*(?:error:\s*)?(.*)', line)
        if not match:
            match = re.search(r'(?:file://)?(.+?\.kt):\s*\((\d+),\s*(\d+)\):\s*(.*)', line)
        if match:
            return {'path': match[1].strip(), 'line': int(match[2]), 'column': int(match[3]), 'message': match[4].strip()}
    return {'path': None, 'line': None, 'column': None, 'message': next((line.strip() for line in lines if line.strip()), '')}


def dependency_diagnostic(text):
    lower = text.lower()
    return any(marker in lower for marker in [
        'unresolved reference', 'cannot access class', 'cannot access built-in', 'unresolved import',
        'class not found', 'missing classpath', 'classpath entry points to a non-existent',
        'could not find or load', 'no such file or directory',
        'unresolved library class', 'class is not available', 'not found on the classpath',
        'unresolved type', 'unknown type', 'unsupported dependency', 'unknown imported',
    ])


def classify(candidate, official, setup_error=None):
    if setup_error:
        return 'setup_failure'
    if candidate.get('timed_out') or official.get('timed_out'):
        return 'timeout'
    if candidate.get('launch_error') or official.get('launch_error'):
        return 'setup_failure'
    if candidate.get('exit_code') not in [0, 1]:
        return 'compiler_failure'
    diagnostic = candidate_diagnostic(candidate)
    if diagnostic is None and candidate.get('stderr', '').lstrip().startswith(('{', '[')):
        return 'compiler_failure'
    category = diagnostic_category(diagnostic) if diagnostic is not None else 'unclassified'
    candidate_text = diagnostic['message'] if diagnostic is not None else candidate.get('stderr', '')
    official_text = official.get('stderr', '')
    if official.get('exit_code') not in [0, 1]:
        return 'compiler_failure'
    if candidate.get('exit_code') != 0 and category in ['setup', 'io']:
        return 'setup_failure'
    if official.get('exit_code') != 0:
        if dependency_diagnostic(official_text):
            return 'dependency_failure'
        if candidate.get('exit_code') == 0:
            return 'compiler_failure'
        if re.search(r'(?:error:|^e:)', official_text, re.MULTILINE):
            return 'invalid_kotlin'
        return 'setup_failure'
    if candidate.get('exit_code') == 0:
        return 'emitted' if candidate.get('emitted_classes_sha256') else 'compiler_failure'
    if category == 'unsupported_language':
        return 'unsupported_language'
    if category in ['invalid_kotlin', 'missing_dependency']:
        return 'compiler_failure'
    if any(marker in candidate_text.lower() for marker in ['unsupported', 'not supported', 'only ', 'require ', 'requires ']):
        return 'unsupported_language'
    return 'compiler_failure'


def derived_counts(cases):
    return {'outcomes': dict(Counter(case['outcome'] for case in cases)),
            'groups': {group: dict(Counter(case['outcome'] for case in cases if case['group'] == group)) for group in sorted({case['group'] for case in cases})},
            'emission': dict(Counter(case['checks']['emission']['outcome'] for case in cases)),
            'behavior': dict(Counter(case['checks']['behavior']['outcome'] for case in cases)),
            'diagnostic_categories': dict(Counter(case['candidate']['diagnostic']['category'] for case in cases if case.get('candidate', {}).get('diagnostic'))),
            'diagnostic_codes': dict(Counter(case['candidate']['diagnostic']['code'] for case in cases if case.get('candidate', {}).get('diagnostic')))}


def compare(previous, cases, compiler, classpath_scope):
    old = {case['id']: case for case in previous.get('cases', [])}
    changes = []
    for case in cases:
        prior = old.get(case['id'])
        if prior is None:
            continue
        old_diagnostic = prior.get('first_diagnostic') or first_diagnostic(prior.get('diagnostic', ''))
        if prior['outcome'] != case['outcome'] or old_diagnostic['message'] != case['first_diagnostic']['message'] or any(old_diagnostic.get(key) != case['first_diagnostic'].get(key) for key in ['category', 'code', 'path', 'line', 'column', 'offset']):
            changes.append({'id': case['id'], 'same_source': prior.get('source_sha256') == case['source_sha256'], 'before': {'outcome': prior['outcome'], 'first_diagnostic': old_diagnostic}, 'after': {'outcome': case['outcome'], 'first_diagnostic': case['first_diagnostic']}})
    return {'prior_compiler': previous.get('compiler'), 'current_compiler': compiler,
            'prior_classpath_scope': previous.get('classpath_scope'), 'current_classpath_scope': classpath_scope,
            'scope': 'Historical rejected outcomes were unclassified and had no official compiler run; outcome renaming alone is not a compatibility improvement.', 'changes': changes}


def resolved_task(model, source):
    suffix = str(Path(source).relative_to(SNAPSHOT))
    module = Path(suffix).parts[0]
    modules = model.get('modules', {})
    entry = modules.get(module, model if model.get('module') == module or 'compilation_tasks' in model else {})
    tasks = entry.get('compilation_tasks', {})
    ranked = sorted(tasks.items(), key=lambda pair: ('test' in pair[0].lower(), 'release' not in pair[0].lower(), pair[0]))
    for name, task in ranked:
        if any(str(item).replace('\\', '/').endswith('/' + suffix) or str(item) == suffix for item in task.get('sources', [])):
            return entry, name, task
    raise ValueError('No resolved compilation task contains preserved source: ' + suffix)


def resolved_setup_errors(module, name, task):
    failures = []
    for key in ['source_collection_incomplete', 'classpath_incomplete', 'friend_paths_incomplete']:
        if task.get(key) or module.get(key):
            failures.append({'field': name + '.' + key, 'diagnostic': 'Resolved model marks this input collection incomplete.'})
    is_test = task.get('role') in ['test', 'tests'] or 'test' in name.lower()
    if is_test and 'friend_paths' not in task:
        failures.append({'field': name + '.friend_paths', 'diagnostic': 'Resolved test task has no captured friend paths input; recapture the Gradle model.'})
    if 'friend_paths' in task:
        if not isinstance(task['friend_paths'], list) or any(not isinstance(path, str) or not path for path in task['friend_paths']):
            failures.append({'field': name + '.friend_paths', 'diagnostic': 'Captured friend paths must be an ordered list of nonempty paths.'})
        provenance = task.get('friend_paths_provenance', {})
        if (is_test or task['friend_paths'] or provenance) and (not isinstance(provenance, dict) or provenance.get('captured') is not True):
            failures.append({'field': name + '.friend_paths_provenance', 'diagnostic': 'Friend path inputs were not authoritatively captured from the Gradle task.'})
    failures += task.get('setup_failures') or []
    failures += [entry for entry in module.get('setup_failures', []) if not entry.get('field') or entry['field'].startswith(name + '.')]
    return failures


def official_options(options):
    if isinstance(options, list):
        return options
    result = []
    for key, flag in [('languageVersion', '-language-version'), ('apiVersion', '-api-version'), ('jvmTarget', '-jvm-target'), ('module_name', '-module-name'), ('jvmDefault', '-jvm-default')]:
        value = options.get(key)
        if value:
            value = str(value)
            if key == 'jvmTarget' and value.startswith('JVM_'):
                value = value.removeprefix('JVM_').replace('_', '.')
            elif key in ['languageVersion', 'apiVersion'] and value.startswith('KOTLIN_'):
                value = value.removeprefix('KOTLIN_').replace('_', '.')
            elif key == 'jvmDefault':
                value = value.lower().replace('_', '-')
            result += [flag, value]
    for key, flag in [('javaParameters', '-java-parameters'), ('allWarningsAsErrors', '-Werror'), ('suppressWarnings', '-nowarn'), ('progressiveMode', '-progressive')]:
        if options.get(key) in [True, 'true']:
            result.append(flag)
    for value in options.get('optIn') or []:
        result.append('-opt-in=' + value)
    result += options.get('freeCompilerArgs') or []
    return result


def tree_hashes(path, suffix=None):
    return {child.relative_to(path).as_posix(): sha(child) for child in sorted(Path(path).rglob('*')) if child.is_file() and (suffix is None or child.name.endswith(suffix))}


class Assessment:
    def __init__(self, args):
        self.args = args
        self.now = datetime.now(timezone.utc)
        self.output = args.output or ROOT / 'reports/correctness' / (self.now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-androidbible-' + args.mode + '-assessment.json')
        if self.output.exists():
            raise ValueError('Refusing to overwrite historical evidence: ' + str(self.output))
        self.output.parent.mkdir(parents=True, exist_ok=True)
        self.evidence = self.output.with_suffix('')
        self.evidence.mkdir(parents=True, exist_ok=False)
        self.work = ROOT / 'target/checks' / self.output.stem
        self.work.mkdir(parents=True, exist_ok=False)
        self.commands = []
        self.lock = threading.Lock()
        self.classpath_inputs = {}
        self.validated_classpaths = {}
        self.pin = json.loads((ROOT / 'references/compiler.json').read_text())
        self.kotlinc = ROOT / self.pin['path'] / 'bin/kotlinc'
        self.stdlib = ROOT / self.pin['path'] / 'lib/kotlin-stdlib.jar'
        self.regression = json.loads(args.regression_report.read_text()) if args.regression_report else None
        self.model = json.loads(args.model.read_text()) if args.model else None

    def run(self, label, argv, timeout=None):
        with self.lock:
            index = len(self.commands)
            entry = {'label': label, 'argv': list(map(str, argv)), 'timeout_seconds': timeout or self.args.timeout}
            self.commands.append(entry)
        try:
            process = subprocess.run(entry['argv'], cwd=ROOT, capture_output=True, text=True, timeout=entry['timeout_seconds'])
            result = {'exit_code': process.returncode, 'stdout': process.stdout, 'stderr': process.stderr}
        except subprocess.TimeoutExpired as error:
            result = {'exit_code': None, 'timed_out': True, 'stdout': error.stdout or b'', 'stderr': error.stderr or b''}
        except OSError as error:
            result = {'exit_code': None, 'launch_error': str(error), 'stdout': '', 'stderr': str(error)}
        for stream in ['stdout', 'stderr']:
            value = result[stream]
            if isinstance(value, bytes):
                value = value.decode(errors='replace')
                result[stream] = value
            path = self.evidence / f'{index:05d}-{stream}.txt'
            path.write_text(value)
            entry[stream] = relative(path)
        entry.update({key: value for key, value in result.items() if key not in ['stdout', 'stderr']})
        result['command_index'] = index
        return result

    def require(self, label, argv):
        result = self.run(label, argv)
        if result['exit_code'] != 0:
            raise ValueError(label + ': ' + result['stderr'][-2000:])
        return result

    def limited_setup(self):
        stdlib = self.work / 'historical-stdlib'
        with zipfile.ZipFile(self.stdlib) as archive:
            for entry in archive.infolist():
                path = PurePosixPath(entry.filename)
                if path.is_absolute() or '..' in path.parts or '\\' in entry.filename:
                    raise ValueError('Unsafe stdlib archive path')
                if not entry.filename.startswith('META-INF/versions/'):
                    archive.extract(entry, stdlib)
        java_sources = [SNAPSHOT / 'AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java', *sorted((SNAPSHOT / 'Bintex/src/main/java').rglob('*.java'))]
        stubs = [ROOT / 'tests/fixtures/interop' / path for path in ['android/os/Parcel.java', 'android/os/Parcelable.java', 'androidx/annotation/NonNull.java']]
        java = self.work / 'historical-java'
        self.require('Historical Java and disclosed Android type stubs', [shutil.which('javac') or 'javac', '--release', '8', '-Xlint:-options', '-d', java, *java_sources, *stubs])
        paths = [stdlib, java]
        dependencies = ['AlkitabModel/src/main/java/yuku/alkitab/model/SingleChapterVerses.kt', 'Alkitab/src/main/java/yuku/alkitab/base/audio/AudioLogEntry.kt']
        for name in dependencies:
            output = self.work / ('dependency-' + Path(name).stem)
            self.require('Historical candidate dependency ' + name, [self.args.compiler, SNAPSHOT / name, '-cp', os.pathsep.join(map(str, paths)), '-d', output])
            paths.append(output)
        self.limited = paths
        return {'java_sources_sha256': {relative(path): sha(path) for path in java_sources}, 'test_only_stubs_sha256': {relative(path): sha(path) for path in stubs}, 'candidate_dependency_outputs_sha256': {relative(path): tree_hashes(path) for path in paths[2:]}, 'policy': LIMITED_POLICY}

    def inputs(self, source):
        if self.args.mode == 'limited':
            return self.limited, ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '21'], {'mode': 'limited', 'dependency_provider': 'candidate; same actual dependency bytes supplied to both compilers', 'plugin_classpath': [], 'plugin_options': [], 'friend_paths': [], 'friend_paths_provenance': {'captured': False, 'scope': 'Historical limited assessment has no resolved Gradle friend inputs.'}}
        module, name, task = resolved_task(self.model, source)
        setup_failures = resolved_setup_errors(module, name, task)
        if setup_failures:
            raise ValueError('Incomplete resolved task inputs: ' + json.dumps(setup_failures, sort_keys=True))
        paths = [Path(path) for path in task['compile_classpath']]
        plugins = [Path(path) for path in task.get('compiler_plugin_classpath', [])]
        friends = [Path(path) for path in task.get('friend_paths', [])]
        missing = [str(path) for path in [*paths, *plugins, *friends] if not path.exists()]
        if missing:
            raise ValueError('Missing resolved dependency/plugin/friend inputs: ' + ', '.join(missing))
        policy_key = tuple(str(path.resolve()) for path in paths)
        with self.lock:
            if policy_key not in self.validated_classpaths:
                self.validated_classpaths[policy_key] = validate_classpath(paths)
        if any(',' in str(path) for path in friends):
            raise ValueError('Captured friend path cannot be represented by the pinned Kotlin CLI comma-delimited -Xfriend-paths option.')
        friend_policy_key = tuple(str(path.resolve()) for path in friends)
        with self.lock:
            if friend_policy_key not in self.validated_classpaths:
                self.validated_classpaths[friend_policy_key] = validate_classpath(friends)
        options = official_options(task.get('compiler_options', module.get('options', {})))
        if friends:
            options.append('-Xfriend-paths=' + ','.join(map(str, friends)))
        options += ['-Xplugin=' + str(path) for path in plugins]
        for option in task.get('compiler_plugin_options', []):
            options += ['-P', option]
        return paths, options, {'mode': 'resolved', 'task': name, 'friend_paths': list(map(str, friends)), 'friend_paths_provenance': task.get('friend_paths_provenance', {'captured': False, 'scope': 'No friend inputs captured for this production task; no friend access granted.'}), 'friend_paths_policy': self.validated_classpaths[friend_policy_key], 'classpath_policy': self.validated_classpaths[policy_key], 'resolved_compiler_version': module.get('resolved_kotlin_compiler_version'), 'compiler_options': task.get('compiler_options'), 'plugin_classpath': list(map(str, plugins)), 'plugin_options': task.get('compiler_plugin_options', []), 'source_roots': task.get('source_roots', []), 'generated_source_roots': task.get('generated_source_roots', []), 'candidate_option_gaps': 'Candidate invocation replays captured friend paths with source/classpath/output; JVM target, language/API, plugins and remaining Gradle options are recorded, not silently claimed equivalent.', 'source_dependency_scope': 'Single-file probe; candidate-owned sibling sources and generated sources are not precompiled as dependencies. Unresolved sibling references are dependency gaps, not intrinsic invalidity.'}

    def probe(self, pair):
        index, source = pair
        case = {'id': relative(source), 'module': source.relative_to(SNAPSHOT).parts[0], 'group': source_group(source), 'source_sha256': sha(source), 'checks': {'behavior': {'outcome': 'not_run', 'reason': 'Emission probe does not execute behavior.'}, 'abi': {'outcome': 'not_run'}, 'kotlin_consumer': {'outcome': 'not_run'}}}
        try:
            paths, options, context = self.inputs(source)
            case['input_context'] = context
            with self.lock:
                for path in [*paths, *[Path(path) for path in context['plugin_classpath']], *[Path(path) for path in context['friend_paths']]]:
                    key = str(path.resolve())
                    if key not in self.classpath_inputs:
                        self.classpath_inputs[key] = {'kind': 'file', 'sha256': sha(path)} if path.is_file() else {'kind': 'directory', 'files_sha256': tree_hashes(path)}
            case['classpath_inputs'] = [str(path.resolve()) for path in paths]
            case['input_context']['friend_paths_inputs'] = {str(Path(path).resolve()): self.classpath_inputs[str(Path(path).resolve())] for path in context['friend_paths']}
            friend_options = ['--friend-paths', os.pathsep.join(context['friend_paths'])] if context['friend_paths'] else []
            cp = os.pathsep.join(map(str, paths))
            destination = self.work / 'candidate' / str(index)
            candidate = self.run('Candidate ' + case['id'], [self.args.compiler, source, '--diagnostic-format', 'json', *friend_options, '-cp', cp, '-d', destination])
            candidate['emitted_classes_sha256'] = tree_hashes(destination, '.class')
            candidate['diagnostic'] = structured_diagnostic(candidate['stderr'])
            candidate['diagnostic_format'] = 'structured_json' if candidate['diagnostic'] is not None else ('invalid_structured_json' if candidate['stderr'].lstrip().startswith(('{', '[')) else ('legacy_text' if candidate['stderr'].strip() else 'none'))
            candidate['classification_basis'] = 'structured_category_code' if candidate['diagnostic'] is not None and diagnostic_category(candidate['diagnostic']) != 'unclassified' else 'legacy_or_unclassified_fallback'
            official_output = self.work / 'official' / str(index)
            official = self.run('Pinned official ' + case['id'], [self.kotlinc, source, *options, '-cp', cp, '-d', official_output], self.args.official_timeout)
            official['emitted_classes_sha256'] = tree_hashes(official_output, '.class')
            case['outcome'] = classify(candidate, official)
            case['first_diagnostic'] = first_diagnostic(candidate['stderr'] or official['stderr'])
            case['checks']['emission'] = {'outcome': 'emitted' if candidate.get('exit_code') == 0 and candidate['emitted_classes_sha256'] else 'not_emitted'}
            for provider, value in [('candidate', candidate), ('official', official)]:
                case[provider] = {key: item for key, item in value.items() if key not in ['stdout', 'stderr']}
            if self.regression:
                match = next((item for item in self.regression.get('cases', []) if item['id'] == case['id']), None)
                if match and match.get('source_sha256') == case['source_sha256'] and self.regression.get('compiler', {}).get('binary_sha256') == sha(self.args.compiler):
                    for kind in ['behavior', 'abi', 'kotlin_consumer']:
                        case['checks'][kind] = {**match['checks'][kind], 'report': relative(self.args.regression_report), 'scope': 'Separate required-regression dependency setup; not inferred from this emission probe.'}
        except (OSError, ValueError, KeyError, zipfile.BadZipFile) as error:
            case.update(outcome='setup_failure', first_diagnostic={'path': None, 'line': None, 'column': None, 'message': str(error)})
            case['checks']['emission'] = {'outcome': 'not_run', 'reason': str(error)}
        return case

    def execute(self):
        report = {'schema_version': 1, 'kind': 'corpus', 'created_at': self.now.isoformat(), 'status': 'running', 'selection': 'Isolated first-diagnostic probes of pinned unchanged .kt sources; production/tests/tools distinguished.', 'reason': 'Class emission, behavior, ABI and Kotlin consumers are separate dimensions. A rejected isolated file may need its source dependency closure; no whole-project language or Android runtime coverage claim.', 'compiler': {}, 'reference_compiler': {}, 'inputs': {}, 'commands': self.commands, 'evidence': [relative(Path(__file__))], 'cases': []}
        try:
            optional_inputs = [path for path in [self.args.model, self.args.compare, self.args.regression_report] if path is not None]
            frozen = {relative(path): sha(path) for path in [*optional_inputs, self.args.compiler, Path(__file__), ROOT / 'tools/classpath_policy.py', ROOT / 'tests/androidbible_regression.json', ROOT / 'benchmarks/source-files.sha256.json', *sorted((ROOT / 'src').rglob('*.rs')), *sorted((ROOT / self.pin['path'] / 'lib').glob('*.jar'))]}
            report['compiler'] = {'binary_sha256': sha(self.args.compiler), 'version': self.require('Candidate identity', [self.args.compiler, '--version'])['stdout'].strip(), 'frozen_inputs_sha256': frozen}
            identity = self.require('Pinned official identity', [self.kotlinc, '-version'])
            if 'kotlinc-jvm ' + self.pin['version'] + ' ' not in identity['stdout'] + identity['stderr']:
                raise ValueError('Pinned official compiler identity mismatch')
            report['reference_compiler'] = {'version': self.pin['version'], 'path': relative(self.kotlinc), 'options_scope': 'Recorded per case; matching dependency bytes, candidate unsupported options disclosed.'}
            if self.args.mode == 'limited':
                report['inputs']['limited_setup'] = self.limited_setup()
            else:
                report['inputs'].update(resolved_model=relative(self.args.model), resolved_model_sha256=sha(self.args.model))
                report['evidence'].append(relative(self.args.model))
            scope = LIMITED_POLICY if self.args.mode == 'limited' else 'Resolved Gradle compilation-task classpath/options/plugin inputs; single-file probes with source-closure gaps disclosed.'
            report['classpath_scope'] = scope
            inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
            sources = sorted(SNAPSHOT.rglob('*.kt'))
            for source in sources:
                if sha(source) != inventory[source.relative_to(SNAPSHOT).as_posix()]:
                    raise ValueError('Pinned source changed: ' + relative(source))
            if self.args.source:
                selected = {str(Path(path)) for path in self.args.source}
                sources = [source for source in sources if relative(source) in selected or source.relative_to(SNAPSHOT).as_posix() in selected]
                if len(sources) != len(selected):
                    raise ValueError('--source entries must name distinct exact pinned paths')
            report['inputs']['candidate_diagnostic_protocol'] = {'requested_format': 'json', 'offset_unit': 'UTF-8 bytes', 'fallback': 'Legacy text or unclassified JSON uses the explicitly retained heuristic; official invalid/dependency outcomes take precedence over candidate language classification.'}
            report['inputs'].update(source_inventory_sha256=sha(ROOT / 'benchmarks/source-files.sha256.json'), androidbible_commit=json.loads((ROOT / 'benchmarks/sources.json').read_text())['sources']['androidbible']['commit'], assessment_mode=self.args.mode)
            with ThreadPoolExecutor(max_workers=self.args.jobs) as pool:
                report['cases'] = list(pool.map(self.probe, enumerate(sources)))
            previous = json.loads(self.args.compare.read_text())
            report['comparison'] = {'report': relative(self.args.compare), **compare(previous, report['cases'], report['compiler'], scope)}
            report['evidence'].append(relative(self.args.compare))
            if self.args.regression_report:
                report['evidence'].append(relative(self.args.regression_report))
            if any(sha(source) != inventory[source.relative_to(SNAPSHOT).as_posix()] for source in sources):
                raise AssertionError('Pinned assessment source changed during probes')
            for name, recorded in self.classpath_inputs.items():
                path = Path(name)
                current = {'kind': 'file', 'sha256': sha(path)} if path.is_file() else {'kind': 'directory', 'files_sha256': tree_hashes(path)}
                if current != recorded:
                    raise AssertionError('Resolved dependency changed during probes: ' + name)
            if any(not (ROOT / path).is_file() or sha(ROOT / path) != value for path, value in frozen.items()):
                raise AssertionError('Frozen compiler, reference, harness or source input changed')
            report['status'] = 'completed'
        except Exception as error:
            report['status'] = 'failed'
            report['failure'] = str(error)
            print(str(error), file=sys.stderr)
        report['inputs']['classpath_inputs'] = self.classpath_inputs
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['derived_counts'] = derived_counts(report['cases'])
        report['evidence'] += [command[stream] for command in self.commands for stream in ['stdout', 'stderr']]
        report['finished_at'] = datetime.now(timezone.utc).isoformat()
        self.output.write_text(json.dumps(report, indent=2) + '\n')
        self.output.with_suffix('.md').write_text('# AndroidBible ' + self.args.mode + ' source assessment\n\n' + report['reason'] + '\n\nPer-case commands, dependency inputs, first diagnostic locations and hashes: [' + self.output.name + '](' + self.output.name + ').\n\n' + '\n'.join('- ' + name + ': ' + str(count) for name, count in sorted(report['counts'].items())) + '\n')
        print('Report: ' + relative(self.output), flush=True)
        fatal = any(case['outcome'] in ['compiler_failure', 'timeout'] for case in report['cases'])
        return 0 if report['status'] == 'completed' and not fatal else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--mode', choices=['limited', 'resolved'], required=True)
    parser.add_argument('--model', type=Path)
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--compare', type=Path, default=HISTORICAL)
    parser.add_argument('--regression-report', type=Path)
    parser.add_argument('--source', action='append', help='Exact snapshot-relative or repository-relative source path; repeat to select a subset')
    parser.add_argument('--jobs', type=int, default=4)
    parser.add_argument('--timeout', type=float, default=30)
    parser.add_argument('--official-timeout', type=float, default=120)
    args = parser.parse_args()
    if args.mode == 'resolved' and not args.model:
        parser.error('--mode resolved requires --model with captured Gradle compilation_tasks')
    if args.jobs < 1 or args.timeout <= 0 or args.official_timeout <= 0:
        parser.error('jobs and timeouts must be positive')
    return Assessment(args).execute()


if __name__ == '__main__':
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print('HTML report: ' + str(generate(ROOT)), flush=True)
