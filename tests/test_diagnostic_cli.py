import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
COMPILER = Path(os.environ.get('KORRUST_COMPILER', ROOT / 'target/release/korrust')).resolve()


class StructuredDiagnosticCliTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='korrust-diagnostic-cli-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.first = self.root / 'First.kt'
        self.second = self.root / 'Second.kt'
        self.destination = self.root / 'output'
        self.first.write_text('package diagnostics.sample\nfun first(): Int = 7\n')
        self.second.write_text('package diagnostics.sample\nfun second(): Int = first() + 1\n')
        result = self.invoke([self.first, self.second])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, '')
        self.assertTrue(list(self.destination.rglob('*.class')))
        self.before = self.snapshot()

    def invoke(self, sources, *extra):
        return subprocess.run(
            [str(COMPILER), *map(str, sources), '-d', str(self.destination),
             '--module-name', 'diagnostic_cli', '--diagnostic-format', 'json', *extra],
            cwd=ROOT, capture_output=True, text=True, timeout=30,
        )

    def snapshot(self):
        return {str(path.relative_to(self.destination)): path.read_bytes()
                for path in self.destination.rglob('*') if path.is_file()}

    def failure(self, sources, *extra):
        result = self.invoke(sources, *extra)
        self.assertEqual(result.returncode, 1, result.stdout)
        self.assertEqual(result.stdout, '')
        diagnostic = json.loads(result.stderr)
        self.assertIsInstance(diagnostic, dict)
        self.assertTrue({'source_identity', 'offset', 'category', 'code', 'message'} <= diagnostic.keys())
        self.assertIs(type(diagnostic['offset']), int)
        self.assertIsInstance(diagnostic['message'], str)
        self.assertTrue(diagnostic['message'])
        self.assertEqual(self.snapshot(), self.before)
        return diagnostic

    def assert_location(self, diagnostic, source, text, token):
        prefix = text[:text.index(token)]
        self.assertEqual(diagnostic['source_identity'], str(source.resolve()))
        self.assertEqual(diagnostic['path'], str(source))
        self.assertEqual(diagnostic['offset'], len(prefix.encode('utf-8')))
        self.assertEqual(diagnostic['line'], prefix.count('\n') + 1)
        self.assertEqual(diagnostic['column'], len(prefix.rsplit('\n', 1)[-1]) + 1)

    def test_invalid_flags_report_setup_without_changing_owned_outputs(self):
        for flag, code in [('--unknown-compiler-option', 'INVALID_ARGUMENTS'),
                           ('invalid-format', 'INVALID_DIAGNOSTIC_FORMAT')]:
            with self.subTest(flag=flag):
                extra = ['--diagnostic-format', flag] if flag == 'invalid-format' else [flag]
                diagnostic = self.failure([self.first], *extra)
                self.assertEqual((diagnostic['category'], diagnostic['code']), ('setup', code))
                self.assertIsNone(diagnostic['source_identity'])
                self.assertEqual(diagnostic['offset'], 0)
                self.assertNotIn('line', diagnostic)
                self.assertNotIn('column', diagnostic)

    def test_missing_source_reports_io_without_fabricating_source_location(self):
        missing = self.root / 'MissingSource.kt'
        diagnostic = self.failure([missing])
        self.assertEqual((diagnostic['category'], diagnostic['code']), ('io', 'SOURCE_FILE_NOT_FOUND'))
        self.assertEqual(diagnostic['source_identity'], str(missing))
        self.assertEqual(diagnostic['path'], str(missing))
        self.assertEqual(diagnostic['offset'], 0)
        self.assertNotIn('line', diagnostic)
        self.assertNotIn('column', diagnostic)

    def test_batch_diagnostic_identifies_failing_file_in_both_source_orders(self):
        text = '\npackage diagnostics.sample\n\nclass Invalid {\n    operator fun contains(value: Int): Int = value\n}\n'
        self.first.write_text('package diagnostics.sample\nfun first(): Int = 99\n')
        self.second.write_text(text)
        observations = []
        for sources in [[self.first, self.second], [self.second, self.first]]:
            diagnostic = self.failure(sources)
            self.assertEqual((diagnostic['category'], diagnostic['code']), ('invalid_kotlin', 'INVALID_OPERATOR_SIGNATURE'))
            self.assert_location(diagnostic, self.second, text, 'operator')
            observations.append(diagnostic)
        self.assertEqual(observations[0], observations[1])

    def test_missing_signature_type_reports_class_code_at_type_token(self):
        text = '\n\npackage diagnostics.sample\n\nfun first(value: Missing): Missing = value\n'
        self.first.write_text(text)
        diagnostic = self.failure([self.first])
        self.assertEqual((diagnostic['category'], diagnostic['code']), ('missing_dependency', 'CLASS_NOT_FOUND'))
        self.assert_location(diagnostic, self.first, text, 'Missing')
        self.assertGreater(diagnostic['line'], 1)

    def test_invalid_operator_reports_invalid_signature_at_declaration(self):
        text = '\n\nclass Invalid {\n    operator fun contains(value: Int): Int = value\n}\n'
        self.first.write_text(text)
        diagnostic = self.failure([self.first])
        self.assertEqual((diagnostic['category'], diagnostic['code']), ('invalid_kotlin', 'INVALID_OPERATOR_SIGNATURE'))
        self.assert_location(diagnostic, self.first, text, 'operator')

    def test_nonproperty_constructor_reference_reports_expression_location(self):
        text = '\n\nclass E(value: Int) {\n    fun f(): Int =\n        value\n}\n'
        self.first.write_text(text)
        diagnostic = self.failure([self.first])
        self.assertEqual((diagnostic['category'], diagnostic['code']), ('invalid_kotlin', 'NON_PROPERTY_CONSTRUCTOR_PARAMETER'))
        self.assert_location(diagnostic, self.first, text, 'value\n')
        self.assertGreater(diagnostic['offset'], text.index('value:'))

    def test_value_class_reports_utf8_byte_offset_and_unicode_column(self):
        text = '// λ🙂\n\n/* café λ🙂 */ @JvmInline value class Identifier(val value: Int)\n'
        self.first.write_text(text)
        diagnostic = self.failure([self.first])
        self.assertEqual((diagnostic['category'], diagnostic['code']), ('unsupported_language', 'VALUE_CLASS_JVM_BOXING_UNSUPPORTED'))
        self.assert_location(diagnostic, self.first, text, 'value class')
        prefix = text[:text.index('value class')]
        self.assertGreater(diagnostic['offset'], len(prefix))
        self.assertNotEqual(diagnostic['column'], len(prefix.rsplit('\n', 1)[-1].encode('utf-8')) + 1)


if __name__ == '__main__':
    unittest.main()
