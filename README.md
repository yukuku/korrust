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

Use a dedicated output directory. Korrust records ownership of emitted classes and Kotlin package metadata there and refuses to overwrite unowned or externally modified outputs. Recompiling a source removes obsolete outputs previously emitted by that source.

Agent workflow and review guidance lives in [AGENTS.md](AGENTS.md).

## Implemented features

These are tested subsets; see the [feature reports](reports/features/) for limits and evidence.

1. Int arithmetic, bitwise operations, shifts, comparisons, and Boolean expressions.
2. Top-level functions, direct calls, local variables, and basic assignments.
3. Conditional branches, while loops, for-until loops, and loop control.
4. Int and Char when statements and expressions.
5. UTF-16 strings, common string operations, and simple string interpolation.
6. Char and Byte values with supported numeric conversions.
7. Primitive arrays with construction, indexing, writes, and selected copying operations.
8. String arrays with indexing and constructor-field support.
9. Character conversion APIs and String construction from character arrays.
10. Java instance and static method calls through directory classpaths.
11. Bare enum classes with generated JVM enum methods.
12. Named and nested objects containing literal constants.
13. Object methods with selected JVM annotations and exact-signature overloads.
14. Thread-local array buffers with lazy initialization.
15. Ordinary final classes with fields, getters, methods, and supported interfaces.
16. Source interfaces with abstract members and supported default methods.
17. Interface companions with string arrays and anonymous implementations.
18. Abstract mutable interface properties and inherited property overrides.
19. Kotlin metadata for supported declarations and package mappings.
20. Empty serializable singleton objects implementing marker interfaces.
21. Long values, arithmetic, bitwise operations, comparisons, and conversions to strings.
22. Public read-only constructor properties with generated getters.
23. Data classes with generated components, copy, equality, hash, and string methods.
24. Nullable references and primitives with boxing, null comparisons, and not-null assertions.
25. Safe calls returning references or boxed primitives.
26. Elvis expressions with lazy fallbacks, returns, and throws.
27. Typed lists of strings, primitives, nullable values, and application objects.
28. String, Boolean, and Long when subjects.
29. Subjectless when statements and expressions.
30. Braced string templates containing supported expressions.
31. Owned-output tracking and stale-output cleanup when recompiling a source.
32. Top-level literal constants with Kotlin metadata.
33. Java field reads and writes, including volatile fields.
34. Bounded overload resolution for Kotlin and Java calls.
35. Implicit no-argument constructors.
36. Constructor parameters that do not become properties.
37. Mutable constructor properties with generated getters and setters.
38. Instance properties initialized in declaration order.
39. Source-defined constructor calls with named arguments.
40. Default function, method, and constructor arguments.
41. Companion properties initialized once in declaration order.
42. Nullable and list-typed class and interface member signatures.
43. Null-check smart casts for stable local variables and parameters.
44. List factories with selected mutable-list operations.
45. Data-class properties containing nullable values, lists, and Float values.
46. Data-class body methods and computed properties.

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

Keep the requested numbered feature overview here concise and grounded in dated reports. Record detailed capabilities, limitations, counts, timings, implementation details, and progress in those reports or executable tests.

Downloaded reference checkouts and app snapshots are excluded from a future Korrust Git repository. Their manifests and reports remain versionable. Do not change pins automatically when upstream moves.
