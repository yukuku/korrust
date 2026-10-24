import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import check_androidbible_ten as ten


class TenFeatureEvidenceTests(unittest.TestCase):
    def test_three_targets_are_exact_preserved_snapshot_sources(self):
        inventory = json.loads((ten.ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        self.assertEqual(len(ten.TARGETS), 3)
        for source in [ten.MODEL, *ten.TARGETS.values()]:
            with self.subTest(source=source):
                self.assertEqual(ten.sha(ten.SNAPSHOT / source), inventory[source])

    def test_relevance_scan_masks_nested_comments_and_escaped_templates(self):
        source = '''/* Long /* null ?: when { */ data class */
// val x: List<String> = null
val text = "Long ?: when { \\${escaped} ${actual}"
val c = '?'
class Entry(val text: String?, private val hidden: Long)
fun f(values: List<String?>): String = values[0] ?: "fallback"
'''
        result = ten.syntax_observations(source)
        self.assertEqual(len(result["long_scalars"]), 1)
        self.assertEqual(len(result["data_classes"]), 0)
        self.assertEqual(len(result["braced_templates"]), 1)
        self.assertEqual(len(result["public_constructor_val"]), 1)
        self.assertEqual(len(result["string_lists"]), 1)
        self.assertEqual(len(result["elvis"]), 1)
        self.assertEqual(len(result["subjectless_when"]), 0)


if __name__ == "__main__":
    unittest.main()
