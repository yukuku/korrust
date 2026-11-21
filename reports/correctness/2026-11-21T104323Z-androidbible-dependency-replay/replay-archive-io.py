#!/usr/bin/env python3
import argparse
import hashlib
import json
from pathlib import Path
import resource
import shutil
import subprocess
import time

here = Path(__file__).resolve().parent
parser = argparse.ArgumentParser(description='Replay the same-artifact raw/buffered ZIP comparison')
parser.add_argument('--artifact', type=Path, required=True)
parser.add_argument('--work-dir', type=Path, required=True)
parser.add_argument('--cargo', default='cargo')
args = parser.parse_args()
artifact, work = args.artifact.resolve(), args.work_dir.resolve()
pins = json.loads((here / 'downloads.json').read_text())
assert hashlib.sha256(artifact.read_bytes()).hexdigest() == pins['android_jar_sha256']
(work / 'src').mkdir(parents=True, exist_ok=True)
for original, target in [('benchmark-Cargo.toml', 'Cargo.toml'), ('benchmark-Cargo.lock', 'Cargo.lock'),
                         ('benchmark-main.rs', 'src/main.rs')]:
    shutil.copy2(here / 'archive-io' / original, work / target)
subprocess.run([args.cargo, 'build', '--locked', '--manifest-path', str(work / 'Cargo.toml')], check=True)
binary = work / 'target/debug/archive33-bench'
samples = []
for mode in ['raw', 'buffered'] * 3:
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    result = subprocess.run([str(binary), mode, str(artifact), '5'], capture_output=True, check=True)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    samples.append({'mode': mode, 'wall_seconds': time.perf_counter() - start,
                    'user_seconds': after.ru_utime - before.ru_utime,
                    'sys_seconds': after.ru_stime - before.ru_stime,
                    'stdout': result.stdout.decode()})
if shutil.which('strace'):
    for mode in ['raw', 'buffered']:
        subprocess.run(['strace', '-c', '-e', 'trace=read,lseek', '-o', str(work / (mode + '-syscalls.txt')),
                        str(binary), mode, str(artifact), '1'], check=True)
receipt = {'artifact': str(artifact), 'artifact_sha256': pins['android_jar_sha256'],
           'build_profile': 'debug; unoptimized + debuginfo; same binary for both reader modes',
           'samples': samples}
(work / 'timings.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps(receipt, indent=2))
