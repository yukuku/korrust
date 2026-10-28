# New Kotlin/JVM families: execution record

Requested scope: approximately twenty substantial new families, guided by the pinned official fixtures and unchanged AndroidBible sources. This is a plan and execution record, not a support claim. The previous thirty-task delivery is complete and excluded from the new count.

Starting revision: `59c4502f8`. The unchanged local release baseline has SHA-256 `e5efeaa7df7ea4ab6392b4ada5fd6558ca79b1bef2d7271e49785a66eb3002ee` and is preserved in `target/new-families-baseline/korrust`. This host required an isolated Rust 1.99.1 installation because installed Rust 1.86 rejects existing let chains. Kotlin 2.3.21 was installed using the existing pinned archive checksum. Five existing documentation JPEG modifications are preserved and excluded from delivery commits.

## Selection and acceptance boundaries

All rows initially remain unassessed. Existing generic functions, objects, ordinary annotations, literal constants, aliases, inline library calls and primitive arrays are regressions, not new families. Selection is based on the current implementation's explicit rejections, upstream originals, and actual application constructs.

| Order | New semantic scope | Initial application priority / acceptance boundary |
| --- | --- | --- |
| 1 | Raw/multiline string templates and standard multiline expression/navigation parsing | UTF-16, raw escapes, line normalization, nested interpolation, newline grammar; original AudioHttpErrorBody and raw-literal tests. Multi-dollar interpolation remains outside the initial boundary. |
| 2 | Source generic classifiers, parameterized inheritance and erased override bridges | AlphanumComparator; bounds, substitution, variance, Java dispatch, signatures and Kotlin metadata. Existing generic functions do not count. |
| 3 | Independent accessor visibility and internal property contracts | BackForwardList; private setters, inaccessible callers, internal JVM names, friend and nonfriend consumers. |
| 4 | JVM overload generation for default arguments | FormattedVerseText and SongInfo; top-level/member/object/companion/constructor overloads, masks, effects, collisions and annotations. |
| 5 | Remaining signed primitive/array representations | Short and Long/Boolean/Float/Double arrays; allocation, indexed read/write, iteration, vararg/spread, boxing and Java ABI. AudioHighlightColor needs real Android dependencies. |
| 6 | Class-reference expressions and bounded KClass/Class interoperability | Primitive/reference/array literals, nullable expression checks and ordinary .java resolution; no full reflection. |
| 7 | Single-field value-class JVM ABI | Gid and Rpp; reference/primitive representation, nullable/generic/interface boxing, mangling and official Kotlin/Java consumers. Experimental multi-field values excluded. |
| 8 | Sealed hierarchies and exhaustive when | VersionFileImporter.Result and AudioBarCommand; exhaustiveness, nullable subjects, module restrictions and metadata. |
| 9 | Inner/local classes and captured state | Actual AndroidBible inner adapters; outer receiver/capture identity, constructor ABI and lexical metadata. Existing nested classes/local functions excluded. |
| 10 | Class/interface delegation | Ordinary interface forwarding, properties, override precedence, evaluation order and generic bridge boundaries. Existing delegated properties excluded. |
| 11 | Expanded object semantics | Data objects, supported supertypes and object expressions; initialization, equality/hash/string behavior and metadata. Existing named singleton support excluded. |
| 12 | Expanded enum semantics | Entry-specific subclasses, interface implementations and constructor defaults; initialization, dispatch, signatures and metadata. Existing enum bodies/arguments excluded. |
| 13 | Generic source aliases | Substitution, bounds, visibility, cycles and exported metadata/Kotlin consumers. Existing nongeneric aliases excluded. |
| 14 | Unsigned representations and operations | Ordinary stdlib resolution through value-class ABI, arithmetic/comparison/conversion/boxing and array boundaries; no API-name substitutions. |
| 15 | Compile-time constant expressions and references | VersionFileImporter MAX_FILE_SIZE and SongBookUtil MAX_RESPONSE_SIZE; overflow, types, dependency constants and annotation values. Existing literals excluded. |
| 16 | Remaining Java member resolution rules | Static imports, inherited Object members and ordinary bean properties; visibility, ambiguity, nullability and annotation metadata. Existing direct fields/methods excluded. |
| 17 | Rich annotation values and source declarations | Numeric/enum/class/array/nested values, defaults, retention/placement and Java reflection/Kotlin consumers. Existing three scalar annotation kinds excluded. |
| 18 | Member/companion extensions and extension properties | Distinct dispatch and extension receivers, overload selection, accessors, captures and nullability. Existing top-level/library extensions excluded. |
| 19 | Source inline/reified semantics | Supported reified tests/casts/class literals, noinline/crossinline, evaluation order and non-local-return boundaries. Existing compiled inline expansion excluded. |
| 20 | Suspend functions and suspend function types | Real Continuation suspension/resumption, spilled state and exceptional completion; no synchronous shim or complete kotlinx.coroutines implementation. |

Before accepting each row, distinguish valid supported, invalid Kotlin, valid but unsupported, unsupported harness requirements, dependency/setup failures and unrun dimensions. Execute assertions/box checks under JVM verification. Where applicable, compile fixed Java callers against official output and reuse their exact bytes against both providers; inspect signatures and Kotlin metadata with official Kotlin consumers. Add independent boundary, nullability, effect-order and exception assertions. Exercise an unchanged app target with real dependencies where practical; otherwise record the specific remaining blocker. A reduced reproduction never establishes its upstream original passes.

The reusable upstream adapter and matrix are infrastructure, excluded from the feature count. They retain original fixture identity/hash, FILE/MODULE boundaries, dependency/friend relationships, settings, backend restrictions, helper requirements and diagnostic source mappings. Unsupported directives are explicit harness outcomes. The baseline must use the unchanged executable before feature edits are accepted. Incremental fixtures require actual edit/rebuild semantics before an incremental result can pass.

## Ownership and integration

User override: lead and implementation work use Sol 6.1 with High reasoning. `upstream_adapter` owns only new `tools/upstream_jvm.py`, `tools/test_upstream_jvm.py`, `tools/upstream_feature_matrix.json`; `syntax_families` initially owns only `src/syntax.rs` and its dedicated new string fixtures/harness. `feature_assessment` is read-only. The lead owns this record, integration, review and publication. Shared compiler ownership transfers explicitly between sequential checkpoints.

Every implemented family receives a separate reviewed commit and push to `master`; unrelated JPEG changes stay unstaged. No force push or PR. Freeze sources/fixtures/harnesses before final validation; fixes require fresh reports. Preserve failed evidence. Run affected Rust tests, formatting, Clippy, Python/report tests, upstream suites and application regressions; regenerate HTML and verify local evidence links. Reassess dependency/generated-input accounting without claiming full-app compatibility or performance.

## Execution

- Initial inspection confirmed the prior thirty-task final acceptance and the 118-entry README overview. No existing feature is newly claimed.
- The latest historical resolved scan has 393 setup failures, four emitted files and three unsupported files; the limited scan has 367 dependency failures, 26 emitted files and seven unsupported files. These are historical first-diagnostic assessments, not demonstrated language coverage.
- The compiler baseline has been built without compiler edits. Upstream baseline selection and adapter implementation are in progress.
- No new family has yet met acceptance.
