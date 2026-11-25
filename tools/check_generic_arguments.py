#!/usr/bin/env python3
"""Pinned Kotlin/JVM evidence for generic source member arguments."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--reference-root', type=Path, required=True)
    parser.add_argument('--family', choices=['defaults', 'varargs'], default='defaults')
    args = parser.parse_args()
    candidate = args.candidate.resolve()
    reference = args.reference_root.resolve()
    pin = json.loads((reference / 'references/compiler.json').read_text())
    kotlin = reference / pin['path'] / 'bin/kotlinc'
    stdlib = kotlin.parent.parent / 'lib/kotlin-stdlib.jar'
    fixtures = ROOT / 'tests/generic_arguments'
    name = 'Default' if args.family == 'defaults' else 'Vararg'
    provider = fixtures / (name + 's.kt')
    java = Path(os.environ['JAVA_HOME']) / 'bin/java'
    javac = java.with_name('javac')
    javap = java.with_name('javap')
    stamp = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H%M%S%fZ')
    work = ROOT / 'target/checks' / (stamp + '-generic-' + args.family)
    work.mkdir(parents=True)
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    frozen_paths = [*ROOT.glob('src/**/*.rs'), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', *fixtures.glob('*'), ROOT / 'tests/generic_arguments.rs', Path(__file__)]
    frozen = {str(p.relative_to(ROOT)):sha(p) for p in frozen_paths}
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), compiler={'binary':str(candidate), 'sha256':sha(candidate), 'source_revision':revision}, reference_compiler={'version':pin['version'], 'archive_sha256':pin['archive_sha256'], 'options':options, 'compiler_jar_sha256':sha(kotlin.parent.parent / 'lib/kotlin-compiler.jar'), 'stdlib_sha256':sha(stdlib)}, inputs={'files_sha256':frozen}, commands=[], cases=[])

    report['reproduction_commands'] = [['python3', 'tools/check_generic_arguments.py', '--candidate', str(candidate), '--reference-root', str(reference), '--family', args.family]]
    report['limitations'] = ['Independent generic method parameters in supported ordinary source classes; existing classifier restrictions remain.', 'Generic inline/suspend and interface varargs remain explicit unsupported boundaries. General source Array<out T> annotations are not enabled by internal projected vararg body types.', 'Java callers compile once against official output and run unchanged against each provider with verification; official Kotlin consumers compile against each provider. No Android, full-app or performance claim.']

    def run(label, command, reject=False):
        result = subprocess.run(list(map(str, command)), cwd=ROOT, capture_output=True, text=True, timeout=180)
        index = len(report['commands'])
        logs = {}
        for stream in ['stdout', 'stderr']:
            path = work / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            logs[stream] = str(path)
        report['commands'].append(dict(label=label, argv=list(map(str, command)), exit_code=result.returncode, **logs))
        if (result.returncode == 0) == reject:
            raise AssertionError(label + '\n' + result.stdout + result.stderr)
        return result.stdout

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    try:
        outputs = {p:work / p for p in ['official', 'candidate']}
        for p, output in outputs.items():
            command = [kotlin, provider, *options] if p == 'official' else [candidate, provider]
            run('provider/' + p, [*command, '-cp', stdlib, '-d', output])
        callers = work / 'fixed-java'
        run('java/compile-once-against-official', [javac, '-cp', cp(outputs['official'], stdlib), '-d', callers, fixtures / (name + 'Calls.java')])
        fixed_java = {str(p.relative_to(callers)):sha(p) for p in callers.rglob('*.class')}
        signatures = {}
        for p, output in outputs.items():
            result = run('java/verified/' + p, [java, '-Xverify:all', '-cp', cp(callers, output, stdlib), name + 'Calls'])
            assert result == 'OK\n', result
            report['cases'].append(dict(id='fixed-java/' + p, outcome='pass', callers_sha256=fixed_java))
            consumer = work / ('consumer-' + p)
            run('kotlin/compile/' + p, [kotlin, fixtures / (name + 'Consumer.kt'), *options, '-cp', cp(output, stdlib), '-d', consumer])
            result = run('kotlin/verified/' + p, [java, '-Xverify:all', '-cp', cp(consumer, output, stdlib), name + 'ConsumerKt'])
            assert result == 'OK\n', result
            report['cases'].append(dict(id='kotlin-consumer/' + p, outcome='pass'))
            signatures[p] = run('abi/' + p, [javap, '-p', '-s', '-classpath', output, 'genericargs.' + name + 's'])
        def normalized_abi(output):
            members, declaration = [], None
            header = next(line for line in output.splitlines() if line.startswith('public '))
            for line in output.splitlines():
                if line.startswith('  ') and not line.startswith('    '):
                    declaration = line.strip()
                elif line.strip().startswith('descriptor:') and declaration:
                    members.append((declaration, line.strip()))
            return header, sorted(members)
        assert normalized_abi(signatures['official']) == normalized_abi(signatures['candidate']), signatures
        report['cases'].append(dict(id='jvm-signatures', outcome='pass'))
        if args.family == 'varargs':
            metadata = work / 'metadata-check'
            compiler_jar = kotlin.parent.parent / 'lib/kotlin-compiler.jar'
            run('metadata/compile-once', [javac, '-cp', cp(outputs['official'], stdlib, compiler_jar), '-d', metadata, fixtures / 'VarargMetadata.java'])
            for p, output in outputs.items():
                result = run('metadata/projections/' + p, [java, '-Xverify:all', '-cp', cp(metadata, output, stdlib, compiler_jar), 'VarargMetadata'])
                assert result == 'OK\n', result
                report['cases'].append(dict(id='metadata-projection/' + p, outcome='pass'))
        for item in json.loads((fixtures / 'Rejections.json').read_text()):
            if item.get('family', 'defaults') != args.family:
                continue
            source = work / (item['id'] + '.kt')
            source.write_text(item['source'])
            run('reject/official/' + item['id'], [kotlin, source, *options, '-d', work / ('invalid-official-' + item['id'])], True)
            run('reject/candidate/' + item['id'], [candidate, source, '-cp', stdlib, '-d', work / ('invalid-candidate-' + item['id'])], True)
            report['cases'].append(dict(id='reject/' + item['id'], outcome='invalid_rejected', source_sha256=sha(source)))
        assert all(sha(ROOT / p) == digest for p, digest in frozen.items())
        assert sha(candidate) == report['compiler']['sha256']
        report['cases'].append(dict(id='frozen-inputs', outcome='pass'))
        report['status'] = 'completed'
    except Exception as error:
        report['status'] = 'failed'
        report['cases'].append(dict(id='harness', outcome='correctness_failure', error=str(error)))
    report['counts'] = {outcome:sum(c['outcome'] == outcome for c in report['cases']) for outcome in sorted({c['outcome'] for c in report['cases']})}
    report['evidence'] = str(work)
    path = work / 'report.json'
    path.write_text(json.dumps(report, indent=2) + '\n')
    print(path)
    print(json.dumps(report['counts']))
    return int(report['status'] != 'completed')


if __name__ == '__main__':
    raise SystemExit(main())
