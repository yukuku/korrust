# Reports

Reports are dated evidence snapshots. Add a new report when results change; do not rewrite old results to match current code. A correction should identify the report it supersedes.

| Directory | Contents |
| --- | --- |
| [correctness/](correctness/) | Fixture selection, outcomes, diagnostics, runtime and interoperability checks |
| [features/](features/) | Feature support claims linked to correctness evidence, with scope and restrictions |
| [benchmarks/](benchmarks/) | Raw timings, environment, workloads, and source assessments |

Use UTC timestamps and descriptive filenames, such as `2026-10-23T030000Z-box-tests.json`. Human analysis may accompany the JSON as a Markdown file. Keep logs and edit patches in a sibling directory named after the report.

Each JSON report records `schema_version`, `kind`, `created_at`, `status`, compiler identity, reference/input revisions, reproduction commands, and evidence paths. Paths in JSON are relative to the Korrust root. The setup records demonstrate the envelope; measurement tools should write future records directly.

## HTML viewer

Open [the generated report index](../target/reports-html/index.html). JSON remains the source of truth; HTML is disposable output under `target/`.

```sh
python3 tools/reports.py                 # Generate once
python3 tools/reports.py --install-watch # Watch in the background on macOS, including after login
python3 tools/reports.py --uninstall-watch
```

The watcher checks for changes recursively every second and waits for a quiet interval before rebuilding. Refresh the browser to see updates. For a foreground watcher on macOS or Linux, use `python3 tools/reports.py --watch`. The test harness also regenerates HTML when it exits, including after a failed run. Watcher logs are in `target/reports-watch.*.log`.

Feature tables can be collapsed by status. Evidence links lead to the supporting correctness cases; report pages retain commands, logs, provenance, and source links. Unknown measurements remain unmeasured. To check the viewer, run `python3 -m unittest discover -s tests -p 'test_reports.py'`.

## Correctness

Record stable upstream fixture paths, directives/options, selected scope, and per-case outcomes. Separate passes, failures, timeouts, unsupported language, unsupported harness directives, excluded targets, and tests not run. Unsupported and expected-failure cases are not passes. Derive counts from case results; do not store an independently edited pass percentage. Compare observable behavior and required binary contracts; exact upstream IR or optimization output is not automatically a Korrust requirement.

## Features

A feature entry names its scope, restrictions, and supporting correctness report/case IDs. Use `unassessed`, `unsupported`, `partial`, or `supported`. Plans and successful parsing alone do not establish support. Empty feature lists mean no support claims have been established.

## Benchmarks

Record input commits and edit patches, compiler/build identity, flags, JDK and tool versions, machine/OS, target tasks, warm-up, daemon/cache conditions, and raw timed samples with explicit units. Separate Kotlin compilation, source generation, Java compilation, and total build time. Record fallback modules and costs. Compare equivalent workloads and validate their outputs before reporting speedups.

## Unmeasured state

Use `not_run` or `not_assessed`, explain why, and leave unknown metrics as `null` or omit them. Zero milliseconds and zero failures must never stand in for missing measurements. The initial setup reports are inventories of readiness, not successful compiler runs.
