#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import shutil

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures/new_families/strings'
UPSTREAM = ROOT / 'references/kotlin/compiler/testData/codegen/box/strings'
BOUNDARIES = [
    ('bare_name_newline_lambda', False, 'fun invokeInt(action: () -> Int): Int = action()\nfun bad(): Int = invokeInt\n{ 3 }'),
    ('ordinary_escape', False, 'fun bad(): String = "\\q"'),
    ('ordinary_newline', False, 'fun bad(): String = "a\nb"'),
    ('raw_unterminated', False, 'fun bad(): String = """a'),
    ('template_unterminated', False, 'fun bad(): String = """${1'),
    ('unresolved_template_name', False, 'fun bad(): String = """${missing}"""'),
    ('newline_multiply', False, 'fun bad(): Int = 1\n* 2'),
    ('parenthesized_missing_operator', False, 'fun bad(): Int = (1\n2)'),
    ('annotated_trailing_lambda', True, 'fun invokeInt(action: () -> Int): Int = action()\nfun boundary(): Int = invokeInt()\n@Suppress("unused") { 3 }'),
    ('multi_dollar', True, 'fun boundary(): String = $$"""$literal $${1 + 2}"""'),
    ('nothing_template', True, 'fun boundary(): String = """${throw IllegalArgumentException("raw")}"""'),
    ('unicode_template_name', True, 'fun boundary(`é`: String): String = """$é"""'),
]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(path):
    return str(Path(path).relative_to(ROOT))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    work = args.output or ROOT / 'target/checks' / (created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-new-strings')
    work = work.resolve()
    work.mkdir(parents=True, exist_ok=False)
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running', cases=[], commands=[], inputs={}, compiler={}, reference_compiler={}, evidence=[], selection='Bounded raw/multiline string templates and Kotlin lexical newline continuation; fixed Java bytecodes compiled only against official output and reused against both providers, independent expected values, JVM verification, public signatures, unchanged pinned raw-string fixtures, invalid Kotlin versus valid unsupported boundaries. No app/module/Android compatibility or performance claim.')
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
    stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    input_compiler = args.compiler.resolve()
    compiler = work / 'korrust-frozen'
    shutil.copy2(input_compiler, compiler)
    frozen = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', *sorted((ROOT / 'src').rglob('*.rs')), ROOT / 'tests/unsupported_syntax_diagnostics.rs', Path(__file__).resolve(), *sorted(FIXTURES.glob('*')), *[UPSTREAM / name for name in ['rawStrings.kt', 'rawStringsWithManyQuotes.kt', 'multilineStringsWithTemplates.kt']]]
    report['inputs']['files_sha256'] = {relative(path): digest(path) for path in frozen}
    report['compiler'] = dict(path=relative(compiler), original_path=str(input_compiler), binary_sha256=digest(compiler))
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'new_strings']
    report['reference_compiler'] = dict(version=pin['version'], options=options, compiler_pin_sha256=digest(ROOT / 'references/compiler.json'), stdlib_sha256=digest(stdlib))

    def run(label, command):
        print(label, flush=True)
        index = len(report['commands'])
        result = subprocess.run(list(map(str, command)), cwd=ROOT, text=True, capture_output=True, timeout=120)
        entry = dict(label=label, argv=list(map(str, command)), exit_code=result.returncode)
        for stream in ['stdout', 'stderr']:
            path = work / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
            report['evidence'].append(relative(path))
        report['commands'].append(entry)
        return result

    def require(result):
        if result.returncode:
            raise AssertionError(result.stderr[-3000:])
        return result

    def case(ident, outcome='pass', **details):
        report['cases'].append(dict(id=ident, outcome=outcome, **details))

    def compile_provider(provider, sources, destination):
        command = [kotlinc, *sources, *options] if provider == 'official' else [compiler, *sources, '--module-name', 'new_strings']
        return run('Compile ' + provider + ' ' + destination.name, [*command, '-cp', stdlib, '-d', destination])

    try:
        identity = require(run('Pinned official compiler identity', [kotlinc, '-version']))
        assert 'kotlinc-jvm ' + pin['version'] + ' ' in identity.stdout + identity.stderr
        require(run('JDK identity', [jdk / 'java', '-version']))
        require(run('Candidate identity', [compiler, '--version']))
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        sources = [FIXTURES / 'RawStrings.kt', FIXTURES / 'RawLineEndings.kt']
        for provider, destination in outputs.items():
            require(compile_provider(provider, sources, destination))
            case('compile/' + provider)
        driver = work / 'driver'
        require(run('Compile Java driver once against official output', [jdk / 'javac', '-cp', os.pathsep.join(map(str, [outputs['official'], stdlib])), '-d', driver, FIXTURES / 'StringsCheck.java']))
        report['inputs']['fixed_java_classes_sha256'] = {str(path.relative_to(driver)): digest(path) for path in driver.rglob('*.class')}
        observations = []
        for provider, destination in outputs.items():
            result = require(run('Run fixed Java driver ' + provider, [jdk / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [driver, destination, stdlib])), 'newfamilies.strings.StringsCheck']))
            observations.append(result.stdout)
            assert result.stdout.strip() == 'checks=43', result.stdout
            case('runtime/' + provider, independent_assertions=43, jvm_verification='all')
        assert observations[0] == observations[1]
        signatures = {}
        for provider, destination in outputs.items():
            signatures[provider] = []
            for class_name in ['newfamilies.strings.RawStringsKt', 'newfamilies.strings.RawLineEndingsKt']:
                abi = require(run('Inspect ' + provider + ' ' + class_name, [jdk / 'javap', '-public', '-s', '-classpath', destination, class_name])).stdout
                signatures[provider].append(abi)
        assert signatures['official'] == signatures['candidate']
        case('public_signatures', scope='Public Java signatures and descriptors; does not establish unrestricted Kotlin metadata compatibility')
        consumers = {}
        for provider, destination in outputs.items():
            consumers[provider] = work / ('consumer-' + provider)
            require(run('Compile official Kotlin metadata consumer ' + provider, [kotlinc, FIXTURES / 'Consumer.kt', *options, '-cp', os.pathsep.join(map(str, [destination, stdlib])), '-d', consumers[provider]]))
        consumer_driver = work / 'consumer-driver'
        require(run('Compile fixed metadata consumer launcher', [jdk / 'javac', '-cp', os.pathsep.join(map(str, [consumers['official'], outputs['official'], stdlib])), '-d', consumer_driver, FIXTURES / 'ConsumerLauncher.java']))
        for provider, destination in outputs.items():
            observation = require(run('Run Kotlin metadata consumer ' + provider, [jdk / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [consumer_driver, consumers[provider], destination, stdlib])), 'ConsumerLauncher']))
            assert observation.stdout.strip() == 'OK'
            case('kotlin_consumer/' + provider, official_compiler=True, fixed_java_launcher=True, jvm_verification='all', scope='Selected public functions, nullable Int/String results and nonnull String parameters/results')
        report['inputs']['fixed_consumer_launcher_classes_sha256'] = {str(path.relative_to(consumer_driver)): digest(path) for path in consumer_driver.rglob('*.class')}

        for name, valid, source in BOUNDARIES:
            path = work / (name + '.kt')
            path.write_text(source + '\n')
            oracle = compile_provider('official', [path], work / ('official-' + name))
            candidate_dir = work / ('candidate-' + name)
            actual = compile_provider('candidate', [path], candidate_dir)
            assert bool(oracle.returncode) != valid, name + ': oracle classification changed'
            assert actual.returncode and not list(candidate_dir.rglob('*.class')), name + ': must reject without classes'
            if valid:
                assert re.search('unsupported|not supported|only.*supported|support only', actual.stderr, re.I), actual.stderr
                case('boundary/' + name, outcome='unsupported_language', official_accepts=True, candidate_rejects=True)
            else:
                case('boundary/' + name, outcome='invalid_kotlin', official_accepts=False, candidate_rejects=True)
        for name in ['rawStrings.kt', 'multilineStringsWithTemplates.kt', 'rawStringsWithManyQuotes.kt']:
            path = UPSTREAM / name
            ident = path.stem
            official = work / ('upstream-official-' + ident)
            candidate = work / ('upstream-candidate-' + ident)
            require(compile_provider('official', [path], official))
            actual = compile_provider('candidate', [path], candidate)
            if actual.returncode:
                outcome = 'unsupported_language' if re.search(r'unsupported|not supported', actual.stderr, re.I) else 'correctness_failure'
                case('upstream/' + ident, outcome=outcome, source=relative(path), source_sha256=digest(path), diagnostic=actual.stderr, directives_preserved=True)
                if outcome == 'correctness_failure':
                    raise AssertionError('Unclassified upstream failure: ' + ident)
                continue
            class_name = ident[0].upper() + ident[1:] + 'Kt'
            java = work / ('Upstream' + ident + '.java')
            java.write_text('public final class Upstream' + ident + ' { public static void main(String[] args) { String value = ' + class_name + '.box(); if (!"OK".equals(value)) throw new AssertionError(value); System.out.println(value); } }\n')
            callers = work / ('upstream-driver-' + ident)
            require(run('Compile fixed upstream caller ' + ident, [jdk / 'javac', '-cp', os.pathsep.join(map(str, [official, stdlib])), '-d', callers, java]))
            for provider, destination in [('official', official), ('candidate', candidate)]:
                observation = require(run('Run upstream ' + ident + ' ' + provider, [jdk / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, destination, stdlib])), 'Upstream' + ident]))
                assert observation.stdout.strip() == 'OK'
            case('upstream/' + ident, source=relative(path), source_sha256=digest(path), unchanged_upstream=True, fixed_java_caller=True, jvm_verification='all', directives_preserved=True)
        app = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/AudioHttpErrorBody.kt'
        app_sha = digest(app)
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        assert inventory[str(app.relative_to(ROOT / 'benchmarks/sources/androidbible'))] == app_sha
        official_app = compile_provider('official', [app], work / 'app-official')
        require(official_app)
        candidate_app = compile_provider('candidate', [app], work / 'app-candidate')
        if not candidate_app.returncode:
            outcome = 'source_compilation_only'
        elif re.search('unsupported|not supported', candidate_app.stderr, re.I):
            outcome = 'unsupported_language'
        else:
            outcome = 'unclassified_compilation_failure'
        case('androidbible/AudioHttpErrorBody', outcome=outcome, source=relative(app), source_sha256=app_sha, official_accepts=True, candidate_diagnostic=candidate_app.stderr, scope='Unchanged source assessment with real pinned stdlib; source compilation alone is not a compatibility pass')
        assert report['inputs']['files_sha256'] == {relative(path): digest(path) for path in frozen}
        assert report['compiler']['binary_sha256'] == digest(compiler)
        case('frozen_inputs')
        report['status'] = 'pass'
    except Exception as error:
        case('harness', 'fail', error=str(error))
        report['status'] = 'fail'
        raise
    finally:
        report['summary'] = dict(Counter(case['outcome'] for case in report['cases']))
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(relative(work / 'report.json'), flush=True)


if __name__ == '__main__':
    main()
