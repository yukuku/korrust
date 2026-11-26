#!/usr/bin/env python3
"""Verify anonymous suspend overrides, erased bridges, and resumed JVM calls."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import urllib.request
import zipfile

from check_member_function_properties import public_signatures

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/anonymous_suspend_methods'
HELPER = ROOT / 'tests/suspend_receiver_functions/Library.kt'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/debug/korrust')
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--coroutines', type=Path)
    parser.add_argument('--no-report', action='store_true')
    parser.add_argument('--runtime-only', action='store_true')
    parser.add_argument('--skip-html', action='store_true')
    args = parser.parse_args()
    candidate = args.candidate.resolve()
    source_root = args.source_root.resolve()
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    reference = ROOT / pin['path']
    artifact_path = ROOT / 'references/coroutines.json'
    artifact = json.loads(artifact_path.read_text())
    coroutine = args.coroutines.resolve() if args.coroutines else ROOT / artifact['path']
    if not coroutine.exists():
        coroutine.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(artifact['url'], timeout=60) as response:
            coroutine.write_bytes(response.read())
    assert sha(coroutine) == artifact['sha256'], 'Coroutine artifact checksum mismatch'
    kotlin = reference / 'bin/kotlinc'
    standard = reference / 'lib/kotlin-stdlib.jar'
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-anonymous-suspend-methods'
    temporary = tempfile.TemporaryDirectory(prefix='korrust-suspend-receivers-') if args.no_report else None
    work = Path(temporary.name) if temporary else ROOT / 'target/checks' / ident
    logs = work / 'logs' if temporary else ROOT / 'reports/correctness' / ident
    work.mkdir(parents=True, exist_ok=True)
    logs.mkdir(parents=True)
    def rel(path):
        return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)
    inputs = [*FIXTURES.glob('*'), artifact_path, ROOT / 'references/compiler.json',
              Path(__file__), ROOT / 'tools/check_member_function_properties.py',
              ROOT / 'tools/check_nov21_generics.py', ROOT / 'tests/anonymous_suspend_methods.rs', HELPER]
    if not temporary:
        inputs += [*source_root.glob('src/**/*.rs'), source_root / 'Cargo.toml', source_root / 'Cargo.lock']
    frozen = {rel(path): sha(path) for path in inputs if path.is_file()}
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
                  compiler=dict(binary=rel(candidate), binary_sha256=sha(candidate), source_root=rel(source_root)),
                  reference_compiler=dict(version=pin['version'], archive_sha256=pin['archive_sha256'], compiler_jar_sha256=sha(reference / 'lib/kotlin-compiler.jar')),
                  coroutine_dependency=artifact, inputs=dict(files_sha256=frozen, stdlib_sha256=sha(standard)),
                  commands=[], cases=[], evidence=[],
                  limitations=['Anonymous non-generic suspend overrides reuse ordinary continuation frames, with separate frames for every suspending implementation and continuation forwarding through erased generic bridges. Suspend fun-interface SAM literals preserve the continuation parameter; inferred/declared covariant returns are supported when their JVM carrier matches the inherited result. Fixed Java callers compile against official output and verify parked resumption, saved receiver/captures, primitive/reference/Unit results, two sequential resumes, base dispatch and direct Continuation ABI. Kotlin provider/frontend consumers are executed by the full mode. Own and generic suspend method declarations, Unit parameters and nullable Unit results remain explicit unsupported boundaries. No reflection, whole-application or performance measurement.'])
    if not temporary:
        archive = logs / 'frozen-inputs.zip'
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
            for path in inputs:
                if path.is_file():
                    bundle.writestr(rel(path), path.read_bytes())
        report['source_archive'] = dict(path=rel(archive), sha256=sha(archive))
    environment = {**os.environ, 'JAVA_OPTS': '-Xmx384m -Xms32m'}
    def run(label, command, accepted=(0,)):
        command = list(map(str, command))
        result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, timeout=180, env=environment)
        entry = dict(label=label, argv=command, exit_code=result.returncode)
        for stream in ['stdout', 'stderr']:
            path = logs / f'{len(report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = rel(path)
        report['commands'].append(entry)
        assert result.returncode in accepted, label + '\n' + result.stdout + result.stderr
        return result
    def case(name, valid):
        report['cases'].append(dict(id=name, outcome='pass' if valid else 'correctness_failure'))
        assert valid, name
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def compile(frontend, source, output, dependencies=()):
        executable = kotlin if frontend == 'official' else candidate
        run('compile ' + frontend + '/' + source.name,
            [executable, source, '-cp', cp(*dependencies, coroutine, standard), '-d', output])
    try:
        library = work / 'library'
        compile('official', HELPER, library)
        contracts = work / 'contracts'
        compile('official', FIXTURES / 'Library.kt', contracts, [library])
        outputs = {name: work / name for name in ['official', 'candidate']}
        for provider, output in outputs.items():
            compile(provider, FIXTURES / 'Values.kt', output, [library, contracts])
        caller = work / 'java'
        run('compile fixed Java caller against official', ['javac', '-cp', cp(outputs['official'], library, contracts, coroutine, standard), '-d', caller, FIXTURES / 'Check.java'])
        signatures = {}
        for provider, output in outputs.items():
            result = run('verified suspended Java caller/' + provider, ['java', '-Xmx256m', '-Xverify:all', '-cp', cp(caller, output, library, contracts, coroutine, standard), 'Check'])
            case('resumed-java/' + provider, result.stdout == 'OK\n')
            result = run('JVM signature/' + provider, ['javap', '-p', '-s', '-v', '-cp', cp(output, library, contracts), 'anonymousfunctions.ValuesKt'])
            signatures[provider] = public_signatures(result.stdout)
        case('JVM-public-nonsynthetic-callable-signatures', signatures['official'] == signatures['candidate'])
        if not args.runtime_only:
            runner = None
            for provider, output in outputs.items():
                for frontend in ['official', 'candidate']:
                    consumer = work / ('consumer-' + provider + '-' + frontend)
                    compile(frontend, FIXTURES / 'Consumer.kt', consumer, [output, library, contracts])
                    if runner is None:
                        runner = work / 'consumer-java'
                        run('compile fixed consumer caller against official', ['javac', '-cp', cp(consumer, caller, output, library, contracts, coroutine, standard), '-d', runner, FIXTURES / 'ConsumerCheck.java'])
                    result = run('verified resumed consumer/' + provider + '/' + frontend, ['java', '-Xmx256m', '-Xverify:all', '-cp', cp(runner, consumer, caller, output, library, contracts, coroutine, standard), 'ConsumerCheck'])
                    case('resumed-consumer/' + provider + '/' + frontend, result.stdout == 'OK\n')
            for item in json.loads((FIXTURES / 'Rejections.json').read_text()):
                source = logs / (item['id'] + '.kt')
                source.write_text(item['source'])
                oracle = run('boundary official/' + item['id'], [kotlin, source, '-cp', cp(library, contracts, coroutine, standard), '-d', work / ('oracle-' + item['id'])], (0, 1))
                result = run('boundary candidate/' + item['id'], [candidate, source, '--diagnostic-format', 'json', '-cp', cp(library, contracts, coroutine, standard), '-d', work / ('candidate-' + item['id'])], (1,))
                diagnostics = [json.loads(line) for line in result.stderr.splitlines() if line.startswith('{')]
                assert oracle.returncode == (0 if item.get('unsupported') else 1), item['id']
                assert diagnostics
                if item.get('unsupported'):
                    assert diagnostics[0]['category'] == 'unsupported_language', diagnostics
                else:
                    assert diagnostics[0]['category'] in ['invalid_kotlin', 'unclassified'], diagnostics
                report['cases'].append(dict(id='boundary/' + item['id'], outcome='unsupported_language' if item.get('unsupported') else 'invalid_rejected', oracle_exit_code=oracle.returncode, diagnostic=diagnostics, rejection_check='pass'))
                report['evidence'].append(rel(source))
        case('frozen-inputs', all(sha(Path(path) if Path(path).is_absolute() else ROOT / path) == value for path, value in frozen.items()))
        report['status'] = 'passed'
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
        raise
    finally:
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        if temporary:
            print(json.dumps(dict(status=report['status'], counts=report['counts'])))
        else:
            path = logs.with_suffix('.json')
            report['reproduction_commands'] = [['python3', rel(Path(__file__)), '--candidate', str(candidate), '--source-root', str(source_root), '--coroutines', str(coroutine)]]
            if args.runtime_only:
                report['reproduction_commands'][0].append('--runtime-only')
            path.write_text(json.dumps(report, indent=2) + '\n')
            print(path)
            if not args.skip_html:
                subprocess.run(['python3', ROOT / 'tools/reports.py'], cwd=ROOT, check=True)


if __name__ == '__main__':
    main()
