import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import check_androidbible_simple as simple


class SimpleAndroidBibleEvidenceTests(unittest.TestCase):
    def test_selected_sources_remain_the_exact_pinned_upstream_files(self):
        inventory = json.loads((simple.ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        for source in [simple.MODEL, *simple.TARGETS.values()]:
            with self.subTest(source=source):
                self.assertEqual(simple.sha(simple.SNAPSHOT / source), inventory[source])

    def test_success_trace_rejects_missing_repeated_or_external_class_outputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "stage"
            destination.mkdir()
            emitted = destination / "Object.class"
            emitted.write_bytes(b"class bytes")
            external = root / "External.class"
            external.write_bytes(b"unrelated bytes")
            self.assertEqual(simple.emitted_outputs(str(emitted) + "\n", destination), [emitted])
            for stdout in ["", str(destination / "Missing.class"), str(external), str(emitted) + "\n" + str(emitted)]:
                with self.subTest(stdout=stdout), self.assertRaises(AssertionError):
                    simple.emitted_outputs(stdout, destination)


if __name__ == "__main__":
    unittest.main()
