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
import zipfile

from check_androidbible_class_listener import ownership
from inline_origins import parse, resolve_class, verify_owned_class_origins

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/nullable_value_coercions'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def public_contract(output):
    entries = []
    declaration = None
    for line in output.splitlines():
        line = line.strip()
        if line.startswith('public ') and '(' in line and line.endswith(';'):
            declaration = line
        elif line.startswith('descriptor:') and declaration:
            entries.append((declaration, line))
            declaration = None
    assert entries, output
    return sorted(entries)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--app-classpath')
    parser.add_argument('--app-friend-path')
    parser.add_argument('--runtime-models', type=Path, required=True)
    parser.add_argument('--runtime-receipt', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--evidence-directory', type=Path, required=True)
    args = parser.parse_args()
    work, evidence = args.output.absolute(), args.evidence_directory.absolute()
    work.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/test/java/yuku/alkitab/base'
    sources = [app / 'verses/VerseRubyTest.kt'] if args.app_classpath else []
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.glob('*')), ROOT / 'tests/nullable_value_coercions.rs', Path(__file__), ROOT / 'references/kotlin/core/reflection.jvm/src/kotlin/reflect/jvm/internal/ReflectKFunction.kt', ROOT / 'references/kotlin/core/reflection.jvm/src/kotlin/reflect/jvm/internal/util.kt', ROOT / 'tools/check_androidbible_class_listener.py', ROOT / 'tools/check_androidbible_survey.py', ROOT / 'tools/inline_origins.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', *sources, args.runtime_models / 'Alkitab.json', args.runtime_receipt]
    def label(path):
        path = Path(path).absolute()
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
    dependencies = [Path(path) for path in args.app_classpath.split(os.pathsep)] if args.app_classpath else []
    runtime_model = json.loads((args.runtime_models / 'Alkitab.json').read_text())['Alkitab/test']
    runtime_receipt = json.loads(args.runtime_receipt.read_text())
    runtime_paths = list(map(Path, dict.fromkeys([*runtime_model['cp'], *runtime_receipt['dependency_classpath'], *runtime_model['outputs'], str(ROOT / 'benchmarks/sources/androidbible/Alkitab/src/test/resources')])))
    dependencies = list(dict.fromkeys([*dependencies, *(path for path in runtime_paths if path.is_file())]))
    def directory_hash(path):
        manifest = [(str(item.relative_to(path)), sha(item)) for item in sorted(path.rglob('*')) if item.is_file()]
        return hashlib.sha256(json.dumps(manifest).encode()).hexdigest()
    frozen_directories = {str(path): directory_hash(path) for path in runtime_paths if path.is_dir()}
    frozen = {label(path): sha(path) for path in [*inputs, *dependencies, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', *([args.baseline] if args.baseline else [])]}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(label(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler=dict(path=label(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), frozen_input_hashes=frozen, input_bundle=dict(path=label(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), scope='Nullable value-class coercions preserve producer logical types and boxing; boxed nullable-underlying default parameters use verified binary dispatcher storage, including constructors and generic controls; original27-test Ruby JUnit regression', limitations=['Original Ruby test bodies execute on the recorded Gradle model/runtime classpath; no full application or general Android compatibility claim', 'Existing inline, suspend, inner-class and metadata support boundaries remain; defaults are limited to 64 logical parameters'], cases=[], commands=[], reproduction_command=['python3', label(__file__), *sum(([flag, str(value)] for flag, value in vars(args).items() if value is not None for flag in ['--' + flag.replace('_', '-')]), [])])
    report['frozen_runtime_directory_hashes'] = frozen_directories
    report_path = evidence.with_suffix('.json')
    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')
    def run(title, argv, accepted=(0,)):
        argv = list(map(str, argv))
        result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(argv) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=title, argv=argv, exit_code=result.returncode, log=label(log)))
        save()
        assert result.returncode in accepted, title + ': ' + result.stdout + result.stderr
        return result
    def case(name, action):
        try:
            detail = action()
            outcome = detail.pop('outcome', 'pass')
            report['cases'].append(dict(id=name, outcome=outcome, detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        save()
    dependency = work / 'dependency'
    run('official binary Kotlin library', [official, FIXTURES / 'Library.kt', '-module-name', 'nullable_values_library', '-d', dependency, *options])
    run('actual physical method and default ABI', [java / 'javap', '-p', '-s', '-classpath', dependency, 'nullablevalues.library.LibraryKt', 'nullablevalues.library.TextHolder', 'nullablevalues.library.NumberHolder', 'nullablevalues.library.MixedHolder'])
    def runtime(packed):
        origin = dependency
        if packed:
            origin = work / 'dependency.jar'
            with zipfile.ZipFile(origin, 'w', zipfile.ZIP_DEFLATED) as archive:
                for path in sorted(dependency.rglob('*')):
                    if path.is_file():
                        archive.write(path, str(path.relative_to(dependency)))
        cp = os.pathsep.join(map(str, [origin, standard]))
        folder = work / ('jar' if packed else 'directory')
        outputs = []
        for name, compiler in [('official', official), ('candidate', candidate)]:
            output = folder / name
            run(name + ' physical method default consumers', [compiler, FIXTURES / 'Consumer.kt', '-cp', cp, '-d', output, *(options if name == 'official' else [])])
            run(name + ' method invocation descriptors', [java / 'javap', '-c', '-p', '-classpath', output, 'nullablevalues.consumer.ConsumerKt'])
            outputs.append(output)
        callers = folder / 'callers'
        run('fixed Java caller compiled against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs[0], origin, standard])), '-d', callers, FIXTURES / 'Calls.java'])
        for name, output in zip(['official', 'candidate'], outputs):
            observed = run(name + ' independent assertions with verifier', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, origin, standard])), 'nullablevalues.consumer.Calls'])
            assert observed.stdout == 'nullable-value-coercions-ok\n', observed.stdout
        signatures = [run(name + ' public descriptors', [java / 'javap', '-public', '-s', '-classpath', output, 'nullablevalues.consumer.ConsumerKt']).stdout for name, output in zip(['official', 'candidate'], outputs)]
        assert public_contract(signatures[0]) == public_contract(signatures[1]), signatures
        return dict(caller_sha256={str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}, assertions='Int/Long/non-null String/nullable String/nullable Int carriers, nullable locals/parameters, field/function/default producers, generic identity and Holder, null sentinels versus wrapped null, boxed default stubs and constructors')
    for packed in [False, True]:
        case('jar_runtime' if packed else 'directory_runtime', lambda packed=packed: runtime(packed))
    cp = os.pathsep.join(map(str, [dependency, standard]))
    for source in ['WrongType']:
        def reject(source=source):
            diagnostic = None
            for name, compiler in [('official', official), ('candidate', candidate)]:
                observed = run(name + ' reject ' + source, [compiler, FIXTURES / (source + '.kt'), '-cp', cp, '-d', work / (name + source), *(options if name == 'official' else ['--diagnostic-format', 'json'])], accepted=(1,))
                if name == 'candidate':
                    diagnostic = json.loads(observed.stderr)
                    assert diagnostic['category'] == 'invalid_kotlin', diagnostic
            return dict(outcome='invalid_rejected', diagnostic=diagnostic)
        case('invalid_' + source, reject)
    def primitive_regression():
        providers = [('official', official), *([('baseline', args.baseline)] if args.baseline else []), ('candidate', candidate)]
        outputs = []
        for name, compiler in providers:
            output = work / ('primitive-' + name)
            run(name + ' primitive nullable producer', [compiler, FIXTURES / 'PrimitiveOnly.kt', '-cp', cp, '-d', output, *(options if name == 'official' else [])])
            outputs.append((name, output))
        callers = work / 'primitive-callers'
        run('fixed primitive caller against official only', [java / 'javac', '--release', '8', '-cp', str(outputs[0][1]) + os.pathsep + cp, '-d', callers, FIXTURES / 'PrimitiveOnlyCalls.java'])
        for name, output in outputs:
            result = run(name + ' primitive nullable runtime', [java / 'java', '-Xverify:all', '-cp', str(callers) + os.pathsep + str(output) + os.pathsep + cp, 'nullablevalues.consumer.PrimitiveOnlyCalls'], accepted=(1,) if name == 'baseline' else (0,))
            if name == 'baseline':
                assert 'ClassCastException' in result.stderr and 'java.lang.Integer' in result.stderr, result.stderr
            else:
                assert result.stdout == 'primitive-boxing-ok\n', result.stdout
        return dict(assertion='Baseline Integer-to-value-class cast reproduces; oracle and candidate preserve the declared producer type', fixed_caller_sha256=sha(callers / 'nullablevalues/consumer/PrimitiveOnlyCalls.class'))
    case('primitive_nullable_boxing_regression', primitive_regression)
    if args.app_classpath:
        def original_ruby():
            source = sources[0]
            cp_paths = args.app_classpath.split(os.pathsep)
            official_output = work / 'ruby-official'
            run('official original Ruby test compilation', [official, source, '-cp', args.app_classpath, '-d', official_output, *options, *(['-Xfriend-paths=' + args.app_friend_path] if args.app_friend_path else [])])
            providers = [('official', official_output)]
            observations = []
            for name, compiler in [*([('baseline', args.baseline)] if args.baseline else []), ('candidate', candidate)]:
                output = work / ('ruby-' + name)
                run(name + ' original Ruby test compilation', [compiler, source, '-cp', args.app_classpath, '-d', output, '--diagnostic-format', 'json', *(['--friend-paths', args.app_friend_path] if args.app_friend_path else [])])
                owned = ownership(output, source)
                origins = verify_owned_class_origins(output, source, cp_paths, owned)
                for entry in owned:
                    if entry.endswith('.class'):
                        try:
                            resolve_class(cp_paths, entry[:-6])
                        except ValueError:
                            continue
                        raise AssertionError('Classpath shadows emitted class: ' + entry)
                observations.append(dict(provider=name, owned_outputs=owned, inline_origins=origins))
                providers.append((name, output))
            binary_name = 'yuku.alkitab.base.verses.VerseRubyTest'
            verifier = work / 'app-verifier'
            verifier.mkdir()
            caller = verifier / 'VerifyApp.java'
            caller.write_text((FIXTURES / 'VerifyApp.java.template').read_text().replace('__APP_CLASS__', binary_name))
            caller_source_hash = sha(caller)
            run('fixed symbolic app caller against official only', [java / 'javac', '--release', '8', '-cp', str(official_output) + os.pathsep + args.app_classpath, '-d', verifier, caller])
            for name, output in providers:
                names = sorted(str(path.relative_to(output).with_suffix('')).replace('/', '.') for path in output.rglob('*.class'))
                verified = run(name + ' every emitted Ruby class verifies without initialization', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join([str(verifier), str(output), args.app_classpath]), 'VerifyApp', *names])
                assert verified.stdout == 'app-classes-verified\n', verified.stdout
            model = json.loads((args.runtime_models / 'Alkitab.json').read_text())['Alkitab/test']
            runtime = json.loads(args.runtime_receipt.read_text())
            runtime_cp = list(dict.fromkeys([str(ROOT / 'benchmarks/sources/androidbible/Alkitab/src/test/resources'), *model['cp'], *runtime['dependency_classpath']]))
            siblings = work / 'siblings.jar'
            written = set()
            owner = binary_name.replace('.', '/')
            with zipfile.ZipFile(siblings, 'w', zipfile.ZIP_DEFLATED) as archive:
                for directory in map(Path, model['outputs']):
                    for path in sorted(directory.rglob('*')):
                        relative = path.relative_to(directory).as_posix()
                        if path.is_file() and relative not in written and relative != owner + '.class' and not relative.startswith(owner + '$'):
                            archive.write(path, relative)
                            written.add(relative)
            runs = []
            for name, output in providers:
                result = run(name + ' original27 Ruby JUnit tests', [java / 'java', '-Xmx384m', '-Xverify:all', '-cp', os.pathsep.join([str(output), str(siblings), *runtime_cp]), 'org.junit.runner.JUnitCore', binary_name], accepted=(1,) if name == 'baseline' else (0,))
                if name == 'baseline':
                    assert 'ClassCastException' in result.stdout and 'java.lang.Integer' in result.stdout and 'FontStyle' in result.stdout and 'Tests run: 27,  Failures: 1' in result.stdout, result.stdout + result.stderr
                    runs.append(dict(provider=name, tests_run=27, tests_failed=1, expected_before_fix_failure=True))
                else:
                    assert 'OK (27 tests)' in result.stdout, result.stdout + result.stderr
                    runs.append(dict(provider=name, tests_passed=27))
            assert sha(caller) == caller_source_hash
            return dict(source=label(source), source_sha256=sha(source), observations=observations, original_junit=runs, fixed_caller_source_sha256=caller_source_hash, fixed_caller_class_sha256=sha(verifier / 'VerifyApp.class'), siblings_sha256=sha(siblings), runtime_classpath=runtime_cp, classpath_shadowing=False)
        case('original_357_junit_regression', original_ruby)
    assert all(sha(path) == digest for path, digest in ((ROOT / key if not Path(key).is_absolute() else Path(key), value) for key, value in frozen.items())), 'Frozen inputs changed'
    assert all(directory_hash(Path(path)) == digest for path, digest in frozen_directories.items()), 'Runtime directories changed'
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    print(json.dumps(dict(report=str(report_path), status=report['status'], counts=report['counts'])))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
