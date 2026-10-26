import importlib.util
from pathlib import Path
import tempfile
import unittest


TOOL = Path(__file__).resolve().parents[1] / "tools/build_androidbible.py"
SPEC = importlib.util.spec_from_file_location("build_androidbible", TOOL)
build = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(build)


class AndroidBibleBuildTests(unittest.TestCase):
    def test_resolved_inputs_retains_missing_generated_dependencies(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "Main.kt"
            source.write_text("object Main")
            dependency = root / "dependency.jar"
            dependency.write_bytes(b"dependency")
            generated = root / "build/generated/Generated.kt"
            model = {"compilation_tasks": {"compileReleaseKotlin": {
                "sources": [str(source), str(generated)],
                "compile_classpath": [str(dependency)],
                "compiler_plugin_classpath": [],
            }}}
            inputs = build.resolved_inputs(model)
            self.assertEqual(inputs["missing_required_inputs"], [str(generated)])
            records = {item["path"]: item for item in inputs["entries"]}
            self.assertEqual(records[str(source)]["sha256"], build.digest(source))
            self.assertEqual(records[str(dependency)]["roles"], ["compileReleaseKotlin:compile_classpath"])

    def test_directory_dependencies_have_content_fingerprints(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "META-INF").mkdir()
            metadata = root / "META-INF/library.kotlin_module"
            metadata.write_bytes(b"package metadata")
            record = build.resolved_inputs({"resolved_kotlin_compiler_classpath": [str(root)]})["entries"][0]
            self.assertEqual(record["files_sha256"], {"META-INF/library.kotlin_module": build.digest(metadata)})

    def test_sdk_and_generated_dependency_failures_are_setup_failures(self):
        for log in ["SDK location not found", "Could not resolve generated dependency", "Could not download AGP", "PKIX path building failed"]:
            with self.subTest(log=log):
                self.assertEqual(build.failure_category(log, False), "setup_failure")
        self.assertEqual(build.failure_category("Kotlin compilation failed", True), "build_failure")

    def test_path_mapping_preserves_order_and_external_dependencies(self):
        model = {"compile_classpath": ["/work/Module/build/b.jar", "/external/a.jar", "/work/Module/build/a.jar"]}
        mapped = build.mapped_model(model, Path("/work/Module/build"), Path("/snapshot"))
        self.assertEqual(mapped["compile_classpath"], ["/snapshot/b.jar", "/external/a.jar", "/snapshot/a.jar"])


if __name__ == "__main__":
    unittest.main()
