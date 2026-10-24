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

This numbered overview summarizes the tested subset. Exact boundaries and reproducible evidence are in the dated [core and Io report](reports/features/2026-10-24T035452569386Z-io-byte-arrays.json), [mutable-interface report](reports/features/2026-10-24T035456509533Z-androidbible-simple-sources.json), [serialization report](reports/features/2026-10-24T035505616295Z-nowinandroid-api-serialization.json), and [ten-addition report](reports/features/2026-10-24T040039672008Z-androidbible-ten-features.json). Support applies to these bounded forms, rather than all combinations of Kotlin features.

1. **Int expressions:** arithmetic, bitwise operations, shifts, comparisons, unary operators, and Boolean short-circuit evaluation.
2. **Functions and locals:** single-file top-level functions with explicit supported signatures, direct calls, Unit bodies/returns, local `val`/`var`, Int `+=` and postfix increment, and tested definite assignment. General overload resolution, named/default arguments, and function values remain unsupported.
3. **Branches and loops:** statement/block `if`/`else`, supported `if` expressions, `while`, `for` over Int `until`, `return`, `break`, and `continue`; bounded Int ranges and membership. Return analysis is conservative.
4. **Int/Char `when`:** statements and bounded expressions, ordered comma conditions, Int range conditions, nested branches, and once-only subject evaluation. No subject declarations, guards, type tests, collection conditions, or trailing condition commas.
5. **Strings:** UTF-16 ordinary escaped literals, parameters/results/locals, structural equality, String-left concatenation, `length`, `isEmpty`, `startsWith`, `endsWith`, `removePrefix`, `substring`, and `compareTo`; simple ASCII `$name` interpolation.
6. **Char and Byte scalars:** UTF-16 Char literals/comparisons, signed Byte signatures, and supported Int/Byte/Char conversions. Char literals contain one UTF-16 code unit; general Char arithmetic is unsupported.
7. **Primitive arrays:** nonnull `IntArray`, `ByteArray`, and `CharArray` signatures, size constructors, size, indexed reads/writes, and bounded `copyOf`; IntArray four-argument `copyInto` preserves overlap behavior and destination identity. No general generic-array construction or default/named copy arguments.
8. **String arrays:** bounded `Array<String>` signatures, constructor fields, size/indexed reads, and companion String-literal arrays. General array element types and operations remain unsupported.
9. **Character conversion APIs:** `String(CharArray, offset, count)` and bounded `Character.toLowerCase`, `highSurrogate`, and `lowSurrogate` intrinsics, with tested Unicode and exception behavior.
10. **Java interop:** directory classpaths, explicit class imports, declared public instance/void calls, scoped platform results and extension receivers, and supported primitive/array descriptors. General JAR classpaths, Java construction, fields, static/interface/inherited calls, overloads, generics, varargs, and annotated APIs remain unsupported; listed intrinsics are bounded exceptions.
11. **Bare enums:** one public/default/internal top-level enum per file, including empty forms and up to 1024 plain entries; generated JVM enum APIs, stable `EnumEntries`, generic signatures, and Java collection/serialization behavior. Constructors, custom members, entry bodies, and Kotlin metadata are unsupported.
12. **Named and nested constant objects:** singleton identity and literal `const val` Int/Boolean/Char/String fields, up to nesting depth 16, 256 classes per source, and 1024 constants per object. Constant expressions/references, Long constants, and constant-object metadata remain unsupported.
13. **Object methods:** public/internal ordinary objects with supported methods, exact-signature overloads, `@JvmStatic`, `@Throws`, `@Suppress`, and bounded private-inline forms; tested Java/Kotlin ABI and metadata.
14. **Thread-local buffers:** inferred private `ThreadLocal.withInitial` fields with noncapturing single array-allocation suppliers, supported `get`/`set` and not-null assertions, lazy allocation, and thread isolation. Arbitrary lambdas/captures and general initialized object fields remain unsupported.
15. **Ordinary final classes:** one public/default/internal final class per source, explicit primary constructor, private `val` fields, getter properties, and straight-line supported method bodies; validated Kotlin interface implementations and default bridges. No general inheritance, mutable fields, constructor defaults, or arbitrary initialization.
16. **Source interfaces:** supported abstract members and bounded default bodies, nested inherited interfaces, default dispatch, `Int.toString`, and `IllegalArgumentException` throws. Float support is restricted to abstract interface descriptors; reference parameters in default bodies are unsupported.
17. **Interface companions and anonymous implementations:** bounded private String-literal arrays and public anonymous implementation values, with tested initialization and identity. No named companions, arbitrary initializers, or general object construction.
18. **Mutable interface properties:** abstract nonnull `var` properties for Int/Byte/Char/Boolean/Float/String and the supported arrays; getter/setter naming, nested/inherited properties, and `var`-over-`val` overrides. Concrete mutable class properties, custom/default mutable accessors, and Korrust-compiled property assignment remain unsupported.
19. **Kotlin metadata:** supported top-level functions, bounded classes/interfaces/companions/dynamic objects, and per-source package mappings; official Kotlin consumer checks cover public imports, internal/friend/private access, and nullability. Enum and constant-only object metadata remain absent.
20. **Empty serializable objects:** top-level public/internal empty `@Serializable` singleton objects implementing bounded marker interfaces, generic serializer ABI, direct/reified discovery, and JSON/polymorphic singleton behavior. No general serialization-plugin support, serializable classes, nonempty/nested/data objects, or custom serializers/names.
21. **Long scalars:** parameters/results, signed decimal/hex/binary literals, expected-type Int literals, MIN/MAX constants, arithmetic, bitwise operations, Int shift counts, comparisons, negation, `toString`, and two-slot locals. No unsigned arithmetic, floating conversions, or general mixed numeric coercion.
22. **Public constructor properties:** ordinary final-class public/default read-only `val` properties, private backing fields, getters including `is` names, mixed private/public fields, and supported nullable String/string-list metadata. No mutable constructor properties or custom accessors.
23. **Data classes:** 1–32 public `val` constructor properties of nonnull Int/Byte/Char/Boolean/Long/String; getters, components, `copy`, generated copy defaults/masks, equality/hash/text, and Kotlin copy/destructuring metadata. No bodies, interfaces, nullable/list/array/mutable properties, or constructor defaults.
24. **Nullable references:** selected String/reference and exact string-list types, null literals, null comparisons, constructor fields, and metadata; supported reference not-null assertions. No primitive boxing, nullable-to-nullable equality, smart casts, general subtype inference, or nullable class-method/interface signatures.
25. **Safe calls:** supported reference-returning method calls, chaining, once-only receivers, skipped arguments for null, and nullable result metadata. Primitive-result safe calls and general optional property access are unsupported.
26. **Elvis expressions:** reference `?:` with lazy fallback, preserved evaluation order, and nullable/nonnullable result types. No primitive boxing or general common-supertype inference.
27. **String lists:** exact read-only `List<String>`/`List<String?>` signatures, constructor fields, top-level parameters/results, size/indexed reads, and element-nullability metadata. No factories, mutation, arbitrary element types, or generic declarations.
28. **String/Boolean/Long `when`:** supported statement/expression subjects, structural String matching, ordered comma conditions, lazy branches, and explicit `else`; no exhaustiveness inference.
29. **Subjectless `when`:** ordered Boolean conditions, short-circuit selection, and lazy statement/expression branches; expressions require `else`. No smart casts or general block-expression branches.
30. **Braced string templates:** ordinary `${expression}` interpolation, nested quoted literals, arithmetic/Elvis expressions, ordered effects, and UTF-16/NUL/surrogate preservation. No raw triple-quoted templates, escaped identifier interpolation, or arbitrary block expressions.
31. **Owned output lifecycle:** safe ownership checks, stale class/package-metadata cleanup on explicit source recompilation, and preservation after compile/preflight errors. No incremental dependency graph, deleted-source discovery, compilation cache, or measured incremental speedup; publication filesystem failures are not fully transactional.

Integration evidence covers the pinned complete AndroidBible **AlkitabModel, AlkitabIo, and AlkitabYes2** modules (4 unchanged Kotlin and 39 Java production sources), real release AARs, original tests, JVM verification, consumers, and strict AGP D8 in the [module report](reports/features/2026-10-23T073136152035Z-androidbible-model-io-yes2.json). The serialization report covers complete unchanged Now in Android **feature:bookmarks:api** and **feature:search:api** modules with real Gradle/AGP AAR builds; nonselected dependencies and build logic use official compilers. The mutable-interface and ten-addition reports add eight selected unchanged Alkitab source files with consumer/ABI and standalone D8 checks. These scopes establish neither full-app/device compatibility nor performance gains.

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
