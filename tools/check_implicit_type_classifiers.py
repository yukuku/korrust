#!/usr/bin/env python3
"""Record frozen differential and lexical-access regression tests for implicit classifier types."""
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--skip-html', action='store_true')
    parser.add_argument('--platform-bounds', action='store_true')
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    family = 'java-platform-parameter-bounds' if args.platform_bounds else 'implicit-type-classifiers'
    stem = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-' + family
    evidence = ROOT / 'reports/correctness' / stem
    evidence.mkdir(parents=True)
    files = [
        *sorted((ROOT / 'src').rglob('*.rs')),
        *sorted((ROOT / 'tests/implicit_type_classifiers').iterdir()),
        *sorted((ROOT / 'tests/private_nested').iterdir()),
        *[ROOT / path for path in [
            'Cargo.toml', 'Cargo.lock', 'references/compiler.json', 'references/sources.json',
            'tests/implicit_type_classifiers.rs', 'tests/private_nested.rs',
            'tests/source_generic_diagnostics.rs', 'tools/check_implicit_type_classifiers.py',
            'references/kotlinc/lib/kotlin-stdlib.jar', 'references/kotlinc/lib/kotlin-compiler.jar',
        ]],
    ]
    frozen = {str(path.relative_to(ROOT)): sha(path) for path in files if path.is_file()}
    argv = ['cargo', 'test', '--locked', '--offline', '--test', 'implicit_type_classifiers', '--test', 'private_nested', '--test', 'source_generic_diagnostics', '--', '--nocapture']
    env = dict(os.environ, RUST_MIN_STACK='33554432')
    commands = [argv]
    if args.platform_bounds:
        commands.insert(0, ['cargo', 'test', '--locked', '--offline', '--lib', 'java_platform_bounds_preserve_parameter_upper_bound_flexibility', '--', '--nocapture'])
    cases, records, success = [], [], True
    for index, command in enumerate(commands):
        result = subprocess.run(command, cwd=ROOT, env=env, capture_output=True, text=True, timeout=1800)
        log = evidence / f'cargo-tests-{index}.log'
        log.write_text(result.stdout + result.stderr)
        records.append(dict(argv=command, exit_code=result.returncode, log=str(log.relative_to(ROOT))))
        success = success and result.returncode == 0
        cases.extend(dict(id=name, outcome='pass' if outcome == 'ok' else 'correctness_failure')
                     for name, outcome in re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', result.stdout, re.M))
    changed = [path for path, expected in frozen.items() if not (ROOT / path).is_file() or sha(ROOT / path) != expected]
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    report = dict(
        schema_version=1, kind='correctness', created_at=now.isoformat(),
        status='complete' if success and cases and not changed else 'failed',
        compiler=dict(implementation='Korrust Rust compiler API', source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=dict(version=pin['version'], archive_sha256=pin['archive_sha256'], options=['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']),
        environment=dict(JAVA_HOME=os.environ.get('JAVA_HOME')),
        frozen_input_hashes=frozen,
        input_integrity=dict(status='unchanged' if not changed else 'changed', changed_paths=changed),
        reproduction_command=['python3', 'tools/check_implicit_type_classifiers.py'] + (['--platform-bounds'] if args.platform_bounds else []),
        commands=records,
        scope=('Kotlin type parameters with nullable upper bounds satisfy compatible Java platform upper bounds, while nonnull Kotlin bounds and incompatible Java bounds still reject. ' if args.platform_bounds else '') + 'Implicit same-package/default classifier lookup in body type syntax and direct nested Java/Kotlin classifier lookup through declared member metadata, including generic Java nested type signatures.',
        limitations=['Inherited Java member-class lookup and protected Java nested classifiers remain unsupported.', 'No Android, app-build or performance claim; independent fixtures use actual javac and pinned Kotlin compiler dependencies.'],
        differential_evidence=dict(
            test='implicit_body_and_nested_signature_classifiers_preserve_interop_and_visibility',
            oracle_and_candidate_inputs='tests/implicit_type_classifiers/Use.kt and Alias.kt with identical real javac/Kotlin-built dependencies',
            runtime='One Java caller compiled against official output runs unchanged against both outputs using -Xverify:all.',
            signatures='javap public signatures and descriptors for Lookup, Observer, Holder, and AliasKt compare exactly.',
            kotlin_consumers='Pinned Kotlin consumers with unconstrained and nullable arguments compile against both providers and run with the fixed Java caller.',
            boundaries='Private/protected/missing/fake Java members, inaccessible Java and Kotlin outers, fake top-level Kotlin dollar names, and corrupt outer/nested classfiles.',
        ),
        cases=cases, counts=dict(Counter(case['outcome'] for case in cases)),
    )
    path = evidence.with_suffix('.json')
    path.write_text(json.dumps(report, indent=2) + '\n')
    if not args.skip_html:
        subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True)
    print(path.relative_to(ROOT))
    raise SystemExit(0 if report['status'] == 'complete' else 1)


if __name__ == '__main__':
    main()
