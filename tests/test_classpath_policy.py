import hashlib
import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import warnings
import zipfile


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("classpath_policy", ROOT / "tools/classpath_policy.py")
policy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(policy)


class ClasspathPolicyTests(unittest.TestCase):
    def archive(self, path, entries, compression=zipfile.ZIP_STORED):
        with zipfile.ZipFile(path, "w", compression=compression) as archive:
            for name, content in entries:
                archive.writestr(name, content)
        return path

    def test_stored_deflated_and_directory_inputs_preserve_order_and_metadata(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entries = [("dep/Library.class", b"class"), ("META-INF/library.kotlin_module", b"metadata"),
                       ("module-info.class", b"descriptor"), ("META-INF/versions/9/module-info.class", b"descriptor")]
            directory = root / "classes"
            for name, content in entries:
                output = directory / name
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(content)
            stored = self.archive(root / "stored.jar", entries)
            deflated = self.archive(root / "deflated.jar", entries, zipfile.ZIP_DEFLATED)
            before = {path: path.read_bytes() for path in [stored, deflated, directory / "META-INF/library.kotlin_module"]}
            inputs = [stored, directory, deflated, stored]
            records = policy.validate_classpath(inputs)
            self.assertEqual([record["path"] for record in records], list(map(str, inputs)))
            self.assertEqual([record["kind"] for record in records], ["jar", "directory", "jar", "jar"])
            self.assertEqual(records[0]["sha256"], hashlib.sha256(stored.read_bytes()).hexdigest())
            self.assertEqual(before, {path: path.read_bytes() for path in before})

    def test_multi_release_overrides_reject_for_directories_and_archives(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            entries = [("dep/Library.class", b"base"), ("META-INF/versions/9/dep/Library.class", b"override")]
            directory = root / "classes"
            for name, content in entries:
                output = directory / name
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(content)
            archive = self.archive(root / "library.jar", entries)
            for path in [directory, archive]:
                with self.subTest(path=path), self.assertRaisesRegex(ValueError, "multi-release JAR class overrides are unsupported"):
                    policy.validate_classpath([path])
            self.assertEqual((directory / entries[1][0]).read_bytes(), b"override")

    def test_malformed_duplicate_and_missing_inputs_reject(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ["../A.class", "/A.class", "p/./A.class", "p//A.class", "p/../A.class", "p\\A.class", "C:/A.class"]:
                archive = self.archive(root / "malformed.jar", [(name, b"class")])
                with self.subTest(name=name), self.assertRaisesRegex(ValueError, "malformed ZIP entry name"):
                    policy.validate_classpath([archive])
            with warnings.catch_warnings():
                warnings.simplefilter("ignore", UserWarning)
                archive = self.archive(root / "duplicate.jar", [("A.class", b"one"), ("A.class", b"two")])
            with self.assertRaisesRegex(ValueError, "duplicate ZIP entry names"):
                policy.validate_classpath([archive])
            with self.assertRaisesRegex(ValueError, "existing directory or JAR/ZIP"):
                policy.validate_classpath([root / "missing.jar"])
            archive.write_bytes(b"malformed archive")
            with self.assertRaisesRegex(ValueError, "Classpath"):
                policy.validate_classpath([archive])

    def test_oversized_and_unsupported_compression_reject(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            path = self.archive(root / "oversized.jar", [("A.class", b"class")])
            data = bytearray(path.read_bytes())
            central = data.index(b"PK\x01\x02")
            struct.pack_into("<I", data, central + 24, policy.MAX_ENTRY + 1)
            path.write_bytes(data)
            with self.assertRaisesRegex(ValueError, "64 MiB"):
                policy.validate_classpath([path])
            path = self.archive(root / "compression.jar", [("A.class", b"class")], zipfile.ZIP_BZIP2)
            with self.assertRaisesRegex(ValueError, "unsupported ZIP compression"):
                policy.validate_classpath([path])


if __name__ == "__main__":
    unittest.main()
