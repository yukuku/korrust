import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest


TOOL = Path(__file__).resolve().parents[1] / "tools/verify_references.py"
SPEC = importlib.util.spec_from_file_location("verify_references", TOOL)
verify = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verify)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class ReferenceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)

    def test_exact_bytes_and_broken_symlink_are_verified(self):
        content = b"source\r\n\x00\xff"
        (self.root / "source.kt").write_bytes(content)
        (self.root / "broken").symlink_to("absent")
        result = verify.verify_snapshot(self.root, {
            "source.kt": sha(content), "broken": {"symlink": "absent"},
        })
        self.assertEqual(result, {"files": 1, "symlinks": 1,
                                  "bytes": len(content), "errors": []})

    def test_changed_and_missing_files_are_aggregated(self):
        (self.root / "changed").write_bytes(b"new")
        result = verify.verify_snapshot(self.root, {"changed": sha(b"old"), "missing": sha(b"")})
        self.assertEqual(len(result["errors"]), 2)
        self.assertIn("SHA-256 mismatch", result["errors"][0])
        self.assertEqual(result["errors"][1], "missing: missing")

    def test_symlink_target_and_file_types_are_checked_without_following(self):
        (self.root / "file").write_bytes(b"contents")
        (self.root / "link").symlink_to("file")
        result = verify.verify_snapshot(self.root, {
            "file": {"symlink": "elsewhere"}, "link": sha(b"contents"),
        })
        self.assertEqual(len(result["errors"]), 2)
        self.assertIn("expected a symlink", result["errors"][0])
        self.assertIn("expected a regular file", result["errors"][1])
        result = verify.verify_snapshot(self.root, {"link": {"symlink": "elsewhere"}}, True)
        self.assertIn("symlink target mismatch", result["errors"][0])

    def test_unsafe_paths_and_symlink_ancestors_are_rejected(self):
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "secret").write_bytes(b"secret")
        snapshot = self.root / "snapshot"
        snapshot.mkdir()
        (snapshot / "escape").symlink_to(outside, target_is_directory=True)
        entries = {path: sha(b"secret") for path in (
            "../outside/secret", str(outside / "secret"), "escape/secret",
            "./file", "double//slash", "windows\\path", ".git/config",
        )}
        result = verify.verify_snapshot(snapshot, entries, True)
        self.assertEqual(len(result["errors"]), len(entries))
        self.assertEqual(result["bytes"], 0)
        self.assertTrue(any("symlink ancestor" in error for error in result["errors"]))

    def test_inventory_includes_unlisted_symlinks_but_skips_git_metadata(self):
        (self.root / "extra").write_bytes(b"extra")
        (self.root / "linked-directory").symlink_to("missing", target_is_directory=True)
        (self.root / "nested/.git").mkdir(parents=True)
        (self.root / "nested/.git/config").write_text("local metadata")
        result = verify.verify_snapshot(self.root, {})
        self.assertEqual(result["errors"], ["extra: extra file or symlink",
                                            "linked-directory: extra file or symlink"])
        self.assertEqual(verify.verify_snapshot(self.root, {}, True)["errors"], [])

    def test_lfs_pointer_has_recovery_instruction_only_when_hash_differs(self):
        pointer = b"version https://git-lfs.github.com/spec/v1\noid sha256:abc\nsize 10\n"
        (self.root / "large.bin").write_bytes(pointer)
        result = verify.verify_snapshot(self.root, {"large.bin": sha(b"actual binary")})
        self.assertIn("Git LFS pointer; run git lfs pull", result["errors"][0])
        self.assertEqual(verify.verify_snapshot(self.root, {"large.bin": sha(pointer)})["errors"], [])

    def test_repository_maps_all_snapshots_and_rejects_symlinked_parent(self):
        for manifest_name, snapshots in verify.MANIFESTS.items():
            manifest_path = self.root / manifest_name
            manifest_path.parent.mkdir(parents=True, exist_ok=True)
            manifest = {}
            for key, relative in snapshots.items():
                source = self.root / relative
                source.mkdir(parents=True)
                (source / "sample").write_bytes(key.encode())
                manifest[key] = {"sample": sha(key.encode())}
            manifest_path.write_text(json.dumps(manifest))
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(verify.main(["--root", str(self.root)]), 0)
        self.assertIn("Source snapshot verification passed", output.getvalue())
        benchmarks = self.root / "benchmarks"
        benchmarks.rename(self.root / "elsewhere")
        benchmarks.symlink_to("elsewhere", target_is_directory=True)
        result = verify.verify_repository(self.root)
        self.assertIn("symlink ancestor", result["benchmarks/source-files.sha256.json"]["errors"][0])

    def test_invalid_manifest_returns_nonzero_without_traceback(self):
        (self.root / "references").mkdir()
        (self.root / "references/source-files.sha256.json").write_text("not JSON")
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(verify.main(["--root", str(self.root)]), 1)


if __name__ == "__main__":
    unittest.main()
