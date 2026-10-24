import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import check_nowinandroid as nia


class NowInAndroidEvidenceTests(unittest.TestCase):
    def test_source_abi_checks_generic_serializer_and_marker(self):
        reference = """Compiled from "Key.kt"
public final class sample.Key implements androidx.navigation3.runtime.NavKey {
  public static final sample.Key INSTANCE;
    descriptor: Lsample/Key;
  public final kotlinx.serialization.KSerializer<sample.Key> serializer();
    descriptor: ()Lkotlinx/serialization/KSerializer;
}
"""
        candidate = reference.replace("Compiled from \"Key.kt\"", "Compiled from \"Other.kt\"").replace(
            "}\n", "  public static final kotlinx.serialization.KSerializer access$serializer();\n    descriptor: ()Lkotlinx/serialization/KSerializer;\n}\n")
        self.assertEqual(nia.source_abi(reference), nia.source_abi(reference.replace("Compiled from \"Key.kt\"", "Compiled from \"Other.kt\"")))
        self.assertNotEqual(nia.source_abi(reference), nia.source_abi(candidate))
        self.assertNotEqual(nia.source_abi(reference), nia.source_abi(reference.replace("KSerializer<sample.Key>", "KSerializer<java.lang.Object>")))
        self.assertNotEqual(nia.source_abi(reference), nia.source_abi(reference.replace(" implements androidx.navigation3.runtime.NavKey", "")))
        with self.assertRaises(AssertionError):
            nia.source_abi(reference.replace("descriptor: Lsample/Key;", "malformed"))

    def test_runtime_results_require_distinct_successful_cases(self):
        success = '{"id":"direct_serializer","outcome":"pass"}\n'
        self.assertEqual(nia.parse_observations(success)[0]["id"], "direct_serializer")
        for invalid in ["", success + success, '{"id":"x","outcome":"fail"}', "compiler warning\n" + success]:
            with self.subTest(invalid=invalid), self.assertRaises((AssertionError, json.JSONDecodeError)):
                nia.parse_observations(invalid)

    def test_freeze_detects_changes_and_tree_excludes_ownership_journal(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            artifact = directory / "Object.class"
            artifact.write_bytes(b"frozen")
            hashes = nia.file_hashes([artifact])
            nia.check_recorded_hashes(hashes)
            journal = directory / ".korrust/index"
            journal.parent.mkdir()
            journal.write_text("bookkeeping")
            self.assertEqual(set(nia.tree_hashes(directory)), {"Object.class"})
            artifact.write_bytes(b"changed")
            with self.assertRaisesRegex(AssertionError, "Frozen input changed"):
                nia.check_recorded_hashes(hashes)


if __name__ == "__main__":
    unittest.main()
