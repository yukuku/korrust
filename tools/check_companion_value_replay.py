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
FIXTURES = ROOT / 'tests/companion_value_replay'


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
    sources = [app / 'verses/VerseRubyTest.kt', app / 'verses/VerseTextFlowTest.kt', app / 'widget/VerseRendererComposeCheckedHighlightTest.kt'] if args.app_classpath else []
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.glob('*')), ROOT / 'tests/companion_value_replay.rs', Path(__file__), ROOT / 'tools/check_androidbible_class_listener.py', ROOT / 'tools/check_androidbible_survey.py', ROOT / 'tools/inline_origins.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', *sources]
    def label(path):
        path = Path(path).absolute()
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
    dependencies = [Path(path) for path in args.app_classpath.split(os.pathsep)] if args.app_classpath else []
    frozen = {label(path): sha(path) for path in [*inputs, *dependencies, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', *([args.baseline] if args.baseline else [])]}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(label(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler=dict(path=label(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()), frozen_input_hashes=frozen, input_bundle=dict(path=label(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')), scope='Companion backing-field metadata replay uses the loaded value-class registry, preserving physical getters, setters, method defaults and ordinary constant/JvmField metadata without relaxing source visibility', limitations=['No Android runtime or full Compose compatibility claim from API stub compilation', 'Existing inline, suspend, inner-class and metadata support boundaries remain; defaults are limited to 64 logical parameters'], cases=[], commands=[], reproduction_command=['python3', label(__file__), *sum(([flag, str(value)] for flag, value in vars(args).items() if value is not None for flag in ['--' + flag.replace('_', '-')]), [])])
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
    run('official binary Kotlin library', [official, FIXTURES / 'Library.kt', '-module-name', 'companion_values_library', '-d', dependency, *options])
    run('actual physical method and default ABI', [java / 'javap', '-p', '-s', '-classpath', dependency, 'companionvalues.library.Palette', 'companionvalues.library.Palette$Companion', 'companionvalues.library.Key$Companion'])
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
            run(name + ' method invocation descriptors', [java / 'javap', '-c', '-p', '-classpath', output, 'companionvalues.consumer.ConsumerKt'])
            outputs.append(output)
        callers = folder / 'callers'
        run('fixed Java caller compiled against official only', [java / 'javac', '-cp', os.pathsep.join(map(str, [outputs[0], origin, standard])), '-d', callers, FIXTURES / 'Calls.java'])
        for name, output in zip(['official', 'candidate'], outputs):
            observed = run(name + ' independent assertions with verifier', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, output, origin, standard])), 'companionvalues.consumer.Calls'])
            assert observed.stdout == 'companion-values-ok\n', observed.stdout
        signatures = [run(name + ' public descriptors', [java / 'javap', '-public', '-s', '-classpath', output, 'companionvalues.consumer.ConsumerKt']).stdout for name, output in zip(['official', 'candidate'], outputs)]
        assert public_contract(signatures[0]) == public_contract(signatures[1]), signatures
        return dict(caller_sha256={str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}, assertions='Primitive/reference/nullable value-class getters, value setter, default method, true JVM fields and constants, value-class-own companion, private property and logically distinct same-carrier setter rejection')
    for packed in [False, True]:
        case('jar_runtime' if packed else 'directory_runtime', lambda packed=packed: runtime(packed))
    cp = os.pathsep.join(map(str, [dependency, standard]))
    for source in ['Private', 'WrongSetter']:
        def reject(source=source):
            diagnostic = None
            for name, compiler in [('official', official), ('candidate', candidate)]:
                observed = run(name + ' reject ' + source, [compiler, FIXTURES / (source + '.kt'), '-cp', cp, '-d', work / (name + source), *(options if name == 'official' else ['--diagnostic-format', 'json'])], accepted=(1,))
                if name == 'candidate':
                    diagnostic = json.loads(observed.stderr)
                    assert diagnostic['category'] == 'invalid_kotlin', diagnostic
            return dict(outcome='invalid_rejected', diagnostic=diagnostic)
        case('invalid_' + source, reject)
    if args.baseline:
        def before():
            result = run('baseline loses physical companion metadata', [args.baseline, FIXTURES / 'Consumer.kt', '-cp', cp, '-d', work / 'baseline', '--diagnostic-format', 'json'], accepted=(1,))
            return dict(outcome='baseline_rejected', diagnostic=json.loads(result.stderr), reason='The pinned official and candidate compile and execute this same fixture in the directory and JAR cases')
        case('baseline_physical_default_boundary', before)
    if args.app_classpath:
        def compose():
            references = []
            signatures = []
            for name, compiler in [('official', official), ('candidate', candidate)]:
                output = work / (name + '-compose')
                run(name + ' actual ordinary Compose method defaults', [compiler, FIXTURES / 'ComposeProbe.kt', '-cp', args.app_classpath, '-d', output, *(options if name == 'official' else [])])
                signatures.append(run(name + ' public value-class ABI', [java / 'javap', '-public', '-s', '-classpath', output, 'companionvalues.compose.ComposeProbeKt']).stdout)
                found = []
                for path in output.rglob('*.class'):
                    info = parse(path.read_bytes())
                    for entry in info['pool']:
                        if not entry or entry[0] != 10:
                            continue
                        owner_index, signature_index = entry[1]
                        owner = info['class_name'](owner_index)
                        _, (method_index, descriptor_index) = info['pool'][signature_index]
                        method, descriptor = info['text'](method_index), info['text'](descriptor_index)
                        if not ((owner.endswith('$Companion') and method.startswith('get')) or method == 'addStyle') or not owner.startswith('androidx/compose/'):
                            continue
                        binary, origin = resolve_class(args.app_classpath.split(os.pathsep), owner)
                        target = parse(binary)
                        assert any(candidate['name'] == method and candidate['descriptor'] == descriptor and candidate['access'] & 1 for candidate in target['methods']), (owner, descriptor, origin)
                        found.append(dict(owner=owner, method=method, descriptor=descriptor, origin=origin, sha256=hashlib.sha256(binary).hexdigest()))
                found = sorted({(entry['owner'], entry['descriptor'], entry['method']): entry for entry in found}.values(), key=lambda entry: (entry['owner'], entry['method'], entry['descriptor']))
                assert len(found) == 3, found
                references.append(found)
            assert references[0] == references[1], references
            assert public_contract(signatures[0]) == public_contract(signatures[1]), signatures
            return dict(physical_member_references=references[-1], assertion='Every selected getter and addStyle reference resolves to an actual public method in the first resolved pinned classpath binary; descriptors and public consumer ABI match the official compiler', limitation='API jars are compilation stubs; no Compose runtime claim')
        case('actual_compose_companion_and_addstyle_descriptors', compose)
        for source in sources:
            def probe(source=source):
                observations = []
                for name, compiler in [*([('baseline', args.baseline)] if args.baseline else []), ('candidate', candidate)]:
                    output = work / (name + source.stem)
                    result = run(name + ' unchanged ' + source.name, [compiler, source, '-cp', args.app_classpath, '-d', output, '--diagnostic-format', 'json', *(['--friend-paths', args.app_friend_path] if args.app_friend_path else [])], accepted=(0, 1))
                    observed = dict(provider=name, exit_code=result.returncode, diagnostic=json.loads(result.stderr) if result.returncode else None)
                    if not result.returncode:
                        owned = ownership(output, source)
                        observed.update(owned_outputs=owned, inline_origins=verify_owned_class_origins(output, source, args.app_classpath.split(os.pathsep), owned))
                        for entry in owned:
                            if entry.endswith('.class'):
                                try:
                                    resolve_class(args.app_classpath.split(os.pathsep), entry[:-6])
                                except ValueError:
                                    continue
                                raise AssertionError('Classpath shadows emitted source class: ' + entry)
                        official_output = work / ('official-' + source.stem)
                        run('official unchanged ' + source.name, [official, source, '-cp', args.app_classpath, '-d', official_output, *options, *(['-Xfriend-paths=' + args.app_friend_path] if args.app_friend_path else [])])
                        main = [entry for entry in owned if entry.endswith('/' + source.stem + '.class')]
                        assert len(main) == 1, main
                        binary_name = main[0][:-6].replace('/', '.')
                        verifier = work / ('verifier-' + source.stem)
                        verifier.mkdir()
                        caller_source = verifier / 'VerifyApp.java'
                        caller_source.write_text((FIXTURES / 'VerifyApp.java.template').read_text().replace('__APP_CLASS__', binary_name))
                        caller_source_hash = sha(caller_source)
                        run('fixed symbolic app caller against official only ' + source.name, [java / 'javac', '--release', '8', '-cp', str(official_output) + os.pathsep + args.app_classpath, '-d', verifier, caller_source])
                        for provider_name, provider in [('official', official_output), ('candidate', output)]:
                            names = sorted(str(path.relative_to(provider).with_suffix('')).replace('/', '.') for path in provider.rglob('*.class'))
                            linked = run(provider_name + ' all app classes JVM verification without initialization ' + source.name, [java / 'java', '-Xverify:all', '-cp', str(verifier) + os.pathsep + str(provider) + os.pathsep + args.app_classpath, 'VerifyApp', *names])
                            assert linked.stdout == 'app-classes-verified\n', linked.stdout
                        assert sha(caller_source) == caller_source_hash
                        observed['jvm_link_verification'] = dict(fixed_java_source_sha256=caller_source_hash, fixed_java_class_sha256=sha(verifier / 'VerifyApp.class'), initializes_classes=False, executes_test_bodies=False, assertion='Identical Java caller compiled against official app class symbol verifies all emitted classes against each output; no classpath source shadows')
                    observations.append(observed)
                return dict(outcome='compiled_unchanged' if not observations[-1]['exit_code'] else 'app_rejected', source=label(source), source_sha256=sha(source), observations=observations)
            case('app_' + source.stem, probe)
    assert all(sha(path) == digest for path, digest in ((ROOT / key if not Path(key).is_absolute() else Path(key), value) for key, value in frozen.items())), 'Frozen inputs changed'
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    print(json.dumps(dict(report=str(report_path), status=report['status'], counts=report['counts'])))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
