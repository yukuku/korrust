#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import xml.etree.ElementTree as ET
import zipfile


def repository_root():
    for directory in (Path(__file__).resolve().parent, Path.cwd()):
        result = subprocess.run(
            ["git", "-C", str(directory), "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            return Path(result.stdout.strip())
    raise RuntimeError("Run from the Korrust checkout or keep this script inside it")


ROOT = repository_root()
MODULES = ["AlkitabModel", "AlkitabIo"]
EXPECTED_COMPILER = "8020ab45fdb0ec2e07e8634f04afbe37f7a635834e45a32c25bc428bdafa3d73"
sys.path.insert(0, str(ROOT / "tools"))
from check_androidbible_module import checked_compile_invocations
from check_androidbible_survey import class_info


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def rel(p):
    try:
        return str(Path(p).relative_to(ROOT))
    except ValueError:
        return str(p)


def tree(p):
    return {
        str(f.relative_to(p)): sha(f)
        for f in sorted(Path(p).rglob("*"))
        if f.is_file() and ".korrust" not in f.relative_to(p).parts
    }


def classes(p):
    return {n: h for n, h in tree(p).items() if n.endswith(".class")}


def require(value, message):
    if not value:
        raise AssertionError(message)


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--official", type=Path, required=True)
    p.add_argument("--candidate", type=Path, required=True)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    evidence = args.output.resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    report_path = evidence.with_suffix(".json")
    report = {
        "schema_version": 1,
        "kind": "correctness",
        "created_at": datetime.now(timezone.utc).isoformat(),
        "status": "running",
        "selection": "Fresh independent official/candidate Gradle builds of unchanged AlkitabModel and AlkitabIo: release AARs plus debug original unit tests. Both selected modules production Kotlin owned by candidate; original Java and external Bintex use javac. No selected production fallback/cache.",
        "compiler": {},
        "inputs": {},
        "cases": [],
        "modules": {},
        "commands": [],
        "evidence": [],
        "limitations": [
            "Configured Gradle JVM target 21 remains recorded; Korrust emits its supported classfile 49/52 rather than claiming target 21 bytecode. Java outputs are classfile 65.",
            "Model has no original unit tests. Io has 8 original tests, executed under each provider. This report does not count absent tests as passes.",
            "Release AARs and debug tests are built; debug AAR packaging, Android-device/full-app runtime, D8, and speed are outside this report. Separate downstream consumer evidence can establish additional ABI/runtime behavior.",
        ],
    }

    def archive(path, name):
        dest = evidence / name
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, dest)
        report["evidence"].append(rel(dest))
        return dest

    def case(name, **details):
        report["cases"].append({"id": name, "outcome": "pass", **details})

    try:
        snapshot = ROOT / "benchmarks/sources/androidbible"
        expected_inventory = json.loads(
            (ROOT / "benchmarks/source-files.sha256.json").read_text()
        )["androidbible"]
        builds = {
            name: json.loads(path.read_text())
            for name, path in [
                ("official", args.official),
                ("candidate", args.candidate),
            ]
        }
        require(
            sha(builds["candidate"]["compiler"]) == EXPECTED_COMPILER,
            "Frozen compiler hash changed",
        )
        revision = lambda name: subprocess.check_output(
            ["git", "rev-parse", name], cwd=ROOT, text=True
        ).strip()
        freeze_revision = revision("60e64ad70")
        build_start_revision = revision("0f36750e0")
        generation_revision = revision("HEAD")
        source_trees = {
            label: revision(commit + ":src")
            for label, commit in [
                ("binary_freeze", freeze_revision),
                ("build_start", build_start_revision),
                ("report_generation", generation_revision),
            ]
        }
        require(
            len(set(source_trees.values())) == 1,
            "Compiler source tree changed across freeze/build/report revisions",
        )
        require(
            not subprocess.check_output(
                [
                    "git",
                    "diff",
                    freeze_revision,
                    build_start_revision,
                    "--",
                    "src",
                    "Cargo.toml",
                    "Cargo.lock",
                ],
                cwd=ROOT,
            ),
            "Compiler inputs differ from binary source freeze",
        )
        report["compiler"] = {
            "path": rel(builds["candidate"]["compiler"]),
            "binary_sha256": EXPECTED_COMPILER,
            "source_revision": freeze_revision,
            "build_start_revision": build_start_revision,
            "report_generation_revision": generation_revision,
            "src_tree_sha1": source_trees,
            "compiler_inputs_match_freeze": True,
        }
        all_sources = {
            str(f.relative_to(snapshot)): sha(f)
            for f in sorted(snapshot.rglob("*"))
            if f.is_file() and f.suffix in (".kt", ".java", ".kts", ".toml")
        }
        selected = {
            n: h
            for n, h in all_sources.items()
            if any(n.startswith(m + "/src/") for m in MODULES)
            and n.endswith((".kt", ".java"))
        }
        external = {
            n: h
            for n, h in all_sources.items()
            if n.startswith("Bintex/src/") and n.endswith((".kt", ".java"))
        }
        for n, h in {**selected, **external}.items():
            require(
                expected_inventory[n] == h, "Preserved source inventory changed:" + n
            )
        for provider, b in builds.items():
            require(
                b["status"] == "passed"
                and b["returncode"] == 0
                and not b["capture_only"]
                and not b["production_kotlin_fallback"],
                "Unsuccessful or fallback build:" + provider,
            )
            require(
                sorted(b["selected_modules"]) == sorted(MODULES),
                "Wrong selected modules",
            )
            require(
                b["sources_unchanged"]
                and b["source_files_sha256"]
                == b["source_files_sha256_after_build"]
                == selected,
                "Incomplete/changed selected source inventory",
            )
            for n, h in all_sources.items():
                require(
                    sha(Path(b["work_dir"]) / n) == h,
                    "Disposable source/build input changed:" + n,
                )
            require(
                Path(b["work_dir"])
                != Path(
                    builds["candidate" if provider == "official" else "official"][
                        "work_dir"
                    ]
                ),
                "Provider worktrees overlap",
            )
            archive(
                args.official if provider == "official" else args.candidate,
                provider + "/build.json",
            )
            archive(b["log"], provider + "/gradle.log")
            adapter = Path(b["log"]).parent / "adapter.init.gradle"
            require(sha(adapter) == b["adapter_sha256"], "Adapter changed")
            archive(adapter, provider + "/adapter.init.gradle")
            log = Path(b["log"]).read_text()
            tasks = {
                name: (status or "EXECUTED")
                for name, status in re.findall(
                    r"^> Task (:\S+?)(?: (UP-TO-DATE|FROM-CACHE|NO-SOURCE|SKIPPED|FAILED))?$",
                    log,
                    re.M,
                )
            }
            for module in MODULES:
                for task in [
                    "compileReleaseKotlin",
                    "compileDebugKotlin",
                    "compileReleaseJavaWithJavac",
                    "compileDebugJavaWithJavac",
                    "bundleReleaseAar",
                ]:
                    require(
                        tasks.get(":" + module + ":" + task) == "EXECUTED",
                        "Selected production task reused/skipped:"
                        + provider
                        + module
                        + task,
                    )
                require(
                    tasks.get(":" + module + ":testDebugUnitTest")
                    == ("NO-SOURCE" if module == "AlkitabModel" else "EXECUTED"),
                    "Wrong original test task status",
                )
            require(
                "FROM-CACHE" not in tasks.values(),
                "Unexpected Gradle build-cache reuse",
            )
            (evidence / provider / "task-status.json").write_text(
                json.dumps(tasks, indent=2) + "\n"
            )
            report["evidence"].append(rel(evidence / provider / "task-status.json"))
            outer = [
                "python3",
                "tools/build_androidbible.py",
                "--mode",
                ("official" if provider == "official" else "korrust"),
                "--modules",
                *MODULES,
                "--work-dir",
                rel(b["work_dir"]),
                "--artifact-dir",
                rel(Path(b["log"]).parent.parent),
                "--result",
                str(args.official if provider == "official" else args.candidate),
                "--gradle",
                b["command"][0],
                "--sdk",
                "/workspace/work/deps33/sdk",
                "--compiler",
                rel(builds["candidate"]["compiler"]),
            ]
            report.setdefault("outer_build_commands_actual", []).append(
                {
                    "provider": provider,
                    "argv": outer,
                    "cwd": ".",
                    "environment": {
                        "JAVA_HOME": "/workspace/korrust-setup/jdk-21",
                        "GRADLE_USER_HOME": "/workspace/work/deps33/gradle-home",
                        "ANDROID_HOME": "/workspace/work/deps33/sdk",
                        "GIT_CEILING_DIRECTORIES": str(ROOT / "target"),
                    },
                }
            )
            report["commands"].append(
                {
                    "provider": provider,
                    "argv": b["command"],
                    "cwd": rel(b["work_dir"]),
                    "environment": {
                        "JAVA_HOME": "/workspace/korrust-setup/jdk-21",
                        "GRADLE_USER_HOME": "/workspace/work/deps33/gradle-home",
                        "ANDROID_HOME": "/workspace/work/deps33/sdk",
                        "GIT_CEILING_DIRECTORIES": str(ROOT / "target"),
                    },
                    "returncode": b["returncode"],
                }
            )
            case(
                "fresh_gradle/" + provider,
                selected_modules=MODULES,
                source_integrity=True,
                build_cache=False,
                production_kotlin_fallback=False,
            )
        report["inputs"] = {
            "all_snapshot_source_and_build_files_sha256": all_sources,
            "selected_source_files_sha256": selected,
            "external_bintex_java_sources_sha256": external,
            "tool_sha256": sha(ROOT / "tools/build_androidbible.py"),
            "classpath_policy_sha256": sha(ROOT / "tools/classpath_policy.py"),
            "class_info_tool_sha256": sha(ROOT / "tools/check_androidbible_survey.py"),
            "androidbible_commit": json.loads(
                (ROOT / "benchmarks/sources.json").read_text()
            )["sources"]["androidbible"]["commit"],
        }
        packaged = {}
        java_maps = {}
        for provider, b in builds.items():
            packaged[provider] = {}
            java_maps[provider] = {}
            for module, m in b["modules"].items():
                require(
                    m["sources_unchanged"]
                    and not m["resolved_inputs"]["missing_required_inputs"]
                    and not m["resolved_inputs"]["friend_context_failures"],
                    "Missing source/dependency/friend inputs",
                )
                model_file = (
                    Path(b["log"]).parent / "modules" / module / "gradle-model.json"
                )
                archive(model_file, provider + "/" + module + "/gradle-model.json")
                expected_kt = sorted(
                    n
                    for n in selected
                    if n.startswith(module + "/src/main/") and n.endswith(".kt")
                )
                expected_java = sorted(
                    n
                    for n in selected
                    if n.startswith(module + "/src/main/") and n.endswith(".java")
                )
                require(
                    m["production_kotlin_sources"] == expected_kt
                    and m["production_java_sources"] == expected_java,
                    "Production inventory incomplete",
                )
                java = classes(m["java_classes"])
                java_maps[provider][module] = java
                java_sources = {
                    class_info((Path(m["java_classes"]) / n).read_bytes())[
                        "source_file"
                    ]
                    for n in java
                }
                require(
                    java_sources == {Path(n).name for n in expected_java},
                    "Java source/output ownership mismatch",
                )
                with zipfile.ZipFile(m["aar"]) as aar:
                    require(
                        "AndroidManifest.xml" in aar.namelist(), "AAR manifest absent"
                    )
                    payload = aar.read("classes.jar")
                    entries = {
                        n: hashlib.sha256(aar.read(n)).hexdigest()
                        for n in sorted(aar.namelist())
                        if not n.endswith("/")
                    }
                with zipfile.ZipFile(io.BytesIO(payload)) as jar:
                    hashes = {
                        n: hashlib.sha256(jar.read(n)).hexdigest()
                        for n in sorted(jar.namelist())
                        if not n.endswith("/")
                    }
                    majors = Counter(
                        class_info(jar.read(n))["class_major_version"]
                        for n in hashes
                        if n.endswith(".class")
                    )
                packaged[provider][module] = {
                    n: h for n, h in hashes.items() if n.endswith(".class")
                }
                release_kt = classes(m["kotlin_classes"])
                debug_kt = classes(m["debug_kotlin_classes"])
                require(
                    packaged[provider][module] == {**java, **release_kt},
                    "AAR class bytes differ from selected Kotlin/Java outputs",
                )
                tests = []
                for xml in m["test_results"]:
                    copied = archive(
                        xml, provider + "/" + module + "/" + Path(xml).name
                    )
                    suite = ET.parse(xml).getroot()
                    require(
                        all(
                            int(suite.get(n, 0)) == 0
                            for n in ["failures", "errors", "skipped"]
                        ),
                        "Original tests did not all pass",
                    )
                    tests.extend(
                        {
                            "class": t.get("classname"),
                            "name": t.get("name"),
                            "outcome": "pass",
                        }
                        for t in suite.findall("testcase")
                    )
                require(
                    len(tests) == (0 if module == "AlkitabModel" else 8),
                    "Unexpected original test count",
                )
                info = {
                    "production_kotlin_sources": expected_kt,
                    "production_java_sources": expected_java,
                    "original_tests": tests,
                    "original_test_count": len(tests),
                    "release_aar": {
                        "path": rel(m["aar"]),
                        "sha256": sha(m["aar"]),
                        "entries_sha256": entries,
                        "classes_jar_sha256": hashlib.sha256(payload).hexdigest(),
                        "entries_in_classes_jar_sha256": hashes,
                        "class_count": len(packaged[provider][module]),
                        "classfile_major_version_counts": dict(majors),
                    },
                    "release_kotlin_classes_sha256": release_kt,
                    "release_java_classes_sha256": java,
                    "debug_kotlin_classes_sha256": debug_kt,
                    "options": m["options"],
                    "dependency_boundary": m["dependency_boundary"],
                    "variant_scope": {
                        "release": "AAR packaged + Kotlin/Javac executed",
                        "debug": "Kotlin/Javac executed + original unit tests; no debug AAR packaging claim",
                    },
                }
                report["modules"].setdefault(module, {})[provider] = info
                case(
                    "packaged_and_debug/" + provider + "/" + module,
                    release_classes=len(packaged[provider][module]),
                    debug_kotlin_classes=len(debug_kt),
                    original_tests=len(tests),
                )
                if provider == "candidate":
                    for trace_path in m["production_compile_traces"]:
                        trace_path = Path(trace_path)
                        request_path = trace_path.with_name(
                            trace_path.stem + "-request.json"
                        )
                        t = json.loads(trace_path.read_text())
                        q = json.loads(request_path.read_text())
                        task = trace_path.stem
                        archive(
                            trace_path, provider + "/" + module + "/" + trace_path.name
                        )
                        archive(
                            request_path,
                            provider + "/" + module + "/" + request_path.name,
                        )
                        require(
                            not t["fallback"]
                            and t["compiler_sha256"] == EXPECTED_COMPILER,
                            "Candidate fallback/compiler mismatch",
                        )
                        checked_compile_invocations(
                            t, q, b["compiler"], selected, b["work_dir"]
                        )
                        require(
                            sorted(
                                str(Path(s["path"]).relative_to(b["work_dir"]))
                                for s in t["sources"]
                            )
                            == expected_kt,
                            "Candidate sources incomplete",
                        )
                        actual_stage = tree(q["stage"])
                        actual_destination = tree(q["destination"])
                        require(
                            actual_stage
                            == actual_destination
                            == t["published_files_sha256"],
                            "Staged/published candidate bytes mismatch",
                        )
                        snap = (
                            release_kt if task == "compileReleaseKotlin" else debug_kt
                        )
                        require(
                            {
                                n: h
                                for n, h in actual_stage.items()
                                if n.endswith(".class")
                            }
                            == snap,
                            "Snapshot candidate Kotlin bytes mismatch",
                        )
                        overlap = []
                        for entry in q["classpath"]:
                            p = Path(entry)
                            if p.is_dir():
                                overlap.extend(
                                    str(p / n) for n in snap if (p / n).is_file()
                                )
                            else:
                                with zipfile.ZipFile(p) as z:
                                    overlap.extend(
                                        str(p) + "!" + n
                                        for n in snap
                                        if n in z.namelist()
                                    )
                        require(
                            not overlap, "Target bytecode found on its own classpath"
                        )
                        case(
                            "candidate_ownership/" + module + "/" + task,
                            source_count=len(t["sources"]),
                            published_class_count=len(snap),
                            own_target_classpath_overlap=overlap,
                            complete_batch=True,
                            staged_and_published_bytes_match=True,
                            release_aar_bytes_verified=(task == "compileReleaseKotlin"),
                            debug_aar_packaging_claimed=False,
                        )
        for module in MODULES:
            same = java_maps["official"][module] == java_maps["candidate"][module]
            report["modules"][module][
                "java_class_bytes_identical_between_providers"
            ] = same
            require(same, "Original Java output differs between providers:" + module)
        for variant in ["Debug", "Release"]:
            task = "compile" + variant + "Kotlin"
            m = builds["candidate"]["modules"]["AlkitabIo"]
            path = next(
                Path(p) for p in m["production_compile_traces"] if Path(p).stem == task
            )
            t = json.loads(path.read_text())
            dependency = [
                i
                for i in t["classpath"]
                if "/AlkitabModel/" in i["path"]
                and "/compile_library_classes_jar/" in i["path"]
            ]
            require(len(dependency) == 1, "Io missing selected Model dependency")
            dep = dependency[0]
            require(
                sha(dep["path"]) == dep["sha256"], "Selected Model dependency changed"
            )
            with zipfile.ZipFile(dep["path"]) as jar:
                dep_classes = {
                    n: hashlib.sha256(jar.read(n)).hexdigest()
                    for n in jar.namelist()
                    if n.endswith(".class")
                }
            expected = report["modules"]["AlkitabModel"]["candidate"][
                (
                    "release_kotlin_classes_sha256"
                    if variant == "Release"
                    else "debug_kotlin_classes_sha256"
                )
            ]
            require(
                all(dep_classes.get(n) == h for n, h in expected.items()),
                "Io selected Model Kotlin fallback",
            )
            require(
                all(
                    dep_classes.get(n) == h
                    for n, h in java_maps["candidate"]["AlkitabModel"].items()
                ),
                "Io selected Model Java dependency mismatch",
            )
            case(
                "candidate_model_to_io/" + variant.lower(),
                dependency_jar=rel(dep["path"]),
                dependency_sha256=dep["sha256"],
                candidate_model_kotlin_classes=len(expected),
                candidate_model_java_classes=len(
                    java_maps["candidate"]["AlkitabModel"]
                ),
                class_bytes_verified=True,
            )
        report["status"] = "passed"
        report["counts"] = dict(Counter(c["outcome"] for c in report["cases"]))
        report["derived_module_counts"] = {
            "production_kotlin_sources": 2,
            "production_java_sources": 17,
            "original_test_java_sources": 2,
            "external_bintex_java_sources": 3,
            "original_tests_per_provider": 8,
        }
        own = archive(Path(__file__), "verify-builds.py")
        archive(Path(__file__).with_name("replay-builds.py"), "replay-builds.py")
        report["outer_build_reproduction_commands"] = [
            [
                "python3",
                rel(evidence / "replay-builds.py"),
                "--repo-root",
                ".",
                "--jdk",
                "/workspace/korrust-setup/jdk-21",
                "--sdk",
                "/workspace/work/deps33/sdk",
                "--gradle",
                "/workspace/work/deps33/gradle-9.7.1/bin/gradle",
                "--gradle-user-home",
                "/workspace/work/deps33/gradle-home",
                "--compiler",
                "target/release/korrust",
            ]
        ]
        report["reproduction_commands"] = [
            [
                "python3",
                rel(own),
                "--official",
                rel(args.official),
                "--candidate",
                rel(args.candidate),
                "--output",
                "reports/correctness/<new-dated-model-io-evidence>",
            ]
        ]
    except Exception as error:
        report["status"] = "failed"
        report["error"] = str(error)
        raise
    finally:
        report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(report_path)


if __name__ == "__main__":
    main()
