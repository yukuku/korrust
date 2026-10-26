#!/usr/bin/env python3
import argparse
from datetime import datetime, timezone
import json
from pathlib import Path
import platform
import re
import shutil
import sys
import zipfile

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_androidbible_simple import Harness as SimpleHarness
from check_nowinandroid import file_hashes, parse_observations, source_abi, tree_hashes

FIXTURES = ROOT / "tests/fixtures/twenty_features"
MODEL = "Alkitab/src/main/java/yuku/alkitab/base/audio/AudioLogEntry.kt"
TARGETS = {
    "PlaybackState": "Alkitab/src/main/java/yuku/alkitab/base/audio/PlaybackState.kt",
    "SongPlaybackState": "Alkitab/src/main/java/yuku/alkitab/songs/SongPlaybackState.kt",
}
SYNTHETICS = ["Consts", "Implicit", "Captured", "MutableCtor", "Initialized", "CompanionInit", "Signatures", "MemberSurface", "MemberImplementation", "RichRecord", "Defaults", "DefaultCtor", "DefaultMethods", "Construction", "Mask32", "MaskCtor", "Ops"]
FEATURES = {
    "top_level_constants": ("Top-level literal const val fields for Int/Long/Float/Boolean/Char/String, ConstantValue ABI, private visibility and official Kotlin constant imports.", "No arbitrary constant-expression folding."),
    "java_static_calls": ("Declared public Java static methods with supported descriptors, real class initialization and exception propagation.", "No inherited, generic, annotated or vararg methods."),
    "java_fields": ("Declared public Java static and instance field reads and nonfinal writes, volatile access, once-only receivers and ConstantValue reads without class initialization.", "Final writes reject; no inherited or annotated fields."),
    "overload_resolution": ("Bounded Kotlin and Java overloads resolved by exact scalar/reference/null applicability and numeric literal preference.", "Ambiguous overloads and Java named arguments reject; no general generic/subtype overload inference."),
    "implicit_constructors": ("Ordinary final classes with omitted empty primary constructors and callable no-argument JVM constructors.", "No secondary constructors or general inheritance."),
    "nonproperty_constructor_parameters": ("Nonproperty constructor arguments available to property initialization without exposed fields or method capture.", "Access from methods/getters rejects."),
    "mutable_constructor_properties": ("Public/private constructor var backing fields, getters/setters and instance assignment.", "No custom mutable accessors."),
    "initialized_instance_properties": ("Typed instance val/var property initialization after constructor fields in declaration order, accessors and mutation.", "No initializer blocks or delegated/lateinit properties."),
    "source_construction_named_arguments": ("Source-class construction, same-package dependency resolution and named arguments evaluated once in source order.", "Concrete supported constructors only."),
    "function_constructor_defaults": ("Top-level function, instance-method and constructor defaults, named omission, prior-parameter references, declaration-order default evaluation, private defaults, synthetic default bridges, no-arg all-default constructors and 32-bit masks with wide slots.", "At most 32 parameters for one default mask; no arbitrary default expression forms."),
    "companion_initialization": ("Companion properties initialized once in declaration order, outer static backing fields, companion getters and source-class construction.", "No general companion methods or init blocks."),
    "nullable_list_member_signatures": ("Nullable/reference/List class and abstract-interface method signatures, class getters and constructors with generic/nullability metadata.", "Concrete supported types; no generic declarations."),
    "nullable_primitive_boxing": ("Nullable Int/Long/Byte/Char/Boolean/Float signatures, primitive-to-nullable boxing and null preservation across Java/Kotlin boundaries.", "No implicit nullable-to-primitive conversion."),
    "primitive_safe_calls": ("Safe calls returning boxed primitives and primitive Elvis fallback, skipped arguments and once-only receiver evaluation.", "No arbitrary receiver/property forms."),
    "stable_local_smartcasts": ("Null-check refinement for stable parameters/locals, early exits and nullable local aliases.", "Only stable vals/parameters refine; mutable locals, unstable fields, lambdas and general type smart casts remain outside scope."),
    "abrupt_elvis": ("Block-body Elvis returns and Elvis throws with terminal control flow, lazy exceptional effects and verified JVM stack behavior.", "Expression-body returns are valid pinned Kotlin but remain unsupported; no general Nothing/common-supertype inference."),
    "typed_lists": ("List of concrete app references and supported scalar/nullable elements, JVM boxing and read casts, exact generic signatures.", "No arbitrary generic types, collection operations or generic declarations."),
    "list_factories_mutation": ("emptyList, listOf and mutableListOf with explicit supported concrete types, indexed reads/writes and bounded add/set operations.", "Read-only List writes reject; no spread arguments or general collections."),
    "rich_data_properties": ("Data classes with nullable/List/Float primary properties, generated copy/components/equality/hash/text and official Kotlin consumers; NaN/signed-zero and structural list behavior.", "No inheritance, arbitrary arrays or general generated-member customization."),
    "data_body_members": ("Data-class ordinary body methods and computed getters, including unchanged PlaybackState.isActive and companion IDLE.", "Only primary properties participate in generated components/copy/equality/hash/text."),
}


def frozen_files():
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for folder in ["src", "tests", "tools"]:
        paths.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
    return file_hashes(paths)


class Harness(SimpleHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-androidbible-twenty-features"
        self.work = ROOT / "target/checks" / self.run_id
        self.evidence = ROOT / "reports/correctness" / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix(".json")
        self.report = {
            "schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running",
            "compiler": {}, "reference_compiler": {}, "inputs": {}, "commands": [], "evidence": [], "cases": [],
            "environment": {"os": platform.platform(), "machine": platform.machine()},
            "selection": "Twenty distinct bounded additions; unchanged PlaybackState and SongPlaybackState plus their real unchanged AudioLogEntry dependency. Full selected class ABI, fixed Java bytecodes, official Kotlin metadata consumers, explicit unsupported boundaries and standalone actual AGP D8. Source-file evidence only; no complete Alkitab Gradle/module/app build, Android-device runtime, incremental timing or performance claim.",
        }
        self.java, self.javac, self.javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([self.java, self.javac, self.javap]):
            raise RuntimeError("JDK tools required")

    def prepare(self):
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        self.kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        self.compiler = self.args.compiler.resolve()
        self.stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
        from build_androidbible import unpack_classpath
        self.stdlib_directories, provenance = unpack_classpath([self.stdlib], self.work / "stdlib-classpath")
        self.report["inputs"]["candidate_stdlib_classpath"] = provenance
        self.options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "androidbible_twenty"]
        version = self.run("Pinned official compiler identity", [self.kotlinc, "-version"])
        if "kotlinc-jvm " + pin["version"] + " " not in version.stdout + version.stderr:
            raise AssertionError("Pinned reference compiler identity mismatch")
        self.frozen = frozen_files()
        self.compiler_sha = sha(self.compiler)
        self.report["compiler"] = {"version": self.run("Korrust identity", [self.compiler, "--version"]).stdout.strip(), "binary_sha256": self.compiler_sha, "source_files_sha256": self.frozen, "kotlin_metadata": True, "incremental_compilation": False, "jvm_target_scope": "Official sources and consumers JVM21/language2.3/API2.3. Candidate target-setting and bytecode equivalence are not claimed; all selected classes run with -Xverify:all."}
        self.reference_hashes = file_hashes((ROOT / pin["path"] / "lib").glob("*.jar"))
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
        self.report["inputs"].update({"androidbible_commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["androidbible"]["commit"], "unchanged_sources_sha256": self.original_hashes, "real_dependency_source": MODEL})
        self.case("unchanged_sources", targets=len(TARGETS), real_dependency_sources=1, scope="Exact original source bytes match preserved benchmark manifest; compile snapshot sources directly without editing or stubs.")
        self.outputs = {provider: {kind: self.work / (provider + "-" + kind) for kind in ["model", "app", "synthetic"]} for provider in ["official", "candidate"]}
        dependencies = self.work / "java-dependencies"
        self.run("Compile shared Java trace dependency", [self.javac, "-d", dependencies, FIXTURES / "Trace.java", FIXTURES / "JavaApi.java"])
        for outputs in self.outputs.values():
            outputs["java"] = dependencies
        self.invocations = []

    def compile_sources(self):
        for provider, outputs in self.outputs.items():
            self.compile(provider, SNAPSHOT / MODEL, outputs["model"])
            for source in TARGETS.values():
                self.compile(provider, SNAPSHOT / source, outputs["app"], [outputs["model"]])
            for name in SYNTHETICS:
                dependencies = [outputs["java"], outputs["model"]]
                if outputs["synthetic"].exists():
                    dependencies.append(outputs["synthetic"])
                self.compile(provider, FIXTURES / (name + ".kt"), outputs["synthetic"], dependencies)
        self.report["inputs"]["candidate_invocations"] = self.invocations
        for name, source in TARGETS.items():
            self.case("unchanged_source/" + name, source=source, source_sha256=self.original_hashes[source])
        cross = self.work / "candidate-official-dependencies"
        for name in ["PlaybackState"]:
            self.compile("candidate", SNAPSHOT / TARGETS[name], cross, [self.outputs["official"]["model"]])
        cross_classes = tree_hashes(cross, ".class")
        if any(checksum != tree_hashes(self.outputs["candidate"]["app"], ".class").get(name) for name, checksum in cross_classes.items()):
            raise AssertionError("Candidate bytes differ when reading official dependency metadata")
        self.case("official_dependency_metadata", classes_sha256=cross_classes)
        for kind in ["model", "app", "synthetic"]:
            inventories = {provider: tree_hashes(outputs[kind], ".class") for provider, outputs in self.outputs.items()}
            if set(inventories["official"]) != set(inventories["candidate"]):
                raise AssertionError("Compiled class inventory differs: " + kind)
            for name in inventories["official"]:
                class_name = name[:-6].replace("/", ".")
                abis = [self.run("Inspect " + provider + " ABI " + class_name, [self.javap, "-p", "-s", "-classpath", self.outputs[provider][kind], class_name]).stdout for provider in self.outputs]
                if source_abi(abis[0]) != source_abi(abis[1]):
                    raise AssertionError("Class ABI differs: " + class_name)
                self.case("source_abi/" + class_name, scope="Every private/public javap member descriptor and generic signature compared; normalize member order only. Includes synthetic generated data defaults and exact List signatures.")
            self.report["inputs"][kind + "_class_files_sha256"] = inventories
            for provider, outputs in self.outputs.items():
                self.report["inputs"][provider + "_" + kind + "_module_metadata_sha256"] = {path.relative_to(outputs[kind]).as_posix(): sha(path) for path in outputs[kind].rglob("*.kotlin_module")}

    def compile(self, provider, source, destination, dependencies=(), allowed=(0,)):
        if provider == "candidate":
            return super().compile(provider, source, destination, dependencies, allowed)
        options = self.options
        self.options = options.copy()
        self.options[self.options.index("-module-name") + 1] += "_" + Path(source).stem.replace("-", "_")
        try:
            return super().compile(provider, source, destination, dependencies, allowed)
        finally:
            self.options = options

    def java_checks(self):
        drivers = self.work / "java-callers"
        self.run("Compile Java callers once against official output", [self.javac, "-cp", classpath(*self.outputs["official"].values(), self.stdlib), "-d", drivers, FIXTURES / "TwentyCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])
        observations = []
        for provider, outputs in self.outputs.items():
            result = self.run("Run fixed Java callers " + provider, [self.java, "-Xverify:all", "-cp", classpath(drivers, *outputs.values(), self.stdlib), "TwentyCheck"])
            observations.append(parse_observations(result.stdout))
            names = [name[:-6].replace("/", ".") for output in outputs.values() for name in tree_hashes(output, ".class")]
            self.run("Verify every emitted class " + provider, [self.java, "-Xverify:all", "-cp", classpath(drivers, *outputs.values(), self.stdlib), "ModuleClassVerification", *names])
            self.case("all_class_verification/" + provider, classes=len(names))
        if observations[0] != observations[1] or {item["id"] for item in observations[0]} != set(FEATURES):
            raise AssertionError("Fixed Java observations differ or twenty distinct feature observations missing")
        for item in observations[0]:
            self.case("java/" + item["id"], runtime_checks=item["checks"], scope="One Java caller compilation against official output; identical bytecodes run against each provider with -Xverify:all and independent expected results.")

    def consumers(self):
        observations, rejections = [], {}
        for provider, outputs in self.outputs.items():
            cp = classpath(*outputs.values(), self.stdlib)
            destination = self.work / (provider + "-consumer")
            self.run("Compile official Kotlin metadata consumer against " + provider, [self.kotlinc, FIXTURES / "TwentyConsumer.kt", *self.options, "-cp", cp, "-Xfriend-paths=" + str(outputs["app"]), "-d", destination])
            result = self.run("Run official Kotlin consumer " + provider, [self.java, "-Xverify:all", "-cp", classpath(destination, cp), "twenty.consumer.TwentyConsumerKt"])
            observations.append(parse_observations(result.stdout))
            rejections[provider] = {}
            for name in ["InternalAccess", "MetadataRejections"]:
                result = self.run("Reject Kotlin metadata contract " + name + " " + provider, [self.kotlinc, FIXTURES / (name + ".kt"), *self.options, "-cp", cp, "-d", self.work / (provider + "-rejected-" + name)], allowed=(1,))
                diagnostics = [line.split("error:", 1)[1].strip() for line in result.stderr.splitlines() if "error:" in line]
                if len(diagnostics) < (2 if name == "InternalAccess" else 6):
                    raise AssertionError("Expected metadata rejection diagnostics missing: " + name)
                rejections[provider][name] = result.stderr
                self.case("metadata_rejection/" + provider + "/" + name, diagnostic_count=len(diagnostics), scope="Official Kotlin rejects private constant/function/field access, nonproperty constructor parameter access, read-only data writes and nullable/list element contract violations. Semantic diagnostics must match providers.")
        if observations[0] != observations[1] or {item["id"] for item in observations[0]} != set(FEATURES) or rejections["official"] != rejections["candidate"]:
            raise AssertionError("Official Kotlin consumer results/metadata rejection diagnostics differ")
        for item in observations[0]:
            self.case("kotlin/" + item["id"], scope="Pinned official Kotlin compiles the same consumer against each provider using emitted metadata and default ABI, then runs with -Xverify:all and independent assertions.")

    def supported_regression(self, case, source):
        regression = case["supported_regression"]
        java_source = FIXTURES / regression["java_fixture"]
        outputs = {}
        for provider in self.outputs:
            destination = self.work / (provider + "-regression-" + case["id"])
            self.compile(provider, source, destination, [self.outputs[provider]["java"]])
            outputs[provider] = destination
        callers = self.work / ("regression-callers-" + case["id"])
        self.run("Compile fixed regression caller against official " + case["id"], [self.javac, "-cp", classpath(outputs["official"], self.stdlib), "-d", callers, java_source])
        fixed_callers = tree_hashes(callers, ".class")
        observations = []
        for provider, output in outputs.items():
            result = self.run("Run fixed regression assertions " + provider + "/" + case["id"], [self.java, "-Xverify:all", "-cp", classpath(callers, output, self.stdlib), regression["main_class"]])
            observation = json.loads(result.stdout)
            if observation.get("id") != case["id"] or type(observation.get("checks")) is not int or observation["checks"] <= 0:
                raise AssertionError("Missing independent regression assertions: " + case["id"])
            observations.append(observation)
        if observations[0] != observations[1] or tree_hashes(callers, ".class") != fixed_callers:
            raise AssertionError("Fixed regression caller or observations differ: " + case["id"])
        abis = [self.run("Inspect supported regression ABI " + provider + "/" + case["id"], [self.javap, "-protected", "-s", "-classpath", output, regression["facade"]]).stdout for provider, output in outputs.items()]
        if source_abi(abis[0]) != source_abi(abis[1]):
            raise AssertionError("Supported regression JVM ABI differs: " + case["id"])
        self.case("regressions/" + case["id"], source_sha256=sha(source), java_fixture_sha256=sha(java_source), fixed_java_callers_sha256=fixed_callers, runtime_checks=observations[0]["checks"], providers=list(outputs), jvm_verification="all", scope=case["scope"])

    def boundaries(self):
        for case in json.loads((FIXTURES / "rejections.json").read_text()):
            source = self.work / ("Boundary_" + re.sub(r"[^A-Za-z0-9_]", "_", case["id"]) + ".kt")
            source.write_text(case["source"] + "\n")
            if case.get("supported_regression"):
                self.supported_regression(case, source)
                continue
            for provider in self.outputs:
                destination = self.work / (provider + "-boundary-" + case["id"])
                result = self.compile(provider, source, destination, [self.outputs[provider]["java"]], allowed=(1,) if provider == "candidate" or case.get("invalid") else (0,))
                if result.returncode and ("error:" not in result.stderr or tree_hashes(destination, ".class")):
                    raise AssertionError("Missing explicit boundary rejection or output emitted")
                marker = case["candidate_diagnostic"] if provider == "candidate" else case.get("official_diagnostic")
                semantic_diagnostics = "\n".join(line.split("error:", 1)[1] for line in result.stderr.splitlines() if "error:" in line)
                if marker and marker.lower() not in semantic_diagnostics.lower():
                    raise AssertionError("Expected semantic boundary diagnostic missing: " + provider + "/" + case["id"])
            self.report["cases"].append({"id": "boundary/" + case["id"], "outcome": "pass" if case.get("invalid") else "unsupported_language", "rejection_verified": True, "scope": case["scope"], "source_sha256": sha(source), "candidate_diagnostic": case["candidate_diagnostic"], "official_diagnostic": case.get("official_diagnostic"), "official_outcome": "rejected" if case.get("invalid") else "accepted"})

    def dex_outputs(self):
        jar, android = self.args.d8_jar.resolve(), self.args.android_jar.resolve()
        version = self.run("Actual AGP embedded D8 identity", [self.java, "-cp", jar, "com.android.tools.r8.D8", "--version"])
        if not version.stdout.startswith("D8 ") or version.stderr.strip():
            raise AssertionError("Unexpected D8 identity/diagnostics")
        self.report["inputs"]["standalone_d8"] = {"jar": str(jar), "sha256": sha(jar), "version": version.stdout.strip(), "android_library": str(android), "android_library_sha256": sha(android), "stdlib_sha256": sha(self.stdlib), "scope": "Actual AGP9.3.2 embedded standalone D8 with Android37 library; no Gradle/AGP Alkitab app build or Android execution."}
        for provider, outputs in self.outputs.items():
            inputs = []
            for kind, output in outputs.items():
                archive = self.work / (provider + "-" + kind + ".jar")
                with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as package:
                    for path in sorted(output.rglob("*.class")):
                        package.write(path, path.relative_to(output).as_posix())
                inputs.append(archive)
                self.archive(archive, archive.name)
            destination = self.work / (provider + "-dex")
            destination.mkdir()
            self.run("Standalone strict warning-free D8 " + provider, [self.java, "-cp", jar, "com.android.tools.r8.D8", "--release", "--min-api", "26", "--lib", android, "--classpath", self.stdlib, "--output", destination, *inputs], empty=True)
            dexes = sorted(destination.glob("*.dex"))
            if not dexes:
                raise AssertionError("D8 emitted no DEX")
            self.case("standalone_d8/" + provider, dex_sha256={relative(self.archive(path, provider + "-" + path.name)): sha(path) for path in dexes}, min_api=26, stdout_empty=True, stderr_empty=True)

    def dex(self):
        self.dex_outputs()
        if frozen_files() != self.frozen:
            raise AssertionError("Frozen input contents or file inventory changed during execution")
        self.report["inputs"]["final_source_files_sha256"] = frozen_files()


    def features(self):
        report = self.report
        feature = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        feature.update({"kind": "features", "status": "assessed", "commands": [["python3", "tools/check_androidbible_twenty.py", "--compiler", str(self.compiler), "--d8-jar", str(self.args.d8_jar), "--android-jar", str(self.args.android_jar)]], "evidence": [relative(self.report_path)], "features": []})
        common = [case["id"] for case in report["cases"] if case["outcome"] == "pass" and case["id"].startswith(("source_abi/", "all_class_verification/", "standalone_d8/", "frozen_inputs"))]
        for name, (scope, restriction) in FEATURES.items():
            feature["features"].append({"name": name, "status": "supported", "scope": scope, "evidence_cases": ["java/" + name, "kotlin/" + name, *common], "restrictions": [restriction, "Two unchanged selected source files and standalone DEX establish neither a full app build nor Android execution or performance gains."]})
        (ROOT / "reports/features" / (self.run_id + ".json")).write_text(json.dumps(feature, indent=2) + "\n")


def preflight(args):
    harness = Harness(args)
    harness.evidence.rmdir()
    harness.evidence = harness.work / "preflight-evidence"
    harness.evidence.mkdir()
    pin = json.loads((ROOT / "references/compiler.json").read_text())
    harness.kotlinc = ROOT / pin["path"] / "bin/kotlinc"
    harness.compiler = args.compiler.resolve()
    harness.compiler_sha = sha(harness.compiler)
    harness.stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
    harness.options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "androidbible_twenty"]
    from build_androidbible import unpack_classpath
    harness.stdlib_directories, _ = unpack_classpath([harness.stdlib], harness.work / "stdlib")
    harness.invocations = []
    destination = ROOT / "target/twenty-preflight" / args.preflight
    if destination.exists():
        shutil.rmtree(destination)
    outputs = {kind: destination / kind for kind in ["model", "app", "synthetic", "java"]}
    harness.run("Preflight Java dependencies", [harness.javac, "-d", outputs["java"], FIXTURES / "Trace.java", FIXTURES / "JavaApi.java"])
    harness.compile(args.preflight, SNAPSHOT / MODEL, outputs["model"])
    for source in TARGETS.values():
        harness.compile(args.preflight, SNAPSHOT / source, outputs["app"], [outputs["model"]])
    for name in SYNTHETICS:
        dependencies = [outputs["java"], outputs["model"]]
        if outputs["synthetic"].exists():
            dependencies.append(outputs["synthetic"])
        harness.compile(args.preflight, FIXTURES / (name + ".kt"), outputs["synthetic"], dependencies)
    official = ROOT / "target/twenty-preflight/official"
    official_cp = classpath(*(official / kind for kind in outputs), harness.stdlib)
    drivers = ROOT / "target/twenty-preflight/java-callers"
    if args.preflight == "official":
        harness.run("Preflight fixed Java consumers", [harness.javac, "-cp", official_cp, "-d", drivers, FIXTURES / "TwentyCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])
    cp = classpath(*outputs.values(), harness.stdlib)
    harness.run("Preflight fixed Java runtime " + args.preflight, [harness.java, "-Xverify:all", "-cp", classpath(drivers, cp), "TwentyCheck"])
    consumer = destination / "consumer"
    harness.run("Preflight official Kotlin consumer " + args.preflight, [harness.kotlinc, FIXTURES / "TwentyConsumer.kt", *harness.options, "-cp", cp, "-d", consumer])
    harness.run("Preflight Kotlin runtime " + args.preflight, [harness.java, "-Xverify:all", "-cp", classpath(consumer, cp), "twenty.consumer.TwentyConsumerKt"])
    print("Preflight passed: " + args.preflight, flush=True)
    return 0


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", type=Path, default=ROOT / "target/release/korrust")
    parser.add_argument("--d8-jar", type=Path, default=Path("/workspace/korrust-setup/gradle-home/caches/modules-2/files-2.1/com.android.tools.build/builder/9.3.2/f9fdaab774037c649303d97015bfb4ffe3d6f1/builder-9.3.2.jar"))
    parser.add_argument("--android-jar", type=Path, default=Path("/workspace/korrust-setup/android-sdk/platforms/android-37.0/android.jar"))
    parser.add_argument("--preflight", choices=["official", "candidate"])
    args = parser.parse_args()
    return preflight(args) if args.preflight else Harness(args).execute()


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
