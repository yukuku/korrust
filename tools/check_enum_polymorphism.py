#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import re
import subprocess
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/enum_polymorphism'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    try:
        return str(Path(path).resolve().relative_to(ROOT))
    except ValueError:
        return str(Path(path).resolve())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', required=True, type=Path)
    args = parser.parse_args()
    directory = args.output.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    (directory / 'commands').mkdir()
    java_home = Path(os.environ['JAVA_HOME'])
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    compiler = ROOT / 'references/kotlinc/lib/kotlin-compiler.jar'
    candidate = args.candidate.resolve()
    frozen_paths = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'tools/upstream_enum_polymorphism.json', candidate, standard, compiler]
    frozen = {relative(path): sha(path) for path in frozen_paths}
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running',
                  compiler=dict(path=relative(candidate), sha256=sha(candidate)), reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
                  frozen_input_hashes=frozen, cases=[], commands=[], reproduction_command=['python3', relative(__file__), '--candidate', relative(candidate), '--output', relative(directory)],
                  scope='Enum entry subclasses, abstract and concrete method dispatch, interface properties, defaults, ordering, JVM ABI and Kotlin metadata consumers',
                  limitations=['No abstract enum property declarations, secondary enum constructors, nested declarations in entries, super property access, or interface delegation', 'No Android or complete Kotlin language compatibility claim'])

    def run(label, command, accepted=(0,)):
        number = len(report['commands'])
        command = [str(part) for part in command]
        result = subprocess.run(command, capture_output=True, text=True, timeout=180, cwd=ROOT)
        log = directory / 'commands' / f'{number:03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=relative(log)))
        if result.returncode not in accepted:
            raise AssertionError(label + ': ' + result.stdout + result.stderr)
        return result

    def case(name, operation):
        try:
            detail = operation()
            report['cases'].append(dict(id=name, outcome=detail.pop('outcome', 'pass') if isinstance(detail, dict) else 'pass', detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')

    official_out, candidate_out = directory / 'official', directory / 'candidate'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    run('official model', [official, FIXTURES / 'Models.kt', '-cp', standard, '-d', official_out, *options])
    run('candidate model', [candidate, FIXTURES / 'Models.kt', '-cp', standard, '-d', candidate_out])
    java_out = directory / 'java'
    run('Java caller against official only', [java_home / 'bin/javac', '-cp', os.pathsep.join(map(str, [official_out, standard, compiler])), '-d', java_out, FIXTURES / 'Calls.java', FIXTURES / 'MetadataCheck.java'])
    callers = {relative(path): sha(path) for path in java_out.rglob('*.class')}

    def java_runtime():
        results = [run(provider + ' verified independent assertions', [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, standard])), 'enums.polymorphism.Calls']).stdout for provider, output in [('official', official_out), ('candidate', candidate_out)]]
        assert results == ['OK\n', 'OK\n'], results
        return 'Identical Java bytecode, compiled against official only, passes independent initialization, dispatch, defaults, enum and reflection assertions under -Xverify:all'
    case('independent_java_runtime', java_runtime)

    for compile_provider, compile_output in [('official', official_out), ('candidate', candidate_out)]:
        def kotlin_consumer(compile_provider=compile_provider, compile_output=compile_output):
            consumer_out = directory / ('consumer-' + compile_provider)
            run('Kotlin consumer against ' + compile_provider, [official, FIXTURES / 'Consumer.kt', '-cp', os.pathsep.join(map(str, [compile_output, standard])), '-d', consumer_out, *options])
            for runtime_provider, runtime_output in [('official', official_out), ('candidate', candidate_out)]:
                result = run('Kotlin consumer ' + compile_provider + ' runs on ' + runtime_provider, [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_out, runtime_output, standard])), 'consumer.ConsumerKt'])
                assert result.stdout == 'OK\n', result.stdout
            return 'Official Kotlin consumer compiled against ' + compile_provider + ' and verified against both outputs'
        case('kotlin_consumer_' + compile_provider, kotlin_consumer)

    owners = ['Task$Companion', 'Trace', 'Task', 'Task$LEFT', 'Task$RIGHT', 'AbstractAction', 'AbstractAction$A', 'AbstractAction$B', 'PerEntry', 'PerEntry$A', 'PerEntry$B', 'DefaultOnly']
    owners = ['enums.polymorphism.' + owner for owner in owners]
    def metadata():
        results = [run(provider + ' decoded Kotlin metadata', [java_home / 'bin/java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [output, java_out, standard, compiler])), 'enums.polymorphism.MetadataCheck', *owners]).stdout for provider, output in [('official', official_out), ('candidate', candidate_out)]]
        assert results[0] == results[1], '\nOFFICIAL\n' + results[0] + '\nCANDIDATE\n' + results[1]
        return 'Decoded class kind/modality/identity, enum entry list, member flags and constructor parameter default flags match pinned metadata'
    case('kotlin_metadata', metadata)

    def abi():
        results = []
        for provider, output in [('official', official_out), ('candidate', candidate_out)]:
            result = run(provider + ' JVM signatures', [java_home / 'bin/javap', '-p', '-s', '-classpath', output, *owners])
            lines = []
            declaration = ''
            for line in result.stdout.splitlines():
                line = line.strip()
                if not line or line.startswith('Compiled from'):
                    continue
                if line.endswith('{'):
                    declaration = line
                lines.append((declaration, line))
            results.append(sorted(lines))
        assert results[0] == results[1], '\nOFFICIAL ONLY\n' + '\n'.join(map(str, sorted(set(results[0]) - set(results[1])))) + '\nCANDIDATE ONLY\n' + '\n'.join(map(str, sorted(set(results[1]) - set(results[0]))))
        return 'All selected enum and entry class JVM fields, constructors and method signatures match javap -p -s'
    case('jvm_signatures', abi)

    invalid = [
        'enum class E { A; abstract fun run(): Int }',
        'enum class E { A {}; abstract fun run(): Int }',
        'enum class E { A { override fun run(): Int = 1 }; fun run(): Int = 0 }',
        'interface I { fun run(): Int }; enum class E : I { A {} }',
        'enum class E { A { override fun missing(): Int = 1 } }',
    ]
    for index, source in enumerate(invalid):
        def rejection(index=index, source=source):
            path = directory / f'Invalid{index}.kt'
            path.write_text(source)
            run('oracle rejects invalid ' + str(index), [official, path, '-cp', standard, '-d', directory / f'invalid-official-{index}', *options], accepted=(1,))
            result = run('candidate rejects invalid ' + str(index), [candidate, path, '-cp', standard, '-d', directory / f'invalid-candidate-{index}', '--diagnostic-format', 'json'], accepted=(1,))
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            assert diagnostics and all(item['category'] == 'invalid_kotlin' for item in diagnostics), diagnostics
            return dict(outcome='invalid_rejected', diagnostics=diagnostics)
        case('invalid_' + str(index), rejection)

    def unsupported_abstract_property():
        path = directory / 'UnsupportedAbstractProperty.kt'
        path.write_text('enum class E { A { override val value: Int = 1 }; abstract val value: Int }')
        run('oracle accepts abstract enum property', [official, path, '-cp', standard, '-d', directory / 'unsupported-official', *options])
        result = run('candidate explicitly rejects abstract enum property', [candidate, path, '-cp', standard, '-d', directory / 'unsupported-candidate', '--diagnostic-format', 'json'], accepted=(1,))
        diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
        assert diagnostics and diagnostics[0]['category'] == 'unsupported_language' and diagnostics[0]['code'] == 'ABSTRACT_CLASS_PROPERTY_UNSUPPORTED', diagnostics
        return dict(outcome='unsupported_language', oracle='valid Kotlin', diagnostics=diagnostics)
    case('unsupported_abstract_property', unsupported_abstract_property)

    assert callers == {relative(path): sha(path) for path in java_out.rglob('*.class')}
    changed = [path for path, value in frozen.items() if sha(ROOT / path) != value]
    report['input_integrity'] = dict(status='changed' if changed else 'unchanged', changed_paths=changed)
    report['java_caller_hashes'] = callers
    report['counts'] = dict(Counter(item['outcome'] for item in report['cases']))
    report['status'] = 'complete' if not changed and all(item['outcome'] in {'pass', 'invalid_rejected', 'unsupported_language'} for item in report['cases']) else 'failed'
    (directory / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(dict(status=report['status'], counts=report['counts'])))
    return int(report['status'] != 'complete')


if __name__ == '__main__':
    raise SystemExit(main())
