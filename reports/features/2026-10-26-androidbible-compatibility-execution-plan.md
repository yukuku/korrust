# AndroidBible compatibility: execution plan for Sol

Status: proposed implementation plan; no implementation tasks completed by writing this document.

This document supersedes both earlier conversational 30-item lists. It preserves the numbering of the revised Astra review. It is the self-contained handoff for implementing all 30 tasks when the user requests execution. Writing this plan does not itself authorize beginning implementation.

## Objective and scope

Restore previously demonstrated compatibility, support compilation of multiple Kotlin source files in a module, and extend reusable Kotlin/JVM semantics sufficiently to validate the named AndroidBible targets. Preserve the original application sources. Assess progress through reproducible compilation, behavior, ABI, Kotlin consumers, and selected module builds.

The 30 items form six delivery checkpoints. If the user requests the entire plan, continue through all checkpoints without seeking renewed permission at each one. Checkpoints control integration and verification; they are not invitations to stop after task 10. Resolve ordinary implementation choices autonomously within the boundaries here. If a target exposes additional blockers, reduce them, implement bounded prerequisites needed for that target, and record the scope change. Do not silently omit acceptance criteria or turn a failed positive case into an expected rejection.

Outside this plan: value classes (`Gid.kt`/`Rpp.kt`), coroutines/suspend lowering, Compose transformations, serialization transformations, KSP/Room generation, general incremental dependency tracking, runtime optimizations, performance promises, production deployment, and a complete Alkitab app build. Task 30 may recommend these as future work; it does not authorize starting them. Existing unsupported constructs must continue to reject explicitly.

## Model and session handoff

The user explicitly intends to switch to Sol to execute this plan. The suggested execution prompt at the end explicitly overrides the repository's standing Astra-as-main preference for this task. With that instruction, Sol is the lead and may make the bounded design decisions here; it must not wait for an Astra agent to become available. Otherwise follow `AGENTS.md`. Use Sol 6.1 with High reasoning for implementation subagents, as the repository requests. Do not silently substitute a different model.

Recommended execution setup: a new chat with Sol 6.1, High reasoning, attached to the same `/workspace/korrust` checkout. A new chat provides a focused implementation context; this file carries the required history. Continuing the existing chat after switching is also valid. A new environment, clone, or worktree may not include uncommitted files: ensure this plan and the current assessment files are present before relying on them. Do not assume they exist on the remote.

The lead owns architecture, integration, shared API changes, evidence review, and completion decisions. Delegate only tasks with agreed interfaces and nonoverlapping file ownership. Most compiler work touches `syntax.rs`, `check.rs`, `classpath.rs`, `lib.rs`, or metadata; do not assign independent agents simultaneous ownership of these files. Tests/harness work can run alongside compiler work after the interface and semantic contracts are agreed. Freeze edits before final checks.

## Starting state and mandatory reading

Repository: `/workspace/korrust`, baseline commit `9fdbdf6736f187c3ffbffc985a0c85b13f49de14`. Inspect the actual checkout before editing; later changes take precedence over this snapshot description.

Read:

- [Agent instructions](../../AGENTS.md), [project contract](../../README.md), [reference policy](../../references/README.md), and [report conventions](../README.md).
- [Current source assessment](../correctness/2026-10-26T073039Z-androidbible-source-assessment.md) and its [machine-readable results](../correctness/2026-10-26T073039Z-androidbible-source-assessment.json).
- [Latest frozen delivery audit](../correctness/2026-10-25T153444729166Z-declarations-delivery.json), [declaration suite](../correctness/2026-10-25T151327943975Z-declarations-twenty.json), and [earlier PlaybackState success](../correctness/2026-10-24T094102532411Z-androidbible-twenty-features.json).

At plan creation, `README.md` has an uncommitted general CLI wording clarification. The current assessment JSON, Markdown, and reproduction script are also untracked. Preserve these changes. The user specifically requested a general README: do not add dated findings, current coverage counts, regression narratives, or this roadmap to it. Detailed progress belongs in reports and this plan's execution record.

The current assessment emitted classes for 20 of 400 pinned `.kt` files. It used a limited dependency set and did not run the official compiler or runtime checks. Fifty first failures were missing dependencies; first failures are not exhaustive feature inventories. Earlier correctness suites supply stronger evidence for selected files. `PlaybackState.kt` previously passed but currently rejects with an inference diagnostic; the exact cause is not established.

The current 20 emission cases are `RibkaEligibility`, `AudioErrorCode`, `AudioLogEntry`, `VersesDialogNormalVerses`, `XrefDialogVerses`, `InternalSingleChapterVerses`, `VerseTextDecoder`, `BookColorUtil`, `IntArrayListUtil`, `Levenshtein`, `RequestCodes`, `ReaderGestureActions`, `VerseNumberPlacement`, `SongPlaybackState`, `SongTextZoomable`, `Yes1SingleChapterVerses`, `Utf8Decoder`, `SingleChapterVerses`, `Yes2SingleChapterVerses`, and `unsignedBinarySearch`. Use the assessment JSON for exact paths and hashes, not filename-only lookup in a potentially changed tree.

## Existing architecture to reuse

| Area | Starting points | Constraint |
| --- | --- | --- |
| Source pipeline | `src/lib.rs`, `src/syntax.rs`, `src/classpath/source.rs` | The compiler already splits declarations within one file and iterates signature inference. Generalize this machinery; do not concatenate source text or build a second compiler pipeline. |
| CLI and output ownership | `src/main.rs`, `src/output.rs`, `tests/test_cli.py` | Existing single-source CLI and ownership checks must remain compatible. |
| Library resolution | `src/classpath.rs`, `src/classpath/{archive,metadata,signature,aliases}.rs`, `src/check_{library,overloads,relations,functions}.rs` | Preserve Kotlin metadata, platform/nullability behavior, real generic signatures, and explicit unsupported boundaries. |
| Lowering and JVM output | `src/bytecode.rs`, `src/bytecode/{defaults,lambdas,interfaces,objects,data,frames}.rs`, `src/metadata.rs` | Runtime, verifier, ABI and Kotlin-consumer evidence are required together. |
| Module integration | `tools/build_androidbible.py`, `tools/check_androidbible_module.py`, `tools/check_androidbible_models.py`, `tools/check_androidbible_io.py` | Existing Gradle model capture and module workflows already exist. Extend them. |
| Differential harnesses | `tools/check.py`, `tools/check_library_interop.py`, `tools/check_declarations_twenty.py`, `tools/check_next_twenty.py`, `tools/check_androidbible_{simple,ten,twenty}.py` | Reuse logging, fingerprints, ABI inspection, official Java callers and consumer patterns. |
| Evidence | `tools/reports.py`, `tests/test_reports.py`, dated `reports/` | Old reports are immutable evidence; write fresh reports for changed results. |

Observed module-driver limitation: `compile_request` invokes the compiler once per source with dependency directories, not a shared source compilation context. Simply adding the output directory to the classpath would still leave order and cyclic-reference problems. Batch semantics must resolve declarations before bodies.

Observed classpath inconsistency: the compiler's JAR reader rejects genuine multi-release class overrides and treats versioned module descriptors specially; the build helper extracts `.class` entries, while the recent assessment used a separate extraction policy. Fix the shared interpretation, not only the assessment workaround. Preserve `.kotlin_module` package metadata when extraction is necessary.

## Environment and initial commands

Read the managed cloud environment skill and inspect environment status if running in the managed environment. Preserve proxy and credential configuration. Do not assume SDKs, cached dependencies, reference binaries, or generated outputs survived a new session.

On the current machine, `/workspace/korrust-setup/env.sh` sets task-specific Cargo/Rustup locations, Rust 1.90.0, JDK 21, and PATH. Inspect before sourcing if it changed. It does not configure a complete Android SDK. Shell environment settings must be supplied to each independent tool command as needed.

```sh
cd /workspace/korrust
git status --short
source /workspace/korrust-setup/env.sh
cargo fetch --locked
python3 tools/setup_reference.py
cargo build --release --locked --offline
```

Use the compiler pin in `references/compiler.json` (2.3.21 at planning time), and verify Gradle's resolved compiler/options separately. Do not replace the pin with the machine's default Kotlin compiler. Existing harness defaults contain machine-specific SDK, D8, and annotation paths; inspect `--help` and pass actual resolved paths instead of treating a nonexistent default as a compiler failure.

Keep snapshots under `benchmarks/sources/` and `references/` unchanged. Run Gradle in disposable app copies prepared by existing tooling. Never use `--no-daemon`; capture logs and retain failure exit status when piping output. Do not commit or push merely because the execution takes place in the cloud: the AndroidBible snapshot's delivery rules do not turn Korrust work into an AndroidBible PR.

## Common acceptance contract

For every compiler feature below:

1. Establish the expected behavior from the pinned specification and official compiler/tests; record gaps or discrepancies.
2. Add an independent minimal fixture before or alongside implementation. Include positive, invalid-Kotlin, and valid-but-unsupported cases with distinct outcomes.
3. Compile matching source/dependency inputs with both compilers. Use matching relevant options; explicitly record any target/option differences the candidate cannot yet support.
4. Run fixed Java caller bytecode compiled against official output against both outputs using `-Xverify:all`. Add independent expected values, failure cases and effect-order traces.
5. Check relevant public/protected signatures and official Kotlin consumers, including negative visibility/nullability cases where applicable. Successful emission or reflection alone is insufficient.
6. Reassess the unchanged app target with its real dependency closure. Record any official-compiled dependency and test-only stub; never count a stubbed Android run as Android compatibility.
7. Preserve previous supported behavior. Add the new case to the persistent regression suite. Do not rewrite historical reports or weaken unrelated assertions.

Use narrow tests while developing; after integrating a checkpoint, run the affected broader suites. Final evidence must refer to a frozen compiler, fixtures, tools, sources and dependency hashes. A fix after a frozen run requires new evidence for the affected checks. Checkpoint completion requires acceptance evidence, not just a task marked done.

## Checkpoint A: dependable baseline (tasks 1–5)

### 1. Reproduce PlaybackState under matched conditions

Dependencies: none.

Work: retrieve exact commands, dependency hashes, compiler fingerprints and options from the earlier success. Reproduce current rejection with both current and official `AudioLogEntry` output. Compare raw JAR versus supported directory classpaths and inspect any harness differences. If the earlier compiler must be built for diagnosis, use an isolated checkout or source copy; preserve the active workspace. Do not assume the `IDLE` initializer is the root cause merely from the generic error.

Acceptance: a dated reproduction distinguishes compiler regression, classpath/harness regression, and any unresolved environmental difference; official Kotlin accepts the unchanged source under the relevant setup. Record a reduced candidate failure as the starting point for task 2.

### 2. Reduce and fix the established defect

Dependencies: 1.

Work: reduce the failing construct while retaining its trigger (for example, inferred companion properties, constructors, named/default arguments or generic list inference, as evidence establishes). Fix the general rule. If the failure is in the harness, fix that harness and avoid an unnecessary compiler change. Never add an explicit type to the original app source as the solution.

Acceptance: reduced fixture plus unchanged `PlaybackState.kt` pass; verify `IDLE` field values, constructor/getter behavior, `isActive`, and relevant data-class/consumer behavior. Include a negative inference case so the repair does not accept invalid cyclic declarations.

### 3. Establish a required app regression suite

Dependencies: 1–2.

Work: create or consolidate an executable target manifest for all 20 current emission files plus restored `PlaybackState`. Link each to exact source, dependency setup, existing runtime/ABI/consumer checks and unsupported claims. Reuse the existing simple/ten/twenty/declaration/baseline harnesses rather than replacing their assertions with compilation-only tests. One documented entry point should run the required selection and produce per-file outcomes.

Acceptance: every listed source compiles; required existing behavioral checks pass; missing checks/dependencies appear explicitly as not run, not as passes. A deliberately failing fixture must make the aggregate run fail. Freeze manifests alongside final compiler inputs.

### 4. Extend existing Gradle model capture

Dependencies: 3; environment setup may proceed alongside 1–3 with independent ownership.

Work: inspect `build_androidbible.py`'s current model capture, module choices and assumptions about `src/main`. Capture resolved production/test source sets, generated source roots, compile/runtime classpaths, compiler/plugin versions and relevant options for selected variants. Reuse module setup and disposable copies. Preserve a clear boundary between externally built dependency modules and the candidate-owned source set.

Acceptance: machine-readable captured inputs can reproduce an official build for selected targets; generated and test sources are identified; no required classpath entries are silently dropped. Missing generated outputs or SDK artifacts are classified as setup failures.

### 5. Make corpus reporting diagnostic and comparable

Dependencies: 3–4.

Work: promote the one-off source assessment into a reusable harness rather than editing its historical reproduction script. Keep the old limited-classpath result reproducible. Add a separate resolved-classpath assessment, grouping production/tests/tools and distinguishing emission, verified behavior, unsupported language, invalid Kotlin, dependency/setup failures, compiler failures and timeouts. Store first-diagnostic locations and stable source hashes.

Acceptance: before/after comparisons name the compiler and classpath changes; derived counts reconcile to per-case outcomes; changes in first blockers are visible. Unit-test classification/counting and verify report links. Do not claim every rejected file is intrinsically uncompilable.

Checkpoint A exit: restored target, passing regression selection, reproducible input model, and trustworthy baseline reports. Record the checkpoint before beginning the shared-source refactor.

## Checkpoint B: module compilation foundation (tasks 6–10)

### 6. Unify archive and directory dependency handling

Dependencies: 4–5.

Work: use one documented classpath policy in compiler-facing tooling. Prefer direct JAR inputs when supported. If extraction is retained, preserve package metadata and ignore non-class module descriptors appropriately; do not erase meaningful multi-release overrides to make a file appear supported. Maintain deterministic duplicate-class precedence and diagnose malformed entries.

Acceptance: equivalent ordinary JAR/directory inputs resolve classes and top-level Kotlin declarations identically, with stored/deflated JAR, module descriptor and missing-entry cases. Genuine unsupported multi-release overrides reject with the same explicit boundary across paths.

### 7. Add a bounded multi-file compilation API and CLI

Dependencies: 3, 6.

Work: define a source input object retaining stable identity, filename, contents and module context, and return artifacts with source ownership. Keep the existing one-file API as a wrapper. Proposed CLI: multiple positional `.kt` sources before `-d`, with the existing `-cp` behavior; retain existing one-file commands. A request/response file may be added for Gradle argument length, but should call the same batch API. Establish explicit module identity for visibility/metadata as needed.

Acceptance: a batch of independent supported files produces deterministic classes/metadata, routes errors to the correct original file, and rejects duplicate JVM owners before publication. Reversing input order does not change observable behavior or exported signatures. Never concatenate source strings.

### 8. Share declaration discovery across files

Dependencies: 7.

Work: generalize the per-file declaration-unit machinery in `lib.rs` and source headers in `classpath/source.rs`. Discover supported top-level/nested declarations before checking bodies. Carry package, lexical/file scope and provenance separately. Resolve same-package names and supported explicit imports through this index; define source/dependency collision behavior from reference evidence. Check cross-file overload sets and package facades.

Acceptance: fixtures cover forward references, mutually referring types with explicit signatures, cross-file functions/properties, same names in different packages and duplicate declarations. Unsupported import forms reject explicitly. Do not simulate source dependencies by precompiling candidate-owned files with official Kotlin.

### 9. Extend signature inference to the batch

Dependencies: 8.

Work: reuse the existing header/inference iteration, extending dependencies across source units. Track progress until stable; resolve explicitly typed cycles without requiring an arbitrary file ordering. Distinguish unresolved external names, valid-but-unsupported inference, invalid cycles and conflicting declarations. Preserve the original expression diagnostic rather than replacing all failures with an offset-zero generic message.

Acceptance: results and diagnostics are stable under input permutations; explicit-signature mutual recursion works within the existing supported call semantics; genuine cyclic inferred types reject. Add a multi-file regression for the class of failure fixed in task 2.

### 10. Integrate batch compilation with Gradle and output ownership

Dependencies: 7–9.

Work: replace the per-file loop in `compile_request` with one candidate-owned batch. Reuse staged output and ownership protections. Define batch manifest ownership for sources and package metadata, removing obsolete owned outputs after successful complete-module compilation. Preserve prior output on compile/preflight failures and refuse to overwrite unowned/externally modified files. Do not claim full filesystem transactionality unless implemented and tested. Account for Kotlin metadata's module identity, internal visibility and file-private visibility.

Acceptance: clean build succeeds without candidate-owned official classfiles; reorder files, remove/rename a declaration or source, and compare with independent clean output. Kotlin consumers resolve both files' package members. Illegal cross-file private access and external internal access reject. An invalid batch leaves prior outputs usable. Java-to-Kotlin/Kotlin-to-Java cycles outside current support are diagnosed and recorded, not resolved via stale classes.

Checkpoint B exit: isolated batch fixtures and at least a compatible existing module path exercise the new driver. Full new module acceptance remains task 29. No feature claims are based on adding already-built app classes to the source set's classpath.

## Checkpoint C: reusable library-call semantics (tasks 11–15)

### 11. Resolve the ReadingPassage inference/bounds blocker

Dependencies: 6, 9.

Work: reuse the original dependency closure and fixtures in `check_declarations_twenty.py`. The earlier report records `Type argument violates upper bound of R` at a `let` call. Reproduce the current state, then reduce and inspect generic substitution, expected types, primitive boxing and nullability. Fix only the established rules without API-name special cases. Review `check_overloads`, `check_relations`, library metadata and inference together.

Acceptance: official comparison includes generic identity/transform functions with nullable/non-null and primitive/reference results; invalid upper-bound cases reject. The original ReadingPassage advances past this blocker; remaining blockers are explicitly carried into task 15, not mislabeled as completion of the app target.

### 12. Validate and repair compiled Kotlin default arguments

Dependencies: 6, 9, 11 where generic constraints interact.

Work: audit existing default-call implementation before adding code. Cover imported constructors, instance/top-level methods, named arguments, omitted defaults and correct synthetic JVM entry points/masks. Verify receiver/argument evaluation order. Do not allow named Kotlin arguments on ordinary Java methods. Reject unsupported combinations rather than manufacturing default values.

Acceptance: fixed official-compiled libraries exercise first/last/multiple omitted arguments, a mask-boundary case, effectful supplied/default arguments, nullability and Kotlin consumers. Existing source-default tests remain green.

### 13. Implement ordinary vararg calls

Dependencies: 11–12.

Work: support calls to Java and Kotlin vararg declarations through normal overload resolution and invocation. Handle primitive versus reference component types, empty varargs, generic element constraints, and competition with fixed-arity overloads. Preserve source argument evaluation order. Source-declared varargs are included where needed for paired fixtures; record any deliberately narrower declaration boundary.

Acceptance: compare empty/single/multiple-element calls, null elements where legal, primitive boxing and overload ambiguity/rejection. Array factories work through this machinery, not bespoke `intArrayOf` recognition. Record limitations around arbitrary reified library bodies separately.

### 14. Implement spread arguments

Dependencies: 13.

Work: combine ordinary and spread elements into correctly typed arrays, preserving Kotlin copy/aliasing semantics, left-to-right effects and exception order. Check primitive and reference arrays, empty spreads and incompatible component types. Do not pass an aliased input array where the reference semantics require a copy.

Acceptance: observable effect traces and mutation checks distinguish correct copying from accidental reuse. Mismatched primitive/reference and nullable component cases reject appropriately; official Java and Kotlin consumers agree.

### 15. Complete the three selected library integration targets

Dependencies: 11–14, with 7–10 available for source dependencies.

Targets: unchanged `PrereleaseVersion.kt`, `ReadingPassage.kt`, `LidToAri.kt` at the paths in the corpus/source snapshot.

Work: build full relevant dependency closures and inspect remaining failures. For `PrereleaseVersion`, cover Regex constructor/default calls, match nullability and group access. For `ReadingPassage`, cover its actual range resolution, local functions, callbacks, array/list operations and parsing dependency. For `LidToAri`, inspect the whole file and cover its table, binary-search and boundary behavior. These are integration targets, not assumptions that the preceding four tasks alone suffice. Add bounded reusable prerequisites exposed by the cases; record them under this task. A broad out-of-scope prerequisite is a documented blocker requiring an explicit scope decision, not permission to alter the app.

Acceptance: all three emit and execute against official Kotlin with independent representative and boundary assertions. Official-compiled helper dependencies are disclosed. Test absent/malformed prerelease suffixes, invalid/empty reading addresses and lower/upper table boundaries as appropriate to the actual source contracts.

Checkpoint C exit: generic/default/vararg/spread behavior validated and the three targets either fully verified or explicitly blocked with concrete evidence. Continue independent later tasks if an environmental blocker remains; do not report the checkpoint complete prematurely.

## Checkpoint D: declaration syntax and functional interfaces (tasks 16–20)

### 16. Support backtick identifiers

Dependencies: 7–9 for cross-file cases; lexical work may begin earlier with agreed ownership.

Work: lex escaped identifiers as identifiers without losing original locations. Support declarations and references, including names containing spaces and keywords used legally. Apply JVM name restrictions at the appropriate boundary. Check interaction with generated names and output ownership validation: accepting a source name does not justify permitting unsafe output paths. Avoid ad hoc replacement of characters in public names.

Acceptance: original descriptive test-method syntax compiles in a minimal real test class with the proper annotation dependency; invocation/reflection sees the expected method name. Invalid JVM names reject explicitly; ordinary identifiers and source locations remain correct.

### 17. Support discarded underscore bindings

Dependencies: 16 for relevant parsing; existing destructuring/lambda machinery.

Work: distinguish a discard from an identifier or a missing declaration. Support underscore lambda parameters and destructuring components in Kotlin-permitted positions; reject use as an expression. Evaluate destructuring initializers and receivers correctly. Establish from the reference which `componentN` calls are skipped for discarded entries; do not assume all components must execute.

Acceptance: effect-trace fixtures prove retained/skipped component calls and initializer order; repeated discards do not collide in scope; illegal ordinary underscore declarations/usages reject.

### 18. Implement fun interface declarations

Dependencies: 8–9; existing source interface machinery.

Work: represent Kotlin functional-interface identity explicitly. Validate exactly one eligible abstract method across supported inheritance, accounting for inherited/default methods and reference rules. Emit interface bodies/default implementations and metadata using the established JVM-default mode. Do not simply erase `fun` and emit an ordinary interface.

Acceptance: valid single-abstract-method interfaces, inherited methods and default helpers compile; zero/multiple abstract method and other invalid forms reject as official Kotlin does. Official Kotlin consumers recognize the result as a functional interface.

### 19. Convert lambdas to Kotlin functional interfaces

Dependencies: 18; existing Java SAM/lambda support.

Work: reuse conversion, capture and bridge machinery with Kotlin functional-interface metadata. Check expected parameter/result types, primitive boxing, interface dispatch and captured immutable/mutable state. Target unchanged `LogInterface.kt`, including the function returned by `logger` and its invocation of `log`.

Acceptance: both official Kotlin and Java consumers use candidate interfaces/implementations, JVM verification passes, and effect assertions prove prefix formatting and capture behavior. Distinguish ordinary function types from SAM types during overload resolution.

### 20. Implement source type aliases

Dependencies: 8–9, 18–19 for combined integration cases.

Work: support concrete aliases and function-type aliases, alias chains and uses in signatures. Preserve alias identity in Kotlin metadata where required while lowering JVM descriptors to the expanded type. Respect package/file visibility and imports; diagnose cycles. Generic aliases and alias forms outside the initial bounded scope must reject explicitly, not expand incorrectly.

Acceptance: same-file/cross-file and compiled-library official Kotlin consumers resolve aliases; cycle/visibility tests reject; the `OnGotoFinished` function alias from `GotoTab.kt` is covered independently before enum integration.

Checkpoint D exit: real LogInterface runtime/consumer evidence, escaped-name and discard tests, and type-alias metadata evidence. Do not count broader Android tests as supported merely because their names now parse.

## Checkpoint E: enums and Double (tasks 21–25)

### 21. Implement enum constructors and entry arguments

Dependencies: 9, 12–14 as required by bounded constructor forms.

Work: extend the existing bare enum implementation with constructor parameters, per-entry argument lowering and Kotlin/JVM enum constructor conventions. Keep implicit name/ordinal parameters distinct from user parameters. Check evaluation order and one-time initialization. Start with the scalar constructor/property shapes used by `GotoTab` and `ReadingGuideMode`; reject unsupported constructor forms explicitly.

Acceptance: Java and Kotlin callers see correct ordinal/name/user values, `values()` returns a defensive copy, and `entries` has correct identity/order. Side-effect fixtures prove entry and initialization ordering; empty/invalid entry cases are covered.

### 22. Support enum properties, methods and companions

Dependencies: 21 and existing ordinary method/property/companion semantics.

Work: reuse ordinary member-body lowering, visibility and metadata where correct for enums. Implement the supported companion/static initialization rules and references to entries. Preserve enum APIs and user-member signatures. Entry-specific class bodies remain explicitly unsupported. Do not accidentally broaden to unrestricted enum inheritance.

Acceptance: methods read initialized per-entry state, companion factories return the correct entries, and official Kotlin metadata consumers work. Include initialization-order/forward-reference rejection cases from reference evidence and regress the existing bare-enum suite.

### 23. Validate GotoTab and assess ReadingGuide

Dependencies: 20–22.

Work: compile the whole unchanged `GotoTab.kt`, including its alias, with no special handling of application names. Exercise all entry values and a consumer of `OnGotoFinished`. Separately assess unchanged `ReadingGuide.kt` with actual Android/app dependencies; it also uses library calls, inferred initializers and Double. Record its next blockers, and reassess after task 25. Full ReadingGuide support is an assessed stretch target, not a prerequisite for claiming GotoTab support.

Acceptance: GotoTab passes emission, enum behavior, signatures and Kotlin consumer checks. ReadingGuide has a documented dependency-aware outcome rather than a prediction based only on enum syntax.

### 24. Implement bounded Double semantics

Dependencies: 9, 11; follow the existing Float/Long implementations without assuming identical rules.

Work: cover scalar `Double`/`Double?` types, literals, local/field/parameter/return values, arithmetic, comparisons, conversions, JVM two-slot values and verifier frames, and boxing/unboxing in supported generic contexts. Specify overflow/conversion, NaN and signed-zero behavior from Kotlin/reference evidence. Ensure descriptors `D` and boxed `java/lang/Double` are used in their correct contexts. DoubleArray and unsupported numeric combinations may remain explicitly rejected if not needed by the target; do not silently coerce them to Float.

Acceptance: boundary literals, infinities, NaN, both zeros, null boxing, mixed numeric conversions and Java method calls pass independent assertions and official comparison. Check equality distinctions between primitive comparisons and boxed/generic contexts. Validate affected existing data-class/generic paths or reject unsupported new shapes explicitly.

### 25. Validate Double through TextColorUtil

Dependencies: 24, 6, 12; actual AndroidX/Android dependencies available through task 4.

Work: compile unchanged `TextColorUtil.kt` against the real ColorUtils implementation and dependencies. Exercise actual library methods, not compiler intrinsics or replacement APIs. Compare deterministic color outputs around luminance/contrast thresholds, extreme channel values and alpha handling. Run in an appropriate Android test environment if required by actual dependency behavior; label pure-JVM checks accurately.

Acceptance: unchanged target compiles, real library calls return matching independently checked results, Java/Kotlin consumers link, and verifier checks pass where applicable. Reassess ReadingGuide and record remaining barriers without silently enlarging this checkpoint into Compose or app UI support.

Checkpoint E exit: GotoTab and TextColorUtil verified, existing enum/numeric regressions pass, and ReadingGuide status is evidence-based.

## Checkpoint F: annotations, visibility and module delivery (tasks 26–30)

### 26. Support ordinary annotation arguments and use-site targets

Dependencies: 8–10, 20; reuse metadata and classfile annotation machinery.

Work: first inventory the annotation forms required by selected original sources/tests. Implement a stated subset of existing annotation classes, constant argument forms, retention and legal use-site targets. Distinguish field/getter/setter/parameter/property placement; Kotlin property annotations may require metadata/synthetic conventions beyond Java fields. Preserve array/class/enum argument boundaries explicitly. Validate how type-use annotations differ rather than treating them as declaration annotations. Do not add custom annotation declaration support unless a selected target requires it.

Acceptance: reflective runtime annotations, class-retained classfile attributes and official Kotlin consumers match the declared scope. Invalid targets/types reject. `@Serializable`, `@Composable`, and other annotations requiring transformations must retain explicit unsupported/plugin-needed behavior; ordinary annotation parsing must not produce false compatibility claims.

### 27. Complete targeted visibility and internal-method behavior

Dependencies: 8–10, 18–22 where those declaration forms are exercised.

Work: implement private top-level declarations and internal object methods needed by assessed sources. Preserve file-private scopes across a batch, module-internal access, generated JVM names and metadata. Reuse established module/friend consumer conventions. Handle overload/member-name collisions after mangling. Verify rules for each supported declaration kind rather than globally enabling visibility keywords.

Acceptance: positive same-file/same-module consumers, negative cross-file-private/external-internal consumers and supported friend-module cases match the reference. JVM-visible members and Kotlin visibility are both checked. Reassess small app candidates before selecting any larger utility as a new integration target.

### 28. Make remaining diagnostics actionable

Dependencies: incorporated throughout earlier tasks; final audit after 26–27.

Work: retain original source path and span across parsing, shared indexing and inference. Name unsupported value classes, unsupported enum entry bodies, plugin requirements, missing dependencies and invalid signatures accurately. Add a stable category/code if needed by the harness; do not let categorization depend only on incidental English text. Distinguish invalid Kotlin from valid unsupported constructs using reference acceptance.

Acceptance: targeted unsupported fixtures identify the actual construct and location; ordinary errors are not all reduced to line 1. Confirm `Gid.kt` reports unsupported value-class semantics rather than an object-modifier error, without implementing value classes.

### 29. Validate a complete selected module

Dependencies: all relevant preceding implementation tasks; 4, 10 and 26–28 are mandatory.

Work: choose the smallest meaningful compatible module from the resolved assessment. Prefer the existing AlkitabModel/AlkitabIo/AlkitabYes2 driver support before extending to a new module. Existing module evidence is a regression baseline; if all selected modules already passed historically, label this as batch-integration revalidation, not new module coverage. Run official and candidate builds in clean disposable copies with matching module inputs/options. Candidate-owned production Kotlin must all use Korrust; external modules may use normal official toolchains with recorded boundaries. Exercise downstream Java/Kotlin consumers, module tests, actual AGP D8/dexing and packaged outputs where supported. A standalone D8 check alone is not a Gradle module build.

Acceptance: recorded clean module builds, real dependency inventories, test outcomes and D8 logs. A second build after a declaration removal/change must agree semantically with an independent clean rebuild; stale artifacts must not mask failure. State exactly which modules/tasks ran, which compiler handled each, and any untested Android/device behavior. If environment provisioning fails, preserve the concrete failure and complete independent work, but keep this task blocked rather than passed.

### 30. Freeze, publish evidence and recommend the next milestone

Dependencies: 1–29 resolved; any remaining blockers must be named, with the plan reported incomplete.

Work: freeze compiler, fixtures, harnesses and inputs; run final relevant suites and corpus/module assessments. Record per-case outcomes, actual commands, versions, fingerprints and logs. Publish fresh correctness/features reports and regenerate HTML with valid evidence links. Keep compiler-class emission, behavior, Kotlin interoperability and module results distinct. Derive counts from cases; do not claim a predetermined coverage number or speedup. Update generic README instructions only if commands/contracts changed, never with today's findings. Review source changes for API-name special cases and changes to tracked upstream snapshots.

Acceptance: current passing evidence supports every completed task, regressions have been fixed or explicitly disclosed as incomplete work, and the execution record points to exact reports. Recommend a subsequent milestone from remaining blockers (possibly value classes); do not implement it. Return a concise delivery summary and remaining limits. Commit/push/PR actions require the execution request or applicable Korrust workflow to call for them; no new PR is needed merely to publish local reports.

## Verification and commands

Use these existing commands after selecting the relevant checks; do not run every expensive suite after every small edit. Restore configured environment variables for each shell invocation.

```sh
cargo fmt --check
cargo test --locked --offline
cargo clippy --all-targets --locked --offline -- -D warnings
cargo build --release --locked --offline
python3 tools/check.py
python3 tools/check_declarations_twenty.py
python3 tools/check_next_twenty.py
python3 -m unittest discover -s tests -p 'test_*.py'
python3 tools/reports.py
```

Run `check_androidbible_simple.py`, `check_androidbible_ten.py`, `check_androidbible_twenty.py`, and `check_library_interop.py` with explicit resolved `--android-jar`, `--d8-jar`, and `--androidx-annotations` where their CLI requires them. Check `--help` before invocation. Use `build_androidbible.py --help` and existing module harnesses to select supported module/variant/work directories; do not invent flags. When adding new regression/batch/feature entry points, document their exact final invocation in the report and execution record.

The common compiler and declaration/interop changes can affect old suites even if their new target passes. At final delivery, rerun the applicable existing suites against the final binary, including the selected app regression manifest, and complete Rust and Python checks. Report-generator tests are required when report generation/classification changes. Existing source inventory verification may encounter documented Git LFS image pointers; verify/fetch them through the established reference workflow, and distinguish setup issues from source mutations.

## Integration, recovery and execution record

Use sequential checkpoint integration. Parallelize independent fixtures/reference research/harness work only when ownership is clear. Do not let agents run final report harnesses while another agent edits the compiler or fixtures. Review that each implementation supports general language semantics and does not recognize application/library names to substitute behavior.

Maintain this section during execution as a working record, separate from immutable dated evidence. After each checkpoint or before a session handoff, append:

- Task IDs completed, in progress and blocked; completion requires the acceptance criteria above.
- Current Git revision and uncommitted files; ownership of any ongoing agent work.
- Exact compiler build and dependency setup, latest passing/failing command, and report paths.
- Implementation decisions affecting interfaces/scope, with the reason and tests.
- The next concrete action and any unresolved blocker. Never record a planned test as passed.

Current execution state: all tasks 1–30 are pending. Only assessment/documentation and this plan exist; no implementation work for the roadmap has started.

Suggested prompt for a new Sol chat (also usable in this chat after switching):

> Execute all 30 tasks in `/workspace/korrust/reports/features/2026-10-26-androidbible-compatibility-execution-plan.md`. Use Sol as the lead for this task, overriding the standing Astra-main preference in AGENTS.md; use Sol 6.1 High for implementation subagents. Read the plan and repository instructions, preserve existing workspace changes and unchanged upstream snapshots, and work through all six checkpoints. Keep the README general. Validate each feature as specified, maintain the execution record, and continue through ordinary implementation decisions without asking me to reapprove each checkpoint. Do not start the deferred value-class or other future milestones. Do not commit, push, or create a PR unless I separately request it.
