# Reference policy

The pinned [Kotlin language specification](kotlin-spec/) defines the language rules it covers. Its version and coverage may lag Kotlin 2.3.21; do not assume that it describes every current feature or JVM detail. Consult it when implementing or reviewing language behavior.

The [official compiler sources and tests](kotlin/) and the compiler distribution pinned in [compiler.json](compiler.json), including its standard library, provide the differential oracle for implementation behavior, JVM binary contracts, and behavior missing from the specification. Compare matching inputs, classpaths, and options. Preserve upstream test directives and distinguish unsupported language or harness directives from correctness failures. Record suspected compiler bugs and specification discrepancies in dated [reports](../reports/README.md), with reproductions and an explicit resolution; do not silently follow an incorrect oracle.

Implement reusable language and interop rules. Library APIs should work through those rules and the matching library classpath, rather than acquiring a custom compiler implementation for each API. Any bounded support must be diagnosed and documented honestly.

[AndroidBible and Now in Android](../benchmarks/README.md) are application compatibility targets, not language standards. The source pin initially follows AndroidBible's declared Kotlin version; verify the compiler version and options resolved by an app's build before claiming compatibility.

Reference sources and app snapshots are tracked unchanged with upstream licenses. Compiler binaries under `kotlinc/` and generated build outputs remain ignored. [sources.json](sources.json), [compiler.json](compiler.json), and the [benchmark manifests](../benchmarks/sources.json) record provenance. Upgrade pins explicitly with dated provenance and refreshed checksums; never follow upstream automatically.

After cloning, run `git lfs install` and `git lfs pull` to fetch the five upstream compiler documentation images, then run `python3 tools/verify_references.py` to check all four source snapshots against their SHA-256 inventories. The source trees omit upstream Git history; revisions are recorded in the manifests. Build upstream projects only in disposable copies.
