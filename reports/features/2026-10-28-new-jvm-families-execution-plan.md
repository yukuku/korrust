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

### Checkpoint 1: strings and newline parsing

Family 1 has bounded acceptance in [fresh correctness evidence](../correctness/2026-10-28T101248695817Z-new-strings.json). Both providers execute 39 independent Java assertions under JVM verification using identical official-compiled caller bytes. Public descriptors match; official Kotlin consumers compile and execute against both providers. Two unchanged pinned originals (`rawStrings.kt`, `multilineStringsWithTemplates.kt`) execute their original box checks. Seven invalid programs reject with both compilers. The complete larger `rawStringsWithManyQuotes.kt` remains unsupported at the existing reference-vararg projection boundary; it is not counted as passing.

Supported scope includes raw backslashes/quote runs, LF/CRLF/CR normalization, UTF-16 and surrogate behavior, nested raw/ordinary interpolation, Char/nullable Char interpolation, side-effect/exception order, parentheses/indexing newline mode and qualified continuation. Multi-dollar strings, Unicode unbraced template names and Nothing-valued interpolation remain unsupported. AudioHttpErrorBody's unchanged source advances past its former navigation guard but remains an unclassified compilation failure at its existing ByteArray/Charset String-constructor boundary. No complete new app source, module or full-app coverage follows from this checkpoint.

Frozen source inputs, command logs and invalid fixture materializations are archived with the report; two failed earlier development reports remain preserved. The frozen working source includes concurrent overload work outside the string claims. No additional family is accepted by that source presence.

The final [string checkpoint](../correctness/2026-10-28T102639408771Z-new-strings.json) supersedes that first snapshot after independent review found newline-separated trailing-lambda parsing gaps. It passes 43 independent assertions per provider, signatures, official Kotlin consumers and the two unchanged original box tests. Eight invalid programs and five valid unsupported boundaries remain separate; AudioHttpErrorBody retains its actual unclassified constructor failure. Annotated call suffixes remain explicitly unsupported; declaration annotations after completed calls retain their original behavior. The complete Rust library suite passes 236 tests on this source, with no ignored or weakened assertions.

This final checkpoint uses an immutable complete source/test/tool copy and archives its exact inputs. Its independently built binary is SHA-256 `b1817b3f81a0a6310c0efe3e438611649f18e36566ef9da16d029ee58cfefd7d`, identical to the independently frozen overload checkpoint. Generic-alias and constant-expression development after that freeze cannot change this evidence.

### Checkpoint 2: JVM function overloads

Family 4 has bounded function-lowering acceptance in [fresh v3 correctness evidence](../correctness/2026-10-28T104259884676Z-new-jvm-overloads.json): 21 selected positive case groups, 13 invalid-Kotlin rejections and three explicit supported-language boundaries. The independent official-compiled Java callers check effects, exceptions, null/default ordering, virtual dispatch, nontrailing defaults, generic signatures, varargs, nullable/wide values, the signed high default-mask bit, parameter/method annotations, and object/static/companion behavior. All 13 selected public/protected owner signatures match. Official Kotlin consumers and the unchanged upstream simpleJavaCall fixture execute against both providers. Frozen Rust validation passes 238 library and three dedicated integration tests; no ignored tests are consumed as evidence.

Scope is ordinary supported functions with the existing 32-parameter source-default limit. Constructor annotations, extension defaults, source-interface overloads and qualified/aliased object JvmStatic forms remain explicit boundaries. Illegal marker placement, inherited/generated overload collisions and overriding declarations introducing defaults reject before returned output artifacts. The constructor-annotation diagnostic preserves adjacent separately annotated declarations.

FormattedVerseText advances beyond its old annotation blocker but still fails at StringBuilder indexing. The unchanged file's unclassified failure and the independently reduced, officially valid program's incorrect TYPE_MISMATCH rejection remain actual failing assessment rows. They are outside selected overload acceptance and are not relabeled as behavior passes or dependency failures. No additional complete app source or module is claimed. Earlier failed/partial reports remain preserved; the final report archives its exact frozen inputs.

### Baseline environment correction and module regression

The unchanged pre-feature source passes [349 Rust tests, 166 Python tests and both providers' original three-module Gradle builds](../correctness/2026-10-28T102327274964Z-new-families-baseline-checks.json). Initial setup failures and logs remain archived. The local Rust 1.99 Clippy additionally found one preexisting unnecessary path clone in an archive-policy test; that assertion is preserved with a borrowed slice.

The inherited JAVA_HOME was a Homebrew launcher prefix without jmods, while the JVM itself used a different actual home. A [fresh upstream baseline](../correctness/2026-10-28T104500Z-upstream-baseline-real-jdk.json) uses the actual JDK 21 home with the same unchanged compiler executable. It records four selected passes, 23 correctness failures (including unclassified valid-source rejections), three explicit language boundaries and six harness boundaries. The former dependency failure becomes an actual compiler rejection with complete JDK input. Previous baseline reports are historical, not current setup truth. These figures describe a selected original-fixture sample, not a language compatibility percentage.

### Checkpoint 3: generic source aliases

Family 13 has bounded type-position acceptance in [fresh v4 alias evidence](../correctness/2026-10-28T105615Z-source-generic-aliases.json): 41 independent Java assertions against official, candidate and reversed source-order outputs, matching protected JVM signatures, and official Kotlin consumers of exported aliases. Public/private scope, arity, cycles, parameter scope and implicit target bounds have 20 distinct invalid-Kotlin outcomes; a projected nested expansion has an explicit unsupported-language outcome. Nine alias Rust unit tests, four integration tests and scoped Clippy pass. The exact source/fixture/harness ZIP and earlier failures remain archived.

Source generic parameters now substitute through nullable, function/receiver-function, arrays, nested lists and compiled generic classifier aliases, with exported parameter metadata. The signature repair recursively emits covariance for known interface/function/type-parameter positions while preserving invariant mutable lists and final-classifier/array behavior. Arbitrary classifier finality and unrestricted generic variance are not claimed.

The original typeAliasFunction box and unchanged GotoTab/OnGotoFinished regression pass. The larger original simple fixture and genericTypeAliasConstructor2 remain failing; reductions do not replace them. Imported generic property getters and alias constructor calls retain actual compiler failures, rather than being relabeled as supported alias behavior. No new AndroidBible application source is claimed from its existing nongeneric alias. The [live capability matrix](2026-10-28-new-jvm-families-capability-matrix.json) records three accepted bounded families and seventeen unassessed families at this checkpoint.

### Checkpoint 4: Java member interoperability

Family 16 passes [fresh standalone commit-source validation](../correctness/2026-10-28T121821034547Z-new-java-interop.json): eight selected positive groups, nine invalid cases and three explicit unsupported boundaries. This repeats the [isolated combined-source checkpoint](../correctness/2026-10-28T115426915021Z-new-java-interop.json) after separating constant-evaluator work, rather than assuming the reduced source tree behaves the same. Fixed official Java callers run against both outputs with verification; signatures and official Kotlin consumers pass. The unchanged inheritedPackageStaticFunction upstream original executes successfully. The earlier frozen source also passes 247 Rust library tests, the static-import regression, all-target Clippy with warnings denied, and seven viewer tests.

Supported scope includes explicit/aliased/inherited static imports and varargs, inherited fields, accessible symbolic owners for package-private declaring ancestors, and synthetic getter/setter properties on ordinary concrete Java classes, including generic receivers. Field precedence, nullability, virtual dispatch and receiver/RHS evaluation order are independently checked. Kotlin Any availability, names and nullability come from the pinned builtin declarations; actual JVM implementations execute normally. Platform String concatenation uses the existing bytecode operation.

Static-member wildcard imports, static interface method imports and synthetic properties on mapped/interface/enum/Kotlin receiver shapes remain explicit boundaries. The unchanged SnappyStreamRoundTripTest passes the official compilation/runtime setup but still fails candidate import resolution for java.util.Random; this is retained as an unclassified external assessment, not a language or dependency success. No full app or additional module result is implied.
