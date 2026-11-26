import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
import re
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/source_composable_types'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--compiler', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    work = (args.output or ROOT / 'target/checks' / (now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-source-composable-types')).resolve()
    work.mkdir(parents=True, exist_ok=False)
    binary = work / 'korrust-frozen'
    shutil.copy2(args.compiler, binary)
    inputs = {str(p.relative_to(ROOT)): sha(p) for folder in ['src', 'tests/source_composable_types'] for p in (ROOT / folder).rglob('*') if p.is_file()}
    inputs.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock', 'tests/source_composable_types.rs', 'tools/check_source_composable_types.py']})
    kotlin = ROOT / 'references/kotlinc/bin/kotlinc'
    stdlib = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    runtime = ROOT / 'references/compose/runtime.jar'
    annotations = ROOT / 'references/compose/annotations.jar'
    plugin = ROOT / 'references/kotlinc/lib/compose-compiler-plugin.jar'
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': now.isoformat(), 'status': 'running', 'selection': 'Source zero-argument Unit composable function types retain logical metadata and physical Function2 ABI. Typed forwarding, identity, capture, invocation with actual Composer and changed0, canonical/alias annotations and four provider/frontend combinations execute with verification.', 'compiler': {'binary_sha256': sha(binary)}, 'reference_compiler': json.loads((ROOT / 'references/compiler.json').read_text()), 'inputs': {'files_sha256': inputs}, 'dependencies': {str(p): sha(p) for p in [stdlib, runtime, annotations, plugin]}, 'commands': [], 'cases': []}

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
        dependencies = cp(*([dependency] if dependency else []), work / 'dependencies', runtime, annotations, stdlib)
        if provider == 'official':
            return run(provider + '/' + output.name, [kotlin, source, '-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8', '-Xlambdas=class', '-Xplugin=' + str(plugin), '-P', 'plugin:androidx.compose.compiler.plugins.kotlin:sourceInformation=false', '-cp', dependencies, '-d', output])
        return run(provider + '/' + output.name, [binary, source, '-cp', dependencies, '--module-name', 'source_composable_types', '--diagnostic-format', 'json', '-d', output])

    try:
        dependency = compile('official', FIXTURES / 'Dependency.kt', work / 'dependencies')
        case('dependency/compile', dependency.returncode == 0)
        providers = {}
        for provider in ['official', 'candidate']:
            output = work / provider
            result = compile(provider, FIXTURES / 'Callbacks.kt', output)
            case(provider + '/compile', result.returncode == 0)
            if result.returncode == 0:
                providers[provider] = output
        if len(providers) != 2:
            return 1
        fixed_use = work / 'fixed-official-use'
        result = compile('official', FIXTURES / 'Use.kt', fixed_use, providers['official'])
        case('fixed-consumer/compile', result.returncode == 0)
        callers = work / 'caller'
        result = run('fixed-java-caller', ['javac', '--release', '8', '-cp', cp(fixed_use, providers['official'], work / 'dependencies', runtime, annotations, stdlib), '-d', callers, FIXTURES / 'Calls.java'])
        case('caller/compile-official', result.returncode == 0)
        report['fixed_java_caller_sha256'] = {str(p.relative_to(callers)): sha(p) for p in callers.rglob('*.class')}
        for provider, output in providers.items():
            result = run(provider + '/runtime', ['java', '-Xverify:all', '-cp', cp(callers, fixed_use, output, work / 'dependencies', runtime, annotations, stdlib), 'composetypeprobe.Calls'])
            case(provider + '/runtime', result.returncode == 0 and result.stdout.strip() == 'source-composable-types-ok', observed=result.stdout.strip())
            signatures = run(provider + '/signatures', ['javap', '-protected', '-s', '-v', '-classpath', output, 'composetypeprobe.CallbacksKt'])
            case(provider + '/signatures-read', signatures.returncode == 0)
            report[provider + '_signatures'] = signatures.stdout
            consumer = work / (provider + '-consumer')
            result = compile('official', FIXTURES / 'Use.kt', consumer, output)
            case(provider + '/kotlin-consumer-compile', result.returncode == 0)
            result = run(provider + '/kotlin-consumer-runtime', ['java', '-Xverify:all', '-cp', cp(callers, consumer, output, work / 'dependencies', runtime, annotations, stdlib), 'composetypeprobe.Calls'])
            case(provider + '/kotlin-consumer-runtime', result.returncode == 0 and result.stdout.strip() == 'source-composable-types-ok', observed=result.stdout.strip())
            compiled_consumer = work / (provider + '-consumer-candidate')
            result = compile('candidate', FIXTURES / 'Use.kt', compiled_consumer, output)
            case(provider + '/candidate-consumer-compile', result.returncode == 0)
            result = run(provider + '/candidate-consumer-runtime', ['java', '-Xverify:all', '-cp', cp(callers, compiled_consumer, output, work / 'dependencies', runtime, annotations, stdlib), 'composetypeprobe.Calls'])
            case(provider + '/candidate-consumer-runtime', result.returncode == 0 and result.stdout.strip() == 'source-composable-types-ok', observed=result.stdout.strip())
        for source in sorted(p for p in [*FIXTURES.glob('Invalid*.kt'), *FIXTURES.glob('Unsupported*.kt')] if True):
            official = compile('official', source, work / (source.stem + '-official'))
            candidate = compile('candidate', source, work / (source.stem + '-candidate'))
            diagnostics = [json.loads(line) for line in candidate.stderr.splitlines() if line.startswith('{')]
            if source.stem.startswith('Unsupported'):
                code = {**dict.fromkeys(['UnsupportedArguments', 'UnsupportedResult', 'UnsupportedReceiver', 'UnsupportedNullable', 'UnsupportedComposite'], 'COMPOSE_CALLBACK_SHAPE_UNSUPPORTED'), 'UnsupportedClass': 'COMPOSE_SOURCE_CLASS_STABILITY_UNSUPPORTED', 'UnsupportedNested': 'COMPOSE_NESTED_CALLBACK_UNSUPPORTED', 'UnsupportedConditional': 'COMPOSE_CONTROL_FLOW_UNSUPPORTED', 'UnsupportedDeclaration': 'COMPILER_PLUGIN_UNSUPPORTED', 'UnsupportedStar': 'ANNOTATION_TYPE_USE_UNSUPPORTED', 'UnsupportedLookalike': 'ANNOTATION_TYPE_USE_UNSUPPORTED'}[source.stem]
                passed = official.returncode == 0 and candidate.returncode != 0 and bool(diagnostics) and all(d.get('category') == 'unsupported_language' and d.get('code') == code for d in diagnostics)
                outcome = 'unsupported_language'
            else:
                passed = official.returncode != 0 and candidate.returncode != 0 and bool(diagnostics) and all(d.get('category') == 'invalid_kotlin' for d in diagnostics)
                outcome = 'invalid_rejected'
            case(source.stem, passed and not list((work / (source.stem + '-candidate')).rglob('*.class')), outcome, diagnostics=diagnostics)
        def signature_contract(text):
            declared, synthetic = {}, {}
            owner = member = descriptor = None
            for line in text.splitlines():
                stripped = line.strip()
                if not line.startswith(' ') and re.match(r'(public|protected).*\b(class|interface) ', line):
                    owner = line
                    member = descriptor = None
                    declared[owner] = []
                    synthetic[owner] = []
                elif re.match(r'^  (public|protected) .+;$', line):
                    member = stripped
                    descriptor = None
                elif member and stripped.startswith('descriptor:'):
                    descriptor = stripped
                elif member and stripped.startswith('flags:'):
                    flags = int(re.search(r'0x([0-9a-fA-F]+)', stripped)[1], 16)
                    destination = synthetic if flags & 0x1000 else declared
                    destination[owner].append((member, descriptor))
                    member = descriptor = None
            return ({owner: sorted(members) for owner, members in declared.items()}, {owner: sorted(members) for owner, members in synthetic.items()})
        contracts = {provider: signature_contract(report[provider + '_signatures']) for provider in ['official', 'candidate']}
        report['declared_api'] = {provider: contract[0] for provider, contract in contracts.items()}
        report['synthetic_members'] = {provider: contract[1] for provider, contract in contracts.items()}
        report['jvm_contract_interpretation'] = 'Declared public/protected API is compared. ACC_SYNTHETIC members are recorded separately: class-based lambdas may require extra private-field access bridges compared with invokedynamic lambdas. Fixed Java and Kotlin consumers check the external contracts.'
        nonempty = all(len(contract[0]) == 1 and all(members for members in contract[0].values()) for contract in contracts.values())
        case('declared-api/match', nonempty and contracts['official'][0] == contracts['candidate'][0])
        current = {str(p.relative_to(ROOT)): sha(p) for folder in ['src', 'tests/source_composable_types'] for p in (ROOT / folder).rglob('*') if p.is_file()}
        current.update({name: sha(ROOT / name) for name in ['Cargo.toml', 'Cargo.lock', 'tests/source_composable_types.rs', 'tools/check_source_composable_types.py']})
        case('inputs/unchanged', current == inputs)
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report['status'] = 'pass' if 'correctness_failure' not in report['counts'] else 'fail'
        report['limitations'] = ['Only direct nonnullable nonreceiver nonsuspend zero-argument Unit callbacks are supported. Canonical qualified annotation names and explicit imports/aliases are recognized; wildcard-only type annotation imports and other type annotations remain unsupported. Source composable declarations, nested literal creation, conditional invocation, composite callback shapes and source class stability exports remain explicit unsupported boundaries.']
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
