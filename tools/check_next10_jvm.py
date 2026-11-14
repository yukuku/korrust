#!/usr/bin/env python3
"""Differential evidence for canonical file facades and nonpublic constructors."""
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

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def cp(*paths):
    return os.pathsep.join(map(str, paths))


def signatures(text):
    lines = []
    declaration = None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith(('public ', 'protected ')) and line.endswith(';'):
            declaration = None if ' access$' in line else line
        elif line.startswith('descriptor:') and declaration:
            lines.append(declaration + ' ' + line)
            declaration = None
    return sorted(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    root, candidate = args.source_root.resolve(), args.candidate.resolve()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-next10-jvm'
    work = args.output.resolve() if args.output else root / 'target/checks' / ident
    logs = root / 'reports/correctness' / ident
    work.mkdir(parents=True, exist_ok=False)
    logs.mkdir(parents=True)
    fixtures = root / 'tests/next10_jvm'
    oracle = root / 'references/kotlinc/bin/kotlinc'
    standard = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
    compiler_jar = root / 'references/kotlinc/lib/kotlin-compiler.jar'
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    files = [*root.glob('src/**/*.rs'), *fixtures.glob('*'), root / 'tests/next10_jvm.rs', root / 'tools/check_next10_jvm.py', root / 'Cargo.toml', root / 'Cargo.lock', root / 'references/compiler.json', root / 'references/sources.json']
    files += [root / 'references/kotlin/compiler/ir/backend.jvm/lower/src/org/jetbrains/kotlin/backend/jvm/lower/FileClassLowering.kt', root / 'references/kotlin/compiler/frontend.common.jvm/src/org/jetbrains/kotlin/fileClasses/JvmFileClassUtil.kt', root / 'references/kotlin-spec/docs/src/md/kotlin.core/declarations.md']
    frozen = {str(path): sha(path) for path in files if path.is_file()}
    frozen.update({str(path): sha(path) for path in [candidate, standard, compiler_jar]})
    def relative(path):
        path = Path(path)
        return str(path.relative_to(root)) if path.is_relative_to(root) else str(path)
    with zipfile.ZipFile(logs / 'source.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            if path.is_file():
                archive.write(path, relative(path))
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running', compiler={'path': str(candidate), 'sha256': sha(candidate), 'source_root': str(root)}, reference_compiler=json.loads((root / 'references/compiler.json').read_text()), frozen_input_hashes={relative(Path(path)): digest for path, digest in frozen.items()}, environment={'JAVA_HOME': os.environ['JAVA_HOME']}, commands=[], cases=[], evidence=[relative(logs / 'source.zip')], reproduction_commands=[['python3', 'tools/check_next10_jvm.py', '--candidate', str(candidate), '--source-root', str(root)]], limitations=['File JvmName consumes canonical annotation identity and folded String arguments; bounded ASCII identifier JVM facade names. Existing non-ASCII source filename boundary remains even with an explicit facade name. Multifile facades, source annotation type aliases and member JvmName remain unsupported.', 'Private/internal primary constructors of ordinary final nongeneric classes, without defaults, constructor annotations or secondary constructors. Protected, inner, value, sealed, data and open combinations retain explicit boundaries. Ordinary constructor callable references retain their existing explicit unsupported boundary.', 'Java callers compile once against official output and run unchanged against both providers with -Xverify:all. Kotlin consumers are compiled against each provider separately; candidate consumers are also compiled against both providers.', 'Custom marks.JvmName source probes expose a documented official short-name lowering discrepancy; those results are excluded from differential passes. No full Android application or performance claim. DownloadMapper and SongFilter motivate visibility but contain broader unsupported semantics.'])
    def run(label, argv, accept=(0,)):
        result = subprocess.run(list(map(str, argv)), cwd=root, capture_output=True, text=True, timeout=240)
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
    def kotlin(label, sources, output, dependencies=(), friend=None):
        argv = [oracle, *sources, '-d', output, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-module-name', 'next_jvm']
        if dependencies:
            argv += ['-cp', cp(*dependencies)]
        if friend:
            argv += ['-Xfriend-paths=' + str(friend)]
        return run(label, argv)
    def korrust(label, sources, output, dependencies=(), friend=None, accept=(0,)):
        argv = [candidate, *sources, '-d', output, '--module-name', 'next_jvm', '-cp', cp(standard, *dependencies), '--diagnostic-format', 'json']
        if friend:
            argv += ['--friend-paths', friend]
        return run(label, argv, accept)
    failures = []
    def test(name, task):
        try:
            task()
        except Exception as error:
            failures.append(name)
            case(name, 'fail', error=str(error))
    official, emitted = work / 'official', work / 'candidate'
    providers = [('official', official), ('candidate', emitted)]
    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Pinned Kotlin identity', [oracle, '-version'])
        sources = [fixtures / name for name in ['Named.kt', 'Classes.kt', 'CrossFile.kt']]
        kotlin('Official provider', sources, official)
        korrust('Candidate provider', sources, emitted)
        case('provider/source-compilation')
        def provider_contract():
            for name, output in providers:
                assert (output / 'nextjvm/api/StableApi.class').is_file()
                assert not (output / 'nextjvm/api/NamedKt.class').exists()
                modules = list((output / 'META-INF').glob('*.kotlin_module'))
                assert modules and any(b'StableApi' in path.read_bytes() for path in modules)
                javap = run(name + ' facade annotation', [jdk / 'javap', '-v', '-classpath', output, 'nextjvm.api.StableApi']).stdout
                assert 'kotlin.jvm.JvmName' in javap and 'name="StableApi"' in javap
            names = ['nextjvm.api.StableApi', 'nextjvm.api.Secret', 'nextjvm.api.Secret$Companion', 'nextjvm.api.Secret$Nested', 'nextjvm.api.ModuleOnly', 'nextjvm.cross.CrossFileKt']
            observed = [signatures(run(name + ' public signatures', [jdk / 'javap', '-public', '-s', '-classpath', output, *names]).stdout) for name, output in providers]
            assert observed[0] == observed[1], '\n'.join(observed[0]) + '\nCANDIDATE\n' + '\n'.join(observed[1])
            case('provider/jvm-signatures-facade-mapping')
        test('provider/jvm-signatures-facade-mapping', provider_contract)
        java = work / 'java'
        run('Compile Java caller once against official provider', [jdk / 'javac', '-cp', cp(official, standard, compiler_jar), '-d', java, fixtures / 'JavaCaller.java', fixtures / 'MetadataCheck.java'])
        for name, output in providers:
            def check_java(name=name, output=output):
                assert run(name + ' Java runtime', [jdk / 'java', '-Xverify:all', '-cp', cp(java, output, standard), 'JavaCaller']).stdout == 'OK\n'
                assert run(name + ' metadata assertions', [jdk / 'java', '-Xverify:all', '-cp', cp(java, output, standard, compiler_jar), 'MetadataCheck']).stdout == 'METADATA\n'
                case('provider/' + name + '/java-runtime-metadata')
            test('provider/' + name + '/java-runtime-metadata', check_java)
            def consumers(name=name, output=output):
                for compiler_name in ['official', 'candidate']:
                    consumer = work / (compiler_name + '-consumer-' + name)
                    if compiler_name == 'official':
                        kotlin('Official Kotlin consumer of ' + name, [fixtures / 'Consumer.kt'], consumer, [output])
                    else:
                        korrust('Candidate Kotlin consumer of ' + name, [fixtures / 'Consumer.kt'], consumer, [output])
                    assert run(compiler_name + ' consumer run with ' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, standard), 'nextjvm.consumer.ConsumerKt']).stdout == 'OK\n'
                    case('consumer/' + compiler_name + '-against-' + name)
                for compiler_name in ['official', 'candidate']:
                    consumer = work / (compiler_name + '-friend-' + name)
                    if compiler_name == 'official':
                        kotlin('Official friend consumer of ' + name, [fixtures / 'Friend.kt'], consumer, [output], output)
                    else:
                        korrust('Candidate friend consumer of ' + name, [fixtures / 'Friend.kt'], consumer, [output], output)
                    assert run(compiler_name + ' friend run with ' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, output, standard), 'nextjvm.consumer.FriendKt']).stdout == 'FRIEND\n'
                    case('consumer/friend-' + compiler_name + '-against-' + name)
                reject_official = run('Official nonfriend access ' + name, [oracle, fixtures / 'Friend.kt', '-cp', output, '-d', work / ('reject-official-' + name)], accept=(1,))
                assert 'cannot access' in reject_official.stderr.lower()
                reject_candidate = korrust('Candidate nonfriend access ' + name, [fixtures / 'Friend.kt'], work / ('reject-candidate-' + name), [output], accept=(1,))
                diagnostic = json.loads(reject_candidate.stderr.splitlines()[0])
                assert diagnostic['category'] == 'invalid_kotlin' and diagnostic['code'] == 'INACCESSIBLE_CONSTRUCTOR', diagnostic
                case('consumer/nonfriend-' + name, 'invalid_kotlin', code=diagnostic['code'])
                for filename in ['PrivateBad.kt', 'PrivateAliasBad.kt']:
                    run('Official compiled private access ' + name + '/' + filename, [oracle, fixtures / filename, '-cp', output, '-d', work / ('private-official-' + name + filename)], accept=(1,))
                    result = korrust('Candidate compiled private access ' + name + '/' + filename, [fixtures / filename], work / ('private-candidate-' + name + filename), [output], accept=(1,))
                    diagnostic = json.loads(result.stderr.splitlines()[0])
                    assert diagnostic['category'] == 'invalid_kotlin' and diagnostic['code'] == 'INACCESSIBLE_CONSTRUCTOR', diagnostic
                    case('consumer/private-' + name + '/' + filename, 'invalid_kotlin', code=diagnostic['code'])
            test('consumer/' + name, consumers)
        def facade_variants():
            for compiler_name in ['official', 'candidate']:
                output = work / ('shadow-' + compiler_name)
                sources = [fixtures / name for name in ['ShadowDefinition.kt', 'ShadowUse.kt', 'ShadowAlias.kt', 'ShadowWildcard.kt', 'Folded.kt']]
                if compiler_name == 'official':
                    kotlin('Official canonical and shadowed annotations', sources, output)
                else:
                    korrust('Candidate canonical and shadowed annotations', sources, output)
                assert (output / 'folded/FoldedApi.class').is_file()
                expected = ['marks/ShadowUseKt.class', 'shadowalias/ShadowAliasKt.class', 'shadowwildcard/ShadowWildcardKt.class']
                oracle_observed = ['marks/NotAFacade.class', 'shadowalias/NotAFacadeEither.class', 'shadowwildcard/NotAWildcardFacade.class']
                for path in expected if compiler_name == 'candidate' else oracle_observed:
                    assert (output / path).is_file(), path
                    classname = path.removesuffix('.class').replace('/', '.')
                    annotation = run(compiler_name + ' custom JvmName identity ' + classname, [jdk / 'javap', '-v', '-classpath', output, classname]).stdout
                    assert 'marks.JvmName(' in annotation and 'kotlin.jvm.JvmName(' not in annotation
                if compiler_name == 'candidate':
                    assert not list(output.rglob('NotAFacade*.class'))
                    case('facade/canonical-shadow-folded-candidate', verification='Independent canonical annotation identity and facade-name assertions')
                else:
                    case('facade/canonical-folded-official')
                    case('facade/custom-name-oracle-discrepancy', 'oracle_discrepancy', official_facades=oracle_observed, canonical_identity_expected_facades=expected, evidence='Official javap records marks.JvmName on each renamed class, including import aliases and wildcard imports.', resolution='Preserve canonical identity and ordinary annotation retention; exclude this case from differential pass counts. Pinned IR FileClassLowering.kt:194-197 compares the annotation constructor parent class name only; the emitted annotation descriptor proves marks.JvmName identity. This is recorded as a suspected oracle bug, not adopted as language behavior.')
        test('facade/canonical-shadow-folded', facade_variants)
        def stale():
            path = work / 'Rename.kt'
            before = (fixtures / 'Rename.kt').read_text()
            path.write_text(before)
            output, clean = work / 'incremental', work / 'clean'
            korrust('Facade initial owned output', [path], output)
            assert (output / 'rename/Before.class').is_file()
            path.write_text(before.replace('"Before"', '"After"'))
            korrust('Facade renamed owned output', [path], output)
            korrust('Facade renamed clean output', [path], clean)
            assert not (output / 'rename/Before.class').exists() and (output / 'rename/After.class').is_file()
            def owned(directory):
                return {str(path.relative_to(directory)): sha(path) for path in directory.rglob('*') if path.is_file() and path.suffix in ['.class', '.kotlin_module']}
            assert owned(output) == owned(clean)
            path.write_text(before.replace('@file:kotlin.jvm.JvmName("Before")\n', ''))
            korrust('Facade annotation removed owned output', [path], output)
            assert (output / 'rename/RenameKt.class').is_file() and not (output / 'rename/After.class').exists()
            case('facade/owned-rename-removal-matches-clean')
        test('facade/owned-rename-removal-matches-clean', stale)
        definitions = json.loads((fixtures / 'Rejections.json').read_text())
        for definition in definitions:
            def boundary(definition=definition):
                source = work / (definition['id'] + '.kt')
                source.write_text(definition['source'])
                expected = 0 if definition['official_accepts'] else 1
                result = run('Official boundary ' + definition['id'], [oracle, source, '-jvm-target', '1.8', '-d', work / ('official-boundary-' + definition['id'])], accept=(expected,))
                candidate_result = korrust('Candidate boundary ' + definition['id'], [source], work / ('candidate-boundary-' + definition['id']), accept=(1,))
                diagnostic = json.loads(candidate_result.stderr.splitlines()[0])
                assert diagnostic['category'] == definition['category'], diagnostic
                if 'code' in definition:
                    assert diagnostic['code'] == definition['code'], diagnostic
                case('boundary/' + definition['id'], definition['category'], diagnostic=diagnostic, oracle='accept' if expected == 0 else 'reject')
            test('boundary/' + definition['id'], boundary)
        def collisions():
            sources = []
            for name, function in [('A.kt', 'a'), ('B.kt', 'b')]:
                path = work / name
                path.write_text('@file:JvmName("Same")\nfun ' + function + '(): Int = 1\n')
                sources.append(path)
            run('Official duplicate facade', [oracle, *sources, '-d', work / 'collision-official'], accept=(1,))
            result = korrust('Candidate duplicate facade', sources, work / 'collision-candidate', accept=(1,))
            assert 'Duplicate source declaration: Same' in result.stderr
            assert not list((work / 'collision-candidate').rglob('*.class'))
            case('boundary/duplicate-facade', 'invalid_kotlin')
        test('boundary/duplicate-facade', collisions)
    except Exception as error:
        failures.append('harness')
        case('harness', 'fail', error=str(error))
    finally:
        changed = [path for path, digest in frozen.items() if sha(path) != digest]
        if changed:
            failures.append('frozen-inputs-changed')
            case('frozen-inputs-changed', 'fail', paths=changed)
        report['status'] = 'fail' if failures else ('pass_with_oracle_discrepancy' if any(case['outcome'] == 'oracle_discrepancy' for case in report['cases']) else 'pass')
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['completed_at'] = datetime.now(timezone.utc).isoformat()
        report_path = logs.with_suffix('.json')
        report_path.write_text(json.dumps(report, indent=2) + '\n')
        if not args.skip_html:
            subprocess.run(['python3', root / 'tools/reports.py'], cwd=root, check=True)
        print(report_path)
        print(json.dumps(report['counts'], sort_keys=True))
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
