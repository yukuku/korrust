from pathlib import Path
from datetime import datetime, timezone
from collections import Counter
import json, hashlib, os, subprocess, shutil, zipfile, argparse
parser = argparse.ArgumentParser()
for name in ['root', 'candidate', 'fixtures', 'output']:
    parser.add_argument('--' + name, required=True, type=Path)
args = parser.parse_args()
ROOT = args.root.resolve()
out = args.output.resolve()
out.mkdir(exist_ok=False)
(out / 'commands').mkdir()
fixtures = args.fixtures.resolve()
env = os.environ.copy()
env['JAVA_HOME'] = os.environ['JAVA_HOME']
jdk = Path(env['JAVA_HOME']) / 'bin'
oracle = ROOT / 'references/kotlinc/bin/kotlinc'
std = ROOT / 'references/kotlinc/lib/kotlin-stdlib.jar'
candidate = out / 'korrust'
shutil.copy2(args.candidate.resolve(), candidate)
hash = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
paths = [*ROOT.glob('src/**/*.rs'), *[p for p in fixtures.rglob('*') if p.is_file()], Path(__file__), ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', ROOT / 'references/compiler.json', ROOT / 'references/sources.json', candidate, std, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar']
frozen = {str(p.relative_to(ROOT)): hash(p) for p in paths}
fixed_callers = {}
report = dict(schema_version=1, kind='correctness', created_at=datetime.now(timezone.utc).isoformat(), status='running', compiler=dict(path=str(candidate.relative_to(ROOT)), sha256=hash(candidate)), frozen_input_hashes=frozen, cases=[], commands=[])
with zipfile.ZipFile(out / 'frozen-sources.zip', 'w', zipfile.ZIP_DEFLATED) as z:
    for p in paths:
        if p not in [candidate, std, ROOT / 'references/kotlinc/lib/kotlin-compiler.jar']:
            z.write(p, str(p.relative_to(ROOT)))

def save():
    report['counts'] = dict(Counter((x['outcome'] for x in report['cases'])))
    (out / 'report.json').write_text(json.dumps(report, indent=2) + '\n')

def run(label, argv, expected=0):
    argv = list(map(str, argv))
    p = subprocess.run(argv, env=env, cwd=ROOT, text=True, capture_output=True, timeout=180)
    log = out / 'commands' / f"{len(report['commands']):03d}.log"
    log.write_text(p.stdout + p.stderr)
    report['commands'].append(dict(label=label, argv=argv, exit_code=p.returncode, log=str(log.relative_to(ROOT))))
    save()
    assert p.returncode == expected, (label, p.returncode, p.stdout, p.stderr)
    return p
opts = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']

def case(name, fn, outcome='pass'):
    try:
        detail = fn()
        report['cases'].append(dict(id=name, outcome=outcome, detail=detail))
    except Exception as e:
        report['cases'].append(dict(id=name, outcome='correctness_failure', error=str(e)))
    save()
run('official dependency', [oracle, fixtures / 'Dependency.kt', '-cp', std, '-d', out / 'dependency', *opts])
run('Java same-package classifiers', [jdk / 'javac', '-d', out / 'java-dependency', fixtures / 'Hidden.java', fixtures / 'Visible.java'])
cp = os.pathsep.join(map(str, [std, out / 'dependency', out / 'java-dependency']))
for (label, binary) in [('official', oracle), ('candidate', candidate)]:
    run(label + ' supported neighbors', [binary, fixtures / 'Supported.kt', fixtures / 'VisibleLiteral.kt', '-cp', cp, '-d', out / label, *opts] if label == 'official' else [binary, fixtures / 'Supported.kt', fixtures / 'VisibleLiteral.kt', '-cp', cp, '-d', out / label])
run('Java caller against official only', [jdk / 'javac', '-cp', os.pathsep.join(map(str, [out / 'official', out / 'dependency', out / 'java-dependency', std])), '-d', out / 'callers', fixtures / 'Calls.java'])
fixed_callers = {str(p.relative_to(ROOT)): hash(p) for p in (out / 'callers').rglob('*.class')}
for label in ['official', 'candidate']:

    def runtime(label=label):
        p = run(label + ' supported runtime', [jdk / 'java', '-Xverify:all', '-cp', os.pathsep.join(map(str, [out / 'callers', out / label, out / 'dependency', out / 'java-dependency', std])), 'Calls'])
        assert p.stdout == 'OK\n'
        return 'Same official-compiled Java caller verifies ordinary generic reference extension, nongeneric value extension and public same-package class literal'
    case('supported_' + label, runtime)
checks = [('generic_value', 'UnsupportedPacket.kt', True, 'unsupported_language', 'GENERIC_VALUE_EXTENSION_ABI_UNSUPPORTED'), ('invalid_return', 'Invalid.kt', False, 'invalid_kotlin', 'TYPE_MISMATCH'), ('hidden_literal', 'HiddenLiteral.kt', True, 'unsupported_language', 'CLASS_LITERAL_VISIBILITY_UNSUPPORTED')]
inline = [('suspend_interface', 'fun interface Reader { suspend fun read(): String }', True, 'unsupported_language', 'SUSPEND_MEMBER_DECLARATION_UNSUPPORTED'), ('private_nested', 'class Outer { private data class Entry(val text: String) }', True, 'unsupported_language', 'CLASS_VISIBILITY_UNSUPPORTED'), ('private_top', 'private class Entry', True, 'unsupported_language', 'CLASS_VISIBILITY_UNSUPPORTED'), ('protected_nested', 'open class Outer { protected class Entry }', True, 'unsupported_language', 'CLASS_VISIBILITY_UNSUPPORTED'), ('protected_top', 'protected class Entry', False, 'invalid_kotlin', 'CLASS_VISIBILITY_INVALID'), ('conflicting_visibility', 'public private class Entry', False, 'invalid_kotlin', 'CLASS_VISIBILITY_INVALID'), ('ordinary_interface', 'fun interface Reader { fun read(): String }', True, None, None), ('public_nested', 'class Outer { class Entry(val text: String) }', True, None, None)]
for (name, source, valid, kind, code) in inline:
    path = out / (name + '.kt')
    path.write_text(source)
    frozen[str(path.relative_to(ROOT))] = hash(path)
    checks.append((name, path, valid, kind, code))
    with zipfile.ZipFile(out / 'frozen-sources.zip', 'a', zipfile.ZIP_DEFLATED) as z:
        z.write(path, str(path.relative_to(ROOT)))
for (name, path, valid, kind, code) in checks:

    def check(name=name, path=path, valid=valid, kind=kind, code=code):
        path = path if isinstance(path, Path) else fixtures / path
        run('official ' + name, [oracle, path, '-cp', cp, '-d', out / (name + '-official'), *opts], 0 if valid else 1)
        p = run('candidate ' + name, [candidate, path, '-cp', cp, '-d', out / (name + '-candidate'), '--diagnostic-format', 'json'], 1 if kind else 0)
        if kind:
            diagnostics = [json.loads(line) for line in p.stderr.splitlines() if line.startswith('{')]
            assert diagnostics and diagnostics[0]['category'] == kind, diagnostics
            assert diagnostics[0]['code'] == code, diagnostics
            return dict(oracle_valid=valid, diagnostics=diagnostics)
        return 'Both providers emit supported neighboring source'
    case(name, check, 'unsupported_language' if kind == 'unsupported_language' else 'invalid_rejected' if kind else 'pass')
frozen.update(fixed_callers)
report['fixed_java_caller_hashes'] = fixed_callers
changed = [p for (p, h) in frozen.items() if hash(ROOT / p) != h]
report['input_integrity'] = dict(status='changed' if changed else 'unchanged', changed_paths=changed)
report['status'] = 'complete' if not changed and all((x['outcome'] != 'correctness_failure' for x in report['cases'])) else 'failed'
save()
print(report['status'], report['counts'])
raise SystemExit(0 if report['status'] == 'complete' else 1)
