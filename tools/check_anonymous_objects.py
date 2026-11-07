#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/anonymous_objects'
UPSTREAM = ROOT / 'references/kotlin/compiler/testData'
UNSUPPORTED = {'ClassSupertype', 'Multiple', 'Property', 'Generic', 'GenericMethod', 'GenericJava', 'ExtraMember', 'Marker', 'NoSupertype', 'Covariant', 'InferredCovariant', 'AbruptInlineInference'}
INVALID = {'Missing', 'Duplicate', 'WrongParameter', 'WrongResult', 'NullResult', 'Uninitialized', 'ValWrite', 'Sealed', 'InferredWrongNumeric'}


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).resolve()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def signatures(output):
    result = []
    active = False
    for line in output.splitlines():
        line = line.strip()
        if line.endswith(';') or line.endswith('{'):
            active = line.startswith(('public ', 'protected ')) and ' access$' not in line
            if active:
                result.append(line)
        elif line.startswith('descriptor:') and active:
            result.append(line)
    return sorted(result)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--source-root', type=Path, default=ROOT)
    args = parser.parse_args()
    stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ') + '-anonymous-objects'
    work = ROOT / 'target/checks' / stamp
    evidence = ROOT / 'reports/correctness' / stamp
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    report_path = evidence.with_suffix('.json')
    candidate = args.compiler.resolve()
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    compiler_jar = ROOT / 'references/kotlinc/lib/kotlin-compiler.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'anonymous_objects']
    upstream = UPSTREAM / 'codegen/box/objects/kt2663_2.kt'
    helpers = UPSTREAM / 'diagnostics/helpers/coroutines/CoroutineHelpers.kt'
    apps = [ROOT / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base' / part for part in ['devotion/DevotionDownloadBackend.kt', 'ac/DevotionActivity.kt']]
    inputs = [*sorted(args.source_root.glob('src/**/*.rs')), args.source_root / 'Cargo.toml', args.source_root / 'Cargo.lock', *sorted(FIXTURES.rglob('*')), ROOT / 'tests/anonymous_objects.rs', Path(__file__), ROOT / 'references/compiler.json', upstream, helpers, *apps]
    inputs = [path for path in inputs if path.is_file()]
    frozen = {relative(path): sha(path) for path in [*inputs, candidate, stdlib, compiler_jar]}
    bundle = evidence / 'frozen-inputs.zip'
    with zipfile.ZipFile(bundle, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler={'path': relative(candidate), 'sha256': sha(candidate)}, reference_compiler={**json.loads((ROOT / 'references/compiler.json').read_text()), 'options': options}, inputs={'files_sha256': frozen}, source_archive={'path': relative(bundle), 'sha256': sha(bundle)}, commands=[], cases=[], evidence=[], reproduction_commands=[['python3', 'tools/check_anonymous_objects.py', '--compiler', relative(candidate), '--source-root', relative(args.source_root)]], limitations=['Single nongeneric nonsealed interface with ordinary override methods, exact parameter/result JVM descriptors, lexical captures, fresh allocation, own and enclosing receivers. Properties, class/multiple/generic supertypes, generic methods, empty marker implementations, annotations and covariant/erased bridges remain explicit unsupported boundaries. Generic inline lambdas with unconditional nonlocal returns still require unsupported Nothing inference; conditional method returns are exercised.', 'Same Java caller compiles once against official provider and runs with -Xverify:all against both. Official and candidate Kotlin consumers compile separately against both providers. Public provider ABI is compared; generated anonymous class names and private capture layouts are not ABI claims.', 'Generated classes omit EnclosingMethod and anonymous-class Kotlin metadata. Reflection equivalence for Class.isAnonymousClass, isLocalClass, enclosingMethod and Kotlin reflection is unsupported and unverified.', 'AndroidBible source inspection motivates scope; no unchanged whole AndroidBible file, full application, APK/device or full Kotlin compatibility claim. Upstream CoroutineHelpers requires generic Continuation, Result value class and property support beyond this scope.'])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, argv, accepted=(0,)):
        argv = list(map(str, argv))
        started = time.monotonic()
        result = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, timeout=240)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(argv) + '\n' + result.stdout + result.stderr)
        report['commands'].append({'label': label, 'argv': argv, 'exit_code': result.returncode, 'wall_time_seconds': time.monotonic() - started, 'log': relative(log)})
        save()
        if result.returncode not in accepted:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def compile_pair(name, sources, dependencies=()):
        outputs = {provider: work / name / provider for provider in ['official', 'candidate']}
        run(name + '/official', [kotlin, *sources, *options, '-cp', cp(*dependencies, stdlib), '-d', outputs['official']])
        run(name + '/candidate', [candidate, *sources, '--module-name', 'anonymous_objects', '--diagnostic-format', 'json', '-cp', cp(*dependencies, stdlib), '-d', outputs['candidate']])
        return outputs

    def exercise(name, sources, driver, main_class, classes, consumer=None, dependencies=()):
        outputs = compile_pair(name, sources, dependencies)
        caller = work / name / 'caller'
        run(name + '/javac-once-official', [jdk / 'javac', '-cp', cp(outputs['official'], *dependencies, stdlib), '-d', caller, driver])
        for provider, output in outputs.items():
            result = run(name + '/java/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, *dependencies, stdlib), main_class])
            assert result.stdout == 'OK\n', result.stdout
            report['cases'].append({'id': name + '/java/' + provider, 'outcome': 'pass', 'assertion': 'Independent Java assertions of effects, identity, captures and dispatch'})
        for class_name in classes:
            abi = {}
            for provider, output in outputs.items():
                abi[provider] = signatures(run(name + '/javap/' + provider + '/' + class_name, [jdk / 'javap', '-s', '-p', '-cp', cp(output, *dependencies, stdlib), class_name]).stdout)
            assert abi['official'] == abi['candidate'], (class_name, abi)
            report['cases'].append({'id': name + '/abi/' + class_name, 'outcome': 'pass', 'signatures': abi['candidate']})
        if consumer:
            launcher = None
            fixed_launcher = None
            for provider, output in outputs.items():
                for frontend in ['official', 'candidate']:
                    target = work / name / (provider + '-' + frontend + '-consumer')
                    command = [kotlin, consumer, *options] if frontend == 'official' else [candidate, consumer, '--module-name', 'consumer']
                    run(name + '/consumer/' + provider + '/' + frontend, [*command, '-cp', cp(output, *dependencies, stdlib), '-d', target])
                    if launcher is None:
                        launcher = work / name / 'consumer-caller'
                        run(name + '/consumer-javac-once-official', [jdk / 'javac', '-cp', cp(target, output, *dependencies, stdlib), '-d', launcher, FIXTURES / 'ConsumerCheck.java'])
                        fixed_launcher = {relative(path): sha(path) for path in launcher.rglob('*.class')}
                    result = run(name + '/consumer-run/' + provider + '/' + frontend, [jdk / 'java', '-Xverify:all', '-cp', cp(launcher, target, output, *dependencies, stdlib), 'ConsumerCheck'])
                    assert result.stdout == 'OK\n'
                    report['cases'].append({'id': name + '/consumer/' + provider + '/' + frontend, 'outcome': 'pass'})
            assert fixed_launcher == {relative(path): sha(path) for path in launcher.rglob('*.class')}
            report['cases'].append({'id': name + '/consumer-fixed-java-unchanged', 'outcome': 'pass', 'classfiles_sha256': fixed_launcher})
        save()

    try:
        exercise('source', [FIXTURES / 'Objects.kt'], FIXTURES / 'ObjectCheck.java', 'ObjectCheck', ['anonymousprobe.Counter', 'anonymousprobe.ObjectsKt', 'anonymousprobe.Owner', 'anonymousprobe.FactoryOwner', 'anonymousprobe.Wide'], FIXTURES / 'Consumer.kt')
        exercise('inline-method-return', [FIXTURES / 'InlineReturns.kt'], FIXTURES / 'InlineReturnCheck.java', 'InlineReturnCheck', ['anonymousinline.Value', 'anonymousinline.InlineReturnsKt'])
        library = work / 'library'
        run('library/official', [kotlin, FIXTURES / 'Library.kt', *options, '-d', library])
        exercise('compiled-interface', [FIXTURES / 'Compiled.kt'], FIXTURES / 'CompiledCheck.java', 'CompiledCheck', ['anonymouscompiled.CompiledKt'], dependencies=(library,))
        exercise('java-interface', [FIXTURES / 'Runnable.kt'], FIXTURES / 'RunnableCheck.java', 'RunnableCheck', ['anonymousjava.RunnableKt'])
        outputs = compile_pair('upstream-kt2663_2', [upstream])
        driver = work / 'UpstreamCheck.java'
        driver.write_text('public class UpstreamCheck { public static void main(String[] args) { if (!Kt2663_2Kt.box().equals("OK")) throw new AssertionError(); System.out.println("OK"); } }\n')
        caller = work / 'upstream-caller'
        run('upstream/javac-once-official', [jdk / 'javac', '-cp', cp(outputs['official'], stdlib), '-d', caller, driver])
        for provider, output in outputs.items():
            result = run('upstream/run/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(caller, output, stdlib), 'UpstreamCheck'])
            assert result.stdout == 'OK\n'
            report['cases'].append({'id': 'upstream/kt2663_2/' + provider, 'outcome': 'pass', 'source': relative(upstream), 'directives': ['TARGET_BACKEND: JVM', 'TARGET_BACKEND: JVM_IR']})
        for path in sorted((FIXTURES / 'boundaries').glob('*.kt')):
            oracle = run('boundary/' + path.stem + '/official', [kotlin, path, *options, '-cp', stdlib, '-d', work / 'boundaries' / path.stem / 'official'], accepted=(0, 1))
            result = run('boundary/' + path.stem + '/candidate', [candidate, path, '--diagnostic-format', 'json', '-cp', stdlib, '-d', work / 'boundaries' / path.stem / 'candidate'], accepted=(0, 1))
            assert result.returncode != 0, path
            diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
            expected = 'unsupported_language' if path.stem in UNSUPPORTED else 'invalid_kotlin'
            assert diagnostics[0]['category'] == expected, (path, diagnostics)
            assert (oracle.returncode == 0) == (path.stem in UNSUPPORTED), (path, oracle.stderr)
            report['cases'].append({'id': 'boundary/' + path.stem, 'outcome': 'unsupported_language' if path.stem in UNSUPPORTED else 'invalid_rejected', 'oracle_exit_code': oracle.returncode, 'candidate_exit_code': result.returncode, 'diagnostic': diagnostics, 'rejection_check': 'pass'})
        report['cases'].append({'id': 'application/DevotionDownloadBackend', 'outcome': 'not_run', 'source': relative(apps[0]), 'reason': 'Request anonymous interface shape assessed; unchanged whole source requires additional application, HTTP and Android dependency support. No compile claim.'})
        report['cases'].append({'id': 'application/DevotionActivity', 'outcome': 'not_run', 'source': relative(apps[1]), 'reason': 'rootListener extends a class and declares mutable properties, explicitly outside the supported interface-only shape.'})
        report['cases'].append({'id': 'upstream/CoroutineHelpers', 'outcome': 'not_run', 'source': relative(helpers), 'reason': 'Anonymous Continuation implementations require parameterized interface, properties and Result value-class semantics.'})
        assert all(sha(Path(path) if Path(path).is_absolute() else ROOT / path) == digest for path, digest in frozen.items()), 'Frozen inputs changed during harness run'
        report['status'] = 'passed'
    except Exception as error:
        report['status'] = 'failed'
        report['cases'].append({'id': 'harness', 'outcome': 'correctness_failure', 'detail': str(error)})
        raise
    finally:
        save()
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, capture_output=True)
        print(relative(report_path), flush=True)


if __name__ == '__main__':
    main()
