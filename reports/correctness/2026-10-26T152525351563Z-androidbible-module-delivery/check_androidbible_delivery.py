#!/usr/bin/env python3
"""Revalidate three existing AndroidBible modules with one frozen compiler."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

from check_androidbible_module import ROOT, relative, sha

MODULES = ['AlkitabModel', 'AlkitabIo', 'AlkitabYes2']
TOOLS = ['build_androidbible.py', 'classpath_policy.py', 'check_androidbible_io.py',
         'check_androidbible_module.py', 'check_androidbible_model_rebuild.py',
         'check_androidbible_delivery.py']


def gradle_environment():
    environment = os.environ.copy()
    environment.setdefault('GRADLE_USER_HOME', '/workspace/korrust-setup/gradle-home')
    environment.setdefault('ANDROID_USER_HOME', '/workspace/korrust-setup/android-user')
    additions = []
    for key, value in [('javax.net.ssl.trustStore', '/etc/ssl/certs/java/cacerts'),
                       ('https.proxyHost', 'proxy'), ('https.proxyPort', '8080'),
                       ('http.proxyHost', 'proxy'), ('http.proxyPort', '8080')]:
        if '-D' + key + '=' not in environment.get('JAVA_TOOL_OPTIONS', ''):
            additions.append('-D' + key + '=' + value)
    environment['JAVA_TOOL_OPTIONS'] = ' '.join([environment.get('JAVA_TOOL_OPTIONS', ''), *additions]).strip()
    return environment


def report_from_stdout(stdout):
    matches = re.findall(r'^(?:Report: )?([^\n]*reports/correctness/[^\n ]+\.json)(?: (?:passed|failed) \{[^\n]*\})?$', stdout, re.MULTILINE)
    if len(matches) != 1:
        raise ValueError('Child workflow did not identify exactly one correctness report')
    path = Path(matches[0])
    return path if path.is_absolute() else ROOT / path


def merge_resolved_models(selected_path, capture_path, destination):
    selected = json.loads(Path(selected_path).read_text())
    captured = json.loads(Path(capture_path).read_text()) if capture_path is not None else None
    modules = dict(selected["modules"])
    if captured is not None:
        overlap = modules.keys() & captured["modules"].keys()
        if overlap:
            raise ValueError("Resolved source ownership overlaps: " + ", ".join(sorted(overlap)))
        modules.update(captured["modules"])
    paths = [selected_path] + ([capture_path] if capture_path is not None else [])
    model = dict(schema_version=2, kind="androidbible_resolved_inputs",
                 created_at=datetime.now(timezone.utc).isoformat(),
                 status="partial" if captured is None or captured["status"] != "passed" else "captured",
                 modules=modules,
                 source_models=[dict(path=relative(path), sha256=sha(path)) for path in paths],
                 source_ownership="Selected library production sources use the recorded frozen candidate; Alkitab capture entry, when present, is input capture only. No app-owned source is official-compiled as an assessment dependency.",
                 limitations="Incomplete source collections, required generated outputs, setup failures and compiler plugins remain explicit module/task data; the assessment must not drop them or count them as compiler passes.")
    Path(destination).write_text(json.dumps(model, indent=2) + "\n")
    return model


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--capture-app-model', action='store_true', help='Also capture partial plain Alkitab inputs without compiling the app, for the separate corpus assessment')
    parser.add_argument('--frozen-root', type=Path, required=True)
    parser.add_argument('--gradle', type=Path, default=Path('/workspace/korrust-setup/gradle-9.7.1/bin/gradle'))
    parser.add_argument('--sdk', type=Path, default=Path('/workspace/korrust-setup/android-sdk'))
    parser.add_argument('--d8-jar', type=Path, default=Path('/workspace/korrust-setup/gradle-home/caches/modules-2/files-2.1/com.android.tools.build/builder/9.2.1/e8ef7f9072cd990708591c54128fd6c6dc42ae7d/builder-9.2.1.jar'))
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    ident = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-androidbible-module-delivery'
    evidence = ROOT / 'reports/correctness' / ident
    evidence.mkdir(parents=True)
    report_path = evidence.with_suffix('.json')
    frozen = args.frozen_root.resolve()
    compiler = frozen / 'target/release/korrust'
    work = frozen / 'target' / ident
    artifacts = work.with_name(work.name + '-artifacts')
    helper = frozen / 'tools/build_androidbible.py'
    report = dict(schema_version=1, kind='correctness', created_at=now.isoformat(), status='running',
                  compiler={}, reference_compiler={}, inputs={}, commands=[], evidence=[], cases=[],
                  selection='Task29 batch-integration revalidation of previously demonstrated AlkitabModel/AlkitabIo/AlkitabYes2 coverage. All selected production Kotlin uses one Korrust batch per module/variant without fallback; Java, original unit-test Kotlin and external modules use normal Gradle toolchains. Real Gradle release AAR/test graphs, fixed official-compiled Java/test bytecode, official Kotlin consumers, ABI, JVM verification and standalone actual AGP embedded D8 are verified separately. A fresh controlled Model declaration-removal/change build must match independent clean candidate/official output with this same frozen binary. No new module, complete-app, Android-device or performance claim.',
                  reproduction_commands=[[sys.executable, *sys.argv]])
    frozen_tools = {}

    def archive(path, name):
        destination = evidence / name
        shutil.copy2(path, destination)
        report['evidence'].append(relative(destination))
        return destination

    def run(label, command, *, environment=None):
        command = list(map(str, command))
        print(label, flush=True)
        result = subprocess.run(command, cwd=ROOT, env=environment, capture_output=True, text=True)
        entry = dict(label=label, argv=command, cwd=str(ROOT), exit_code=result.returncode)
        index = len(report['commands'])
        for stream, text in [('stdout', result.stdout), ('stderr', result.stderr)]:
            path = evidence / f'{index:03d}-{stream}.txt'
            path.write_text(text)
            entry[stream] = relative(path)
            report['evidence'].append(relative(path))
        report['commands'].append(entry)
        return result

    def collect(path, prefix):
        child = json.loads(path.read_text())
        report['evidence'].append(relative(path))
        report['inputs'][prefix + '_report'] = dict(path=relative(path), sha256=sha(path))
        if child.get('compiler', {}).get('binary_sha256') != report['compiler']['binary_sha256']:
            raise AssertionError('Child workflow used a different compiler: ' + str(path))
        for case in child['cases']:
            entry = dict(case)
            entry['id'] = prefix + '/' + case['id']
            entry['evidence_report'] = relative(path)
            report['cases'].append(entry)
        if child['status'] not in ('completed', 'passed'):
            raise AssertionError('Child workflow failed: ' + str(path))
        return child

    try:
        for name in TOOLS:
            current, copied = ROOT / 'tools' / name, frozen / 'tools' / name
            if not copied.is_file() or sha(current) != sha(copied):
                raise AssertionError('Delivery tools must match the frozen snapshot: ' + name)
            frozen_tools[relative(current)] = sha(current)
            archive(current, name)
        if not compiler.is_file():
            raise FileNotFoundError('Frozen release compiler missing: ' + str(compiler))
        for dependency in [args.gradle, args.sdk / 'platforms/android-36/android.jar', args.d8_jar]:
            if not dependency.is_file():
                raise FileNotFoundError('Required module environment input missing: ' + str(dependency))
        fixture_files = [path for path in (ROOT / 'tests/fixtures').rglob('*') if path.is_file()]
        frozen_fixtures = {}
        for path in fixture_files:
            copied = frozen / path.relative_to(ROOT)
            if not copied.is_file() or sha(path) != sha(copied):
                raise AssertionError('Delivery fixture differs from frozen snapshot: ' + str(path))
            frozen_fixtures[relative(path)] = sha(path)
        source_paths = [frozen / 'Cargo.toml', frozen / 'Cargo.lock', *sorted((frozen / 'src').rglob('*.rs'))]
        compiler_sources = {relative(path): sha(path) for path in source_paths}
        report['compiler'] = dict(path=str(compiler), binary_sha256=sha(compiler), source_files_sha256=compiler_sources)
        report['inputs'] = dict(frozen_root=str(frozen), frozen_tools_sha256=frozen_tools, frozen_fixture_files_sha256=frozen_fixtures,
                                module_set=MODULES, work_dir=str(work), artifact_dir=str(artifacts),
                                gradle=str(args.gradle.resolve()), sdk=str(args.sdk.resolve()), d8_jar_sha256=sha(args.d8_jar))
        for mode in ['official', 'korrust']:
            result = run('Clean unchanged Gradle module set ' + mode,
                         [sys.executable, helper, '--mode', mode, '--modules', *MODULES,
                          '--compiler', compiler, '--work-dir', work, '--artifact-dir', artifacts,
                          '--gradle', args.gradle.resolve(), '--sdk', args.sdk.resolve()], environment=gradle_environment())
            build_path = artifacts / mode / 'build.json'
            if build_path.is_file():
                build = json.loads(build_path.read_text())
                archive(build_path, mode + '-build.json')
                archive(build['log'], mode + '-gradle.log')
                report['inputs'][mode + '_build'] = dict(path=relative(build_path), sha256=sha(build_path))
            else:
                build = {}
            if result.returncode or build.get('status') != 'passed':
                report['cases'].append(dict(id='build/' + mode, outcome=build.get('failure_category', 'build_failure'), exit_code=result.returncode))
                raise RuntimeError('Clean module build failed: ' + mode)
        verification = run('Verify module ABI, original tests, consumers and standalone AGP D8',
                           [sys.executable, ROOT / 'tools/check_androidbible_io.py',
                            '--official-json', artifacts / 'official/build.json',
                            '--candidate-json', artifacts / 'korrust/build.json',
                            '--compiler-root', frozen, '--build-tool', helper,
                            '--sdk', args.sdk.resolve(), '--d8-jar', args.d8_jar.resolve()])
        child = collect(report_from_stdout(verification.stdout), 'modules')
        report['reference_compiler'] = child['reference_compiler']
        if verification.returncode:
            raise RuntimeError('Module verification workflow failed')
        rebuild = run('Fresh controlled declaration removal/change versus independent clean builds',
                      [sys.executable, ROOT / 'tools/check_androidbible_model_rebuild.py',
                       '--frozen-root', frozen, '--gradle', args.gradle.resolve(), '--sdk', args.sdk.resolve()])
        collect(report_from_stdout(rebuild.stdout), 'controlled_edit')
        if rebuild.returncode:
            raise RuntimeError('Controlled-edit workflow failed')
        captured_path = None
        if args.capture_app_model:
            captured = run('Capture partial Alkitab inputs without compiling production sources',
                           [sys.executable, helper, '--capture-only', '--modules', 'Alkitab',
                            '--work-dir', work, '--artifact-dir', artifacts, '--gradle', args.gradle.resolve(),
                            '--sdk', args.sdk.resolve()], environment=gradle_environment())
            candidate_path = artifacts / 'capture/build.json'
            if candidate_path.is_file():
                captured_path = candidate_path
                data = json.loads(candidate_path.read_text())
                archive(candidate_path, 'app-capture-build.json')
                archive(data['log'], 'app-capture-gradle.log')
                module_model = artifacts / 'capture/modules/Alkitab/gradle-model.json'
                if module_model.is_file():
                    archive(module_model, 'app-capture-gradle-model.json')
                report['inputs']['app_capture'] = dict(status=data['status'], failure_category=data.get('failure_category'),
                                                      exit_code=captured.returncode, scope='Separate diagnostic input capture; not part of the selected module acceptance count.')
            else:
                report['inputs']['app_capture'] = dict(status='setup_failure', exit_code=captured.returncode, scope='App model was not emitted; selected module proof is unaffected.')
        merged_path = evidence / 'resolved-module-inputs.json'
        merge_resolved_models(artifacts / 'korrust/build.json', captured_path, merged_path)
        report['evidence'].append(relative(merged_path))
        report['inputs']['merged_resolved_input_model'] = dict(path=relative(merged_path), sha256=sha(merged_path))
        is_frozen = (sha(compiler) == report['compiler']['binary_sha256']
                     and all(sha(ROOT / path) == checksum for path, checksum in compiler_sources.items())
                     and all(sha(ROOT / path) == checksum for path, checksum in frozen_tools.items())
                     and all(sha(ROOT / path) == checksum for path, checksum in frozen_fixtures.items()))
        report['cases'].append(dict(id='frozen_delivery_inputs', outcome='pass' if is_frozen else 'fail'))
        report['status'] = 'completed' if is_frozen else 'failed'
    except Exception as error:
        report['status'] = 'failed'
        report['cases'].append(dict(id='harness_failure', outcome='fail', message=str(error)))
        print(str(error), file=sys.stderr)
    report['counts'] = dict(Counter(case['outcome'] for case in report['cases']))
    report['finished_at'] = datetime.now(timezone.utc).isoformat()
    report_path.write_text(json.dumps(report, indent=2) + '\n')
    print('Report: ' + relative(report_path), flush=True)
    return 0 if report['status'] == 'completed' else 1


if __name__ == '__main__':
    sys.exit(main())
