#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile


ROOT = Path(__file__).resolve().parents[1]
SNAPSHOT = ROOT / "benchmarks/sources/androidbible"
MODULE = "AlkitabYes2"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def relative(path):
    path = Path(path)
    try:
        return str(path.relative_to(ROOT))
    except ValueError:
        return str(path)


def classpath(*paths):
    return os.pathsep.join(str(path) for path in paths)


def trace_source_records(trace):
    if trace.get("schema_version", 1) >= 2:
        return [{"source": item["path"], "source_sha256": item["sha256"]} for item in trace["sources"]]
    return trace["invocations"]


def checked_compile_invocations(trace, request, compiler, expected_sources, work):
    records = trace_source_records(trace)
    paths = [item["source"] for item in records]
    if sorted(paths) != sorted(request["sources"]) or len(paths) != len(set(paths)):
        raise AssertionError("Candidate trace/request source inventory mismatch")
    for item in records:
        name = str(Path(item["source"]).relative_to(work))
        if item["source_sha256"] != expected_sources[name]:
            raise AssertionError("Candidate source hash differs from preserved input")
    invocations = trace["invocations"]
    if trace.get("schema_version", 1) >= 2:
        command = [compiler, *sorted(paths), "-d", request["stage"], "--module-name", request["module_name"]]
        if request["classpath"]:
            command += ["-cp", os.pathsep.join(request["classpath"])]
        if len(invocations) != 1 or invocations[0]["command"] != command or invocations[0]["sources"] != request["sources"]:
            raise AssertionError("Candidate compilation must use one complete module batch")
    else:
        for invocation in invocations:
            command = invocation["command"]
            if command[0] != compiler or command[1] != invocation["source"] or command[command.index("-d") + 1] != request["stage"]:
                raise AssertionError("Candidate per-source command differs from provenance")
    if any(item["returncode"] != 0 for item in invocations):
        raise AssertionError("Candidate compile invocation failed")
    return invocations


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--official-json", type=Path, default=ROOT / "target/androidbible-module-artifacts/official/build.json")
    parser.add_argument("--candidate-json", type=Path, default=ROOT / "target/androidbible-module-artifacts/korrust/build.json")
    parser.add_argument("--sdk", type=Path, default=Path(os.environ.get("ANDROID_SDK_ROOT", "/workspace/korrust-setup/android-sdk")))
    args = parser.parse_args()
    args.official_json = args.official_json.resolve()
    args.candidate_json = args.candidate_json.resolve()
    args.sdk = args.sdk.resolve()
    now = datetime.now(timezone.utc)
    run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-androidbible-yes2-module"
    work = ROOT / "target/checks" / run_id
    evidence = ROOT / "reports/correctness" / run_id
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    report_path = evidence.with_suffix(".json")
    cases, commands = [], []
    report = {
        "schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running",
        "compiler": {}, "reference_compiler": {}, "inputs": {}, "commands": commands, "evidence": [], "cases": cases,
        "environment": {"os": platform.platform(), "machine": platform.machine()},
        "selection": "Complete unchanged AndroidBible AlkitabYes2 production module: both Kotlin sources and all 22 Java sources through real Gradle/AGP dependency, Java, resource and AAR tasks. Candidate production Kotlin uses Korrust without fallback; dependency modules and original Kotlin unit tests use normal official compilers. Independent Java assertions, Kotlin metadata consumers, all-class JVM verification, original JUnit tests, AAR inventories and D8 generation are checked. This does not establish full-app or Android-device runtime compatibility or incremental speed.",
    }

    def copy_evidence(path, name):
        destination = evidence / name
        shutil.copy2(path, destination)
        report["evidence"].append(relative(destination))
        return destination

    def run(label, arguments, allowed=(0,), timeout=120):
        index = len(commands)
        entry = {"label": label, "argv": [str(item) for item in arguments], "cwd": ".", "timeout_seconds": timeout}
        commands.append(entry)
        print(label, flush=True)
        try:
            result = subprocess.run(entry["argv"], cwd=ROOT, capture_output=True, text=True, timeout=timeout)
            entry["exit_code"] = result.returncode
            stdout, stderr = result.stdout, result.stderr
        except subprocess.TimeoutExpired as error:
            entry["timed_out"] = True
            stdout, stderr = error.stdout or b"", error.stderr or b""
            if isinstance(stdout, bytes):
                stdout = stdout.decode(errors="replace")
            if isinstance(stderr, bytes):
                stderr = stderr.decode(errors="replace")
            result = None
        for stream, text in [("stdout", stdout), ("stderr", stderr)]:
            path = evidence / f"{index:03d}-{stream}.txt"
            path.write_text(text)
            entry[stream] = relative(path)
            report["evidence"].append(relative(path))
        if result is None:
            raise TimeoutError(label + " timed out")
        if result.returncode not in allowed:
            raise RuntimeError(f"{label} exited {result.returncode}: {stderr[-3000:]}")
        return result

    def junit_results(build, provider):
        results = {}
        for index, item in enumerate(build["test_results"]):
            path = copy_evidence(item, f"{provider}-junit-{index}.xml")
            tree = ET.parse(path)
            for case in tree.findall(".//testcase"):
                name = case.attrib["classname"] + "/" + case.attrib["name"]
                outcome = "fail" if case.find("failure") is not None or case.find("error") is not None else "skipped" if case.find("skipped") is not None else "pass"
                if name in results:
                    raise AssertionError("Duplicate original JUnit test: " + name)
                results[name] = outcome
        return results

    try:
        java, javac, javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([java, javac, javap]):
            raise RuntimeError("Java, javac and javap are required")
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        builds = {}
        for provider, path in [("official", args.official_json), ("candidate", args.candidate_json)]:
            build = json.loads(path.read_text())
            if build["status"] != "passed" or build["returncode"] != 0 or build["production_kotlin_fallback"]:
                raise AssertionError("Module build did not pass without production fallback: " + provider)
            builds[provider] = build
            copy_evidence(path, provider + "-build.json")
            copy_evidence(build["log"], provider + "-gradle.log")
            artifact_root = path.parent
            for item in sorted(artifact_root.glob("compile*Kotlin*.json")):
                copy_evidence(item, provider + "-" + item.name)
            report["inputs"][provider + "_build"] = {"path": relative(path), "sha256": sha(path), "command": build["command"], "options": build["options"], "production_kotlin_compiler": build["production_kotlin_compiler"], "production_kotlin_fallback": False}
            cases.append({"id": "gradle_module/" + provider, "outcome": "pass", "scope": "Real Gradle/AGP module build, release AAR and original debug unit tests; dependencies and test Kotlin use official compilers."})
        official, candidate = builds["official"], builds["candidate"]
        version_check = official["resolved_kotlin_compiler_version_check"]
        version_text = version_check.get("stdout", "") + version_check.get("stderr", "")
        if official["resolved_kotlin_compiler_version"] != pin["version"] or version_check["returncode"] != 0 or ("kotlinc-jvm " + pin["version"] + " ") not in version_text:
            raise AssertionError("Resolved Gradle reference compiler differs from pinned Kotlin")
        for compiler_file, checksum in official["resolved_kotlin_compiler_files_sha256"].items():
            if sha(compiler_file) != checksum:
                raise AssertionError("Resolved official compiler dependency changed")
        if sha(candidate["compiler"]) != candidate["compiler_sha256"]:
            raise AssertionError("Candidate compiler changed after module build")
        owned = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            owned.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
        report["compiler"] = {"version": run("Korrust version", [candidate["compiler"], "--version"]).stdout.strip(), "binary_sha256": candidate["compiler_sha256"], "source_files_sha256": {relative(path): sha(path) for path in sorted(owned)}, "kotlin_metadata": True, "kotlin_metadata_scope": "Bounded classes and top-level function facades; per-source package mappings", "incremental_compilation": False}
        report["reference_compiler"] = {"version": pin["version"], "compiler_jar_sha256": sha(ROOT / pin["path"] / "lib/kotlin-compiler.jar"), "gradle_compiler_version": official.get("resolved_kotlin_compiler_version"), "gradle_compiler_files_sha256": official.get("resolved_kotlin_compiler_files_sha256"), "options": official["options"]}
        report["environment"]["java"] = run("Java version", [java, "-version"]).stderr.strip()
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        main_sources = sorted(path for path in (SNAPSHOT / MODULE / "src/main").rglob("*") if path.suffix in [".kt", ".java"])
        test_sources = sorted(path for path in (SNAPSHOT / MODULE / "src/test").rglob("*") if path.suffix in [".kt", ".java"])
        kotlin_sources = [str(path.relative_to(SNAPSHOT)) for path in main_sources if path.suffix == ".kt"]
        java_sources = [str(path.relative_to(SNAPSHOT)) for path in main_sources if path.suffix == ".java"]
        if len(kotlin_sources) != 2 or len(java_sources) != 22:
            raise AssertionError("Unexpected original production module inventory")
        source_hashes = {}
        for path in main_sources + test_sources:
            name = str(path.relative_to(SNAPSHOT))
            checksum = sha(path)
            if checksum != inventory[name]:
                raise AssertionError("Original app snapshot was changed: " + name)
            source_hashes[name] = checksum
            for build in builds.values():
                if sha(Path(build["work_dir"]) / name) != checksum:
                    raise AssertionError("Build copy source was changed: " + name)
                if name in build["source_files_sha256"] and build["source_files_sha256"][name] != checksum:
                    raise AssertionError("Build source hash differs: " + name)
        for build in builds.values():
            if build["production_kotlin_sources"] != kotlin_sources or build["production_java_sources"] != java_sources:
                raise AssertionError("Gradle production inventory differs from snapshot")
        report["inputs"].update({"androidbible_commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["androidbible"]["commit"], "production_kotlin_sources": kotlin_sources, "production_java_sources": java_sources, "source_files_sha256": source_hashes})
        cases.append({"id": "unchanged_module_sources", "outcome": "pass", "kotlin_sources": 2, "java_sources": 22, "scope": "All production and test source files match preserved snapshot and manifest hashes in both build copies."})
        jars, inventories, major_versions = {}, {}, {}
        for provider, build in builds.items():
            aar = copy_evidence(build["aar"], provider + "-AlkitabYes2-release.aar")
            with zipfile.ZipFile(aar) as archive:
                if "AndroidManifest.xml" not in archive.namelist() or "classes.jar" not in archive.namelist():
                    raise AssertionError("AAR lacks manifest or classes.jar")
                jar_bytes = archive.read("classes.jar")
            jar = work / (provider + "-classes.jar")
            jar.write_bytes(jar_bytes)
            jars[provider] = jar
            with zipfile.ZipFile(io.BytesIO(jar_bytes)) as archive:
                classes = {name: hashlib.sha256(archive.read(name)).hexdigest() for name in sorted(archive.namelist()) if name.endswith(".class")}
                major_versions[provider] = sorted({int.from_bytes(archive.read(name)[6:8], "big") for name in classes})
                mappings = {name: hashlib.sha256(archive.read(name)).hexdigest() for name in sorted(archive.namelist()) if name.endswith(".kotlin_module")}
            inventories[provider] = classes
            report["inputs"][provider + "_aar"] = {"path": relative(aar), "sha256": sha(aar), "classes_jar_sha256": sha(jar), "classes_sha256": classes, "module_mappings_sha256": mappings, "classfile_major_versions": major_versions[provider]}
        trace_root = args.candidate_json.parent
        for task in ["compileDebugKotlin", "compileReleaseKotlin"]:
            trace = json.loads((trace_root / (task + ".json")).read_text())
            request = json.loads((trace_root / (task + "-request.json")).read_text())
            if trace["fallback"] or trace["compiler_sha256"] != candidate["compiler_sha256"] or trace["task"] != ":AlkitabYes2:" + task:
                raise AssertionError("Candidate compile trace compiler/task/fallback mismatch")
            invocations = checked_compile_invocations(trace, request, candidate["compiler"], source_hashes, candidate["work_dir"])
            invoked_sources = [str(Path(item["source"]).relative_to(candidate["work_dir"])) for item in trace_source_records(trace)]
            if sorted(invoked_sources) != kotlin_sources:
                raise AssertionError("Candidate compile trace does not cover exactly both original Kotlin sources")
            stage = trace_root / "module-build/tmp/korrust" / task
            recorded_stage = Path(request["stage"])
            emitted = set()
            with zipfile.ZipFile(jars["candidate"]) as archive:
                for invocation in invocations:
                    class_outputs = [Path(line.strip()).relative_to(recorded_stage) for line in invocation["stdout"].splitlines() if line.strip().endswith(".class")]
                    if not class_outputs:
                        raise AssertionError("Candidate invocation records no emitted class")
                    for output in class_outputs:
                        name = output.as_posix()
                        emitted.add(name)
                        staged = (stage / output).read_bytes()
                        destination = Path(candidate["kotlin_classes"] if task == "compileReleaseKotlin" else candidate["debug_kotlin_classes"]) / output
                        if staged != destination.read_bytes():
                            raise AssertionError("Korrust-stage class differs from Gradle Kotlin output")
                        if task == "compileReleaseKotlin" and staged != archive.read(name):
                            raise AssertionError("Korrust-stage class bytes differ from packaged release AAR")
            staged_classes = {path.relative_to(stage).as_posix() for path in stage.rglob("*.class")}
            if emitted != staged_classes or len(emitted) != 2:
                raise AssertionError("Korrust-stage emitted class inventory differs from trace")
            cases.append({"id": "candidate_production_trace/" + task, "outcome": "pass", "kotlin_sources": 2, "compiler_sha256": trace["compiler_sha256"], "emitted_classes": sorted(emitted), "scope": "Both exact original source hashes invoked Korrust successfully without fallback; staged class bytes match Gradle output and release bytes match packaged AAR."})
        report["compiler"]["classfile_major_versions"] = major_versions["candidate"]
        report["compiler"]["configured_gradle_jvm_target"] = candidate["options"]["kotlin_jvm_target"]
        report["compiler"]["jvm_target_scope"] = "Gradle retains target21; Korrust emits compatible older class versions49/52 and has no configurable target option"
        if inventories["official"].keys() != inventories["candidate"].keys():
            raise AssertionError("Full module AAR class inventories differ")
        names = [name[:-6].replace("/", ".") for name in inventories["official"]]
        for required in ["yuku.alkitab.yes2.Yes2SingleChapterVerses", "yuku.alkitab.yes2.util.UnsignedBinarySearchKt"]:
            if required not in names:
                raise AssertionError("Module AAR missing selected Kotlin output: " + required)
        cases.append({"id": "aar_class_inventory", "outcome": "pass", "classes": len(names), "scope": "Both real release AARs contain identical complete class inventories; original Kotlin source outputs are present. Exact class bytes and module mapping names may differ."})
        shared = [path for path in official["test_runtime_classpath"] if not str(path).startswith(str(args.official_json.parent / "module-build")) or "debugUnitTest" in str(path)]
        shared = list(dict.fromkeys([*official["test_classes"], *shared]))
        drivers = work / "drivers"
        run("Compile independent Java callers against official module AAR", [javac, "-cp", classpath(jars["official"], *shared), "-d", drivers, ROOT / "tests/fixtures/Yes2Check.java", ROOT / "tests/fixtures/BinarySearchCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])
        for name, case_id in [("Yes2Check", "module_java_chapter"), ("BinarySearchCheck", "module_java_unsigned_search")]:
            outputs = []
            for provider in builds:
                outputs.append(run("Run " + name + " against " + provider + " AAR", [java, "-Xverify:all", "-cp", classpath(drivers, jars[provider], *shared), name]).stdout)
            if outputs[0] != outputs[1]:
                raise AssertionError("Independent module Java observations differ: " + name)
            result = json.loads(outputs[0])
            cases.append({"id": case_id, "outcome": "pass", "runtime_checks": result["checks"], "result_sha256": result["sha256"], "scope": "Java caller compiled once against official module AAR and run against each provider with JVM verification all."})
        for provider in builds:
            run("Verify all " + provider + " module classes", [java, "-Xverify:all", "-cp", classpath(drivers, jars[provider], *shared), "ModuleClassVerification", *names])
            cases.append({"id": "all_class_verification/" + provider, "outcome": "pass", "classes": len(names), "scope": "Class loading without initialization and reflective field/method/constructor resolution under -Xverify:all."})
        for name in names:
            expected = run("Inspect official module JVM signatures " + name, [javap, "-p", "-s", "-classpath", jars["official"], name]).stdout
            actual = run("Inspect candidate module JVM signatures " + name, [javap, "-p", "-s", "-classpath", jars["candidate"], name]).stdout
            if expected != actual:
                raise AssertionError("Module JVM signatures differ: " + name)
            cases.append({"id": "module_jvm_signatures/" + name, "outcome": "pass", "scope": "javap -p -s of complete module class inventory."})
        options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", official["options"]["kotlin_jvm_target"]]
        outputs = []
        for provider in builds:
            destination = work / (provider + "-consumer")
            run("Compile friend Kotlin consumer against " + provider + " AAR", [kotlinc, ROOT / "tests/fixtures/consumers/ModuleFriendConsumer.kt", "-cp", classpath(jars[provider], *shared), *options, "-Xfriend-paths=" + str(jars[provider]), "-d", destination])
            outputs.append(run("Run friend Kotlin consumer against " + provider + " AAR", [java, "-Xverify:all", "-cp", classpath(destination, jars[provider], *shared), "korrust.consumers.ModuleFriendConsumerKt"]).stdout)
            for consumer, friend, marker in [("InternalClassAccess", False, "internal"), ("PrivateFieldAccess", True, "private")]:
                rejected = run("Kotlin module metadata rejection " + consumer + " against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers" / (consumer + ".kt"), "-cp", classpath(jars[provider], *shared), *options, *(["-Xfriend-paths=" + str(jars[provider])] if friend else []), "-d", work / (provider + "-rejected-" + consumer)], allowed=(1,))
                if "error:" not in rejected.stderr or marker not in rejected.stderr.lower():
                    raise AssertionError("Unexpected Kotlin module metadata diagnostic")
        if outputs[0] != outputs[1]:
            raise AssertionError("Module Kotlin metadata consumer differs")
        cases.append({"id": "module_kotlin_metadata", "outcome": "pass", "scope": "Pinned official Kotlin compiles and runs friend consumer of internal class and public facade from each AAR; no-friend internal class and friend-private field access rejected."})
        original_results = {provider: junit_results(build, provider) for provider, build in builds.items()}
        if not original_results["official"] or original_results["official"] != original_results["candidate"]:
            raise AssertionError("Original module JUnit identities/outcomes differ")
        for name, outcome in sorted(original_results["official"].items()):
            if outcome != "pass":
                raise AssertionError("Original module JUnit did not pass: " + name)
            cases.append({"id": "original_junit/" + name, "outcome": "pass", "providers": ["official", "candidate"]})
        test_names = sorted({name.split("/", 1)[0] for name in original_results["official"]})
        for provider in builds:
            result = run("Run fixed official-compiled original JUnit suite against " + provider + " AAR", [java, "-Xverify:all", "-cp", classpath(*official["test_classes"], jars[provider], *shared), "org.junit.runner.JUnitCore", *test_names])
            if not re.search(r"OK \(" + str(len(original_results["official"])) + r" tests?\)", result.stdout):
                raise AssertionError("Fixed original JUnit suite count differs")
        cases.append({"id": "fixed_original_test_bytecode", "outcome": "pass", "original_tests": len(original_results["official"]), "scope": "Identical original test classes compiled against official production run against both release AARs under -Xverify:all."})
        d8 = args.sdk / "build-tools/36.0.0/d8"
        android = args.sdk / "platforms/android-36/android.jar"
        if not d8.is_file() or not android.is_file():
            raise RuntimeError("Android build-tools 36.0.0 D8 and API36 android.jar are required")
        for provider, build in builds.items():
            destination = work / (provider + "-dex")
            destination.mkdir()
            d8_classpath = [path for path in build["classpath"] if Path(path) != android]
            d8_options = [item for path in d8_classpath for item in ["--classpath", path]]
            run("Generate " + provider + " module DEX", [d8, "--release", "--min-api", "26", "--lib", android, *d8_options, "--output", destination, jars[provider]])
            dex = sorted(destination.glob("*.dex"))
            if not dex:
                raise AssertionError("D8 emitted no DEX")
            cases.append({"id": "d8_generation/" + provider, "outcome": "pass", "min_api": 26, "dex_sha256": {relative(path): sha(path) for path in dex}, "scope": "D8 accepts complete module classes with actual dependency classpath and API36 library. No Android-device/runtime execution."})
        report["status"] = "completed"
    except Exception as error:
        report["status"] = "failed"
        cases.append({"id": "harness_failure", "outcome": "timeout" if isinstance(error, TimeoutError) else "fail", "message": str(error)})
        print(str(error), file=sys.stderr)
    finally:
        report["counts"] = dict(Counter(case["outcome"] for case in cases))
        report["finished_at"] = datetime.now(timezone.utc).isoformat()
        report_path.write_text(json.dumps(report, indent=2) + "\n")
        print("Report: " + relative(report_path), flush=True)
    if report["status"] == "completed":
        common = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        feature = {**common, "kind": "features", "status": "assessed", "commands": [["python3", "tools/check_androidbible_module.py"]], "evidence": [relative(report_path)], "features": [{"name": "androidbible_alkitabyes2_module", "status": "supported", "scope": "All unchanged production sources of the pinned AlkitabYes2 module compile through real Gradle/AGP with Korrust for both Kotlin sources and javac for all 22 Java sources; real release AAR, original 19 JUnit tests, independent Java/Kotlin consumers, complete JVM signature/class verification and D8 generation validated.", "evidence_cases": [case["id"] for case in cases], "restrictions": ["Only this pinned module/source snapshot is established. Dependencies and test Kotlin use normal compilers; no selected production Kotlin fallback.", "D8 generation and JVM tests do not establish Android-device runtime behavior or complete application compatibility.", "No incremental dependency graph/cache or controlled compile/edit-to-running speed measurement is established."]}]}
        (ROOT / "reports/features" / (run_id + ".json")).write_text(json.dumps(feature, indent=2) + "\n")
    return 0 if report["status"] == "completed" else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
