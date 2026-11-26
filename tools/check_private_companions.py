import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/private_companions'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-private-companions')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    binary = work / 'korrust-frozen'
    shutil.copy2(args.compiler, binary)
    inputs = {str(p.relative_to(ROOT)): sha(p) for folder in ['src', 'tests/private_companions'] for p in (ROOT / folder).rglob('*') if p.is_file()}
    inputs.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock', 'tests/private_companions.rs', 'tools/check_private_companions.py']})
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': now.isoformat(), 'status': 'running', 'selection': 'Private unnamed class companions: source and dependency constant, method and property visibility, private JVM singleton, nonpublic companion class, private InnerClasses entry, private Kotlin companion metadata, and public physical Java const fields. A fixed Java caller compiled against official output runs against both providers with verification.', 'compiler': {'binary_sha256': sha(binary)}, 'reference_compiler': json.loads((ROOT / 'references/compiler.json').read_text()), 'inputs': {'files_sha256': inputs}, 'dependencies': {str(stdlib): sha(stdlib)}, 'commands': [], 'cases': []}

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def run(label, argv):
        result = subprocess.run(list(map(str, argv)), cwd=ROOT, capture_output=True, text=True, timeout=240)
        index = len(report['commands'])
        stdout, stderr = work / f'{index:03d}-stdout.txt', work / f'{index:03d}-stderr.txt'
        stdout.write_text(result.stdout)
        stderr.write_text(result.stderr)
        report['commands'].append({'id': label, 'argv': list(map(str, argv)), 'exit_code': result.returncode, 'stdout': str(stdout.relative_to(ROOT)), 'stderr': str(stderr.relative_to(ROOT))})
        print(label, result.returncode, result.stderr[:160].strip(), flush=True)
        return result

    def case(label, success, outcome='pass', **extra):
        report['cases'].append({'id': label, 'outcome': outcome if success else 'correctness_failure', **extra})

    def compile(provider, source, output, dependency=None):
        dependencies = cp(*([dependency] if dependency else []), stdlib)
        if provider == 'official':
            return run(provider + '/' + output.name, [kotlin, source, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-cp', dependencies, '-d', output])
        return run(provider + '/' + output.name, [binary, source, '-cp', dependencies, '--module-name', 'private_companions', '--diagnostic-format', 'json', '-d', output])

    try:
        providers = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, FIXTURES / 'Hidden.kt', output)
            case(provider + '/compile', result.returncode == 0)
            if result.returncode == 0:
                providers[provider] = output
        if len(providers) != 2:
            return 1
        callers = work / 'caller'
        result = run('fixed-java-caller', ['javac', '--release', '8', '-cp', cp(providers['official'], stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        case('caller/compile-official', result.returncode == 0)
        report['fixed_java_caller_sha256'] = {str(p.relative_to(callers)): sha(p) for p in callers.rglob('*.class')}
        for provider, output in providers.items():
            result = run(provider + '/runtime', ['java', '-Xverify:all', '-cp', cp(callers, output, stdlib), 'companioncalls.Calls'])
            case(provider + '/runtime', result.returncode == 0 and result.stdout.strip() == 'private-companion-ok', observed=result.stdout.strip())
            for name in ['Vault', 'Vault$Companion', 'Empty$Companion']:
                result = run(provider + '/jvm/' + name, ['javap', '-p', '-v', '-classpath', output, 'companionhidden.' + name])
                flags = 'flags: (0x0030) ACC_FINAL, ACC_SUPER' if '$' in name else 'flags: (0x0031) ACC_PUBLIC, ACC_FINAL, ACC_SUPER'
                private_inner = 'private static final' in result.stdout and '=class companionhidden/' in result.stdout
                case(provider + '/jvm/' + name, result.returncode == 0 and flags in result.stdout and private_inner)
            result = compile('official', FIXTURES / 'ValidConsumer.kt', work / (provider + '-consumer'), output)
            case(provider + '/public-consumer', result.returncode == 0)
            for source in sorted(FIXTURES.glob('Invalid*.kt')):
                if source.name == 'InvalidSource.kt':
                    continue
                official = compile('official', source, work / (provider + '-' + source.stem + '-official'), output)
                candidate = compile('candidate', source, work / (provider + '-' + source.stem + '-candidate'), output)
                diagnostic = json.loads(candidate.stderr.strip().splitlines()[-1]) if candidate.returncode else {}
                case(provider + '/' + source.stem, official.returncode != 0 and candidate.returncode != 0 and diagnostic.get('category') == 'invalid_kotlin', 'invalid_rejected', diagnostic=diagnostic)
        source = FIXTURES / 'InvalidSource.kt'
        official = compile('official', source, work / 'source-invalid-official')
        candidate = compile('candidate', source, work / 'source-invalid-candidate')
        diagnostic = json.loads(candidate.stderr.strip().splitlines()[-1]) if candidate.returncode else {}
        case('source/private-constant', official.returncode != 0 and candidate.returncode != 0 and diagnostic.get('category') == 'invalid_kotlin', 'invalid_rejected', diagnostic=diagnostic)
        current = {str(p.relative_to(ROOT)): sha(p) for folder in ['src', 'tests/private_companions'] for p in (ROOT / folder).rglob('*') if p.is_file()}
        current.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock', 'tests/private_companions.rs', 'tools/check_private_companions.py']})
        case('inputs/unchanged', current == inputs)
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['status'] = 'pass' if 'correctness_failure' not in report['counts'] else 'fail'
        report['limitations'] = ['Only unnamed class companions are covered. Existing named companion and nested companion member boundaries remain. No general JVM bytecode equivalence or full-application compatibility claim.']
        return int(report['status'] != 'pass')
    except Exception as error:
        case('harness', False, reason=f'{type(error).__name__}: {error}')
        return 1
    finally:
        report['input_integrity'] = {'status': 'unchanged' if all(sha(ROOT / name) == digest for name, digest in inputs.items()) else 'changed'}
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['status'] = 'pass' if report['cases'] and 'correctness_failure' not in report['counts'] and report['input_integrity']['status'] == 'unchanged' else 'failed'
        report['evidence'] = [str(p.relative_to(ROOT)) for p in sorted(work.iterdir()) if p.is_file() and p != binary]
        (work / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        print(work / 'report.json', flush=True)


if __name__ == '__main__':
    raise SystemExit(main())
