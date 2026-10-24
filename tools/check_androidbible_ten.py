#!/usr/bin/env python3
import argparse
from collections import Counter
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
from check_androidbible_simple import emitted_outputs
from check_nowinandroid import check_recorded_hashes, file_hashes, parse_observations, source_abi, tree_hashes


FIXTURES = ROOT / "tests/fixtures/ten_features"
MODEL = "AlkitabModel/src/main/java/yuku/alkitab/model/SingleChapterVerses.kt"
TARGETS = {
    "AudioLogEntry": "Alkitab/src/main/java/yuku/alkitab/base/audio/AudioLogEntry.kt",
    "VersesDialogNormalVerses": "Alkitab/src/main/java/yuku/alkitab/base/dialog/VersesDialogNormalVerses.kt",
    "XrefDialogVerses": "Alkitab/src/main/java/yuku/alkitab/base/dialog/XrefDialogVerses.kt",
}
SYNTHETICS = ["LongOps", "TenOps", "PublicFields", "ScalarRecord", "ThirtyTwo", "Frames"]
FEATURES = {
    "long_scalars": ("Long parameters, results, signed decimal/hex/binary literals, expected-type Int literal coercion, MIN/MAX constants, arithmetic, bitwise operations, Int shift counts, comparisons, unary negation, toString and two-slot locals.", "No unsigned arithmetic, floating conversions or general mixed numeric coercion."),
    "public_constructor_val": ("Ordinary final classes with public/default read-only constructor properties, private backing fields, getters including is names, nullable String and exact string-list property metadata, mixed private/public constructor fields and nonnull checks.", "This suite tests read-only properties; mutable constructors and defaults have separate twenty-addition evidence. Custom accessors and general inheritance remain outside this scope."),
    "data_classes": ("Final data classes with 1..32 public val constructor properties of nonnull Int/Byte/Char/Boolean/Long/String: constructor/getters, components, copy, copy$default masks, equals/hashCode/toString and official Kotlin data/copy/default/destructuring metadata. Includes unchanged AudioLogEntry.", "This suite tests nonnull scalar/String data properties and copy defaults. Nullable/List/Float properties, body members and constructor defaults have separate twenty-addition evidence; arrays and interfaces remain outside this scope."),
    "nullable_references": ("Nullable reference annotations and null literals for selected String/reference and exact string-list types; parameters/results, comparisons with null, constructor fields and metadata nullability. Existing nonnull structural String equality remains available.", "Nullable primitives, member signatures and stable-local smart casts have separate twenty-addition evidence. General nullable equality and subtype inference remain outside this suite."),
    "safe_reference_calls": ("Null-safe invocation of existing supported methods that return references; receiver evaluates once, arguments skip for null, chained safe calls and nullable results preserve metadata.", "Primitive-result safe calls have separate twenty-addition evidence; this suite claims reference-result safe calls only."),
    "elvis": ("Reference Elvis expressions select the original nonnull left result or lazily evaluate the fallback; nested calls/list reads preserve evaluation order and nullable/nonnullable result types.", "Primitive boxing and abrupt Elvis have separate twenty-addition evidence; no general common-supertype inference is claimed here."),
    "string_lists": ("Exact read-only List<String> and List<String?> signatures, constructor fields, top-level parameters/results, size, indexed reads with String casts and element nullability metadata. Includes two unchanged dialog verse classes.", "Typed app/scalar Lists, factories and mutable lists have separate twenty-addition evidence. Generic declarations and general collection operations remain outside this scope."),
    "string_boolean_when": ("String and Boolean when subjects in expressions/statements, structural String matching, ordered comma conditions, lazy branches, once-only subject evaluation and explicit else. Long subjects receive separate scalar validation.", "Existing Int/Char when expressions are not counted as a new feature. No subject declarations, guards or exhaustiveness inference."),
    "subjectless_when": ("Subjectless statement and expression when forms with ordered Boolean conditions, short-circuit selection, lazy branch evaluation and explicit expression else.", "Stable-local smart casts have separate twenty-addition evidence; general block-expression branches remain outside this scope."),
    "braced_templates": ("Braced expressions in ordinary string templates, nested quoted literals, arithmetic/Elvis expressions, ordered side effects and UTF-16/NUL/surrogate preservation.", "No raw triple-quoted templates, escaped identifier interpolation or arbitrary block expressions."),
}


def syntax_observations(source):
    output = list(source)
    templates = []
    index = 0

    def blank(start, end):
        for position in range(start, end):
            if output[position] != "\n":
                output[position] = " "

    while index < len(source):
        start = index
        if source.startswith("//", index):
            end = source.find("\n", index)
            index = len(source) if end < 0 else end
            blank(start, index)
        elif source.startswith("/*", index):
            depth = 1
            index += 2
            while index < len(source) and depth:
                if source.startswith("/*", index):
                    depth += 1
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            blank(start, index)
        elif source[index] in "\"'":
            quote = source[index]
            triple = source.startswith('"""', index)
            delimiter = '"""' if triple else quote
            index += len(delimiter)
            while index < len(source):
                if not triple and source[index] == "\\":
                    index += 2
                    continue
                if source.startswith(delimiter, index):
                    index += len(delimiter)
                    break
                if quote == '"' and source.startswith("${", index):
                    templates.append(index)
                index += 1
            blank(start, min(index, len(source)))
        else:
            index += 1
    masked = "".join(output)
    patterns = {
        "long_scalars": r"\bLong\b|\b(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|[0-9][0-9_]*)[lL]\b",
        "data_classes": r"\bdata\s+class\b",
        "nullable_references": r"\b(?:String|[A-Z][A-Za-z0-9_.]*(?:<[^<>]*>)?)\?|\bnull\b",
        "safe_reference_calls": r"\?\.",
        "elvis": r"\?:",
        "string_lists": r"\bList\s*<\s*String\s*\??\s*>",
        "string_boolean_when": r"\bwhen\s*\(",
        "subjectless_when": r"\bwhen\s*\{",
    }
    matches = {name: [item.start() for item in re.finditer(pattern, masked)] for name, pattern in patterns.items()}
    constructors = []
    for match in re.finditer(r"\bclass\s+\w+\s*\(([^)]*)\)", masked):
        for parameter in match.group(1).split(","):
            if re.search(r"\bval\b", parameter) and not re.search(r"\b(?:private|protected|internal)\b", parameter):
                constructors.append(match.start())
    matches["public_constructor_val"] = constructors
    matches["braced_templates"] = templates
    return matches


class Harness(SimpleHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-androidbible-ten-features"
        self.work = ROOT / "target/checks" / self.run_id
        self.evidence = ROOT / "reports/correctness" / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix(".json")
        self.report = {
            "schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running",
            "compiler": {}, "reference_compiler": {}, "inputs": {}, "commands": [], "evidence": [], "cases": [],
            "environment": {"os": platform.platform(), "machine": platform.machine()},
            "selection": "Ten distinct bounded language additions; three unchanged AndroidBible Alkitab files (AudioLogEntry, VersesDialogNormalVerses, XrefDialogVerses) plus the real unchanged SingleChapterVerses dependency. Full selected class ABI, fixed Java bytecodes, official Kotlin metadata consumers, explicit unsupported boundaries and standalone actual AGP D8. Source-file evidence only; no complete Alkitab Gradle/module/app build, Android-device runtime, incremental timing or performance claim.",
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
        self.options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "androidbible_ten"]
        version = self.run("Pinned official compiler identity", [self.kotlinc, "-version"])
        if "kotlinc-jvm " + pin["version"] + " " not in version.stdout + version.stderr:
            raise AssertionError("Pinned reference compiler identity mismatch")
        owned = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            owned.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
        self.frozen = file_hashes(owned)
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
        self.run("Compile shared Java trace dependency", [self.javac, "-d", dependencies, FIXTURES / "Trace.java"])
        for outputs in self.outputs.values():
            outputs["java"] = dependencies
        self.invocations = []
        corpus = ROOT / "reports/correctness/2026-10-24T021954838656Z-androidbible-corpus.json"
        baseline = json.loads(corpus.read_text())
        observations = {name: {"files": [], "occurrences": 0, "examples": []} for name in FEATURES}
        for case in baseline["cases"]:
            if case["outcome"] != "rejected":
                continue
            path = ROOT / case["id"]
            source = path.read_text()
            if sha(path) != case["source_sha256"]:
                raise AssertionError("Source assessment input changed")
            for name, matches in syntax_observations(source).items():
                if matches:
                    entry = observations[name]
                    entry["files"].append(case["id"])
                    entry["occurrences"] += len(matches)
                    if len(entry["examples"]) < 3:
                        entry["examples"].append({"path": case["id"], "line": source.count("\n", 0, matches[0]) + 1})
        self.report["inputs"]["selection_assessment"] = {"baseline_report": relative(corpus), "baseline_sha256": sha(corpus), "selected_rejected_files": sum(case["outcome"] == "rejected" for case in baseline["cases"]), "method": "Lexical heuristics mask nested comments, quoted strings and character literals, recording braced interpolation openings separately. Constructor parsing, generic nesting and nested quoted template expressions are approximate. Nullable and safe-call observations include constructs beyond the supported subset. String/Boolean-when relevance counts all subject when syntax because resolving subject types needs full app dependencies; it does not assert every occurrence uses a newly supported subject. Counts overlap and are relevance evidence, not compilability/blocker/coverage percentages.", "observations": {name: {**entry, "file_count": len(entry["files"])} for name, entry in observations.items()}}

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
        for name in ["VersesDialogNormalVerses", "XrefDialogVerses"]:
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
        self.run("Compile Java callers once against official output", [self.javac, "-cp", classpath(*self.outputs["official"].values(), self.stdlib), "-d", drivers, FIXTURES / "TenCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])
        observations = []
        for provider, outputs in self.outputs.items():
            result = self.run("Run fixed Java callers " + provider, [self.java, "-Xverify:all", "-cp", classpath(drivers, *outputs.values(), self.stdlib), "TenCheck"])
            observations.append(parse_observations(result.stdout))
            names = [name[:-6].replace("/", ".") for output in outputs.values() for name in tree_hashes(output, ".class")]
            self.run("Verify every emitted class " + provider, [self.java, "-Xverify:all", "-cp", classpath(drivers, *outputs.values(), self.stdlib), "ModuleClassVerification", *names])
            self.case("all_class_verification/" + provider, classes=len(names))
        if observations[0] != observations[1] or {item["id"] for item in observations[0]} != set(FEATURES):
            raise AssertionError("Fixed Java observations differ or ten distinct feature observations missing")
        for item in observations[0]:
            self.case("java/" + item["id"], runtime_checks=item["checks"], scope="One Java caller compilation against official output; identical bytecodes run against each provider with -Xverify:all and independent expected results.")

    def consumers(self):
        observations, rejections = [], {}
        for provider, outputs in self.outputs.items():
            cp = classpath(*outputs.values(), self.stdlib)
            destination = self.work / (provider + "-consumer")
            self.run("Compile official Kotlin metadata consumer against " + provider, [self.kotlinc, FIXTURES / "TenConsumer.kt", *self.options, "-cp", cp, "-Xfriend-paths=" + str(outputs["app"]), "-d", destination])
            result = self.run("Run official Kotlin consumer " + provider, [self.java, "-Xverify:all", "-cp", classpath(destination, cp), "ten.consumer.TenConsumerKt"])
            observations.append(parse_observations(result.stdout))
            rejections[provider] = {}
            for name in ["InternalAccess", "MetadataRejections"]:
                result = self.run("Reject Kotlin metadata contract " + name + " " + provider, [self.kotlinc, FIXTURES / (name + ".kt"), *self.options, "-cp", cp, "-d", self.work / (provider + "-rejected-" + name)], allowed=(1,))
                diagnostics = [line.split("error:", 1)[1].strip() for line in result.stderr.splitlines() if "error:" in line]
                if len(diagnostics) < (4 if name == "InternalAccess" else 6):
                    raise AssertionError("Expected metadata rejection diagnostics missing: " + name)
                rejections[provider][name] = result.stderr
                self.case("metadata_rejection/" + provider + "/" + name, diagnostic_count=len(diagnostics), scope="Official Kotlin rejects external internal access, null nonnull arguments/copy, val writes, nullable String and nullable-list element mismatches. Complete diagnostics must match providers.")
        if observations[0] != observations[1] or rejections["official"] != rejections["candidate"]:
            raise AssertionError("Official Kotlin consumer results/metadata rejection diagnostics differ")
        for item in observations[0]:
            self.case("kotlin/" + item["id"], scope="Pinned official Kotlin compiles named constructors, property/list/nullability, data destructuring and generated default/named copy against each provider, runs with -Xverify:all and independent assertions.")

    def boundaries(self):
        for case in json.loads((FIXTURES / "rejections.json").read_text()):
            source = self.work / ("Boundary_" + case["id"] + ".kt")
            source.write_text(case["source"] + "\n")
            for provider in self.outputs:
                destination = self.work / (provider + "-boundary-" + case["id"])
                result = self.compile(provider, source, destination, [self.outputs[provider]["java"]], allowed=(1,) if provider == "candidate" or case.get("invalid") else (0,))
                if result.returncode and ("error:" not in result.stderr or tree_hashes(destination, ".class")):
                    raise AssertionError("Missing explicit boundary rejection or output emitted")
                marker = case["candidate_diagnostic"] if provider == "candidate" else case.get("official_diagnostic")
                semantic_diagnostics = "\n".join(line.split("error:", 1)[1] for line in result.stderr.splitlines() if "error:" in line)
                if marker and marker.lower() not in semantic_diagnostics.lower():
                    raise AssertionError("Expected semantic boundary diagnostic missing: " + provider + "/" + case["id"])
            self.report["cases"].append({"id": "boundary/" + case["id"], "outcome": "pass" if case.get("invalid") else "unsupported_language", "rejection_verified": True, "scope": case["scope"], "source_sha256": sha(source), "candidate_diagnostic": case["candidate_diagnostic"], "official_diagnostic": case.get("official_diagnostic")})

    def dex(self):
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

    def features(self):
        report = self.report
        feature = {key: report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        feature.update({"kind": "features", "status": "assessed", "commands": [["python3", "tools/check_androidbible_ten.py"]], "evidence": [relative(self.report_path)], "features": []})
        metadata_cases = ["long_metadata", "public_val_metadata", "data_metadata_copy_destructure", "nullable_metadata", "safe_call_metadata", "elvis_metadata", "list_metadata_and_internal_friends", "when_subject_metadata", "subjectless_metadata", "template_metadata"]
        common = [case["id"] for case in report["cases"] if case["outcome"] == "pass" and case["id"].startswith(("source_abi/", "all_class_verification/", "standalone_d8/", "frozen_inputs"))]
        for (name, (scope, restriction)), metadata_case in zip(FEATURES.items(), metadata_cases):
            feature["features"].append({"name": name, "status": "supported", "scope": scope, "evidence_cases": ["java/" + name, "kotlin/" + metadata_case, *common], "restrictions": [restriction, "Three unchanged selected source files and standalone DEX evidence do not establish full Alkitab module/app compatibility, Android-device behavior or any performance improvement."]})
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
