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
import zipfile

from check_androidbible_module import ROOT, classpath, relative, sha


MODULES = ["feature:bookmarks:api", "feature:search:api"]
VARIANTS = ["demoDebug", "demoRelease", "prodDebug", "prodRelease"]
SNAPSHOT = ROOT / "benchmarks/sources/nowinandroid"
FIXTURES = ROOT / "tests/fixtures/serialization"
NAV_CLASSES = {
    "feature:bookmarks:api": "com.google.samples.apps.nowinandroid.feature.bookmarks.api.navigation.BookmarksNavKey",
    "feature:search:api": "com.google.samples.apps.nowinandroid.feature.search.api.navigation.SearchNavKey",
}


def module_path(module):
    return Path(*module.split(":"))


def file_hashes(paths):
    return {relative(path): sha(path) for path in sorted(set(map(Path, paths)))}


def tree_hashes(directory, suffix=None):
    directory = Path(directory)
    return {path.relative_to(directory).as_posix(): sha(path) for path in sorted(directory.rglob("*"))
            if path.is_file() and ".korrust" not in path.parts and (suffix is None or path.suffix == suffix)}


def source_abi(output):
    lines = [line.strip() for line in output.splitlines() if line.strip() and not line.startswith("Compiled from")]
    if not lines or not lines[-1] == "}":
        raise AssertionError("Unexpected javap ABI output")
    members, index = [], 1
    while index < len(lines) - 1:
        signature = lines[index]
        if index + 1 >= len(lines) or not lines[index + 1].startswith("descriptor: "):
            raise AssertionError("Unexpected javap member descriptor")
        descriptor = lines[index + 1]
        members.append((signature, descriptor))
        index += 2
    return lines[0], sorted(members)


def check_recorded_hashes(hashes):
    for path, expected in hashes.items():
        if sha(ROOT / path) != expected:
            raise AssertionError("Frozen input changed: " + path)


def parse_observations(text):
    observations = [json.loads(line) for line in text.splitlines() if line.strip()]
    if not observations or len({item["id"] for item in observations}) != len(observations):
        raise AssertionError("Missing or duplicate runtime observations")
    if any(item.get("outcome") != "pass" for item in observations):
        raise AssertionError("Runtime observation failed")
    return observations


class Harness:
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-nowinandroid-api-serialization"
        self.work = ROOT / "target/checks" / self.run_id
        self.evidence = ROOT / "reports/correctness" / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix(".json")
        self.report = {
            "schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running",
            "compiler": {}, "reference_compiler": {}, "inputs": {}, "commands": [], "evidence": [], "cases": [],
            "environment": {"os": platform.platform(), "machine": platform.machine()},
            "selection": "Complete unchanged pinned Now in Android feature:bookmarks:api and feature:search:api modules, real Gradle/AGP builds and packaged release AARs. Selected production Kotlin exclusively Korrust; ordinary dependency/build Kotlin uses official compilers. Bounded empty @Serializable singleton objects with marker interfaces; no general serialization-plugin, full-app, Android-device runtime or performance claim.",
        }
        self.java, self.javac, self.javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([self.java, self.javac, self.javap]):
            raise RuntimeError("JDK java, javac and javap required")

    def case(self, name, **details):
        self.report["cases"].append({"id": name, "outcome": "pass", **details})

    def archive(self, path, name):
        destination = self.evidence / name
        shutil.copy2(path, destination)
        self.report["evidence"].append(relative(destination))
        return destination

    def run(self, label, argv, allowed=(0,), timeout=120, empty=False):
        print(label, flush=True)
        commands = self.report["commands"]
        entry = {"label": label, "argv": list(map(str, argv)), "cwd": ".", "timeout_seconds": timeout}
        index = len(commands)
        commands.append(entry)
        try:
            result = subprocess.run(entry["argv"], cwd=ROOT, capture_output=True, text=True, timeout=timeout)
            entry["exit_code"] = result.returncode
            stdout, stderr = result.stdout, result.stderr
        except subprocess.TimeoutExpired as error:
            entry["timed_out"] = True
            stdout, stderr = error.stdout or b"", error.stderr or b""
            stdout = stdout.decode(errors="replace") if isinstance(stdout, bytes) else stdout
            stderr = stderr.decode(errors="replace") if isinstance(stderr, bytes) else stderr
            result = None
        for stream, payload in [("stdout", stdout), ("stderr", stderr)]:
            path = self.evidence / f"{index:03d}-{stream}.txt"
            path.write_text(payload)
            entry[stream] = relative(path)
            self.report["evidence"].append(relative(path))
        if result is None:
            raise TimeoutError(label + " timed out")
        if result.returncode not in allowed:
            raise AssertionError(f"{label} exited {result.returncode}: {stderr[-3000:]}")
        if empty and (stdout.strip() or stderr.strip()):
            raise AssertionError(label + " emitted unexpected diagnostics")
        return result

    def load_builds(self):
        self.builds, self.jars, self.class_hashes = {}, {}, {}
        self.variant_jars, self.variant_hashes = {}, {}
        manifests = {"official": self.args.official_json.resolve(), "candidate": self.args.candidate_json.resolve()}
        self.manifests = manifests
        for provider, path in manifests.items():
            build = json.loads(path.read_text())
            if build["status"] != "passed" or build["returncode"] != 0 or build["production_kotlin_fallback"]:
                raise AssertionError("Selected Gradle build did not pass without fallback: " + provider)
            if sorted(build["selected_modules"]) != MODULES:
                raise AssertionError("Selected Gradle module inventory mismatch")
            if not build["sources_unchanged"] or build["source_files_sha256"] != build["source_files_sha256_after_build"]:
                raise AssertionError("Gradle source hashes changed")
            if build["tool_sha256"] != sha(ROOT / "tools/build_nowinandroid.py"):
                raise AssertionError("Build adapter changed after build")
            adapter = path.parent / "adapter.init.gradle"
            if sha(adapter) != build["adapter_sha256"]:
                raise AssertionError("Gradle adapter changed after build")
            self.builds[provider] = build
            self.archive(path, provider + "-build.json")
            self.archive(adapter, provider + "-adapter.init.gradle")
            self.archive(build["log"], provider + "-gradle.log")
            self.report["inputs"][provider + "_build"] = {"path": relative(path), "sha256": sha(path), "command": build["command"]}
            self.case("gradle/" + provider, scope="Complete selected API module real Gradle/AGP release AAR builds; no selected production Kotlin fallback.")
        official, candidate = self.builds["official"], self.builds["candidate"]
        if sha(candidate["compiler"]) != candidate["compiler_sha256"]:
            raise AssertionError("Candidate binary changed since Gradle build")
        self.compiler = candidate["compiler"]
        compiler_paths = official["resolved_kotlin_compiler_classpath"]
        compiler_hashes = official["resolved_kotlin_compiler_files_sha256"]
        check_recorded_hashes(compiler_hashes)
        if set(compiler_paths) != set(compiler_hashes):
            raise AssertionError("Incomplete resolved reference compiler provenance")
        version = official["resolved_kotlin_compiler_version"]
        version_check = official["resolved_kotlin_compiler_version_check"]
        if version_check["returncode"] != 0 or "kotlinc-jvm " + version + " " not in version_check.get("stdout", "") + version_check.get("stderr", ""):
            raise AssertionError("Resolved Gradle compiler identity check failed")
        self.reference = [self.java, "-cp", classpath(*compiler_paths), "org.jetbrains.kotlin.cli.jvm.K2JVMCompiler"]
        resolved = self.run("Resolved official compiler version", [*self.reference, "-version"])
        if "kotlinc-jvm " + version + " " not in resolved.stdout + resolved.stderr:
            raise AssertionError("Consumer compiler differs from actual Gradle compiler")
        plugins = official["resolved_serialization_plugin_classpath"]
        plugin_hashes = official["resolved_serialization_plugin_files_sha256"]
        if not plugins or set(plugins) != set(plugin_hashes):
            raise AssertionError("Missing resolved serialization compiler plugin")
        check_recorded_hashes(plugin_hashes)
        for path in plugins:
            if version not in Path(path).name or "serialization" not in Path(path).name:
                raise AssertionError("Serialization plugin does not match resolved compiler")
        if candidate["resolved_kotlin_compiler_files_sha256"] != compiler_hashes or candidate["resolved_serialization_plugin_files_sha256"] != plugin_hashes:
            raise AssertionError("Selected builds resolved different compiler/plugin inputs")
        self.plugin_options = ["-Xplugin=" + str(path) for path in plugins]
        self.report["reference_compiler"] = {"version": version, "source": "actual Gradle resolved compiler, independent of repository historical pin", "compiler_files_sha256": compiler_hashes, "serialization_plugin_files_sha256": plugin_hashes}
        owned = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            owned.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
        self.frozen = file_hashes(owned)
        self.report["compiler"] = {"version": self.run("Korrust version", [self.compiler, "--version"]).stdout.strip(), "binary_sha256": candidate["compiler_sha256"], "source_files_sha256": self.frozen, "kotlin_metadata": True, "incremental_compilation": False}
        self.report["environment"]["java"] = self.run("Java version", [self.java, "-version"]).stderr.strip()
        self.check_sources()
        self.dependencies, self.variant_dependencies = {}, {}
        dependency_hashes = {}
        for provider, build in self.builds.items():
            self.jars[provider], self.class_hashes[provider] = {}, {}
            self.variant_jars[provider], self.variant_hashes[provider] = {}, {}
            self.variant_dependencies[provider] = {variant: set() for variant in VARIANTS}
            for module in MODULES:
                info = build["modules"][module]
                name = module.replace(":", "-")
                if sorted(info["variants"]) != VARIANTS:
                    raise AssertionError("Missing selected Gradle build variants")
                self.variant_jars[provider][module], self.variant_hashes[provider][module] = {}, {}
                for variant in VARIANTS:
                    selected = info["variants"][variant]
                    aar = self.archive(selected["aar"], provider + "-" + name + "-" + variant + ".aar")
                    with zipfile.ZipFile(aar) as package:
                        if "AndroidManifest.xml" not in package.namelist():
                            raise AssertionError("AAR manifest missing")
                        payload = package.read("classes.jar")
                    jar = self.work / (provider + "-" + name + "-" + variant + ".jar")
                    jar.write_bytes(payload)
                    self.variant_jars[provider][module][variant] = jar
                    with zipfile.ZipFile(io.BytesIO(payload)) as package:
                        hashes = {entry: hashlib.sha256(package.read(entry)).hexdigest() for entry in sorted(package.namelist()) if entry.endswith(".class")}
                        majors = sorted({int.from_bytes(package.read(entry)[6:8], "big") for entry in hashes})
                        mappings = {entry: hashlib.sha256(package.read(entry)).hexdigest() for entry in package.namelist() if entry.endswith(".kotlin_module")}
                    self.variant_hashes[provider][module][variant] = hashes
                    self.report["inputs"][provider + "_" + name + "_" + variant + "_aar"] = {"path": relative(aar), "sha256": sha(aar), "classes_jar_sha256": sha(jar), "class_files_sha256": hashes, "classfile_major_versions": majors, "module_mappings_sha256": mappings, "options": selected["options"]}
                    self.variant_dependencies[provider][variant].update(selected["classpath"])
                self.jars[provider][module] = self.variant_jars[provider][module]["demoRelease"]
                self.class_hashes[provider][module] = self.variant_hashes[provider][module]["demoRelease"]
            self.variant_dependencies[provider] = {variant: sorted(paths) for variant, paths in self.variant_dependencies[provider].items()}
            dependency_hashes[provider] = {}
            for variant, paths in self.variant_dependencies[provider].items():
                hashes = {}
                for path in paths:
                    try:
                        normalized = "selected_artifacts/" + Path(path).relative_to(manifests[provider].parent).as_posix()
                    except ValueError:
                        normalized = path
                    hashes[normalized] = sha(path)
                dependency_hashes[provider][variant] = hashes
        if dependency_hashes["official"] != dependency_hashes["candidate"]:
            raise AssertionError("Actual external compiler/D8 dependency inputs differ")
        self.report["inputs"]["actual_variant_compilation_dependencies_sha256"] = dependency_hashes["official"]
        core_jars = [Path(path) for path in self.variant_dependencies["official"]["demoRelease"] if Path(path).name.startswith("kotlinx-serialization-core-jvm-")]
        if len(core_jars) != 1:
            raise AssertionError("Missing/ambiguous actual serialization runtime")
        core = core_jars[0]
        runtime_version = core.parent.parent.name
        json_jars = sorted((core.parents[3] / "kotlinx-serialization-json-jvm" / runtime_version).glob("*/*.jar"))
        if len(json_jars) != 1:
            raise AssertionError("JSON validation runtime matching resolved serialization-core version required")
        self.json_jar = json_jars[0]
        self.report["inputs"]["json_validation_runtime"] = {"version": runtime_version, "path": str(self.json_jar), "sha256": sha(self.json_jar), "scope": "Supplemental JSON test runtime with exactly the actual Gradle resolved serialization-core version; selected API modules depend on core only."}
        self.dependencies = {provider: [*variants["demoRelease"], str(self.json_jar)] for provider, variants in self.variant_dependencies.items()}
        options = official["modules"][MODULES[0]]["options"]
        self.options = ["-no-stdlib", "-no-reflect", "-jvm-target", str(options["kotlin_jvm_target"])]
        for field, switch in [("language_version", "-language-version"), ("api_version", "-api-version")]:
            if options.get(field):
                self.options += [switch, str(options[field])]
        self.report["compiler"]["jvm_target_scope"] = "Actual Gradle Kotlin target " + str(options["kotlin_jvm_target"]) + "; Korrust has no target option and emits classfile49/52, compatible JVM verification/D8 subset only. Classfile target-setting equivalence is not claimed."

    def check_sources(self):
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["nowinandroid"]
        expected, expected_kotlin, expected_java = {}, {}, {}
        for module in MODULES:
            prefix = module_path(module)
            folder = SNAPSHOT / prefix / "src"
            paths = sorted([path for path in folder.rglob("*") if path.is_file()] + [SNAPSHOT / prefix / "build.gradle.kts"])
            production = [path for path in paths if path.suffix in [".kt", ".java"] and path.is_relative_to(folder / "main")]
            expected_kotlin[module] = [path.relative_to(SNAPSHOT).as_posix() for path in production if path.suffix == ".kt"]
            expected_java[module] = [path.relative_to(SNAPSHOT).as_posix() for path in production if path.suffix == ".java"]
            for path in paths:
                name = path.relative_to(SNAPSHOT).as_posix()
                checksum = sha(path)
                if checksum != inventory[name]:
                    raise AssertionError("Preserved NIA snapshot changed: " + name)
                expected[name] = checksum
                for build in self.builds.values():
                    if sha(Path(build["work_dir"]) / name) != checksum:
                        raise AssertionError("Disposable Gradle source differs from snapshot")
            for build in self.builds.values():
                info = build["modules"][module]
                per_module = {name: checksum for name, checksum in expected.items() if name.startswith(prefix.as_posix() + "/")}
                if info["production_kotlin_sources"] != expected_kotlin[module] or info["production_java_sources"] != expected_java[module]:
                    raise AssertionError("Module source inventory mismatch")
                if info["source_files_sha256"] != per_module or info["source_files_sha256_after_build"] != per_module or not info["sources_unchanged"]:
                    raise AssertionError("Module unchanged source provenance mismatch")
        if sum(map(len, expected_kotlin.values())) != 2 or any(expected_java.values()):
            raise AssertionError("Unexpected selected original production source count")
        for build in self.builds.values():
            if build["source_files_sha256"] != expected or build["source_files_sha256_after_build"] != expected:
                raise AssertionError("Complete selected source manifest mismatch")
        self.source_hashes, self.expected_kotlin = expected, expected_kotlin
        self.report["inputs"].update({"nowinandroid_commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["nowinandroid"]["commit"], "source_files_sha256": expected, "production_kotlin_sources": expected_kotlin, "production_java_sources": expected_java})
        module_files = {}
        for module in MODULES:
            for path in sorted((SNAPSHOT / module_path(module)).rglob("*")):
                if not path.is_file():
                    continue
                name = path.relative_to(SNAPSHOT).as_posix()
                checksum = sha(path)
                if checksum != inventory[name] or any(sha(Path(build["work_dir"]) / name) != checksum for build in self.builds.values()):
                    raise AssertionError("Complete module file differs from pinned source: " + name)
                module_files[name] = checksum
                self.archive(path, "source-" + name.replace("/", "-"))
        self.report["inputs"]["complete_module_files_sha256"] = module_files
        self.case("unchanged_sources", kotlin_sources=2, java_sources=0, scope="All selected production/test sources match pinned upstream inventory and both disposable Gradle copies.")

    def traces(self):
        candidate = self.builds["candidate"]
        for module in MODULES:
            trace_root = self.manifests["candidate"].parent / "modules" / module_path(module)
            for variant in VARIANTS:
                task = "compile" + variant[0].upper() + variant[1:] + "Kotlin"
                trace_path = trace_root / (task + ".json")
                request_path = trace_root / (task + "-request.json")
                prefix = module.replace(":", "-") + "-" + task
                self.archive(trace_path, prefix + ".json")
                self.archive(request_path, prefix + "-request.json")
                trace, request = json.loads(trace_path.read_text()), json.loads(request_path.read_text())
                if trace["fallback"] or trace["compiler_sha256"] != candidate["compiler_sha256"] or trace["task"] != ":" + module + ":" + task:
                    raise AssertionError("Candidate trace task/compiler/fallback mismatch")
                invocations = trace["invocations"]
                traced_paths = [item["path"] for item in trace["classpath"]]
                if traced_paths != request["classpath"]:
                    raise AssertionError("Actual trace classpath differs from Gradle request")
                for dependency in trace["classpath"]:
                    path = Path(dependency["path"])
                    if dependency["kind"] != "jar" or sha(path) != dependency["sha256"]:
                        raise AssertionError("Actual trace dependency hash changed/unsupported")
                    directory = Path(dependency["extracted"])
                    with zipfile.ZipFile(path) as package:
                        expected_classes = {name: hashlib.sha256(package.read(name)).hexdigest() for name in package.namelist() if name.endswith(".class")}
                    if tree_hashes(directory, ".class") != expected_classes:
                        raise AssertionError("Actual extracted candidate classpath differs from resolved dependency")
                expected_cp = classpath(*[item["extracted"] for item in trace["classpath"]])
                invoked = [Path(item["source"]).relative_to(candidate["work_dir"]).as_posix() for item in invocations]
                if sorted(invoked) != self.expected_kotlin[module]:
                    raise AssertionError("Actual candidate invocation source inventory mismatch")
                stage = trace_root / "module-build/tmp/korrust" / task
                hashes = tree_hashes(stage, ".class")
                emitted = set()
                with zipfile.ZipFile(self.variant_jars["candidate"][module][variant]) as package:
                    for invocation in invocations:
                        source = Path(invocation["source"]).relative_to(candidate["work_dir"]).as_posix()
                        command = invocation["command"]
                        if invocation["returncode"] != 0 or invocation["source_sha256"] != self.source_hashes[source] or command[0] != candidate["compiler"] or command[1] != invocation["source"] or Path(command[command.index("-d") + 1]) != Path(request["stage"]):
                            raise AssertionError("Actual candidate invocation provenance mismatch")
                        if "-cp" not in command or command[command.index("-cp") + 1] != expected_cp:
                            raise AssertionError("Actual candidate invocation classpath differs from hashed Gradle inputs")
                        outputs = [Path(line.strip()).relative_to(request["stage"]).as_posix() for line in invocation["stdout"].splitlines() if line.strip().endswith(".class")]
                        if not outputs:
                            raise AssertionError("No candidate emitted classes recorded")
                        for output in outputs:
                            emitted.add(output)
                            payload = (stage / output).read_bytes()
                            destination = Path(candidate["modules"][module]["variants"][variant]["kotlin_classes"]) / output
                            if payload != destination.read_bytes() or payload != package.read(output):
                                raise AssertionError("Staged candidate class bytes differ from Gradle/AAR")
                if emitted != set(hashes):
                    raise AssertionError("Candidate stage includes untraced classes")
                if hashes != self.variant_hashes["candidate"][module][variant]:
                    raise AssertionError("Packaged class inventory differs from exact candidate stage")
                self.case("candidate_trace/" + module + "/" + task, emitted_classes_sha256=hashes, scope="Exact unchanged selected source invokes frozen Korrust without fallback; every staged class byte matches Gradle and selected variant packaged AAR.")

    def compile_reference(self, label, source, cp, destination, allowed=(0,), plugins=True):
        return self.run(label, [*self.reference, source, "-cp", cp, *self.options, *(self.plugin_options if plugins else []), "-d", destination], allowed=allowed)

    def abi_and_java(self):
        shared = self.dependencies["official"]
        drivers = self.work / "java-callers"
        self.run("Compile Java callers once against official AARs", [self.javac, "-cp", classpath(*self.jars["official"].values(), *shared), "-d", drivers, FIXTURES / "SerializationCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])
        observations = []
        for provider in self.builds:
            for variant in VARIANTS:
                cp = classpath(drivers, *[self.variant_jars[provider][module][variant] for module in MODULES], *self.variant_dependencies[provider][variant])
                result = self.run("Run independent Java assertions " + provider + " " + variant, [self.java, "-Xverify:all", "-cp", cp, "SerializationCheck"])
                observations.append(json.loads(result.stdout))
                names = [name[:-6].replace("/", ".") for module in MODULES for name in self.variant_hashes[provider][module][variant]]
                self.run("Verify every packaged class " + provider + " " + variant, [self.java, "-Xverify:all", "-cp", cp, "ModuleClassVerification", *names])
                self.case("all_class_verification/" + provider + "/" + variant, classes=len(names))
                if provider == "candidate":
                    helpers = self.run("Verify declared serializer lowering helpers " + variant, [self.java, "-Xverify:all", "-cp", cp, "SerializationCheck", "--helpers"])
                    self.case("serializer_lowering_helpers/" + variant, runtime_checks=json.loads(helpers.stdout)["checks"], scope="Exact SerializerSupplier is synthetic final package-private Function0 with no captured fields, private constructor, singleton factory, erased invoke, fresh real serializers and synthetic Kotlin metadata kind3.")
        if any(item != observations[0] for item in observations):
            raise AssertionError("Java observations differ")
        self.case("java_callers", runtime_checks=observations[0]["checks"], scope="Java compiled once against official module set and run against both with -Xverify:all; independent singleton/serializer identity, marker interface, generic signature, metadata presence, descriptor assertions and concurrent first serializer access.")
        for module, name in NAV_CLASSES.items():
            original = name.replace(".", "/") + ".class"
            official_names = set(self.class_hashes["official"][module])
            candidate_names = set(self.class_hashes["candidate"][module])
            helpers = candidate_names - {original}
            if official_names != {original} or candidate_names != {original, original[:-6] + "$SerializerSupplier.class"}:
                raise AssertionError("Unexpected source/generated class inventory")
            for variant in VARIANTS:
                if set(self.variant_hashes["official"][module][variant]) != official_names or set(self.variant_hashes["candidate"][module][variant]) != candidate_names:
                    raise AssertionError("Variant class inventory differs")
                outputs = [self.run("Inspect public generic ABI " + provider + " " + variant + " " + name, [self.javap, "-public", "-s", "-classpath", self.variant_jars[provider][module][variant], name]).stdout for provider in self.builds]
                if source_abi(outputs[0]) != source_abi(outputs[1]):
                    raise AssertionError("Source ABI/generic serializer signature differs: " + name)
                for provider in self.builds:
                    detail = self.run("Inspect metadata/annotations " + provider + " " + variant + " " + name, [self.javap, "-v", "-p", "-classpath", self.variant_jars[provider][module][variant], name]).stdout
                    for marker in ["kotlinx.serialization.Serializable", "kotlin.Metadata", "kotlinx/serialization/KSerializer", "serializer"]:
                        if marker not in detail:
                            raise AssertionError("Missing serialization ABI/metadata marker: " + marker)
            self.case("source_abi/" + module, official_classes=sorted(official_names), candidate_classes=sorted(candidate_names), generated_helpers=sorted(helpers), scope="All javap public descriptors/generic signatures compared with ordering normalized; exactly one declared SerializerSupplier lowering helper per singleton. Metadata/serialization annotations present and Kotlin consumer semantic validation follows. Private lowering and metadata bytes may differ.")

    def kotlin_consumers(self):
        outputs = []
        for provider in self.builds:
            for variant in VARIANTS:
                cp = classpath(*[self.variant_jars[provider][module][variant] for module in MODULES], *self.variant_dependencies[provider][variant], self.json_jar)
                destination = self.work / (provider + "-" + variant + "-consumer")
                self.compile_reference("Compile Kotlin serialization consumer " + provider + " " + variant, FIXTURES / "SerializationConsumer.kt", cp, destination)
                result = self.run("Run Kotlin serialization consumer " + provider + " " + variant, [self.java, "-Xverify:all", "-cp", classpath(destination, cp), "verification.serialization.SerializationConsumerKt"])
                outputs.append(parse_observations(result.stdout))
        if any(item != outputs[0] for item in outputs):
            raise AssertionError("Kotlin JSON/serializer/polymorphic observations differ")
        if len(outputs[0]) != 26:
            raise AssertionError("Unexpected independent consumer case count")
        for observation in outputs[0]:
            self.case("kotlin/" + observation["id"], providers=list(self.builds), variants=VARIANTS, scope="Actual Gradle resolved official Kotlin compiler with matching serialization plugin compiles all four variants from both outputs independently; runtime JVM verification all.")

    def synthetics(self):
        from build_nowinandroid import unpack_classpath
        dependencies = self.dependencies["official"]
        dirs, provenance = unpack_classpath(dependencies, self.work / "synthetic-classpath")
        self.report["inputs"]["synthetic_classpath"] = provenance
        candidate_cp = classpath(*dirs)
        official_cp = classpath(*dependencies)
        fixtures = ["EmptyRoute", "AliasedDestination", "QualifiedToken"]
        outputs = {}
        for provider in self.builds:
            destination = self.work / (provider + "-synthetics")
            for name in [*fixtures, "InternalSignal"]:
                source = FIXTURES / (name + ".kt")
                if provider == "official":
                    self.compile_reference("Compile official synthetic " + name, source, official_cp, destination)
                else:
                    self.run("Compile candidate synthetic " + name, [self.compiler, source, "-cp", candidate_cp, "-d", destination])
            consumer = self.work / (provider + "-synthetic-consumer")
            self.compile_reference("Compile synthetic Kotlin metadata consumer " + provider, FIXTURES / "SyntheticConsumer.kt", classpath(destination, official_cp), consumer)
            result = self.run("Run synthetic Kotlin metadata consumer " + provider, [self.java, "-Xverify:all", "-cp", classpath(consumer, destination, official_cp), "verification.serialization.SyntheticConsumerKt"])
            outputs[provider] = parse_observations(result.stdout)
            rejected = self.compile_reference("Reject external internal-object access " + provider, FIXTURES / "InternalConsumer.kt", classpath(destination, official_cp), self.work / (provider + "-internal-rejected"), allowed=(1,))
            if "internal" not in rejected.stderr.lower() or "error:" not in rejected.stderr:
                raise AssertionError("Internal serialization metadata visibility not enforced")
        if outputs["official"] != outputs["candidate"]:
            raise AssertionError("Synthetic metadata/runtime observations differ")
        self.case("synthetic_serializable_objects", runtime_checks=sum(item["checks"] for item in outputs["official"]), scope="Unrelated class/package names; empty braces; annotation and marker aliases; fully qualified annotation/interface; serializer direct/reified singleton identity; official consumers enforce internal object visibility.")
        for fixture in json.loads((FIXTURES / "rejections.json").read_text()):
            source = self.work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"] + "\n")
            self.archive(source, "rejection-" + fixture["id"] + ".kt")
            destination = self.work / ("rejected-" + fixture["id"])
            result = self.run("Reject unsupported serialization " + fixture["id"], [self.compiler, source, "-cp", candidate_cp, "-d", destination], allowed=(1,))
            if not result.stderr.strip() or tree_hashes(destination, ".class"):
                raise AssertionError("Unsupported serialization emitted classes or no diagnostic")
            self.report["cases"].append({"id": "rejection/" + fixture["id"], "outcome": "unsupported_language", "rejection_verified": True, "reason": fixture["reason"], "scope": "Explicit candidate rejection; not counted as a supported/passing Kotlin feature."})
        self.lifecycle(candidate_cp)

    def lifecycle(self, cp):
        source = self.work / "Lifecycle.kt"
        destination = self.work / "lifecycle-incremental"
        versions = [
            "package life\nimport kotlinx.serialization.Serializable\n@Serializable object Signal",
            "package life\nobject Signal",
            "package moved\nimport kotlinx.serialization.Serializable\n@Serializable object Renamed",
            "package moved\nobject Final",
        ]
        for index, text in enumerate(versions):
            source.write_text(text + "\n")
            self.archive(source, "lifecycle-" + str(index) + ".kt")
            clean = self.work / ("lifecycle-clean-" + str(index))
            for output in [destination, clean]:
                self.run("Compile singleton lifecycle " + str(index) + " " + output.name, [self.compiler, source, "-cp", cp, "-d", output])
            if tree_hashes(destination) != tree_hashes(clean):
                raise AssertionError("Serialized/plain/renamed singleton outputs differ from clean compilation")
        self.case("serialization_output_lifecycle", versions=len(versions), scope="Same source/output serialized→plain→renamed serialized→renamed plain; all class/package metadata bytes match clean compilation and obsolete generated helper outputs are removed. No incremental performance claim.")

    def d8(self):
        actual = self.builds["official"]["agp_d8_classpath"]
        if len(actual) != 1 or actual != self.builds["candidate"]["agp_d8_classpath"]:
            raise AssertionError("Actual AGP D8 dependency differs/missing")
        jar = Path(actual[0]).resolve()
        if self.args.d8_jar and self.args.d8_jar.resolve() != jar:
            raise AssertionError("D8 override differs from actual resolved AGP jar")
        version = self.run("Actual AGP D8 version", [self.java, "-cp", jar, "com.android.tools.r8.D8", "--version"])
        if not version.stdout.startswith("D8 ") or version.stderr.strip():
            raise AssertionError("Unexpected actual AGP D8 identity/diagnostics")
        self.report["inputs"]["agp_d8"] = {"jar": str(jar), "sha256": sha(jar), "version": version.stdout.strip()}
        androids = [path for path in self.variant_dependencies["official"]["demoRelease"] if Path(path).name == "android.jar"]
        if len(androids) != 1:
            raise AssertionError("Actual Android compilation library missing/ambiguous")
        android = androids[0]
        min_sdk = self.builds["official"]["min_sdk"]
        if min_sdk != self.builds["candidate"]["min_sdk"]:
            raise AssertionError("Resolved minimum Android SDK differs")
        for provider in self.builds:
            for variant in VARIANTS:
                output = self.work / (provider + "-" + variant + "-dex")
                output.mkdir()
                deps = [item for path in self.variant_dependencies[provider][variant] if path != android for item in ["--classpath", path]]
                self.run("Strict warning-free AGP D8 " + provider + " " + variant, [self.java, "-cp", jar, "com.android.tools.r8.D8", "--release", "--min-api", str(min_sdk), "--lib", android, *deps, "--output", output, *[self.variant_jars[provider][module][variant] for module in MODULES]], empty=True)
                dexes = sorted(output.glob("*.dex"))
                if not dexes:
                    raise AssertionError("No DEX output")
                hashes = {relative(self.archive(path, provider + "-" + variant + "-" + path.name)): sha(path) for path in dexes}
                self.case("agp_d8/" + provider + "/" + variant, min_api=min_sdk, dex_sha256=hashes, stdout_empty=True, stderr_empty=True, scope="Both selected packaged AAR class jars accepted together by real AGP embedded D8 with actual resolved dependency jars and Android library; no Android device execution.")

    def execute(self):
        try:
            self.load_builds()
            self.traces()
            self.abi_and_java()
            self.kotlin_consumers()
            self.synthetics()
            self.d8()
            check_recorded_hashes(self.frozen)
            if sha(self.compiler) != self.report["compiler"]["binary_sha256"]:
                raise AssertionError("Candidate binary changed during validation")
            self.case("frozen_inputs", files=len(self.frozen), scope="All owned compiler sources, tools and fixtures unchanged from start through completion; candidate binary unchanged from Gradle builds.")
            self.report["status"] = "completed"
        except Exception as error:
            self.report["status"] = "failed"
            self.report["cases"].append({"id": "harness_failure", "outcome": "timeout" if isinstance(error, TimeoutError) else "fail", "message": str(error)})
            print(str(error), file=sys.stderr)
        finally:
            self.report["counts"] = dict(Counter(case["outcome"] for case in self.report["cases"]))
            self.report["finished_at"] = datetime.now(timezone.utc).isoformat()
            self.report_path.write_text(json.dumps(self.report, indent=2) + "\n")
            print("Report: " + relative(self.report_path), flush=True)
        if self.report["status"] == "completed":
            self.write_features()
        return 0 if self.report["status"] == "completed" else 1

    def write_features(self):
        report = self.report
        feature = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        feature.update({"kind": "features", "status": "assessed", "commands": [["python3", "tools/check_nowinandroid.py"]], "evidence": [relative(self.report_path)], "features": [{
            "name": "nowinandroid_bookmarks_search_api_serializable_objects", "status": "supported",
            "scope": "Complete pinned unchanged feature:bookmarks:api and feature:search:api modules. Empty serializable singleton objects implementing real navigation3 NavKey: real Gradle AARs, exact selected compiler traces, public generic serializer ABI, metadata consumers with matched actual official compiler/plugin, direct/reified serializer discovery, JSON/polymorphic singleton identity and errors, Java callers/JVM verification and warning-free AGP D8.",
            "evidence_cases": [case["id"] for case in report["cases"] if case["outcome"] == "pass"],
            "restrictions": ["Bounded empty top-level public/internal serializable singleton objects and marker interfaces only; no general kotlinx.serialization compiler-plugin compatibility.", "Serialization arguments, custom names/serializers, data objects, serializable classes/nonempty objects, nesting and duplicate/unknown annotations are rejected explicitly.", "Nonselected dependencies/build logic use normal official compilers; no full-app build, Android device runtime, target-bytecode equivalence or incremental performance claim."]}]})
        (ROOT / "reports/features" / (self.run_id + ".json")).write_text(json.dumps(feature, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--official-json", type=Path, default=ROOT / "target/nowinandroid-api-artifacts/official/build.json")
    parser.add_argument("--candidate-json", type=Path, default=ROOT / "target/nowinandroid-api-artifacts/korrust/build.json")
    parser.add_argument("--d8-jar", type=Path)
    return Harness(parser.parse_args()).execute()


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
