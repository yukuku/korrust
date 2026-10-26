import importlib.util
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
spec = importlib.util.spec_from_file_location("check_compatibility_calls", ROOT / "tools/check_compatibility_calls.py")
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


class DiagnosticClassificationTests(unittest.TestCase):
    def test_invalid_kotlin_requires_actual_rejection_without_emission(self):
        self.assertEqual(checks.rejected_outcome(1, "error: Type mismatch", [], invalid=True), "pass")
        self.assertEqual(checks.rejected_outcome(0, "error: Type mismatch", [], invalid=True), "fail")
        self.assertEqual(checks.rejected_outcome(1, "error: Type mismatch", ["Bad.class"], invalid=True), "fail")
        self.assertEqual(checks.rejected_outcome(1, "", [], invalid=True), "fail")
        self.assertEqual(checks.rejected_outcome(1, "JVM could not start", [], invalid=True), "fail")
        self.assertEqual(checks.rejected_outcome(-9, "error: Type mismatch", [], invalid=True), "fail")

    def test_invalid_boundary_cannot_pass_by_incidental_parser_failure(self):
        pattern = checks.NEGATIVE_DIAGNOSTICS["InvalidPrimitiveSpread"]
        self.assertEqual(checks.rejected_outcome(1, "InvalidPrimitiveSpread.kt:2:1: error: Expected identifier", [], invalid=True, semantic_pattern=pattern), "fail")
        self.assertEqual(checks.rejected_outcome(1, "error: Spread array type mismatch", [], invalid=True, semantic_pattern=pattern), "pass")

    def test_unsupported_invalid_fixture_does_not_count_as_pass(self):
        self.assertEqual(checks.rejected_outcome(1, "error: Unsupported spread argument", [], invalid=True), "unsupported_language")

    def test_positive_reference_accepted_case_cannot_pass_by_rejection(self):
        self.assertEqual(checks.rejected_outcome(1, "error: Type mismatch", []), "fail")
        self.assertEqual(checks.rejected_outcome(1, "error: Spread arguments are not yet supported", []), "unsupported_language")


if __name__ == "__main__":
    unittest.main()
