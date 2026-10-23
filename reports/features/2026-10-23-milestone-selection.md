# AndroidBible milestone selection

Decision recorded 2026-10-23. Input: pinned AndroidBible commit `624629093af37d0d7f85699bfa0caa562fc41e61`.

The next implementation milestone is non-null String support, validated by compiling `Alkitab/src/main/java/yuku/alkitab/base/audio/AudioErrorCode.kt` unchanged. Its `stripErrorCodePrefix` function requires an internal top-level function, String parameters/results, a string literal, and the standard-library `removePrefix` operation. The existing application has three focused tests for this function. This is a small, testable expansion into a widely used type; it does not establish general String API or application compatibility.

## Evidence used to choose the milestone

Run `python3 tools/corpus.py` after building the release compiler to reproduce the standalone compilation probes and syntax inventory. The baseline report is `reports/correctness/2026-10-23T032048127249Z-androidbible-corpus.json`.

| Observed syntax | Files containing it, out of 400 |
| --- | ---: |
| Imports | 369 |
| Class declarations, including data/enum classes | 313 |
| String literals | 273 |
| String type/name token | 246 |
| Nullable syntax or safe calls | 234 |
| Lambda or block arrows | 205 |
| Object declarations/expressions | 182 |
| For loops | 146 |
| When expressions/statements | 86 |
| Data classes | 59 |
| Internal modifier/name token | 51 |
| Interfaces | 43 |
| Suspend modifier/name token | 10 |

These are overlapping regex observations across all .kt files, including tests and tools. They exclude Gradle scripts and generated sources. Comment/string masking is heuristic and does not fully parse nested comments or template expressions. These numbers are prioritization evidence, not supported-feature coverage percentages. The baseline emitted one file and rejected 399; standalone rejection only records the first blocker and does not validate the complete application or its dependencies.

## Correctness gate

Compile the original AndroidBible sources and synthetic semantic probes with pinned official Kotlin and Kurtlin. Compile Java callers against official output, then run them against both outputs under full JVM verification. Compare independent assertions, deterministic UTF-16 results, JVM method signatures, rejection behavior, and null-boundary behavior. Include NUL, supplementary characters, isolated surrogates, escapes, distinct-but-equal strings, concatenation ordering, and invalid substring bounds. Keep unsupported templates, nullable types, and other constructs explicit. Dated correctness and feature JSON reports contain the actual outcomes.

## Following milestones

1. Imports and Java interop, targeting `Alkitab/src/main/java/yuku/alkitab/base/util/IntArrayListUtil.kt`: classpath lookup, Java method signatures, extension receivers, IntArray allocation, array copy, and Unit/expression statements. Test against the original Java IntArrayList class. This is a proposed next target, not implemented support.
2. Nullable reference types, safe calls/Elvis, basic classes/properties/constructors, and cross-file resolution. Test ordinary classes before data/value classes, whose generated contracts add complexity.
3. Kotlin metadata and official Kotlin consumers, then a complete supported module and incremental dependency invalidation. An incremental build must match a clean build after body edits, API edits, additions, and removals before speedup claims.

Compose, coroutines, serialization, and KSP remain later integration milestones. The String milestone does not measure incremental compilation or establish the 2–5x target.
