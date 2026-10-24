#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import zipfile

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_nowinandroid import check_recorded_hashes, file_hashes, parse_observations, source_abi, tree_hashes


FIXTURES = ROOT / "tests/fixtures/simple_androidbible"
MODEL = "AlkitabModel/src/main/java/yuku/alkitab/model/SingleChapterVerses.kt"
TARGETS = {
    "SongTextZoomable": "Alkitab/src/main/java/yuku/alkitab/songs/SongTextZoomable.kt",
    "VerseTextDecoder": "Alkitab/src/main/java/yuku/alkitab/base/storage/VerseTextDecoder.kt",
    "ReaderGestureActions": "Alkitab/src/main/java/yuku/alkitab/base/widget/ReaderGestureActions.kt",
    "InternalSingleChapterVerses": "Alkitab/src/main/java/yuku/alkitab/base/storage/InternalSingleChapterVerses.kt",
    "Yes1SingleChapterVerses": "Alkitab/src/main/java/yuku/alkitab/yes1/Yes1SingleChapterVerses.kt",
}
SYNTHETICS = ["MutableSurface", "MutableInherited", "ReadOnlyBase", "MutableOverride"]


def emitted_outputs(stdout, destination):
    classes = [Path(line.strip()) for line in stdout.splitlines() if line.strip().endswith(".class")]
    if not classes or len(set(classes)) != len(classes) or any(not path.is_relative_to(destination) or not path.is_file() for path in classes):
        raise AssertionError("Candidate outputs missing, repeated or outside selected destination")
    return classes


class Harness:
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-androidbible-simple-sources"
        self.work = ROOT / "target/checks" / self.run_id
        self.evidence = ROOT / "reports/correctness" / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix(".json")
        self.report = {
            "schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running",
            "compiler": {}, "reference_compiler": {}, "inputs": {}, "commands": [], "evidence": [], "cases": [],
            "environment": {"os": platform.platform(), "machine": platform.machine()},
            "selection": "Five unchanged AndroidBible Alkitab Kotlin source files: SongTextZoomable (new abstract mutable interface property support), VerseTextDecoder, ReaderGestureActions, InternalSingleChapterVerses and Yes1SingleChapterVerses (four coverage additions using existing language support). Real unchanged SingleChapterVerses dependency compiled independently by both compilers; no stand-in stubs. Source-file ABI, Java and official Kotlin consumers, synthetic mutable-property type/override boundaries and standalone AGP D8 validation. This is not a complete Alkitab Gradle/module/app build or Android runtime/performance evidence.",
        }
        self.java, self.javac, self.javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([self.java, self.javac, self.javap]):
            raise RuntimeError("JDK tools required")

    def case(self, name, **details):
        self.report["cases"].append({"id": name, "outcome": "pass", **details})

    def archive(self, path, name):
        output = self.evidence / name
        shutil.copy2(path, output)
        self.report["evidence"].append(relative(output))
        return output

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
        for stream, text in [("stdout", stdout), ("stderr", stderr)]:
            path = self.evidence / f"{index:03d}-{stream}.txt"
            path.write_text(text)
            entry[stream] = relative(path)
            self.report["evidence"].append(relative(path))
        if result is None:
            raise TimeoutError(label + " timed out")
        if result.returncode not in allowed:
            raise AssertionError(f"{label} exited {result.returncode}: {stderr[-3000:]}")
        if empty and (stdout.strip() or stderr.strip()):
            raise AssertionError(label + " emitted unexpected diagnostics")
        return result

    def prepare(self):
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        self.kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        self.compiler = self.args.compiler.resolve()
        self.stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
        from build_androidbible import unpack_classpath
        self.stdlib_directories, provenance = unpack_classpath([self.stdlib], self.work / "stdlib-classpath")
        self.report["inputs"]["candidate_stdlib_classpath"] = provenance
        self.options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "androidbible_simple"]
        version = self.run("Pinned official compiler identity", [self.kotlinc, "-version"])
        if "kotlinc-jvm " + pin["version"] + " " not in version.stdout + version.stderr:
            raise AssertionError("Pinned reference compiler identity mismatch")
        owned = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            owned.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
        self.frozen = file_hashes(owned)
        self.compiler_sha = sha(self.compiler)
        self.report["compiler"] = {"version": self.run("Korrust identity", [self.compiler, "--version"]).stdout.strip(), "binary_sha256": self.compiler_sha, "source_files_sha256": self.frozen, "kotlin_metadata": True, "incremental_compilation": False, "jvm_target_scope": "Official sources and consumers use JVM21/language2.3/API2.3. Korrust has no target-setting option and emits classfile49/52; verified source/ABI/runtime subset, not target-bytecode equivalence."}
        reference_libs = sorted((ROOT / pin["path"] / "lib").glob("*.jar"))
        self.reference_hashes = file_hashes(reference_libs)
        self.report["reference_compiler"] = {"version": pin["version"], "compiler_distribution_libs_sha256": self.reference_hashes, "options": self.options}
        self.report["environment"]["java"] = self.run("Java identity", [self.java, "-version"]).stderr.strip()
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        self.original_hashes = {}
        for name in [MODEL, *TARGETS.values()]:
            path = SNAPSHOT / name
            if sha(path) != inventory[name]:
                raise AssertionError("Original pinned snapshot changed: " + name)
            self.original_hashes[name] = sha(path)
            self.archive(path, "original-" + path.name)
        self.report["inputs"].update({"androidbible_commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["androidbible"]["commit"], "unchanged_sources_sha256": self.original_hashes, "new_language_target": TARGETS["SongTextZoomable"], "existing_language_coverage_additions": [value for key, value in TARGETS.items() if key != "SongTextZoomable"], "real_dependency_source": MODEL})
        self.case("unchanged_sources", targets=5, real_dependency_sources=1, scope="Exact original source bytes match preserved benchmark manifest; original snapshot is compiled directly and never edited.")
        self.outputs = {provider: {kind: self.work / (provider + "-" + kind) for kind in ["model", "app", "synthetic"]} for provider in ["official", "candidate"]}
        self.invocations = []

    def compile(self, provider, source, destination, dependencies=(), allowed=(0,)):
        if provider == "official":
            argv = [self.kotlinc, source, *self.options, "-cp", classpath(self.stdlib, *dependencies), "-d", destination]
        else:
            argv = [self.compiler, source, "-cp", classpath(*self.stdlib_directories, *dependencies), "-d", destination]
        result = self.run("Compile " + provider + " " + Path(source).name, argv, allowed=allowed)
        if provider == "candidate" and result.returncode == 0:
            classes = emitted_outputs(result.stdout, destination)
            self.invocations.append({"source": relative(source), "source_sha256": sha(source), "command": list(map(str, argv)), "compiler_sha256": self.compiler_sha, "fallback": False, "emitted_classes_sha256": file_hashes(classes)})
        return result

    def compile_sources(self):
        for provider, outputs in self.outputs.items():
            self.compile(provider, SNAPSHOT / MODEL, outputs["model"])
            for name in TARGETS.values():
                self.compile(provider, SNAPSHOT / name, outputs["app"], [outputs["model"]])
            for name in SYNTHETICS:
                self.compile(provider, FIXTURES / (name + ".kt"), outputs["synthetic"], [outputs["synthetic"]] if outputs["synthetic"].exists() else [])
        self.report["inputs"]["candidate_invocations"] = self.invocations
        for name in TARGETS:
            self.case("unchanged_source/" + name, source=TARGETS[name], source_sha256=self.original_hashes[TARGETS[name]], new_language_support=name == "SongTextZoomable")
        cross = self.work / "candidate-official-dependencies"
        for name in ["MutableInherited", "MutableOverride"]:
            self.compile("candidate", FIXTURES / (name + ".kt"), cross, [self.outputs["official"]["synthetic"]])
        for name in ["InternalSingleChapterVerses", "Yes1SingleChapterVerses"]:
            self.compile("candidate", SNAPSHOT / TARGETS[name], cross, [self.outputs["official"]["model"]])
        cross_classes = tree_hashes(cross, ".class")
        candidate_classes = {**tree_hashes(self.outputs["candidate"]["app"], ".class"), **tree_hashes(self.outputs["candidate"]["synthetic"], ".class")}
        if any(checksum != candidate_classes.get(name) for name, checksum in cross_classes.items()):
            raise AssertionError("Candidate bytes differ when reading official dependency metadata")
        self.case("official_dependency_metadata", classes_sha256=cross_classes, scope="Candidate inherited mutable interfaces/var-over-val and both unchanged chapter classes cross-compiled against official dependency metadata; every emitted class byte equals candidate-dependency compilation.")
        for kind in ["model", "app", "synthetic"]:
            inventories = {provider: tree_hashes(outputs[kind], ".class") for provider, outputs in self.outputs.items()}
            if set(inventories["official"]) != set(inventories["candidate"]):
                raise AssertionError("Compiled class inventory differs: " + kind)
            for name in inventories["official"]:
                class_name = name[:-6].replace("/", ".")
                abis = [self.run("Inspect " + provider + " ABI " + class_name, [self.javap, "-p", "-s", "-classpath", self.outputs[provider][kind], class_name]).stdout for provider in self.outputs]
                if source_abi(abis[0]) != source_abi(abis[1]):
                    raise AssertionError("Class ABI differs: " + class_name)
                self.case("source_abi/" + class_name, scope="All javap private/public descriptors and generic signatures compared, normalizing only member order; Kotlin consumers establish relevant metadata behavior separately.")
            self.report["inputs"][kind + "_class_files_sha256"] = inventories
            for provider, outputs in self.outputs.items():
                mappings = {path.relative_to(outputs[kind]).as_posix(): sha(path) for path in outputs[kind].rglob("*.kotlin_module")}
                self.report["inputs"][provider + "_" + kind + "_module_metadata_sha256"] = mappings

    def java_checks(self):
        drivers = self.work / "java-callers"
        official = self.outputs["official"]
        fixtures = [FIXTURES / "SimpleAndroidBibleCheck.java", ROOT / "tests/fixtures/InterfaceAssertions.java", ROOT / "tests/fixtures/ModelCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"]
        self.run("Compile Java callers once against official outputs", [self.javac, "-cp", classpath(*official.values(), self.stdlib), "-d", drivers, *fixtures])
        for name in ["SimpleAndroidBibleCheck", "ModelCheck"]:
            observations = []
            for provider, outputs in self.outputs.items():
                result = self.run("Run fixed Java " + name + " " + provider, [self.java, "-Xverify:all", "-cp", classpath(drivers, *outputs.values(), self.stdlib), name])
                observations.append(json.loads(result.stdout))
            if observations[0] != observations[1]:
                raise AssertionError("Fixed Java observations differ: " + name)
            self.case("java/" + name, runtime_checks=observations[0]["checks"], result_sha256=observations[0]["sha256"], scope="Same Java caller bytecodes compiled once against official class set run against both outputs with -Xverify:all; independent expected observations.")
        for provider, outputs in self.outputs.items():
            names = [name[:-6].replace("/", ".") for output in outputs.values() for name in tree_hashes(output, ".class")]
            self.run("Verify every emitted class " + provider, [self.java, "-Xverify:all", "-cp", classpath(drivers, *outputs.values(), self.stdlib), "ModuleClassVerification", *names])
            self.case("all_class_verification/" + provider, classes=len(names))

    def consumers(self):
        observations = []
        rejection_diagnostics = {}
        for provider, outputs in self.outputs.items():
            rejection_diagnostics[provider] = {}
            cp = classpath(*outputs.values(), self.stdlib)
            destination = self.work / (provider + "-consumer")
            self.run("Compile official Kotlin consumer against " + provider, [self.kotlinc, FIXTURES / "SimpleConsumer.kt", *self.options, "-cp", cp, "-Xfriend-paths=" + str(outputs["app"]), "-d", destination])
            result = self.run("Run Kotlin consumer " + provider, [self.java, "-Xverify:all", "-cp", classpath(destination, cp), "simple.consumer.SimpleConsumerKt"])
            observations.append(parse_observations(result.stdout))
            for name, marker, expected_errors in [("InternalAccess", "internal", 3), ("NullableArguments", "null", 3), ("MutableNullAssignment", "null", 5), ("ReadOnlySetter", "val", 1)]:
                result = self.run("Reject Kotlin metadata access " + name + " " + provider, [self.kotlinc, FIXTURES / (name + ".kt"), *self.options, "-cp", cp, "-d", self.work / (provider + "-rejected-" + name)], allowed=(1,))
                diagnostics = [line.split("error:", 1)[1].strip() for line in result.stderr.splitlines() if "error:" in line]
                if len(diagnostics) != expected_errors or marker not in "\n".join(diagnostics).lower():
                    raise AssertionError("Expected Kotlin metadata rejection missing: " + name)
                rejection_diagnostics[provider][name] = result.stderr
                self.case("metadata_rejection/" + provider + "/" + name, diagnostic_count=len(diagnostics), scope="Pinned official Kotlin validates external internal visibility, nonnull arguments/setters/array element types and val/var metadata against selected provider; every expected invalid statement diagnoses and complete diagnostics must match across providers.")
        if observations[0] != observations[1]:
            raise AssertionError("Kotlin metadata consumer observations differ")
        if rejection_diagnostics["official"] != rejection_diagnostics["candidate"]:
            raise AssertionError("Kotlin metadata rejection diagnostics differ")
        for observation in observations[0]:
            self.case("kotlin/" + observation["id"], providers=list(self.outputs), scope="Pinned official Kotlin compiles implementations, overrides, property reads/writes and real chapter consumers against both sets; each independently runs with JVM verification all.")

    def boundaries(self):
        for name in ["ValOverrideVar", "MismatchedMutableOverride"]:
            for provider, outputs in self.outputs.items():
                result = self.compile(provider, FIXTURES / (name + ".kt"), self.work / (provider + "-rejected-" + name), [outputs["synthetic"]], allowed=(1,))
                if "error:" not in result.stderr or tree_hashes(self.work / (provider + "-rejected-" + name), ".class"):
                    raise AssertionError("Missing override rejection diagnostic")
            self.case("invalid_override/" + name, scope="Both official and candidate reject invalid getter-only override of var or mismatched var type; no output accepted.")
        for provider, outputs in self.outputs.items():
            destination = self.work / (provider + "-class-readonly-rejected")
            result = self.compile(provider, FIXTURES / "ClassReadOnlyOverride.kt", destination, [outputs["app"]], allowed=(1,))
            marker = "setsongtextzoompercent" if provider == "candidate" else "var"
            if "error:" not in result.stderr or marker not in result.stderr.lower():
                raise AssertionError("Class getter-only var override did not reject for setter/override boundary")
            if tree_hashes(destination, ".class"):
                raise AssertionError("Invalid getter-only class override emitted classes")
        self.case("invalid_override/ClassReadOnlyOverride", scope="Both compilers reject a class providing only val/getter for the sole inherited mutable Int property; required setter/var contract is enforced.")
        source = FIXTURES / "DefaultMutableAccessor.kt"
        self.compile("official", source, self.work / "official-default-accessor")
        destination = self.work / "candidate-default-accessor"
        result = self.compile("candidate", source, destination, allowed=(1,))
        if "Mutable interface properties require abstract default accessors" not in result.stderr or tree_hashes(destination, ".class"):
            raise AssertionError("Unsupported default mutable accessor not explicitly rejected")
        self.report["cases"].append({"id": "unsupported/default_mutable_accessor", "outcome": "unsupported_language", "rejection_verified": True, "scope": "Official Kotlin accepts custom/default mutable getter+setter; bounded candidate implementation rejects explicitly without class output. This is not a supported feature/pass."})

    def dex(self):
        jar, android = self.args.d8_jar.resolve(), self.args.android_jar.resolve()
        version = self.run("Actual AGP embedded D8 identity", [self.java, "-cp", jar, "com.android.tools.r8.D8", "--version"])
        if not version.stdout.startswith("D8 ") or version.stderr.strip():
            raise AssertionError("Unexpected D8 identity/diagnostics")
        self.report["inputs"]["standalone_d8"] = {"jar": str(jar), "sha256": sha(jar), "version": version.stdout.strip(), "android_library": str(android), "android_library_sha256": sha(android), "stdlib_sha256": sha(self.stdlib), "scope": "Actual AGP9.3.2 embedded D8 executable, standalone source-file class jars with Android37 library. No Gradle/AGP Alkitab app build or Android device execution."}
        for provider, outputs in self.outputs.items():
            inputs = []
            for kind in ["model", "app", "synthetic"]:
                archive = self.work / (provider + "-" + kind + ".jar")
                with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as package:
                    for path in sorted(outputs[kind].rglob("*.class")):
                        package.write(path, path.relative_to(outputs[kind]).as_posix())
                inputs.append(archive)
                self.archive(archive, archive.name)
            destination = self.work / (provider + "-dex")
            destination.mkdir()
            self.run("Standalone strict warning-free D8 " + provider, [self.java, "-cp", jar, "com.android.tools.r8.D8", "--release", "--min-api", "26", "--lib", android, "--classpath", self.stdlib, "--output", destination, *inputs], empty=True)
            dexes = sorted(destination.glob("*.dex"))
            if not dexes:
                raise AssertionError("D8 emitted no DEX")
            hashes = {relative(self.archive(path, provider + "-" + path.name)): sha(path) for path in dexes}
            self.case("standalone_d8/" + provider, dex_sha256=hashes, min_api=26, stdout_empty=True, stderr_empty=True, scope="All five real selected source classes plus actual model dependency and synthetic interface fixtures accepted by standalone real AGP D8 without warnings; not an Alkitab AGP/module build.")

    def execute(self):
        try:
            self.prepare()
            self.compile_sources()
            self.java_checks()
            self.consumers()
            self.boundaries()
            self.dex()
            check_recorded_hashes(self.frozen)
            check_recorded_hashes(self.reference_hashes)
            if sha(self.compiler) != self.compiler_sha or any(sha(SNAPSHOT / name) != checksum for name, checksum in self.original_hashes.items()):
                raise AssertionError("Frozen candidate binary or original source changed")
            self.case("frozen_inputs", files=len(self.frozen), scope="All compiler source, test, fixture and tool hashes unchanged from start through completion; reference compiler distribution jars, candidate binary and pinned original app sources unchanged.")
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
            self.features()
        return 0 if self.report["status"] == "completed" else 1

    def features(self):
        report = self.report
        feature = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        feature.update({"kind": "features", "status": "assessed", "commands": [["python3", "tools/check_androidbible_simple.py"]], "evidence": [relative(self.report_path)], "features": [{
            "name": "abstract_mutable_interface_properties", "status": "supported", "scope": "Abstract nonnull mutable interface properties for existing Int, Byte, Char, Boolean, Float, String, IntArray, ByteArray, CharArray and Array<String> types; getter/setter JVM signatures and Kotlin property metadata; isReady/isTitle naming; nested/inherited var and var-over-val overrides. Unchanged AndroidBible SongTextZoomable establishes one new real-source language target.",
            "evidence_cases": [case["id"] for case in report["cases"] if case["outcome"] == "pass"],
            "restrictions": ["This suite covers abstract nonnull mutable interface properties. Concrete mutable class properties and source property assignment have separate twenty-addition evidence; custom/default mutable accessors and arbitrary generic declarations remain outside this scope.", "Getter-only override of var and mismatched mutable types are rejected. Consumer Kotlin implementation/assignment bytecode uses the pinned official compiler.", "Standalone source-file/DEX evidence only; no complete Alkitab module/app Gradle build, Android device runtime, JVM target-setting equivalence or performance claim."]
        }, {
            "name": "androidbible_simple_source_coverage", "status": "supported", "scope": "Four unchanged source coverage additions using existing support: VerseTextDecoder, ReaderGestureActions, InternalSingleChapterVerses and Yes1SingleChapterVerses. Real unchanged SingleChapterVerses compiled independently by each compiler as dependency. Java/Kotlin consumers cover interfaces, array/string contracts, internal visibility, chapter defaults, aliasing, null and index/overflow boundaries.",
            "evidence_cases": [case["id"] for case in report["cases"] if case["outcome"] == "pass"],
            "restrictions": ["These five selected Alkitab source files do not establish a complete Alkitab module/app build; no Android device or performance claim."]
        }]})
        (ROOT / "reports/features" / (self.run_id + ".json")).write_text(json.dumps(feature, indent=2) + "\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", type=Path, default=ROOT / "target/release/korrust")
    parser.add_argument("--d8-jar", type=Path, default=Path("/workspace/korrust-setup/gradle-home/caches/modules-2/files-2.1/com.android.tools.build/builder/9.3.2/f9fdaab774037c649303d97015bfb4ffe3d6f1/builder-9.3.2.jar"))
    parser.add_argument("--android-jar", type=Path, default=Path("/workspace/korrust-setup/android-sdk/platforms/android-37.0/android.jar"))
    return Harness(parser.parse_args()).execute()


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
