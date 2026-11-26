#!/usr/bin/env python3
import argparse
import hashlib
import io
import json
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def sha(data):
    return hashlib.sha256(data).hexdigest()


def install(pin, output, artifact):
    if output.is_file() and sha(output.read_bytes()) == pin['classes_sha256']:
        print(output)
        return
    if artifact:
        data = artifact.read_bytes()
    else:
        with urllib.request.urlopen(pin['url'], timeout=60) as response:
            data = response.read(32 * 1024 * 1024 + 1)
    if len(data) > 32 * 1024 * 1024 or sha(data) != pin['sha256']:
        raise SystemExit('Compose Runtime artifact checksum mismatch')
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        classes = archive.read(pin['classes_entry'])
    if sha(classes) != pin['classes_sha256']:
        raise SystemExit('Compose Runtime classes checksum mismatch')
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_suffix('.tmp')
    temporary.write_bytes(classes)
    temporary.replace(output)
    print(output)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--artifact', type=Path)
    parser.add_argument('--annotation-artifact', type=Path)
    args = parser.parse_args()
    pin = json.loads((ROOT / 'references/compose-runtime.json').read_text())
    install(pin, ROOT / 'references/compose/runtime.jar', args.artifact)
    install(pin['annotations'], ROOT / 'references/compose/annotations.jar', args.annotation_artifact)


if __name__ == '__main__':
    main()
