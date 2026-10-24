#!/usr/bin/env python3
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import zipfile
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SEARCH = Path("benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/util/unsignedBinarySearch.kt")

INT_LIST = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/IntArrayListUtil.kt")
JAVA_LIST = Path("benchmarks/sources/androidbible/AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java")

VERSE_PLACEMENT = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/widget/VerseNumberPlacement.kt")

IO_SOURCE = Path("benchmarks/sources/androidbible/AlkitabIo/src/main/java/yuku/alkitab/io/Utf8Decoder.kt")

CHAPTER = Path("benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/Yes2SingleChapterVerses.kt")
CHAPTER_INTERFACE = Path("benchmarks/sources/androidbible/AlkitabModel/src/main/java/yuku/alkitab/model/SingleChapterVerses.kt")

OBJECT = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/RequestCodes.kt")

ENUM = Path("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/actionmode/RibkaEligibility.kt")

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
    run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-io-byte-arrays"
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
        "selection": "Unchanged AndroidBible Utf8Decoder plus independent exhaustive Unicode/byte/thread-local and generic control-flow/object metadata probes; candidate noncapturing Supplier helper classes differ intentionally from official indy and are explicitly validated. AndroidBible unsignedIntBinarySearch, AudioErrorCode, unchanged IntArrayListUtil using the original Java IntArrayList and unchanged VerseNumberPlacement, RibkaEligibility and RequestCodes and Yes2SingleChapterVerses; ordinary final classes with private constructor properties, String arrays and Kotlin interface implementations; candidate-built SingleChapterVerses plus generic interface companions/defaults/anonymous objects, nested Float signatures and simple templates; Java/Kotlin metadata consumers and visibility/nullability probes; bounded named/nested objects with literal constants and Java singleton/nesting/runtime-constant assertions; bounded bare enums with Java runtime, generic-signature, EnumEntries, collection and serialization assertions; JVM arithmetic/control-flow, UTF-16 String/Char/CharArray, subject when, Int += and scoped Java interop probes, invalid and unsupported syntax. Android classes are test-only type stubs; no parcel runtime, upstream Kotlin fixture adapter or whole-app tests.",
        "cases": cases,
    }

    def run(label, args, allowed=(0,), timeout=120, env=None):
        index = len(commands)
        print(label, flush=True)
        entry = {"label": label, "argv": [str(a) for a in args], "cwd": ".", "timeout_seconds": timeout}
        if env:
            entry["environment"] = env
        commands.append(entry)
        try:
            result = subprocess.run(entry["argv"], cwd=ROOT, text=True, capture_output=True, timeout=timeout, env=None if env is None else {**os.environ, **env})
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
        for baseline in [INT_LIST, JAVA_LIST, VERSE_PLACEMENT, ENUM, OBJECT, CHAPTER, CHAPTER_INTERFACE, IO_SOURCE, *[path.relative_to(ROOT) for path in sorted((ROOT / "benchmarks/sources/androidbible/Bintex/src/main/java").rglob("*.java"))]]:
            relative = str(baseline.relative_to("benchmarks/sources/androidbible"))
            if sha(ROOT / baseline) != inventory["androidbible"][relative]:
                raise RuntimeError("AndroidBible source was changed: " + str(baseline))
        report["inputs"] = {
            "io_source": str(IO_SOURCE),
            "io_source_sha256": sha(ROOT / IO_SOURCE),
            "io_fixtures_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "tests/fixtures/io").glob("*")) if p.is_file()},
            "bintex_sources_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "benchmarks/sources/androidbible/Bintex/src/main/java").rglob("*.java"))},
            "yes2_chapter_source": str(CHAPTER),
            "yes2_chapter_source_sha256": sha(ROOT / CHAPTER),
            "chapter_interface_source": str(CHAPTER_INTERFACE),
            "chapter_interface_source_sha256": sha(ROOT / CHAPTER_INTERFACE),
            "model_fixtures_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "tests/fixtures/models").glob("*.kt"))},
            "class_fixtures_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "tests/fixtures/classes").glob("*.kt"))},
            "consumer_fixtures_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "tests/fixtures/consumers").glob("*.kt"))},
            "request_codes_source": str(OBJECT),
            "request_codes_source_sha256": sha(ROOT / OBJECT),
            "object_fixtures_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "tests/fixtures/objects").glob("*.kt"))},
            "ribka_eligibility_source": str(ENUM),
            "ribka_eligibility_source_sha256": sha(ROOT / ENUM),
            "enum_fixtures_sha256": {str(p.relative_to(ROOT)): sha(p) for p in sorted((ROOT / "tests/fixtures/enums").glob("*.kt"))},
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
        run("Rust formatting", [cargo, "fmt", "--check"])
        run("Rust Clippy", [cargo, "clippy", "--all-targets", "--locked", "--offline", "--", "-D", "warnings"])
        run("Report generator tests", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_reports.py"])
        run("Rust tests", [cargo, "test", "--locked", "--offline"])
        cases.append({"id": "rust_frontend", "outcome": "pass", "scope": "Frontend diagnostics and boundary tests; see command log"})
        run("Release build", [cargo, "build", "--release", "--locked", "--offline"])
        compiler = ROOT / "target/release/korrust"
        run("CLI output lifecycle tests", [sys.executable, "-m", "unittest", "discover", "-s", "tests", "-p", "test_cli.py"], env={"KORRUST_COMPILER": str(compiler)})
        cases.append({"id": "cli_output_lifecycle", "outcome": "pass", "scope": "Repeated compilation of edited source matches clean outputs; owned stale classes, failures, collisions and output-path protection tested. No dependency graph or deleted-source discovery; incremental compilation performance unmeasured."})
        owned = [ROOT / "Cargo.toml", ROOT / "Cargo.lock"]
        for folder in ["src", "tests", "tools"]:
            owned.extend(p for p in (ROOT / folder).rglob("*") if p.is_file() and "__pycache__" not in p.parts)
        fingerprints = {str(p.relative_to(ROOT)): sha(p) for p in sorted(owned)}
        report["compiler"] = {
            "version": run("Korrust version", [compiler, "--version"]).stdout.strip(),
            "binary_sha256": sha(compiler),
            "source_files_sha256": fingerprints,
            "classfile_major_versions": [49, 52],
            "kotlin_metadata": True,
            "kotlin_metadata_scope": {"top_level_functions": True, "bounded_classes": True, "bounded_interfaces_companions": True, "bounded_dynamic_objects": True, "enums": False, "constant_only_objects": False},
            "module_metadata": "Per-source module mappings for top-level function facades",
            "incremental_compilation": False,
        }
        official, candidate, drivers, foreign = [work / n for n in ["official", "candidate", "drivers", "foreign"]]
        cp = lambda *paths: os.pathsep.join(str(p) for p in paths)
        jmod = Path(os.environ.get("JAVA_HOME", str(Path(java).resolve().parents[1]))) / "jmods/java.base.jmod"
        jdk_classes = ["java/io/IOException", "java/io/Closeable", "java/lang/AutoCloseable", "java/lang/Exception", "java/lang/Throwable", "java/lang/Object", "java/lang/Deprecated"]
        with zipfile.ZipFile(jmod) as archive:
            for name in jdk_classes:
                output = foreign / (name + ".class")
                output.parent.mkdir(parents=True, exist_ok=True)
                output.write_bytes(archive.read("classes/" + name + ".class"))
        report["inputs"]["jdk_classpath"] = {"jmod": str(jmod), "jmod_sha256": sha(jmod), "classes_sha256": {name: sha(foreign / (name + ".class")) for name in jdk_classes}}
        java_fixtures = sorted((ROOT / "tests/fixtures/interop").rglob("*.java"))
        run("Compile original Java IntArrayList with test-only Android type stubs", [javac, "-d", foreign, ROOT / JAVA_LIST, *java_fixtures, ROOT / "tests/fixtures/io/IoTrace.java", *sorted((ROOT / "benchmarks/sources/androidbible/Bintex/src/main/java").rglob("*.java"))])
        options = report["reference_compiler"]["options"]
        probe = ROOT / "tests/fixtures/Arithmetic.kt"
        strings = ROOT / "tests/fixtures/Strings.kt"
        interop = ROOT / "tests/fixtures/Interop.kt"
        characters = ROOT / "tests/fixtures/Characters.kt"
        class_probe = ROOT / "tests/fixtures/classes/SyntheticChapter.kt"
        run("Compile original Kotlin interface and generic interface fixture", [kotlinc, ROOT / CHAPTER_INTERFACE, ROOT / "tests/fixtures/classes/IndexedText.kt", ROOT / "tests/fixtures/classes/UnsupportedInterfaces.kt", *options, "-d", foreign])
        object_fixtures = sorted((ROOT / "tests/fixtures/objects").glob("*.kt"))
        enum_fixtures = sorted((ROOT / "tests/fixtures/enums").glob("*.kt"))
        run("Compile Kotlin metadata rejection fixture", [kotlinc, ROOT / "tests/fixtures/MetadataForeign.kt", *options, "-d", foreign])
        run("Compile fixtures with official Kotlin", [kotlinc, ROOT / SEARCH, ROOT / AUDIO, ROOT / INT_LIST, ROOT / VERSE_PLACEMENT, strings, probe, interop, characters, ROOT / ENUM, *enum_fixtures, ROOT / OBJECT, *object_fixtures, ROOT / CHAPTER, class_probe, "-cp", foreign, *options, "-d", official])
        for source in [ROOT / SEARCH, ROOT / AUDIO, ROOT / INT_LIST, ROOT / VERSE_PLACEMENT, probe, strings, interop, characters, ROOT / ENUM, *enum_fixtures, ROOT / OBJECT, *object_fixtures, ROOT / CHAPTER, class_probe]:
            run("Compile " + source.name + " with Korrust", [compiler, source, "-cp", foreign, "-d", candidate])
        run("Compile Java callers against official Kotlin", [
            javac, "-cp", cp(official, foreign, stdlib), "-d", drivers,
            ROOT / "tests/fixtures/BinarySearchCheck.java", ROOT / "tests/fixtures/ArithmeticCheck.java", ROOT / "tests/fixtures/StringsCheck.java", ROOT / "tests/fixtures/InteropCheck.java", ROOT / "tests/fixtures/CharactersCheck.java", ROOT / "tests/fixtures/EnumsCheck.java", ROOT / "tests/fixtures/ObjectsCheck.java", ROOT / "tests/fixtures/Yes2Check.java", ROOT / "tests/fixtures/ClassesCheck.java",
        ])
        for name, case_id in [("BinarySearchCheck", "androidbible_unsigned_search"), ("ArithmeticCheck", "jvm_semantics"), ("StringsCheck", "string_semantics"), ("InteropCheck", "java_interop_semantics"), ("CharactersCheck", "character_when_semantics"), ("EnumsCheck", "bare_enum_semantics"), ("ObjectsCheck", "object_constant_semantics"), ("Yes2Check", "androidbible_yes2_chapter"), ("ClassesCheck", "generic_class_semantics")]:
            reference = run("Run official " + name, [java, "-Xverify:all", "-cp", cp(drivers, official, foreign, stdlib), name], timeout=30)
            actual = run("Run Korrust " + name, [java, "-Xverify:all", "-cp", cp(drivers, candidate, foreign, stdlib), name], timeout=30)
            if reference.stdout != actual.stdout:
                raise AssertionError(name + ": reference and candidate differ")
            result = json.loads(actual.stdout)
            cases.append({"id": case_id, "outcome": "pass", "runtime_checks": result["checks"], "result_sha256": result["sha256"],
                          "oracle": "Official Kotlin plus independent Java assertions", "jvm_verification": "all"})
        for name in ["yuku.alkitab.yes2.util.UnsignedBinarySearchKt", "korrust.probes.ArithmeticKt", "korrust.probes.StringsKt", "yuku.alkitab.base.audio.AudioErrorCodeKt", "yuku.alkitab.base.util.IntArrayListUtilKt", "korrust.probes.InteropKt", "korrust.probes.CharactersKt", "yuku.alkitab.base.widget.VerseNumberPlacementKt", "yuku.alkitab.yes2.Yes2SingleChapterVerses", "korrust.probes.classes.TextStore"]:
            reference = run("Inspect official JVM signatures", [javap, "-p", "-s", "-classpath", official, name])
            actual = run("Inspect Korrust JVM signatures", [javap, "-p", "-s", "-classpath", candidate, name])
            if reference.stdout != actual.stdout:
                raise AssertionError("JVM signatures differ: " + name)
            cases.append({"id": "jvm_signatures/" + name, "outcome": "pass", "scope": "javap -p -s; does not assert Kotlin metadata compatibility"})
        enum_classes = ["yuku.alkitab.base.actionmode.RibkaEligibility", *["korrust.probes.enums." + source.stem for source in enum_fixtures]]
        for name in enum_classes:
            reference = run("Inspect official enum generic JVM signatures", [javap, "-p", "-s", "-classpath", official, name])
            actual = run("Inspect Korrust enum generic JVM signatures", [javap, "-p", "-s", "-classpath", candidate, name])
            if reference.stdout != actual.stdout:
                raise AssertionError("Enum generic JVM signatures differ: " + name)
            for output in [official, candidate]:
                if not (output / (name.replace(".", "/") + ".class")).is_file():
                    raise AssertionError("Missing declared enum class: " + name)
                if (output / (name.replace(".", "/") + "Kt.class")).exists():
                    raise AssertionError("Enum unexpectedly emitted a file facade: " + name)
            cases.append({"id": "jvm_signatures/" + name, "outcome": "pass", "scope": "javap -p -s generic superclass, method signatures and descriptors; independent reflection assertions include generic superclass/getEntries. Declared enum class and absence of Kt facade checked; Kotlin metadata and Kotlin consumers unvalidated."})
        def object_inventory(output):
            fixtures = [str(p.relative_to(output).with_suffix("")).replace(os.sep, ".") for p in (output / "korrust/probes/objects").glob("*.class")]
            app = [str(p.relative_to(output).with_suffix("")).replace(os.sep, ".") for p in (output / "yuku/alkitab/base/util").glob("RequestCodes*.class")]
            return sorted(fixtures + app)
        object_classes = object_inventory(official)
        if not object_classes or object_classes != object_inventory(candidate):
            raise AssertionError("Object emitted-class inventories differ")
        for name in object_classes:
            reference = run("Inspect official object JVM constants and signatures", [javap, "-p", "-s", "-constants", "-classpath", official, name])
            actual = run("Inspect Korrust object JVM constants and signatures", [javap, "-p", "-s", "-constants", "-classpath", candidate, name])
            if reference.stdout != actual.stdout:
                raise AssertionError("Object JVM constants/signatures differ: " + name)
            cases.append({"id": "jvm_signatures/" + name, "outcome": "pass", "scope": "javap -p -s -constants; singleton/const descriptors, modifiers and ConstantValue contents. Java reflection separately checks nested naming and declaring/enclosing/declared classes. Kotlin metadata and Kotlin consumers unvalidated."})
        model_reference, model_candidate = work / "model-official", work / "model-candidate"
        model_sources = [ROOT / CHAPTER_INTERFACE, *sorted((ROOT / "tests/fixtures/models").glob("*.kt"))]
        run("Compile model interface fixtures with official Kotlin", [kotlinc, *model_sources, *options, "-d", model_reference])
        for source in model_sources:
            run("Compile model interface " + source.name + " with Korrust", [compiler, source, "-d", model_candidate])
        model_drivers = work / "model-drivers"
        run("Compile independent model Java callers against official", [javac, "-cp", cp(model_reference, stdlib), "-d", model_drivers, ROOT / "tests/fixtures/InterfaceAssertions.java", ROOT / "tests/fixtures/ModelCheck.java", ROOT / "tests/fixtures/InterfaceCheck.java"])
        for name, case_id in [("ModelCheck", "androidbible_model_interface"), ("InterfaceCheck", "generic_interface_semantics")]:
            reference = run("Run official model " + name, [java, "-Xverify:all", "-cp", cp(model_drivers, model_reference, stdlib), name])
            actual = run("Run Korrust model " + name, [java, "-Xverify:all", "-cp", cp(model_drivers, model_candidate, stdlib), name])
            if reference.stdout != actual.stdout:
                raise AssertionError("Model Java observations differ: " + name)
            result = json.loads(actual.stdout)
            cases.append({"id": case_id, "outcome": "pass", "runtime_checks": result["checks"], "result_sha256": result["sha256"], "scope": "Official-compiled Java assertions, JVM verification all; companion/private-array initialization and aliasing, anonymous errors, default helpers including null receivers/override bypass, nested Float descriptors and simple templates."})
        model_classes = sorted(str(path.relative_to(model_reference).with_suffix("")).replace(os.sep, ".") for path in model_reference.rglob("*.class"))
        actual_model_classes = sorted(str(path.relative_to(model_candidate).with_suffix("")).replace(os.sep, ".") for path in model_candidate.rglob("*.class"))
        if model_classes != actual_model_classes:
            raise AssertionError("Model interface emitted class inventories differ")
        for name in model_classes:
            reference = run("Inspect official model interface ABI", [javap, "-p", "-s", "-classpath", model_reference, name])
            actual = run("Inspect candidate model interface ABI", [javap, "-p", "-s", "-classpath", model_candidate, name])
            if reference.stdout != actual.stdout:
                raise AssertionError("Model interface ABI differs: " + name)
            cases.append({"id": "model_jvm_signatures/" + name, "outcome": "pass", "scope": "Complete source-interface class inventory including Companion/anonymous/DefaultImpls/nested-interface helper descriptors; javap -p -s."})
        model_consumer_results = []
        for provider, output in [("official", model_reference), ("candidate", model_candidate)]:
            destination = work / (provider + "-model-consumer")
            run("Compile Kotlin model consumer against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers/ModelConsumer.kt", "-cp", output, *options, "-d", destination])
            model_consumer_results.append(run("Run Kotlin model consumer against " + provider, [java, "-Xverify:all", "-cp", cp(destination, output, stdlib), "korrust.consumers.ModelConsumerKt"]).stdout)
            rejected = run("Reject Kotlin private companion access against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers/ModelPrivateAccess.kt", "-cp", output, *options, "-d", work / (provider + "-model-private")], allowed=(1,))
            if "error:" not in rejected.stderr or "private" not in rejected.stderr.lower():
                raise AssertionError("Private companion metadata was not enforced")
            dependent = work / (provider + "-yes2-with-model")
            if provider == "official":
                run("Compile official Yes2 against official model interface", [kotlinc, ROOT / CHAPTER, "-cp", output, *options, "-d", dependent])
            else:
                run("Compile Korrust Yes2 against candidate model interface", [compiler, ROOT / CHAPTER, "-cp", output, "-d", dependent])
            observed = run("Run Yes2Check against " + provider + " source-built model", [java, "-Xverify:all", "-cp", cp(drivers, dependent, output, stdlib), "Yes2Check"])
            if provider == "official":
                dependent_reference = observed.stdout
            elif observed.stdout != dependent_reference:
                raise AssertionError("Yes2 with candidate-built interface differs")
        if model_consumer_results[0] != model_consumer_results[1]:
            raise AssertionError("Kotlin model metadata consumers differ")
        cases.append({"id": "kotlin_metadata/ModelConsumer", "outcome": "pass", "scope": "Official Kotlin consumes each emitted interface/companion/anonymous/nested-Float surface, executes default methods, and rejects private companion access."})
        cases.append({"id": "androidbible_yes2_candidate_model", "outcome": "pass", "runtime_checks": json.loads(dependent_reference)["checks"], "scope": "Korrust Yes2 is compiled against Korrust original SingleChapterVerses classpath; official-compiled Java caller results match entirely official pair under JVM verification all."})
        from check_androidbible_io import source_abi
        io_reference, io_candidate = work / "io-official", work / "io-candidate"
        io_sources = [ROOT / IO_SOURCE, *sorted((ROOT / "tests/fixtures/io").glob("*.kt"))]
        run("Compile Io and generic byte fixtures with official Kotlin", [kotlinc, *io_sources, "-cp", foreign, *options, "-d", io_reference])
        for source in io_sources:
            run("Compile Io fixture " + source.name, [compiler, source, "-cp", foreign, "-d", io_candidate])
        io_drivers = work / "io-drivers"
        run("Compile independent Io/byte Java callers against official Kotlin", [javac, "-cp", cp(io_reference, foreign, stdlib), "-d", io_drivers, ROOT / "tests/fixtures/InterfaceAssertions.java", *sorted((ROOT / "tests/fixtures/io").glob("*Check.java"))])
        for name, case_id in [("IoCheck", "androidbible_io_semantics"), ("BytesCheck", "generic_byte_array_semantics")]:
            results = [run("Run " + name + " against " + provider, [java, "-Xverify:all", "-cp", cp(io_drivers, output, foreign, stdlib), name], timeout=60).stdout for provider, output in [("official", io_reference), ("candidate", io_candidate)]]
            if results[0] != results[1]:
                raise AssertionError("Io observations differ: " + name)
            result = json.loads(results[0])
            cases.append({"id": case_id, "outcome": "pass", "runtime_checks": result["checks"], "result_sha256": result["sha256"], "scope": "Official-compiled Java callers, JVM verification all; Unicode scalars, bytes, buffer/concurrency, lazy suppliers, control-flow/definite assignment and observable evaluation order."})
        supplier_fields = {"yuku.alkitab.io.Utf8Decoder": {"byteBuffer": "byte", "charBuffer": "char"}, "korrust.probes.io.ByteWorkbench": {"octets": "byte", "letters": "char"}, "korrust.probes.io.HiddenBuffers": {"state": "char"}, "korrust.probes.io.LazyScratch": {"failedBytes": "byte", "failedChars": "char"}}
        reference_classes = {path.relative_to(io_reference).with_suffix("").as_posix().replace("/", ".") for path in io_reference.rglob("*.class")}
        candidate_classes = {path.relative_to(io_candidate).with_suffix("").as_posix().replace("/", ".") for path in io_candidate.rglob("*.class")}
        expected_helpers = {owner + "$" + field + "$Supplier" for owner, fields in supplier_fields.items() for field in fields}
        if candidate_classes != reference_classes | expected_helpers:
            raise AssertionError("Io source/synthetic class inventories differ")
        for name in sorted(reference_classes):
            expected = run("Inspect official Io source ABI", [javap, "-p", "-s", "-classpath", io_reference, name]).stdout
            actual = run("Inspect candidate Io source ABI", [javap, "-p", "-s", "-classpath", io_candidate, name]).stdout
            if source_abi(expected, supplier_fields.get(name, {})) != source_abi(actual, supplier_fields.get(name, {})):
                raise AssertionError("Io source ABI differs: " + name)
            cases.append({"id": "io_jvm_signatures/" + name, "outcome": "pass", "scope": "All source-defined JVM descriptors/generic fields/throws/modifiers; only exact derived synthetic Supplier accessors and member order normalized. Separate synthetic helper contract verified."})
        run("Verify candidate synthetic Supplier implementation contracts", [java, "-Xverify:all", "-cp", cp(io_drivers, io_candidate, foreign, stdlib), "SupplierCheck", *supplier_fields])
        cases.append({"id": "io_supplier_lowering", "outcome": "pass", "synthetic_classes": sorted(expected_helpers), "scope": "Exactly derived synthetic noncapturing Suppliers/accessors; fresh allocation on Supplier.get, thread-local cache identity and deferred negative allocations tested. Official indy and candidate synthetic inventories differ legitimately."})
        for consumer, friend in [("IoMetadataConsumer", False), ("IoInternalConsumer", True)]:
            results = []
            for provider, output in [("official", io_reference), ("candidate", io_candidate)]:
                destination = work / (provider + "-" + consumer)
                run("Compile " + consumer + " against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers" / (consumer + ".kt"), "-cp", cp(output, foreign), *options, *(["-Xfriend-paths=" + str(output)] if friend else []), "-d", destination])
                results.append(run("Run " + consumer + " against " + provider, [java, "-Xverify:all", "-cp", cp(destination, output, foreign, stdlib), "korrust.consumers." + consumer + "Kt"]).stdout)
            if results[0] != results[1]:
                raise AssertionError("Io Kotlin consumer differs")
            cases.append({"id": "kotlin_metadata/" + consumer, "outcome": "pass", "scope": "Official Kotlin compiles/runs Byte, ByteArray, overload/static object or internal-object friend consumer against each provider."})
        for consumer, marker in [("IoInternalConsumer", "internal"), ("IoPrivateAccess", "private"), ("IoNullAccess", "null")]:
            for provider, output in [("official", io_reference), ("candidate", io_candidate)]:
                rejected = run("Io Kotlin rejection " + consumer + " against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers" / (consumer + ".kt"), "-cp", cp(output, foreign), *options, "-d", work / (provider + "-reject-" + consumer)], allowed=(1,))
                if "error:" not in rejected.stderr or marker not in rejected.stderr.lower():
                    raise AssertionError("Unexpected Io Kotlin rejection")
            cases.append({"id": "kotlin_metadata/" + consumer + "_rejection", "outcome": "pass", "scope": "Official Kotlin enforces internal/private/non-null object metadata against both providers."})
        for consumer, friend in [("PublicMetadataConsumer", False), ("FriendMetadataConsumer", True)]:
            results = []
            for provider, output in [("official", official), ("candidate", candidate)]:
                consumer_output = work / "consumers" / provider / consumer
                friend_options = ["-Xfriend-paths=" + str(output)] if friend else []
                run("Compile " + consumer + " against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers" / (consumer + ".kt"), "-cp", cp(output, foreign), *options, *friend_options, "-d", consumer_output])
                results.append(run("Run " + consumer + " against " + provider, [java, "-Xverify:all", "-cp", cp(consumer_output, output, foreign, stdlib), "korrust.consumers." + consumer + "Kt"]).stdout)
            if results[0] != results[1]:
                raise AssertionError("Kotlin metadata consumer results differ: " + consumer)
            cases.append({"id": "kotlin_metadata/" + consumer, "outcome": "pass", "scope": "Official Kotlin separately compiles and runs consumer against each provider; public multi-facade imports or internal class/function friend-path access; JVM verification all."})
        for consumer, friend, marker in [("InternalClassAccess", False, "internal"), ("PrivateFieldAccess", True, "private"), ("PublicNullAccess", False, "null"), ("InternalFunctionAccess", False, "internal")]:
            for provider, output in [("official", official), ("candidate", candidate)]:
                friend_options = ["-Xfriend-paths=" + str(output)] if friend else []
                rejected = run("Kotlin metadata rejection " + consumer + " against " + provider, [kotlinc, ROOT / "tests/fixtures/consumers" / (consumer + ".kt"), "-cp", cp(output, foreign), *options, *friend_options, "-d", work / "consumer-rejections" / provider / consumer], allowed=(1,))
                if "error:" not in rejected.stderr or marker not in rejected.stderr.lower():
                    raise AssertionError("Unexpected Kotlin metadata rejection: " + consumer)
            cases.append({"id": "kotlin_metadata/" + consumer, "outcome": "pass", "scope": "Official Kotlin rejects access/nullability misuse against both providers with checked diagnostic category."})
        for fixture in json.loads((ROOT / "tests/diagnostics.json").read_text()):
            source = work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"])
            fixture_cp = ["-cp", str(foreign)] if fixture.get("classpath") else []
            reference = run("Reference rejection: " + fixture["id"], [kotlinc, source, *fixture_cp, *options, "-d", work / "negative-reference"], allowed=(1,))
            output = work / ("negative-" + fixture["id"])
            actual = run("Korrust rejection: " + fixture["id"], [compiler, source, *fixture_cp, "-d", output], allowed=(1,))
            if "error:" not in reference.stderr or fixture["message"] not in actual.stderr:
                raise AssertionError("Unexpected diagnostic: " + fixture["id"])
            if output.exists():
                raise AssertionError("Rejected input produced output")
            cases.append({"id": "diagnostics/" + fixture["id"], "outcome": "pass",
                          "scope": "Both compilers reject; Korrust diagnostic category checked. Exact official error wording is not compared."})
        def supported_regression(fixture, source, dependencies):
            regression = fixture["supported_regression"]
            directory = work / "supported-regressions" / fixture["id"]
            directory.mkdir(parents=True)
            java_source = directory / "RegressionCheck.java"
            java_source.write_text("public class RegressionCheck { public static void main(String[] args) {\n" + regression["java_assertions"] + "\nSystem.out.println(\"ok\"); } }\n")
            outputs = {}
            for provider, tool in [("official", kotlinc), ("candidate", compiler)]:
                output = directory / provider
                args = [tool, source, *(["-cp", cp(*dependencies)] if dependencies else []), *([] if provider == "candidate" else options), "-d", output]
                run("Compile supported regression " + fixture["id"] + " with " + provider, args)
                outputs[provider] = output
            driver_output = directory / "drivers"
            run("Compile Java regression caller " + fixture["id"] + " against official Kotlin", [javac, "-cp", cp(outputs["official"], *dependencies, stdlib), "-d", driver_output, java_source])
            for provider, output in outputs.items():
                result = run("Run supported regression " + fixture["id"] + " against " + provider, [java, "-Xverify:all", "-cp", cp(driver_output, output, *dependencies, stdlib), "RegressionCheck"])
                if result.stdout != "ok\n":
                    raise AssertionError("Unexpected regression output: " + fixture["id"])
            signatures = [run("Inspect supported regression ABI " + fixture["id"] + " from " + provider, [javap, "-p", "-s", "-classpath", output, regression["facade"]]).stdout for provider, output in outputs.items()]
            if signatures[0] != signatures[1]:
                raise AssertionError("Regression JVM signatures differ: " + fixture["id"])
            cases.append({"id": "regressions/" + fixture["id"], "outcome": "pass", "scope": "Former unsupported fixture now compiles with both providers; Java caller compiled against official output runs independent assertions against each provider under JVM verification all; javap signatures match.", "source_sha256": sha(source), "driver_sha256": sha(java_source)})

        for fixture in json.loads((ROOT / "tests/unsupported.json").read_text()):
            source = work / (fixture["id"] + ".kt")
            source.write_text(fixture["source"])
            fixture_cp = ["-cp", str(foreign)] if fixture.get("classpath") else []
            if fixture.get("supported_regression"):
                supported_regression(fixture, source, [foreign] if fixture.get("classpath") else [])
                continue
            run("Reference acceptance: " + fixture["id"], [kotlinc, source, *fixture_cp, *options, "-d", work / "unsupported-reference"])
            actual = run("Explicit unsupported rejection: " + fixture["id"], [compiler, source, *fixture_cp, "-d", work / ("unsupported-" + fixture["id"])], allowed=(1,))
            if fixture.get("message") and fixture["message"] not in actual.stderr:
                raise AssertionError("Unexpected unsupported diagnostic: " + fixture["id"])
            if (work / ("unsupported-" + fixture["id"])).exists():
                raise AssertionError("Unsupported input produced output")
            cases.append({"id": "unsupported/" + fixture["id"], "outcome": "unsupported_language",
                          "diagnostic": actual.stderr.strip(), **({"feature_name": fixture["feature_name"]} if "feature_name" in fixture else {})})
        classpath_probe = work / "ClasspathProbe.kt"
        classpath_probe.write_text("import yuku.alkitab.util.IntArrayList\nfun f(value: IntArrayList): Int = value.size()")
        empty_classpath = work / "empty-classpath"
        empty_classpath.mkdir()
        classpath_driver = work / "ClasspathCheck.java"
        classpath_driver.write_text("public class ClasspathCheck { public static void main(String[] args) { yuku.alkitab.util.IntArrayList value = new yuku.alkitab.util.IntArrayList(); for (int i = 0; i != 6; ++i) { if (ClasspathProbeKt.f(value) != i) throw new AssertionError(\"size \" + i); value.add(i); } System.out.println(\"classpath:ok\"); } }\n")
        classpath_inputs = [("directory", foreign)]
        for label, compression in [("stored_jar", zipfile.ZIP_STORED), ("deflated_jar", zipfile.ZIP_DEFLATED)]:
            archive = work / (label + ".jar")
            with zipfile.ZipFile(archive, "w", compression=compression) as jar:
                for path in sorted(foreign.rglob("*")):
                    if path.is_file():
                        jar.write(path, path.relative_to(foreign).as_posix())
            classpath_inputs.append((label, archive))
        for label, entry in classpath_inputs:
            destinations = {}
            for provider, tool in [("official", kotlinc), ("candidate", compiler)]:
                output = work / ("classpath-" + label + "-" + provider)
                run("Compile classpath runtime probe " + label + " with " + provider, [tool, classpath_probe, "--classpath" if provider == "candidate" else "-cp", cp(empty_classpath, entry), *([] if provider == "candidate" else options), "-d", output])
                destinations[provider] = output
            driver_output = work / ("classpath-" + label + "-drivers")
            run("Compile classpath Java caller against official " + label, [javac, "-cp", cp(destinations["official"], entry, stdlib), "-d", driver_output, classpath_driver])
            for provider, output in destinations.items():
                result = run("Run classpath probe " + label + " against " + provider, [java, "-Xverify:all", "-cp", cp(driver_output, output, entry, stdlib), "ClasspathCheck"])
                if result.stdout != "classpath:ok\n":
                    raise AssertionError("Classpath runtime probe failed: " + label)
            cases.append({"id": "interop_classpath/" + label, "outcome": "pass", "scope": "Both compilers resolve the original Java IntArrayList through directory or stored/deflated JAR classpaths after an empty entry; an official-compiled Java caller independently checks six sizes against both outputs with JVM verification all.", "classpath_entry_sha256": sha(entry) if entry.is_file() else None})
        malformed = work / "malformed-classpath.jar"
        malformed.write_bytes(b"This is not a ZIP archive")
        for entry, marker in [(malformed, "zip"), (work / "absent-classpath.jar", "existing directory")]:
            output = work / ("invalid-classpath-" + entry.name)
            rejected = run("Reject malformed or missing classpath entry", [compiler, classpath_probe, "-cp", entry, "-d", output], allowed=(1,))
            if marker not in rejected.stderr.lower() or "Classpath" not in rejected.stderr or output.exists():
                raise AssertionError("Classpath configuration rejection was not explicit")
        cases.append({"id": "interop_diagnostics/classpath_configuration", "outcome": "pass", "scope": "--classpath alias and ordered search through an empty entry; directory and real stored/deflated JAR runtime probes above; malformed ZIP and nonexistent JAR entries explicitly reject without producing output."})
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
            if fixture.get("supported_regression"):
                supported_regression(fixture, source, [foreign])
                continue
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
                features.append({"name": case.get("feature_name", case["id"].split("/", 1)[1]), "status": "unsupported", "evidence_cases": [case["id"]]})
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
