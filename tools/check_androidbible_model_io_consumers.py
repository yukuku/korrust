#!/usr/bin/env python3
"""Downstream JVM and Kotlin-metadata verification of fresh Model/Io AAR pairs."""

import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import zipfile

from check_androidbible_io import SUPPLIER_CLASSES, source_abi
from check_androidbible_module import ROOT, SNAPSHOT, classpath, relative, sha

if not __debug__:
    raise RuntimeError(
        "Optimized Python (-O) is unsupported: verification assertions must remain enabled"
    )

MODULES = ["AlkitabModel", "AlkitabIo"]
FIXTURES = ROOT / "tests/fixtures/model_io_consumers"
DOWNSTREAM = (
    SNAPSHOT / "AlkitabYes2/src/main/java/yuku/alkitab/yes2/Yes2SingleChapterVerses.kt"
)
LEGACY = [
    ROOT / "tests/fixtures" / name
    for name in [
        "InterfaceAssertions.java",
        "ModelCheck.java",
        "ModuleClassVerification.java",
        "io/IoCheck.java",
        "io/SupplierCheck.java",
    ]
]
NEGATIVE = [
    ("ModelPrivateAccess.kt", ["INVISIBLE_MEMBER", "INVISIBLE_REFERENCE"]),
    ("IoPrivateAccess.kt", ["INVISIBLE_MEMBER", "INVISIBLE_REFERENCE"]),
    ("IoNullAccess.kt", ["NULL_FOR_NONNULL_TYPE"]),
]


def class_hashes(data):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        return {
            name: hashlib.sha256(archive.read(name)).hexdigest()
            for name in sorted(archive.namelist())
            if name.endswith(".class")
        }


class Verification:
    def __init__(self, args):
        self.args = args
        now = datetime.now(timezone.utc)
        self.ident = (
            now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-model-io-downstream-consumers"
        )
        self.work = ROOT / "target/checks" / self.ident
        self.evidence = ROOT / "reports/correctness" / self.ident
        self.work.mkdir(parents=True)
        self.evidence.mkdir(parents=True)
        self.path = self.evidence.with_suffix(".json")
        self.builds = {
            provider: json.loads(path.read_text())
            for provider, path in [
                ("official", args.official_json),
                ("candidate", args.candidate_json),
            ]
        }
        self.pin = json.loads((ROOT / "references/compiler.json").read_text())
        kotlin = ROOT / self.pin["path"]
        self.kotlinc, self.stdlib = (
            kotlin / "bin/kotlinc",
            kotlin / "lib/kotlin-stdlib.jar",
        )
        self.jdk = Path(os.environ["JAVA_HOME"]) / "bin"
        candidate = self.builds["candidate"]
        files = [
            Path(__file__),
            ROOT / "tools/check_androidbible_io.py",
            ROOT / "tools/check_androidbible_module.py",
            *FIXTURES.iterdir(),
            *LEGACY,
            DOWNSTREAM,
            ROOT / "references/compiler.json",
            ROOT / "benchmarks/sources.json",
            ROOT / "benchmarks/source-files.sha256.json",
            kotlin / "lib/kotlin-compiler.jar",
            self.stdlib,
            args.official_json,
            args.candidate_json,
            Path(candidate["compiler"]),
        ]
        files.extend(ROOT / "tests/fixtures/consumers" / name for name, _ in NEGATIVE)
        for build in self.builds.values():
            for name in build["source_files_sha256"]:
                files.append(SNAPSHOT / name)
            for module in MODULES:
                info = build["modules"][module]
                files.append(Path(info["aar"]))
                files.extend(Path(p) for p in info["classpath"])
        if args.build_evidence:
            files.append(args.build_evidence)
        self.frozen = {str(path.resolve()): sha(path) for path in sorted(set(files))}
        self.report = dict(
            schema_version=1,
            kind="correctness",
            created_at=now.isoformat(),
            status="running",
            compiler={
                "binary": relative(Path(candidate["compiler"])),
                "sha256": candidate["compiler_sha256"],
                "identity_source": "Fresh candidate Gradle build receipt; this runner does not rebuild production classes.",
            },
            reference_compiler={
                "version": self.pin["version"],
                "archive_sha256": self.pin["archive_sha256"],
                "compiler_jar_sha256": sha(kotlin / "lib/kotlin-compiler.jar"),
            },
            inputs={
                "files_sha256": {relative(Path(p)): h for p, h in self.frozen.items()},
                "build_receipts": {
                    p: {"path": relative(path), "sha256": sha(path)}
                    for p, path in [
                        ("official", args.official_json),
                        ("candidate", args.candidate_json),
                    ]
                },
                "build_evidence": (
                    relative(args.build_evidence) if args.build_evidence else None
                ),
                "downstream_original_source": relative(DOWNSTREAM),
                "androidbible_revision": json.loads(
                    (ROOT / "benchmarks/sources.json").read_text()
                )["sources"]["androidbible"]["commit"],
            },
            reproduction_commands=[
                [
                    sys.executable,
                    "tools/check_androidbible_model_io_consumers.py",
                    "--official-json",
                    str(args.official_json),
                    "--candidate-json",
                    str(args.candidate_json),
                    *(
                        ["--build-evidence", str(args.build_evidence)]
                        if args.build_evidence
                        else []
                    ),
                ]
            ],
            commands=[],
            cases=[],
            evidence=[],
            scope="Fresh AlkitabModel/AlkitabIo AAR downstream consumption, with fixed official Java and official-Kotlin consumer bytecode, unchanged real downstream Yes2SingleChapterVerses source and independently asserted behavior.",
            limitations=[
                "Existing module coverage is revalidated; no compiler feature or newly supported module is counted. Production Gradle/source-ownership/original-unit-test evidence is linked separately rather than duplicated.",
                "Only classes.jar from each fresh AAR supplies selected production classes. Shared genuine external dependencies are checked disjoint from every selected class, and runtime class origins are asserted for both module APIs and the unchanged downstream source.",
                "Source-facing JVM ABI comparison reuses the existing module checker, excluding only the two exact synthetic Supplier access bridges. The two legitimate candidate Supplier helpers are independently verified through the existing SupplierCheck.",
                "The Kotlin consumer is compiled by pinned official Kotlin twice, against each provider pair; the Java caller compiled against the official pair is fixed. All four consumer/provider combinations execute with -Xverify:all. The unchanged internal Yes2 implementation is compiled in the consumer module, without changing its visibility or friend access.",
                "The Android SDK jar provides linkage types for original Parcelable declarations; no Android framework method implementation or device/emulator behavior is tested. No D8, complete app, incremental compilation or performance claim.",
            ],
        )

    def run(self, label, argv, allowed=(0,), timeout=300):
        print(label, flush=True)
        command = list(map(str, argv))
        result = subprocess.run(
            command, cwd=ROOT, capture_output=True, text=True, timeout=timeout
        )
        entry = {
            "label": label,
            "argv": command,
            "cwd": ".",
            "exit_code": result.returncode,
        }
        for stream in ["stdout", "stderr"]:
            path = self.evidence / f'{len(self.report["commands"]):03d}-{stream}.txt'
            path.write_text(getattr(result, stream))
            entry[stream] = relative(path)
            self.report["evidence"].append(relative(path))
        self.report["commands"].append(entry)
        if result.returncode not in allowed:
            raise AssertionError(label + "\n" + result.stdout + result.stderr)
        return result

    def case(self, name, valid=True, **fields):
        self.report["cases"].append(
            dict(id=name, outcome="pass" if valid else "correctness_failure", **fields)
        )
        if not valid:
            raise AssertionError(name)

    def execute(self):
        inventory = json.loads(
            (ROOT / "benchmarks/source-files.sha256.json").read_text()
        )["androidbible"]
        targets = set()
        for provider, build in self.builds.items():
            expected_mode = "official" if provider == "official" else "korrust"
            expected_compiler = (
                "official Kotlin Gradle plugin" if provider == "official" else "korrust"
            )
            assert (
                build["mode"] == expected_mode
            ), "Build receipt provider mode mismatch"
            assert (
                build["production_kotlin_compiler"] == expected_compiler
            ), "Build receipt production compiler mismatch"
            assert all(
                build["modules"][m]["production_kotlin_compiler"] == expected_compiler
                for m in MODULES
            ), "Per-module production compiler mismatch"
            assert (
                build["status"] == "passed"
                and build["returncode"] == 0
                and not build["production_kotlin_fallback"]
            )
            assert (
                sorted(build["selected_modules"]) == sorted(MODULES)
                and build["sources_unchanged"]
            )
            assert (
                build["source_files_sha256"] == build["source_files_sha256_after_build"]
            )
            assert build["resolved_kotlin_compiler_version"] == self.pin["version"]
            for name, digest in build["source_files_sha256"].items():
                assert inventory[name] == digest == sha(SNAPSHOT / name), name
            for module in MODULES:
                info = build["modules"][module]
                assert (
                    info["sources_unchanged"] and not info["production_kotlin_fallback"]
                )
                targets.add(info["options"]["kotlin_jvm_target"])
        assert len(targets) == 1
        assert (
            sha(self.builds["candidate"]["compiler"])
            == self.builds["candidate"]["compiler_sha256"]
        )
        self.options = [
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            targets.pop(),
            "-module-name",
            "nov25_downstream",
        ]
        self.report["reference_compiler"]["consumer_options"] = self.options
        self.case("fresh_completed_build_inputs", selected_modules=MODULES)
        self.jars, self.classes = {}, {}
        artifacts = {}
        for provider, build in self.builds.items():
            self.jars[provider], self.classes[provider], artifacts[provider] = (
                {},
                {},
                {},
            )
            for module in MODULES:
                aar = Path(build["modules"][module]["aar"])
                with zipfile.ZipFile(aar) as archive:
                    assert "AndroidManifest.xml" in archive.namelist()
                    data = archive.read("classes.jar")
                jar = self.work / (provider + "-" + module + ".jar")
                jar.write_bytes(data)
                self.jars[provider][module] = jar
                self.classes[provider][module] = class_hashes(data)
                artifacts[provider][module] = {
                    "aar": relative(aar),
                    "aar_sha256": sha(aar),
                    "classes_jar": relative(jar),
                    "classes_jar_sha256": sha(jar),
                    "classes_sha256": self.classes[provider][module],
                }
        self.report["inputs"]["artifacts"] = artifacts
        for module in MODULES:
            official = set(self.classes["official"][module])
            candidate = set(self.classes["candidate"][module])
            extras = SUPPLIER_CLASSES if module == "AlkitabIo" else set()
            self.case(
                "aar_inventory/" + module,
                candidate == official | extras,
                official_classes=len(official),
                candidate_classes=len(candidate),
                legitimate_candidate_extras=sorted(extras),
            )
        selected = set().union(
            *(
                set(classes)
                for modules in self.classes.values()
                for classes in modules.values()
            )
        )
        external, maps, omitted, shadowed = {}, {}, {}, {}
        for provider, build in self.builds.items():
            (
                external[provider],
                maps[provider],
                omitted[provider],
                shadowed[provider],
            ) = ([], {}, [], [])
            paths = list(
                dict.fromkeys(
                    p
                    for module in MODULES
                    for p in build["modules"][module]["classpath"]
                )
            )
            for name in paths:
                path = Path(name)
                classes = class_hashes(path.read_bytes())
                if selected.intersection(classes):
                    omitted[provider].append(
                        {
                            "path": relative(path),
                            "sha256": sha(path),
                            "reason": "Selected module compile output replaced by its freshly packaged AAR classes.jar",
                        }
                    )
                    continue
                external[provider].append(path)
                for owner, digest in classes.items():
                    if owner in maps[provider]:
                        shadowed[provider].append(
                            {
                                "class": owner,
                                "path": relative(path),
                                "sha256": digest,
                                "effective_sha256": maps[provider][owner],
                            }
                        )
                    else:
                        maps[provider][owner] = digest
        self.case(
            "shared_external_dependencies",
            maps["official"] == maps["candidate"],
            selected_classes_disjoint=True,
        )
        self.shared = external["official"]
        self.report["inputs"]["shared_classpath"] = [relative(p) for p in self.shared]
        self.report["inputs"]["omitted_selected_compile_jars"] = omitted
        self.report["inputs"]["shadowed_external_class_entries"] = shadowed
        self.report["inputs"]["external_class_map_sha256"] = hashlib.sha256(
            json.dumps(maps["official"], sort_keys=True).encode()
        ).hexdigest()
        for module in MODULES:
            for owner in self.classes["official"][module]:
                name = owner.removesuffix(".class").replace("/", ".")
                views = {}
                for provider in self.builds:
                    result = self.run(
                        "Source JVM ABI " + provider + "/" + name,
                        [
                            self.jdk / "javap",
                            "-p",
                            "-s",
                            "-classpath",
                            self.jars[provider][module],
                            name,
                        ],
                    )
                    views[provider] = source_abi(
                        result.stdout,
                        None if name == "yuku.alkitab.io.Utf8Decoder" else {},
                    )
                self.case(
                    "jvm_source_abi/" + name, views["official"] == views["candidate"]
                )
        consumers = {}
        for provider in self.builds:
            output = self.work / (provider + "-kotlin-consumer")
            self.run(
                "Official Kotlin downstream consumer compile against " + provider,
                [
                    self.kotlinc,
                    DOWNSTREAM,
                    FIXTURES / "Consumer.kt",
                    *self.options,
                    "-cp",
                    classpath(*self.jars[provider].values(), *self.shared),
                    "-d",
                    output,
                ],
            )
            consumers[provider] = output
            self.case("kotlin_metadata_consumer_compile/" + provider)
        drivers = self.work / "fixed-official-java-callers"
        self.run(
            "Compile fixed Java callers against official AARs and official Kotlin consumer",
            [
                self.jdk / "javac",
                "-cp",
                classpath(
                    consumers["official"], *self.jars["official"].values(), *self.shared
                ),
                "-d",
                drivers,
                *LEGACY,
                FIXTURES / "BridgeCheck.java",
            ],
        )
        self.case("fixed_official_java_caller_compile")
        observations = {}
        for name in ["ModelCheck", "IoCheck"]:
            observations[name] = {}
            for provider in self.builds:
                result = self.run(
                    "Fixed Java " + name + "/" + provider,
                    [
                        self.jdk / "java",
                        "-Xverify:all",
                        "-cp",
                        classpath(drivers, *self.jars[provider].values(), *self.shared),
                        name,
                    ],
                )
                observed = json.loads(result.stdout)
                assert observed["checks"] > 0 and len(observed["sha256"]) == 64
                observations[name][provider] = observed
                self.case(
                    "fixed_java_runtime/" + name + "/" + provider,
                    runtime_assertions=observed["checks"],
                    observation_sha256=observed["sha256"],
                )
            assert observations[name]["official"] == observations[name]["candidate"]
        self.report["runtime_observations"] = observations
        for consumer, output in consumers.items():
            for provider in self.builds:
                result = self.run(
                    "Fixed Java and Kotlin downstream runtime "
                    + consumer
                    + "/"
                    + provider,
                    [
                        self.jdk / "java",
                        "-Xverify:all",
                        "-cp",
                        classpath(
                            drivers, output, *self.jars[provider].values(), *self.shared
                        ),
                        "nov25.moduleconsumer.BridgeCheck",
                        self.jars[provider]["AlkitabModel"],
                        self.jars[provider]["AlkitabIo"],
                        output,
                    ],
                )
                observed = json.loads(result.stdout)
                self.case(
                    "downstream_pair_runtime/" + consumer + "/" + provider,
                    observed["marker"] == "model_io_downstream"
                    and observed["checks"] > 0,
                    runtime_assertions=observed["checks"],
                    class_origins_verified=True,
                )
        for provider in self.builds:
            for module in MODULES:
                names = [
                    name.removesuffix(".class").replace("/", ".")
                    for name in self.classes[provider][module]
                ]
                self.run(
                    "All-class JVM linkage " + provider + "/" + module,
                    [
                        self.jdk / "java",
                        "-Xverify:all",
                        "-cp",
                        classpath(drivers, *self.jars[provider].values(), *self.shared),
                        "ModuleClassVerification",
                        *names,
                    ],
                )
                self.case(
                    "all_class_linkage/" + provider + "/" + module, classes=len(names)
                )
        self.run(
            "Verify candidate synthetic Supplier lowering",
            [
                self.jdk / "java",
                "-Xverify:all",
                "-cp",
                classpath(drivers, *self.jars["candidate"].values(), *self.shared),
                "SupplierCheck",
                "yuku.alkitab.io.Utf8Decoder",
            ],
        )
        self.case("candidate_supplier_lowering")
        for provider in self.builds:
            for name, diagnostics in NEGATIVE:
                result = self.run(
                    "Kotlin metadata rejection " + provider + "/" + name,
                    [
                        self.kotlinc,
                        ROOT / "tests/fixtures/consumers" / name,
                        *self.options,
                        "-Xrender-internal-diagnostic-names",
                        "-cp",
                        classpath(*self.jars[provider].values(), *self.shared),
                        "-d",
                        self.work / (provider + "-reject-" + name),
                    ],
                    allowed=(1,),
                )
                valid = any(
                    "[" + marker + "]" in result.stderr for marker in diagnostics
                )
                self.report["cases"].append(
                    {
                        "id": "metadata_rejection/" + provider + "/" + name,
                        "outcome": (
                            "invalid_rejected" if valid else "correctness_failure"
                        ),
                        "expected_diagnostic_names": diagnostics,
                    }
                )
        self.report["fixed_callers_classes_sha256"] = {
            relative(p): sha(p) for p in drivers.rglob("*.class")
        }
        self.report["kotlin_consumer_classes_sha256"] = {
            provider: {relative(p): sha(p) for p in output.rglob("*.class")}
            for provider, output in consumers.items()
        }

    def finish(self):
        changed = [
            relative(Path(p))
            for p, digest in self.frozen.items()
            if not Path(p).is_file() or sha(p) != digest
        ]
        self.report["cases"].append(
            {
                "id": "frozen_inputs",
                "outcome": "pass" if not changed else "correctness_failure",
                "changed": changed,
            }
        )
        self.report["counts"] = dict(
            Counter(c["outcome"] for c in self.report["cases"])
        )
        self.report["status"] = (
            "failed"
            if self.report["counts"].get("correctness_failure")
            else "completed"
        )
        self.path.write_text(json.dumps(self.report, indent=2) + "\n")
        print(relative(self.path))
        print(json.dumps(self.report["counts"], sort_keys=True))
        if not self.args.skip_html:
            subprocess.run(
                [sys.executable, ROOT / "tools/reports.py"], cwd=ROOT, check=True
            )
        return int(self.report["status"] != "completed")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--official-json", type=Path, required=True)
    parser.add_argument("--candidate-json", type=Path, required=True)
    parser.add_argument("--build-evidence", type=Path)
    parser.add_argument("--skip-html", action="store_true")
    args = parser.parse_args()
    for key in ["official_json", "candidate_json", "build_evidence"]:
        value = getattr(args, key)
        if value:
            setattr(args, key, value.resolve())
    check = Verification(args)
    try:
        check.execute()
    except Exception as error:
        check.report["cases"].append(
            {
                "id": "harness_failure",
                "outcome": "correctness_failure",
                "error": str(error),
            }
        )
    return check.finish()


if __name__ == "__main__":
    raise SystemExit(main())
