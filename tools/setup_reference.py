#!/usr/bin/env python3
import hashlib
import json
import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def main():
    pin = json.loads((ROOT / "references/compiler.json").read_text())
    destination = ROOT / pin["path"]
    if not destination.exists():
        with tempfile.TemporaryDirectory(dir=ROOT / "references") as temporary:
            staging = Path(temporary)
            archive = staging / "compiler.zip"
            subprocess.run([
                "curl", "--fail", "--location", "--silent", "--show-error",
                "--connect-timeout", "20", "--max-time", "180", "--retry", "2",
                "--output", str(archive), pin["url"],
            ], check=True)
            if hashlib.sha256(archive.read_bytes()).hexdigest() != pin["archive_sha256"]:
                raise RuntimeError("Reference compiler archive checksum mismatch")
            with zipfile.ZipFile(archive) as zipped:
                for member in zipped.infolist():
                    relative = Path(member.filename)
                    if relative.is_absolute() or ".." in relative.parts or relative.parts[0] != "kotlinc":
                        raise RuntimeError("Unsafe archive entry")
                    output = staging / relative
                    if member.is_dir():
                        output.mkdir(parents=True, exist_ok=True)
                    else:
                        output.parent.mkdir(parents=True, exist_ok=True)
                        output.write_bytes(zipped.read(member))
                        mode = (member.external_attr >> 16) & 0o777
                        if mode:
                            output.chmod(mode)
            shutil.move(str(staging / "kotlinc"), destination)
    result = subprocess.run([destination / "bin/kotlinc", "-version"], text=True, capture_output=True, check=True)
    text = result.stdout + result.stderr
    if f"kotlinc-jvm {pin['version']} " not in text:
        raise RuntimeError("Reference compiler version mismatch")
    print(text.strip())


if __name__ == "__main__":
    main()
