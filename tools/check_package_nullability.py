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
from inline_origins import verify_owned_class_origins

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/package_nullability'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def label(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--app-classpath')
    parser.add_argument('--app-friend-path')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-java-package-defaults'
    work = (args.output or ROOT / 'target/checks' / ident).absolute()
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / ident).absolute()
    work.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src'
    app_sources = [app / 'main/java/yuku/alkitab/base/audio/BibleAudioService.kt', app / 'main/java/yuku/alkitab/songs/SongAudioService.kt', app / 'test/java/yuku/alkitab/base/util/DownloadMapperTest.kt'] if args.app_classpath else []
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(path for path in FIXTURES.rglob('*') if path.is_file()), ROOT / 'tests/package_nullability.rs', Path(__file__), ROOT / 'tools/check_androidbible_class_listener.py', ROOT / 'tools/check_androidbible_survey.py', ROOT / 'tools/inline_origins.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', *app_sources]
    dependencies = [Path(path) for path in args.app_classpath.split(os.pathsep)] if args.app_classpath else []
    frozen = {label(path): sha(path) for path in [*inputs, *dependencies, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', *([args.baseline] if args.baseline else [])]}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(label(path), path.read_bytes())
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
        compiler=dict(path=label(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), frozen_input_hashes=frozen,
        input_bundle=dict(path=label(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')),
        scope='Java package qualifier defaults resolved by JSR305 meta-annotations and strict and warning migration contracts; arbitrary qualifier names and nicknames; return, parameter, field and nested type-use enhancement; explicit nullability precedence; non-null Java values checked in required Kotlin contexts; preserved nullable/Elvis/safe/discarded/Java receiver and array evaluation semantics; exact ordered package metadata lookup',
        limitations=['Declaration default overrides, definitely non-null type parameters and JSpecify default contexts remain explicit unsupported boundaries; warning diagnostics are not emitted', 'Existing nested explicit type annotation, JVM and source language boundaries apply', 'The receiver-temporary regression is checked independently using explicit Java annotations in a package without defaults', 'Unchanged app probes record compilation or exact later rejection only; no Android runtime or full app compatibility claim'],
        effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'), cases=[], commands=[],
        reproduction_command=['python3', label(__file__), '--candidate', label(args.candidate)])
    for flag, value in [('--baseline', args.baseline), ('--app-classpath', args.app_classpath), ('--app-friend-path', args.app_friend_path)]:
        if value:
            report['reproduction_command'].extend([flag, str(value)])

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
            outcome = detail.pop('outcome', 'pass') if isinstance(detail, dict) else 'pass'
            report['cases'].append(dict(id=name, outcome=outcome, detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        save()

    dependency = work / 'dependency'
    run('Java binary dependency', [java / 'javac', '--release', '8', '-d', dependency, *sorted((FIXTURES / 'java').rglob('*.java'))])
    run('Kotlin metadata overrides Java package defaults', [official, FIXTURES / 'KotlinLibrary.kt', '-cp', dependency, '-module-name', 'default_symbols', '-d', dependency, *options])

    def interop(packed):
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
            run(name + ' supported defaults', [compiler, FIXTURES / 'Consumer.kt', FIXTURES / 'Broken.kt', FIXTURES / 'Warnings.kt', '-cp', cp, '-d', output, *(options if name == 'official' else [])])
            outputs.append(output)
        callers = folder / 'callers'
        run('fixed Java assertions compiled against official', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs[0], origin, standard])), '-d', callers, FIXTURES / 'Calls.java'])
        for name, output in zip(['official', 'candidate'], outputs):
            result = run(name + ' independent JVM assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, origin, standard])), 'defaults.consumer.Calls'])
            assert result.stdout == 'package-defaults-ok\n', result.stdout
        signatures = []
        for name, output in zip(['official', 'candidate'], outputs):
            listing = run(name + ' public JVM descriptors', [java / 'javap', '-public', '-s', '-classpath', output, 'defaults.consumer.ConsumerKt', 'defaults.consumer.BrokenKt', 'defaults.consumer.WarningsKt'])
            signatures.append(listing.stdout)
        assert signatures[0] == signatures[1], signatures
        return dict(caller_sha256={str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}, assertions='Strict and nullable defaults, explicit overrides, generic/array elements, qualifier nicknames, required/inferred Java null checks, nullable/Elvis/safe/discarded contexts and Java/array receiver evaluation effects')

    for packed in [False, True]:
        case('directory_oracle_runtime' if not packed else 'jar_oracle_runtime', lambda packed=packed: interop(packed))
    cp = os.pathsep.join(map(str, [dependency, standard]))
    for source in ['NullArgument', 'NullableArgument', 'ParameterDefault', 'NullableResult', 'ExplicitRequired', 'NestedResult', 'NestedArrayResult']:
        def reject(source=source):
            diagnostic = None
            for name, compiler in [('official', official), ('candidate', candidate)]:
                result = run(name + ' reject ' + source, [compiler, FIXTURES / (source + '.kt'), '-cp', cp, '-d', work / (name + source), *(options if name == 'official' else ['--diagnostic-format', 'json'])], accepted=(1,))
                if name == 'candidate':
                    diagnostic = json.loads(result.stderr)
                    assert diagnostic['category'] == 'invalid_kotlin', diagnostic
            return dict(outcome='invalid_rejected', diagnostic=diagnostic)
        case('invalid_' + source, reject)
    for source in ['GenericBoundary', 'ScopeBoundary']:
        def boundary(source=source):
            run('official boundary ' + source, [official, FIXTURES / (source + '.kt'), '-cp', cp, '-d', work / ('official' + source), *options])
            result = run('candidate boundary ' + source, [candidate, FIXTURES / (source + '.kt'), '-cp', cp, '-d', work / ('candidate' + source), '--diagnostic-format', 'json'], accepted=(1,))
            diagnostic = json.loads(result.stderr)
            assert diagnostic['category'] == 'unsupported_language', diagnostic
            return dict(outcome='unsupported_language', diagnostic=diagnostic)
        case('boundary_' + source, boundary)

    def ordered():
        shadow = work / 'shadow'
        package = shadow / 'nonnull/package-info.java'
        package.parent.mkdir(parents=True)
        package.write_text('@qualifiers.Optional\npackage nonnull;\n')
        run('earlier package metadata binary', [java / 'javac', '--release', '8', '-cp', dependency, '-d', shadow, package])
        ordered_cp = os.pathsep.join(map(str, [shadow, dependency, standard]))
        for name, compiler in [('official', official), ('candidate', candidate)]:
            run(name + ' first resolved package metadata', [compiler, FIXTURES / 'ShadowConsumer.kt', '-cp', ordered_cp, '-d', work / ('shadow-' + name), *(options if name == 'official' else [])])
        return 'Earlier package-info permits null even though the class binary comes from a later entry with strict non-null package metadata'
    case('first_package_metadata_in_classpath_order', ordered)
    def mapped_receiver_finding():
        outputs = []
        for name, compiler in [('official', official), *([('baseline', args.baseline)] if args.baseline else []), ('candidate', candidate)]:
            output = work / ('mapped-' + name)
            run(name + ' separate pre-existing mapped receiver probe', [compiler, FIXTURES / 'MappedReceiver.kt', '-cp', cp, '-d', output, *(options if name == 'official' else [])])
            outputs.append((name, output))
        callers = work / 'mapped-callers'
        run('fixed mapped receiver caller against official', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs[0][1], dependency, standard])), '-d', callers, FIXTURES / 'MappedReceiverCalls.java'])
        observed = {}
        for name, output in outputs:
            result = run(name + ' mapped receiver observations', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, dependency, standard])), 'preexisting.MappedReceiverCalls'])
            observed[name] = result.stdout.strip()
        assert observed['official'] == '0' and observed['candidate'] == '0', observed
        if args.baseline:
            assert observed['baseline'] == '1', observed
        return dict(outcome='pass', observed=observed, reason='Both compilers resolve the stdlib extension; required Java receiver checks must run before evaluating arguments, even when the receiver is saved to a synthetic local. The baseline performs the check too late; the candidate preserves oracle evaluation order', caller_sha256={str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')})
    case('java_annotation_receiver_temp_coercion_regression', mapped_receiver_finding)
    if args.app_classpath:
        for source in app_sources:
            def probe(source=source):
                observations = []
                for name, compiler in [*([('baseline', args.baseline)] if args.baseline else []), ('candidate', candidate)]:
                    result = run(name + ' unchanged ' + source.name, [compiler, source, '-cp', args.app_classpath, '-d', work / (name + source.stem), '--diagnostic-format', 'json', *(['--friend-paths', args.app_friend_path] if args.app_friend_path else [])], accepted=(0, 1))
                    observation = dict(provider=name, exit_code=result.returncode, diagnostic=json.loads(result.stderr) if result.returncode else None)
                    if result.returncode == 0:
                        output = work / (name + source.stem)
                        owned = ownership(output, source)
                        origins = verify_owned_class_origins(output, source, args.app_classpath.split(os.pathsep), owned)
                        assert any(path.endswith('.class') for path in owned), owned
                        observation.update(owned_outputs=owned, inline_origins=origins)
                    observations.append(observation)
                return dict(outcome='compiled_unchanged' if observations[-1]['exit_code'] == 0 else 'app_rejected', source=label(source), source_sha256=sha(source), observations=observations)
            case('app_' + source.stem, probe)
    changed = [name for name, before in frozen.items() if sha(ROOT / name) != before]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    print(json.dumps(dict(report=label(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
