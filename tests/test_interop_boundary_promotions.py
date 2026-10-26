import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock
import zipfile

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'tools'))
import check as baseline


SOURCES = {
    'java_generic_method': 'import fixtures.ForeignMethods\nfun f(value: ForeignMethods): String = value.generic("x")',
    'java_annotated_method': 'import fixtures.ForeignMethods\nfun f(value: ForeignMethods): Int = value.annotated()',
    'java_inherited_member': 'import fixtures.ForeignMethods\nfun f(value: ForeignMethods): Int = value.hashCode()',
    'java_hierarchy_overload': 'import fixtures.ChildMethods\nfun f(value: ChildMethods): Int = value.value(1)',
    'java_interface': 'import fixtures.ForeignInterface\nfun f(value: ForeignInterface): Int = value.value()',
    'java_annotated_class': 'import fixtures.AnnotatedForeign\nfun f(value: AnnotatedForeign): Int = value.value()',
    'java_annotated_package': 'import fixtures.annotated.PackageForeign\nfun f(value: PackageForeign): Int = value.value()',
}
NEWLY_SUPPORTED = {'java_annotated_method'}
ABI = 'Compiled from "Positive.kt"\npublic final class PositiveKt {\n  public static final int f();\n    descriptor: ()I\n}\n'


class InteropBoundaryPromotionTests(unittest.TestCase):
    def test_all_seven_original_sources_and_ids_are_preserved(self):
        rows = json.loads((ROOT / 'tests/interop_unsupported.json').read_text())
        self.assertEqual(len(rows), 7)
        self.assertEqual({row['id']: row['source'] for row in rows}, SOURCES)
        self.assertEqual({row['id'] for row in rows if not row.get('supported_regression')}, {'java_inherited_member', 'java_annotated_class', 'java_annotated_package'})
        for row in rows:
            if not row.get('supported_regression'):
                continue
            regression = row['supported_regression']
            self.assertEqual(regression['facade'], row['id'][0].upper() + row['id'][1:] + 'Kt')
            if row['id'] in NEWLY_SUPPORTED:
                self.assertEqual(regression['runtime_checks'], 6)
                for marker in ['calls[0]', '!= -6', '!= 11', 'actual != expected',
                               'NullPointerException', '.f(null)']:
                    self.assertIn(marker, regression['java_assertions'])

    def execute_mock(self, temporary, *, wrong_runtime=False, wrong_abi=False,
                     mutate_driver=False, verifier=None):
        work = Path(temporary)
        source = work / 'positive.kt'
        source.write_text('fun f(): Int = 1')
        fixture = {'id': 'positive', 'supported_regression': {
            'facade': 'PositiveKt', 'java_assertions': 'if (PositiveKt.f() != 1) throw new AssertionError();'}}
        if verifier is not None:
            fixture['supported_regression']['supplier_fields'] = {'state': 'byte'}

        def invoke(label, args):
            if label.startswith('Compile Java regression caller'):
                output = Path(args[args.index('-d') + 1])
                output.mkdir(parents=True)
                (output / 'RegressionCheck.class').write_bytes(b'official-fixed-driver')
            if label.startswith('Run supported regression'):
                if mutate_driver and label.endswith('candidate'):
                    driver = work / 'supported-regressions/positive/drivers/RegressionCheck.class'
                    driver.write_bytes(b'changed-driver')
                return SimpleNamespace(stdout='wrong\n' if wrong_runtime and label.endswith('candidate') else 'ok\n')
            if label.startswith('Inspect supported regression ABI'):
                return SimpleNamespace(stdout=ABI.replace('()I', '()J') if wrong_abi and label.endswith('candidate') else ABI)
            return SimpleNamespace(stdout='')

        run = Mock(side_effect=invoke)
        result = baseline.run_supported_regression(
            fixture, source, [work / 'dependency'], work=work, kotlinc='kotlinc',
            compiler='korrust', stdlib=work / 'stdlib.jar', options=['-jvm-target', '1.8'],
            tools={'java': 'java', 'javac': 'javac', 'javap': 'javap'}, run=run,
            verify_candidate=verifier)
        return result, run

    def test_one_official_compiled_driver_is_reused_with_verification(self):
        with tempfile.TemporaryDirectory() as temporary:
            result, run = self.execute_mock(temporary)
            calls = run.call_args_list
            javac = [call for call in calls if call.args[0].startswith('Compile Java regression caller')]
            self.assertEqual(len(javac), 1)
            self.assertIn('against official Kotlin', javac[0].args[0])
            cp = javac[0].args[1][javac[0].args[1].index('-cp') + 1]
            self.assertIn('/official', cp)
            self.assertNotIn('/candidate', cp)
            java = [call for call in calls if call.args[0].startswith('Run supported regression')]
            self.assertEqual(len(java), 2)
            self.assertTrue(all('-Xverify:all' in call.args[1] for call in java))
            self.assertEqual(result['driver_classfiles_sha256'], {
                'RegressionCheck.class': hashlib.sha256(b'official-fixed-driver').hexdigest()})

    def test_wrong_runtime_output_cannot_record_a_pass(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(AssertionError, 'Unexpected regression output'):
                self.execute_mock(temporary, wrong_runtime=True)

    def test_wrong_abi_cannot_record_a_pass(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(AssertionError, 'JVM signatures differ'):
                self.execute_mock(temporary, wrong_abi=True)

    def test_driver_mutation_cannot_record_a_pass(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(AssertionError, 'caller bytecode changed'):
                self.execute_mock(temporary, mutate_driver=True)

    def test_supplier_verification_is_preserved_and_failures_propagate(self):
        with tempfile.TemporaryDirectory() as temporary:
            verifier = Mock(side_effect=AssertionError('supplier contract failed'))
            with self.assertRaisesRegex(AssertionError, 'supplier contract failed'):
                self.execute_mock(temporary, verifier=verifier)
            verifier.assert_called_once_with(Path(temporary) / 'supported-regressions/positive/candidate')


def audit(compiler, destination):
    destination.mkdir(parents=True)
    logs = destination / 'logs'
    logs.mkdir()
    report = {'status': 'running', 'commands': [], 'cases': [],
              'compiler': {'path': str(compiler), 'sha256': baseline.sha(compiler)}}

    def run(label, args, allowed=(0,)):
        entry = {'label': label, 'argv': [str(arg) for arg in args], 'cwd': str(ROOT)}
        index = len(report['commands'])
        report['commands'].append(entry)
        result = subprocess.run(entry['argv'], cwd=ROOT, capture_output=True, text=True, timeout=120)
        entry['exit_code'] = result.returncode
        for stream in ['stdout', 'stderr']:
            path = logs / f'{index:03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = str(path.relative_to(destination))
        if result.returncode not in allowed:
            raise AssertionError(f'{label}: {result.stderr}')
        return result

    try:
        tools = {name: shutil.which(name) for name in ['java', 'javac', 'javap']}
        if not all(tools.values()):
            raise AssertionError('JDK tools are unavailable')
        pin = json.loads((ROOT / 'references/compiler.json').read_text())
        kotlinc = ROOT / pin['path'] / 'bin/kotlinc'
        stdlib = ROOT / pin['path'] / 'lib/kotlin-stdlib.jar'
        version = run('Pinned reference identity', [kotlinc, '-version'])
        if f"kotlinc-jvm {pin['version']} " not in version.stdout + version.stderr:
            raise AssertionError('Pinned reference version mismatch')
        report['reference'] = {'version': pin['version'], 'compiler_sha256': baseline.sha(kotlinc.parent.parent / 'lib/kotlin-compiler.jar'),
                               'stdlib_sha256': baseline.sha(stdlib)}
        foreign = destination / 'foreign'
        java_home = Path(os.environ.get('JAVA_HOME', str(Path(tools['java']).resolve().parents[1])))
        jmod = java_home / 'jmods/java.base.jmod'
        jdk_classes = ['java/io/IOException', 'java/io/Closeable', 'java/lang/AutoCloseable',
                       'java/lang/Exception', 'java/lang/Throwable', 'java/lang/Object', 'java/lang/Deprecated']
        with zipfile.ZipFile(jmod) as archive:
            for name in jdk_classes:
                path = foreign / (name + '.class')
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(archive.read('classes/' + name + '.class'))
        java_sources = sorted((ROOT / 'tests/fixtures/interop').rglob('*.java'))
        report['dependencies'] = {'jmod_sha256': baseline.sha(jmod),
                                  'jdk_classes_sha256': {name: baseline.sha(foreign / (name + '.class')) for name in jdk_classes},
                                  'java_sources_sha256': {str(path.relative_to(ROOT)): baseline.sha(path) for path in java_sources}}
        run('Compile unchanged Java interop fixtures', [tools['javac'], '-d', foreign, *java_sources])
        rows = json.loads((ROOT / 'tests/interop_unsupported.json').read_text())
        if {row['id']: row['source'] for row in rows} != SOURCES or len(rows) != 7:
            raise AssertionError('Original interop sources or IDs changed')
        report['unchanged_sources_and_ids'] = 7
        report['previously_supported_rows'] = 3
        report['newly_supported_rows'] = sorted(NEWLY_SUPPORTED)
        options = ['-language-version', '2.3', '-api-version', '2.3', '-jvm-target', '1.8']
        for row in rows:
            source = destination / (row['id'] + '.kt')
            source.write_text(row['source'])
            if row.get('supported_regression'):
                report['cases'].append(baseline.run_supported_regression(
                    row, source, [foreign], work=destination, kotlinc=kotlinc, compiler=compiler,
                    stdlib=stdlib, options=options, tools=tools, run=run))
            else:
                run('Reference Java interop acceptance: ' + row['id'],
                    [kotlinc, source, '-cp', foreign, *options, '-d', destination / 'unsupported-reference'])
                output = destination / ('unsupported-' + row['id'])
                actual = run('Explicit Java interop unsupported rejection: ' + row['id'],
                             [compiler, source, '-cp', foreign, '-d', output], allowed=(1,))
                if output.exists():
                    raise AssertionError('Unsupported input produced output: ' + row['id'])
                report['cases'].append({'id': 'unsupported/' + row['id'], 'outcome': 'unsupported_language',
                                        'source_sha256': baseline.sha(source), 'diagnostic': actual.stderr.strip(),
                                        'scope': 'Pinned official accepts; candidate rejects with exit1 and no output directory.'})
        if baseline.sha(compiler) != report['compiler']['sha256']:
            raise AssertionError('Frozen compiler changed')
        report['status'] = 'completed'
    except Exception as error:
        report['status'] = 'failed'
        report['error'] = str(error)
        raise
    finally:
        (destination / 'result.json').write_text(json.dumps(report, indent=2) + '\n')
    return report


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == '--audit':
        parser = argparse.ArgumentParser()
        parser.add_argument('--audit', type=Path, required=True)
        parser.add_argument('--compiler', type=Path, required=True)
        args = parser.parse_args()
        result = audit(args.compiler.resolve(), args.audit.resolve())
        print(json.dumps({'status': result['status'], 'cases': len(result['cases'])}))
    else:
        unittest.main()
