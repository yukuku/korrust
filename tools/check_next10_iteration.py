#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    root = args.source_root.resolve()
    fixture = root / 'tests/next10_iteration'
    stamp = datetime.now(timezone.utc)
    name = stamp.strftime('%Y-%m-%dT%H%M%S%fZ') + '-next10-iteration'
    work = root / 'target/checks' / name
    evidence = root / 'reports/correctness' / name
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    logs = evidence / 'commands'
    logs.mkdir()
    candidate = work / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    pin = json.loads((root / 'references/compiler.json').read_text())
    official = root / pin['path'] / 'bin/kotlinc'
    standard = root / pin['path'] / 'lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'iteration']
    def rel(path):
        path = Path(path)
        return str(path.relative_to(root)) if path.is_relative_to(root) else str(path)
    frozen_files = [*sorted((root / 'src').rglob('*.rs')), *sorted(fixture.iterdir()), root / 'tests/next10_iteration.rs', root / 'tests/fixtures/ModuleClassVerification.java', root / 'tests/next_frontend.rs', root / 'tools/check_next10_iteration.py', root / 'Cargo.toml', root / 'Cargo.lock', root / 'references/compiler.json', root / 'references/sources.json']
    frozen = {rel(path): sha(path) for path in frozen_files}
    bundle = evidence / 'inputs.zip'
    with zipfile.ZipFile(bundle, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in frozen_files:
            archive.writestr(rel(path), path.read_bytes())
    report = dict(schema_version=1, kind='correctness', created_at=stamp.isoformat(), status='running', compiler=dict(path=rel(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip()), reference_compiler=dict(pin=pin, options=options, standard_sha256=sha(standard), compiler_sha256=sha(official.parent.parent / 'lib/kotlin-compiler.jar')), inputs=dict(files_sha256=frozen, bundle=rel(bundle), bundle_sha256=sha(bundle)), commands=[], cases=[], evidence=[rel(logs)], reproduction_commands=[['python3', 'tools/check_next10_iteration.py', '--candidate', str(args.candidate.resolve()), '--source-root', str(root)]], builtin_primitive_provenance=[dict(path=name, sha256=sha(root / name)) for name in ['references/kotlin-spec/docs/src/md/kotlin.core/statements.md', 'references/kotlin-spec/docs/src/md/kotlin.core/operators.md', 'references/kotlin/libraries/stdlib/src/kotlin/Primitives.kt', 'references/kotlin/libraries/stdlib/src/kotlin/Char.kt', 'references/kotlin/compiler/ir/backend.jvm/codegen/src/org/jetbrains/kotlin/backend/jvm/intrinsics/RangeUntil.kt']], scope='Ordinary source/compiled member and extension iterator protocols, destructuring and labeled loop control; ..< custom rangeUntil resolution and same-type built-in Int/Long/Char ranges. Java callers compile against official once and run against both with -Xverify:all.', limitations=['Mixed integral, Byte/Short, unsigned and nullable primitive open ranges are unsupported', 'No additional AndroidBible unchanged-file or Android/device compatibility claim', 'Unit/Null iterator element types remain unsupported', 'No generic source/imported operator precedence equivalence claim'])
    path = evidence.with_suffix('.json')
    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        path.write_text(json.dumps(report, indent=2) + '\n')
    def run(label, argv, expected=0):
        argv = list(map(str, argv))
        result = subprocess.run(argv, cwd=root, capture_output=True, text=True, timeout=240)
        log = logs / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(argv) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=argv, exit_code=result.returncode, log=rel(log)))
        save()
        if result.returncode != expected:
            raise AssertionError(label + ': ' + result.stdout + result.stderr)
        return result
    def case(ident, outcome='pass', **details):
        report['cases'].append(dict(id=ident, outcome=outcome, **details))
        save()
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    try:
        version = run('pinned compiler identity', [official, '-version'])
        assert 'kotlinc-jvm ' + pin['version'] + ' ' in version.stdout + version.stderr
        run('JDK identity', [java / 'java', '-version'])
        run('candidate identity', [candidate, '--version'])
        dependency = work / 'dependency'
        run('official custom protocol library', [official, fixture / 'Dependency.kt', *options, '-d', dependency])
        run('independent trace dependency', [java / 'javac', '-d', dependency, fixture / 'Trace.java'])
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, output in outputs.items():
            command = [official, *options] if provider == 'official' else [candidate, '--module-name', 'iteration']
            run('compile ' + provider, [*command, fixture / 'Models.kt', '-cp', cp(dependency, standard), '-d', output])
            case('compile/' + provider)
        callers = work / 'java'
        run('fixed Java callers compiled against official', [java / 'javac', '-cp', cp(outputs['official'], dependency, standard), '-d', callers, fixture / 'Calls.java', root / 'tests/fixtures/ModuleClassVerification.java'])
        caller_hashes = {str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}
        for provider, output in outputs.items():
            result = run('independent Java checks ' + provider, [java / 'java', '-Xverify:all', '-cp', cp(callers, output, dependency, standard), 'iteration.Calls'])
            assert result.stdout == 'iteration-ok\n', result.stdout
            case('runtime/' + provider, scope='Independent min/max/empty and singleton-identity range checks; ordered once-only evaluation; custom/source/compiled extensions; destructuring and labeled continue/break; Long element widths')
            names = [str(path.relative_to(output)).removesuffix('.class').replace('/', '.') for path in output.rglob('*.class')]
            run('verify every class ' + provider, [java / 'java', '-Xverify:all', '-cp', cp(callers, output, dependency, standard), 'ModuleClassVerification', *names])
            case('verification/' + provider, classes=len(names))
        run('official primitive rangeUntil bytecode', [java / 'javap', '-c', '-p', '-classpath', outputs['official'], 'iteration.ModelsKt'])
        primitive_bytecode = run('candidate primitive rangeUntil bytecode', [java / 'javap', '-c', '-p', '-classpath', outputs['candidate'], 'iteration.ModelsKt']).stdout
        assert 'kotlin/ranges/RangesKt.until' in primitive_bytecode
        case('builtin_primitive_runtime_dispatch', scope='Pinned built-in rangeUntil intrinsic maps to exact real stdlib until signatures; imported/source extension names do not replace built-in dispatch')
        signatures = []
        for provider, output in outputs.items():
            listing = run('public JVM signatures ' + provider, [java / 'javap', '-public', '-s', '-classpath', output, 'iteration.ModelsKt']).stdout
            signatures.append(sorted(line.strip() for line in listing.splitlines() if line.strip().startswith(('public static', 'descriptor:'))))
        assert signatures[0] == signatures[1], signatures
        case('public_signatures')
        for provider, output in outputs.items():
            consumer = work / ('consumer-' + provider)
            run('Kotlin consumer compile ' + provider, [official, fixture / 'Consumer.kt', *options, '-cp', cp(output, dependency, standard), '-d', consumer])
            for runtime, runtime_output in outputs.items():
                result = run('Kotlin consumer ' + provider + ' on ' + runtime, [java / 'java', '-Xverify:all', '-cp', cp(consumer, runtime_output, dependency, standard), 'consumer.ConsumerKt'])
                assert result.stdout == 'consumer-ok\n', result.stdout
            case('kotlin_consumer/' + provider)
        invalid = [
            ('iterator_modifier', 'class C { fun iterator():C=this; operator fun hasNext():Boolean=false; operator fun next():Int=0 }; fun f() { for(n in C()) {} }'),
            ('hasNext_modifier', 'class C { operator fun iterator():C=this; fun hasNext():Boolean=false; operator fun next():Int=0 }; fun f() { for(n in C()) {} }'),
            ('next_modifier', 'class C { operator fun iterator():C=this; operator fun hasNext():Boolean=false; fun next():Int=0 }; fun f() { for(n in C()) {} }'),
            ('hasNext_type', 'class C { operator fun iterator():C=this; operator fun hasNext()=1; operator fun next():Int=0 }; fun f() { for(n in C()) {} }'),
            ('nullable_container', 'class C { operator fun iterator():C=this; operator fun hasNext():Boolean=false; operator fun next():Int=0 }; fun f(c:C?) { for(n in c) {} }'),
            ('range_modifier', 'class C { fun rangeUntil(other:C):Int=1 }; fun f():Int=C()..<C()'),
            ('source_argument_type', 'class C; operator fun C.rangeUntil(other:String):Int=1; fun f():Int=C()..<C()'),
        ]
        unsupported = [
            ('mixed_integral', 'fun f(a:Int,b:Long):LongRange=a..<b'),
            ('byte_integral', 'fun f(a:Byte,b:Byte):IntRange=a..<b'),
            ('unit_element', 'class C { operator fun iterator():C=this; operator fun hasNext():Boolean=false; operator fun next() {} }; fun f() { for(n in C()) {} }'),
        ]
        for valid, cases in [(False, invalid), (True, unsupported)]:
            for ident, source in cases:
                source_path = work / (ident + '.kt')
                source_path.write_text(source)
                run('oracle boundary ' + ident, [official, source_path, *options, '-cp', standard, '-d', work / ('oracle-' + ident)], expected=0 if valid else 1)
                result = run('candidate boundary ' + ident, [candidate, source_path, '-cp', standard, '-d', work / ('candidate-' + ident), '--diagnostic-format', 'json'], expected=1)
                assert not list((work / ('candidate-' + ident)).rglob('*.class'))
                if valid:
                    assert 'unsupported_language' in result.stderr, result.stderr
                case('boundary/' + ident, 'unsupported_language' if valid else 'invalid_rejected', diagnostic=result.stderr, source=rel(source_path), source_sha256=sha(source_path))
        app_root = root / 'benchmarks/sources/androidbible'
        app = app_root / 'Alkitab/src/main/java/yuku/alkitab/songs/SongFilter.kt'
        app_sha = sha(app)
        inventory = json.loads((root / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        assert inventory[str(app.relative_to(app_root))] == app_sha
        case('androidbible_assessment', 'not_assessed', source=rel(app), sha256=app_sha, observations='SongFilter.kt lines 101/120 use collection loops; snapshot search found no custom operator iterator declarations or ..< occurrences. This feature expands reusable semantics without claiming a new unchanged app file.')
        assert frozen == {rel(path): sha(path) for path in frozen_files}
        assert caller_hashes == {str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}
        report['inputs']['fixed_java_classes_sha256'] = caller_hashes
        case('frozen_inputs')
        report['status'] = 'pass'
    except Exception as error:
        case('harness', 'correctness_failure', error=str(error))
        report['status'] = 'fail'
    finally:
        save()
        if not args.skip_html:
            subprocess.run(['python3', str(root / 'tools/reports.py')], cwd=root, check=False, capture_output=True)
    print(json.dumps(dict(status=report['status'], counts=report['counts'], report=rel(path))))
    return int(report['status'] != 'pass')


if __name__ == '__main__':
    raise SystemExit(main())
