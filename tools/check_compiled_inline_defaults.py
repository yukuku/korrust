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
from inline_origins import ATTRIBUTE, parse, verify_inline_origin, verify_owned_class_origins

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/compiled_inline_defaults'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    parser.add_argument('--mockk-classpath', help='Optional complete MockK runtime JAR classpath')
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-compiled-inline-defaults'
    directory = (args.output or ROOT / 'target/checks' / stamp).absolute()
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / stamp).absolute()
    directory.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/compiled_inline_defaults.rs', Path(__file__), ROOT / 'tools/inline_origins.py', ROOT / 'tools/check_androidbible_class_listener.py', ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json']
    dependency_inputs = [Path(path).absolute() for path in args.mockk_classpath.split(os.pathsep)] if args.mockk_classpath else []
    frozen = {relative(path): sha(path) for path in [*inputs, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', *dependency_inputs]}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
        frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')),
        scope='Compiled static Kotlin inline defaults: real dispatcher evaluation order, dependent defaults, lambda substitution, non-local returns through finally, reified defaults, capture-free default lambda regeneration, private JVM inline-only stdlib functions, class property initializers, output cleanup and optional live MockK execution',
        limitations=['Default dispatcher guards are bounded to the Kotlin synthetic mask-prefix form', 'Default lambda regeneration is bounded to public capture-free function singletons; lambda bodies requiring reified specialization remain explicitly unsupported', 'Existing inline body, JVM access and local-slot limits apply; no full MockK suite or application compatibility claim'],
        effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'),
        cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate), '--output', relative(directory), '--evidence-directory', relative(evidence)])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command):
        command = [str(part) for part in command]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        assert result.returncode == 0, label + ': ' + result.stdout + result.stderr
        return result

    def case(name, operation):
        try:
            detail = operation()
            outcome = detail.pop('outcome', 'pass') if isinstance(detail, dict) else 'pass'
            report['cases'].append(dict(id=name, outcome=outcome, detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        save()

    if args.mockk_classpath:
        report['reproduction_command'].extend(['--mockk-classpath', args.mockk_classpath])
    save()
    dependency = directory / 'dependency'
    run('official dependency with metadata 1.7', [official, FIXTURES / 'Library.kt', '-d', dependency, '-Xmetadata-version=1.7.0', *options])
    cp = os.pathsep.join(map(str, [dependency, standard]))
    providers = [('official', directory / 'official'), ('candidate', directory / 'candidate')]
    for name, output in providers:
        def compile_consumer(name=name, output=output):
            run(name + ' consumer', [(official if name == 'official' else candidate), FIXTURES / 'Consumer.kt', '-cp', cp, '-d', output, *(options if name == 'official' else [])])
            return 'Same unmodified source, compiled dependency and standard library'
        case(name + '_compile', compile_consumer)
    def provenance():
        output = providers[1][1]
        records = ownership(output, FIXTURES / 'Consumer.kt')
        origins = verify_owned_class_origins(output, FIXTURES / 'Consumer.kt', [dependency, standard], records)
        assert origins, records
        helper = output / next(iter(origins))
        data = helper.read_bytes()
        info = parse(data)
        marker = next(entry for entry in info['attributes'] if entry['name'] == ATTRIBUTE)
        errors = []

        def rejects(label, operation):
            try:
                operation()
            except (ValueError, IndexError, KeyError, UnicodeError) as error:
                errors.append(dict(id=label, diagnostic=str(error)))
            else:
                raise AssertionError('Invalid origin accepted: ' + label)

        changed = bytearray(data)
        changed[info['methods'][0]['attributes'][0]['payload_start'] + 8] ^= 1
        rejects('changed_method_code', lambda: verify_inline_origin(bytes(changed), [dependency, standard]))
        rejects('missing_original', lambda: verify_inline_origin(data, [standard]))
        changed = bytearray(data)
        changed[marker['end'] - 1] ^= 1
        rejects('forged_origin_digest', lambda: verify_inline_origin(bytes(changed), [dependency, standard]))
        changed = data.replace(b'Library.kt', b'Forged!.kt')
        assert changed != data
        rejects('forged_SourceFile', lambda: verify_inline_origin(changed, [dependency, standard]))
        shadow = directory / 'origin-shadow'
        original_name = next(iter(origins.values()))['origin']
        original = dependency / (original_name + '.class')
        replacement = shadow / (original_name + '.class')
        replacement.parent.mkdir(parents=True, exist_ok=True)
        replacement.write_bytes(original.read_bytes().replace(b'Library.kt', b'Changed.kt'))
        rejects('first_resolved_classpath_shadow', lambda: verify_inline_origin(data, [shadow, dependency, standard]))
        absent = dict(records)
        del absent[next(name for name in absent if name == origins[next(iter(origins))]['caller'] + '.class')]
        rejects('missing_owned_caller', lambda: verify_owned_class_origins(output, FIXTURES / 'Consumer.kt', [dependency, standard], absent))
        target = directory / 'forged-no-origin'
        shutil.copytree(output, target)
        stripped = data[:info['attributes_start']] + (len(info['attributes']) - 1).to_bytes(2, 'big')
        stripped += b''.join(data[entry['start']:entry['end']] for entry in info['attributes'] if entry['name'] != ATTRIBUTE)
        (target / helper.relative_to(output)).write_bytes(stripped)
        rejects('unattributed_foreign_SourceFile', lambda: verify_owned_class_origins(target, FIXTURES / 'Consumer.kt', [dependency, standard], records))
        rejects('changed_output_manifest_hash', lambda: ownership(target, FIXTURES / 'Consumer.kt'))
        return dict(origins=origins, invalid_origins_rejected=errors)
    case('strict_inline_origin_provenance', provenance)
    def runtime():
        caller = directory / 'caller'
        run('fixed Java caller against official', [java / 'javac', '-cp', os.pathsep.join(map(str, [providers[0][1], dependency, standard])), '-d', caller, FIXTURES / 'Calls.java'])
        hashes = {str(path.relative_to(caller)): sha(path) for path in caller.rglob('*.class')}
        for name, output in providers:
            result = run(name + ' independent runtime assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [caller, output, dependency, standard])), 'defaults.consumer.Calls'])
            assert result.stdout == 'inline-defaults-ok\n', result.stdout
        return dict(caller_sha256=hashes, assertions='Dependent defaults evaluated as 1 then 2; explicit argument evaluated before default (order 51); non-local return and finally order 29; reified Class default; regenerated generic identity lambda; unused reified default discarded; stdlib default UTF-8 charset; two mask words including bit 31 with evaluation order 123; regenerated default lambda used by an ordinary class property initializer')
    case('fixed_java_caller_both_providers', runtime)
    def signatures():
        listings = []
        for name, output in providers:
            result = run(name + ' public JVM signatures', [java / 'javap', '-public', '-s', '-classpath', output, 'defaults.consumer.ConsumerKt'])
            listings.append(result.stdout)
        assert listings[0] == listings[1], listings
        return listings[0]
    case('public_jvm_signatures', signatures)
    def unsupported_lambda():
        source = FIXTURES / 'UnsupportedLambda.kt'
        run('official reified default lambda boundary', [official, source, '-cp', cp, '-d', directory / 'official-unsupported-lambda', *options])
        command = [str(part) for part in [candidate, source, '-cp', cp, '-d', directory / 'candidate-unsupported-lambda', '--diagnostic-format', 'json']]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label='candidate reified default lambda boundary', argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        assert result.returncode == 1, result.stdout + result.stderr
        diagnostic = json.loads(result.stderr)
        assert diagnostic['category'] == 'unsupported_language', diagnostic
        return dict(outcome='unsupported_language', diagnostic=diagnostic)
    case('reified_default_lambda_regeneration_boundary', unsupported_lambda)
    def cleanup():
        source = directory / 'Cleanup.kt'
        output = directory / 'cleanup'
        source.write_bytes((FIXTURES / 'CleanupInitial.kt').read_bytes())
        run('candidate initial default lambda ownership', [candidate, source, '-cp', cp, '-d', output])
        before = {str(path.relative_to(output)): sha(path) for path in output.rglob('*.class')}
        assert any('$inlineDefault$' in name for name in before), before
        source.write_bytes((FIXTURES / 'CleanupUpdated.kt').read_bytes())
        run('candidate default lambda removal', [candidate, source, '-cp', cp, '-d', output])
        after = {str(path.relative_to(output)): sha(path) for path in output.rglob('*.class')}
        assert not any('$inlineDefault$' in name for name in after), after
        assert set(after) == {'defaults/cleanup/CleanupKt.class'}, after
        return dict(before=before, after=after)
    case('regenerated_lambda_output_cleanup', cleanup)
    def mockk():
        if not args.mockk_classpath:
            return dict(outcome='not_run', reason='Provide --mockk-classpath with MockK and all runtime JAR dependencies')
        model = FIXTURES / 'MockKSmoke.kt'
        outputs = [('official', directory / 'mockk-official'), ('candidate', directory / 'mockk-candidate')]
        for name, output in outputs:
            run(name + ' live MockK compile', [(official if name == 'official' else candidate), model, '-cp', args.mockk_classpath, '-d', output, *(options if name == 'official' else [])])
        callers = directory / 'mockk-callers'
        run('fixed Java MockK caller against official', [java / 'javac', '-cp', os.pathsep.join([str(outputs[0][1]), args.mockk_classpath]), '-d', callers, FIXTURES / 'MockKCalls.java'])
        caller_hashes = {str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}
        records = ownership(outputs[1][1], model)
        origins = verify_owned_class_origins(outputs[1][1], model, args.mockk_classpath.split(os.pathsep), records)
        assert origins, records
        for name, output in outputs:
            result = run(name + ' live MockK independent assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join([str(callers), str(output), args.mockk_classpath]), 'defaults.mocking.MockKCalls'])
            assert result.stdout == 'mockk-inline-defaults-ok\n', result.stdout
        return dict(inline_origins=origins, caller_sha256=caller_hashes, dependencies={relative(path): sha(path) for path in dependency_inputs}, assertions='Real relaxed mock returns zero before stubbing and 17 after stubbing; real strict mock with all defaults returns 19 after stubbing')
    case('live_mockk_default_dispatch_and_stubbing', mockk)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    if not args.skip_html:
        subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
