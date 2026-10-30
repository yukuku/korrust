# Next five Kotlin/JVM families

The user authorized architecture review, necessary refactoring, repair of known failures, and five additional substantial families after the completed ten. The integration baseline is `39e87a38ae59075ce14cd110c88efbab46858131`; the initial master checkout was clean. Lead and implementation agents use Sol 6.1 with High reasoning, continuing the explicit model override. Direct master commits/pushes remain authorized; no PR is requested.

## Architecture decision

Retain the existing parser, typed checking IR, feature modules, metadata emitter and classfile emitter. File size alone does not justify a rewrite. Separate physical JVM descriptor decoding from logical Kotlin metadata types before further ABI work: interface bridge generation currently consults the logical accessor for JVM loads, slots and return opcodes. Reproduce and validate the correction. Audit capture traversal omissions separately before extending receiver/capture semantics.

## Repair gate

Inventory the historical failures without changing old reports. Re-run original inputs after fixes. Invalid subject-when Boolean syntax is a confirmed false acceptance. Legal numeric/punctuated upstream FILE names currently fail before language analysis. Larger upstream fixtures may require features below or still-unsupported constructs; a newly explicit unsupported diagnostic is not a pass. App assessments require their real captured dependencies. Repair work and infrastructure do not count toward the five families.

## Selected scope and acceptance

1. Rich annotation values and declarations (original family 17): source annotation classes; supported scalar, enum, class, array and nested values/defaults; retention and placement; exact classfile values, Java reflection, Kotlin metadata and independently compiled consumers. Reject unsupported shapes explicitly. Recover archived partial work by three-way integration and fresh tests, not by reusing its old acceptance status.
2. Class/interface delegation (family 10): interface forwarding, properties, explicit override precedence, one-time delegate evaluation, generic representation boundaries and ABI/metadata. Existing delegated properties are excluded. No current AndroidBible delegated supertype was found, so justify this family through reusable semantics and unchanged upstream originals.
3. Expanded object semantics (family 11): data objects and bounded object supertypes/companions with singleton initialization, equality/hash/string contracts and metadata. Existing ordinary singleton features are regressions, not new work. Target actual AudioBar data-object declarations with real inputs.
4. Member/companion extensions and extension properties (family 18): distinct dispatch/extension receivers, resolution priority, accessors, captures and nullability. Existing top-level/compiled extensions are excluded. Exercise actual ScrollbarSetter and MarkerListActivity source patterns; retain any whole-file blocker.
5. Source inline/reified semantics (family 19): real call-site substitution with bounded reified type tests/casts/class literals, noinline/crossinline and non-local-return boundaries, evaluation order and exceptions. Existing compiled inline expansion or an ordinary call to a source inline method does not qualify. Full original primitiveClassEquality additionally requires the remaining Short representation; report that dependency independently.

Each family needs positive, invalid and valid-unsupported cases; fixed Java callers compiled against official output executed with JVM verification against both providers; relevant signatures/metadata and official Kotlin consumers; unchanged upstream original checks plus independent effects/exception/nullability assertions. Freeze inputs before final evidence and preserve failed attempts. Revalidate existing support and historical AndroidBible modules; full-app results remain separate. Update README only after validation, regenerate HTML and verify links. Commit and push each accepted family, stop after five.

## Execution

- Architecture, failure and selection audits started against the clean baseline.
- Physical descriptor refactor and filename-policy repair have nonoverlapping live ownership. The lead owns the subject-when repair and integration.
- Annotation implementation uses an isolated baseline under target; archived work is unaccepted until fresh integration and validation.

### Prerequisite checkpoints

Commit `986398d57` separates physical JVM descriptors and repairs legal ASCII facade naming, alias constructors and selected expression resolution. Four unchanged upstream originals now execute successfully: typeAliasConstructor, simpleFromOtherFile, enum/inner and genericTypeAliasConstructor2. The initial publication path error preserved execution results and was corrected without rerunning or overwriting history.

The subsequent library-contract checkpoint resolves checked Kotlin/JVM renamed members through actual methods and distinguishes passive Java declaration annotations from semantic nullability/default/type-use annotations. Fixed Java differential contracts, descriptor comparisons, focused Rust regressions and warnings-denied Clippy pass. BackForwardList progresses past removeAt to compound assignment; SDK Intent.addFlags compilation is distinct from Android runtime. No new family is accepted by either repair checkpoint.

Compound-assignment repair now resolves mutating assignment operators and binary replacement independently, including primitive/String extension ambiguity. Complete unchanged BackForwardList passes fixed official Java runtime, ABI and official Kotlin consumer checks. Two differential tests and fifteen frontend regressions pass; this repair does not count as a new selected family.

### Accepted family 1 of 5: class-header interface delegation

Fresh integrated frozen report `2026-10-30T062315415007Z-new-class-delegation.json` passes 48 selected groups, rejects 12 invalid inputs and 23 valid unsupported shapes explicitly. An additional AndroidBible inventory finds zero class-header delegation uses; it is excluded from acceptance counts. Fixed Java runtime callers, official Kotlin consumers, ABI/metadata, unchanged multimodule upstream originals, side effects and numeric/null representation checks pass. Integrated focused Rust regressions, formatting, Clippy and report tests pass. Final combined milestone regression remains required.

### Accepted family 2 of 5: rich annotations

Integrated frozen suite passes 46 positive groups, 46 invalid rejections and 18 explicit unsupported cases. Fixed Java reflection/read callers, metadata, signatures and official Kotlin consumers pass; unchanged NIA NiaDispatchers uses real javax.inject. Unchanged upstream propertyWithPropertyInInitializerAsParameter passes. Larger kt10136 remains a correctness failure at ordinary assertion/lambda resolution and defaultParameterValues is explicitly unsupported; see the separate original-fixture report. Five failed development reports and frozen source archives remain available. Integrated library regression has 268 initial passes plus one stale compound-error-message expectation corrected and rerun successfully; delegation diagnostics add four passing tests. This test-only correction does not alter the frozen release executable or fixture results.
