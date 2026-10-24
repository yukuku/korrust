#!/usr/bin/env python3
"""Verify the pinned reference and application source snapshots without network access."""

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys


ROOT = Path(__file__).resolve().parents[1]
MANIFESTS = {
    "references/source-files.sha256.json": {
        "kotlin": "references/kotlin",
        "kotlin-spec": "references/kotlin-spec",
    },
    "benchmarks/source-files.sha256.json": {
        "androidbible": "benchmarks/sources/androidbible",
        "nowinandroid": "benchmarks/sources/nowinandroid",
    },
}


def safe_path(root, relative):
    if not isinstance(relative, str) or not relative or "\\" in relative:
        raise ValueError("unsafe manifest path")
    parts = relative.split("/")
    if PurePosixPath(relative).is_absolute() or any(
        part in ("", ".", "..", ".git") for part in parts
    ):
        raise ValueError("unsafe manifest path")
    current = root
    for part in parts[:-1]:
        current = current / part
        try:
            mode = current.lstat().st_mode
        except FileNotFoundError:
            continue
        if stat.S_ISLNK(mode):
            raise ValueError("symlink ancestor is forbidden")
        if not stat.S_ISDIR(mode):
            raise ValueError("ancestor is not a directory")
    return root.joinpath(*parts)


def inventory(root):
    found = set()
    pending = [root]
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as children:
            for child in children:
                if child.name == ".git":
                    continue
                path = Path(child.path)
                if child.is_dir(follow_symlinks=False):
                    pending.append(path)
                else:
                    found.add(path.relative_to(root).as_posix())
    return found


def hash_file(path):
    digest = hashlib.sha256()
    size = 0
    prefix = b""
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as source:
        if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
            raise ValueError("expected a regular file")
        while chunk := source.read(1024 * 1024):
            if not prefix:
                prefix = chunk[:1024]
            digest.update(chunk)
            size += len(chunk)
    return digest.hexdigest(), size, prefix


def verify_snapshot(root, entries, allow_extra=False):
    result = {"files": 0, "symlinks": 0, "bytes": 0, "errors": []}
    errors = result["errors"]
    if not isinstance(entries, dict):
        errors.append("manifest inventory must be an object")
        return result
    try:
        mode = root.lstat().st_mode
        if not stat.S_ISDIR(mode):
            raise ValueError("snapshot root must be a directory, not a symlink")
    except (OSError, ValueError) as error:
        errors.append(f"snapshot unavailable: {error}")
        return result
    for relative, expected in sorted(entries.items()):
        try:
            path = safe_path(root, relative)
            if isinstance(expected, str):
                if not re.fullmatch(r"[0-9a-f]{64}", expected):
                    raise ValueError("invalid SHA-256 digest")
            elif not (isinstance(expected, dict) and set(expected) == {"symlink"}
                      and isinstance(expected["symlink"], str)):
                raise ValueError("expected a SHA-256 digest or a symlink target object")
            mode = path.lstat().st_mode
            if isinstance(expected, dict):
                if not stat.S_ISLNK(mode):
                    raise ValueError("expected a symlink")
                target = os.readlink(path)
                if target != expected["symlink"]:
                    raise ValueError(f"symlink target mismatch: {target!r}")
                result["symlinks"] += 1
            else:
                if not stat.S_ISREG(mode):
                    raise ValueError("expected a regular file")
                actual, size, prefix = hash_file(path)
                result["bytes"] += size
                if actual != expected:
                    message = f"SHA-256 mismatch: expected {expected}, got {actual}"
                    if prefix.splitlines()[:1] == [b"version https://git-lfs.github.com/spec/v1"]:
                        message += f" (Git LFS pointer; run git lfs pull in {root})"
                    raise ValueError(message)
                result["files"] += 1
        except FileNotFoundError:
            errors.append(f"{relative}: missing")
        except (OSError, ValueError) as error:
            errors.append(f"{relative}: {error}")
    if not allow_extra:
        try:
            for relative in sorted(inventory(root) - entries.keys()):
                errors.append(f"{relative}: extra file or symlink")
        except OSError as error:
            errors.append(f"cannot inventory snapshot: {error}")
    return result


def verify_repository(root, allow_extra=False):
    results = {}
    for manifest_name, snapshots in MANIFESTS.items():
        try:
            manifest_path = safe_path(root, manifest_name)
            if not stat.S_ISREG(manifest_path.lstat().st_mode):
                raise ValueError("manifest must be a regular file")
            manifest = json.loads(manifest_path.read_text())
            if not isinstance(manifest, dict) or set(manifest) != set(snapshots):
                raise ValueError(f"manifest keys must be {', '.join(snapshots)}")
        except (OSError, ValueError) as error:
            results[manifest_name] = {"files": 0, "symlinks": 0, "bytes": 0,
                                      "errors": [f"cannot read manifest: {error}"]}
            continue
        for name, relative in snapshots.items():
            try:
                snapshot = safe_path(root, relative)
                results[relative] = verify_snapshot(snapshot, manifest[name], allow_extra)
            except (OSError, ValueError) as error:
                results[relative] = {"files": 0, "symlinks": 0, "bytes": 0,
                                     "errors": [str(error)]}
    return results


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=ROOT, help="repository root (default: script's repository)")
    parser.add_argument("--allow-extra", action="store_true", help="skip checking for unlisted files and symlinks")
    args = parser.parse_args(argv)
    results = verify_repository(args.root.resolve(), args.allow_extra)
    error_count = 0
    for name, result in results.items():
        errors = result["errors"]
        error_count += len(errors)
        print(f"{name}: {result['files']} files, {result['symlinks']} symlinks verified; "
              f"{result['bytes']} bytes hashed; {len(errors)} errors")
        for error in errors[:10]:
            print(f"  {error}", file=sys.stderr)
        if len(errors) > 10:
            print(f"  ... {len(errors) - 10} more errors", file=sys.stderr)
    print(f"Source snapshot verification {'FAILED' if error_count else 'passed'}: {error_count} errors")
    return int(bool(error_count))


if __name__ == "__main__":
    sys.exit(main())
