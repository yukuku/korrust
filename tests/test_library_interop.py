import json
from pathlib import Path
import re
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import check_library_interop as interop


class LibraryInteropEvidenceTests(unittest.TestCase):
    def test_twenty_features_have_fixed_java_and_kotlin_observations(self):
        features = interop.FEATURES
        ids = {item["id"] for item in features}
        self.assertEqual(len(features), 20)
        self.assertEqual(len(ids), 20)
        self.assertEqual(ids, set(interop.CONSUMER_CALLS))
        self.assertEqual(ids, set(interop.RESTRICTIONS))
        driver = (interop.FIXTURES / "LibraryInteropCheck.java").read_text()
        self.assertEqual(ids, set(re.findall(r'case "([a-z_]+)":', driver)))
        for item in features:
            with self.subTest(feature=item["id"]):
                self.assertTrue((interop.FIXTURES / item["fixture"]).is_file())
                self.assertTrue(item["scope"])

    def test_support_requires_compile_runtime_abi_and_kotlin_consumer(self):
        ident = "jar_classpath"
        cases = [{"id": "compile/" + ident, "outcome": "pass"}]
        self.assertEqual(interop.feature_status(cases, ident), "partial")
        cases.extend({"id": name, "outcome": "pass"} for name in ["java/" + ident, "source_abi/" + ident, "class_verification/" + ident, "kotlin_consumer", "frozen_inputs"])
        self.assertEqual(interop.feature_status(cases, ident), "supported")
        cases[0]["outcome"] = "unsupported_language"
        self.assertEqual(interop.feature_status(cases, ident), "unsupported")
        cases[0]["outcome"] = "fail"
        self.assertEqual(interop.feature_status(cases, ident), "partial")

    def test_unsupported_diagnostic_does_not_turn_valid_source_into_pass(self):
        self.assertEqual(interop.rejection_outcome("error: unsupported generic return"), "unsupported_language")
        self.assertEqual(interop.rejection_outcome("error: lambdas are not yet supported"), "unsupported_language")
        self.assertEqual(interop.rejection_outcome("error: unresolved function"), "fail")
        self.assertEqual(interop.rejection_outcome("segmentation fault"), "fail")

    def test_invalid_boundary_requires_relevant_semantic_diagnostic(self):
        boundary = {"invalid": True, "candidate_diagnostic_regex": "upper bound"}
        self.assertEqual(interop.boundary_outcome(boundary, 1, "error: Unresolved local: interop", {}), "fail")
        self.assertEqual(interop.boundary_outcome(boundary, 1, "error: Type argument violates upper bound of T", {}), "pass")
        self.assertEqual(interop.boundary_outcome(boundary, 1, "error: Unsupported generic declaration", {}), "unsupported_language")
        self.assertEqual(interop.boundary_outcome(boundary, 1, "error: Type argument violates upper bound of T", {"Bad.class": "hash"}), "fail")

    def test_negative_contracts_include_inline_restrictions_and_generic_bounds(self):
        cases = json.loads((interop.FIXTURES / "rejections.json").read_text())
        ids = {item["id"] for item in cases}
        self.assertEqual(len(cases), len(ids))
        self.assertTrue({"wrong_generic_bound", "annotated_nullable_dereference", "noinline_nonlocal_return", "crossinline_nonlocal_return", "projected_list_write"}.issubset(ids))
        self.assertTrue(all(item["source"].strip() for item in cases))
        self.assertTrue(all(item["candidate_diagnostic_regex"] for item in cases))
        self.assertTrue(any(item["invalid"] for item in cases))
        self.assertTrue(any(not item["invalid"] for item in cases))

    def test_runtime_covers_shared_capture_and_receiver_order(self):
        capture = (interop.FIXTURES / "Captures.kt").read_text()
        self.assertIn("val alias = block", capture)
        self.assertIn("alias(3)", capture)
        self.assertIn("total += value", capture)
        driver = (interop.FIXTURES / "LibraryInteropCheck.java").read_text()
        self.assertIn('eq(JavaApi.events, "Ba")', driver)
        self.assertIn('eq(JavaApi.events, "BA")', driver)
        self.assertIn('eq(JavaApi.events, "R")', driver)
        self.assertIn('eq(JavaApi.events, "AB")', driver)

    def test_original_app_target_and_dependency_match_preserved_inventory(self):
        inventory = json.loads((interop.ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        for name in [interop.APP_SOURCE, interop.APP_JAVA]:
            with self.subTest(source=name):
                self.assertEqual(interop.sha(interop.SNAPSHOT / name), inventory[name])


if __name__ == "__main__":
    unittest.main()
