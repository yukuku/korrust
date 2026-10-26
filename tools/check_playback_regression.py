#!/usr/bin/env python3
import argparse
from collections import Counter
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/fixtures/compatibility"
SNAPSHOT = ROOT / "benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio"


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--frozen-inputs", type=Path, required=True)
    args = parser.parse_args()
    now = datetime.now(timezone.utc)
    run_id = now.strftime("%Y-%m-%dT%H%M%S%fZ") + "-playback-regression"
    work = ROOT / "target/checks" / run_id
    evidence = ROOT / "reports/correctness" / run_id
    work.mkdir(parents=True)
    evidence.mkdir(parents=True)
    pin = json.loads((ROOT / "references/compiler.json").read_text())
    kotlin = ROOT / pin["path"] / "bin/kotlinc"
    stdlib = ROOT / pin["path"] / "lib/kotlin-stdlib.jar"
    options = ["-language-version", "2.3", "-api-version", "2.3", "-jvm-target", "21", "-module-name", "androidbible_twenty"]
    report = dict(schema_version=1, kind="correctness", created_at=now.isoformat(), status="running", commands=[], cases=[], evidence=[],
                  compiler={"binary_sha256": sha(args.compiler), "source_files_sha256": json.loads(args.frozen_inputs.read_text())},
                  reference_compiler={"version": pin["version"], "options": options, "stdlib_sha256": sha(stdlib)},
                  inputs={"baseline_revision": "9fdbdf6736f187c3ffbffc985a0c85b13f49de14", "baseline_binary_sha256": sha(args.baseline)},
                  selection="Matched reproduction and repair validation of expected generic result inference; unchanged PlaybackState and AudioLogEntry. JVM source/consumer subset; no Android or full-module claim.")

    def case(name, **details):
        report["cases"].append(dict(id=name, outcome="pass", **details))

    def run(label, command, accepted=(0,)):
        command = list(map(str, command))
        index = len(report["commands"])
        result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=180)
        item = dict(label=label, argv=command, cwd=".", timeout_seconds=180, exit_code=result.returncode)
        for stream in ("stdout", "stderr"):
            path = evidence / f"{index:03d}-{stream}.txt"
            path.write_text(getattr(result, stream))
            item[stream] = str(path.relative_to(ROOT))
            report["evidence"].append(item[stream])
        report["commands"].append(item)
        if result.returncode not in accepted:
            raise AssertionError(f"{label}: {result.returncode}: {result.stderr[-3000:]}")
        print(label, flush=True)
        return result

    def cp(*paths):
        return os.pathsep.join(map(str, paths))

    def compile_source(provider, source, output, dependencies):
        command = [kotlin, source, *options] if provider == "official" else [args.compiler, source]
        run(f"{provider}/{source.name}", [*command, "-cp", cp(*dependencies), "-d", output])

    try:
        run("official identity", [kotlin, "-version"])
        run("candidate identity", [args.compiler, "--version"])
        run("JDK identity", ["java", "-version"])
        report["inputs"]["sources_sha256"] = {str(p.relative_to(ROOT)): sha(p) for p in [SNAPSHOT / "PlaybackState.kt", SNAPSHOT / "AudioLogEntry.kt", *sorted(FIXTURES.iterdir()), Path(__file__).resolve()]}
        inventory = json.loads((ROOT / "benchmarks/source-files.sha256.json").read_text())["androidbible"]
        for name in ("PlaybackState.kt", "AudioLogEntry.kt"):
            path = SNAPSHOT / name
            assert sha(path) == inventory[str(path.relative_to(ROOT / "benchmarks/sources/androidbible"))]
        stdlib_dir = work / "stdlib"
        with zipfile.ZipFile(stdlib) as archive:
            for entry in archive.infolist():
                if Path(entry.filename).name != "module-info.class":
                    archive.extract(entry, stdlib_dir)
        report["inputs"]["stdlib_directory_policy"] = "Supported directory extraction including package metadata; omit only root/versioned module descriptors, never meaningful class overrides. Full extraction exposed a separately recorded descriptor-indexing defect addressed by task 6."
        outputs = {}
        for provider in ("official", "candidate"):
            model = work / (provider + "-model")
            compile_source(provider, SNAPSHOT / "AudioLogEntry.kt", model, [stdlib])
            outputs[provider] = (model, work / (provider + "-app"))
        for model_provider, (model, _) in outputs.items():
            for classpath_kind, library in (("jar", stdlib), ("directory", stdlib_dir)):
                for target in (SNAPSHOT / "PlaybackState.kt", FIXTURES / "ExpectedResult.kt"):
                    label = f"{model_provider}/{classpath_kind}/{target.stem}"
                    baseline = run("baseline rejection " + label, [args.baseline, target, "-cp", cp(library, model), "-d", work / ("baseline-" + label.replace("/", "-"))], (1,))
                    assert "error:" in baseline.stderr
                    case("baseline_rejection/" + label, diagnostic=baseline.stderr.strip(), expected="Compiler regression under real stdlib; previously successful report linked separately.")
                    destination = outputs["candidate"][1] if model_provider == "candidate" and classpath_kind == "jar" else work / ("fixed-" + label.replace("/", "-"))
                    compile_source("candidate", target, destination, [library, model])
                    case("fixed_emission/" + label)
        baseline_no_stdlib = run("baseline without explicit stdlib", [args.baseline, SNAPSHOT / "PlaybackState.kt", "-cp", outputs["candidate"][0], "-d", work / "baseline-no-stdlib"])
        case("classpath_trigger", scope="Unchanged current source accepts without explicit stdlib but rejects with the ordinary library generic factory present. Raw JAR and full directory inputs fail alike before repair.")
        for provider, (model, output) in outputs.items():
            if provider == "official":
                for source in (SNAPSHOT / "PlaybackState.kt", FIXTURES / "ExpectedResult.kt"):
                    compile_source(provider, source, output, [stdlib, model])
            rejected = run(provider + " invalid inferred cycle", [kotlin if provider == "official" else args.compiler, FIXTURES / "CyclicInference.kt", "-d", work / (provider + "-cycle")], (1,))
            assert "error:" in rejected.stderr
            case("invalid_cycle/" + provider)
        java_output = work / "java-caller"
        model, app = outputs["official"]
        run("fixed Java caller against official output", ["javac", "-cp", cp(stdlib, model, app), "-d", java_output, FIXTURES / "PlaybackCheck.java"])
        java_observations = []
        kotlin_observations = []
        for provider, (model, output) in outputs.items():
            java_observations.append(run("verified fixed Java caller/" + provider, ["java", "-Xverify:all", "-cp", cp(java_output, model, output, stdlib), "PlaybackCheck"]).stdout)
            consumer = work / (provider + "-consumer")
            run("official Kotlin consumer/" + provider, [kotlin, FIXTURES / "PlaybackConsumer.kt", *options, "-cp", cp(model, output, stdlib), "-d", consumer])
            kotlin_observations.append(run("verified Kotlin consumer/" + provider, ["java", "-Xverify:all", "-cp", cp(consumer, model, output, stdlib), "PlaybackConsumerKt"]).stdout)
        assert java_observations[0] == java_observations[1] == "checks=26\n", java_observations
        assert kotlin_observations[0] == kotlin_observations[1] == "kotlin-consumer=pass\n"
        case("fixed_java_runtime", independent_assertions=26, checks="All IDLE values, identity, copy/getters/components/equality/hash, null failure, reduced factory result.")
        case("official_kotlin_consumers", scope="Metadata, named copy/default arguments, companion IDLE/EMPTY, generic nullable result.")
        from check_nowinandroid import source_abi
        for owner in ("yuku.alkitab.base.audio.PlaybackState", "yuku.alkitab.base.audio.PlaybackState$Companion", "compatibility.Snapshot", "compatibility.ExpectedResultKt"):
            signatures = []
            for provider, (model, output) in outputs.items():
                signatures.append(source_abi(run("ABI/" + provider + "/" + owner, ["javap", "-p", "-s", "-classpath", cp(model, output), owner]).stdout))
            assert signatures[0] == signatures[1], owner
            case("abi/" + owner)
        report["analysis"] = {"cause": "Expected result constraints were lost when compiled generic calls were checked as constructor arguments. Presence of real stdlib activates ordinary library resolution instead of the previous fallback.", "fix": "Propagate expected argument/result types through generic call specialization; preserve conflicting explicit and argument constraints, nullability, and original companion inference diagnostics.", "historical_success": "reports/correctness/2026-10-24T094102532411Z-androidbible-twenty-features.json", "scope": "Current baseline freshly built from isolated 9fdbdf673 sources. Historical report preserves its exact old compiler, commands, dependency/output hashes and JVM21 options; older binary was not rebuilt. Candidate has no JVM target switch, so target-bytecode equivalence is not claimed."}
        report["status"] = "completed"
    except Exception as error:
        report["status"] = "failed"
        report["failure"] = str(error)
        report["cases"].append(dict(id="run_failure", outcome="failure", message=str(error)))
        raise
    finally:
        report["counts"] = dict(Counter(case["outcome"] for case in report["cases"]))
        path = evidence.with_suffix(".json")
        path.write_text(json.dumps(report, indent=2) + "\n")
        shutil.copy2(__file__, evidence / Path(__file__).name)
        for fixture in FIXTURES.iterdir():
            shutil.copy2(fixture, evidence / fixture.name)
        print(path.relative_to(ROOT), flush=True)


if __name__ == "__main__":
    main()
