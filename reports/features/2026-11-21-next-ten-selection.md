# Compiler feature selection for 2026-11-21

Baseline commit `c2ab87153` includes the previous ten accepted features. The November 13 survey compiled 22 of 33 selected unchanged AndroidBible files. Eight of the remaining eleven first failed while resolving application dependencies. For example, supplying the original sibling sources lets the baseline compile both PlaybackState with AudioLogEntry and Rpp with Gid. Neither pair represents a new feature.

| Feature to investigate | Scope and evidence for selection |
| --- | --- |
| Expected result types in explicit returns | QueryTokenizer returns `emptyArray()` from a function declared to return `Array<String>`. Explicit array component expectations also matter when an initializer calls a Java method with a platform result. |
| Nullability contracts from Kotlin library metadata | QueryTokenizer uses `query` after an early return guarded by `isNullOrEmpty()`. Test arbitrary compiled contracts and stable variables. |
| Reified reference array allocation | OldVerseTextDecoder and QueryTokenizer exercise compiled inline library code that allocates a concrete component array. The baseline rejects the reified bytecode operation. |
| Reference values in string templates | SyncUtils interpolates an interface parameter. JVM string conversion must preserve virtual dispatch and nullable values. |
| Checked exceptions on JVM methods | SyncUtils declares `@Throws` on an interface method. Object-method support already exists, including the unchanged Utf8Decoder with its original Bintex dependency. |
| Synchronized JVM methods | Expose Kotlin's method monitor semantics and flags to Java callers on the supported method owners. |
| Custom increment and decrement | Extend operator resolution to read, update, and write back supported assignable expressions, preserving prefix/postfix values. |
| Compound assignments on properties and indexed values | Preserve receiver/index evaluation order, setter semantics, and operator ambiguity checks. |
| JVM wildcard signatures from declaration-site variance | Build on the previous milestone's variance checking. Compare nested signatures and cross-compiler consumers. |
| Method type parameters inside generic classifiers | Ordinary methods with their own type parameters in nongeneric classes already work. Test independent method and classifier parameters and interface dispatch. |

The fixed AndroidBible survey, original app JUnit tests, and differential fixtures exercise different scopes. Accepted claims and exact boundaries belong in the completed reports.
