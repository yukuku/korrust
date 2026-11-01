# Remaining Kotlin/JVM families

The user resumed work after the pushed stop checkpoint `a404f53775fe3ed937464a33e95d29baa9d69322`. The initial checkout is clean. Sol 6.1 High remains the explicitly requested lead and implementation model. Direct commits and pushes to master remain authorized. Upstream snapshots and unrelated workspace changes must be preserved.

## Counting and architecture

Finish final acceptance of the already implemented member/companion extension family first; this is not another new implementation. Then pursue the five remaining families from the original pool below. The target is six accepted remaining families including that carryover, not six newly invented families. Repairs and infrastructure are uncounted. Previous bounded capabilities remain bounded.

Retain the existing parser, typed checking, metadata and classfile pipeline. Extend representation contracts where required rather than introducing application-specific lowering. Value classes and unsigned values need a coherent distinction between logical Kotlin types and physical JVM representation. Captured classes need explicit outer/capture storage. Suspension needs genuine continuation state and exception behavior. These are architectural requirements, not promises of unrestricted coverage.

Implementation agents own distinct isolated directories under target, with sequential reviewed integration into shared compiler files. The lead owns this execution record, integration, acceptance and delivery. Baseline validation uses a frozen copy so later changes cannot alter its inputs.

## Scope and acceptance matrix

| Family | Proposed bounded capability | Required evidence and application priority |
| --- | --- | --- |
| Carryover: member/companion extensions | Existing distinct dispatch/extension receivers and computed properties; resolve interrupted final acceptance | Frozen full family suite, generic getter regression, Rust regressions; preserve source named companion/interface/generic-setter limits |
| Single-field JVM value classes | Source declarations, underlying primitive/reference representation, boxing at nullable/generic/interface boundaries, mangling, metadata and compiled consumption | Unchanged AndroidBible Gid and Rpp first; fixed Java callers, official Kotlin consumers, invalid declarations, identity/representation/effect assertions; exclude experimental multifield classes |
| Expanded enums | Entry-specific subclass bodies and overrides, abstract/shared members, interface dispatch, constructor defaults/named arguments and initialization | AndroidBible DevotionKind source usage; unchanged upstream originals, Java/Kotlin ABI, entry effects, exceptions and defaults; old ordinary enum support not recounted |
| Inner/local classes | Outer-instance access and bounded captured state with real JVM construction/capture contracts | Actual AndroidBible inner adapter classes guide priorities; nested existing static classes excluded; Java/Kotlin consumers and independent mutation/capture assertions |
| Unsigned numeric values | UInt/ULong and bounded smaller unsigned values through correct arithmetic/comparison/conversion/boxing contracts | Depend on value representation; actual UIntArray test usage assessed separately; original unsignedBinarySearch is already supported and is not new evidence; overflow/high-bit/null/generic boundaries |
| Suspend foundation | Bounded suspend functions/function types with real suspension, continuation resumption and exception propagation | Kotlin Continuation machinery and independent immediate/delayed/error assertions; actual AudioHttp/repository declarations guide scope; complete kotlinx.coroutines, Compose and serialization transformations excluded |

For every family distinguish valid supported, invalid Kotlin, valid unsupported and unsupported harness requirements. Run pinned Kotlin 2.3.21 with matched inputs/options. Preserve FILE/MODULE/friend/dependency/directive structure and source identities through the existing upstream adapter. Execute original box/assertion fixtures with JVM verification; reductions are independent cases, not original passes. Check signatures, metadata and official Kotlin consumers where relevant. Compile Java callers once against official output and execute unchanged against both providers. Include evaluation order, effects, exceptions, nullability and representation boundaries.

Record narrower supported boundaries explicitly if prerequisites block the proposed scope. Do not count a partial family as fully accepted. Exercise unchanged app sources with real inputs where practical, preserving dependency/generated-input blockers separately from language outcomes. Revalidate three-module integration and assess full-app inputs honestly; do not infer a full-app result from source emission or module results.

Freeze code, fixtures and harnesses before acceptance; preserve failed histories and publish fresh reports. Run affected Rust/Python tests, formatting, warnings-denied Clippy, differential upstream checks and app regressions. Update README only for accepted new capabilities, regenerate HTML and verify evidence links. Commit and push each implemented feature checkpoint without force pushing or creating a PR.

## Execution

- Baseline a404f5377 and clean tree verified. Reference policy, upstream test infrastructure, prior execution records and current capability matrix read.
- Baseline member-extension and Rust validation assigned a frozen copy.
- Independent isolated implementations assigned for enum entry polymorphism and value-class representation after initial source/upstream assessment.

### Initial scope boundaries and adapter foundation

- Value-class implementation starts with nongeneric single-val Int/Long/String carriers, nullable/generic/interface boxing boundaries and ordinary Rpp data-class consumption. Generic underlying carriers, nested value wrappers and secondary/default constructors remain explicit exclusions until separately demonstrated.
- Inner-class implementation starts with nongeneric named inner classes, real stored outer state, implicit outer access and this@Outer, instance-qualified construction and Kotlin inner metadata. Local classes and more complex constructor/inheritance forms are not assumed supported and do not become separate family counts.
- Enum entry polymorphism is confirmed in unchanged DevotionActivity.DevotionKind, including five bodies and abstract/open methods. ReadingGuideMode ordinary enum support is existing work.
- The upstream adapter now follows the pinned CoroutineHelpersSourceFilesProvider for WITH_COROUTINES and its state-machine/tail-call checker sources. Each unchanged helper is compiled as source with each provider; per-helper identity, mappings and hashes are retained. Unknown WITH_CONTINUATION remains a harness boundary. Fourteen adapter tests pass. The first unchanged coroutine original compiles with the official compiler and fails on the current candidate's unclassified suspend-type diagnostic in the correctly mapped helper. This is preserved as a baseline correctness failure in `reports/correctness/2026-11-02-coroutine-helper-baseline.json`, not a suspension pass.

### Resumed baseline and lexical repair

The frozen stopped compiler passes member-extension acceptance (62 positive checks,9 invalid rejections,11 unsupported) in the fresh resume report. Full Rust regression exposed real nested-object and enum-entry lookup regressions, plus stale data-object/member-extension rejection expectations and an invalid String.length() acceptance. These remain separately recorded; the whole baseline is not clean yet.

The lexical repair registers nearest lexical classifier/enum-entry groups before original import tiers, preserves original import priority outside those scopes, and makes qualified assignment use the same classifier resolution as reads. The original nested declaration and enum regressions pass, as do fixed Java execution against both providers, Kotlin consumers, import/shadowing/capture/state checks and all-target Clippy. Frozen source/evidence is in `reports/correctness/2026-11-02-lexical-value-repair.json`. This is repair work and adds no family count. Historical failures remain intact.

### Baseline property contract and test maintenance

The remaining real frontend failure was Kotlin `CharSequence.length` being exposed as a callable JVM method. The checker now uses the pinned special-property mapping, preserving unrelated same-name methods and declared extensions. The frozen differential report `reports/correctness/2026-11-02-builtin-property-contract.json` records two verified-provider executions and three oracle-invalid rejections. All 60 frontend tests and the focused smartcast-boundary test pass after removing stale negative expectations for already validated objects/member extensions and updating rejection text. Formatting and all-target warnings-denied Clippy pass. A first focused smartcast command omitted JAVA_HOME and failed setup; its log is retained alongside the successful rerun. These changes add no family count.

The upstream adapter also reproduces the pinned JVM optional-value-annotation preprocessor in disposable copies, retaining original identities, transformations, diagnostic mappings and provider hashes. Sixteen adapter unit tests pass. The experimental `+ValueClasses` language setting remains an explicit harness boundary; stable single-field support must not silently substitute for it.

### Carryover acceptance and expanded enums

The frozen carryover member-extension suite completed with 62 positive, nine invalid-Kotlin and 11 valid-unsupported cases (`reports/correctness/2026-11-02-resume-member-extensions.json`). The full resumed starting-compiler Rust baseline completed all 35 binaries: 440 pass and seven failures, with no additional failures beyond the repaired lexical/property/stale-boundary issues. The baseline report remains failed; focused repair results do not rewrite it or imply final combined-suite acceptance.

Expanded enums now use ordinary class checking and emission for entry subclasses, with hidden name/ordinal constructor parameters, correct default-mask indices, declared interface metadata and combined InnerClasses attributes. The integrated focused V3 suite passes five runtime/ABI/metadata cases, rejects five oracle-invalid declarations, and explicitly rejects one oracle-valid abstract-property case. Separate verified executions check nullable interface dispatch, throwing defaults/entry initialization, side-effect order and repeat access after initialization failure. All seven enum Rust tests, 28 metadata tests, import-priority runtime regression and warnings-denied Clippy pass. A cross-module upstream fixture exposed a general JVM field-type resolution bug: a successfully loaded reference type no longer requires a redundant explicit import. Fresh upstream V3 results supersede V1; V2 was interrupted for formatting and remains preserved.

The unchanged DevotionActivity probe uses captured real Android dependencies, but is not a whole-file pass. Korrust stops earlier at the `1f` literal. The pinned compiler exits with a Compose-plugin classloader ClassCastException under the captured plugin inputs. This is an oracle/plugin execution blocker, not evidence that the app source is invalid. Abstract enum properties, secondary enum constructors, nested entry declarations and enum-to-string interpolation remain outside this acceptance scope.

Final enum upstream V3: ten unchanged originals pass, four explicitly reject unsupported language and one excludes the JVM target. There are no correctness failures in this selected run; input hashes remained unchanged. This is a selected fixture result, not a compatibility percentage.
