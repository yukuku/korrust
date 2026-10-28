import json
import subprocess
import tempfile
import unittest
from pathlib import Path

import upstream_jvm as adapter


class UpstreamAdapterTests(unittest.TestCase):
    def fixture(self, text):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        path = Path(directory.name) / 'original.kt'
        path.write_text(text)
        return adapter.parse_fixture(path)

    def test_modules_friends_and_files(self):
        fixture = self.fixture('// WITH_STDLIB\n// MODULE: library\n// FILE: a.kt\ninternal fun a() = 1\n// MODULE: main(library)(library)\n// FILE: b.kt\nfun box() = "OK"\n')
        self.assertEqual([module['name'] for module in fixture['modules']], ['library', 'main'])
        self.assertEqual(fixture['modules'][1]['dependencies'], ['library'])
        self.assertEqual(fixture['modules'][1]['friends'], ['library'])
        self.assertEqual([file['name'] for module in fixture['modules'] for file in module['files']], ['a.kt', 'b.kt'])
        self.assertEqual(adapter.assess(fixture, 'box'), ('ready', []))

    def test_unknown_directives_are_not_silently_ignored(self):
        fixture = self.fixture('// SOME_FUTURE_SETTING: true\nfun box() = "OK"\n')
        status, reasons = adapter.assess(fixture, 'box')
        self.assertEqual(status, 'harness_unsupported')
        self.assertIn('SOME_FUTURE_SETTING', reasons[0])

    def test_nested_markers_preserve_original_unicode_positions(self):
        fixture = self.fixture('val λ = <!TYPE_MISMATCH!><!UNRESOLVED_REFERENCE!>missing<!><!>\n')
        source = fixture['modules'][0]['files'][0]
        self.assertEqual(source['text'], 'val λ = missing\n')
        self.assertEqual(len(source['expected_diagnostics']), 2)
        clean_offset = source['text'].index('missing')
        location = adapter.source_location(fixture, source, clean_offset)
        self.assertEqual(location['char_offset'], fixture['original'].index('missing'))
        self.assertEqual(location['byte_offset'], location['char_offset'] + 1)
        self.assertEqual(location['column'], location['char_offset'] + 1)

    def test_multiplatform_and_incremental_are_harness_unsupported(self):
        fixture = self.fixture('// MODULE: common\nfun a() = 1\n// MODULE: main()()(common)\nfun b() = 2\n')
        self.assertEqual(adapter.assess(fixture, 'box')[0], 'harness_unsupported')
        self.assertEqual(adapter.assess(self.fixture('fun a() = 1\n'), 'incremental')[0], 'harness_unsupported')

    def test_non_jvm_target_is_excluded(self):
        fixture = self.fixture('// TARGET_BACKEND: JS_IR\nfun box() = "OK"\n')
        self.assertEqual(adapter.assess(fixture, 'box')[0], 'excluded_target')

    def test_unsafe_and_duplicate_files_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Unsafe'):
            self.fixture('// FILE: ../escape.kt\nfun a() = 1\n')
        with self.assertRaisesRegex(ValueError, 'Duplicate FILE'):
            self.fixture('// FILE: a.kt\nfun a() = 1\n// FILE: a.kt\nfun b() = 2\n')

    def test_language_options_preserve_requested_features(self):
        fixture = self.fixture('// LANGUAGE: +ContextParameters -SomeFeature\n// JVM_TARGET: 17\nfun a() = 1\n')
        options = adapter.options_for(fixture['modules'][0])
        self.assertIn('-XXLanguage:+ContextParameters', options)
        self.assertIn('-XXLanguage:-SomeFeature', options)
        self.assertEqual(options[options.index('-jvm-target') + 1], '17')

    def test_byte_offset_diagnostic_maps_to_original_source(self):
        fixture = self.fixture('val λ = <!UNRESOLVED_REFERENCE!>missing<!>\n')
        source = fixture['modules'][0]['files'][0]
        with tempfile.TemporaryDirectory() as directory:
            sources = adapter.write_sources(fixture, Path(directory))
            path = sources['main']['paths'][0]
            result = subprocess.CompletedProcess([], 1, '', json.dumps({'path': str(path), 'offset': len('val λ = '.encode()), 'category': 'unresolved_reference'}))
            diagnostic = adapter.mapped_diagnostics(result, fixture, sources)[0]
            self.assertEqual(diagnostic['upstream_location']['char_offset'], fixture['original'].index('missing'))

    def test_report_publication_preserves_commands_and_archives_evidence(self):
        with tempfile.TemporaryDirectory(dir=adapter.ROOT / 'target') as directory:
            temporary = Path(directory)
            work = temporary / 'work'
            work.mkdir()
            (work / 'log.txt').write_text('evidence')
            source_map = work / 'a.kt.source-map.json'
            source_map.write_text(json.dumps({'materialized': adapter.relative(work / 'a.kt')}))
            report = {'commands': [{'argv': [str(work / 'a.kt')], 'stdout': adapter.relative(work / 'log.txt')}],
                      'cases': [{'source_maps': [adapter.relative(source_map)]}]}
            destination = temporary / 'published.json'
            adapter.publish_report(report, work, destination)
            published = json.loads(destination.read_text())
            self.assertEqual(published['commands'][0]['argv'], report['commands'][0]['argv'])
            archived_log = adapter.ROOT / published['commands'][0]['stdout']
            self.assertEqual(archived_log.read_text(), 'evidence')
            archived_map = adapter.ROOT / published['cases'][0]['source_maps'][0]
            self.assertTrue(json.loads(archived_map.read_text())['materialized'].startswith(adapter.relative(destination.with_suffix(''))))
            with self.assertRaisesRegex(RuntimeError, 'overwrite'):
                adapter.publish_report(report, work, destination)


    def test_structured_setup_and_dependency_failures_are_distinct(self):
        for category, expected in [('missing_dependency', 'dependency_failure'), ('setup', 'setup_failure'), ('io', 'setup_failure')]:
            result = subprocess.CompletedProcess([], 1, '', json.dumps({'category': category}))
            self.assertEqual(adapter.classify_candidate(result), expected)

    def test_unclassified_errors_are_failures(self):
        result = subprocess.CompletedProcess([], 1, '', 'compiler crashed')
        self.assertEqual(adapter.classify_candidate(result), 'correctness_failure')
        structured = subprocess.CompletedProcess([], 1, '', '{"category":"unclassified","message":"Unsupported thing"}\n')
        self.assertEqual(adapter.classify_candidate(structured), 'correctness_failure')
        result = subprocess.CompletedProcess([], 1, '', '{"category":"unsupported_language"}\n')
        self.assertEqual(adapter.classify_candidate(result), 'unsupported_language')


if __name__ == '__main__':
    unittest.main()
