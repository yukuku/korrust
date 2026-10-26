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
from check_androidbible_simple import Harness as LoggedHarness
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / "tests/fixtures/library_interop"
APP_SOURCE = "Alkitab/src/main/java/yuku/alkitab/base/util/IntArrayListUtil.kt"
APP_JAVA = "AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java"
FEATURES = json.loads((FIXTURES / "cases.json").read_text())
CONSUMER_CALLS = {
    "jar_classpath": ('jarProbe()', '7'),
    "kotlin_metadata": ('metadataProbe()', '"metadata"'),
    "top_level_imports": ('importProbe()', '18'),
    "java_generic_signatures": ('javaGenericProbe()', '"signature"'),
    "type_variables_bounds": ('boundProbe()', '9'),
    "generic_inference": ('inferenceProbe()', '"inferred"'),
    "variance_projections": ('varianceProbe(listOf<String>("first"))', '"first"'),
    "annotations_platform": ('platformProbe(false)', '"platform"'),
    "overload_specificity": ('specificityProbe()', '"string"'),
    "compiled_extensions": ('extensionProbe()', '12'),
    "library_properties": ('propertyProbe()', '49'),
    "imported_named_defaults": ('defaultProbe()', '28'),
    "function_invocation": ('invocationProbe { it * 3 }', '27'),
    "capturing_lambdas": ('captureProbe(5)', '10'),
    "contextual_lambda_inference": ('lambdaInferenceProbe()', '8'),
    "callable_references": ('boundReferenceProbe(interop.lib.Counter(10))', '14'),
    "inherited_dispatch": ('inheritedProbe(interop.lib.DerivedSource())', '12'),
    "java_sam": ('samProbe(7)', '12'),
    "compiled_inline": ('inlineProbe()', '17'),
    "nonlocal_inline_returns": ('nonlocalProbe()', '37'),
}
FEATURE_BOUNDARIES = {
    "java_generic_signatures": ["wrong_generic_argument", "raw_java_classifier"],
    "type_variables_bounds": ["wrong_generic_bound", "generic_classifier_arity", "generic_classifier_bound", "primitive_source_type_bound"],
    "annotations_platform": ["annotated_nullable_dereference"],
    "overload_specificity": ["java_named_argument", "ambiguous_overload_specificity"],
    "library_properties": ["read_only_library_property", "internal_library_setter"],
    "imported_named_defaults": ["unknown_kotlin_named_argument", "missing_kotlin_argument"],
    "variance_projections": ["projected_list_write"],
    "contextual_lambda_inference": ["wrong_function_result"],
    "capturing_lambdas": ["nonlocal_ordinary_lambda"],
    "callable_references": ["unbound_reference_receiver"],
    "nonlocal_inline_returns": ["noinline_nonlocal_return", "crossinline_nonlocal_return"],
}
RESTRICTIONS = {
    "jar_classpath": ["Directory and ordinary stored/DEFLATE JAR entries with explicit classpath precedence; duplicate archive names and entries over 64 MiB reject. No Gradle/module dependency discovery or multi-release JAR selection claim."],
    "kotlin_metadata": ["Pinned Kotlin metadata major 2/minor through 3 only; supported package/class/function declarations with validated JVM signatures. Metadata 1.x and unsupported declaration flags reject; no universal Kotlin binary compatibility claim."],
    "top_level_imports": ["Explicit/wildcard imported package declarations and recorded JVM owners; no full Kotlin import-resolution compatibility claim."],
    "java_generic_signatures": ["Supported Java class/method Signature types, concrete substitutions and wildcards; generic enclosing-class signatures and object arrays beyond supported String arrays reject. Java Short/Double types remain unsupported."],
    "type_variables_bounds": ["Source generic functions allow ordered type parameters with one inline reference upper bound each. Primitive/Unit and boxed-primitive source bounds explicitly reject because their pinned JVM primitive erasure requires separate ABI lowering. Generic source classes/interfaces, where-clause multiple bounds, reified parameters and declaration variance on function parameters reject."],
    "generic_inference": ["Fixture establishes argument-driven inference for a generic identity call. It does not establish general expected-result, recursive-constraint or higher-kinded inference."],
    "variance_projections": ["Recorded List projection and Java extends-wildcard reads plus projected-write rejection; no unrestricted capture conversion, variance inference or collection API claim."],
    "annotations_platform": ["Marker JetBrains/JSpecify/AndroidX NonNull/Nullable and javax.annotation Nonnull/Nullable annotations without arguments are supported. Conflicts, unknown annotations, type-use attributes and every package-info/package-default configuration reject explicitly."],
    "overload_specificity": ["Recorded Object/CharSequence/String subtype overloads, source generic versus concrete candidates, null with defaulted parameters, and incomparable candidate ambiguity. Multiple applicable candidates requiring intersection-bound specificity reject explicitly. No full postponed-constraint or complete Kotlin overload-resolution equivalence claim."],
    "compiled_extensions": ["Custom compiled String extension and actual pinned kotlin.text.repeat establish receiver metadata and static lowering. Other APIs must satisfy the same supported signature/body rules; no per-API substitute is used."],
    "library_properties": ["Recorded top-level getter/setter properties with ordinary static accessors and separately compiled public getter/internal setter visibility. No delegated properties, reflection/property references or arbitrary accessor visibility claim."],
    "imported_named_defaults": ["Recorded Kotlin parameter names, reordered source argument evaluation and one default-mask bridge with at most 32 parameters. Compiled inline default and reordered named arguments reject."],
    "function_invocation": ["Ordinary JVM FunctionN signatures and invocation; parser function arities 0 through 22 do not establish runtime evidence for every arity. Suspend and receiver function types reject."],
    "capturing_lambdas": ["Recorded immutable Int captures and mutable Int cell shared through an alias. No arbitrary escaping control flow, capture of uninitialized variables, or coroutine claim."],
    "contextual_lambda_inference": ["Expected concrete function parameter types, explicit parameters and implicit it; no general nested generic builder inference or suspend lambda support."],
    "callable_references": ["Bound/unbound ordinary method references and top-level function references lower to typed lambdas; expected function type is required. Property/constructor references, reference reflection and mixed static/instance same-name overload sets are unsupported."],
    "inherited_dispatch": ["Recorded inherited virtual and interface method lookup, including runtime override selection; generic hierarchy substitution is bounded by supported signatures and source inheritance remains outside this suite."],
    "java_sam": ["Recorded IntUnaryOperator conversion with immutable Int capture; target must expose one supported abstract method. No arbitrary SAM constructor, intersection type, generic SAM or default-method compatibility claim."],
    "compiled_inline": ["Only supported static inline bodies, with direct literal-lambda FunctionN.invoke or null-check uses. Escaping, aliased or forwarded literal lambdas and incompatible lambda identities at control-flow joins reject. Reified functions, reordered named/default inline arguments, unsupported numeric/conversion opcodes and relocated locals beyond 256 reject."],
    "nonlocal_inline_returns": ["Recorded non-local Int/Long/String returns, conditional captures and pending caller operands through supported static inline bodies. noinline/crossinline contracts and ordinary non-inline lambda returns have separate negative outcomes; labeled returns, suspend/reified and arbitrary escaping inline lambdas are unsupported."],
}


def frozen_files():
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for folder in ["src", "tests", "tools"]:
        paths.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
    return file_hashes(paths)


def rejection_outcome(stderr):
    diagnostics = "\n".join(line.split("error:", 1)[1] for line in stderr.splitlines() if "error:" in line)
    if not diagnostics:
        return "fail"
    return "unsupported_language" if re.search(r"\bunsupported\b|\bnot (?:yet )?supported\b", diagnostics, re.I) else "fail"


def boundary_outcome(boundary, returncode, stderr, emitted):
    diagnostics = "\n".join(line.split("error:", 1)[1] for line in stderr.splitlines() if "error:" in line)
    if not returncode or not diagnostics or emitted:
        return "fail"
    matched = re.search(boundary["candidate_diagnostic_regex"], diagnostics, re.I)
    unsupported = rejection_outcome(stderr) == "unsupported_language"
    if not boundary.get("invalid", False):
        return "unsupported_language" if matched and unsupported else "fail"
    if unsupported:
        return "unsupported_language"
    return "pass" if matched else "fail"


def feature_status(cases, feature):
    outcomes = {case["id"]: case["outcome"] for case in cases}
    required = ["compile/" + feature, "java/" + feature, "source_abi/" + feature, "class_verification/" + feature, "kotlin_consumer", "frozen_inputs"]
    required.extend("boundary/" + name for name in FEATURE_BOUNDARIES.get(feature, []))
    if feature in ["compiled_inline", "nonlocal_inline_returns"]:
        required.append("inline_expansion/" + feature)
    if all(outcomes.get(name) == "pass" for name in required):
        return "supported"
    if outcomes.get(required[0]) == "unsupported_language":
        return "unsupported"
    return "partial" if any(outcomes.get(name) == "pass" for name in required) else "unassessed"


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-library-interop"
        self.work = ROOT / "target/checks" / self.run_id
        self.evidence = self.work / "evidence" if args.preflight else ROOT / "reports/correctness" / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix(".json")
        self.report = {
            "schema_version": 1, "kind": "correctness", "created_at": now.isoformat(), "status": "running",
            "compiler": {}, "reference_compiler": {}, "inputs": {}, "commands": [], "evidence": [], "cases": [],
            "environment": {"os": platform.platform(), "machine": platform.machine()},
            "selection": "Twenty independently assessed library interoperability features plus unchanged AndroidBible IntArrayListUtil against its original Java IntArrayList packaged as JAR using genuine Android/AndroidX dependencies. Official-compiled Kotlin/Java library JAR, actual pinned stdlib JAR, fixed Java bytecodes with independent assertions, Kotlin metadata consumers, public/protected facade ABI, explicit boundaries and actual inline call-site inspection. Source/JVM evidence only; no whole-app, Android-device or performance claim.",
            "reproduction_commands": [["python3", "tools/check_library_interop.py", "--compiler", str(args.compiler), "--android-jar", str(args.android_jar), "--androidx-annotations", str(args.androidx_annotations)]],
        }
        self.java, self.javac, self.javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([self.java, self.javac, self.javap]):
            raise RuntimeError("JDK tools required")

    def prepare(self):
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        self.kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        self.stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
        self.annotations = ROOT / pin["path"] / "lib/annotations-13.0.jar"
        self.compiler = self.args.compiler.resolve()
        self.options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "library_interop"]
        version = self.run("Pinned official compiler identity", [self.kotlinc, "-version"])
        if "kotlinc-jvm " + pin["version"] + " " not in version.stdout + version.stderr:
            raise AssertionError("Pinned reference compiler identity mismatch")
        self.frozen = frozen_files()
        self.reference_hashes = file_hashes((ROOT / pin["path"] / "lib").glob("*.jar"))
        self.report["reference_compiler"] = {"version": pin["version"], "options": self.options, "distribution_libs_sha256": self.reference_hashes}
        self.report["inputs"].update({"reference_sources_manifest_sha256": sha(ROOT / "references/sources.json"), "reference_compiler_manifest_sha256": sha(ROOT / "references/compiler.json"), "repository_revision": self.run("Repository revision", ["git", "rev-parse", "HEAD"]).stdout.strip()})
        if self.args.preflight != "official":
            self.compiler_sha = sha(self.compiler)
            self.report["compiler"] = {"version": self.run("Candidate identity", [self.compiler, "--version"]).stdout.strip(), "binary_sha256": self.compiler_sha, "source_files_sha256": self.frozen, "incremental_compilation": False}
        self.report["environment"]["java"] = self.run("Java identity", [self.java, "-version"]).stderr.strip()
        classes = self.work / "library-classes"
        self.run("Compile independent Java dependency", [self.javac, "-cp", self.annotations, "-d", classes, FIXTURES / "JavaApi.java"])
        self.run("Compile independent Kotlin library", [self.kotlinc, FIXTURES / "Library.kt", *self.options, "-cp", classpath(classes, self.annotations), "-d", classes])
        visibility_options = self.options.copy()
        visibility_options[visibility_options.index("-module-name") + 1] = "visibility_dependency"
        self.run("Compile separate Kotlin accessor visibility dependency", [self.kotlinc, FIXTURES / "VisibilityLibrary.kt", *visibility_options, "-d", classes])
        self.library = self.work / "independent-library.jar"
        with zipfile.ZipFile(self.library, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(classes.rglob("*")):
                if path.is_file():
                    archive.write(path, path.relative_to(classes).as_posix())
        self.archive(self.library, self.library.name)
        self.report["inputs"].update({"library_sources_sha256": file_hashes([FIXTURES / "JavaApi.java", FIXTURES / "Library.kt", FIXTURES / "VisibilityLibrary.kt"]), "fixtures_sha256": file_hashes(path for path in FIXTURES.rglob("*") if path.is_file()), "library_jar_sha256": sha(self.library), "stdlib_jar_sha256": sha(self.stdlib), "classpath_format": "JAR inputs passed directly to both compilers; no extraction or API substitutes", "separate_visibility_dependency_module": "visibility_dependency"})
        self.official = self.work / "official"
        self.candidate = self.work / "candidate"
        self.run("Compile all official feature probes", [self.kotlinc, *[FIXTURES / item["fixture"] for item in FEATURES], *self.options, "-cp", classpath(self.library, self.stdlib), "-d", self.official])
        self.drivers = self.work / "java-callers"
        self.run("Compile fixed Java caller against official output", [self.javac, "-cp", classpath(self.official, self.library, self.stdlib), "-d", self.drivers, FIXTURES / "LibraryInteropCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])
        names = [name[:-6].replace("/", ".") for name in tree_hashes(self.official, ".class")]
        self.run("Verify every official emitted probe class", [self.java, "-Xverify:all", "-cp", classpath(self.drivers, self.official, self.library, self.stdlib), "ModuleClassVerification", *names])
        self.case("official_class_verification", classes=len(names))

    def compile_sources(self):
        self.compiled = []
        for item in FEATURES:
            ident = item["id"]
            expected = self.run("Independent official Java assertions " + ident, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, self.official, self.library, self.stdlib), "LibraryInteropCheck", ident])
            self.case("oracle/" + ident, **{key: value for key, value in json.loads(expected.stdout).items() if key not in ["id", "outcome"]})
            if self.args.preflight == "official":
                continue
            destination = self.candidate / ident
            result = self.run("Compile candidate " + ident, [self.compiler, FIXTURES / item["fixture"], "-cp", classpath(self.library, self.stdlib), "-d", destination], allowed=(0, 1))
            if result.returncode:
                self.report["cases"].append({"id": "compile/" + ident, "outcome": rejection_outcome(result.stderr), "official_outcome": "accepted", "diagnostics": result.stderr})
                if tree_hashes(destination, ".class"):
                    raise AssertionError("Rejected source emitted class files: " + ident)
                continue
            self.compiled.append(ident)
            self.case("compile/" + ident, emitted_classes_sha256=tree_hashes(destination, ".class"))
            actual = self.run("Independent candidate Java assertions " + ident, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, destination, self.library, self.stdlib), "LibraryInteropCheck", ident], allowed=(0, 1))
            if actual.returncode or actual.stdout != expected.stdout:
                self.report["cases"].append({"id": "java/" + ident, "outcome": "fail", "message": "Fixed Java caller runtime assertion or differential comparison failed"})
            else:
                self.case("java/" + ident, runtime_checks=json.loads(actual.stdout)["checks"], jvm_verification="all", oracle="Same Java bytecodes compiled once against official output; independent expected values, exceptions and evaluation order")
            abi = [self.run("Inspect " + provider + " facade ABI " + ident, [self.javap, "-protected", "-s", "-classpath", directory, item["facade"]]).stdout for provider, directory in [("official", self.official), ("candidate", destination)]]
            self.report["cases"].append({"id": "source_abi/" + ident, "outcome": "pass" if source_abi(abi[0]) == source_abi(abi[1]) else "fail", "scope": "Every public/protected facade member descriptor and generic signature, including public synthetic default bridges; normalize member order only. Private lambda lowering helpers and generated closure class layout differ and are excluded; every emitted class is verified separately."})
            names = [name[:-6].replace("/", ".") for name in tree_hashes(destination, ".class")]
            verified = self.run("Verify every candidate class " + ident, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, destination, self.library, self.stdlib), "ModuleClassVerification", *names], allowed=(0, 1))
            self.report["cases"].append({"id": "class_verification/" + ident, "outcome": "pass" if verified.returncode == 0 else "fail", "classes": len(names)})
            if ident in ["compiled_inline", "nonlocal_inline_returns"]:
                code = self.run("Inspect actual inline expansion " + ident, [self.javap, "-p", "-c", "-classpath", destination, item["facade"]]).stdout
                calls_body = re.search(r"invokestatic.*interop/lib/LibraryKt\.(?:actualInline|gate|twice|branch|inlineSimple)", code)
                self.report["cases"].append({"id": "inline_expansion/" + ident, "outcome": "fail" if calls_body else "pass", "scope": "javap call-site bytecode must contain no invokestatic call to tested library inline bodies; runtime assertions check body effects, branches/loops, captures and non-local return"})

    def consumers(self):
        source = self.work / "LibraryConsumer.kt"
        lines = ["package interop.consumer", "import interop.probes.*", "fun main() {"]
        for ident, (expression, expected) in CONSUMER_CALLS.items():
            lines.append("check(" + expression + " == " + expected + ")")
        lines.extend(['check(stdlibRepeatProbe("ab", 2) == "abab")', 'check(restrictedGetterProbe() == 6)', 'check(nullDefaultSpecificityProbe() == 1)', 'check(genericSpecificityProbe() == 2)', 'check(subtypeSpecificityProbe() == 1)', 'check(preferNullable(null) == 1)', 'check(preferConcrete("text") == 2)', 'println("{\\\"checks\\\":27}")', "}"])
        source.write_text("\n".join(lines) + "\n")
        self.archive(source, source.name)
        providers = [("official", [self.official])]
        if self.args.preflight != "official" and len(self.compiled) == len(FEATURES):
            providers.append(("candidate", [self.candidate / item["id"] for item in FEATURES]))
        for provider, outputs in providers:
            destination = self.work / (provider + "-consumer")
            self.run("Compile official Kotlin consumer against " + provider, [self.kotlinc, source, *self.options, "-cp", classpath(*outputs, self.library, self.stdlib), "-d", destination])
            self.run("Run official Kotlin consumer " + provider, [self.java, "-Xverify:all", "-cp", classpath(destination, *outputs, self.library, self.stdlib), "interop.consumer.LibraryConsumerKt"])
        if len(providers) == 2:
            self.case("kotlin_consumer", runtime_checks=27, scope="Pinned official Kotlin compiles and runs identical consumer against both providers, exercising emitted Kotlin metadata and JVM ABI")
        elif self.args.preflight != "official":
            self.report["cases"].append({"id": "kotlin_consumer", "outcome": "not_run", "reason": "Some candidate features did not compile"})

    def supported_boundary(self, boundary, source):
        regression = boundary["supported_regression"]
        java_options = regression.get("java_options", [])
        self.report["inputs"].setdefault("supported_boundary_runtime_options", {})[boundary["id"]] = {"java_options": java_options, "scope": regression["scope"]}
        outputs = {"official": self.work / ("official-boundary-" + boundary["id"])}
        self.run("Compile supported official boundary " + boundary["id"], [self.kotlinc, source, *self.options, "-cp", classpath(self.library, self.stdlib), "-d", outputs["official"]])
        if self.args.preflight != "official":
            outputs["candidate"] = self.work / ("candidate-boundary-" + boundary["id"])
            self.run("Compile supported candidate boundary " + boundary["id"], [self.compiler, source, "-cp", classpath(self.library, self.stdlib), "-d", outputs["candidate"]])
        java_source = FIXTURES / regression["java_fixture"]
        self.archive(java_source, "supported-" + boundary["id"] + "-" + java_source.name)
        callers = self.work / ("supported-boundary-callers-" + boundary["id"])
        self.run("Compile fixed Java boundary caller against official " + boundary["id"], [self.javac, "-cp", classpath(outputs["official"], self.library, self.stdlib), "-d", callers, java_source])
        fixed_callers = tree_hashes(callers, ".class")
        observations, abis = [], []
        for provider, output in outputs.items():
            if not tree_hashes(output, ".class"):
                raise AssertionError("Supported boundary emitted no classes: " + boundary["id"])
            result = self.run("Run fixed Java boundary caller " + provider + "/" + boundary["id"], [self.java, *java_options, "-Xverify:all", "-cp", classpath(callers, output, self.library, self.stdlib), regression["main_class"]])
            observation = json.loads(result.stdout)
            if observation.get("id") != boundary["id"] or not isinstance(observation.get("checks"), int) or observation["checks"] <= 0:
                raise AssertionError("Missing independent positive boundary assertions: " + boundary["id"])
            observations.append(observation)
            abis.append(self.run("Inspect supported boundary ABI " + provider + "/" + boundary["id"], [self.javap, "-protected", "-s", "-classpath", output, regression["facade"]]).stdout)
        if any(value != observations[0] for value in observations) or any(source_abi(value) != source_abi(abis[0]) for value in abis):
            raise AssertionError("Positive boundary runtime or ABI differs: " + boundary["id"])
        if fixed_callers != tree_hashes(callers, ".class"):
            raise AssertionError("Fixed Java boundary caller changed: " + boundary["id"])
        self.case("boundary/" + boundary["id"], supported_regression=True, official_outcome="accepted", source_sha256=sha(source), java_fixture_sha256=sha(java_source), fixed_java_callers_sha256=fixed_callers, runtime_checks=observations[0]["checks"], providers=list(outputs), java_options=java_options, jvm_verification="all", scope=regression["scope"])

    def boundaries(self):
        for boundary in json.loads((FIXTURES / "rejections.json").read_text()):
            source = self.work / ("Boundary_" + boundary["id"] + ".kt")
            source.write_text(boundary["source"] + "\n")
            self.archive(source, source.name)
            if boundary.get("supported_regression"):
                self.supported_boundary(boundary, source)
                continue
            invalid = boundary.get("invalid", False)
            self.run("Official boundary " + boundary["id"], [self.kotlinc, source, *self.options, "-cp", classpath(self.library, self.stdlib), "-d", self.work / ("official-boundary-" + boundary["id"])], allowed=(1,) if invalid else (0,))
            if self.args.preflight == "official":
                continue
            destination = self.work / ("candidate-rejected-" + boundary["id"])
            result = self.run("Candidate negative boundary " + boundary["id"], [self.compiler, source, "-cp", classpath(self.library, self.stdlib), "-d", destination], allowed=(0, 1))
            outcome = boundary_outcome(boundary, result.returncode, result.stderr, tree_hashes(destination, ".class"))
            self.report["cases"].append({"id": "boundary/" + boundary["id"], "outcome": outcome, "official_outcome": "rejected" if invalid else "accepted", "source_sha256": sha(source), "candidate_diagnostics": result.stderr})

    def androidbible(self):
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        original = {}
        for name in [APP_SOURCE, APP_JAVA]:
            if sha(SNAPSHOT / name) != inventory[name]:
                raise AssertionError("Pinned original AndroidBible source changed: " + name)
            original[name] = sha(SNAPSHOT / name)
            self.archive(SNAPSHOT / name, "original-" + Path(name).name)
        android, annotations = self.args.android_jar.resolve(), self.args.androidx_annotations.resolve()
        self.app_external_hashes = {str(path): sha(path) for path in [android, annotations]}
        self.app_original_hashes = original
        self.report["inputs"]["androidbible"] = {
            "commit": json.loads((ROOT / "benchmarks/sources.json").read_text())["sources"]["androidbible"]["commit"],
            "unchanged_sources_sha256": original,
            "real_dependencies_sha256": self.app_external_hashes,
            "scope": "Original Java IntArrayList compiled unchanged using actual Android platform and androidx.annotation JARs, packaged into a JAR passed directly to both compilers. No source stubs. JVM checks exercise array conversion only; Android Parcel APIs are not executed.",
        }
        self.case("androidbible/unchanged_sources", source_files=len(original))
        classes = self.work / "androidbible-java"
        self.run("Compile unchanged real Java IntArrayList", [self.javac, "-cp", classpath(android, annotations, self.stdlib), "-d", classes, SNAPSHOT / APP_JAVA])
        library = self.work / "androidbible-intarraylist.jar"
        with zipfile.ZipFile(library, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(classes.rglob("*.class")):
                archive.write(path, path.relative_to(classes).as_posix())
        self.archive(library, library.name)
        self.report["inputs"]["androidbible"]["java_dependency_jar_sha256"] = sha(library)
        dependencies = [library, android, annotations, self.stdlib]
        official = self.work / "androidbible-official"
        self.run("Compile unchanged AndroidBible Kotlin against original Java JAR official", [self.kotlinc, SNAPSHOT / APP_SOURCE, *self.options, "-cp", classpath(*dependencies), "-d", official])
        drivers = self.work / "androidbible-callers"
        self.run("Compile fixed app Java caller against official output", [self.javac, "-cp", classpath(official, *dependencies), "-d", drivers, FIXTURES / "AndroidBibleJarCheck.java"])
        expected = self.run("Independent original app official JVM assertions", [self.java, "-Xverify:all", "-cp", classpath(drivers, official, *dependencies), "AndroidBibleJarCheck"])
        self.case("androidbible/oracle", runtime_checks=json.loads(expected.stdout)["checks"])
        if self.args.preflight == "official":
            return
        candidate = self.work / "androidbible-candidate"
        result = self.run("Compile unchanged AndroidBible Kotlin against original Java JAR candidate", [self.compiler, SNAPSHOT / APP_SOURCE, "-cp", classpath(*dependencies), "-d", candidate], allowed=(0, 1))
        if result.returncode:
            self.report["cases"].append({"id": "androidbible/compile", "outcome": rejection_outcome(result.stderr), "official_outcome": "accepted", "candidate_diagnostics": result.stderr})
            return
        self.case("androidbible/compile", emitted_classes_sha256=tree_hashes(candidate, ".class"))
        actual = self.run("Independent original app candidate JVM assertions", [self.java, "-Xverify:all", "-cp", classpath(drivers, candidate, *dependencies), "AndroidBibleJarCheck"], allowed=(0, 1))
        self.report["cases"].append({"id": "androidbible/java", "outcome": "pass" if actual.returncode == 0 and actual.stdout == expected.stdout else "fail", "runtime_checks": json.loads(expected.stdout)["checks"], "jvm_verification": "all", "scope": "Identical Java caller compiled once against official output, independent expected array contents, alias isolation, deterministic random samples, null/negative exceptions and virtual method order"})
        facade = "yuku.alkitab.base.util.IntArrayListUtilKt"
        abis = [self.run("Inspect original app " + provider + " ABI", [self.javap, "-p", "-s", "-classpath", output, facade]).stdout for provider, output in [("official", official), ("candidate", candidate)]]
        self.report["cases"].append({"id": "androidbible/source_abi", "outcome": "pass" if source_abi(abis[0]) == source_abi(abis[1]) else "fail"})
        consumer = self.work / "AppConsumer.kt"
        consumer.write_text('package interop.appconsumer\nimport yuku.alkitab.util.IntArrayList\nimport yuku.alkitab.base.util.toIntArray\nfun main() { val value = IntArrayList(0); value.add(17); check(value.toIntArray().contentEquals(intArrayOf(17))); println("app consumer passed") }\n')
        self.archive(consumer, consumer.name)
        observations = []
        for provider, output in [("official", official), ("candidate", candidate)]:
            destination = self.work / ("app-consumer-" + provider)
            self.run("Compile official Kotlin app consumer against " + provider, [self.kotlinc, consumer, *self.options, "-cp", classpath(output, *dependencies), "-d", destination])
            observations.append(self.run("Run official Kotlin app consumer " + provider, [self.java, "-Xverify:all", "-cp", classpath(destination, output, *dependencies), "interop.appconsumer.AppConsumerKt"]).stdout)
        self.report["cases"].append({"id": "androidbible/kotlin_consumer", "outcome": "pass" if observations[0] == observations[1] else "fail"})

    def features(self):
        report = {key: self.report[key] for key in ["schema_version", "created_at", "compiler", "reference_compiler", "inputs"]}
        report.update({"kind": "features", "status": "assessed", "commands": self.report["reproduction_commands"], "evidence": [relative(self.report_path)], "features": []})
        for item in FEATURES:
            ident = item["id"]
            status = feature_status(self.report["cases"], ident)
            related = {"boundary/" + name for name in FEATURE_BOUNDARIES.get(ident, [])}
            restrictions = [*RESTRICTIONS[ident], "Evidence is bounded to the recorded fixtures and pinned dependencies. No whole-app, Android-device runtime, or compilation-performance claim."]
            if ident in ["compiled_inline", "nonlocal_inline_returns"]:
                restrictions.append("Ordinary referenced-class allocation in compiled inline bodies is tested under recorded controlled HotSpot identity hash options. Generated/nested inline-owner class regeneration, exception tables, invokedynamic, switches and unsupported stack operations still reject explicitly. This suite exercises Function0/Function1 bodies with branches/loops, primitive and captured values; the remaining exception-handler boundary is a rejection.")
                related.update({"boundary/inline_allocation", "boundary/inline_exception_handler"})
            if ident in ["jar_classpath", "compiled_extensions"]:
                related.update(case["id"] for case in self.report["cases"] if case["id"].startswith("androidbible/"))
            if ident == "type_variables_bounds":
                related.add("boundary/primitive_source_type_bound")
            report["features"].append({"name": ident, "status": status, "scope": item["scope"], "evidence_cases": [case["id"] for case in self.report["cases"] if case["id"].endswith("/" + ident) or case["id"] in related | {"kotlin_consumer", "frozen_inputs"}], "restrictions": restrictions})
        (ROOT / "reports/features" / (self.run_id + ".json")).write_text(json.dumps(report, indent=2) + "\n")

    def execute(self):
        try:
            self.prepare()
            self.compile_sources()
            self.consumers()
            self.boundaries()
            self.androidbible()
            if not self.args.preflight:
                if self.frozen != frozen_files() or self.reference_hashes != file_hashes(Path(ROOT / json.loads((ROOT / "references/compiler.json").read_text())["path"] / "lib").glob("*.jar")) or sha(self.compiler) != self.compiler_sha:
                    raise AssertionError("Frozen compiler, fixtures, tool inventory or reference JARs changed during final run")
                if any(sha(Path(path)) != checksum for path, checksum in self.app_external_hashes.items()) or any(sha(SNAPSHOT / name) != checksum for name, checksum in self.app_original_hashes.items()):
                    raise AssertionError("Frozen original app sources or real Android/annotation dependencies changed")
                self.case("frozen_inputs", source_files=len(self.frozen), compiler_binary_sha256=self.compiler_sha)
            self.report["status"] = "failed" if any(case["outcome"] in ["fail", "timeout"] for case in self.report["cases"]) else "completed"
        except Exception as error:
            self.report["status"] = "failed"
            self.report["cases"].append({"id": "harness_failure", "outcome": "timeout" if isinstance(error, TimeoutError) else "fail", "message": str(error)})
            print(str(error), file=sys.stderr)
        finally:
            self.report["counts"] = dict(Counter(case["outcome"] for case in self.report["cases"]))
            self.report["finished_at"] = datetime.now(timezone.utc).isoformat()
            self.report_path.write_text(json.dumps(self.report, indent=2) + "\n")
            if not self.args.preflight:
                self.features()
            print("Report: " + relative(self.report_path), flush=True)
        return 0 if self.report["status"] == "completed" else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", type=Path, default=ROOT / "target/release/korrust")
    parser.add_argument("--preflight", choices=["official", "candidate"])
    parser.add_argument("--android-jar", type=Path, default=Path("/workspace/korrust-setup/android-sdk/platforms/android-37.0/android.jar"))
    parser.add_argument("--androidx-annotations", type=Path, default=Path("/workspace/korrust-setup/gradle-home/caches/modules-2/files-2.1/androidx.annotation/annotation-jvm/1.9.1/b17951747e38bf3986a24431b9ba0d039958aa5f/annotation-jvm-1.9.1.jar"))
    args = parser.parse_args()
    return Harness(args).execute()


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
