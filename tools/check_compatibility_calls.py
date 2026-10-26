#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import json
import hashlib
from pathlib import Path, PurePosixPath
import platform
import re
import shutil
import sys
import zipfile

from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha
from check_androidbible_simple import Harness as LoggedHarness
from check_nowinandroid import file_hashes, source_abi, tree_hashes

FIXTURES = ROOT / "tests/fixtures/compatibility_calls"
CASES = json.loads((FIXTURES / "cases.json").read_text())
APP_SOURCES = {
    "PrereleaseVersion": "Alkitab/src/main/java/yuku/alkitab/base/feedback/PrereleaseVersion.kt",
    "ReadingPassage": "Alkitab/src/main/java/yuku/alkitab/base/util/ReadingPassage.kt",
    "LidToAri": "Alkitab/src/main/java/yuku/alkitab/base/util/LidToAri.kt",
}
JAVA_SOURCES = ["AlkitabModel/src/main/java/yuku/alkitab/model/Book.java", "AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java", "AlkitabModel/src/main/java/yuku/alkitab/util/Ari.java"]
PARSER_SOURCE = "Alkitab/src/main/java/yuku/alkitabconverter/util/DesktopVerseParser.kt"
REFERENCE_PATHS = [
    "references/kotlin-spec/docs/src/md/kotlin.core/packages.md",
    "references/kotlin/libraries/stdlib/jvm/builtins/Library.kt",
    "references/kotlin-spec/docs/src/md/kotlin.core/type-constraints.md",
    "references/kotlin-spec/docs/src/md/kotlin.core/type-system.md",
    "references/kotlin-spec/docs/src/md/kotlin.core/type-inference.md",
    "references/kotlin-spec/docs/src/md/kotlin.core/expressions.md",
    "references/kotlin-spec/docs/src/md/kotlin.core/declarations.md",
    "references/kotlin/compiler/testData/codegen/box/vararg/spreadCopiesArray.kt",
    "references/kotlin/compiler/testData/codegen/box/vararg/evaluationOrder.kt",
    "references/kotlin/compiler/testData/codegen/box/vararg/boxingArgumentsForVararg.kt",
    "references/kotlin/compiler/testData/codegen/bytecodeText/defaultArguments/maskCheckSequence.kt",
]

NEGATIVE_DIAGNOSTICS = {
    "InvalidGenericBound": r"bound|type|argument|overload",
    "InvalidPlatformNullableFirst": r"type|nullable|mismatch",
    "InvalidNullablePlatformFirst": r"type|nullable|mismatch",
    "InvalidJavaNamed": r"named|argument|overload",
    "InvalidNullableSpread": r"spread|component|array|type|argument|overload",
    "InvalidPrimitiveSpread": r"spread|component|array|type|argument|overload",
    "InvalidSpreadScalar": r"spread|array|type|argument",
    "InvalidVarargBound": r"bound|type|argument|overload",
    "InvalidVarargAmbiguity": r"ambig|multiple.*overload|more than one.*overload",
}


def frozen_files():
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
    for folder in ["src", "tests", "tools"]:
        paths.extend(path for path in (ROOT / folder).rglob("*") if path.is_file() and "__pycache__" not in path.parts)
    return file_hashes(paths)


def rejected_outcome(returncode, diagnostics, emitted, invalid=False, semantic_pattern=None):
    messages = "\n".join(re.findall(r"\berror:\s*([^\n]*)", diagnostics, re.I))
    if returncode != 1 or emitted or not messages:
        return "fail"
    if re.search(r"\bunsupported\b|\bnot (?:yet )?supported\b", messages, re.I):
        return "unsupported_language"
    if invalid and semantic_pattern and not re.search(semantic_pattern, messages, re.I):
        return "fail"
    return "pass" if invalid else "fail"


class Harness(LoggedHarness):
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-compatibility-calls"
        self.work = ROOT / "target/checks" / self.run_id
        self.evidence = self.work / "evidence" if args.preflight else ROOT / "reports/correctness" / self.run_id
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.report_path = self.evidence.with_suffix(".json")
        self.report = dict(schema_version=1, kind="correctness", created_at=now.isoformat(), status="running", compiler={}, reference_compiler={}, inputs={}, commands=[], evidence=[], cases=[], environment=dict(os=platform.platform(), machine=platform.machine()), selection="Checkpoint C: generic constraints, compiled Kotlin defaults, ordinary/source varargs and spread copy/evaluation semantics, followed by three unchanged AndroidBible sources against their real dependency closure. Fixed official-compiled Java callers, independent assertions, JVM verification, public source ABI and actual Kotlin consumers. No Android Parcel operation, device, full-app or performance claim.", reproduction_commands=[["python3", "tools/check_compatibility_calls.py", "--compiler", str(args.compiler), "--android-jar", str(args.android_jar), "--androidx-annotations", str(args.androidx_annotations)] + (["--preflight", args.preflight] if args.preflight else []) + (["--parity-only"] if getattr(args, "parity_only", False) else [])])
        if getattr(args, "parity_only", False):
            self.report["selection"] = "Focused actual-stdlib builtin factory JAR/directory parity, with full physical resource hash provenance and fixed official Java callers. No whole checkpoint or app compatibility claim."
        self.java, self.javac, self.javap = [shutil.which(name) for name in ["java", "javac", "javap"]]
        if not all([self.java, self.javac, self.javap]):
            raise RuntimeError("JDK tools required")

    def record(self, ident, outcome, **details):
        self.report["cases"].append(dict(id=ident, outcome=outcome, **details))

    def prepare(self):
        pin = json.loads((ROOT / "references/compiler.json").read_text())
        self.kotlinc = ROOT / pin["path"] / "bin/kotlinc"
        self.stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
        self.compiler = self.args.compiler.resolve()
        self.options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "compatibility_calls"]
        version = self.run("Pinned compiler identity", [self.kotlinc, "-version"])
        if "kotlinc-jvm " + pin["version"] + " " not in version.stdout + version.stderr:
            raise AssertionError("Pinned compiler version mismatch")
        self.frozen = frozen_files()
        self.reference_hashes = file_hashes((ROOT / pin["path"] / "lib").glob("*.jar"))
        self.report["reference_compiler"] = dict(version=pin["version"], options=self.options, distribution_sha256=self.reference_hashes)
        if self.args.preflight != "official":
            self.compiler_sha = sha(self.compiler)
            self.report["compiler"] = dict(binary_sha256=self.compiler_sha, source_files_sha256=self.frozen, version=self.run("Candidate identity", [self.compiler, "--version"]).stdout.strip(), jvm_target_scope="Official JVM21; candidate default classfile target, verified source/ABI/runtime subset.")
        self.report["environment"]["java"] = self.run("Java identity", [self.java, "-version"]).stderr.strip()
        self.report["inputs"].update(fixtures_sha256=file_hashes(path for path in FIXTURES.rglob("*") if path.is_file()), references_sha256=file_hashes(ROOT / name for name in REFERENCE_PATHS), repository_revision=self.run("Repository revision", ["git", "rev-parse", "HEAD"]).stdout.strip())
        if getattr(self.args, "parity_only", False):
            self.official = self.work / "official"
            self.run("Compile official builtin factory", [self.kotlinc, FIXTURES / "BuiltinFactories.kt", *self.options, "-no-stdlib", "-cp", self.stdlib, "-d", self.official])
            self.drivers = self.work / "java-callers"
            self.run("Compile fixed builtin factory Java caller", [self.javac, "-cp", classpath(self.official, self.stdlib), "-d", self.drivers, FIXTURES / "BuiltinFactoryCheck.java"])
            return
        self.library = self.work / "library"
        self.run("Compile Java dependency", [self.javac, "-d", self.library, FIXTURES / "CallEffects.java"])
        self.run("Compile Kotlin dependency", [self.kotlinc, FIXTURES / "CallLibrary.kt", *self.options, "-cp", self.library, "-d", self.library])
        self.library_jar = self.work / "calls-library.jar"
        with zipfile.ZipFile(self.library_jar, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for path in sorted(self.library.rglob("*")):
                if path.is_file():
                    archive.write(path, path.relative_to(self.library).as_posix())
        self.archive(self.library_jar, self.library_jar.name)
        self.report["inputs"]["independent_library_jar_sha256"] = sha(self.library_jar)
        self.official = self.work / "official"
        self.run("Compile official call fixtures", [self.kotlinc, *[FIXTURES / item["fixture"] for item in CASES], FIXTURES / "BuiltinFactories.kt", *self.options, "-cp", self.library_jar, "-d", self.official])
        self.drivers = self.work / "java-callers"
        self.run("Compile fixed Java callers against official output", [self.javac, "-cp", classpath(self.official, self.library_jar, self.stdlib), "-d", self.drivers, FIXTURES / "CallCheck.java", FIXTURES / "PlatformJoinCheck.java", FIXTURES / "BuiltinFactoryCheck.java", ROOT / "tests/fixtures/ModuleClassVerification.java"])

    def verify(self, ident, output, dependencies, owner, consumer_source, driver, argument):
        cp = classpath(output, *dependencies, self.stdlib)
        result = self.run("Fixed Java runtime " + ident, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, cp), driver, argument])
        self.case("java/" + ident, stdout=result.stdout.strip())
        names = [name[:-6].replace("/", ".") for name in tree_hashes(output, ".class")]
        self.run("Verify every emitted class " + ident, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, cp), "ModuleClassVerification", *names])
        self.case("verification/" + ident, classes=len(names))
        source = self.work / ("Main" + re.sub(r"[^A-Za-z0-9]", "", ident) + "Consumer.kt")
        source.write_text(consumer_source)
        self.archive(source, source.name)
        destination = self.work / (ident.replace("/", "-") + "-consumer")
        self.run("Official Kotlin consumer " + ident, [self.kotlinc, source, *self.options, "-cp", cp, "-d", destination])
        self.run("Kotlin consumer runtime " + ident, [self.java, "-Xverify:all", "-cp", classpath(destination, cp), source.stem + "Kt"])
        self.case("kotlin_consumer/" + ident)
        return self.run("Public signatures " + ident, [self.javap, "-classpath", cp, "-protected", "-s", owner]).stdout

    def classpath_parity(self):
        directory = self.work / "stdlib-directory"
        directory.mkdir()
        entries = {}
        with zipfile.ZipFile(self.stdlib) as archive:
            for info in archive.infolist():
                name = PurePosixPath(info.filename)
                if name.is_absolute() or ".." in name.parts or "\\" in info.filename:
                    raise ValueError("Unsafe pinned stdlib entry: " + info.filename)
                if info.is_dir():
                    continue
                if info.filename in entries:
                    raise ValueError("Duplicate pinned stdlib entry: " + info.filename)
                data = archive.read(info)
                output = directory / info.filename
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(data)
                checksum = hashlib.sha256(data).hexdigest()
                if sha(output) != checksum:
                    raise AssertionError("Extracted stdlib resource differs: " + info.filename)
                entries[info.filename] = checksum
        manifest = self.work / "stdlib-directory-sha256.json"
        manifest.write_text(json.dumps(dict(source_jar_sha256=sha(self.stdlib), physical_directory=relative(directory), entries_sha256=entries), indent=2) + "\n")
        archived = self.archive(manifest, manifest.name)
        resources = {relative(directory / name): checksum for name, checksum in entries.items() if name.endswith((".kotlin_builtins", ".kotlin_module", "/module-info.class")) or name == "module-info.class"}
        if not any(name.endswith(".kotlin_builtins") for name in resources) or not any(name.endswith(".kotlin_module") for name in resources) or not any(name.endswith("module-info.class") for name in resources):
            raise AssertionError("Pinned stdlib parity resources missing")
        self.report["inputs"]["stdlib_classpath_parity"] = dict(source_jar=relative(self.stdlib), source_jar_sha256=sha(self.stdlib), physical_directory=relative(directory), entries=len(entries), full_extraction_manifest=relative(archived), manifest_sha256=sha(archived), physical_resources_sha256=resources, extraction_scope="Every regular entry, including metadata, builtin resources and versioned module descriptor, preserved byte-for-byte; no filtering.")
        official_directory = self.work / "official-builtin-directory"
        self.run("Compile official builtin factory with directory classpath", [self.kotlinc, FIXTURES / "BuiltinFactories.kt", *self.options, "-no-stdlib", "-cp", directory, "-d", official_directory])
        runtimes = {}
        for name, output, dependency in [("official-jar", self.official, self.stdlib), ("official-directory", official_directory, directory)]:
            runtimes[name] = self.run("Fixed builtin factory runtime " + name, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, output, dependency), "BuiltinFactoryCheck"]).stdout.strip()
        if self.args.preflight == "official":
            self.case("stdlib_builtin_classpath_parity", candidate_checked=False, runtimes=runtimes, scope="Official reference JAR and fully preserved directory only; candidate not run.")
            return
        artifacts = {}
        try:
            for name, dependency in [("jar", self.stdlib), ("directory", directory)]:
                output = self.work / ("candidate-builtin-" + name)
                result = self.run("Compile candidate builtin factory with " + name, [self.compiler, FIXTURES / "BuiltinFactories.kt", "--module-name", "compatibility_calls", "-cp", dependency, "-d", output], allowed=(0, 1))
                if result.returncode:
                    self.record("stdlib_builtin_classpath_parity", rejected_outcome(result.returncode, result.stderr, list(output.rglob("*.class"))), variant=name, diagnostic=result.stderr, candidate_checked=True)
                    return
                runtimes[name] = self.run("Fixed builtin factory runtime candidate " + name, [self.java, "-Xverify:all", "-cp", classpath(self.drivers, output, dependency), "BuiltinFactoryCheck"]).stdout.strip()
                artifacts[name] = tree_hashes(output)
            if artifacts["jar"] != artifacts["directory"] or len(set(runtimes.values())) != 1:
                raise AssertionError("JAR/directory emitted class/module bytes or runtime differ")
            self.case("stdlib_builtin_classpath_parity", candidate_checked=True, emitted_files_sha256=artifacts, runtimes=runtimes, scope="Same source and module identity; actual pinned stdlib JAR versus byte-for-byte full extraction. Fixed official Java caller and JVM verification for all four providers.")
        except (AssertionError, TimeoutError) as error:
            self.record("stdlib_builtin_classpath_parity", "timeout" if isinstance(error, TimeoutError) else "fail", message=str(error), candidate_checked=True)

    def nullable_consumer(self, ident, output, dependencies, name):
        expressions = {
            "PrereleaseVersion": 'fun invalid(): yuku.alkitab.base.feedback.PrereleaseVersion = yuku.alkitab.base.feedback.PrereleaseVersion.parse("x")',
            "ReadingPassage": 'fun invalid(): List<yuku.alkitab.base.util.ReadingRange> = yuku.alkitab.base.util.ReadingPassage.resolve(IntArray(0), { null })',
        }
        if name not in expressions:
            return
        source = self.work / ("Nullable" + re.sub(r"[^A-Za-z0-9]", "", ident) + ".kt")
        source.write_text(expressions[name] + "\n")
        self.archive(source, source.name)
        destination = self.work / ("nullable-" + ident)
        result = self.run("Kotlin nullable return boundary " + ident, [self.kotlinc, source, *self.options, "-cp", classpath(output, *dependencies, self.stdlib), "-d", destination], allowed=(0, 1))
        outcome = rejected_outcome(result.returncode, result.stderr, list(destination.rglob("*.class")), invalid=True, semantic_pattern=r"type.*mismatch|mismatch.*type")
        self.record("kotlin_nullability/" + ident, outcome, diagnostic=result.stderr)
        if ident.startswith("official-") and outcome != "pass":
            raise AssertionError("Official nullable consumer boundary did not reject semantically: " + name)

    def calls(self):
        for item in CASES:
            ident = item["id"]
            consumer = "fun main() { check(compatibility.calls." + ident + "() == " + json.dumps(item["expected"]) + "); " + ("check(compatibility.calls.sourceSum(3, 4) == 7)" if ident == "sourceVarargs" else "") + " }\n"
            consumer = item.get("consumer", consumer)
            oracle_abi = self.verify("official-" + ident, self.official, [self.library_jar], item["owner"], consumer, "CallCheck", ident)
            if self.args.preflight == "official":
                continue
            output = self.work / ("candidate-" + ident)
            result = self.run("Compile candidate " + ident, [self.compiler, FIXTURES / item["fixture"], "-cp", classpath(self.library_jar, self.stdlib), "--module-name", "compatibility_calls", "-d", output], allowed=(0, 1))
            if result.returncode:
                self.record("compile/" + ident, rejected_outcome(result.returncode, result.stderr, list(output.rglob("*.class"))), diagnostic=result.stderr)
                continue
            self.case("compile/" + ident)
            try:
                candidate_abi = self.verify(ident, output, [self.library_jar], item["owner"], consumer, "CallCheck", ident)
                self.record("source_abi/" + ident, "pass" if source_abi(candidate_abi) == source_abi(oracle_abi) else "fail")
            except (AssertionError, TimeoutError) as error:
                self.record("validation/" + ident, "timeout" if isinstance(error, TimeoutError) else "fail", message=str(error))

    def boundaries(self):
        for source in sorted(FIXTURES.glob("Invalid*.kt")):
            official = self.work / (source.stem + "-official")
            result = self.run("Official invalid " + source.stem, [self.kotlinc, source, *self.options, "-cp", self.library_jar, "-d", official], allowed=(0, 1))
            if result.returncode == 0:
                raise AssertionError("Invalid boundary accepted officially: " + source.name)
            self.case("official_invalid/" + source.stem)
            if self.args.preflight == "official":
                continue
            output = self.work / (source.stem + "-candidate")
            result = self.run("Candidate invalid " + source.stem, [self.compiler, source, "-cp", classpath(self.library_jar, self.stdlib), "-d", output], allowed=(0, 1))
            self.record("invalid/" + source.stem, rejected_outcome(result.returncode, result.stderr, list(output.rglob("*.class")) + list(output.rglob("*.kotlin_module")), invalid=True, semantic_pattern=NEGATIVE_DIAGNOSTICS[source.stem]), diagnostic=result.stderr, official_rejects=True)

    def apps(self):
        for path in [self.args.android_jar, self.args.androidx_annotations]:
            if not path.is_file():
                raise FileNotFoundError("Real Android/annotation dependency missing: " + str(path))
        originals = [SNAPSHOT / name for name in [*APP_SOURCES.values(), *JAVA_SOURCES, PARSER_SOURCE]]
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        for path in originals:
            if sha(path) != inventory[path.relative_to(SNAPSHOT).as_posix()]:
                raise AssertionError("Original app source modified: " + str(path))
        self.app_hashes = file_hashes(originals)
        self.external_hashes = {str(path.resolve()): sha(path) for path in [self.args.android_jar, self.args.androidx_annotations]}
        self.report["inputs"].update(unchanged_app_sources_sha256=self.app_hashes, real_android_dependencies_sha256=self.external_hashes, official_compiled_helper_sources=[PARSER_SOURCE, *JAVA_SOURCES], runtime_scope="Pure JVM app methods using actual SDK class definitions; no Android Parcel methods called.")
        dependencies = self.work / "app-dependencies"
        self.run("Original Java app dependencies against real Android SDK", [self.javac, "-cp", classpath(self.args.android_jar, self.args.androidx_annotations), "-d", dependencies, *[SNAPSHOT / name for name in JAVA_SOURCES]])
        self.run("Original DesktopVerseParser helper official", [self.kotlinc, SNAPSHOT / PARSER_SOURCE, *self.options, "-cp", classpath(dependencies, self.stdlib, self.args.android_jar, self.args.androidx_annotations), "-d", dependencies])
        app_official = self.work / "app-official"
        self.run("Original three app targets official", [self.kotlinc, *[SNAPSHOT / name for name in APP_SOURCES.values()], *self.options, "-cp", classpath(dependencies, self.stdlib, self.args.android_jar, self.args.androidx_annotations), "-d", app_official])
        self.run("Fixed original-app Java caller official", [self.javac, "-cp", classpath(app_official, dependencies, self.stdlib, self.args.android_jar), "-d", self.drivers, FIXTURES / "AppCheck.java"])
        deps = [dependencies, self.args.android_jar, self.args.androidx_annotations]
        for name, source in APP_SOURCES.items():
            package = "yuku.alkitab.base.feedback." if name == "PrereleaseVersion" else "yuku.alkitab.base.util."
            owner = package + name
            expression = {"PrereleaseVersion": 'check(yuku.alkitab.base.feedback.PrereleaseVersion.parse("1-dev.2")!!.label == "Dev 2")', "LidToAri": 'check(yuku.alkitab.base.util.LidToAri.lidToAri(31102) == 0x411615)', "ReadingPassage": 'check(yuku.alkitab.base.util.ReadingPassage.parse("", { null }) == null)'}[name]
            consumer = "fun main() { " + expression + " }\n"
            abi = self.verify("official-app-" + name, app_official, deps, owner, consumer, "AppCheck", name)
            self.nullable_consumer("official-app-" + name, app_official, deps, name)
            if self.args.preflight == "official":
                continue
            output = self.work / ("candidate-app-" + name)
            result = self.run("Original " + name + " candidate", [self.compiler, SNAPSHOT / source, "--module-name", "compatibility_calls", "-cp", classpath(dependencies, self.stdlib, self.args.android_jar, self.args.androidx_annotations), "-d", output], allowed=(0, 1))
            if result.returncode:
                self.record("compile/app-" + name, rejected_outcome(result.returncode, result.stderr, list(output.rglob("*.class"))), diagnostic=result.stderr, source=source)
                continue
            self.case("compile/app-" + name)
            try:
                candidate_abi = self.verify("app-" + name, output, deps, owner, consumer, "AppCheck", name)
                self.nullable_consumer("app-" + name, output, deps, name)
                self.record("source_abi/app-" + name, "pass" if source_abi(abi) == source_abi(candidate_abi) else "fail")
            except (AssertionError, TimeoutError) as error:
                self.record("validation/app-" + name, "timeout" if isinstance(error, TimeoutError) else "fail", message=str(error))

    def execute(self):
        try:
            self.prepare()
            self.classpath_parity()
            if not getattr(self.args, "parity_only", False):
                self.calls()
                self.boundaries()
                self.apps()
            if not self.args.preflight:
                if self.frozen != frozen_files() or sha(self.compiler) != self.compiler_sha:
                    raise AssertionError("Compiler, tools or fixtures changed during final run")
                if any(sha(ROOT / path) != value for path, value in self.app_hashes.items()) or any(sha(Path(path)) != value for path, value in self.external_hashes.items()):
                    raise AssertionError("App inputs changed during final run")
                current_libs = file_hashes((self.stdlib.parent).glob("*.jar"))
                if self.reference_hashes != current_libs:
                    raise AssertionError("Reference JARs changed during final run")
                self.case("frozen_inputs", compiler_binary_sha256=self.compiler_sha)
            outcomes = {case["outcome"] for case in self.report["cases"]}
            self.report["status"] = "failed" if outcomes & {"fail", "timeout"} else "incomplete" if "unsupported_language" in outcomes else "completed"
        except Exception as error:
            self.report["status"] = "failed"
            self.record("harness_failure", "timeout" if isinstance(error, TimeoutError) else "fail", message=str(error))
            print(str(error), file=sys.stderr)
        finally:
            self.report["counts"] = dict(Counter(case["outcome"] for case in self.report["cases"]))
            self.report["finished_at"] = datetime.now(timezone.utc).isoformat()
            self.report_path.write_text(json.dumps(self.report, indent=2) + "\n")
            print("Report: " + relative(self.report_path), flush=True)
        return 0 if self.report["status"] == "completed" else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", type=Path, default=ROOT / "target/release/korrust")
    parser.add_argument("--preflight", choices=["official", "candidate"])
    parser.add_argument("--parity-only", action="store_true", help="Run only builtin stdlib JAR/directory parity in preflight mode")
    parser.add_argument("--android-jar", type=Path, required=True)
    parser.add_argument("--androidx-annotations", type=Path, required=True)
    args = parser.parse_args()
    if args.parity_only and not args.preflight:
        parser.error("--parity-only requires --preflight")
    return Harness(args).execute()


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        from reports import generate
        print("HTML report: " + str(generate(ROOT)), flush=True)
