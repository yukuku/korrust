"""Validate ordered compiler classpaths without extracting or rewriting dependencies.

Ordinary stored/deflated archives and directories preserve Kotlin package metadata.
Module descriptors are ignored by the compiler; other versioned classes reject,
regardless of the manifest. The first dependency defining a class takes precedence.
"""

import hashlib
import os
from pathlib import Path
import zipfile


MAX_ENTRY = 64 * 1024 * 1024


def _valid_name(name):
    name = name[:-1] if name.endswith("/") else name
    return (
        bool(name)
        and "\\" not in name
        and "\0" not in name
        and not (len(name) > 1 and name[1] == ":")
        and all(part not in ("", ".", "..") for part in name.split("/"))
    )


def _validate_multi_release(path, name):
    if (
        name.startswith("META-INF/versions/")
        and name.endswith(".class")
        and name.rsplit("/", 1)[-1] != "module-info.class"
    ):
        raise ValueError(f"Classpath {path}: multi-release JAR class overrides are unsupported")


def _raise(error):
    raise error


def validate_classpath(paths):
    """Return provenance in input order after matching the compiler's archive policy."""
    records = []
    for item in paths:
        path = Path(item)
        try:
            if path.is_dir():
                versions = path / "META-INF/versions"
                if versions.exists():
                    for parent, directories, files in os.walk(versions, onerror=_raise):
                        directories.sort()
                        for name in sorted(files):
                            _validate_multi_release(
                                path, (Path(parent) / name).relative_to(path).as_posix()
                            )
                records.append({"path": str(path), "kind": "directory"})
                continue
            if not path.is_file():
                raise ValueError(f"Classpath {path}: must be an existing directory or JAR/ZIP archive")
            with zipfile.ZipFile(path) as archive:
                names = set()
                for entry in archive.infolist():
                    name = entry.orig_filename
                    if not _valid_name(name):
                        raise ValueError(f"Classpath {path}: malformed ZIP entry name")
                    _validate_multi_release(path, name)
                    if name in names:
                        raise ValueError(f"Classpath {path}: duplicate ZIP entry names are unsupported")
                    names.add(name)
                    if entry.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED):
                        raise ValueError(f"Classpath {path}: unsupported ZIP compression method")
                    if entry.file_size > MAX_ENTRY:
                        raise ValueError(f"Classpath {path}: ZIP entry exceeds the 64 MiB limit")
                    with archive.open(entry):
                        pass
            checksum = hashlib.sha256()
            with path.open("rb") as stream:
                for block in iter(lambda: stream.read(1024 * 1024), b""):
                    checksum.update(block)
            records.append({"path": str(path), "kind": "jar", "sha256": checksum.hexdigest()})
        except (OSError, zipfile.BadZipFile, RuntimeError) as error:
            raise ValueError(f"Classpath {path}: {error}") from error
    return records
