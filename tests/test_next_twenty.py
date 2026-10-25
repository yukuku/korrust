import json
from pathlib import Path
import re
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import check_next_twenty as checks


class NextTwentyEvidenceTests(unittest.TestCase):
    def test_feature_inventory_is_unique_and_has_independent_java_assertions(self):
        ids = {item['id'] for item in checks.FEATURES}
        self.assertEqual(len(ids), 20)
        self.assertEqual(len(checks.FEATURES), 20)
        self.assertEqual(ids, set(checks.RESTRICTIONS))
        driver = (checks.FIXTURES / 'NextTwentyCheck.java').read_text()
        self.assertEqual(ids, set(re.findall(r'case "([a-z_]+)":', driver)))
        for item in checks.FEATURES:
            with self.subTest(feature=item['id']):
                self.assertTrue((checks.FIXTURES / item['fixture']).is_file())
                self.assertTrue(item['scope'])

    def test_feature_support_requires_all_observable_contracts_and_frozen_inputs(self):
        ident = 'float_comparisons'
        cases = [dict(id=kind + '/' + ident, outcome='pass') for kind in ['oracle', 'compile', 'java', 'source_abi', 'class_verification']]
        cases.append(dict(id='kotlin_consumer', outcome='pass'))
        self.assertEqual(checks.feature_status(cases, ident), 'partial')
        cases.append(dict(id='frozen_inputs', outcome='pass'))
        self.assertEqual(checks.feature_status(cases, ident), 'supported')
        for index in range(len(cases)):
            for outcome in ['fail', 'timeout', 'not_run', 'unsupported_language']:
                changed = [dict(case) for case in cases]
                changed[index]['outcome'] = outcome
                self.assertNotEqual(checks.feature_status(changed, ident), 'supported')

    def test_progression_support_requires_shadowing_evidence(self):
        for ident, shadow in [('descending_progressions', 'downTo'), ('stepped_progressions', 'step')]:
            cases = [dict(id=kind + '/' + ident, outcome='pass') for kind in ['oracle', 'compile', 'java', 'source_abi', 'class_verification']]
            cases.extend(dict(id=name, outcome='pass') for name in ['kotlin_consumer', 'frozen_inputs'])
            self.assertEqual(checks.feature_status(cases, ident), 'partial')
            cases.append(dict(id='shadow/' + shadow, outcome='pass'))
            if ident == 'descending_progressions':
                cases.extend(dict(id=name, outcome='pass') for name in ['source_abi/source_infix_extension', 'source_abi/source_infix_member', 'boundary/noninfix_extension'])
            self.assertEqual(checks.feature_status(cases, ident), 'supported')

    def test_source_citations_preserve_actual_app_snapshot(self):
        inventory = json.loads((checks.ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        names = {checks.APP_SOURCE, checks.APP_PROBE, checks.BOOK_COLOR_SOURCE}
        for suffix, needle in checks.CITATIONS.values():
            name = 'Alkitab/src/main/java/yuku/alkitab/' + suffix
            names.add(name)
            self.assertIn(needle, (checks.SNAPSHOT / name).read_text())
        for name in names:
            with self.subTest(source=name):
                self.assertEqual(checks.sha(checks.SNAPSHOT / name), inventory[name])

    def test_negative_results_require_semantic_rejection_and_no_outputs(self):
        boundary = next(item for item in checks.BOUNDARIES if item['id'] == 'immutable_compound')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: Cannot assign to immutable val', {}), 'pass')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: Cannot assign to immutable val', {'Bad.class': 'hash'}), 'fail')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: Unresolved function bad', {}), 'fail')
        self.assertEqual(checks.boundary_outcome(boundary, 0, '', {}), 'fail')
        self.assertEqual(checks.boundary_outcome(boundary, 1, 'error: unsupported expression', {}), 'unsupported_language')

    def test_required_runtime_boundaries_are_independent(self):
        driver = (checks.FIXTURES / 'NextTwentyCheck.java').read_text()
        for evidence in ['Integer.MAX_VALUE', 'Integer.MIN_VALUE', 'Long.MAX_VALUE', 'Long.MIN_VALUE', 'Float.NaN', 'Float.POSITIVE_INFINITY', 'Float.floatToIntBits', 'ConcurrentModificationException', 'NullPointerException', 'ClassCastException', 'iteratorCalls', 'events.equals("13")', 'events.equals("NV")']:
            self.assertIn(evidence, driver)
        self.assertIn('\\ud83d\\ude00', driver)
        self.assertIn('nullableFloatEq', driver)
        self.assertIn('equalObjects(Float.NaN, Float.NaN)', driver)


if __name__ == '__main__':
    unittest.main()
