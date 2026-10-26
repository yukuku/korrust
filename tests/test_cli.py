import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
COMPILER = Path(os.environ.get("KORRUST_COMPILER", ROOT / "target/release/korrust")).resolve()


class OutputLifecycleTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="korrust-cli-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.source = self.root / "Example.kt"
        self.destination = self.root / "output"

    def compile(self, text, *, source=None, destination=None, success=True):
        source = source or self.source
        destination = destination or self.destination
        source.parent.mkdir(parents=True, exist_ok=True)
        source.write_text(text)
        result = subprocess.run(
            [str(COMPILER), str(source), "-d", str(destination)],
            capture_output=True, text=True, timeout=30,
        )
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result

    def classes(self, destination=None):
        destination = destination or self.destination
        return {str(path.relative_to(destination)): path.read_bytes()
                for path in destination.rglob("*.class")}

    def snapshot(self, destination=None):
        destination = destination or self.destination
        return {str(path.relative_to(destination)): path.read_bytes()
                for path in destination.rglob("*") if path.is_file()}

    def assert_preserved(self, text, *, source=None):
        before = self.snapshot()
        result = self.compile(text, source=source, success=False)
        self.assertEqual(self.snapshot(), before)
        return result

    def rewrite_index(self, body):
        digest = 0xcbf29ce484222325
        for byte in body.encode():
            digest = ((digest ^ byte) * 0x100000001b3) & ((1 << 64) - 1)
        (self.destination / ".korrust/index").write_text(
            f"korrust-output-v1\t{digest:016x}\n{body}"
        )

    def test_repeated_removal_rename_and_package_change_match_clean_compilation(self):
        versions = [
            "package original; object Root { object A { object Deep { const val VALUE = 1 } }; object B }",
            "package original; object Root { object A { const val VALUE = 2 } }",
            "package original; object Root { object Renamed { const val VALUE = 3 } }",
            "package moved; object Changed { object Renamed { const val VALUE = 4 } }",
            "object Final { const val VALUE = 5 }",
        ]
        for number, source in enumerate(versions):
            with self.subTest(version=number):
                result = self.compile(source)
                clean = self.root / f"clean-{number}"
                self.compile(source, destination=clean)
                self.assertEqual(self.classes(), self.classes(clean))
                self.assertEqual(len(result.stdout.splitlines()), len(self.classes()))
        self.assertEqual(set(self.classes()), {"Final.class"})

    def test_object_supplier_removal_matches_clean_compilation(self):
        versions = [
            "object Buffers { private val bytes = ThreadLocal.withInitial { ByteArray(3) }; private val chars = ThreadLocal.withInitial { CharArray(5) }; @JvmStatic fun size(): Int = bytes.get()!!.size + chars.get()!!.size }",
            "object Buffers { private val chars = ThreadLocal.withInitial { CharArray(7) }; @JvmStatic fun size(): Int = chars.get()!!.size }",
            "object Buffers { const val SIZE = 9 }",
            "package moved; object Renamed { const val SIZE = 11 }",
        ]
        for number, source in enumerate(versions):
            with self.subTest(version=number):
                self.compile(source)
                clean = self.root / f"supplier-clean-{number}"
                self.compile(source, destination=clean)
                self.assertEqual(self.snapshot(), self.snapshot(clean))
                if number == 0:
                    self.assertEqual(len(self.classes()), 3)
                elif number == 1:
                    self.assertEqual(len(self.classes()), 2)
        self.assertEqual(set(self.classes()), {"moved/Renamed.class"})

    def test_package_mapping_cleanup_matches_clean_compilation(self):
        versions = [
            "package first; fun value(): Int { return 1 }",
            "package second; fun value(): Int { return 2 }",
            "object Changed",
        ]
        for number, source in enumerate(versions):
            self.compile(source)
            clean = self.root / f"mapping-clean-{number}"
            self.compile(source, destination=clean)
            self.assertEqual(self.snapshot(), self.snapshot(clean))
        self.assertEqual(list(self.destination.rglob("*.kotlin_module")), [])

    def test_multiple_source_package_mappings_survive_recompilation(self):
        self.compile("package sample; fun first(): Int { return 1 }")
        other = self.root / "Other.kt"
        self.compile("package sample; fun second(): Int { return 2 }", source=other)
        mappings = {p.name: p.read_bytes() for p in self.destination.rglob("*.kotlin_module")}
        self.assertEqual(len(mappings), 2)
        self.compile("package moved; fun first(): Int { return 3 }")
        current = {p.name: p.read_bytes() for p in self.destination.rglob("*.kotlin_module")}
        self.assertEqual(len(current), 2)
        self.assertEqual(len(mappings.keys() & current.keys()), 1)
        for name in mappings.keys() & current.keys():
            self.assertEqual(mappings[name], current[name])

    def test_long_package_uses_bounded_mapping_filename(self):
        package = ".".join(["abcdefghij"] * 20)
        self.compile(f"package {package}; fun value(): Int {{ return 1 }}")
        mapping, = self.destination.rglob("*.kotlin_module")
        self.assertLessEqual(len(mapping.name.encode()), 255)
        self.assertIn(package.replace(".", "/") + "/ExampleKt.class", self.classes())
        self.compile("object Changed")
        self.assertFalse(mapping.exists())

    def test_changed_package_mapping_rejects_without_changes(self):
        self.compile("fun value(): Int { return 1 }")
        mapping, = self.destination.rglob("*.kotlin_module")
        mapping.write_bytes(b"changed externally")
        self.assert_preserved("object Changed")

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_package_mapping_directory_symlink_rejects(self):
        self.destination.mkdir()
        outside = self.root / "outside"
        outside.mkdir()
        (self.destination / "META-INF").symlink_to(outside, target_is_directory=True)
        self.compile("fun value(): Int { return 1 }", success=False)
        self.assertEqual(list(outside.iterdir()), [])

    def test_interface_helpers_and_anonymous_outputs_match_clean_recompilation(self):
        versions = [
            "interface Root { fun value(): Int { return 7 }; interface Child : Root { fun other(): Int }; companion object { val EMPTY = object : Root { override fun value(): Int { return 0 } } } }",
            "interface Root { fun value(): Int }",
            "package moved; interface Renamed { fun count(): Int }",
        ]
        for number, source in enumerate(versions):
            self.compile(source)
            clean = self.root / f"interface-clean-{number}"
            self.compile(source, destination=clean)
            self.assertEqual(self.snapshot(), self.snapshot(clean))
            if number == 0:
                self.assertEqual(set(self.classes()), {
                    "Root.class", "Root$DefaultImpls.class", "Root$Child.class",
                    "Root$Child$DefaultImpls.class", "Root$Companion.class",
                    "Root$Companion$EMPTY$1.class",
                })
        self.assertEqual(set(self.classes()), {"moved/Renamed.class"})

    def test_invalid_source_preserves_previous_outputs_and_index(self):
        self.compile("object Root { object Child }")
        self.assert_preserved("object Root { val invalid: Int = true }")

    def test_invalid_source_does_not_create_destination(self):
        self.compile("object Root { val invalid: Int = true }", success=False)
        self.assertFalse(self.destination.exists())

    def test_cross_source_collision_preserves_both_sources(self):
        self.compile("object Root { object Child }")
        other = self.root / "Other.kt"
        self.compile("object Other", source=other)
        result = self.assert_preserved("object Root", source=other)
        self.assertIn("different source", result.stderr)

    def test_unowned_class_is_not_adopted_or_overwritten(self):
        self.destination.mkdir()
        (self.destination / "Root.class").write_bytes(b"unowned data")
        self.compile("object Root", success=False)
        self.assertEqual((self.destination / "Root.class").read_bytes(), b"unowned data")
        self.assertFalse((self.destination / ".korrust/index").exists())

    def test_unrelated_outputs_survive_stale_cleanup(self):
        self.compile("object Root { object Child }")
        other = self.root / "Other.kt"
        self.compile("object Other { const val VALUE = 7 }", source=other)
        unrelated = self.destination / "untouched.class"
        unrelated.write_bytes(b"foreign")
        other_bytes = (self.destination / "Other.class").read_bytes()
        self.compile("object Renamed")
        self.assertEqual(unrelated.read_bytes(), b"foreign")
        self.assertEqual((self.destination / "Other.class").read_bytes(), other_bytes)
        self.assertEqual(set(self.classes()), {"Renamed.class", "Other.class", "untouched.class"})

    def test_corrupt_index_rejects_without_changes(self):
        self.compile("object Root { object Child }")
        (self.destination / ".korrust/index").write_text("corrupt\n")
        self.assert_preserved("object Renamed")

    def test_traversal_and_absolute_manifest_paths_reject_without_changes(self):
        self.compile("object Root { object Child }")
        outside = self.root / "victim.class"
        outside.write_bytes(b"preserve")
        index = self.destination / ".korrust/index"
        valid = index.read_text()
        record = valid.splitlines()[1].split("\t")
        for path in ["../victim.class", str(outside), "pkg/../../victim.class", ".korrust/index"]:
            with self.subTest(path=path):
                modified = [record[0], path.encode().hex(), record[2]]
                self.rewrite_index("\t".join(modified) + "\n")
                result = self.assert_preserved("object Renamed")
                self.assertIn("Invalid output ownership index path", result.stderr)
                self.assertEqual(outside.read_bytes(), b"preserve")
        index.write_text(valid)

    def test_duplicate_manifest_record_rejects_without_changes(self):
        self.compile("object Root")
        record = (self.destination / ".korrust/index").read_text().splitlines()[1]
        self.rewrite_index(record + "\n" + record + "\n")
        self.assert_preserved("object Renamed")

    def test_changed_owned_output_rejects_without_changes(self):
        self.compile("object Root { object Child }")
        (self.destination / "Root$Child.class").write_bytes(b"changed externally")
        self.assert_preserved("object Renamed")

    def test_missing_owned_output_rejects_without_changes(self):
        self.compile("object Root { object Child }")
        (self.destination / "Root$Child.class").unlink()
        self.assert_preserved("object Renamed")

    def test_existing_lock_rejects_without_changes(self):
        self.compile("object Root")
        (self.destination / ".korrust/lock").write_text("held")
        self.assert_preserved("object Renamed")

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_package_symlink_cannot_escape_destination(self):
        self.compile("object Root { object Child }")
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "sentinel").write_bytes(b"preserve")
        (self.destination / "escape").symlink_to(outside, target_is_directory=True)
        self.assert_preserved("package escape; object Renamed")
        self.assertEqual((outside / "sentinel").read_bytes(), b"preserve")
        self.assertFalse((outside / "Renamed.class").exists())

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_owned_output_symlink_cannot_delete_target(self):
        self.compile("object Root { object Child }")
        owned = self.destination / "Root$Child.class"
        outside = self.root / "victim.class"
        outside.write_bytes(owned.read_bytes())
        owned.unlink()
        owned.symlink_to(outside)
        self.assert_preserved("object Renamed")
        self.assertTrue(owned.is_symlink())
        self.assertTrue(outside.exists())

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_metadata_and_destination_symlinks_reject(self):
        outside = self.root / "outside"
        outside.mkdir()
        self.destination.symlink_to(outside, target_is_directory=True)
        self.compile("object Root", success=False)
        self.compile("object Root", destination=str(self.destination) + "/", success=False)
        self.assertEqual(list(outside.iterdir()), [])
        self.destination.unlink()
        self.destination.mkdir()
        (self.destination / ".korrust").symlink_to(outside, target_is_directory=True)
        self.compile("object Root", success=False)
        self.assertEqual(list(outside.iterdir()), [])

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_canonical_source_alias_updates_existing_owner(self):
        self.compile("object Root { object Child }")
        alias = self.root / "Alias.kt"
        alias.symlink_to(self.source)
        self.compile("object Renamed", source=alias)
        self.assertEqual(set(self.classes()), {"Renamed.class"})

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_symlink_ancestor_above_destination_supports_creation_and_cleanup(self):
        physical = self.root / "physical"
        physical.mkdir()
        alias = self.root / "alias"
        alias.symlink_to(physical, target_is_directory=True)
        destination = alias / "missing-parent" / "output"
        real_destination = physical / "missing-parent" / "output"
        self.compile("object Root { object Child }", destination=destination)
        self.assertEqual(set(self.classes(real_destination)), {"Root.class", "Root$Child.class"})
        self.compile("object Renamed", destination=destination)
        self.assertEqual(set(self.classes(real_destination)), {"Renamed.class"})
        self.compile("object Final", destination=real_destination)
        self.assertEqual(set(self.classes(destination)), {"Final.class"})

    def test_parent_traversal_after_missing_directory_rejects(self):
        destination = self.root / "missing" / ".." / "escaped"
        self.compile("object Root", destination=destination, success=False)
        self.assertFalse((self.root / "missing").exists())
        self.assertFalse((self.root / "escaped").exists())

    def test_source_path_encoding_handles_whitespace(self):
        source = self.root / "space tab\tline\nfolder" / "Example.kt"
        self.compile("object Root { object Child }", source=source)
        self.compile("object Renamed", source=source)
        self.assertEqual(set(self.classes()), {"Renamed.class"})


class BatchOutputLifecycleTests(unittest.TestCase):
    setUp = OutputLifecycleTests.setUp
    compile = OutputLifecycleTests.compile
    classes = OutputLifecycleTests.classes
    snapshot = OutputLifecycleTests.snapshot
    def batch(self, sources, *, module="sample", destination=None, success=True):
        paths = []
        for name, text in sources:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            if text is not None:
                path.write_text(text)
            paths.append(str(path))
        args = [str(COMPILER), *paths, "-d", str(destination or self.destination)]
        if module is not None:
            args.extend(["--module-name", module])
        result = subprocess.run(args, capture_output=True, text=True, timeout=30)
        if success:
            self.assertEqual(result.returncode, 0, result.stderr)
        else:
            self.assertNotEqual(result.returncode, 0, result.stdout)
        return result

    def test_batch_reordering_is_byte_deterministic(self):
        sources = [("First.kt", "package sample; fun first(): Int = 1"),
                   ("Second.kt", "package sample; object Second { object Nested }")]
        first = self.batch(sources)
        before = self.snapshot()
        second = self.batch(list(reversed(sources)))
        self.assertEqual(first.stdout, second.stdout)
        self.assertEqual(before, self.snapshot())
        clean = self.root / "clean"
        self.batch(list(reversed(sources)), destination=clean)
        self.assertEqual(before, self.snapshot(clean))

    def test_removed_sources_facades_nested_outputs_and_empty_module_match_clean(self):
        self.batch([("First.kt", "package sample; fun first(): Int = 1"),
                    ("Second.kt", "object Second { object Nested }")])
        (self.root / "First.kt").unlink()
        self.batch([("Second.kt", "package moved; object Renamed")])
        clean = self.root / "clean"
        self.batch([("Second.kt", None)], destination=clean)
        self.assertEqual(self.snapshot(), self.snapshot(clean))
        self.assertEqual(set(self.classes()), {"moved/Renamed.class"})
        self.assertFalse(list(self.destination.rglob("*.kotlin_module")))
        (self.root / "Second.kt").unlink()
        self.batch([])
        empty = self.root / "empty"
        self.batch([], destination=empty)
        self.assertEqual(self.snapshot(), self.snapshot(empty))
        self.assertEqual(self.classes(), {})

    def test_batch_duplicate_jvm_owner_preserves_existing_outputs(self):
        self.batch([("First.kt", "object First"), ("Second.kt", "object Second")])
        before = self.snapshot()
        self.batch([("First.kt", "object Duplicate"),
                    ("Second.kt", "object Duplicate")], success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_compile_failure_uses_original_source_and_preserves_outputs(self):
        self.batch([("First.kt", "object First"), ("Second.kt", "object Second")])
        before = self.snapshot()
        sources = [("First.kt", "object First"),
                   ("Second.kt", "object Second { val bad: Int = true }")]
        result = self.batch(sources, success=False)
        self.assertIn(str(self.root / "Second.kt") + ":", result.stderr)
        self.assertEqual(before, self.snapshot())
        reversed_result = self.batch(list(reversed(sources)), success=False)
        self.assertEqual(result.stderr, reversed_result.stderr)
        self.assertEqual(before, self.snapshot())

    def test_batch_preflight_unowned_output_preserves_all_bytes(self):
        self.batch([("First.kt", "object First { object Nested }")])
        (self.destination / "Second.class").write_bytes(b"unowned")
        before = self.snapshot()
        result = self.batch([("First.kt", "object Changed"),
                             ("Second.kt", "object Second")], success=False)
        self.assertIn("no owner", result.stderr)
        self.assertEqual(before, self.snapshot())

    def test_batch_changed_removed_source_output_blocks_cleanup(self):
        self.batch([("First.kt", "object First"), ("Second.kt", "object Second")])
        (self.destination / "Second.class").write_bytes(b"modified")
        (self.root / "Second.kt").unlink()
        before = self.snapshot()
        self.batch([("First.kt", "object Changed")], success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_changed_mapping_blocks_empty_publication(self):
        self.batch([("First.kt", "fun first(): Int = 1")])
        mapping, = self.destination.rglob("*.kotlin_module")
        mapping.write_bytes(b"modified")
        before = self.snapshot()
        self.batch([], success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_missing_output_blocks_cleanup(self):
        self.batch([("First.kt", "object First { object Nested }")])
        (self.destination / "First$Nested.class").unlink()
        before = self.snapshot()
        self.batch([], success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_unrelated_module_and_legacy_outputs_survive_cleanup(self):
        self.compile("object Legacy")
        self.batch([("First.kt", "object First")], module="first-module")
        self.batch([("Second.kt", "object Second")], module="second-module")
        self.batch([], module="first-module")
        self.assertEqual(set(self.classes()), {"Legacy.class", "Second.class"})

    def test_batch_legacy_and_other_module_ownership_collisions_preserve_bytes(self):
        self.compile("object Legacy")
        self.batch([("First.kt", "object First")])
        for sources, module in [([("Other.kt", "object First")], "other"),
                                ([("Other.kt", "object Legacy")], "sample")]:
            before = self.snapshot()
            self.batch(sources, module=module, success=False)
            self.assertEqual(before, self.snapshot())
        before = self.snapshot()
        self.compile("object First", source=self.root / "LegacyCaller.kt", success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_corrupt_module_manifest_preserves_all_bytes(self):
        self.batch([("First.kt", "object First")])
        manifest, = (self.destination / ".korrust").glob("module-*.index")
        manifest.write_text("corrupt\n")
        before = self.snapshot()
        self.batch([("First.kt", "object Changed")], success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_staging_failure_preserves_prior_outputs_and_metadata(self):
        self.batch([("First.kt", "object First { object Nested }")])
        (self.destination / ".korrust/index.next").write_bytes(b"left by external writer")
        before = self.snapshot()
        self.batch([("First.kt", "object Changed")], success=False)
        self.assertEqual(before, self.snapshot())

    def test_batch_source_ownership_can_transfer_within_same_complete_module(self):
        self.batch([("First.kt", "object Shared"), ("Second.kt", "object Other")])
        self.batch([("First.kt", "object New"), ("Second.kt", "object Shared")])
        clean = self.root / "clean"
        self.batch([("First.kt", None), ("Second.kt", None)], destination=clean)
        self.assertEqual(self.snapshot(), self.snapshot(clean))
        self.assertEqual(set(self.classes()), {"New.class", "Shared.class"})

    def test_batch_source_paths_support_non_ascii_and_whitespace(self):
        sources = [("日本語/First.kt", "object First"),
                   ("é space/Second.kt", "object Second { object Nested }")]
        self.batch(sources)
        self.batch([("日本語/First.kt", "object Changed")])
        self.assertEqual(set(self.classes()), {"Changed.class"})

    def test_batch_default_module_and_explicit_main_are_same_owner(self):
        self.batch([("First.kt", "object First"), ("Second.kt", "object Second")], module=None)
        self.batch([("First.kt", "object Changed")], module="main")
        self.assertEqual(set(self.classes()), {"Changed.class"})

    def test_batch_module_names_and_missing_sources_reject_before_publication(self):
        for module in ["", "../escape", "name/path", "name\\path", "a" * 121]:
            result = self.batch([("First.kt", "object First")], module=module, success=False)
            self.assertIn("module name", result.stderr)
            self.assertFalse(self.destination.exists())
        result = self.batch([], module=None, success=False)
        self.assertIn("Usage:", result.stderr)
        self.assertFalse(self.destination.exists())

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_batch_duplicate_source_alias_rejects_before_publication(self):
        self.source.write_text("object Root")
        alias = self.root / "Alias.kt"
        alias.symlink_to(self.source)
        result = self.batch([("Example.kt", None), ("Alias.kt", None)], success=False)
        self.assertIn("Duplicate source identity", result.stderr)
        self.assertFalse(self.destination.exists())

    @unittest.skipUnless(hasattr(os, "symlink"), "symlinks unavailable")
    def test_batch_symlink_preflight_preserves_existing_outputs(self):
        self.batch([("First.kt", "object First")])
        outside = self.root / "outside"
        outside.mkdir()
        (self.destination / "escape").symlink_to(outside, target_is_directory=True)
        before = self.snapshot()
        self.batch([("First.kt", "package escape; object Changed")], success=False)
        self.assertEqual(before, self.snapshot())
        self.assertEqual(list(outside.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
