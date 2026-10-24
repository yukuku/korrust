import json
from pathlib import Path
import re
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import check_androidbible_twenty as twenty


class TwentyFeatureEvidenceTests(unittest.TestCase):
    def test_targets_are_preserved_real_sources_with_real_dependency(self):
        inventory = json.loads((twenty.ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        self.assertEqual(set(twenty.TARGETS), {"PlaybackState", "SongPlaybackState"})
        self.assertTrue(twenty.MODEL.endswith("/AudioLogEntry.kt"))
        for source in [twenty.MODEL, *twenty.TARGETS.values()]:
            with self.subTest(source=source):
                self.assertEqual(twenty.sha(twenty.SNAPSHOT / source), inventory[source])

    def test_twenty_distinct_features_have_independent_java_and_kotlin_cases(self):
        self.assertEqual(len(twenty.FEATURES), 20)
        for name in ["TwentyCheck.java", "TwentyConsumer.kt"]:
            cases = re.findall(r'observe\("([a-z_]+)"', (twenty.FIXTURES / name).read_text())
            self.assertEqual(set(cases), set(twenty.FEATURES), name)
            self.assertEqual(len(cases), len(set(cases)), name)
        for name in twenty.SYNTHETICS:
            self.assertTrue((twenty.FIXTURES / (name + ".kt")).is_file())

    def test_boundaries_distinguish_invalid_from_officially_supported(self):
        cases = json.loads((twenty.FIXTURES / "rejections.json").read_text())
        self.assertEqual(len(cases), len({case["id"] for case in cases}))
        for case in cases:
            with self.subTest(case=case["id"]):
                self.assertRegex(case["id"], r"^[a-z][a-z0-9_]*$")
                self.assertTrue(case["source"].strip())
                self.assertTrue(case["candidate_diagnostic"])
                self.assertNotIn(case["candidate_diagnostic"].lower(), ("Boundary_" + case["id"] + ".kt").lower())
                if case["invalid"]:
                    self.assertTrue(case["official_diagnostic"])
                else:
                    self.assertIsNone(case["official_diagnostic"])
        self.assertTrue(any(case["invalid"] for case in cases))
        self.assertTrue(any(not case["invalid"] for case in cases))

    def test_mask_fixture_reaches_high_bit_with_two_slot_parameters(self):
        source = (twenty.FIXTURES / "Mask32.kt").read_text()
        self.assertEqual(len(re.findall(r"p\d+: (?:Int|Long) =", source)), 32)
        self.assertIn("p1: Long", source)
        self.assertIn("p16: Long", source)
        self.assertIn("p32 = 90", source)
        boundary = next(case for case in json.loads((twenty.FIXTURES / "rejections.json").read_text()) if case["id"] == "default_limit")
        self.assertEqual(len(re.findall(r"p\d+: Int =", boundary["source"])), 33)


if __name__ == "__main__":
    unittest.main()
