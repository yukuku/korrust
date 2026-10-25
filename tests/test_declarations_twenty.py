import json
from pathlib import Path
import re
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import check_declarations_twenty as checks


class DeclarationsTwentyEvidenceTests(unittest.TestCase):
    def test_twenty_independent_cases_have_java_and_kotlin_consumers(self):
        ids = {item['id'] for item in checks.FEATURES}
        self.assertEqual(len(checks.FEATURES), 20)
        self.assertEqual(len(ids), 20)
        self.assertEqual(ids, set(checks.RESTRICTIONS))
        driver = (checks.FIXTURES / 'DeclarationsCheck.java').read_text()
        self.assertEqual(ids, set(re.findall(r'case "([a-z_]+)":', driver)))
        for item in checks.FEATURES:
            with self.subTest(feature=item['id']):
                fixture = checks.FIXTURES / item['fixture']
                self.assertTrue(fixture.is_file())
                self.assertTrue((checks.FIXTURES / ('Consumer' + fixture.stem + '.kt')).is_file())
                self.assertTrue(item['scope'])

    def test_each_java_driver_has_only_its_own_fixture_references(self):
        for item in checks.FEATURES:
            source = checks.java_driver_source(item['id'])
            self.assertIn('public final class DeclarationsCheck_' + item['id'], source)
            self.assertEqual(re.findall(r'case "([a-z_]+)":', source), [item['id']])
        self.assertNotIn('new Parent', checks.java_driver_source('init_blocks'))

    def test_supported_requires_runtime_abi_metadata_freeze_and_semantic_boundaries(self):
        for item in checks.FEATURES:
            ident = item['id']
            cases = [dict(id=kind + '/' + ident, outcome='pass') for kind in ['oracle', 'compile', 'java', 'source_abi', 'class_verification', 'kotlin_consumer']]
            cases += [dict(id='frozen_inputs', outcome='pass')]
            cases += [dict(id='boundary/' + name, outcome='pass') for name in checks.NEGATIVE.get(ident, [])]
            if ident == 'generic_arrays':
                cases.append(dict(id='array_order/oracle', outcome='pass'))
                cases.append(dict(id='array_order/effectful_initializer', outcome='unsupported_language'))
            self.assertEqual(checks.feature_status(cases, ident), 'supported')
            for index in range(len(cases)):
                for outcome in ['pass', 'fail', 'timeout', 'not_run', 'unsupported_language']:
                    if outcome == cases[index]['outcome']:
                        continue
                    changed = [dict(case) for case in cases]
                    changed[index]['outcome'] = outcome
                    self.assertNotEqual(checks.feature_status(changed, ident), 'supported')

    def test_parsing_and_emission_alone_do_not_establish_support(self):
        for item in checks.FEATURES:
            self.assertEqual(checks.feature_status([dict(id='compile/' + item['id'], outcome='pass')], item['id']), 'partial')
            self.assertEqual(checks.feature_status([dict(id='compile/' + item['id'], outcome='unsupported_language')], item['id']), 'unsupported')

    def test_negative_results_require_semantic_rejection_and_no_output(self):
        boundary = next(item for item in checks.BOUNDARIES if item['id'] == 'contains_operator_modifier')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: contains requires operator modifier', {}), 'pass')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: contains requires operator modifier', {'Bad.class': 'hash'}), 'fail')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: unresolved unrelated name', {}), 'fail')
        self.assertEqual(checks.boundary_outcome(boundary, 0, '', {}), 'fail')

    def test_effectful_array_initializer_is_an_explicit_unsupported_boundary(self):
        self.assertEqual(checks.effectful_initializer_outcome(1, 'error: Effectful array initializer expressions are not supported', {}), 'unsupported_language')
        self.assertEqual(checks.effectful_initializer_outcome(0, '', {}), 'fail')
        self.assertEqual(checks.effectful_initializer_outcome(1, 'error: Effectful array initializer expressions are not supported', {'Unexpected.class': 'hash'}), 'fail')
        self.assertEqual(checks.effectful_initializer_outcome(1, 'error: Unsupported arbitrary function body', {}), 'fail')
        self.assertEqual(checks.effectful_initializer_outcome(1, 'error: Bad initializer type', {}), 'fail')

    def test_app_sources_match_snapshot_manifest(self):
        inventory = json.loads((checks.ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        for name in [checks.BOOK_COLOR_SOURCE, checks.READING_SOURCE, checks.LEVENSHTEIN_SOURCE, checks.PARSER_SOURCE, *checks.READING_JAVA_SOURCES]:
            self.assertEqual(checks.sha(checks.SNAPSHOT / name), inventory[name])

    def test_required_independent_boundaries(self):
        driver = (checks.FIXTURES / 'DeclarationsCheck.java').read_text()
        for evidence in ['UninitializedPropertyAccessException', 'IndexOutOfBoundsException', 'getMessage().equals("original")', 'finallyOverride()', 'new Parent(4)', 'new Child(4)', 'new Outer.Nested', 'events("12123")', 'events("332111")', 'localFunctions(2) == 52', 'new PrivateCapture(7)', 'sourceReceiver(receiver', 'javaBase.compute() == 12', 'caught == original', 'nestedCatchFinally() == 5', 'finallyBypass() == 8', 'mutableReference(true).compute() == 9', 'tryReference(false).compute() == 5', 'new SecondaryCollision(5)', '.protectedCapture() == 5', 'BodyObject.INSTANCE.weighted']:
            self.assertIn(evidence, driver)
        protocols = (checks.FIXTURES / 'Protocols.kt').read_text()
        for evidence in ['provideDelegate', 'owner !== expectedOwner', 'property.name', 'provisions != 1', 'operator fun getValue', 'operator fun setValue']:
            self.assertIn(evidence, protocols)
        boundaries = {item['id'] for item in checks.BOUNDARIES}
        self.assertTrue({'contains_operator_modifier', 'get_operator_modifier', 'set_operator_modifier', 'generic_set_type', 'private_access', 'protected_access', 'final_class', 'final_override', 'lateinit_initializer', 'local_function_scope', 'local_capture_scope', 'break_across_lambda', 'continue_across_lambda', 'private_inline_import', 'protected_inline_call'} <= boundaries)


if __name__ == '__main__':
    unittest.main()
