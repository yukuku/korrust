import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parents[3]
SNAPSHOT = ROOT / 'benchmarks/sources/androidbible'
STDLIB_SHA = '6f64eac736db9434dd6925b4a518b9d1d17177652320c37916cf9ba3ce7d7d7a'


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--stdlib', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    assert sha(args.stdlib) == STDLIB_SHA
    compiler = ROOT / 'target/debug/korrust'
    javac = Path(os.environ['JAVA_HOME']) / 'bin/javac'
    inputs = [ROOT / 'Cargo.toml', ROOT / 'Cargo.lock', Path(__file__), *sorted((ROOT / 'src').rglob('*.rs'))]
    frozen = {str(p.relative_to(ROOT)): sha(p) for p in inputs}
    commands = []

    def run(command):
        result = subprocess.run(list(map(str, command)), capture_output=True, text=True, timeout=30)
        commands.append(dict(command=list(map(str, command)), exit_code=result.returncode,
                             stdout=result.stdout, stderr=result.stderr))
        assert result.returncode == 0, result.stderr

    with tempfile.TemporaryDirectory(prefix='korrust-source-assessment-') as temporary:
        work = Path(temporary)
        stdlib = work / 'stdlib'
        with zipfile.ZipFile(args.stdlib) as archive:
            for entry in archive.infolist():
                path = Path(entry.filename)
                assert not path.is_absolute() and '..' not in path.parts
                if not entry.filename.startswith('META-INF/versions/'):
                    archive.extract(entry, stdlib)
        java = work / 'java'
        java.mkdir()
        java_sources = [SNAPSHOT / 'AlkitabModel/src/main/java/yuku/alkitab/util/IntArrayList.java',
                        *sorted((SNAPSHOT / 'Bintex/src/main/java').rglob('*.java'))]
        stubs = [ROOT / 'tests/fixtures/interop' / p for p in
                 ['android/os/Parcel.java', 'android/os/Parcelable.java', 'androidx/annotation/NonNull.java']]
        run([javac, '--release', '8', '-Xlint:-options', '-d', java, *java_sources, *stubs])
        classpath = [stdlib, java]
        dependency_hashes = {}
        for name in ['SingleChapterVerses.kt', 'AudioLogEntry.kt']:
            source = next(SNAPSHOT.rglob(name))
            destination = work / name
            run([compiler, source, '-cp', os.pathsep.join(map(str, classpath)), '-d', destination])
            classpath.append(destination)
            dependency_hashes.update({str(p.relative_to(work)): sha(p) for p in destination.rglob('*.class')})

        def probe(pair):
            index, source = pair
            destination = work / 'corpus' / str(index)
            command = [str(compiler), str(source), '-cp', os.pathsep.join(map(str, classpath)), '-d', str(destination)]
            case = dict(id=str(source.relative_to(ROOT)), source_sha256=sha(source))
            try:
                result = subprocess.run(command, capture_output=True, text=True, timeout=30)
                case.update(outcome='emitted' if result.returncode == 0 else 'rejected',
                            exit_code=result.returncode, diagnostic=result.stderr.strip(),
                            emitted_classes_sha256={str(p.relative_to(destination)): sha(p)
                                                    for p in sorted(destination.rglob('*.class'))})
            except subprocess.TimeoutExpired:
                case.update(outcome='timeout')
            return case

        sources = sorted(SNAPSHOT.rglob('*.kt'))
        inventory = json.loads((ROOT / 'benchmarks/source-files.sha256.json').read_text())['androidbible']
        assert all(sha(p) == inventory[str(p.relative_to(SNAPSHOT))] for p in sources)
        with ThreadPoolExecutor(max_workers=4) as pool:
            cases = list(pool.map(probe, enumerate(sources)))
        assert frozen == {str(p.relative_to(ROOT)): sha(p) for p in inputs}
        report = dict(schema_version=1, kind='corpus', status='completed',
                      created_at=datetime.now(timezone.utc).isoformat(),
                      selection='All 400 pinned AndroidBible .kt files, including tests and tools; no .kts or generated sources.',
                      reason='Source emission assessment only. No runtime, ABI, Kotlin consumer, Android, full-app or performance checks. Rejection is the first blocker and may be a missing dependency; this is not whole-project language coverage.',
                      compiler=dict(version='korrust 0.1.0', binary_sha256=sha(compiler), profile='debug', source_files_sha256=frozen),
                      reference_compiler=dict(version='2.3.21', execution='not_run', role='Pinned stdlib only; historical correctness evidence remains separate.'),
                      inputs=dict(androidbible_manifest='benchmarks/sources.json',
                                  source_manifest_sha256=sha(ROOT / 'benchmarks/source-files.sha256.json'),
                                  stdlib_sha256=STDLIB_SHA, java_home=os.environ['JAVA_HOME'],
                                  java_sources_sha256={str(p.relative_to(ROOT)): sha(p) for p in java_sources},
                                  android_type_stubs_sha256={str(p.relative_to(ROOT)): sha(p) for p in stubs},
                                  candidate_dependency_classes_sha256=dependency_hashes),
                      classpath_scope='Pinned stdlib base entries (multi-release META-INF/versions entries excluded), original IntArrayList and Bintex Java classes, test-only Android type stubs, current Korrust SingleChapterVerses and AudioLogEntry. No other app, Android SDK, coroutine, Compose, serialization or test libraries.',
                      commands=commands,
                      reproduction_commands=[['cargo', 'build', '--locked', '--offline'],
                                             ['python3', str(Path(__file__).relative_to(ROOT)), '--stdlib', str(args.stdlib), '--output', str(args.output)]],
                      probe_command_template=['target/debug/korrust', '<source.kt>', '-cp', '<stdlib>:<java>:<SingleChapterVerses>:<AudioLogEntry>', '-d', '<fresh-per-source-directory>'],
                      cases=cases, counts=dict(Counter(c['outcome'] for c in cases)),
                      historical_comparison=dict(
                          source='benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/PlaybackState.kt',
                          prior_report='reports/correctness/2026-10-24T094102532411Z-androidbible-twenty-features.json',
                          prior_case='unchanged_source/PlaybackState', prior_outcome='pass',
                          current_outcome='rejected', scope='Current source-emission regression; compiler not changed or fixed by this assessment.'),
                      evidence=[str(Path(__file__).relative_to(ROOT))])
        args.output.write_text(json.dumps(report, indent=2) + '\n')
        print(json.dumps(report['counts']))


if __name__ == '__main__':
    main()
