#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import zipfile
from upstream_jvm import Commands, run_case

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, required=True)
    parser.add_argument('--source-root', type=Path, default=ROOT)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--report', type=Path)
    args = parser.parse_args()
    root = args.source_root.resolve()
    work = args.output.resolve()
    work.mkdir(parents=True, exist_ok=False)
    report_path = args.report.resolve() if args.report else work / 'report.json'
    evidence = report_path.with_suffix('') if args.report else work
    if evidence != work:
        evidence.mkdir(parents=True, exist_ok=False)
    (evidence / 'commands').mkdir()
    commands = Commands(evidence / 'commands', dict(os.environ))
    candidate = args.candidate.resolve()
    jdk = Path(os.environ['JAVA_HOME']) / 'bin'
    lib = root / 'references/kotlinc/lib'
    standard = lib / 'kotlin-stdlib.jar'
    oracle = root / 'references/kotlinc/bin/kotlinc'
    fixtures = root / 'tests/next10_operators'
    app = root / 'benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/QueryTokenizer.kt'
    upstream = [root / 'references/kotlin/compiler/testData/codegen/box' / path for path in [
        'operatorConventions/compareTo/extensionObject.kt', 'functions/recursiveCompareTo.kt']]
    inputs = sorted([path for directory in ['src', 'tests/next10_operators'] for path in (root / directory).rglob('*') if path.is_file()] + [root / 'tests/next10_operators.rs', root / 'tests/inline_analysis.rs', Path(__file__), root / 'tools/upstream_jvm.py', root / 'Cargo.toml', root / 'Cargo.lock'])
    frozen = {str(path): sha(path) for path in [*inputs, *upstream, app, candidate, standard, lib / 'kotlin-compiler.jar', root / 'references/compiler.json', root / 'references/sources.json']}
    report = {'schema_version': 1, 'kind': 'correctness', 'created_at': datetime.now(timezone.utc).isoformat(), 'status': 'running', 'cases': [], 'commands': commands.entries,
              'compiler': {'path': str(candidate), 'sha256': sha(candidate)},
              'reference_compiler': {'version': '2.3.21', 'compiler_jar_sha256': sha(lib / 'kotlin-compiler.jar'), 'stdlib_sha256': sha(standard)},
              'environment': {'JAVA_HOME': os.environ['JAVA_HOME']}, 'frozen_input_hashes': frozen,
              'reproduction_commands': [f'python3 tools/check_next10_operators.py --candidate {candidate} --source-root {root} --output NEW_DIRECTORY'],
              'claims': 'Custom source and compiled unaryPlus/unaryMinus/not resolved through ordinary operator members/extensions; source compareTo operator declarations and comparisons, with compiled member/extension interoperability. Compiled nominal comparison lowering existed in the baseline.',
              'limitations': ['Nullable unary/comparison extension receivers and source covariant overrides requiring JVM bridges remain explicitly unsupported; compiled covariance is tested.', 'The existing ordinary overload-resolution and generic/inline call boundaries apply.', 'Unchanged QueryTokenizer is assessed but candidate compilation remains blocked; official JVM caller evidence is retained. Android runtime and full-application compatibility remain unassessed.'],
              'reference_rules': ['references/kotlin-spec/docs/src/md/kotlin.core/expressions.md:612-619', 'references/kotlin-spec/docs/src/md/kotlin.core/expressions.md:848-870']}
    archive_path = evidence / 'source.zip'
    with zipfile.ZipFile(archive_path, 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.write(path, str(path.relative_to(root)) if path.is_relative_to(root) else path.name)
    def relative(path):
        return str(path.relative_to(root)) if path.is_relative_to(root) else str(path)
    report['evidence'] = {'directory': relative(evidence), 'commands': relative(evidence / 'commands'), 'source_archive': {'path': relative(archive_path), 'sha256': sha(archive_path)}, 'generated_outputs': relative(work)}
    def cp(*paths):
        return os.pathsep.join(map(str, paths))
    def run(label, argv):
        result = commands.run(label, argv, timeout=600)
        if result is None or result.returncode:
            raise RuntimeError(label + ': ' + (result.stderr if result else 'timeout'))
        return result.stdout.strip()
    def case(name, **values):
        report['cases'].append({'id': name, 'outcome': 'pass', **values})
    def compile(provider, sources, dest, dependencies):
        argv = [oracle if provider == 'official' else candidate, *sources, '-cp', cp(*dependencies), '-d', dest]
        argv += ['-jvm-target', '1.8', '-language-version', '2.3', '-api-version', '2.3'] if provider == 'official' else ['--diagnostic-format', 'json']
        run('Compile ' + provider + '/' + dest.name, argv)
    try:
        run('JDK identity', [jdk / 'java', '-version'])
        run('Official compiler identity', [oracle, '-version'])
        dependency = work / 'dependency'
        compile('official', [fixtures / 'dependency/Library.kt'], dependency, [standard])
        outputs = {provider: work / provider for provider in ['official', 'candidate']}
        for provider, dest in outputs.items():
            compile(provider, [fixtures / 'Operators.kt', fixtures / 'Imported.kt'], dest, [dependency, standard])
            case('compile/' + provider)
        java = work / 'java'
        run('Compile fixed Java caller against official output', [jdk / 'javac', '-cp', cp(outputs['official'], dependency, standard), '-d', java, fixtures / 'Check.java'])
        caller_hashes = {str(path): sha(path) for path in java.rglob('*.class')}
        observations = {}
        for provider, dest in outputs.items():
            observations[provider] = run('Run fixed Java caller/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(java, dest, dependency, standard), 'Check'])
            if not observations[provider].startswith('OK '):
                raise RuntimeError(observations[provider])
            case('runtime/' + provider, assertions=int(observations[provider].split()[1]), output=observations[provider])
        if len(set(observations.values())) != 1:
            raise RuntimeError('Runtime observations differ')
        for owner in ['operators.Measure', 'operators.Shifted', 'operators.Plain', 'operators.Origin', 'operators.Middle', 'operators.Descendant', 'operators.BaseFlag', 'operators.DerivedFlag', 'operators.Switch', 'operators.OperatorsKt', 'imported.ImportedKt']:
            signatures = {provider: run('Inspect ABI/' + provider + '/' + owner, [jdk / 'javap', '-public', '-s', '-classpath', dest, owner]) for provider, dest in outputs.items()}
            def normalized(value):
                return sorted(line.strip() for line in value.splitlines() if line.strip() and not line.startswith('Compiled from'))
            if normalized(signatures['official']) != normalized(signatures['candidate']):
                raise RuntimeError('Public ABI differs: ' + owner)
            case('abi/' + owner)
        for against, dependency_output in outputs.items():
            consumer = work / ('consumer-against-' + against)
            compile('official', [fixtures / 'Consumer.kt'], consumer, [dependency_output, dependency, standard])
            for provider, dest in outputs.items():
                observation = run('Run Kotlin consumer compiled against ' + against + '/' + provider, [jdk / 'java', '-Xverify:all', '-cp', cp(consumer, dest, dependency, standard), 'consumer.ConsumerKt'])
                if observation != 'OK':
                    raise RuntimeError('Kotlin consumer: ' + observation)
                case('kotlin_consumer/' + against + '/' + provider, output=observation)
        app_outputs = {provider: work / ('query-' + provider) for provider in outputs}
        compile('official', [app], app_outputs['official'], [standard])
        case('app/compile/official', path=str(app), source_sha256=sha(app), unchanged_source=True)
        app_java = work / 'query-caller'
        run('Compile fixed QueryTokenizer Java caller against official output', [jdk / 'javac', '-cp', cp(app_outputs['official'], standard), '-d', app_java, fixtures / 'QueryTokenizerCaller.java'])
        app_caller_hashes = {str(path): sha(path) for path in app_java.rglob('*.class')}
        observation = run('Run unchanged QueryTokenizer/official', [jdk / 'java', '-Xverify:all', '-cp', cp(app_java, app_outputs['official'], standard), 'QueryTokenizerCaller'])
        if observation != 'OK:13':
            raise RuntimeError('QueryTokenizer official runtime: ' + observation)
        case('app/runtime/official', assertions=13, output=observation)
        checked = commands.run('Assess unchanged QueryTokenizer/candidate', [candidate, app, '-cp', standard, '-d', app_outputs['candidate'], '--diagnostic-format', 'json'], timeout=600)
        if checked is None:
            raise RuntimeError('QueryTokenizer candidate timeout')
        if checked.returncode:
            report['cases'].append({'id': 'app/compile/candidate', 'outcome': 'unsupported_language', 'path': str(app), 'source_sha256': sha(app), 'unchanged_source': True, 'diagnostic': checked.stderr.strip(), 'scope': 'Existing expected-result generic inference/contract refinement boundary; no app-file compilation pass claimed.'})
            report['cases'].append({'id': 'app/runtime/candidate', 'outcome': 'not_run', 'reason': 'Unchanged source compilation rejected; see app/compile/candidate.'})
        else:
            case('app/compile/candidate', path=str(app), source_sha256=sha(app), unchanged_source=True)
            observation = run('Run unchanged QueryTokenizer/candidate', [jdk / 'java', '-Xverify:all', '-cp', cp(app_java, app_outputs['candidate'], standard), 'QueryTokenizerCaller'])
            if observation != 'OK:13':
                raise RuntimeError('QueryTokenizer candidate runtime: ' + observation)
            case('app/runtime/candidate', assertions=13, output=observation)
        if app_caller_hashes != {str(path): sha(path) for path in app_java.rglob('*.class')}:
            raise RuntimeError('Fixed QueryTokenizer caller changed')
        case('app/fixed_caller_integrity', class_hashes=app_caller_hashes)
        invalid = ['UnaryArity', 'NotArity', 'CompareResult', 'CompareInferredResult', 'CompareArity', 'CompareDefault', 'CompareVararg', 'MissingUnaryModifier', 'MissingCompareModifier', 'PrivateUnary', 'PrivateCompare', 'UnrelatedOverload', 'PrivateAncestor', 'MissingInterfaceModifier']
        for name in [*invalid, 'NullableUnary', 'NullableCompare', 'CovariantOverride']:
            path = fixtures / 'boundaries' / (name + '.kt')
            valid = name not in invalid
            official = commands.run('Boundary official/' + name, [oracle, path, '-cp', standard, '-d', work / ('official-' + name)], timeout=600)
            checked = commands.run('Boundary candidate/' + name, [candidate, path, '-cp', standard, '-d', work / ('candidate-' + name), '--diagnostic-format', 'json'], timeout=600)
            category = 'unsupported_language' if valid else 'invalid_kotlin'
            if official is None or (official.returncode == 0) != valid:
                raise RuntimeError(name + ': unexpected official classification')
            if checked is None or checked.returncode == 0 or (name not in ['PrivateUnary', 'PrivateCompare'] and category not in checked.stderr):
                raise RuntimeError(name + ': wrong candidate classification ' + (checked.stderr if checked else 'timeout'))
            if name in invalid[:7] and 'INVALID_OPERATOR_SIGNATURE' not in checked.stderr:
                raise RuntimeError(name + ': wrong signature diagnostic ' + checked.stderr)
            report['cases'].append({'id': 'boundary/' + name, 'outcome': category, 'official_accepts': valid, 'source_sha256': sha(path)})
        context = {'directory': work / 'upstream', 'commands': commands, 'candidate': candidate, 'official': oracle, 'stdlib': standard, 'testlib': lib / 'kotlin-test.jar', **{name: str(jdk / name) for name in ['java', 'javac', 'javap']}}
        for path in upstream:
            result = run_case({'id': path.stem, 'family': 'custom_operators', 'root': 'box', 'tier': 'selected', 'kind': 'box', 'path': str(path), 'sha256': sha(path)}, context)
            result['acceptance_scope'] = 'unchanged_original_assessment'
            report['cases'].append(result)
            if result['outcome'] not in {'pass', 'unsupported_language', 'harness_unsupported'}:
                raise RuntimeError('Upstream case failed: ' + str(result))
        if caller_hashes != {str(path): sha(path) for path in java.rglob('*.class')}:
            raise RuntimeError('Fixed Java caller changed')
        case('fixed_caller_integrity', class_hashes=caller_hashes)
        report['status'] = 'complete'
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
    changed = [path for path, value in frozen.items() if not Path(path).is_file() or sha(path) != value]
    report['input_integrity'] = {'status': 'changed' if changed else 'unchanged', 'changed_paths': changed}
    if changed:
        report['status'] = 'invalidated'
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'status': report['status'], 'counts': report['counts'], 'error': report.get('error'), 'report': str(report_path)}))
    return 0 if report['status'] == 'complete' else 1


if __name__ == '__main__':
    raise SystemExit(main())
