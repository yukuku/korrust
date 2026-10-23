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
        self.assert_preserved("object Root { val unsupported = 1 }")

    def test_invalid_source_does_not_create_destination(self):
        self.compile("object Root { val unsupported = 1 }", success=False)
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


if __name__ == "__main__":
    unittest.main()
