#!/usr/bin/env python3
"""Run fresh official/candidate Model+Io release AAR and debug-test builds."""

import argparse
from datetime import datetime, timezone
import hashlib
import os
from pathlib import Path
import subprocess
import sys

EXPECTED_COMPILER = "8020ab45fdb0ec2e07e8634f04afbe37f7a635834e45a32c25bc428bdafa3d73"


def repository_root():
    for directory in (Path(__file__).resolve().parent, Path.cwd()):
        result = subprocess.run(
            ["git", "-C", str(directory), "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
        )
        if result.returncode == 0:
            return Path(result.stdout.strip())
    raise RuntimeError("Pass --repo-root or run from the Korrust repository")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path)
    parser.add_argument("--jdk", type=Path, required=True)
    parser.add_argument("--sdk", type=Path, required=True)
    parser.add_argument("--gradle", type=Path, required=True)
    parser.add_argument("--gradle-user-home", type=Path, required=True)
    parser.add_argument("--compiler", type=Path, default=Path("target/release/korrust"))
    parser.add_argument(
        "--run-id",
        default=datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ") + "-model-io",
    )
    args = parser.parse_args()
    root = args.repo_root.resolve() if args.repo_root else repository_root()
    compiler = (root / args.compiler).resolve()
    if hashlib.sha256(compiler.read_bytes()).hexdigest() != EXPECTED_COMPILER:
        raise ValueError("Use the recorded frozen compiler; its SHA256 differs")
    if Path(args.run_id).name != args.run_id:
        raise ValueError("--run-id must be one directory-name component")
    output = root / "target" / args.run_id
    if output.exists():
        raise ValueError(
            "Fresh independent disposable directory required: " + str(output)
        )
    output.mkdir(parents=True)
    environment = os.environ.copy()
    environment.update(
        JAVA_HOME=str(args.jdk.resolve()),
        ANDROID_HOME=str(args.sdk.resolve()),
        GRADLE_USER_HOME=str(args.gradle_user_home.resolve()),
        GIT_CEILING_DIRECTORIES=str(root / "target"),
    )
    environment["PATH"] = (
        str(args.jdk.resolve() / "bin") + os.pathsep + environment.get("PATH", "")
    )
    for provider, mode in [("official", "official"), ("candidate", "korrust")]:
        command = [
            sys.executable,
            str(root / "tools/build_androidbible.py"),
            "--mode",
            mode,
            "--modules",
            "AlkitabModel",
            "AlkitabIo",
            "--work-dir",
            str(output / (provider + "-project")),
            "--artifact-dir",
            str(output / (provider + "-artifacts")),
            "--result",
            str(output / (provider + "-build.json")),
            "--gradle",
            str(args.gradle.resolve()),
            "--sdk",
            str(args.sdk.resolve()),
            "--compiler",
            str(compiler),
        ]
        with (output / (provider + "-driver.log")).open("w") as log:
            result = subprocess.run(
                command, cwd=root, env=environment, stdout=log, stderr=subprocess.STDOUT
            )
        if result.returncode:
            raise RuntimeError(
                "Build failed; see " + str(output / (provider + "-driver.log"))
            )
    print(output)


if __name__ == "__main__":
    main()
