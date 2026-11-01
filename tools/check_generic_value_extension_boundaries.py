from pathlib import Path
from datetime import datetime, timezone
from collections import Counter
import subprocess, json, os, shutil, hashlib, argparse
parser = argparse.ArgumentParser()
for name in ['root', 'candidate', 'fixtures', 'output']:
    parser.add_argument('--' + name, required=True, type=Path)
args = parser.parse_args()
root = args.root.resolve()
f = args.fixtures.resolve()
out = args.output.resolve()
out.mkdir(exist_ok=False)
env = os.environ.copy()
env['JAVA_HOME'] = os.environ['JAVA_HOME']
jdk = Path(env['JAVA_HOME']) / 'bin'
oracle = root / 'references/kotlinc/bin/kotlinc'
std = root / 'references/kotlinc/lib/kotlin-stdlib.jar'
candidate = out / 'korrust'
shutil.copy2(args.candidate.resolve(), candidate)
cmds = []
cases = []
opts = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
h = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
paths = [*root.glob('src/**/*.rs'), *[p for p in f.rglob('*') if p.is_file()], Path(__file__), candidate, std, root / 'Cargo.toml', root / 'Cargo.lock', root / 'references/compiler.json', root / 'references/sources.json', root / 'references/kotlinc/lib/kotlin-compiler.jar']
fixed_callers = {}
frozen = {str(p.relative_to(root)): h(p) for p in paths}

def save(status='running'):
    (out / 'report.json').write_text(json.dumps(dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status=status, counts=dict(Counter((c['outcome'] for c in cases))), commands=cmds, cases=cases, frozen_input_hashes=frozen), indent=2) + '\n')

def cp(*p):
    return os.pathsep.join(map(str, p))

def run(label, argv, expected=0):
    p = subprocess.run(list(map(str, argv)), cwd=root, env=env, capture_output=True, text=True, timeout=180)
    log = out / (str(len(cmds)) + '.log')
    log.write_text(p.stdout + p.stderr)
    cmds.append(dict(label=label, argv=list(map(str, argv)), exit_code=p.returncode, log=str(log.relative_to(root))))
    save()
    assert p.returncode == expected, (label, p.stdout, p.stderr)
    if Path(argv[0]).name == 'javac' and '-d' in argv:
        directory = Path(argv[argv.index('-d') + 1])
        if directory.name.endswith('caller'):
            fixed_callers.update({str(p.relative_to(root)): h(p) for p in directory.rglob('*.class')})
    return p
try:
    run('oracle Packet dependency', [oracle, f / 'Dependency.kt', '-cp', std, '-d', out / 'dependency', *opts])
    run('oracle extension overloads', [oracle, f / 'FallbackDependency.kt', f / 'HigherDependency.kt', f / 'LowerDependency.kt', f / 'LocalDependency.kt', '-cp', cp(std, out / 'dependency'), '-d', out / 'extensions', '-Xallow-kotlin-package', *opts])
    dependency = cp(std, out / 'dependency', out / 'extensions')
    run('oracle fallback probe', [oracle, f / 'Fallback.kt', '-cp', dependency, '-d', out / 'fallback-official', *opts])
    run('official-only fallback Java caller', [jdk / 'javac', '-cp', cp(std, out / 'dependency', out / 'fallback-official'), '-d', out / 'fallback-caller', f / 'FallbackCalls.java'])
    p = run('oracle chooses Packet overload', [jdk / 'java', '-Xverify:all', '-cp', cp(std, out / 'dependency', out / 'extensions', out / 'fallback-official', out / 'fallback-caller'), 'FallbackCalls'])
    assert p.stdout == 'OK\n'
    cases.append(dict(id='oracle_prefers_generic_value_receiver', outcome='pass'))
    p = run('candidate rejects applicable unsupported overload', [candidate, f / 'Fallback.kt', '-cp', dependency, '-d', out / 'fallback-candidate', '--diagnostic-format', 'json'], 1)
    d = [json.loads(x) for x in p.stderr.splitlines() if x.startswith('{')]
    assert d[0]['category'] == 'unsupported_language' and d[0]['code'] == 'GENERIC_VALUE_EXTENSION_ABI_UNSUPPORTED', d
    assert not list((out / 'fallback-candidate').rglob('*.class'))
    cases.append(dict(id='no_Any_fallback_emission', outcome='unsupported_language', diagnostics=d))
    for (name, binary) in [('official', oracle), ('candidate', candidate)]:
        run(name + ' earlier import priority', [binary, f / 'Priority.kt', f / 'LocalPriority.kt', '-cp', dependency, '-d', out / ('priority-' + name), *opts] if name == 'official' else [binary, f / 'Priority.kt', f / 'LocalPriority.kt', '-cp', dependency, '-d', out / ('priority-' + name)])
    run('fixed Java priority caller official only', [jdk / 'javac', '-cp', cp(std, out / 'dependency', out / 'priority-official'), '-d', out / 'priority-caller', f / 'PriorityCalls.java'])
    for name in ['official', 'candidate']:
        p = run('earlier import executes ' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(std, out / 'dependency', out / 'extensions', out / ('priority-' + name), out / 'priority-caller'), 'PriorityCalls'])
        assert p.stdout == 'OK\n'
        cases.append(dict(id='earlier_import_' + name, outcome='pass'))
    run('oracle non-infix invalid', [oracle, f / 'InvalidInfix.kt', '-cp', dependency, '-d', out / 'non-infix-official', *opts], 1)
    p = run('candidate non-infix invalid', [candidate, f / 'InvalidInfix.kt', '-cp', dependency, '-d', out / 'non-infix-candidate', '--diagnostic-format', 'json'], 1)
    d = [json.loads(x) for x in p.stderr.splitlines() if x.startswith('{')]
    assert d[0]['category'] == 'invalid_kotlin' and d[0]['code'] == 'INFIX_MODIFIER_REQUIRED', d
    cases.append(dict(id='known_non_infix_stays_invalid', outcome='invalid_rejected', diagnostics=d))
    run('Java classifier', [jdk / 'javac', '-d', out / 'java', f / 'Visible.java', f / 'Alias.java', f / 'Remote.java'])
    for (name, binary) in [('official', oracle), ('candidate', candidate)]:
        run(name + ' explicit alias classifier', [binary, f / 'AliasLiteral.kt', f / 'AliasHidden.kt', '-cp', cp(std, out / 'java'), '-d', out / ('alias-' + name), *opts] if name == 'official' else [binary, f / 'AliasLiteral.kt', f / 'AliasHidden.kt', '-cp', cp(std, out / 'java'), '-d', out / ('alias-' + name)])
    run('official-only alias Java caller', [jdk / 'javac', '-cp', cp(std, out / 'java', out / 'alias-official'), '-d', out / 'alias-caller', f / 'AliasCalls.java'])
    for name in ['official', 'candidate']:
        p = run('same-package original and alias ' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(std, out / 'java', out / ('alias-' + name), out / 'alias-caller'), 'AliasCalls'])
        assert p.stdout == 'OK\n'
        cases.append(dict(id='alias_binding_and_same_package_original_' + name, outcome='pass'))
    run('oracle hidden foreign original', [oracle, f / 'ForeignAliasHidden.kt', '-cp', cp(std, out / 'java'), '-d', out / 'foreign-original-official', *opts], 1)
    p = run('candidate hidden foreign original', [candidate, f / 'ForeignAliasHidden.kt', '-cp', cp(std, out / 'java'), '-d', out / 'foreign-original-candidate', '--diagnostic-format', 'json'], 1)
    cases.append(dict(id='different_package_original_stays_hidden', outcome='invalid_rejected', scope='Unresolved-name rejection only; diagnostic equivalence unassessed'))
    run('Java named classifier', [jdk / 'javac', '-d', out / 'named-java', f / 'named/Visible.java'])
    for (name, binary) in [('official', oracle), ('candidate', candidate)]:
        run(name + ' named aliases and plain lookup', [binary, f / 'named/Alias.kt', f / 'named/Plain.kt', '-cp', cp(std, out / 'named-java'), '-d', out / ('named-' + name), *opts] if name == 'official' else [binary, f / 'named/Alias.kt', f / 'named/Plain.kt', '-cp', cp(std, out / 'named-java'), '-d', out / ('named-' + name)])
    run('official-only named Java caller', [jdk / 'javac', '-cp', cp(std, out / 'named-java', out / 'named-official'), '-d', out / 'named-caller', f / 'named/NamedCalls.java'])
    for name in ['official', 'candidate']:
        p = run('named alias and plain runtime ' + name, [jdk / 'java', '-Xverify:all', '-cp', cp(std, out / 'named-java', out / ('named-' + name), out / 'named-caller'), 'named.NamedCalls'])
        assert p.stdout == 'OK\n'
        cases.append(dict(id='named_alias_and_plain_' + name, outcome='pass'))
    run('oracle named original hidden', [oracle, f / 'named/Original.kt', '-cp', cp(std, out / 'named-java'), '-d', out / 'named-original-official', *opts], 1)
    run('candidate named original hidden', [candidate, f / 'named/Original.kt', '-cp', cp(std, out / 'named-java'), '-d', out / 'named-original-candidate', '--diagnostic-format', 'json'], 1)
    cases.append(dict(id='named_package_original_stays_hidden', outcome='invalid_rejected', scope='Unresolved-name rejection only; diagnostic equivalence unassessed'))
    frozen.update(fixed_callers)
    changed = [p for (p, x) in frozen.items() if h(root / p) != x]
    report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='complete' if not changed else 'failed', compiler=dict(path=str(candidate.relative_to(root)), sha256=h(candidate)), commands=cmds, cases=cases, counts=dict(Counter((c['outcome'] for c in cases))), fixed_java_caller_hashes=fixed_callers, frozen_input_hashes=frozen, input_integrity=dict(status='changed' if changed else 'unchanged', changed_paths=changed), scope='Diagnostic and import-priority regressions only. Generic value support is unchanged; unsupported candidate is never emitted or run.')
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(report['status'], cases)
except Exception as e:
    cases.append(dict(id='failure', outcome='correctness_failure', error=str(e)))
    save('failed')
    print('failed', str(e))
    raise
raise SystemExit(0 if report['status'] == 'complete' else 1)
