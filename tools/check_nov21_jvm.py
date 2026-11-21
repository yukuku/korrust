#!/usr/bin/env python3
"""Frozen differential evidence for canonical JVM exception and monitor annotations."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    parser.add_argument('--supersedes', type=Path)
    args = parser.parse_args()
    root, candidate = args.source_root.resolve(), args.candidate.resolve()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-nov21-jvm'
    work = args.output.resolve() if args.output else root / 'target/checks' / ident
    logs = root / 'reports/correctness' / ident
    work.mkdir(parents=True, exist_ok=False)
    logs.mkdir(parents=True)
    fixtures = root / 'tests/nov21_jvm'
    oracle = root / 'references/kotlinc/bin/kotlinc'
    standard = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
    compiler_jar = root / 'references/kotlinc/lib/kotlin-compiler.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    files = [*root.glob('src/**/*.rs'), *fixtures.glob('*'), root / 'tests/nov21_jvm.rs', root / 'tools/check_nov21_jvm.py', root / 'Cargo.toml', root / 'Cargo.lock', root / 'references/compiler.json', root / 'references/sources.json']
    frozen = {str(path): sha(path) for path in files if path.is_file()}
    frozen.update({str(path): sha(path) for path in [candidate, standard, compiler_jar]})

    def relative(path):
        path = Path(path)
        return str(path.relative_to(root)) if path.is_relative_to(root) else str(path)

    with zipfile.ZipFile(logs / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            if path.is_file():
                archive.write(path, relative(path))
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'path': str(candidate), 'sha256': sha(candidate), 'source_root': str(root)}, reference_compiler=json.loads((root / 'references/compiler.json').read_text()), reference_sources=json.loads((root / 'references/sources.json').read_text()), input_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(), frozen_input_hashes={relative(Path(path)): digest for path, digest in frozen.items()}, environment={'JAVA_HOME': os.environ['JAVA_HOME'], 'jvm_target': '1.8'}, commands=[], cases=[], evidence=[relative(logs / 'source.zip')], reproduction_commands=[['python3', 'tools/check_nov21_jvm.py', '--candidate', str(candidate), '--source-root', str(root)]], limitations=['Canonical Throws for supported top-level, class, object, companion, interface abstract/default/override methods and public primary constructors; source/import aliases and real accessible Throwable subclasses. Supported function/constructor defaults preserve Exceptions on synthetic helpers and overloads. Throws suspend functions and secondary-constructor annotations retain explicit boundaries. Inheriting a Throws-bearing interface default requires an explicit override; generated inherited wrappers otherwise lose the Exceptions contract, so class/object/subinterface inheritance is rejected explicitly.', 'Canonical Synchronized on supported concrete static/instance methods and supported accessor use sites uses JVM ACC_SYNCHRONIZED. Interface/abstract members and invalid placements reject; suspend and value-class members reject as invalid; inline and default-argument combinations retain explicit unsupported boundaries.', 'Java caller is compiled once against official output and reused unchanged with -Xverify:all for both providers. Kotlin consumers compile against each provider separately. Signatures are compared for selected public declarations; existing unrelated interface DefaultImpls compatibility helpers are outside exact class-shape equivalence.', 'No new count for existing Utf8Decoder object Throws support, full Android application compatibility, or performance claim.'])

    if args.supersedes:
        report['supersedes'] = relative(args.supersedes.resolve())
        report['reproduction_commands'][0].extend(['--supersedes', str(args.supersedes)])

    def run(label, argv, accept=(0,)):
        result = subprocess.run(list(map(str, argv)), cwd=root, capture_output=True, text=True, timeout=300)
        index = len(report['commands'])
        command = {'label': label, 'argv': list(map(str, argv)), 'returncode': result.returncode}
        for stream in ['stdout', 'stderr']:
            path = logs / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            command[stream] = relative(path)
        report['commands'].append(command)
        if result.returncode not in accept:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result

    def case(name, outcome='pass', **detail):
        report['cases'].append(dict(id=name, outcome=outcome, **detail))

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def kotlin(label, sources, output, dependencies=(), accept=(0,)):
        return run(label, [oracle, *sources, '-d', output, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'nov21_jvm', '-cp', cp(*dependencies)] if dependencies else [oracle, *sources, '-d', output, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'nov21_jvm'], accept)

    def korrust(label, sources, output, dependencies=(), accept=(0,)):
        return run(label, [candidate, *sources, '-d', output, '--module-name', 'nov21_jvm', '-cp', cp(standard, *dependencies), '--diagnostic-format', 'json'], accept)

    failures = []

    def test(name, task):
        try:
            task()
        except Exception as error:
            failures.append(name)
            case(name, 'fail', error=str(error))

    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Pinned Kotlin identity', [oracle, '-version'])
        support = work / 'support'
        run('Java exception hierarchy and monitor probe', [jdk / 'javac', '-d', support, fixtures / 'Probe.java', fixtures / 'CustomFailure.java'])
        official, emitted = work / 'official', work / 'candidate'
        sources = [fixtures / name for name in ['Functions.kt', 'Aliases.kt', 'Shadow.kt', 'ShadowAlias.kt']]
        kotlin('Official provider', sources, official, [support])
        korrust('Candidate provider', sources, emitted, [support])
        case('provider/source-compilation')
        java = work / 'java'
        run('Compile fixed Java caller once against official provider', [jdk / 'javac', '-cp', cp(official, support, standard), '-d', java, fixtures / 'JavaCaller.java'])
        caller_hash = sha(java / 'JavaCaller.class')
        report['fixed_java_caller_sha256'] = caller_hash
        for name, output in [('official', official), ('candidate', emitted)]:
            def behavior(name=name, output=output):
                result = run(name + ' Java reflection, exception contracts, monitor identity and release', [jdk / 'java', '-Xverify:all', '-cp', cp(java, output, support, standard), 'JavaCaller'])
                assert result.stdout == 'JAVA\n', result.stdout
                assert sha(java / 'JavaCaller.class') == caller_hash
                case('provider/' + name + '/java-contract-monitor-release')
            test('provider/' + name + '/java-contract-monitor-release', behavior)
            for consumer_compiler in ['official', 'candidate']:
                def consumer(name=name, output=output, consumer_compiler=consumer_compiler):
                    consumer_output = work / (consumer_compiler + '-consumer-' + name)
                    if consumer_compiler == 'official':
                        kotlin(consumer_compiler + ' consumer of ' + name, [fixtures / 'Consumer.kt'], consumer_output, [output, support])
                    else:
                        korrust(consumer_compiler + ' consumer of ' + name, [fixtures / 'Consumer.kt'], consumer_output, [output, support])
                    result = run(consumer_compiler + ' consumer run with ' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer_output, output, support, standard), 'novjvm.consumer.ConsumerKt'])
                    assert result.stdout == 'KOTLIN\n', result.stdout
                    case('consumer/' + consumer_compiler + '-against-' + name)
                test('consumer/' + consumer_compiler + '-against-' + name, consumer)

        def attributes():
            owners = ['novjvm.FunctionsKt', 'novjvm.Worker', 'novjvm.Constructed', 'novjvm.Parent', 'novjvm.Child', 'novjvm.Wider', 'novjvm.Contract', 'novjvm.Contract$DefaultImpls', 'novjvm.Implementation', 'novjvm.AbstractTask', 'novjvm.Task', 'novjvm.Host', 'novjvm.Host$ThrowEverythingRunnable', 'novjvm.Host$Companion', 'novjvm.Singleton', 'novjvm.Defaults', 'novjvm.aliases.AliasesKt']
            def signatures(output, name):
                text = run(name + ' signatures and method attributes', [jdk / 'javap', '-p', '-s', '-v', '-classpath', output, *owners]).stdout
                for marker in ['kotlin.jvm.Throws(', 'kotlin.jvm.Synchronized(']:
                    assert marker not in text, marker
                selected = []
                for line in text.splitlines():
                    line = line.strip()
                    if line.startswith(('public ', 'protected ')) and line.endswith(';') and '$default' not in line and ' access$' not in line:
                        selected.append(line)
                return sorted(selected)
            expected, actual = signatures(official, 'official'), signatures(emitted, 'candidate')
            assert expected == actual, '\n'.join(expected) + '\nCANDIDATE\n' + '\n'.join(actual)
            case('provider/public-signatures-exceptions-flags')
        test('provider/public-signatures-exceptions-flags', attributes)

        def aliases_and_shadowing():
            for name, output in [('official', official), ('candidate', emitted)]:
                text = run(name + ' canonical aliases and ordinary shadow annotations', [jdk / 'javap', '-p', '-v', '-classpath', output, 'novjvm.aliases.AliasesKt', 'novjvm.shadow.ShadowKt', 'novjvm.shadow.Holder', 'novjvm.shadowalias.ShadowAliasKt']).stdout
                assert 'common() throws java.io.IOException;' in text
                assert 'sourceAlias() throws java.io.IOException;' in text
                assert 'qualified() throws java.io.IOException;' in text
                assert 'synchronized boolean sourceMonitor();' in text
                assert 'synchronized boolean qualifiedMonitor();' in text
                assert 'novjvm.shadow.Throws(' in text and 'novjvm.shadow.Synchronized' in text
                assert 'synchronized int plain()' not in text and 'plain() throws' not in text
                case('provider/' + name + '/aliases-shadowing')
        test('provider/aliases-shadowing', aliases_and_shadowing)
        rejections = json.loads((fixtures / 'Rejections.json').read_text())
        invalid = [item for item in rejections if item['category'] == 'invalid_kotlin']
        oracle_sources = []
        for item in invalid:
            source = work / ('reject_' + item['id'] + '.kt')
            source.write_text('package rejected.' + item['id'] + '\n' + item['source'])
            oracle_sources.append(source)
        oracle_reject = kotlin('Official invalid-placement/value boundary cohort', oracle_sources, work / 'oracle-reject', accept=(1,))
        for item in invalid:
            assert ('reject_' + item['id'] + '.kt:') in oracle_reject.stderr, item['id'] + '\n' + oracle_reject.stderr
        for item in rejections:
            def rejection(item=item):
                source = work / ('reject_' + item['id'] + '.kt')
                source.write_text(item['source'])
                dependencies = [official if item['provider'] == 'official' else emitted, support] if 'provider' in item else ()
                result = korrust('Candidate boundary ' + item['id'], [source], work / ('rejected-' + item['id']), dependencies, accept=(1,))
                diagnostic = json.loads(result.stderr.splitlines()[0])
                assert diagnostic['category'] == item['category'] and diagnostic['code'] == item['code'], diagnostic
                assert not list((work / ('rejected-' + item['id'])).rglob('*.class'))
                case('boundary/' + item['id'], item['category'], code=diagnostic['code'], oracle='invalid' if item in invalid else 'accepted below')
            test('boundary/' + item['id'], rejection)
        valid = [item for item in rejections if item not in invalid]
        sources = []
        for item in valid:
            path = work / ('accepted_' + item['id'] + '.kt')
            path.write_text('package accepted.' + item['id'] + '\n' + item['source'])
            sources.append(path)
        kotlin('Official valid explicitly unsupported combinations', sources, work / 'oracle-supported-boundaries', [official, support])
        case('boundary/official-valid-unsupported-combinations', verification=[item['id'] for item in valid])
    except Exception as error:
        failures.append('harness')
        case('harness', 'fail', error=str(error))
    finally:
        changed = [relative(Path(path)) for path, digest in frozen.items() if not Path(path).is_file() or sha(path) != digest]
        if changed:
            failures.append('freeze')
            case('freeze', 'fail', changed=changed)
        else:
            case('freeze/source-fixture-candidate-hashes')
        report['status'] = 'failed' if failures else 'passed'
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['failures'] = failures
        path = root / 'reports/correctness' / (ident + '.json')
        path.write_text(json.dumps(report, indent=2) + '\n')
        if not args.skip_html:
            subprocess.run(['python3', root / 'tools/reports.py'], cwd=root, check=True)
        print(path)
        print(json.dumps(report['counts'], sort_keys=True))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
