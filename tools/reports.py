#!/usr/bin/env python3
import argparse
import hashlib
import html
import json
import os
import plistlib
import subprocess
import sys
import tempfile
import time
from collections import Counter
from pathlib import Path
from urllib.parse import quote

ROOT = Path(__file__).resolve().parents[1]
CSS = """
body { font: 16px/1.5 system-ui, sans-serif; color: #222; background: #fff;
       max-width: 1250px; margin: 2rem auto; padding: 0 1rem; }
a { color: #14559a; } h1, h2 { line-height: 1.25; }
table { border-collapse: collapse; width: 100%; margin: .75rem 0 1.5rem; }
th, td { border: 1px solid #bbb; padding: .45rem .65rem; text-align: left;
         vertical-align: top; overflow-wrap: anywhere; }
th { background: #eee; } .scroll { overflow-x: auto; }
summary { cursor: pointer; font-weight: 600; padding: .5rem 0; }
pre { white-space: pre-wrap; overflow-wrap: anywhere; font: .9rem/1.5 monospace; }
ul { padding-left: 1.2rem; } td ul { margin: 0; }
.muted { color: #555; } .error { border-left: 4px solid #a11; padding-left: 1rem; }
@media print { body { max-width: none; } details { display: block; } }
"""


def escape(value):
    return html.escape(str(value), quote=True)


def page_path(output, relative):
    return output / "pages" / (str(relative) + ".html")


def link(page, destination, label, fragment=""):
    url = quote(os.path.relpath(destination, page.parent), safe="/") + fragment
    return f'<a href="{escape(url)}">{escape(label)}</a>'


def case_anchor(case_id):
    return "case-" + hashlib.sha256(str(case_id).encode()).hexdigest()[:16]


class Renderer:
    def __init__(self, root, output, page, report):
        self.root, self.output, self.page = root, output, page
        self.report = report

    def value(self, value):
        if value is None:
            return '<span class="muted">Not recorded</span>'
        if isinstance(value, bool):
            return "Yes" if value else "No"
        if isinstance(value, dict):
            return self.fields(value)
        if isinstance(value, list):
            if not value:
                return '<span class="muted">None recorded</span>'
            return "<ul>" + "".join("<li>" + self.value(v) + "</li>" for v in value) + "</ul>"
        if isinstance(value, str) and len(value) < 1024 and "\n" not in value:
            try:
                source = (self.root / value).resolve()
                if source.is_relative_to(self.root) and source.is_file():
                    if source.is_relative_to(self.root / "reports") and source.suffix in {".json", ".md"}:
                        destination = page_path(self.output, source.relative_to(self.root / "reports"))
                    else:
                        destination = source
                    return link(self.page, destination, value)
            except (OSError, ValueError):
                pass
        return escape(value)

    def fields(self, fields):
        rows = "".join(f"<tr><th scope=\"row\">{escape(k)}</th><td>{self.value(v)}</td></tr>" for k, v in fields.items())
        return '<div class="scroll"><table><tbody>' + rows + "</tbody></table></div>"

    def evidence_cases(self, cases):
        evidence = self.report.get("evidence", [])
        for item in evidence if isinstance(evidence, list) else []:
            if not isinstance(item, str):
                continue
            source = (self.root / item).resolve()
            if not source.is_relative_to(self.root / "reports") or source.suffix != ".json":
                continue
            try:
                data = json.loads(source.read_text())
                ids = {case["id"] for case in data.get("cases", [])}
            except (OSError, ValueError, TypeError, KeyError, AttributeError):
                continue
            if isinstance(cases, list) and all(case in ids for case in cases):
                destination = page_path(self.output, source.relative_to(self.root / "reports"))
                return "<ul>" + "".join("<li>" + link(self.page, destination, c, "#" + case_anchor(c)) + "</li>" for c in cases) + "</ul>"
        return self.value(cases)

    def table(self, rows):
        if not rows:
            return "<p>None recorded.</p>"
        if not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows):
            return self.value(rows)
        columns = list(dict.fromkeys(key for row in rows for key in row))
        header = "".join(f'<th scope="col">{escape(c)}</th>' for c in columns)
        body = []
        for row in rows:
            cells = []
            for column in columns:
                value = row.get(column)
                rendered = self.evidence_cases(value) if column == "evidence_cases" else self.value(value)
                cells.append("<td>" + rendered + "</td>")
            anchor = f' id="{case_anchor(row["id"])}"' if "id" in row else ""
            body.append(f"<tr{anchor}>" + "".join(cells) + "</tr>")
        return '<div class="scroll"><table><thead><tr>' + header + "</tr></thead><tbody>" + "".join(body) + "</tbody></table></div>"

    def features(self, rows):
        if not rows:
            return "<p>No feature support claims recorded.</p>"
        if not isinstance(rows, list) or any(not isinstance(row, dict) for row in rows):
            return self.value(rows)
        statuses = list(dict.fromkeys(str(row.get("status", "unassessed")) for row in rows))
        result = "<p>Counts describe recorded feature entries, not coverage of the whole Kotlin language.</p>"
        for status in statuses:
            group = [row for row in rows if str(row.get("status", "unassessed")) == status]
            result += details(f"{status} ({len(group)})", self.table(group), opened=True)
        return result


def details(title, content, opened=False):
    return f'<details{" open" if opened else ""}><summary>{escape(title)}</summary>{content}</details>'


def document(title, body):
    return f'<!doctype html>\n<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{escape(title)}</title><style>{CSS}</style></head><body>{body}</body></html>\n'


def atomic_write(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.read_text() == content:
        return
    with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, delete=False) as temporary:
        temporary.write(content)
    os.replace(temporary.name, path)


def generate(root=ROOT, output=None):
    root = root.resolve()
    output = (output or root / "target/reports-html").resolve()
    reports = root / "reports"
    if output.is_relative_to(reports):
        raise ValueError("HTML output must be outside reports/")
    records = []
    written = set()
    for source in sorted(reports.rglob("*")):
        if not source.is_file() or source.suffix not in {".json", ".md"}:
            continue
        relative = source.relative_to(reports)
        page = page_path(output, relative)
        nav = link(page, output / "index.html", "All reports") + " · " + link(page, source, "Source")
        title = str(relative)
        body = f"<nav>{nav}</nav><h1>{escape(title)}</h1>"
        record = {"relative": relative, "page": page, "kind": "notes", "status": "note", "created_at": ""}
        if source.suffix == ".md":
            body += "<pre>" + escape(source.read_text()) + "</pre>"
        else:
            try:
                data = json.loads(source.read_text())
                if not isinstance(data, dict):
                    raise ValueError("Report must be a JSON object")
                record.update({key: str(data.get(key, "Not recorded")) for key in ["kind", "status", "created_at"]})
                record["data"] = data
                render = Renderer(root, output, page, data)
                summary = {k: data[k] for k in ["kind", "created_at", "finished_at", "status", "reason", "selection"] if k in data}
                body += render.fields(summary)
                for key, heading in [("features", "Feature coverage"), ("cases", "Correctness cases"), ("samples", "Benchmark samples"), ("workloads", "Workloads")]:
                    if key in data:
                        content = render.features(data[key]) if key == "features" else render.table(data[key])
                        body += f"<h2>{heading}</h2>" + content
                if data.get("kind") == "benchmarks" and not data.get("samples"):
                    body += "<p>No timing samples recorded. No measured speedup can be inferred.</p>"
                displayed = set(summary) | {"features", "cases", "samples", "workloads"}
                for key, value in data.items():
                    if key not in displayed:
                        content = render.table(value) if key == "commands" else render.value(value)
                        body += details(key.replace("_", " ").capitalize(), content)
            except (ValueError, OSError) as error:
                record["status"] = "unreadable"
                body += '<p class="error">Cannot read this report: ' + escape(error) + "</p>"
        atomic_write(page, document(title, body))
        written.add(str(page.relative_to(output)))
        records.append(record)
    index = output / "index.html"
    body = "<h1>Korrust reports</h1><p>Generated from dated source reports. Refresh this page after a report changes.</p>"
    unreadable = [r for r in records if r["status"] == "unreadable"]
    if unreadable:
        body += '<p class="error">Unreadable reports: ' + ", ".join(link(index, r["page"], str(r["relative"])) for r in unreadable) + ". Latest summaries below use readable reports.</p>"
    body += "<p>Randy Sugianto (Yuku) · <a href=\"https://github.com/yukuku\">@yukuku</a></p>"
    records.sort(key=lambda r: (r["created_at"], str(r["relative"])), reverse=True)
    for kind, heading in [("features", "Feature coverage"), ("correctness", "Correctness"), ("benchmarks", "Benchmarks")]:
        body += f"<h2>{heading}</h2>"
        matching = [r for r in records if r["kind"] == kind]
        if not matching:
            body += "<p>No reports recorded.</p>"
            continue
        latest = matching[0]
        data = latest["data"]
        body += "<p>Latest: " + link(index, latest["page"], str(latest["relative"])) + " — " + escape(latest["status"]) + "</p>"
        render = Renderer(root, output, index, data)
        if kind == "features":
            body += render.features(data.get("features", []))
        elif kind == "correctness":
            counts = Counter(str(case.get("outcome", "unknown")) for case in data.get("cases", []) if isinstance(case, dict))
            body += render.table([{"outcome": outcome, "case groups": count} for outcome, count in sorted(counts.items())])
            body += "<p>These are case groups; runtime input counts are recorded in the individual cases. Unsupported cases are separate from passes.</p>"
        elif not data.get("samples"):
            body += "<p>No timing samples recorded. No measured speedup can be inferred.</p>"
        else:
            body += render.table(data["samples"])
        if data.get("reason"):
            body += "<p>" + escape(data["reason"]) + "</p>"
    body += "<h2>All reports and notes</h2>"
    history = "<tr><th>Report</th><th>Kind</th><th>Status</th><th>Created (UTC)</th></tr>"
    for record in records:
        history += "<tr><td>" + link(index, record["page"], str(record["relative"])) + "</td>" + "".join("<td>" + escape(record[k]) + "</td>" for k in ["kind", "status", "created_at"]) + "</tr>"
    body += '<div class="scroll"><table>' + history + "</table></div>"
    atomic_write(index, document("Korrust reports", body))
    written.add("index.html")
    manifest = output / ".generated.json"
    previous = json.loads(manifest.read_text()) if manifest.exists() else []
    for name in set(previous) - written:
        stale = (output / name).resolve()
        if stale.is_relative_to(output) and stale.suffix == ".html":
            stale.unlink(missing_ok=True)
    atomic_write(manifest, json.dumps(sorted(written)) + "\n")
    return index


def snapshot(root):
    result = []
    for path in sorted((root / "reports").rglob("*")):
        try:
            if path.is_file():
                stat = path.stat()
                result.append((str(path), stat.st_mtime_ns, stat.st_ctime_ns, stat.st_size))
        except FileNotFoundError:
            pass
    return tuple(result)


def watch(root, output, interval):
    previous = None
    while True:
        current = snapshot(root)
        if current != previous:
            time.sleep(interval)
            if snapshot(root) != current:
                continue
            try:
                print("Generated " + str(generate(root, output)), flush=True)
                previous = current
            except (OSError, ValueError) as error:
                print("Report generation failed: " + str(error), file=sys.stderr, flush=True)
        time.sleep(interval)


def configure_watch(root, output, remove):
    if sys.platform != "darwin":
        raise RuntimeError("Automatic service installation requires macOS; use --watch elsewhere")
    suffix = hashlib.sha256(str(root).encode()).hexdigest()[:12]
    label = "dev.korrust.reports." + suffix
    plist = Path.home() / "Library/LaunchAgents" / (label + ".plist")
    domain = "gui/" + str(os.getuid())
    if plist.exists():
        subprocess.run(["launchctl", "bootout", domain, str(plist)], capture_output=True)
    if remove:
        plist.unlink(missing_ok=True)
        print("Removed watcher: " + label)
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    configuration = {
        "Label": label,
        "ProgramArguments": [sys.executable, str(Path(__file__).resolve()), "--root", str(root), "--output", str(output), "--watch"],
        "WorkingDirectory": str(root),
        "RunAtLoad": True,
        "KeepAlive": True,
        "ThrottleInterval": 10,
        "StandardOutPath": str(output.parent / "reports-watch.stdout.log"),
        "StandardErrorPath": str(output.parent / "reports-watch.stderr.log"),
    }
    plist.parent.mkdir(parents=True, exist_ok=True)
    plist.write_bytes(plistlib.dumps(configuration))
    subprocess.run(["launchctl", "bootstrap", domain, str(plist)], check=True)
    print("Installed watcher: " + label)


def main():
    parser = argparse.ArgumentParser(description="Generate plain HTML from Korrust reports")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", type=Path)
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--watch", action="store_true")
    mode.add_argument("--install-watch", action="store_true")
    mode.add_argument("--uninstall-watch", action="store_true")
    parser.add_argument("--interval", type=float, default=1.0)
    args = parser.parse_args()
    if args.interval <= 0:
        parser.error("--interval must be positive")
    root = args.root.resolve()
    output = (args.output or root / "target/reports-html").resolve()
    if output.is_relative_to(root / "reports"):
        parser.error("HTML output must be outside reports/")
    if args.install_watch or args.uninstall_watch:
        configure_watch(root, output, args.uninstall_watch)
    elif args.watch:
        watch(root, output, args.interval)
    else:
        print(generate(root, output))


if __name__ == "__main__":
    main()
