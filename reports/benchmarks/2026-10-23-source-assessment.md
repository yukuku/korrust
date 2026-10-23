# Kurtlin benchmark corpus

Captured on 2026-10-23 (Asia/Singapore). These are source snapshots for compiler development, compatibility testing, and future incremental-compilation benchmarks. No build timings or compiler speedups have been measured.

## Source snapshots

| Project | Location | Role |
| --- | --- | --- |
| AndroidBible | [sources/androidbible](../../benchmarks/sources/androidbible) | First application compatibility target; mixed Java/Kotlin interop and existing application tests |
| Now in Android | [sources/nowinandroid](../../benchmarks/sources/nowinandroid) | Public modular Kotlin/Compose workload; cross-module invalidation and generated-code integration |

[sources.json](../../benchmarks/sources.json) records upstream URLs, exact commits, capture time, and copy policies. [source-files.sha256.json](../../benchmarks/source-files.sha256.json) records the initial checksum of every regular file and the target of every symlink.

AndroidBible was copied from the tracked working-tree files in `/Users/yuku/f/git/androidbible`. They were clean against HEAD when copied. Untracked files, ignored machine configuration, Git internals, and generated build directories were omitted. Now in Android is the complete GitHub source archive at the recorded commit. Both snapshots retain upstream licenses and build files but omit Git history. They are independent copies, not links to the original checkout.

Initial inventories, counting the entire snapshots including tests and tools:

| Project | Files and symlinks | Kotlin .kt files | Java .java files | Modules included by root settings |
| --- | ---: | ---: | ---: | ---: |
| AndroidBible | 1,800 | 400 | 249 | 7 |
| Now in Android | 715 | 310 | 0 | 35 |

Generated sources are not included in these language counts. Gradle Kotlin scripts are also excluded from the .kt counts.

## Now in Android assessment

Now in Android complements AndroidBible because it spreads Kotlin across core libraries and feature API/implementation modules. This provides useful module boundaries for gradual Kurtlin adoption and for testing whether an edit recompiles the correct consumers. It does not establish performance at the scale of a million-line application.

The snapshot declares Kotlin 2.3.0, AGP 9.3.2, and KSP 2.3.4 in its version catalog. AndroidBible declares Kotlin 2.3.21, AGP 9.2.1, and KSP 2.3.8. Verify the resolved compiler versions and options from actual Gradle tasks before benchmarking; declarations alone are not a measured build environment.

Recommended progression:

1. Compile small AndroidBible functions to establish the complete Rust-to-JVM compilation path.
2. Target Now in Android's `core:model` as an early complete-module milestone. Its build applies the JVM library convention and depends on kotlinx.datetime, without applying Compose or KSP. Source examples include data classes, nullable properties, generic collections, secondary constructors, named arguments, extension functions, and collection lambdas.
3. Test official Kotlin consumers against Kurtlin-produced model artifacts, including generated data-class members and Kotlin metadata.
4. Expand into coroutine and dependency-injection modules. `core:common` applies the Hilt convention, which applies KSP, so it is not a plugin-free next step.
5. Add Room/KSP, serialization, protobuf-generated Java/Kotlin, and Compose integration as separate milestones.

The existing `benchmarks` module and benchmark app variant concern runtime performance and baseline profiles. Kurtlin needs its own compilation benchmark harness.

Use the documented `demoDebug` application variant for eventual end-to-end validation. The demo flavor uses local data; production variants depend on a backend that is not publicly available.

## Incremental benchmark scenarios

- No-op rebuild, to measure build-system overhead separately.
- Ordinary function-body edit with unchanged externally visible information.
- Public model/API change that requires downstream consumers to recompile.
- Inline-body or compile-time-constant change.
- Added overload, import change, and declaration removal.
- Compose function-body edit.
- Changes that affect generated sources.

Measure Kotlin compilation, source generation, Java compilation, and total build wall time separately. Record warm/cold daemon and cache conditions, the modules handled by each compiler, and fallback costs. Compare incremental results with clean compilations after every edit sequence.

The immediate benchmark scope is supported modules, followed by the full app. Partial-module results must not be presented as whole-app speedups.

## Inspected sources

- [Now in Android README](../../benchmarks/sources/nowinandroid/README.md)
- [Module settings](../../benchmarks/sources/nowinandroid/settings.gradle.kts)
- [Modularization guide](../../benchmarks/sources/nowinandroid/docs/ModularizationLearningJourney.md)
- [Dependency versions](../../benchmarks/sources/nowinandroid/gradle/libs.versions.toml)
- [Gradle cache and daemon configuration](../../benchmarks/sources/nowinandroid/gradle.properties)
- [Model module](../../benchmarks/sources/nowinandroid/core/model/build.gradle.kts)
- [Model source example](../../benchmarks/sources/nowinandroid/core/model/src/main/kotlin/com/google/samples/apps/nowinandroid/core/model/data/UserNewsResource.kt)
- [Hilt and KSP convention](../../benchmarks/sources/nowinandroid/build-logic/convention/src/main/kotlin/HiltConventionPlugin.kt)
- [Network and serialization module](../../benchmarks/sources/nowinandroid/core/network/build.gradle.kts)
- [Database module](../../benchmarks/sources/nowinandroid/core/database/build.gradle.kts)
- [Protobuf generation](../../benchmarks/sources/nowinandroid/core/datastore-proto/build.gradle.kts)
- [AndroidBible build configuration](../../benchmarks/sources/androidbible/Alkitab/build.gradle.kts)

## Validation performed

Every copied file was verified against its recorded SHA-256 checksum; symlinks were checked against their recorded targets. AndroidBible regular files were also compared directly against the original checkout after copying. No Git, Gradle, Kotlin cache, or build-output directories were present in either snapshot.

No Gradle builds or application tests were run. Machine-specific SDK configuration and dependencies may still be required to build these copies.
