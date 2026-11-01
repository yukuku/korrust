#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import time
import zipfile
from upstream_jvm import parse_fixture, assess, options_for, candidate_options

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/inner_classes'
UPSTREAM = ROOT / 'references/kotlin/compiler/testData/codegen/box'
DECLARED_INNERS = {'innerprobe.Outer$Inner', 'innerprobe.Initialization$Child', 'innerprobe.Shadow$Item', 'innerprobe.PairOuter$Pair', 'innerprobe.ReferenceOuter$Entry', 'innerprobe.GenericArgumentsOuter$Entry'}
APPS = [('Alkitab/src/main/java/yuku/alkitab/base/dialog/ProgressMarkListDialog.kt', 'Inner adapter uses Android RecyclerView and application dependencies plus remaining unsupported UI/source constructs.'), ('Alkitab/src/main/java/yuku/alkitab/songs/SongViewActivity.kt', 'MediaState captures activity state and calls a private outer method; whole file requires remaining Android, lifecycle and application semantics.'), ('Alkitab/src/main/java/yuku/alkitab/base/util/BackForwardListController.kt', 'Inner adapter captures generic outer state and lambda callbacks; generic enclosing classes remain unsupported.')]
ORIGINALS = [('innerNested/innerSimple.kt', True), ('innerNested/innerLabeledThis.kt', True), ('innerNested/kt3132.kt', True), ('innerNested/innerGeneric.kt', False), ('innerNested/innerWithDefaultArgument.kt', False), ('innerNested/innerImplicitParameter.kt', False)]



def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
def cp(*paths): return os.pathsep.join(map(str, paths))
def signatures(output):
    result = []
    active = None
    def flush():
        if active and not active['excluded']: result.extend(active['lines'])
    for line in output.splitlines():
        line = line.strip()
        if line.startswith(('public ', 'protected ')) and ' #' not in line:
            flush()
            active = {'lines': [line] if ';' in line or line.endswith('{') else [], 'header': line, 'excluded': False}
        elif line == '}':
            flush()
            active = None
        elif line.startswith('descriptor:') and active:
            active['lines'].append(line)
        elif line.startswith('flags:') and active:
            active['excluded'] = '(' in active['header'] and ' access$' in active['header'] and 'ACC_STATIC' in line and 'ACC_SYNTHETIC' in line and 'ACC_BRIDGE' not in line
        elif line.startswith('Signature:') and '// ' in line:
            signature = 'Signature: ' + line.split('// ', 1)[1]
            if active: active['lines'].append(signature)
            else: result.append(signature)
    flush()
    return sorted(result)



class Check:
    def __init__(self, args):
        self.args = args; self.compiler = args.compiler.resolve(); now = datetime.now(timezone.utc)
        self.ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-inner-classes'
        self.work = ROOT / 'target/checks' / self.ident; self.logs = ROOT / 'reports/correctness' / self.ident
        self.work.mkdir(parents=True); self.logs.mkdir(parents=True)
        pin = json.loads((ROOT / 'references/compiler.json').read_text()); self.kotlin = ROOT / pin['path'] / 'bin/kotlinc'; self.stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'; self.compiler_jar = ROOT / pin['path'] / 'lib/kotlin-compiler.jar'
        home = Path(os.environ['JAVA_HOME']); self.java, self.javac, self.javap = [home / 'bin' / name for name in ['java', 'javac', 'javap']]
        self.options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'inner_classes']
        frozen = [*args.source_root.glob('src/**/*.rs'), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *FIXTURES.glob('*'), Path(__file__), ROOT / 'tests/test_check_inner_classes.py', ROOT / 'tests/inner_classes.rs', ROOT / 'tools/upstream_jvm.py', ROOT / 'references/compiler.json', self.compiler, self.stdlib, self.compiler_jar, *[UPSTREAM / name for name, _ in ORIGINALS], *[ROOT / 'benchmarks/sources/androidbible' / name for name, _ in APPS]]
        self.frozen = {relative(path): sha(path) for path in sorted(frozen)}
        archive = self.logs / 'frozen-sources.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as output:
            for path in sorted(frozen):
                if path not in (self.compiler, self.stdlib, self.compiler_jar):
                    output.write(path, relative(path))
        self.report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', environment={'platform': platform.platform(), 'java_home': str(home)}, compiler={'binary': relative(self.compiler), 'binary_sha256': sha(self.compiler), 'frozen_source_root': relative(args.source_root)}, reference_compiler={'version': pin['version'], 'archive_sha256': pin['archive_sha256'], 'compiler_jar_sha256': sha(self.compiler_jar), 'options': self.options}, inputs={'files_sha256': self.frozen, 'stdlib_sha256': sha(self.stdlib)}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_inner_classes.py', '--compiler', relative(self.compiler), '--source-root', relative(args.source_root)]], limitations=['One bounded family: named inner classes capturing the enclosing instance. Existing static nested classes, ordinary inheritance, extension methods and lambdas are prerequisites.', 'Nongeneric ordinary outer classes and final inner classes. Captured outer identity, state access, method calls, initialization, qualified construction, source/compiled constructors, Kotlin consumers and JVM metadata are checked. Generic/chained source inner classes, secondary/default constructors, open/abstract source inner classes, enum outers, reserved capture-field collisions and local classes remain explicit unsupported boundaries. Nongeneric bound and unbound inner constructor references use ordinary closure lowering. Invalid static nested declarations within inner classes remain invalid Kotlin.', 'Fixed Java callers compile once against official output and execute against both outputs with full JVM verification. Relevant public ABI and decoded Kotlin metadata are compared. No unchanged whole AndroidBible file, APK/device/full-app, optimization, or complete Kotlin compatibility claim.'])

        self.report['source_archive'] = {'path': relative(archive), 'sha256': sha(archive)}

    def run(self, label, command, accepted=(0,)):
        started = time.monotonic(); result = subprocess.run(list(map(str, command)), cwd=ROOT, capture_output=True, text=True, timeout=300)
        entry = {'label': label, 'argv': list(map(str, command)), 'cwd': '.', 'exit_code': result.returncode, 'wall_time_seconds': time.monotonic() - started}
        for stream in ['stdout', 'stderr']:
            path = self.logs / f'{len(self.report["commands"]):03d}-{stream}.txt'; path.write_text(getattr(result, stream)); entry[stream] = relative(path)
        self.report['commands'].append(entry)
        if result.returncode not in accepted: raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(self, ident, passed, **fields): self.report['cases'].append({'id': ident, 'outcome': 'pass' if passed else 'correctness_failure', **fields})

    def compile(self, label, sources, output, dependencies=()):
        self.run(label + '/official', [self.kotlin, *sources, *self.options, '-cp', cp(*dependencies, self.stdlib), '-d', output['official']])
        self.run(label + '/candidate', [self.compiler, *sources, '--module-name', 'inner_classes', '--diagnostic-format', 'json', '-cp', cp(*dependencies, self.stdlib), '-d', output['candidate']])

    def exercise(self, name, source, driver, consumer=None, dependencies=()):
        directory = self.work / name; outputs = {provider: directory / provider for provider in ['official', 'candidate']}
        self.compile(name, source if isinstance(source, list) else [source], outputs, dependencies)
        callers = directory / 'callers'; consumer_outputs = {}
        if consumer:
            for provider, output in outputs.items():
                for consumer_compiler in ['official', 'candidate']:
                    target = directory / f'{provider}-{consumer_compiler}-consumer'
                    if consumer_compiler == 'official': self.run(f'{name}/consumer/{provider}/official', [self.kotlin, consumer, *self.options, '-cp', cp(output, *dependencies, self.stdlib), '-d', target])
                    else: self.run(f'{name}/consumer/{provider}/candidate', [self.compiler, consumer, '--module-name', 'inner_classes', '--diagnostic-format', 'json', '-cp', cp(output, *dependencies, self.stdlib), '-d', target])
                    consumer_outputs[provider, consumer_compiler] = target
        self.run(name + '/fixed-java', [self.javac, '-cp', cp(outputs['official'], consumer_outputs.get(('official', 'official'), outputs['official']), *dependencies, self.stdlib), '-d', callers, driver])
        fixed_callers = {relative(file): sha(file) for file in callers.rglob('*.class')}
        self.report.setdefault('fixed_java_callers_sha256', {})[name] = fixed_callers
        candidate_callers = directory / 'candidate-java-source'
        self.run(name + '/candidate-java-source', [self.javac, '-cp', cp(outputs['candidate'], consumer_outputs.get(('candidate', 'official'), outputs['candidate']), *dependencies, self.stdlib), '-d', candidate_callers, driver])
        self.case(name + '/candidate_java_source_compiles', True)
        for provider, output in outputs.items():
            for consumer_compiler in (['official', 'candidate'] if consumer else ['none']):
                result = self.run(f'{name}/verified/{provider}/{consumer_compiler}', [self.java, '-Xverify:all', '-cp', cp(callers, output, consumer_outputs.get((provider, consumer_compiler), output), *dependencies, self.stdlib), driver.stem])
                self.case(f'{name}/runtime/{provider}/{consumer_compiler}', result.stdout == '', reused_official_java_bytecodes=True)
        self.case(name + '/fixed_java_callers_unchanged', fixed_callers == {relative(file): sha(file) for file in callers.rglob('*.class')})
        for file in sorted(outputs['official'].rglob('*.class')):
            owner = str(file.relative_to(outputs['official']).with_suffix('')).replace(os.sep, '.')
            if '$' in owner and owner not in DECLARED_INNERS: continue
            expected = self.run(name + '/abi/official/' + owner, [self.javap, '-protected', '-s', '-v', '-classpath', outputs['official'], owner]).stdout
            actual = self.run(name + '/abi/candidate/' + owner, [self.javap, '-protected', '-s', '-v', '-classpath', outputs['candidate'], owner]).stdout
            self.case(name + '/abi/' + owner, signatures(expected) == signatures(actual), expected=signatures(expected), actual=signatures(actual))
            for provider in ['official', 'candidate']: self.run(name + '/raw-all-members/' + provider + '/' + owner, [self.javap, '-p', '-s', '-v', '-classpath', outputs[provider], owner])
        self.report.setdefault('emitted_classes_sha256', {})[name] = {provider: {relative(file): sha(file) for file in output.rglob('*.class')} for provider, output in outputs.items()}
        return outputs

    def boundary(self, item):
        directory = self.work / 'boundaries' / item['id']; directory.mkdir(parents=True); source = directory / 'Boundary.kt'; source.write_text(item['source'])
        expected = self.run('boundary/official/' + item['id'], [self.kotlin, source, *self.options, '-cp', self.stdlib, '-d', directory / 'official'], (0, 1))
        actual = self.run('boundary/candidate/' + item['id'], [self.compiler, source, '--diagnostic-format', 'json', '-cp', self.stdlib, '-d', directory / 'candidate'], (0, 1))
        diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]; category = 'invalid_kotlin' if item['official'] == 'reject' else 'unsupported_language'
        passed = (expected.returncode == 0) == (item['official'] == 'accept') and actual.returncode != 0 and any(error.get('category') == category and error.get('code') == item['code'] for error in diagnostics)
        self.report['cases'].append({'id': 'boundary/' + item['id'], 'outcome': ('invalid_rejected' if item['official'] == 'reject' else 'unsupported_language') if passed else 'correctness_failure', 'official_accepted': expected.returncode == 0, 'diagnostics': diagnostics, 'source_sha256': sha(source)})

    def original(self, name, supported):
        original = UPSTREAM / name; fixture = parse_fixture(original); state, reasons = assess(fixture, 'box')
        if state != 'ready': self.report['cases'].append({'id': 'upstream/' + name, 'outcome': state, 'reasons': reasons}); return
        if len(fixture['modules']) != 1 or len(fixture['modules'][0]['files']) != 1: raise AssertionError('Unexpected upstream module structure: ' + name)
        module = fixture['modules'][0]
        candidate_settings, gaps = candidate_options(module)
        if gaps:
            self.report['cases'].append({'id': 'upstream/' + name, 'outcome': 'harness_unsupported', 'reasons': gaps})
            return
        file = module['files'][0]; directory = self.work / 'upstream' / Path(name).stem; source = directory / file['name']; source.parent.mkdir(parents=True); source.write_text(file['text'])
        self.report.setdefault('upstream_materializations', []).append({'original': relative(original), 'sha256': fixture['sha256'], 'module': {key: value for key, value in module.items() if key != 'files'}, 'materialized': relative(source), 'materialized_sha256': sha(source), 'original_char_offsets': file['original_char_offsets']})
        expected = self.run('upstream/official/' + name, [self.kotlin, source, *options_for(module), '-module-name', module['name'], '-cp', self.stdlib, '-d', directory / 'official'])
        actual = self.run('upstream/candidate/' + name, [self.compiler, source, '--diagnostic-format', 'json', '--module-name', module['name'], *candidate_settings, '-cp', self.stdlib, '-d', directory / 'candidate'], (0, 1))
        if not supported:
            diagnostics = [json.loads(line) for line in actual.stderr.splitlines() if line.startswith('{')]
            valid = actual.returncode != 0 and diagnostics and diagnostics[0].get('category') == 'unsupported_language'
            self.report['cases'].append({'id': 'upstream/' + name, 'outcome': 'unsupported_language' if valid else 'correctness_failure', 'diagnostics': diagnostics, 'original_sha256': fixture['sha256']}); return
        if actual.returncode: self.case('upstream/' + name, False, diagnostics=actual.stderr); return
        facade = Path(file['name']).stem; facade = facade[0].upper() + facade[1:] + 'Kt'; package = re.search(r'^package\s+([\w.]+)', file['text'], re.M); owner = (package.group(1) + '.' if package else '') + facade
        driver = directory / 'OriginalCheck.java'; driver.write_text('public final class OriginalCheck { public static void main(String[] args) { if (!' + owner + '.box().equals("OK")) throw new AssertionError(); } }')
        self.run('upstream/fixed-java/' + name, [self.javac, '-cp', cp(directory / 'official', self.stdlib), '-d', directory / 'callers', driver])
        fixed_callers = {relative(file): sha(file) for file in (directory / 'callers').rglob('*.class')}
        self.report.setdefault('fixed_java_callers_sha256', {})['upstream/' + name] = fixed_callers
        self.report.setdefault('generated_sources_sha256', {})[relative(driver)] = sha(driver)
        for provider in ['official', 'candidate']:
            result = self.run('upstream/verified/' + provider + '/' + name, [self.java, '-Xverify:all', '-cp', cp(directory / 'callers', directory / provider, self.stdlib), 'OriginalCheck'])
            self.case('upstream/' + name + '/' + provider, result.stdout == '', original_sha256=fixture['sha256'], reused_official_java_bytecodes=True)

        self.case('upstream/' + name + '/fixed_java_callers_unchanged', fixed_callers == {relative(file): sha(file) for file in (directory / 'callers').rglob('*.class')})
        for file in sorted((directory / 'official').rglob('*.class')):
            owner = str(file.relative_to(directory / 'official').with_suffix('')).replace(os.sep, '.')
            expected = self.run('upstream/abi/official/' + name + '/' + owner, [self.javap, '-protected', '-s', '-v', '-classpath', directory / 'official', owner]).stdout
            actual = self.run('upstream/abi/candidate/' + name + '/' + owner, [self.javap, '-protected', '-s', '-v', '-classpath', directory / 'candidate', owner]).stdout
            self.case('upstream/' + name + '/abi/' + owner, signatures(expected) == signatures(actual), expected=signatures(expected), actual=signatures(actual))

    def execute(self):
        self.report['abi_policy'] = {'required': 'Public/protected declared methods, constructors, fields, signatures, generic bridges and default helpers are compared. Static synthetic access$ private-access helpers are excluded using method identity and JVM modifiers together; source methods are not excluded by name alone.', 'known_difference': 'Korrust retains its existing package-private synthetic lexical bridge names/visibility. Kotlin uses public final synthetic private-access helpers. Raw all-member javap output is preserved.', 'provider_replacement_scope': 'Each provider supplies the complete related outer and inner class set. Official generated inner classes are not mixed with candidate outer classes. No reflection-exact claim.'}
        self.report['inputs']['androidbible_sources_reviewed'] = [{'path': 'benchmarks/sources/androidbible/' + name, 'sha256': sha(ROOT / 'benchmarks/sources/androidbible' / name), 'whole_file_status': 'not_assessed', 'reason': reason, 'new_unchanged_file_claim': False} for name, reason in APPS]
        self.run('JDK identity', [self.java, '-version'])
        outer = self.exercise('outer', FIXTURES / 'Outer.kt', FIXTURES / 'InnerCheck.java', FIXTURES / 'Consumer.kt')
        self.exercise('batch_import', [FIXTURES / 'Outer.kt', FIXTURES / 'BatchConsumer.kt'], FIXTURES / 'BatchCheck.java')
        dependency = self.work / 'java-parent'
        self.run('Java superclass dependency', [self.javac, '-d', dependency, FIXTURES / 'JavaParent.java'])
        initialization = self.exercise('initialization', FIXTURES / 'Initialization.kt', FIXTURES / 'InitializationCheck.java', dependencies=(dependency,))
        tool = self.work / 'metadata-tool'
        self.run('Metadata decoder', [self.javac, '-cp', cp(self.compiler_jar, self.stdlib), '-d', tool, FIXTURES / 'MetadataDump.java'])
        decoded = {}
        for provider in ['official', 'candidate']:
            result = self.run('Decoded inner metadata/' + provider, [self.java, '-cp', cp(tool, outer[provider], initialization[provider], dependency, self.compiler_jar, self.stdlib), 'MetadataDump', 'innerprobe.Outer$Inner', 'innerprobe.Initialization$Child', 'innerprobe.Shadow$Item', 'innerprobe.PairOuter$Pair', 'innerprobe.ReferenceOuter$Entry', 'innerprobe.GenericArgumentsOuter$Entry'])
            decoded[provider] = sorted(result.stdout.splitlines())
        self.case('metadata/inner_receiver_and_constructor_facts', decoded['official'] == decoded['candidate'], expected=decoded['official'], actual=decoded['candidate'])
        for item in json.loads((FIXTURES / 'Rejections.json').read_text()): self.boundary(item)
        for name, supported in ORIGINALS: self.original(name, supported)

    def finish(self):
        changed = [path for path, digest in self.frozen.items() if sha(ROOT / path) != digest]; self.case('frozen_inputs', not changed and sha(self.compiler) == self.report['compiler']['binary_sha256'], changed=changed)
        counts = dict(Counter(case['outcome'] for case in self.report['cases'])); self.report['counts'] = counts; failed = sum(count for outcome, count in counts.items() if outcome not in ['pass', 'invalid_rejected', 'unsupported_language'])
        self.report['feature_acceptance'] = {'family': 'named_inner_classes_with_enclosing_instance', 'status': 'failed' if failed else 'passed', 'derived_counts': counts}
        self.report['status'] = 'failed' if failed else 'completed'; path = ROOT / 'reports/correctness' / (self.ident + '.json'); path.write_text(json.dumps(self.report, indent=2) + '\n')
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True, capture_output=True, text=True); print(relative(path)); print(json.dumps(counts, sort_keys=True)); return int(bool(failed))


def main():
    parser = argparse.ArgumentParser(); parser.add_argument('--compiler', type=Path, required=True); parser.add_argument('--source-root', type=Path, default=ROOT); args = parser.parse_args(); args.source_root = args.source_root.resolve(); check = Check(args)
    try: check.execute()
    except Exception as error: check.report['cases'].append({'id': 'harness', 'outcome': 'correctness_failure', 'error': str(error)})
    return check.finish()


if __name__ == '__main__': raise SystemExit(main())
