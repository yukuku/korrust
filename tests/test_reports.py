import json
import subprocess
import sys
import tempfile
import time
import unittest
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import reports


class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.hrefs = []
        self.ids = set()

    def handle_starttag(self, tag, attrs):
        attributes = dict(attrs)
        if tag == "a":
            self.hrefs.append(attributes["href"])
        if "id" in attributes:
            self.ids.add(attributes["id"])


class ReportsTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name).resolve()
        self.output = self.root / "target/reports-html"
        self.write("correctness/check.json", {
            "kind": "correctness", "status": "completed", "created_at": "2026-10-23",
            "cases": [{"id": "binary_search", "outcome": "pass", "runtime_checks": 123},
                      {"id": "unsupported", "outcome": "unsupported_language"}],
            "counts": {"pass": 999},
        })
        self.write("features/coverage.json", {
            "kind": "features", "status": "assessed", "created_at": "2026-10-23",
            "features": [{"name": "ints <script>alert(1)</script>", "status": "partial",
                          "restrictions": ["No Long"], "evidence_cases": ["binary_search"]}],
            "evidence": ["reports/correctness/check.json"],
        })
        self.write("benchmarks/not run.json", {
            "kind": "benchmarks", "status": "not_run", "created_at": "2026-10-23",
            "samples": [], "speedup": None,
        })

    def write(self, relative, data):
        path = self.root / "reports" / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(data))
        return path

    def test_tables_evidence_links_escaping_and_unmeasured_results(self):
        index = reports.generate(self.root)
        content = index.read_text()
        self.assertIn("<details open>", content)
        self.assertIn("partial (1)", content)
        self.assertIn("&lt;script&gt;", content)
        self.assertNotIn("<script>", content)
        self.assertIn("No Long", content)
        self.assertIn("No measured speedup", content)
        self.assertIn("unsupported_language", content)
        self.assertNotIn("999", content)
        for page in self.output.rglob("*.html"):
            parsed = Links()
            parsed.feed(page.read_text())
            for href in parsed.hrefs:
                url = urlsplit(href)
                if url.scheme:
                    self.assertEqual("https", url.scheme)
                    continue
                destination = (page.parent / unquote(url.path)).resolve()
                self.assertTrue(destination.exists(), str(destination))
                if url.fragment:
                    target = Links()
                    target.feed(destination.read_text())
                    self.assertIn(url.fragment, target.ids)

    def test_undated_reports_do_not_replace_the_latest_dated_result(self):
        for name, timestamp in [("missing", {}), ("null", {"created_at": None})]:
            self.write("correctness/" + name + ".json", {
                "kind": "correctness", "status": "unassessed", "cases": [], **timestamp,
            })
        index = reports.generate(self.root).read_text()
        latest = index.split("<h2>Correctness</h2>", 1)[1].split("<h2>", 1)[0]
        self.assertIn("correctness/check.json", latest)
        self.assertNotIn("correctness/missing.json", latest)
        self.assertNotIn("correctness/null.json", latest)
        self.assertIn("correctness/missing.json", index)

    def test_partial_write_and_recovery_are_visible(self):
        broken = self.root / "reports/features/incomplete.json"
        broken.write_text('{"kind":')
        index = reports.generate(self.root)
        self.assertIn("Unreadable reports", index.read_text())
        broken.write_text(json.dumps({"kind": "features", "created_at": "2027", "status": "not_assessed", "features": []}))
        reports.generate(self.root)
        self.assertNotIn("Unreadable reports", index.read_text())
        self.assertIn("No feature support claims", index.read_text())

    def test_non_object_json_is_visible(self):
        self.write("invalid.json", [])
        index = reports.generate(self.root)
        self.assertIn("Unreadable reports", index.read_text())

    def test_untyped_evidence_keeps_its_page_without_entering_report_history(self):
        self.write("correctness/evidence/source-map.json", {"original": "fixture.kt", "offsets": [0, 1]})
        data = json.loads((self.root / "reports/correctness/check.json").read_text())
        data["evidence"] = ["reports/correctness/evidence/source-map.json"]
        self.write("correctness/check.json", data)
        index = reports.generate(self.root)
        self.assertNotIn("source-map.json", index.read_text())
        evidence = self.output / "pages/correctness/evidence/source-map.json.html"
        self.assertTrue(evidence.is_file())
        report = self.output / "pages/correctness/check.json.html"
        self.assertIn("evidence/source-map.json.html", report.read_text())

    def test_idempotence_and_deleted_report_cleanup(self):
        index = reports.generate(self.root)
        modified = index.stat().st_mtime_ns
        reports.generate(self.root)
        self.assertEqual(modified, index.stat().st_mtime_ns)
        (self.root / "reports/features/coverage.json").unlink()
        reports.generate(self.root)
        self.assertFalse((self.output / "pages/features/coverage.json.html").exists())
        self.assertNotIn("coverage.json", index.read_text())

    def test_output_cannot_be_inside_inputs(self):
        with self.assertRaises(ValueError):
            reports.generate(self.root, self.root / "reports/html")

    def test_watch_observes_edits_additions_and_deletions(self):
        process = subprocess.Popen([sys.executable, str(reports.ROOT / "tools/reports.py"), "--root", str(self.root), "--watch", "--interval", "0.05"], stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        try:
            index = self.output / "index.html"
            self.wait_for(lambda: index.exists())
            path = self.write("features/nested/new.json", {
                "kind": "features", "created_at": "2027", "status": "assessed",
                "features": [{"name": "watch_added", "status": "partial"}],
            })
            self.wait_for(lambda: "watch_added" in index.read_text())
            self.write("features/nested/new.json", {
                "kind": "features", "created_at": "2027", "status": "assessed",
                "features": [{"name": "watch_edited", "status": "partial"}],
            })
            self.wait_for(lambda: "watch_edited" in index.read_text())
            path.unlink()
            self.wait_for(lambda: "watch_edited" not in index.read_text())
        finally:
            process.terminate()
            process.communicate(timeout=5)

    def wait_for(self, condition):
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if condition():
                return
            time.sleep(0.05)
        self.fail("Timed out waiting for report watcher")


if __name__ == "__main__":
    unittest.main()
