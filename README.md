# Korrust

A Kotlin/JVM compiler written in Rust for faster incremental Android development builds.

Created by **Randy Sugianto (Yuku)** ([**@yukuku**](https://github.com/yukuku)).

## Run

Install Rust, Python, and JDK 21. Cargo should be on PATH; the test harness also checks `~/.cargo/bin`.

```sh
python3 tools/setup_reference.py
python3 tools/check.py
```

The installer uses the pinned artifact and checksum in [references/compiler.json](references/compiler.json). The harness builds Korrust, compiles selected sources with both compilers, runs JVM checks, and writes dated reports.

To compile a source file directly:

```sh
cargo run --release --locked --offline -- benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/util/unsignedBinarySearch.kt -d target/androidbible
```

Open the [HTML reports](target/reports-html/index.html); see [report generation and automatic watching](reports/README.md#html-viewer).

Use [feature reports](reports/features/) to assess the tested subset and limitations. The command accepts one source file and an output directory. Pass compiled Java dependency directories with `-cp <classpath>`; separate multiple directories with the platform path separator (`:` on macOS/Linux). The differential harness prepares its Java dependencies automatically.

Agent workflow and review guidance lives in [AGENTS.md](AGENTS.md).

## Project contract

- Correctness comes first, then compilation performance. Runtime optimization is secondary.
- Preserve Kotlin behavior and the binary contracts required by Java and Kotlin consumers.
- Adopt per Gradle module; use official Kotlin for unsupported modules and production builds.
- Diagnose unsupported constructs explicitly. Never silently approximate their behavior.
- Use pinned official Kotlin tests and executables as references. Preserve upstream test directives and distinguish unsupported fixtures from failures.
- Validate incremental output against clean compilation, including declaration removal and stale-output cleanup.
- Treat a 2–5× incremental-compilation speedup as a target to measure, not an established result.

## Workspace

| Location | Purpose |
| --- | --- |
| [references/sources.json](references/sources.json) | Kotlin source pin, provenance, and useful upstream test paths |
| [references/compiler.json](references/compiler.json) | Official compiler artifact pin and checksum |
| [references/kotlin/](references/kotlin/) | Independent, shallow upstream checkout; keep it unchanged |
| [benchmarks/](benchmarks/README.md) | AndroidBible and Now in Android source snapshots and provenance |
| [reports/](reports/README.md) | Dated correctness results, feature evidence, and performance measurements |

The source pin initially follows AndroidBible's declared Kotlin version. Confirm the compiler version and options resolved by each app's build before claiming compatibility.

Keep docs limited to enduring decisions and how to find evidence. Record current capabilities, counts, timings, implementation details, and progress in dated reports or executable tests. Do not maintain a second feature checklist in prose.

Downloaded reference checkouts and app snapshots are excluded from a future Korrust Git repository. Their manifests and reports remain versionable. Do not change pins automatically when upstream moves.
