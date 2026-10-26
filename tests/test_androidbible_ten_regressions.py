import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import check_androidbible_ten as ten


class TenSupportedRegressionTests(unittest.TestCase):
    def test_nullable_double_preserves_original_source_and_selects_positive_checks(self):
        cases = json.loads((ten.FIXTURES / 'rejections.json').read_text())
        case = next(case for case in cases if case['id'] == 'nullable_double')
        self.assertEqual(case['source'], 'fun f(value: Double?): Double? = value')
        self.assertNotIn('candidate_diagnostic', case)
        self.assertTrue((ten.FIXTURES / case['supported_regression']['java_fixture']).is_file())
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            (work / 'rejections.json').write_text(json.dumps([case]))
            harness = ten.Harness.__new__(ten.Harness)
            harness.work = work
            harness.supported_regression = Mock()
            harness.compile = Mock()
            with patch.object(ten, 'FIXTURES', work):
                harness.boundaries()
            source = work / 'Boundary_nullable_double.kt'
            self.assertEqual(source.read_text(), case['source'] + '\n')
            harness.supported_regression.assert_called_once_with(case, source)
            harness.compile.assert_not_called()

    def test_disagreeing_positive_observations_fail_without_recording_pass(self):
        case = next(case for case in json.loads((ten.FIXTURES / 'rejections.json').read_text()) if case['id'] == 'nullable_double')
        with tempfile.TemporaryDirectory() as temporary:
            harness = ten.Harness.__new__(ten.Harness)
            harness.work = Path(temporary)
            harness.outputs = {'official': {'java': harness.work}, 'candidate': {'java': harness.work}}
            harness.stdlib = harness.work / 'stdlib.jar'
            harness.javac, harness.java = 'javac', 'java'
            harness.compile = Mock()
            harness.case = Mock()
            harness.run = Mock(side_effect=[SimpleNamespace(stdout=''), SimpleNamespace(stdout='{"id":"nullable_double","checks":21}'), SimpleNamespace(stdout='{"id":"nullable_double","checks":20}')])
            with self.assertRaisesRegex(AssertionError, 'observations differ'):
                harness.supported_regression(case, harness.work / 'Boundary_nullable_double.kt')
            harness.case.assert_not_called()
            self.assertEqual(harness.compile.call_count, 2)


if __name__ == '__main__':
    unittest.main()
