#!/usr/bin/env python3
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEARCH = Path("benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/util/unsignedBinarySearch.kt")

INT_LIST = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/IntArrayListUtil.kt")
JAVA_LIST = Path("benchmarks/sources/androidbible/AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java")

VERSE_PLACEMENT = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/widget/VerseNumberPlacement.kt")

AUDIO = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/AudioErrorCode.kt")

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def executable(name):
    found = shutil.which(name)
    if found:
        return found
    candidate = Path.home() / ".cargo/bin" / name
    if candidate.is_file():
        return str(candidate)
    raise RuntimeError("Required executable is missing: " + name)


def main():
    now = datetime.now(timezone.utc)
    run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-characters"
    work = ROOT / "target/checks" / run_id
    artifacts = ROOT / "reports/correctness" / run_id
    work.mkdir(parents=True)
    artifacts.mkdir(parents=True)
    report_path = ROOT / "reports/correctness" / (run_id + ".json")
    cases = []
    commands = []
    report = {
        "schema_version": 1,
        "kind": "correctness",
        "created_at": now.isoformat(),
        "status": "running",
        "compiler": None,
        "reference_compiler": None,
        "inputs": {},
        "environment": {"os": platform.platform(), "machine": platform.machine()},
        "commands": commands,
        "evidence": [],
        "selection": "AndroidBible unsignedIntBinarySearch, AudioErrorCode, unchanged IntArrayListUtil using the original Java IntArrayList and unchanged VerseNumberPlacement; JVM arithmetic/control-flow, UTF-16 String/Char/CharArray, subject when, Int += and scoped Java interop probes, invalid and unsupported syntax. Android classes are test-only type stubs; no parcel runtime, upstream Kotlin fixture adapter or whole-app tests.",
        "cases": cases,
    }

    def run(label, args, allowed=(0,), timeout=120):
        index = len(commands)
        print(label, flush=True)
        entry = {"label": label, "argv": [str(a) for a in args], "cwd": ".", "timeout_seconds": timeout}
        commands.append(entry)
        try:
            result = subprocess.run(entry["argv"], cwd=ROOT, text=True, capture_output=True, timeout=timeout)
            stdout, stderr = result.stdout, result.stderr
            entry["exit_code"] = result.returncode
        except subprocess.TimeoutExpired as error:
            stdout = error.stdout or b""
            stderr = error.stderr or b""
            if isinstance(stdout, bytes):
                stdout = stdout.decode(errors="replace")
            if isinstance(stderr, bytes):
                stderr = stderr.decode(errors="replace")
            entry["timed_out"] = True
            result = None
        for stream, value in [("stdout", stdout), ("stderr", stderr)]:
            path = artifacts / f"{index:03d}-{stream}.txt"
            path.write_text(value)
            entry[stream] = str(path.relative_to(ROOT))
            report["evidence"].append(entry[stream])
        if result is None:
            raise TimeoutError(label + " timed out")
        if result.returncode not in allowed:
            raise RuntimeError(f"{label} exited {result.returncode}: {stderr[-2000:]}")
        return result

    try:
        cargo = executable("cargo")
        rustc = executable("rustc")
        java, javac, javap = [executable(n) for n in ["java", "javac", "javap"]]
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
        if not kotlinc.is_file():
            raise RuntimeError("Run python3 tools/setup_reference.py first")
        report["environment"]["rustc"] = run("Rust version", [rustc, "--version"]).stdout.strip()
        report["environment"]["java"] = run("Java version", [java, "-version"]).stderr.strip()
        version = run("Reference compiler version", [kotlinc, "-version"])
        version_text = version.stdout + version.stderr
        if f"kotlinc-jvm {pin['version']} " not in version_text:
            raise RuntimeError("Reference compiler version does not match the pin")
        report["reference_compiler"] = {
            "version": pin["version"],
            "archive_sha256": pin["archive_sha256"],
            "compiler_jar_sha256": sha(ROOT / pin["path"] / "lib/kotlin-compiler.jar"),
            "stdlib_sha256": sha(stdlib),
            "version_output": version_text.strip(),
            "options": ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "1.8"],
        }
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())
        relative_search = str(SEARCH.relative_to("benchmarks/sources/androidbible"))
        if sha(ROOT / SEARCH) != inventory["androidbible"][relative_search]:
            raise RuntimeError("AndroidBible baseline source was changed")
        relative_audio = str(AUDIO.relative_to("benchmarks/sources/androidbible"))
        if sha(ROOT / AUDIO) != inventory["androidbible"][relative_audio]:
            raise RuntimeError("AndroidBible AudioErrorCode source was changed")
        for baseline in [INT_LIST, JAVA_LIST, VERSE_PLACEMENT]:
            relative = str(baseline.relative_to("benchmarks/sources/androidbible"))
            if sha(ROOT / baseline) != inventory["androidbible"][relative]:
                raise RuntimeError("AndroidBible source was changed: " + str(baseline))
        report["inputs"] = {
            "verse_placement_source": str(VERSE_PLACEMENT),
            "verse_placement_source_sha256": sha(ROOT / VERSE_PLACEMENT),
            "int_list_extension_source": str(INT_LIST),
            "int_list_extension_source_sha256": sha(ROOT / INT_LIST),
            "int_list_java_source": str(JAVA_LIST),
            "int_list_java_source_sha256": sha(ROOT / JAVA_LIST),
            "audio_source": str(AUDIO),
            "audio_source_sha256": sha(ROOT / AUDIO),
            "source": str(SEARCH),
            "source_sha256": sha(ROOT / SEARCH),
            "reference_commit": json.loads((ROOT / "references/sources.json").read_text())["kotlin"]["commit"],
            "androidbible_commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["androidbible"]["commit"],
        }
        run("Rust tests", [cargo, "test", "--locked", "--offline"])
        cases.append({"id": "rust_frontend", "outcome": "pass", "scope": "Frontend diagnostics and boundary tests; see command log"})
        run("Release build", [cargo, "build", "--release", "--locked", "--offline"])
        compiler = ROOT / "target/release/korrust"
        owned = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            owned.extend(p for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts)
        fingerprints = {str(p.relative_to(ROOT)): sha(p) for p in sorted(owned)}
        report["compiler"] = {
            "version": run("Korrust version", [compiler, "--version"]).stdout.strip(),
            "binary_sha256": sha(compiler),
            "source_files_sha256": fingerprints,
            "classfile_major_version": 49,
            "kotlin_metadata": False,
            "incremental_compilation": False,
        }
        official, candidate, drivers, foreign = [work / n for n in ["official", "candidate", "drivers", "foreign"]]
        cp = lambda *paths: os.pathsep.join(str(p) for p in paths)
        java_fixtures = sorted((ROOT / "tests/fixtures/interop").rglob("*.java"))
        run("Compile original Java IntArrayList with test-only Android type stubs", [javac, "-d", foreign, ROOT / JAVA_LIST, *java_fixtures])
        options = report["reference_compiler"]["options"]
        probe = ROOT / "tests/fixtures/Arithmetic.kt"
        strings = ROOT / "tests/fixtures/Strings.kt"
        interop = ROOT / "tests/fixtures/Interop.kt"
        characters = ROOT / "tests/fixtures/Characters.kt"
        run("Compile Kotlin metadata rejection fixture", [kotlinc, ROOT / "tests/fixtures/MetadataForeign.kt", *options, "-d", foreign])
        run("Compile fixtures with official Kotlin", [kotlinc, ROOT / SEARCH, ROOT / AUDIO, ROOT / INT_LIST, ROOT / VERSE_PLACEMENT, strings, probe, interop, characters, "-cp", foreign, *options, "-d", official])
        for source in [ROOT / SEARCH, ROOT / AUDIO, ROOT / INT_LIST, ROOT / VERSE_PLACEMENT, probe, strings, interop, characters]:
            run("Compile " + source.name + " with Korrust", [compiler, source, "-cp", foreign, "-d", candidate])
        run("Compile Java callers against official Kotlin", [
            javac, "-cp", cp(official, foreign, stdlib), "-d", drivers,
            ROOT / "tests/fixtures/BinarySearchCheck.java", ROOT / "tests/fixtures/ArithmeticCheck.java", ROOT / "tests/fixtures/StringsCheck.java", ROOT / "tests/fixtures/InteropCheck.java", ROOT / "tests/fixtures/CharactersCheck.java",
        ])
        for name, case_id in [("BinarySearchCheck", "androidbible_unsigned_search"), ("ArithmeticCheck", "jvm_semantics"), ("StringsCheck", "string_semantics"), ("InteropCheck", "java_interop_semantics"), ("CharactersCheck", "character_when_semantics")]:
            reference = run("Run official " + name, [java, "-Xverify:all", "-cp", cp(drivers, official, foreign, stdlib), name], timeout=30)
            actual = run("Run Korrust " + name, [java, "-Xverify:all", "-cp", cp(drivers, candidate, foreign, stdlib), name], timeout=30)
            if reference.stdout != actual.stdout:
                raise AssertionError(name + ": reference and candidate differ")
            result = json.loads(actual.stdout)
            cases.append({"id": case_id, "outcome": "pass", "runtime_checks": result["checks"], "result_sha256": result["sha256"],
                          "oracle": "Official Kotlin plus independent Java assertions", "jvm_verification": "all"})
        for name in ["yuku.alkitab.yes2.util.UnsignedBinarySearchKt", "korrust.probes.ArithmeticKt", "korrust.probes.StringsKt", "yuku.alkitab.base.audio.AudioErrorCodeKt", "yuku.alkitab.base.util.IntArrayListUtilKt", "korrust.probes.InteropKt", "korrust.probes.CharactersKt", "yuku.alkitab.base.widget.VerseNumberPlacementKt"]:
            reference = run("Inspect official JVM signatures", [javap, "-p", "-s", "-classpath", official, name])
            actual = run("Inspect Korrust JVM signatures", [javap, "-p", "-s", "-classpath", candidate, name])
            if reference.stdout != actual.stdout:
                raise AssertionError("JVM signatures differ: " + name)
            cases.append({"id": "jvm_signatures/" + name, "outcome": "pass", "scope": "javap -p -s; does not assert Kotlin metadata compatibility"})
        for fixture in json.loads((ROOT / "tests/diagnostics.json").read_text()):
            source = work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"])
            reference = run("Reference rejection: " + fixture["id"], [kotlinc, source, *options, "-d", work / "negative-reference"], allowed=(1,))
            output = work / ("negative-" + fixture["id"])
            actual = run("Korrust rejection: " + fixture["id"], [compiler, source, "-d", output], allowed=(1,))
            if "error:" not in reference.stderr or fixture["message"] not in actual.stderr:
                raise AssertionError("Unexpected diagnostic: " + fixture["id"])
            if output.exists():
                raise AssertionError("Rejected input produced output")
            cases.append({"id": "diagnostics/" + fixture["id"], "outcome": "pass",
                          "scope": "Both compilers reject; Korrust diagnostic category checked. Exact official error wording is not compared."})
        for fixture in json.loads((ROOT / "tests/unsupported.json").read_text()):
            source = work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"])
            run("Reference acceptance: " + fixture["id"], [kotlinc, source, *options, "-d", work / "unsupported-reference"])
            actual = run("Explicit unsupported rejection: " + fixture["id"], [compiler, source, "-d", work / ("unsupported-" + fixture["id"])], allowed=(1,))
            cases.append({"id": "unsupported/" + fixture["id"], "outcome": "unsupported_language",
                          "diagnostic": actual.stderr.strip()})
        classpath_probe = work / "ClasspathProbe.kt"
        classpath_probe.write_text("import yuku.alkitab.util.IntArrayList\nfun f(value: IntArrayList): Int = value.size()")
        empty_classpath = work / "empty-classpath"
        empty_classpath.mkdir()
        run("Classpath alias and ordered directory search", [compiler, classpath_probe, "--classpath", cp(empty_classpath, foreign), "-d", work / "classpath-alias"])
        for entry in [stdlib, work / "absent-classpath"]:
            output = work / ("invalid-classpath-" + entry.name)
            rejected = run("Reject unsupported or missing classpath entry", [compiler, classpath_probe, "-cp", entry, "-d", output], allowed=(1,))
            if "existing directory" not in rejected.stderr or output.exists():
                raise AssertionError("Classpath configuration rejection was not explicit")
        cases.append({"id": "interop_diagnostics/classpath_configuration", "outcome": "pass", "scope": "--classpath alias, multiple ordered directories, nonexistent directory and unsupported JAR entry rejection."})
        for fixture in json.loads((ROOT / "tests/interop_diagnostics.json").read_text()):
            source = work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"])
            reference = run("Reference Java interop rejection: " + fixture["id"], [kotlinc, source, "-cp", foreign, *options, "-d", work / "interop-invalid-reference"], allowed=(1,))
            output = work / ("interop-invalid-" + fixture["id"])
            actual = run("Korrust Java interop rejection: " + fixture["id"], [compiler, source, "-cp", foreign, "-d", output], allowed=(1,))
            if "error:" not in reference.stderr or fixture["message"].lower() not in actual.stderr.lower():
                raise AssertionError("Unexpected Java interop diagnostic: " + fixture["id"] + ": " + actual.stderr)
            if output.exists():
                raise AssertionError("Rejected input produced output")
            cases.append({"id": "interop_diagnostics/" + fixture["id"], "outcome": "pass", "scope": "Both compilers reject with directory classpath; candidate diagnostic category checked."})
        missing_cp = work / "MissingClasspath.kt"
        missing_cp.write_text("import yuku.alkitab.util.IntArrayList\nfun f(value: IntArrayList): Int = value.size()")
        run("Reference rejects missing classpath", [kotlinc, missing_cp, *options, "-d", work / "missing-classpath-reference"], allowed=(1,))
        missing = run("Korrust rejects missing classpath", [compiler, missing_cp, "-d", work / "missing-classpath-candidate"], allowed=(1,))
        if "not found" not in missing.stderr:
            raise AssertionError("Missing classpath diagnostic was not explicit")
        cases.append({"id": "interop_diagnostics/missing_classpath", "outcome": "pass", "scope": "Both compilers reject unavailable imported Java class."})
        for fixture in json.loads((ROOT / "tests/interop_unsupported.json").read_text()):
            source = work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"])
            run("Reference Java interop acceptance: " + fixture["id"], [kotlinc, source, "-cp", foreign, *options, "-d", work / "interop-unsupported-reference"])
            output = work / ("interop-unsupported-" + fixture["id"])
            actual = run("Explicit Java interop unsupported rejection: " + fixture["id"], [compiler, source, "-cp", foreign, "-d", output], allowed=(1,))
            if output.exists():
                raise AssertionError("Unsupported input produced output")
            cases.append({"id": "unsupported/" + fixture["id"], "outcome": "unsupported_language", "diagnostic": actual.stderr.strip(), "scope": "Official compiler accepts with the same directory classpath."})
        report["status"] = "completed"
    except Exception as error:
        report["status"] = "failed"
        cases.append({"id": "harness_failure", "outcome": "timeout" if isinstance(error, TimeoutError) else "fail", "message": str(error)})
        print(str(error), file=sys.stderr)
    finally:
        report["counts"] = dict(Counter(case["outcome"] for case in cases))
        report["finished_at"] = datetime.now(timezone.utc).isoformat()
        report_path.write_text(json.dumps(report, indent=2) + "\n")
        print("Report: " + str(report_path.relative_to(ROOT)), flush=True)
    if report["status"] == "completed":
        passed = {case["id"] for case in cases if case["outcome"] == "pass"}
        features = []
        for feature in json.loads((ROOT / "tests/features.json").read_text()):
            if not set(feature["evidence_cases"]).issubset(passed):
                raise RuntimeError("Missing feature evidence: " + feature["name"])
            features.append({**feature, "status": "partial"})
        for case in cases:
            if case["outcome"] == "unsupported_language":
                features.append({"name": case["id"].split("/", 1)[1], "status": "unsupported", "evidence_cases": [case["id"]]})
        common = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        common["commands"] = [["python3", "tools/check.py"]]
        common["evidence"] = [str(report_path.relative_to(ROOT))]
        feature_report = {**common, "kind": "features", "status": "assessed", "features": features}
        feature_path = ROOT / "reports/features" / (run_id + ".json")
        feature_path.write_text(json.dumps(feature_report, indent=2) + "\n")
        benchmark_report = {
            **common, "kind": "benchmarks", "status": "not_run",
            "reason": "Functional differential testing only. No controlled compilation benchmarks or incremental compiler exist.",
            "samples": [], "speedup": None,
        }
        benchmark_path = ROOT / "reports/benchmarks" / (run_id + ".json")
        benchmark_path.write_text(json.dumps(benchmark_report, indent=2) + "\n")
        print("Feature report: " + str(feature_path.relative_to(ROOT)), flush=True)
    return 0 if report["status"] == "completed" else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
