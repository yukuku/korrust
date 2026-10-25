# Korrust

A Kotlin/JVM compiler written in Rust for faster incremental Android development builds.

Created by **Randy Sugianto (Yuku)** ([**@yukuku**](https://github.com/yukuku)).

## Run

Install Rust, Python, and JDK 21, and set `JAVA_HOME` to that JDK. Cargo should be on PATH; the test harness also checks `~/.cargo/bin`.

```sh
cargo fetch --locked
python3 tools/setup_reference.py
python3 tools/check.py
```

The installer uses the pinned artifact and checksum in [references/compiler.json](references/compiler.json). The harness builds Korrust, compiles selected sources with both compilers, runs JVM checks, and writes dated reports.

To compile a source file directly:

```sh
cargo run --release --locked --offline -- benchmarks/sources/androidbible/AlkitabYes2/src/main/java/yuku/alkitab/yes2/util/unsignedBinarySearch.kt -d target/androidbible
```

Open the [HTML reports](target/reports-html/index.html); see [report generation and automatic watching](reports/README.md#html-viewer).

Use [feature reports](reports/features/) to assess the tested subset and limitations. Run `python3 tools/check_declarations_twenty.py` after a release build for the declaration, control-flow, and operator milestone checks. The command accepts one source file and an output directory. Pass dependency directories or JAR files with `-cp <classpath>`; separate entries with the platform path separator (`:` on macOS/Linux). JDK classes are read from `JAVA_HOME/jmods/java.base.jmod` after explicit dependencies. The differential harness prepares its dependencies automatically.

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
47. Dependencies loaded directly from JAR files.
48. Kotlin library metadata for classes, functions, and properties.
49. Imported top-level declarations across compiled files and modules.
50. Java generic signatures decoded from dependency class files.
51. Type parameters, substitution, and checked generic bounds.
52. Generic call type arguments inferred from supported argument constraints.
53. Generic inheritance, variance, and supported type projections.
54. Library nullability annotations and Java platform types.
55. Overload resolution using generic constraints and candidate specificity.
56. Extension functions resolved from compiled Kotlin libraries.
57. Library properties accessed through their getters and setters.
58. Named and default arguments for imported Kotlin functions.
59. Function types with boxed JVM invocation bridges.
60. Lambdas capturing immutable locals and shared mutable cells.
61. Lambda parameter and result types inferred from expected contexts.
62. Bound, unbound, and top-level function references.
63. Inherited member resolution and virtual or interface dispatch.
64. Lambda conversion to Java single-method interfaces.
65. Supported compiled inline bytecode expanded at call sites.
66. Non-local returns from supported inline lambda arguments.
67. Inclusive Int range loops with overflow-safe termination.
68. Name-independent infix calls to supported source and compiled functions.
69. Iteration over runtime progressions returned by ordinary resolved calls.
70. Iteration over supported primitive arrays.
71. String iteration over UTF-16 code units.
72. Typed List iteration through JVM iterators.
73. Compiled extension-property resolution, including indices used in loops.
74. Do-while loops with break and continue.
75. Float arithmetic and supported mixed numeric operands.
76. Float ordering and equality with Kotlin NaN and signed-zero behavior.
77. Float, Int, and Long numeric conversions.
78. Numeric local compound assignments.
79. Prefix and postfix Int-local decrement.
80. Nullable reference identity comparisons.
81. Nullable and object structural equality.
82. Runtime type tests for supported concrete types.
83. Checked casts with null and wrong-type failures.
84. Safe casts with nullable results.
85. Type-check smart casts for stable locals and parameters.
86. When type-test branches with supported smart casts.
87. Signed integer literals with hexadecimal/binary notation, separators, suffixes, and range checking.
88. Mixed top-level functions, properties, classes, and objects in one source file.
89. Shared executable-body support for class, object, and interface methods and accessors.
90. Object and companion state initialized through ordinary expressions.
91. Initialization blocks and declaration-order property initialization.
92. Nested declarations with JVM nesting metadata.
93. Local functions with captures and recursion.
94. Operator-based contains calls for in, !in, and when branches.
95. Concrete generic arrays with construction, indexing, writes, and iteration.
96. Receiver function types and receiver lambdas with captures.
97. Labeled returns, breaks, and continues within supported control-flow boundaries.
98. Try/catch/finally expressions and abrupt exits.
99. Operator-based get/set indexing through source and compiled declarations.
100. Destructuring declarations in locals, loops, and lambdas.
101. When subject variables with scoped bindings.
102. Custom property accessors and backing fields.
103. Delegated instance properties with getValue, setValue, provideDelegate, and property metadata.
104. Late-initialized reference properties with checked reads.
105. Open and abstract classes, inheritance, overrides, protected access, and super calls.
106. Secondary constructors and constructor delegation.

## Project contract

- Correctness comes first, then compilation performance.
- Prioritize reusable language semantics and general library-call resolution that advance unchanged AndroidBible source compilation. Library function names are examples of usage, not separate compiler features.
- Defer runtime optimization and API-specific shortcuts until explicitly requested. Execute library functions through normal resolution and calling conventions; do not substitute implementations based on names such as `downTo` or `step`.
- Preserve Kotlin behavior and the binary contracts required by Java and Kotlin consumers.
- Adopt per Gradle module; use official Kotlin for unsupported modules and production builds.
- Diagnose unsupported constructs explicitly. Never silently approximate their behavior.
- Use the pinned Kotlin specification for covered language rules and the official compiler, standard library, and tests for version-specific behavior and JVM contracts; follow the [reference policy](references/README.md).
- Validate incremental output against clean compilation, including declaration removal and stale-output cleanup.
- Treat a 2–5× incremental-compilation speedup as a target to measure, not an established result.

## Workspace

| Location | Purpose |
| --- | --- |
| [references/](references/README.md) | Language specification, compiler oracle, and reference policy |
| [references/sources.json](references/sources.json) | Kotlin source pin, provenance, and useful upstream test paths |
| [references/compiler.json](references/compiler.json) | Official compiler artifact pin and checksum |
| [references/kotlin/](references/kotlin/) | Tracked, unchanged upstream compiler sources and tests |
| [references/kotlin-spec/](references/kotlin-spec/) | Tracked, unchanged upstream language specification |
| [benchmarks/](benchmarks/README.md) | AndroidBible and Now in Android source snapshots and provenance |
| [reports/](reports/README.md) | Dated correctness results, feature evidence, and performance measurements |

The source pin initially follows AndroidBible's declared Kotlin version. Confirm the compiler version and options resolved by each app's build before claiming compatibility.

Keep the requested numbered feature overview here concise and grounded in dated reports. Record detailed capabilities, limitations, counts, timings, implementation details, and progress in those reports or executable tests.

Reference sources and app snapshots are tracked unchanged with their provenance and upstream licenses. Downloaded compiler binaries and generated build outputs remain ignored. Change pins explicitly and record dated provenance; do not follow upstream automatically.
