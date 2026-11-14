# Next ten bounded language features

Baseline: `720c490c9`. This records selection, not accepted support. Selection follows the November 7 completed milestone, its correctness reports, and inspection of unchanged AndroidBible sources including `BackForwardList`, `SongFilter`, `AudioHttp`, `AudioHttpErrorBody`, and `DownloadMapper`.

1. Source and compiled custom unary operators (`unaryPlus`, `unaryMinus`, `not`).
2. Source comparison operator declarations (`compareTo`), using existing ordinary call resolution and comparison lowering.
3. Custom `iterator`/`hasNext`/`next` operator protocols in for loops.
4. Open-ended range expressions and ordinary `rangeUntil` operator resolution.
5. Declaration-site variance on supported source generic classes and interfaces.
6. Independent interface upper bounds on source classifier type parameters.
7. Integral numeric promotion, including Byte/Short arithmetic and mixed Int/Long arithmetic.
8. Short arrays through supported primitive-array operations and calling conventions.
9. Canonical `@file:JvmName` facade names and associated metadata/output ownership.
10. Nonpublic primary constructors with checked accessibility and JVM/Kotlin contracts.

The baseline already supports portions of Short scalar values and compiled comparison calls; those portions are prerequisites, not new features. Constructor visibility occurs directly in `SongFilter.CompiledFilter` and `DownloadMapper`. Other selected additions strengthen general language semantics; they do not by themselves establish new unchanged application coverage. The independent baseline scan compiles 21 of 33 selected small AndroidBible sources with the standard library and JDK only; it is neither a whole-app build nor runtime evidence.

Implementation uses five isolated worktrees with separate ownership. Acceptance requires bounded positive and negative fixtures, matching pinned-official inputs, independent runtime assertions, Java callers compiled against official output and reused against both providers with JVM verification where applicable, and relevant Kotlin consumer/signature checks. Final integration freezes compiler and fixtures before rerunning differential evidence, formatting, Clippy, Rust/Python tests, and HTML generation. Reports retain hashes, command logs, and derived per-case counts. Supporting correctness repairs do not increase the feature count. No commit, push, deployment, performance claim, or additional milestone is requested.
