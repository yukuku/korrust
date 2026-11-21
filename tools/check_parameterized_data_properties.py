#!/usr/bin/env python3
"""Freeze differential evidence for parameterized reference properties in data classes."""
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
    now = datetime.now(timezone.utc)
    stem = now.strftime('%Y-%m-%dT%H%M%S%fZ') + '-parameterized-data-properties'
    evidence = ROOT / 'reports/correctness' / stem
    evidence.mkdir(parents=True)
    files = [
        *sorted((ROOT / 'src').rglob('*.rs')),
        *sorted((ROOT / 'tests/parameterized_data_properties').iterdir()),
        *[ROOT / path for path in [
            'tests/parameterized_data_properties.rs', 'tests/source_generic_diagnostics.rs',
            'tools/check_parameterized_data_properties.py', 'Cargo.toml', 'Cargo.lock',
            'references/compiler.json', 'references/sources.json',
            'references/kotlinc/lib/kotlin-stdlib.jar', 'references/kotlinc/lib/kotlin-compiler.jar',
            'references/kotlin-spec/docs/src/md/kotlin.core/declarations.md',
            'references/kotlin/compiler/ir/ir.tree/src/org/jetbrains/kotlin/ir/util/DataClassMembersGenerator.kt',
        ]],
    ]
    frozen = {str(path.relative_to(ROOT)): sha(path) for path in files}
    argv = ['cargo', 'test', '--locked', '--offline', '--test', 'parameterized_data_properties', '--test', 'source_generic_diagnostics', '--', '--nocapture']
    result = subprocess.run(argv, cwd=ROOT, env=dict(os.environ, RUST_MIN_STACK='33554432'), capture_output=True, text=True, timeout=1800)
    log = evidence / 'cargo-tests.log'
    log.write_text(result.stdout + result.stderr)
    cases = []
    for name, outcome in re.findall(r'^test (\S+) \.\.\. (ok|FAILED)$', result.stdout, re.M):
        status = 'pass' if outcome == 'ok' else 'correctness_failure'
        if outcome == 'ok' and name == 'direct_arrays_remain_explicitly_unsupported':
            status = 'unsupported_language'
        elif outcome == 'ok' and name == 'invalid_parameterized_data_properties_do_not_return_artifacts':
            status = 'invalid_rejected'
        cases.append(dict(id=name, outcome=status))
    changed = [path for path, expected in frozen.items() if not (ROOT / path).is_file() or sha(ROOT / path) != expected]
    pin = json.loads((ROOT / 'references/compiler.json').read_text())
    report = dict(
        schema_version=1, kind='correctness', created_at=now.isoformat(),
        status='complete' if result.returncode == 0 and cases and not changed else 'failed',
        compiler=dict(implementation='Korrust Rust compiler API', source_revision=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()),
        reference_compiler=dict(version=pin['version'], archive_sha256=pin['archive_sha256'], options=['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']),
        environment=dict(JAVA_HOME=os.environ.get('JAVA_HOME')),
        frozen_input_hashes=frozen,
        input_integrity=dict(status='unchanged' if not changed else 'changed', changed_paths=changed),
        reproduction_command=['python3', 'tools/check_parameterized_data_properties.py'],
        commands=[dict(argv=argv, exit_code=result.returncode, log=str(log.relative_to(ROOT)))],
        scope='Validated parameterized JVM reference properties in otherwise-supported nongeneric data classes: Set, Map with nullable values, ordinary generic references, and nullable parameterized properties. Public constructors/getters/components/copy signatures, shallow copy/defaults, equality, hashCode, toString, null checks, and Kotlin metadata consumers are checked.',
        differential_evidence=dict(
            inputs='tests/parameterized_data_properties/Models.kt, Consumer.kt and Driver.java',
            runtime='One Java caller compiled against the official provider and official Kotlin consumer runs unchanged in all four official/candidate provider/consumer combinations with -Xverify:all.',
            signatures='Sorted public member declarations plus associated JVM descriptors compare exactly; declaration order is not an ABI requirement.',
            independent_assertions='Collection structural equality and custom generic-object identity, equality self fast path, delegated hashCode calls, property hash arithmetic, exact toString, component reference identity, shallow copy/default copy, empty collections, nullable property/value, and constructor/copy null rejection.',
            boundaries='Both compilers reject invalid generic arity/bounds; the oracle accepts direct arrays while Korrust explicitly reports DATA_PROPERTY_SHAPE_UNSUPPORTED.',
        ),
        limitations=['Existing declaration/mutability restrictions, generic data-class declarations and array/function property restrictions remain.', 'Private backing-field generic signatures are not compared.', 'No unchanged app-file, Android build or performance claim in this independent differential report.'],
        cases=cases, counts=dict(Counter(case['outcome'] for case in cases)),
    )
    path = evidence.with_suffix('.json')
    path.write_text(json.dumps(report, indent=2) + '\n')
    subprocess.run(['python3', 'tools/reports.py'], cwd=ROOT, check=True)
    print(path.relative_to(ROOT))
    raise SystemExit(0 if report['status'] == 'complete' else 1)


if __name__ == '__main__':
    main()
