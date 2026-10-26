import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import check_androidbible_regression as regression


class AndroidBibleRegressionTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(regression.MANIFEST.read_text())
        self.compiler = regression.ROOT / 'target/release/korrust'

    def reports(self):
        suites = {name: {'outcome': 'pass', 'report': {'cases': [], 'commands': []}} for name in self.manifest['suites']}
        for target in self.manifest['cases']:
            for requirements in target['checks'].values():
                for requirement in requirements:
                    report = suites[requirement['suite']]['report']
                    if 'compile_source' in requirement:
                        report['commands'].append({'argv': [str(self.compiler), str(regression.ROOT / requirement['compile_source']), '-d', '/tmp/output'], 'exit_code': 0})
                    elif not any(case['id'] == requirement['case'] for case in report['cases']):
                        report['cases'].append({'id': requirement['case'], 'outcome': 'pass'})
        return suites

    def test_manifest_is_exact_pinned_baseline_and_restored_target(self):
        regression.validate_manifest(self.manifest)
        self.assertEqual(len(self.manifest['cases']), 21)
        self.assertEqual(len([case for case in self.manifest['cases'] if case['id'] == 'PlaybackState']), 1)
        incomplete = copy.deepcopy(self.manifest)
        incomplete['cases'].pop()
        with self.assertRaisesRegex(ValueError, 'exactly'):
            regression.validate_manifest(incomplete)

    def test_preserved_harness_cases_exist_in_dated_evidence(self):
        historical = {
            'baseline': '2026-10-24T094102516587Z-io-byte-arrays.json',
            'simple': '2026-10-24T094102515073Z-androidbible-simple-sources.json',
            'ten': '2026-10-24T094102560385Z-androidbible-ten-features.json',
            'twenty': '2026-10-24T094102532411Z-androidbible-twenty-features.json',
            'declarations': '2026-10-25T151327943975Z-declarations-twenty.json',
            'interop': '2026-10-24T094102511868Z-library-interop.json',
        }
        available = {}
        for suite, name in historical.items():
            path = regression.ROOT / 'reports/correctness' / name
            self.assertTrue(path.is_file(), name)
            available[suite] = {case['id'] for case in json.loads(path.read_text())['cases']}
        for target in self.manifest['cases']:
            for requirements in target['checks'].values():
                for requirement in requirements:
                    if 'case' in requirement and requirement['suite'] in available:
                        self.assertIn(requirement['case'], available[requirement['suite']], target['id'])

    def test_missing_consumer_assertions_remain_not_run(self):
        cases = regression.aggregate(self.manifest, self.reports(), self.compiler)
        self.assertTrue(all(case['outcome'] == 'pass' for case in cases))
        target = next(case for case in cases if case['id'].endswith('/BookColorUtil.kt'))
        self.assertEqual(target['checks']['kotlin_consumer']['outcome'], 'not_run')
        self.assertTrue(target['checks']['kotlin_consumer']['reason'])

    def test_missing_or_duplicate_required_case_fails_target(self):
        suites = self.reports()
        target = next(case for case in self.manifest['cases'] if case['id'] == 'PlaybackState')
        required = target['checks']['behavior'][0]['case']
        suites['twenty']['report']['cases'] = [case for case in suites['twenty']['report']['cases'] if case['id'] != required]
        cases = regression.aggregate(self.manifest, suites, self.compiler)
        self.assertEqual(next(case for case in cases if case['id'] == target['source'])['outcome'], 'fail')

    def test_deliberately_failing_fixture_fails_aggregate_even_with_exit_zero(self):
        with tempfile.TemporaryDirectory() as work:
            fixture = Path(work) / 'failure.py'
            fixture.write_text('import json\ntry:\n    assert 2 + 2 == 5\nexcept AssertionError:\n    print(json.dumps({"status":"completed", "compiler":{"binary_sha256":"fixture"}, "cases":[{"id":"java/rich_data_properties", "outcome":"fail"}]}))\n')
            result = subprocess.run([sys.executable, str(fixture)], capture_output=True, text=True, check=True)
        report = json.loads(result.stdout)
        suites = self.reports()
        suites['twenty']['outcome'] = regression.assess_suite(report, result.returncode, 'fixture')
        suites['twenty']['report']['cases'] = [report['cases'][0] if case['id'] == report['cases'][0]['id'] else case for case in suites['twenty']['report']['cases']]
        cases = regression.aggregate(self.manifest, suites, self.compiler)
        self.assertEqual(suites['twenty']['outcome'], 'fail')
        self.assertTrue(any(case['outcome'] == 'fail' for case in cases))
        self.assertEqual(next(case for case in cases if case['id'].endswith('/PlaybackState.kt'))['checks']['behavior']['outcome'], 'fail')

    def test_official_compile_cannot_establish_candidate_emission(self):
        report = {'commands': [{'argv': ['references/kotlinc/bin/kotlinc', str(regression.ROOT / 'example.kt')], 'exit_code': 0}]}
        self.assertFalse(regression.compile_evidence(report, 'example.kt', self.compiler))

    def test_completed_report_with_failure_outcome_cannot_pass(self):
        report = {'status': 'completed', 'compiler': {'binary_sha256': 'a'}, 'cases': [{'id': 'fixture', 'outcome': 'failure'}]}
        self.assertEqual(regression.assess_suite(report, 0, 'a'), 'fail')

    def test_compiler_mismatch_and_nonzero_exit_fail_suite(self):
        report = {'status': 'completed', 'compiler': {'binary_sha256': 'a'}, 'cases': []}
        self.assertEqual(regression.assess_suite(report, 0, 'b'), 'fail')
        self.assertEqual(regression.assess_suite(report, 1, 'a'), 'fail')


if __name__ == '__main__':
    unittest.main()
