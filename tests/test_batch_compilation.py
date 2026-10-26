import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
from check_batch_compilation import invalid_rejection


class InvalidBoundaryTests(unittest.TestCase):
    def test_invalid_rejection_does_not_accept_unsupported_or_crashed_compilation(self):
        for code, diagnostic, emitted, expected in [
            (1, "Access.kt:2:1: error: Private declaration is inaccessible", {}, True),
            (1, "Access.kt:2:1: error: Unsupported private declaration", {}, False),
            (1, "Access.kt:2:1: error: Private declarations are not yet supported", {}, False),
            (101, "panicked while checking private declaration", {}, False),
            (0, "Access.kt:2:1: error: Private declaration", {}, False),
            (1, "Access.kt:2:1: error: Private declaration", {"AccessKt.class": "hash"}, False),
            (1, "Access.kt:2:1: error: Missing external dependency", {}, False),
        ]:
            with self.subTest(diagnostic=diagnostic, code=code, emitted=emitted):
                self.assertEqual(bool(invalid_rejection(code, diagnostic, emitted, r"private|access")), expected)


if __name__ == "__main__":
    unittest.main()
