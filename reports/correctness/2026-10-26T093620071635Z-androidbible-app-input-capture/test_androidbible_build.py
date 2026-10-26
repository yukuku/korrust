import importlib.util
from pathlib import Path
import tempfile
import sys
import unittest
from unittest import mock
import json
import subprocess
from types import SimpleNamespace


TOOL = Path(__file__).resolve().parents[1] / "tools/build_androidbible.py"
sys.path.insert(0, str(TOOL.parent))
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

    def test_app_module_is_capture_only(self):
        with self.assertRaisesRegex(ValueError, "available only with --capture-only"):
            build.build(SimpleNamespace(modules=["Alkitab"], capture_only=False))

    def test_missing_generated_task_outputs_are_setup_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            generated = str(Path(temporary) / "generated/ksp/kotlin")
            model = {"compilation_tasks": {"compilePlainDebugKotlin": {
                "source_generation_dependencies": [{"task": ":Alkitab:kspPlainDebugKotlin", "outputs": [generated]}]
            }}}
            inputs = build.resolved_inputs(model)
            self.assertEqual(inputs["missing_required_inputs"], [generated])
            self.assertIn("source_generation", inputs["entries"][0]["roles"][0])

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

    def test_stage_publication_removes_stale_owned_outputs_and_retains_metadata(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage, destination, manifest = root / "stage", root / "classes", root / "publication.json"
            stage.mkdir()
            (stage / "Old.class").write_bytes(b"old")
            build.publish_stage(stage, destination, manifest)
            (stage / "Old.class").unlink()
            (stage / "New.class").write_bytes(b"new")
            (stage / "META-INF").mkdir()
            (stage / "META-INF/module.kotlin_module").write_bytes(b"metadata")
            (stage / ".korrust").mkdir()
            (stage / ".korrust/private-index").write_bytes(b"private")
            build.publish_stage(stage, destination, manifest)
            self.assertEqual(build.publication_files(destination), {
                "New.class": build.digest(stage / "New.class"),
                "META-INF/module.kotlin_module": build.digest(stage / "META-INF/module.kotlin_module"),
            })
            self.assertFalse((destination / ".korrust").exists())

    def test_publication_refuses_unowned_and_modified_outputs_before_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stage, destination, manifest = root / "stage", root / "classes", root / "publication.json"
            stage.mkdir()
            destination.mkdir()
            (stage / "Owned.class").write_bytes(b"new")
            (destination / "Foreign.class").write_bytes(b"foreign")
            with self.assertRaisesRegex(ValueError, "unowned"):
                build.publish_stage(stage, destination, manifest)
            self.assertFalse((destination / "Owned.class").exists())
            (destination / "Foreign.class").unlink()
            build.publish_stage(stage, destination, manifest)
            (destination / "Owned.class").write_bytes(b"changed")
            with self.assertRaisesRegex(ValueError, "externally modified"):
                build.publish_stage(stage, destination, manifest)
            self.assertEqual((destination / "Owned.class").read_bytes(), b"changed")
            (destination / "Owned.class").unlink()
            with self.assertRaisesRegex(ValueError, "missing"):
                build.publish_stage(stage, destination, manifest)

    def test_publication_refuses_manifest_escape_and_symbolic_link(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "classes"
            destination.mkdir()
            manifest = root / "publication.json"
            manifest.write_text(json.dumps({"../outside": "checksum"}))
            with self.assertRaisesRegex(ValueError, "Unsafe publication manifest"):
                build.preflight_publication(destination, manifest)
            manifest.unlink()
            outside = root / "outside"
            outside.write_bytes(b"data")
            (destination / "Link.class").symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "symbolic link"):
                build.preflight_publication(destination, manifest)

    def make_request(self, root):
        source_root = root / "Module/src/debug/kotlin"
        source_root.mkdir(parents=True)
        sources = [source_root / "B.kt", source_root / "A.kt"]
        for source in sources:
            source.write_text("object " + source.stem)
        compiler = root / "compiler"
        compiler.write_bytes(b"compiler identity")
        request = {"module": "Module", "module_name": "Module.debug", "work_dir": str(root),
                   "task": ":Module:compileDebugKotlin", "compiler": str(compiler),
                   "sources": [str(source) for source in sources], "source_roots": [str(source_root)],
                   "classpath": [], "stage": str(root / "stage"), "destination": str(root / "classes"),
                   "trace": str(root / "trace.json")}
        path = root / "request.json"
        path.write_text(json.dumps(request))
        return path, request

    def test_compile_request_invokes_one_sorted_module_batch(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path, request = self.make_request(root)
            def compile_batch(command, **kwargs):
                stage = Path(request["stage"])
                stage.mkdir()
                (stage / "Combined.class").write_bytes(b"candidate")
                return subprocess.CompletedProcess(command, 0, "", "")
            with mock.patch.object(build.subprocess, "run", side_effect=compile_batch) as run:
                self.assertEqual(build.compile_request(path), 0)
            self.assertEqual(run.call_count, 1)
            command = run.call_args.args[0]
            self.assertEqual(command[1:3], sorted(request["sources"]))
            self.assertEqual(command[-2:], ["--module-name", "Module.debug"])
            trace = json.loads((root / "trace.json").read_text())
            self.assertEqual(len(trace["sources"]), 2)
            self.assertFalse(trace["fallback"])
            self.assertEqual((root / "classes/Combined.class").read_bytes(), b"candidate")

    def test_failed_batch_preserves_published_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path, request = self.make_request(root)
            stage = Path(request["stage"])
            stage.mkdir()
            (stage / "Previous.class").write_bytes(b"previous")
            destination = Path(request["destination"])
            build.publish_stage(stage, destination, stage.parent / (stage.name + "-publication.json"))
            with mock.patch.object(build.subprocess, "run", return_value=subprocess.CompletedProcess([], 7, "", "rejected")):
                self.assertEqual(build.compile_request(path), 7)
            self.assertEqual((destination / "Previous.class").read_bytes(), b"previous")

    def test_unassessed_semantic_options_reject(self):
        with self.assertRaisesRegex(ValueError, "compiler arguments"):
            build.validate_compile_options({"compiler_options": {"freeCompilerArgs": ["-Xjvm-default=disable"]}})
        with self.assertRaisesRegex(ValueError, "plugins"):
            build.validate_compile_options({"compiler_plugin_options": ["plugin:example:key=value"]})

    def test_module_evidence_rejects_partial_or_per_file_batch_traces(self):
        from check_androidbible_module import checked_compile_invocations
        request = {"sources": ["/work/B.kt", "/work/A.kt"], "stage": "/stage",
                   "module_name": "module", "classpath": ["/second.jar", "/first.jar"]}
        trace = {"schema_version": 2, "sources": [{"path": "/work/A.kt", "sha256": "a"},
                  {"path": "/work/B.kt", "sha256": "b"}], "invocations": [{"sources": request["sources"],
                  "command": ["compiler", "/work/A.kt", "/work/B.kt", "-d", "/stage", "--module-name", "module",
                              "-cp", "/second.jar:/first.jar"], "returncode": 0}]}
        expected = {"A.kt": "a", "B.kt": "b"}
        self.assertEqual(len(checked_compile_invocations(trace, request, "compiler", expected, "/work")), 1)
        trace["invocations"][0]["command"].remove("/work/B.kt")
        with self.assertRaisesRegex(AssertionError, "complete module batch"):
            checked_compile_invocations(trace, request, "compiler", expected, "/work")


if __name__ == "__main__":
    unittest.main()
