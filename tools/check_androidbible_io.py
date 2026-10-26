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

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha, trace_source_records, checked_compile_invocations


MODULES = ["AlkitabIo", "AlkitabModel", "AlkitabYes2"]


SUPPLIER_CLASSES = {"yuku/alkitab/io/Utf8Decoder$byteBuffer$Supplier.class", "yuku/alkitab/io/Utf8Decoder$charBuffer$Supplier.class"}


def source_abi(text, fields=None):
    fields = {"byteBuffer": "byte", "charBuffer": "char"} if fields is None else fields
    allowed = {(f"public static final {kind}[] access${name}$lambda$0();", "descriptor: ()[" + ("B" if kind == "byte" else "C")) for name, kind in fields.items()}
    lines = [line for line in text.splitlines() if line.strip()]
    prefix, members = [], []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith("  ") and not line.lstrip().startswith("descriptor:"):
            descriptor = lines[index + 1].strip()
            if not descriptor.startswith("descriptor:"):
                raise AssertionError("Unexpected javap member format")
            if (line.strip(), descriptor) not in allowed:
                members.append((line.strip(), descriptor))
            index += 2
        else:
            prefix.append(line.strip())
            index += 1
    return prefix, sorted(members)


def compiler_diagnostics(text):
    return "\n".join(line for line in text.splitlines()
                     if not line.startswith(("Picked up JAVA_TOOL_OPTIONS:", "Picked up _JAVA_OPTIONS:", "NOTE: Picked up JDK_JAVA_OPTIONS:")))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--official-json", type=Path, default=ROOT / "target/androidbible-model-io-yes2-artifacts/official/build.json")
    parser.add_argument("--candidate-json", type=Path, default=ROOT / "target/androidbible-model-io-yes2-artifacts/korrust/build.json")
    parser.add_argument("--d8-jar", type=Path, default=Path("/workspace/korrust-setup/gradle-home/caches/modules-2/files-2.1/com.android.tools.build/builder/9.2.1/e8ef7f9072cd990708591c54128fd6c6dc42ae7d/builder-9.2.1.jar"))
    parser.add_argument("--compiler-root", type=Path, default=ROOT)
    parser.add_argument("--build-tool", type=Path, default=ROOT / "tools/build_androidbible.py")
    parser.add_argument("--sdk", type=Path, default=Path("/workspace/korrust-setup/android-sdk"))
    args = parser.parse_args()
    args.official_json, args.candidate_json = args.official_json.resolve(), args.candidate_json.resolve()
    now = datetime.now(timezone.utc)
    run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-androidbible-model-io-yes2"
    work = ROOT / "target/checks" / run_id
    evidence = ROOT / "reports/correctness" / run_id
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    report_path = evidence.with_suffix(".json")
    cases, commands = [], []
    report = {"schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running", "compiler": {}, "reference_compiler": {}, "inputs": {}, "environment": {"os": platform.platform(), "machine": platform.machine()}, "commands": commands, "evidence": [], "cases": cases, "selection": "Batch-integration revalidation of previously demonstrated module coverage, not new module coverage. Complete unchanged AlkitabModel, AlkitabIo and AlkitabYes2 production modules: 4 Kotlin and 39 Java sources through real Gradle/AGP Java/resource/dependency/AAR tasks, all selected modules production Kotlin compiled exclusively by Korrust. Dependencies and original test Kotlin use normal compilers. All three AAR class inventories/signatures, exact candidate source traces/staged bytes, independent Java/Kotlin consumers, 27 original Io/Yes2 JUnit tests and standalone invocation of actual AGP9.2.1 embedded D8 with strict warning-free diagnostics are checked. The Gradle library AAR task graph and standalone D8 check are separate evidence. No Android-device/full-app runtime or incremental speed claim."}

    def archive(path, name):
        destination = evidence / name
        shutil.copy2(path, destination)
        report["evidence"].append(relative(destination))
        return destination

    def run(label, argv, allowed=(0,), timeout=120, empty=False):
        entry = {"label": label, "argv": [str(item) for item in argv], "cwd": ".", "timeout_seconds": timeout}
        index = len(commands)
        commands.append(entry)
        print(label, flush=True)
        try:
            result = subprocess.run(entry["argv"], cwd=ROOT, capture_output=True, text=True, timeout=timeout)
            stdout, stderr = result.stdout, result.stderr
            entry["exit_code"] = result.returncode
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
        if empty and (compiler_diagnostics(stdout).strip() or compiler_diagnostics(stderr).strip()):
            raise AssertionError(label + " emitted unexpected diagnostics")
        return result

    try:
        java, javac, javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([java, javac, javap]):
            raise RuntimeError("JDK tools are required")
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        builds = {}
        manifests = {"official": args.official_json, "candidate": args.candidate_json}
        for provider, path in manifests.items():
            build = json.loads(path.read_text())
            if build["status"] != "passed" or build["returncode"] != 0 or build["production_kotlin_fallback"] or sorted(build["selected_modules"]) != MODULES:
                raise AssertionError("Combined build did not pass without selected production fallback")
            if not build["sources_unchanged"] or build["source_files_sha256"] != build["source_files_sha256_after_build"]:
                raise AssertionError("Combined source hashes changed during build")
            builds[provider] = build
            archive(path, provider + "-build.json")
            archive(build["log"], provider + "-gradle.log")
            if build["tool_sha256"] != sha(args.build_tool.resolve()):
                raise AssertionError("Build adapter tool changed after build")
            adapter = path.parent / "adapter.init.gradle"
            if sha(adapter) != build["adapter_sha256"]:
                raise AssertionError("Build init adapter changed after build")
            archive(adapter, provider + "-adapter.init.gradle")
            report["inputs"][provider + "_build"] = {"path": relative(path), "sha256": sha(path), "command": build["command"]}
            cases.append({"id": "combined_gradle/" + provider, "outcome": "pass", "scope": "All selected modules real release AARs and test tasks; Model has no original unit tests, Yes2 original tests use official test compiler."})
        official, candidate = builds["official"], builds["candidate"]
        version = official["resolved_kotlin_compiler_version_check"]
        if official["resolved_kotlin_compiler_version"] != pin["version"] or version["returncode"] != 0 or ("kotlinc-jvm " + pin["version"] + " ") not in version.get("stdout", "") + version.get("stderr", ""):
            raise AssertionError("Resolved Gradle Kotlin reference is not pinned")
        for path, checksum in official["resolved_kotlin_compiler_files_sha256"].items():
            if sha(path) != checksum:
                raise AssertionError("Reference compiler dependency changed")
        if sha(candidate["compiler"]) != candidate["compiler_sha256"]:
            raise AssertionError("Candidate compiler changed after build")
        compiler_root = args.compiler_root.resolve()
        sources = [compiler_root / "Cargo.toml", compiler_root / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            sources.extend(path for path in (compiler_root / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
        frozen_files = {relative(path): sha(path) for path in sorted(sources)}
        report["inputs"]["verification_tool_sha256"] = sha(Path(__file__))
        report["reproduction_commands"] = [[sys.executable, *sys.argv]]
        report["compiler"] = {"version": run("Korrust version", [candidate["compiler"], "--version"]).stdout.strip(), "binary_sha256": candidate["compiler_sha256"], "source_files_sha256": frozen_files, "kotlin_metadata": True, "incremental_compilation": False, "jvm_target_scope": "Configured Gradle target21; Korrust emits classfile49/52 and javac emits65. No configurable Korrust target option; Io may emit synthetic Supplier helper classes instead of official invokedynamic."}
        report["reference_compiler"] = {"version": pin["version"], "compiler_jar_sha256": sha(ROOT / pin["path"] / "lib/kotlin-compiler.jar"), "resolved_gradle_compiler_files_sha256": official["resolved_kotlin_compiler_files_sha256"]}
        report["environment"]["java"] = run("Java version", [java, "-version"]).stderr.strip()
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        expected_sources, expected_kotlin, expected_java = {}, {}, {}
        for module in MODULES:
            main = sorted(path for path in (SNAPSHOT / module / "src/main").rglob("*") if path.suffix in [".kt", ".java"])
            expected_kotlin[module] = [str(path.relative_to(SNAPSHOT)) for path in main if path.suffix == ".kt"]
            expected_java[module] = [str(path.relative_to(SNAPSHOT)) for path in main if path.suffix == ".java"]
            tests = sorted(path for path in (SNAPSHOT / module / "src/test").rglob("*") if path.suffix in [".kt", ".java"])
            for path in main + tests:
                name = str(path.relative_to(SNAPSHOT))
                checksum = sha(path)
                if checksum != inventory[name]:
                    raise AssertionError("Preserved app snapshot changed: " + name)
                expected_sources[name] = checksum
                for build in builds.values():
                    if sha(Path(build["work_dir"]) / name) != checksum or (name in build["source_files_sha256"] and build["source_files_sha256"][name] != checksum):
                        raise AssertionError("Build source differs from pinned snapshot: " + name)
            for build in builds.values():
                info = build["modules"][module]
                if info["production_kotlin_sources"] != expected_kotlin[module] or info["production_java_sources"] != expected_java[module]:
                    raise AssertionError("Selected source inventory mismatch")
        for build in builds.values():
            if build["source_files_sha256"] != expected_sources or build["source_files_sha256_after_build"] != expected_sources:
                raise AssertionError("Complete selected source manifest differs from pinned inventory")
            for module in MODULES:
                expected_module = {name: checksum for name, checksum in expected_sources.items() if name.startswith(module + "/")}
                info = build["modules"][module]
                if info["source_files_sha256"] != expected_module or info["source_files_sha256_after_build"] != expected_module or not info["sources_unchanged"]:
                    raise AssertionError("Per-module source manifest differs from pinned inventory")
        if sum(map(len, expected_kotlin.values())) != 4 or sum(map(len, expected_java.values())) != 39:
            raise AssertionError("Unexpected selected module production source counts")
        report["inputs"].update({"androidbible_commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["androidbible"]["commit"], "production_kotlin_sources": expected_kotlin, "production_java_sources": expected_java, "source_files_sha256": expected_sources})
        cases.append({"id": "unchanged_combined_sources", "outcome": "pass", "kotlin_sources": 4, "java_sources": 39, "scope": "Every production/test source checked against pinned snapshot inventory and both disposable build copies."})
        jars, class_hashes = {}, {}
        for provider, build in builds.items():
            jars[provider], class_hashes[provider] = {}, {}
            for module in MODULES:
                info = build["modules"][module]
                aar = archive(info["aar"], provider + "-" + module + ".aar")
                with zipfile.ZipFile(aar) as package:
                    if "AndroidManifest.xml" not in package.namelist():
                        raise AssertionError("AAR manifest missing")
                    jar_bytes = package.read("classes.jar")
                jar = work / (provider + "-" + module + "-classes.jar")
                jar.write_bytes(jar_bytes)
                jars[provider][module] = jar
                with zipfile.ZipFile(io.BytesIO(jar_bytes)) as classes:
                    hashes = {name: hashlib.sha256(classes.read(name)).hexdigest() for name in sorted(classes.namelist()) if name.endswith(".class")}
                    majors = sorted({int.from_bytes(classes.read(name)[6:8], "big") for name in hashes})
                    mappings = {name: hashlib.sha256(classes.read(name)).hexdigest() for name in classes.namelist() if name.endswith(".kotlin_module")}
                class_hashes[provider][module] = hashes
                report["inputs"][provider + "_" + module + "_aar"] = {"path": relative(aar), "sha256": sha(aar), "classes_jar_sha256": sha(jar), "classes_sha256": hashes, "classfile_major_versions": majors, "module_mappings_sha256": mappings, "options": info["options"]}
        for module in MODULES:
            official_names, candidate_names = set(class_hashes["official"][module]), set(class_hashes["candidate"][module])
            allowed_extras = SUPPLIER_CLASSES if module == "AlkitabIo" else set()
            if candidate_names != official_names | allowed_extras:
                raise AssertionError("AAR source/synthetic class inventories differ: " + module)
            cases.append({"id": "aar_inventory/" + module, "outcome": "pass", "official_classes": len(official_names), "candidate_classes": len(candidate_names), "legitimate_synthetic_extras": sorted(allowed_extras), "scope": "Source-defined classes match. Io additionally has exactly two declared synthetic noncapturing Supplier lowering classes, validated independently; synthetic inventories intentionally differ."})
            trace_root = args.candidate_json.parent / "modules" / module
            for task in ["compileDebugKotlin", "compileReleaseKotlin"]:
                trace_path = trace_root / (task + ".json")
                request_path = trace_root / (task + "-request.json")
                archive(trace_path, module + "-" + task + ".json")
                archive(request_path, module + "-" + task + "-request.json")
                trace, request = json.loads(trace_path.read_text()), json.loads(request_path.read_text())
                if trace["fallback"] or trace["compiler_sha256"] != candidate["compiler_sha256"] or trace["task"] != ":" + module + ":" + task:
                    raise AssertionError("Candidate trace task/compiler/fallback mismatch")
                invocations = checked_compile_invocations(trace, request, candidate["compiler"], expected_sources, candidate["work_dir"])
                invoked = [str(Path(item["source"]).relative_to(candidate["work_dir"])) for item in trace_source_records(trace)]
                if sorted(invoked) != expected_kotlin[module]:
                    raise AssertionError("Trace source inventory mismatch")
                emitted = set()
                stage = trace_root / "module-build/tmp/korrust" / task
                with zipfile.ZipFile(jars["candidate"][module]) as package:
                    for invocation in invocations:
                        outputs = [Path(line.strip()).relative_to(request["stage"]) for line in invocation["stdout"].splitlines() if line.strip().endswith(".class")]
                        if not outputs:
                            raise AssertionError("No class outputs recorded")
                        for output in outputs:
                            emitted.add(output.as_posix())
                            payload = (stage / output).read_bytes()
                            destination = Path(candidate["modules"][module]["kotlin_classes" if task == "compileReleaseKotlin" else "debug_kotlin_classes"]) / output
                            if payload != destination.read_bytes() or (task == "compileReleaseKotlin" and payload != package.read(output.as_posix())):
                                raise AssertionError("Candidate staged bytes differ from Gradle/AAR")
                if emitted != {path.relative_to(stage).as_posix() for path in stage.rglob("*.class")}:
                    raise AssertionError("Candidate staged class inventory mismatch")
                for dependency_module in (["AlkitabModel"] if module == "AlkitabIo" else ["AlkitabModel", "AlkitabIo"] if module == "AlkitabYes2" else []):
                    variant = "release" if task == "compileReleaseKotlin" else "debug"
                    dependencies = [item for item in trace["classpath"] if item["kind"] == "jar" and ("/" + dependency_module + "/") in item["path"] and ("/compile_library_classes_jar/" + variant + "/") in item["path"]]
                    if len(dependencies) != 1:
                        raise AssertionError("Selected compilation dependency missing/ambiguous: " + module + " -> " + dependency_module)
                    dependency = dependencies[0]
                    snapshot = args.candidate_json.parent / "modules" / dependency_module / "module-build"
                    try:
                        suffix = Path(dependency["path"]).relative_to(Path(candidate["work_dir"]) / dependency_module / "build")
                    except ValueError:
                        suffix = Path(dependency["path"]).relative_to(snapshot)
                    archived_dependency = snapshot / suffix
                    if sha(archived_dependency) != dependency["sha256"]:
                        raise AssertionError("Archived selected dependency differs from actual compiler trace")
                    producer = candidate["modules"][dependency_module]
                    kotlin_output = Path(producer["kotlin_classes"] if variant == "release" else producer["debug_kotlin_classes"])
                    java_output = Path(producer["java_classes"] if variant == "release" else producer["java_classes"].replace("/javac/release/compileReleaseJavaWithJavac/", "/javac/debug/compileDebugJavaWithJavac/"))
                    producer_hashes = {path.relative_to(folder).as_posix(): sha(path) for folder in [kotlin_output, java_output] for path in folder.rglob("*.class")}
                    if producer_hashes.keys() != class_hashes["candidate"][dependency_module].keys():
                        raise AssertionError("Selected producer variant class inventory differs")
                    with zipfile.ZipFile(archived_dependency) as dependency_jar:
                        for class_name, checksum in producer_hashes.items():
                            if hashlib.sha256(dependency_jar.read(class_name)).hexdigest() != checksum:
                                raise AssertionError("Selected dependency contains different class bytes: " + class_name)
                    cases.append({"id": "selected_dependency/" + module + "/" + dependency_module + "/" + task, "outcome": "pass", "dependency_jar": relative(archived_dependency), "dependency_sha256": sha(archived_dependency), "scope": "Actual consumer Korrust trace resolves the selected producer jar; every producer class byte, including Io Supplier helpers, matches its candidate packaged AAR. No official selected-module fallback dependency."})
                cases.append({"id": "candidate_trace/" + module + "/" + task, "outcome": "pass", "sources": len(invoked), "emitted_classes": sorted(emitted), "scope": "Exact original sources invoked same frozen Korrust without fallback; staged bytes match Gradle outputs and release packaged AAR."})
        official_tests = official["modules"]["AlkitabYes2"]
        selected_roots = [str(manifests["official"].parent / "modules" / module) for module in MODULES]
        fixed_test_classes = [path for module in ["AlkitabIo", "AlkitabYes2"] for path in official["modules"][module]["test_classes"]]
        shared = [path for module in ["AlkitabIo", "AlkitabYes2"] for path in official["modules"][module]["test_runtime_classpath"] if not any(str(path).startswith(prefix) for prefix in selected_roots) or "debugUnitTest" in str(path)]
        shared = list(dict.fromkeys([*fixed_test_classes, *shared]))
        report["inputs"]["fixed_official_test_classes_sha256"] = {str(path): sha(path) for directory in fixed_test_classes for path in Path(directory).rglob("*.class")}
        drivers = work / "drivers"
        run("Compile independent Java callers against official module set", [javac, "-cp", classpath(*jars["official"].values(), *shared), "-d", drivers, *[ROOT / "tests/fixtures" / name for name in ["InterfaceAssertions.java", "ModelCheck.java", "Yes2Check.java", "BinarySearchCheck.java", "ModuleClassVerification.java", "io/IoCheck.java", "io/SupplierCheck.java"]]])
        for name, case_id in [("ModelCheck", "combined_model_java"), ("Yes2Check", "combined_yes2_java"), ("BinarySearchCheck", "combined_unsigned_search_java"), ("IoCheck", "combined_io_java")]:
            results = [run("Run " + name + " against " + provider + " module set", [java, "-Xverify:all", "-cp", classpath(drivers, *jars[provider].values(), *shared), name]).stdout for provider in builds]
            if results[0] != results[1]:
                raise AssertionError("Combined Java observations differ: " + name)
            result = json.loads(results[0])
            cases.append({"id": case_id, "outcome": "pass", "runtime_checks": result["checks"], "result_sha256": result["sha256"], "scope": "Java compiled once against official module set and run against both module sets with JVM verification all."})
        for module in MODULES:
            names = [name[:-6].replace("/", ".") for name in class_hashes["official"][module]]
            for provider in builds:
                run("Verify all " + module + " " + provider + " classes", [java, "-Xverify:all", "-cp", classpath(drivers, *jars[provider].values(), *shared), "ModuleClassVerification", *[item[:-6].replace("/", ".") for item in class_hashes[provider][module]]])
                cases.append({"id": "all_class_verification/" + module + "/" + provider, "outcome": "pass", "classes": len(class_hashes[provider][module])})
            for name in names:
                outputs = [run("Inspect " + provider + " combined ABI " + name, [javap, "-p", "-s", "-classpath", jars[provider][module], name]).stdout for provider in builds]
                fields = None if name == "yuku.alkitab.io.Utf8Decoder" else {}
                if source_abi(outputs[0], fields) != source_abi(outputs[1], fields):
                    raise AssertionError("Combined class ABI differs: " + name)
                cases.append({"id": "combined_jvm_signatures/" + name, "outcome": "pass", "scope": "All source-defined production classes, javap -p -s; Io object comparison excludes only derived synthetic Supplier access bridges and ignores member order. Candidate Supplier classes validated separately."})
        run("Verify explicit candidate Supplier lowering", [java, "-Xverify:all", "-cp", classpath(drivers, *jars["candidate"].values(), *shared), "SupplierCheck", "yuku.alkitab.io.Utf8Decoder"])
        cases.append({"id": "io_supplier_lowering", "outcome": "pass", "scope": "Exactly two synthetic stateless Supplier classes implement erased get() through synthetic array access bridges; per-get fresh arrays and per-thread cache identity verified. This is a legitimate alternative to official invokedynamic lowering."})
        options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", official_tests["options"]["kotlin_jvm_target"]]
        consumers = []
        for provider in builds:
            destination = work / (provider + "-consumer")
            run("Compile Kotlin consumer against " + provider + " module set", [kotlinc, ROOT / "tests/fixtures/consumers/ModuleIoConsumer.kt", "-cp", classpath(*jars[provider].values(), *shared), *options, "-Xfriend-paths=" + str(jars[provider]["AlkitabYes2"]), "-d", destination])
            consumers.append(run("Run Kotlin consumer against " + provider + " module set", [java, "-Xverify:all", "-cp", classpath(destination, *jars[provider].values(), *shared), "korrust.consumers.ModuleIoConsumerKt"]).stdout)
            for consumer, friend, marker in [("InternalClassAccess", False, "internal"), ("ModelPrivateAccess", False, "private"), ("IoPrivateAccess", False, "private"), ("IoNullAccess", False, "null")]:
                rejected = run("Kotlin combined rejection " + consumer + " against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers" / (consumer + ".kt"), "-cp", classpath(*jars[provider].values(), *shared), *options, *(["-Xfriend-paths=" + str(jars[provider]["AlkitabYes2"])] if friend else []), "-d", work / (provider + "-rejected-" + consumer)], allowed=(1,))
                if "error:" not in rejected.stderr or marker not in rejected.stderr.lower():
                    raise AssertionError("Unexpected combined metadata rejection")
        if consumers[0] != consumers[1]:
            raise AssertionError("Combined metadata consumer differs")
        cases.append({"id": "combined_kotlin_metadata", "outcome": "pass", "scope": "Official Kotlin compiles/runs ByteArray/overloaded-static Io object plus Model companion and internal Yes2 friend consumer against both three-module AAR sets; internal/private access enforced."})
        junit = {}
        for provider, build in builds.items():
            tests = {}
            for tested_module in ["AlkitabIo", "AlkitabYes2"]:
                for index, item in enumerate(build["modules"][tested_module]["test_results"]):
                    path = archive(item, provider + "-" + tested_module + "-junit-" + str(index) + ".xml")
                    for case in ET.parse(path).findall(".//testcase"):
                        name = case.attrib["classname"] + "/" + case.attrib["name"]
                        if name in tests or any(case.find(tag) is not None for tag in ["failure", "error", "skipped"]):
                            raise AssertionError("Original JUnit failed/skipped or duplicate")
                        tests[name] = "pass"
            junit[provider] = tests
        if len(junit["official"]) != 27 or junit["official"] != junit["candidate"]:
            raise AssertionError("Original 27 JUnit identities/outcomes differ")
        for name in sorted(junit["official"]):
            cases.append({"id": "original_junit/" + name, "outcome": "pass", "providers": ["official", "candidate"]})
        names = sorted({name.split("/", 1)[0] for name in junit["official"]})
        for provider in builds:
            result = run("Run fixed official original27JUnit against " + provider + " module set", [java, "-Xverify:all", "-cp", classpath(*fixed_test_classes, *jars[provider].values(), *shared), "org.junit.runner.JUnitCore", *names])
            if not re.search(r"OK \(27 tests\)", result.stdout):
                raise AssertionError("Fixed original JUnit count mismatch")
        cases.append({"id": "fixed_original_test_bytecode", "outcome": "pass", "original_tests": 27, "scope": "Same 27 original Io/Yes2 test bytecodes compiled against official module set run against both module sets under -Xverify:all."})
        d8_version = run("Actual AGP embedded D8 version", [java, "-cp", args.d8_jar, "com.android.tools.r8.D8", "--version"])
        if "D8 9.2.14 " not in d8_version.stdout or compiler_diagnostics(d8_version.stderr).strip():
            raise AssertionError("Unexpected actual AGP D8 version/diagnostics")
        report["inputs"]["agp_d8"] = {"jar": str(args.d8_jar), "sha256": sha(args.d8_jar), "version": d8_version.stdout.strip(), "agp_version": "9.2.1"}
        android = args.sdk / "platforms/android-36/android.jar"
        dependency_maps = {}
        for provider, build in builds.items():
            selected_prefixes = [str(manifests[provider].parent / "modules" / module) for module in MODULES]
            paths = {str(android), *[path for module in MODULES for path in build["modules"][module]["classpath"] if Path(path) != android and not any(str(path).startswith(prefix) for prefix in selected_prefixes)]}
            dependency_maps[provider] = {path: sha(path) for path in sorted(paths)}
        if dependency_maps["official"] != dependency_maps["candidate"]:
            raise AssertionError("Actual D8 external dependency/library inputs differ between providers")
        report["inputs"]["d8_external_dependencies_sha256"] = dependency_maps["official"]
        for provider, build in builds.items():
            output = work / (provider + "-dex")
            output.mkdir()
            selected_prefixes = [str(manifests[provider].parent / "modules" / module) for module in MODULES]
            dependencies = list(dict.fromkeys(path for module in MODULES for path in build["modules"][module]["classpath"] if Path(path) != android and not any(str(path).startswith(prefix) for prefix in selected_prefixes)))
            switches = [item for path in dependencies for item in ["--classpath", path]]
            run("Generate warning-free actual AGP DEX " + provider, [java, "-cp", args.d8_jar, "com.android.tools.r8.D8", "--release", "--min-api", "26", "--lib", android, *switches, "--output", output, *jars[provider].values()], empty=True)
            dex_files = sorted(output.glob("*.dex"))
            if not dex_files:
                raise AssertionError("D8 emitted no DEX")
            dex_hashes = {}
            for dex in dex_files:
                copied = archive(dex, provider + "-" + dex.name)
                dex_hashes[relative(copied)] = sha(copied)
            cases.append({"id": "combined_agp_d8/" + provider, "outcome": "pass", "min_api": 26, "dex_sha256": dex_hashes, "d8_diagnostics_empty": True, "jvm_startup_banners": "Preserved in raw command logs; excluded from D8 diagnostics.", "scope": "All three complete selected module AAR class jars accepted together by a standalone invocation of actual AGP9.2.1 embedded D8 9.2.14 without warnings/errors. Gradle AAR builds are recorded separately; no Gradle dexing task or Android-device execution is claimed."})
        if sha(candidate["compiler"]) != candidate["compiler_sha256"] or any(sha(ROOT / path) != checksum for path, checksum in frozen_files.items()):
            raise AssertionError("Frozen candidate/compiler source inputs changed during verification")
        cases.append({"id": "frozen_inputs", "outcome": "pass", "compiler_root": str(compiler_root)})
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
        feature = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        feature.update({"kind": "features", "status": "assessed", "commands": [["python3", "tools/check_androidbible_io.py"]], "evidence": [relative(report_path)], "features": [{"name": "androidbible_model_io_yes2_modules", "status": "supported", "scope": "Complete pinned AlkitabModel, AlkitabIo and AlkitabYes2 production modules, all 4 unchanged Kotlin and 39 Java sources. Korrust selected production Kotlin without fallback; real release AARs, all class ABI/JVM verification, independent Java/Kotlin consumers, 27 original Io/Yes2 tests and strict warning-free standalone AGP embedded D8 generation. This revalidates historical module coverage with the batch driver.", "evidence_cases": [case["id"] for case in cases], "restrictions": ["Only these pinned modules/sources are established; nonselected dependencies and original test Kotlin use normal compilers.", "D8/JVM evidence does not establish Android-device or full-application runtime compatibility; no incremental performance/dependency graph/cache claim."]}]})
        (ROOT / "reports/features" / (run_id + ".json")).write_text(json.dumps(feature, indent=2) + "\n")
    return 0 if report["status"] == "completed" else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
