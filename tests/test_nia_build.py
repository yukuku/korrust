import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


TOOL = Path(__file__).resolve().parents[1] / "tools/build_nowinandroid.py"
SPEC = importlib.util.spec_from_file_location("build_nowinandroid", TOOL)
build = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build)


class NiaBuildTests(unittest.TestCase):
    def test_module_inventory_includes_android_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            module = root / "feature/bookmarks/api"
            (module / "src/main/res/values").mkdir(parents=True)
            (module / "build.gradle.kts").write_text("plugins {}")
            (module / "src/main/res/values/strings.xml").write_text("<resources/>")
            (module / "src/main/Main.kt").write_text("object Main")
            inventory = build.source_inventory(root, ["feature:bookmarks:api"])
            self.assertEqual(len(inventory), 3)
            self.assertIn("feature/bookmarks/api/src/main/res/values/strings.xml", inventory)
            self.assertEqual(build.module_inventory(inventory, "feature:bookmarks:api")
                             ["production_kotlin_sources"], ["feature/bookmarks/api/src/main/Main.kt"])

    def test_compile_request_rejects_incomplete_production_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            sources = root / "feature/bookmarks/api/src/main"
            sources.mkdir(parents=True)
            (sources / "Main.kt").write_text("object Main")
            request = root / "request.json"
            request.write_text(json.dumps({"work_dir": str(root), "module": "feature:bookmarks:api",
                                           "sources": []}))
            with self.assertRaisesRegex(ValueError, "differ from selected module inventory"):
                build.compile_request(request)

    def test_unassessed_semantic_compiler_option_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Unassessed selected Kotlin compiler arguments"):
            build.validate_compile_options({"jvm_target": "11", "free_compiler_args": ["-Xjvm-default=disable"]})

    def test_all_flavored_production_compile_tasks_selected(self):
        self.assertEqual(set(build.COMPILE_TASKS), {
            "compileDemoDebugKotlin", "compileDemoReleaseKotlin",
            "compileProdDebugKotlin", "compileProdReleaseKotlin",
        })


if __name__ == "__main__":
    unittest.main()
