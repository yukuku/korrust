# AndroidBible source assessment, 2026-10-26

The current compiler emitted JVM classes for **20 of 400** unchanged Kotlin source files with the dependencies below. The other 380 probes rejected. A rejection records the first blocker and can mean a missing dependency; these counts are not whole-project language coverage. Source emission alone does not establish runtime correctness, ABI/metadata compatibility, or a full Android application build.

The compiler sources match the frozen 2026-10-25 delivery snapshot. This assessment rebuilt the compiler in debug mode and verified every Kotlin source against the pinned AndroidBible checksum inventory. Historical correctness and feature reports remain unchanged.

Per-file diagnostics, input hashes, emitted-class hashes and commands: [JSON evidence](2026-10-26T073039Z-androidbible-source-assessment.json).

## Files that emit classes

| Module | File |
| --- | --- |
| Alkitab | [RibkaEligibility.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/actionmode/RibkaEligibility.kt) |
| Alkitab | [AudioErrorCode.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/AudioErrorCode.kt) |
| Alkitab | [AudioLogEntry.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/AudioLogEntry.kt) |
| Alkitab | [VersesDialogNormalVerses.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/dialog/VersesDialogNormalVerses.kt) |
| Alkitab | [XrefDialogVerses.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/dialog/XrefDialogVerses.kt) |
| Alkitab | [InternalSingleChapterVerses.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/storage/InternalSingleChapterVerses.kt) |
| Alkitab | [VerseTextDecoder.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/storage/VerseTextDecoder.kt) |
| Alkitab | [BookColorUtil.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/BookColorUtil.kt) |
| Alkitab | [IntArrayListUtil.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/IntArrayListUtil.kt) |
| Alkitab | [Levenshtein.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/Levenshtein.kt) |
| Alkitab | [RequestCodes.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/RequestCodes.kt) |
| Alkitab | [ReaderGestureActions.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/widget/ReaderGestureActions.kt) |
| Alkitab | [VerseNumberPlacement.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/widget/VerseNumberPlacement.kt) |
| Alkitab | [SongPlaybackState.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/songs/SongPlaybackState.kt) |
| Alkitab | [SongTextZoomable.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/songs/SongTextZoomable.kt) |
| Alkitab | [Yes1SingleChapterVerses.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/yes1/Yes1SingleChapterVerses.kt) |
| AlkitabIo | [Utf8Decoder.kt](../../benchmarks/sources/androidbible/AlkitabIo/src/main/java/yuku/alkitab/io/Utf8Decoder.kt) |
| AlkitabModel | [SingleChapterVerses.kt](../../benchmarks/sources/androidbible/AlkitabModel/src/main/java/yuku/alkitab/model/SingleChapterVerses.kt) |
| AlkitabYes2 | [Yes2SingleChapterVerses.kt](../../benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/Yes2SingleChapterVerses.kt) |
| AlkitabYes2 | [unsignedBinarySearch.kt](../../benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/util/unsignedBinarySearch.kt) |

## Three small unsupported files

Physical line counts include blank lines. These examples have no external Android or application-library dependencies, so their first blockers are compiler limitations.

| File | Lines | First unsupported construct | Current diagnostic |
| --- | ---: | --- | --- |
| [Gid.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/datatransfer/model/Gid.kt) | 8 | `@JvmInline value class`: inline/value-class declarations and their JVM representation are unsupported. The parser reports an object-modifier error before reaching the class body. | `Unsupported object modifier; only public and internal are supported` |
| [GotoTab.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/compose/goto/GotoTab.kt) | 9 | `enum class GotoTab(val ordinal1Based: Int)` and entries such as `DIALER(1)`: enum constructor parameters and entry arguments are unsupported. The later type alias is not reached by this probe. | `Enum constructors, supertypes, and generics are not supported` |
| [LogInterface.kt](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/datatransfer/process/LogInterface.kt) | 10 | `fun interface LogInterface`: declaring a Kotlin functional interface is unsupported. The parser treats `fun` as the start of a function declaration and fails at `interface`. | `Expected an identifier` |

## Regression requiring follow-up

[`PlaybackState.kt`](../../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/audio/PlaybackState.kt) passed the [2026-10-24 twenty-feature suite](2026-10-24T094102532411Z-androidbible-twenty-features.json). Its unchanged source currently rejects with `Cannot infer a source declaration signature: cyclic or unsupported initializer or expression body; add an explicit type`. The companion `IDLE` property has an inferred type and is initialized with a `PlaybackState(...)` constructor call. It is excluded from the 20-file list; this assessment does not repair the compiler.

## Dependencies and limits

The classpath contains the checksum-pinned Kotlin 2.3.21 standard library, original Java `IntArrayList` and Bintex sources compiled with JDK 21, and current Korrust output for original `SingleChapterVerses.kt` and `AudioLogEntry.kt`. Existing test-only `Parcel`, `Parcelable`, and `NonNull` stubs provide Java types; they do not validate Android behavior. The standard library is unpacked with multi-release `META-INF/versions/` entries excluded. Other Android SDK, application, Compose, coroutine, serialization, and test-library dependencies are absent.

## Reproduce

Set `JAVA_HOME` to JDK 21 and put Cargo on `PATH`, as described in the project README. Fetch Cargo dependencies before using the offline build if the local cache is empty.

```sh
cargo build --locked --offline
mkdir -p target/compatibility-inspection
curl --fail --location --output target/compatibility-inspection/kotlin-stdlib-2.3.21.jar \
  https://repo.maven.apache.org/maven2/org/jetbrains/kotlin/kotlin-stdlib/2.3.21/kotlin-stdlib-2.3.21.jar
python3 reports/correctness/2026-10-26T073039Z-androidbible-source-assessment/reproduce.py \
  --stdlib target/compatibility-inspection/kotlin-stdlib-2.3.21.jar \
  --output target/compatibility-inspection/reproduced-assessment.json
```

The script verifies the standard-library checksum, builds the recorded dependency subset, checks every pinned Kotlin source, and records each probe using a fresh output directory. It runs compilation probes only.
