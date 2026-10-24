# Benchmark inputs

- [AndroidBible](sources/androidbible/): mixed Java/Kotlin application compatibility.
- [Now in Android](sources/nowinandroid/): modular Kotlin/Compose application compatibility.

[sources.json](sources.json) records exact source revisions and copy policies. [source-files.sha256.json](source-files.sha256.json) records the imported files.

These unchanged snapshots are tracked in Git with their upstream licenses. Treat them as application compatibility targets, not language standards; see the [reference policy](../references/README.md). Run edits and builds in disposable working copies, preserving the baseline and recording patches with benchmark results.

Upgrade pins explicitly and record dated provenance and checksums. Generated build outputs remain ignored.

The initial source assessment is archived in [reports](../reports/benchmarks/2026-10-23-source-assessment.md). Store subsequent measurements under [reports/benchmarks](../reports/benchmarks/).
