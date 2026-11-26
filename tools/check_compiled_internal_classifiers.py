#!/usr/bin/env python3
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
FIXTURES = ROOT / 'tests/compiled_internal_classifiers'
APP = ROOT / 'benchmarks/sources/androidbible/Alkitab/src/test/java/yuku/alkitab/base/appwidget'


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path).absolute()
    return str(path.relative_to(ROOT)) if path.is_relative_to(ROOT) else str(path)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--candidate', type=Path, default=ROOT / 'target/release/korrust')
    parser.add_argument('--baseline', type=Path)
    parser.add_argument('--app-classpath')
    parser.add_argument('--app-friend-path')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--evidence-directory', type=Path)
    args = parser.parse_args()
    created = datetime.now(timezone.utc)
    stamp = created.strftime('%Y-%m-%dT%H%M%S%fZ') + '-compiled-internal-classifiers'
    directory = (args.output or ROOT / 'target/checks' / stamp).absolute()
    evidence = (args.evidence_directory or ROOT / 'reports/correctness' / stamp).absolute()
    directory.mkdir(parents=True, exist_ok=False)
    evidence.mkdir(parents=True, exist_ok=False)
    candidate = directory / 'korrust-frozen'
    shutil.copy2(args.candidate.resolve(), candidate)
    official = ROOT / 'references/kotlinc/bin/kotlinc'
    standard = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
    java = Path(os.environ['JAVA_HOME']) / 'bin'
    options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
    app_sources = [APP / (name + '.kt') for name in ['DailyVerseConfigurationTest', 'DailyVerseConfigurationScreenTest']] if args.app_classpath else []
    inputs = [*sorted((ROOT / 'src').rglob('*.rs')), *sorted(FIXTURES.iterdir()), ROOT / 'tests/compiled_internal_classifiers.rs', Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', *app_sources]
    dependencies = [Path(path) for path in args.app_classpath.split(os.pathsep) if Path(path).is_file()] if args.app_classpath else []
    frozen = {relative(path): sha(path) for path in [*inputs, candidate, standard, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar', *dependencies, *([args.baseline] if args.baseline else [])]}
    with zipfile.ZipFile(evidence / 'inputs.zip', 'w', zipfile.ZIP_DEFLATED) as archive:
        for path in inputs:
            archive.writestr(relative(path), path.read_bytes())
    report_path = evidence.with_suffix('.json')
    report = dict(schema_version=1, kind='correctness', created_at=created.isoformat(), status='running',
        compiler=dict(path=relative(candidate), sha256=sha(candidate), source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=json.loads((ROOT / 'references/compiler.json').read_text()),
        frozen_input_hashes=frozen, input_bundle=dict(path=relative(evidence / 'inputs.zip'), sha256=sha(evidence / 'inputs.zip')),
        scope='Compiled internal class, data-class, interface, object and enum metadata; exact directory/JAR friend origins; explicit/wildcard/same-package references; named/default calls; public class implementation of internal interfaces; visibility, shadowing and public exposure validation; optional real AndroidBible binary and unchanged test probes',
        limitations=['Existing classifier shape, member signature and language boundaries still apply', 'A later rejection of an unchanged app file is progress evidence, not a pass or application compatibility claim'],
        effective_options=dict(language_version='2.3', api_version='2.3', jvm_target='1.8'), cases=[], commands=[],
        reproduction_command=['python3', relative(__file__), '--candidate', relative(args.candidate)])
    for flag, value in [('--baseline', args.baseline), ('--app-classpath', args.app_classpath), ('--app-friend-path', args.app_friend_path)]:
        if value:
            report['reproduction_command'].extend([flag, str(value)])

    def save():
        report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
        report_path.write_text(json.dumps(report, indent=2) + '\n')

    def run(label, command, accepted=(0,)):
        command = [str(part) for part in command]
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=360)
        log = evidence / f'{len(report["commands"]):03d}.txt'
        log.write_text('$ ' + ' '.join(command) + '\n' + result.stdout + result.stderr)
        report['commands'].append(dict(label=label, argv=command, exit_code=result.returncode, log=relative(log)))
        save()
        assert result.returncode in accepted, label + ': ' + result.stdout + result.stderr
        return result

    def case(name, operation):
        try:
            detail = operation()
            outcome = detail.pop('outcome', 'pass') if isinstance(detail, dict) else 'pass'
            report['cases'].append(dict(id=name, outcome=outcome, detail=detail))
        except Exception as error:
            report['cases'].append(dict(id=name, outcome='correctness_failure', detail=str(error)))
        save()

    def check(version, archive):
        work = directory / (version + ('-jar' if archive else '-directory'))
        work.mkdir()
        dependency = work / 'dependency'
        run('official dependency ' + str(work.name), [official, FIXTURES / 'Library.kt', '-module-name', 'friends_production', '-Xmetadata-version=' + version, '-d', dependency, *options])
        if archive:
            packed = work / 'dependency.jar'
            with zipfile.ZipFile(packed, 'w', zipfile.ZIP_DEFLATED) as jar:
                for path in sorted(dependency.rglob('*')):
                    if path.is_file():
                        jar.write(path, str(path.relative_to(dependency)))
            dependency = packed
        cp = os.pathsep.join(map(str, [dependency, standard]))
        providers = []
        for name, compiler in [('official', official), ('candidate', candidate)]:
            models, public = work / name, work / (name + '-public')
            sources = [FIXTURES / (source + '.kt') for source in ['Consumer', 'Explicit', 'SamePackage', 'AllowedSignatures', 'SuperInterface', 'PrivateAlias']]
            run(name + ' friend declarations', [compiler, *sources, '-cp', cp, '-d', models, *(['-Xfriend-paths=' + str(dependency), *options] if name == 'official' else ['--friend-paths', dependency])])
            run(name + ' public inherited members and lexical shadow', [compiler, FIXTURES / 'PublicInheritance.kt', FIXTURES / 'Shadow.kt', '-cp', cp, '-d', public, *(options if name == 'official' else [])])
            providers.append((name, models, public))
        callers = work / 'callers'
        run('fixed Java caller against official', [java / 'javac', '-cp', os.pathsep.join(map(str, [providers[0][1], providers[0][2], dependency, standard])), '-d', callers, FIXTURES / 'Calls.java'])
        for name, models, public in providers:
            result = run(name + ' independent JVM assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [callers, models, public, dependency, standard])), 'friends.consumer.Calls'])
            assert result.stdout == 'internal-classifiers-ok\n', result.stdout
        signatures = []
        for name, models, public in providers:
            listing = run(name + ' public JVM descriptors', [java / 'javap', '-public', '-s', '-classpath', os.pathsep.join(map(str, [models, public, dependency, standard])), 'friends.consumer.ConsumerKt', 'friends.consumer.ExplicitKt', 'friends.library.SamePackageKt', 'friends.consumer.PublicCounter', 'friends.consumer.PublicInheritanceKt', 'friends.library.ShadowKt'])
            lines = listing.stdout.splitlines()
            signatures.append([(lines[index - 1].strip().split('(')[0].split()[-1], line.strip()) for index, line in enumerate(lines) if line.strip().startswith('descriptor:')])
        assert signatures[0] == signatures[1], signatures
        return dict(dependency=str(dependency), caller_sha256={str(path.relative_to(callers)): sha(path) for path in callers.rglob('*.class')}, public_jvm_descriptors=signatures[0], assertions='Friend calls sum to 41, alias 9, same-package 8; public class implements internal interface; public inherited default member returns 31; inaccessible classifier does not shadow local variable (12)')

    for version, archive in [('1.7.0', False), ('2.3.0', False), ('2.3.0', True)]:
        case('friend_metadata_' + version + ('_jar' if archive else '_directory'), lambda version=version, archive=archive: check(version, archive))

    dependency = directory / '2.3.0-directory/dependency'
    cp = os.pathsep.join(map(str, [dependency, standard]))
    def reject(source, friend, label, wrong=False):
        unrelated = directory / 'unrelated'
        unrelated.mkdir(exist_ok=True)
        for name, compiler in [('official', official), ('candidate', candidate)]:
            extra = (['-Xfriend-paths=' + str(unrelated if wrong else dependency)] if name == 'official' else ['--friend-paths', unrelated if wrong else dependency]) if friend else []
            result = run(name + ' reject ' + label, [compiler, FIXTURES / (source + '.kt'), '-cp', cp, '-module-name' if name == 'official' else '--module-name', 'friends_production', '-d', directory / (name + '-' + label), *extra, *(options if name == 'official' else ['--diagnostic-format', 'json'])], accepted=(1,))
            if name == 'candidate':
                diagnostic = json.loads(result.stderr)
                assert diagnostic['category'] == 'invalid_kotlin', diagnostic
        return dict(outcome='invalid_rejected', diagnostic=diagnostic)
    for source in ['Explicit', 'Consumer', 'SamePackage', 'NoFriendType']:
        case('no_friend_' + source, lambda source=source: reject(source, False, 'no-friend-' + source))
    case('unrelated_friend_origin', lambda: reject('Explicit', True, 'unrelated-friend', True))
    for source in ['PrivateFriend', *[path.stem for path in sorted(FIXTURES.glob('Exposed*.kt'))]]:
        case('friend_reject_' + source, lambda source=source: reject(source, True, source))

    if args.app_classpath and args.app_friend_path:
        def actual():
            outputs = []
            for name, compiler in [('official', official), ('candidate', candidate)]:
                output = directory / ('actual-' + name)
                extra = ['-Xfriend-paths=' + args.app_friend_path, *options] if name == 'official' else ['--friend-paths', args.app_friend_path]
                run(name + ' actual production internal data class', [compiler, FIXTURES / 'ActualConfiguration.kt', '-cp', args.app_classpath, '-d', output, *extra])
                outputs.append(output)
            callers = directory / 'actual-callers'
            run('fixed actual binary caller against official', [java / 'javac', '-cp', os.pathsep.join([str(outputs[0]), args.app_classpath]), '-d', callers, FIXTURES / 'ActualCalls.java'])
            for name, output in zip(['official', 'candidate'], outputs):
                result = run(name + ' actual production class JVM assertions', [java / 'java', '-Xverify:all', '-cp', os.pathsep.join([str(callers), str(output), args.app_classpath]), 'yuku.alkitab.base.appwidget.ActualCalls'])
                assert result.stdout == 'actual-internal-configuration-ok\n', result.stdout
            return 'Real unmodified DailyVerseConfiguration binary: named constructor defaults, copy defaults, getters and computed Boolean property yield 25'
        case('actual_production_internal_data_class', actual)
        if args.baseline:
            for source in app_sources:
                def advance(source=source):
                    results = []
                    for name, compiler in [('baseline', args.baseline), ('candidate', candidate)]:
                        result = run(name + ' unchanged ' + source.name, [compiler, source, '-cp', args.app_classpath, '--friend-paths', args.app_friend_path, '-d', directory / (name + '-' + source.stem), '--diagnostic-format', 'json'], accepted=(0, 1))
                        results.append(result)
                    assert results[0].returncode == 1 and 'Same-package class is inaccessible or unsupported' in results[0].stderr, results[0].stderr
                    assert 'Same-package class is inaccessible or unsupported' not in results[1].stderr, results[1].stderr
                    return dict(outcome='compiled_unchanged' if results[1].returncode == 0 else 'advanced_to_later_rejection', source=relative(source), source_sha256=sha(source), before=results[0].stderr, after=results[1].stderr)
                case('unchanged_' + source.stem, advance)
    changed = [name for name, digest in frozen.items() if sha(ROOT / name) != digest]
    report['cases'].append(dict(id='frozen_inputs', outcome='correctness_failure' if changed else 'pass', changed=changed))
    report['status'] = 'failed' if any(case['outcome'] == 'correctness_failure' for case in report['cases']) else 'complete'
    save()
    print(json.dumps(dict(report=relative(report_path), status=report['status'], counts=report['counts'])))
    return report['status'] != 'complete'


if __name__ == '__main__':
    raise SystemExit(main())
