# AndroidBible compatibility: execution plan for Sol

Status: completed all 30 tasks across all six checkpoints, within the recorded bounded acceptance scope. See the final accepted delivery record and linked reports below.

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

Initial execution state at plan creation: all tasks 1–30 were pending. Only assessment/documentation and this plan existed; no implementation work for the roadmap had started.

### Execution started, 2026-10-26

- User authorized all six checkpoints with Sol leading and no repeated checkpoint approval. No commit, push, or PR was requested.
- The managed checkout initially contained only baseline `9fdbdf6736f187c3ffbffc985a0c85b13f49de14`, on `work`, with a clean index and worktree. The plan and assessment were missing. Read-only remote inspection found the exact handoff in `6244955d3504db548a673987cd00780c4d2d95f7`; fetched it and created local `master` tracking `origin/master`. This restores the assessment and general README clarification without discarding workspace changes or creating a commit.
- Tasks 1–2 are in progress with Sol 6.1 High compiler lead `sol_lead`, who exclusively owns Rust compiler files and Rust tests. Tasks 3 and 5 are in progress with Sol 6.1 High agent `regression_harness`, owning new regression/assessment tools and their dedicated tests. Task 4 is in progress with Sol 6.1 High agent `build_model`, owning `tools/build_androidbible.py` and dedicated build-model tests. Tasks 6–30 remain pending; later integration depends on checkpoint A evidence.
- Read project/reference/report instructions and the managed cloud environment runtime skill. Environment reports connected; startup networking policy permits unrestricted HTTP through the inherited proxy. No credential values were inspected.
- Setup command: `source /workspace/korrust-setup/env.sh && cargo fetch --locked && python3 tools/setup_reference.py && cargo build --release --locked --offline`. Cargo dependencies fetched; pinned `kotlinc-jvm 2.3.21` reports JRE 21.0.12.1+1-LTS. Initial release build is in progress; no compiler feature validation is yet claimed.
- Android SDK and Gradle caches are absent. The build-model owner is investigating reversible provisioning and will retain setup failures separately from compiler outcomes.
- Next actions: reproduce the unchanged PlaybackState failure under matched dependencies, reduce its cause, finish the required target manifest and resolved-input model, then run checkpoint A verification before beginning the shared-source refactor. Historical evidence and upstream snapshots remain unchanged.

### Checkpoint A development and isolated repair verification

- Tasks 1–2: reproduced the unchanged target with an independently rebuilt baseline at `9fdbdf673`. The trigger is ordinary compiled generic factory resolution losing constructor parameter expected types, not a special companion/IDLE rule. Before repair, adding the real standard library makes `PlaybackState` reject with either raw JAR or supported directory dependencies; omitting it accepts. Both official and candidate `AudioLogEntry` dependencies reproduce the issue. The pinned standard-library hash matches the historical successful suite.
- General expected-result inference repair propagates argument/result constraints, preserves explicit/argument constraint conflicts and nullable shapes, and retains original companion inference diagnostics. Persistent Rust coverage uses a custom `fabricate<T>()` library name. Added independent reduced fixtures, a 26-assertion fixed Java caller and an official Kotlin consumer under `tests/fixtures/compatibility/`.
- Fresh immutable repair evidence: `reports/correctness/2026-10-26T083849375925Z-playback-regression.json`, generated by `source /workspace/korrust-setup/env.sh && python3 tools/check_playback_regression.py --compiler target/checkpoint-a-compiler/korrust-fixed-release --baseline target/checkpoint-a-baseline-build/release/korrust --frozen-inputs target/checkpoint-a-compiler/frozen-inputs.json`. Java caller bytecode was compiled against official output and run against both providers with `-Xverify:all`; Kotlin consumers and relevant signatures match. Invalid inferred cycles reject with both compilers. The earlier `2026-10-26T083731818965Z-playback-regression.json` remains as a failed full-directory probe exposing versioned module-descriptor indexing; task 6 addresses that separate classpath defect. The successful reproduction omits only module descriptors from the supported directory input, retaining package metadata and meaningful overrides.
- Task 3 tooling/manifest and task 5 reusable limited/resolved assessments are implemented with dedicated unit tests, but full aggregate/corpus execution remains pending the next shared integration freeze. The regression selection names all 21 required files and retains existing suites; six missing target-specific Kotlin consumer checks are explicit `not_run`.
- Task 4's actual official AlkitabModel build/model now passes in a disposable copy after SDK/Gradle provisioning. Working evidence is `target/androidbible-model-task4-artifacts/official/build.json`; publication into immutable dated evidence is pending review. Captured production/test/generated sources, resolved Kotlin/compiler plugin inputs, ordered compile/runtime paths and missing-input checks must remain distinct from candidate acceptance.
- Frozen repair binary and inputs are preserved under `target/checkpoint-a-compiler/`. Compiler work has moved into an isolated next development stage while required A integration evidence is still pending; checkpoint A is **not yet complete**. Sol compiler lead owns batch API/index/inference (`src/lib.rs` and checking/source machinery); `archive_policy` owns `src/classpath/archive.rs` and shared Python classpath policy; `batch_publication` owns CLI/output ownership and CLI tests. `build_model` retains Gradle driver ownership. New `library_calls` agent prepares independent checkpoint C fixtures/reference evidence without editing the compiler.
- Tasks 6–10 are in development, not accepted. Tasks 11–15 have reference/fixture preparation only; 16–30 remain pending. Next action: finish batch integration, freeze all compiler/fixture/harness inputs, run the required app regression selection and limited/resolved assessments, and integrate checkpoint A/B evidence before claiming completion.

### Independent A/B validation snapshots and later development

- Preserved independent source/tool/fixture copies at `target/checkpoint-ab-snapshot` (B1) and `target/checkpoint-ab-repaired` (B2). Validation uses their own binaries and immutable inputs; subsequent compiler development occurs only in the main checkout. These are checkpoint snapshots, not final compiler evidence. B2 copies upstream trees through unchanged hardlinks rather than directory symlinks, preserving repository-relative assessment identities. No checkpoint has been accepted based only on emission.
- B1 batch differential report `2026-10-26T085832906847Z-batch-compilation.json` passes 38 cases. The later B2 report `2026-10-26T090534634250Z-batch-compilation.json` also passes 38 cases with stricter invalid-Kotlin classification. They include official-compiled fixed Java callers, official Kotlin metadata consumers, public signatures, reversed input order, cross-file inference/recursion/defaults/overloads, invalid cycles/visibility/declarations, and changed/removed-source cleanup versus independent clean output. Reports are initially generated beneath each snapshot's `reports/`; copy immutable finished evidence to the main report tree at integration.
- Required regression suites exposed a real expected-type interception regression: non-generic Kotlin String/array constructor calls entered unsupported compiled inline expansion. The repair restricts expected-result interception to actual generic library candidates without checking names. B1 Rust failure evidence is `2026-10-26T085831583637Z-io-byte-arrays.json`; B2 clears the three affected object tests. Two remaining Rust frontend failures concern changed diagnostics for duplicate functions and a missing explicitly imported dependency; they remain unresolved rather than counted as passing.
- B1 simple-source suite passes (`2026-10-26T090006615315Z-androidbible-simple-sources.json`). The ten-feature suite exposes a separate list-indexing runtime mismatch (`2026-10-26T090418280799Z-androidbible-ten-features.json`): Java-corrupted `List<String>` contents cause an extra candidate nonnull assertion where official Kotlin returns null. The compiler lead is reducing the mapped generic-return contract; existing assertions will remain intact. The required aggregate remains incomplete.
- The initial real resolved AlkitabModel assessment timed out at the 30-second candidate limit under concurrent work (`2026-10-26T090024400894Z-androidbible-resolved-assessment.json`). A fresh B2 run with 180-second candidate/official limits completes and both compilers emit the unchanged source (`2026-10-26T090536133522Z-androidbible-resolved-assessment.json`); behavioral/ABI/consumer dimensions remain separately `not_run` in this probe. The redundant intermediate whole-corpus scan was stopped; final corpus evidence must run against the final frozen compiler.
- The Gradle owner is validating frozen B2 complete-module builds, AARs, fixed Java callers, Kotlin consumers, signatures, and a controlled disposable-source edit/removal against an independent clean rebuild. Existing source-snapshot invariants remain enforced; edit experiments must record their patches and separate them from unchanged builds.
- C tasks 11–12 have narrow general lambda-result/default-mask repairs and official comparison underway. Tasks 13–14 (ordinary vararg/spread) are in implementation. All three task-15 targets have official runtime/consumer reference fixtures using real dependency closures; candidate acceptance is pending. The syntax owner delegated by the Sol lead owns `src/syntax.rs` and dedicated syntax tests; other compiler files remain with the lead.
- D tasks 16–20 have completed official fixture preparation, including unchanged LogInterface, escaped test names with real JUnit, discard effect traces, fun-interface identity/capture, and type-alias consumer/negative cases. Official-only development evidence is `target/checks/2026-10-26T085244673576Z-compatibility-declarations/evidence.json`; candidate implementation and acceptance are pending.
- E tasks 21–25 have official-only reference evidence in `reports/correctness/2026-10-26T090554488129Z-compatibility-enums-numeric.json`. Its 14 passing reference groups and four expected rejections are not candidate support claims. Actual TextColorUtil runs against AndroidX core 1.17.0 and the Android framework implementation with 2,725 independent color cases; GotoTab, numeric boundaries, and ReadingGuide's disclosed real dependency closure also have reference fixtures. Candidate implementation remains pending.
- F tasks 26–28 have annotation/visibility/diagnostic fixture preparation underway; 29–30 remain pending final relevant features and a final freeze. README edits describe only the general batch CLI and ownership contract. No commit, push, PR, value-class implementation, or other deferred milestone has been started.

### Regressions carried forward and the builtin declaration boundary

- Intermediate A/B evidence is published at `reports/correctness/2026-10-26T091301199680Z-compatibility-ab-development.json`, with exact source/tool/fixture ZIPs, hashes and logs. Its status is incomplete. The first whole required aggregate additionally exposed same-package compiled dependency lookup in `Construction.kt`; this and list-null behavior remain required repairs, not expected rejections.
- Frozen B2 task-10 module proof passes all 18 cases: `reports/correctness/2026-10-26T090950355596Z-androidbible-batch-model.json`. It includes real official/candidate AlkitabModel AAR builds, fixed Java/Kotlin consumers, signatures, a second actual Gradle build after a controlled declaration removal/change in a disposable copy, equality with independent clean candidate outputs, stale nested-class deletion and preserved output after an invalid batch. This is scoped task-10 acceptance, not acceptance of all A/B regressions, task 29, Android-device behavior or new historical module coverage.
- The original plan's assumption that array factories have ordinary executable JVM bodies is incorrect. Pinned `references/kotlin/libraries/stdlib/jvm/builtins/Library.kt` is marked `@file:kotlin.internal.JvmBuiltin` and `@file:kotlin.internal.SuppressBytecodeGeneration`; its array factories are bodyless platform builtins. The actual pinned standard library contains their serialized declarations in `kotlin/kotlin.kotlin_builtins`. Task 13 will use shared declaration-driven builtin resolution and supported vararg/array signature shapes, preserving normal scope/overload rules and using the same packing/copy machinery. It must not synthesize an ordinary library method body or dispatch on an application/API name. Other builtin shapes remain explicitly bounded. This is a general compiler prerequisite for task 15, within the authorized milestone.
- The compiler lead has implemented the general mapped-collection generic-result correction, early external-import validation and updated duplicate-declaration diagnostic assertions; these are development changes awaiting a fresh freeze/run. Narrow C11/12 generic/default checks pass, including compiled 34-parameter defaults. Ordinary Java/Kotlin/source varargs and spread effects are executing actual method bodies; builtin declaration support remains in development.
- A fresh upstream inventory verification confirms zero changes in both application snapshots and the specification; all Kotlin source files also match except the same five preexisting documentation-image Git LFS pointers. Exact logs are retained in the A/B development report. No upstream source was edited to unblock compilation.

### Checkpoint C frozen feature validation

- Compiler, fixtures and tools were copied independently to `target/checkpoint-c-snapshot`; its 37 Rust source hashes are in `compiler-sources.json`. The settled C checker/fixtures were copied before the final run. The lead then resumed D development only in the active checkout. No snapshot compiler/fixture/tool input changed during the C feature run.
- `reports/correctness/2026-10-26T093913821458Z-compatibility-calls.json` completes with **124 derived passing cases**. Exact invocation: `source /workspace/korrust-setup/env.sh && python3 tools/check_compatibility_calls.py --android-jar /workspace/korrust-setup/android-sdk/platforms/android-36/android.jar --androidx-annotations /workspace/korrust-setup/gradle-home/caches/modules-2/files-2.1/androidx.annotation/annotation-jvm/1.6.0/a7257339a052df0f91433cf9651231bbb802b502/annotation-jvm-1.6.0.jar`, run inside the frozen C copy. Finished reports/logs are published in the main report tree; a reproduction ZIP of the snapshot's own source/tool/fixture files is archived beside the report.
- Tasks 11–15 now have scoped feature acceptance evidence: nullable generic constraints, compiled constructor/instance/top-level defaults including a 34-parameter mask boundary, ordinary Java/Kotlin/source varargs, copy-preserving spreads/effect/exception order, and all three unchanged integration targets. Fixed Java callers are compiled against official output and run against both outputs with `-Xverify:all`; official Kotlin consumers, negative nullability consumers and relevant public signatures are checked. ReadingPassage uses the original official-compiled DesktopVerseParser and actual unchanged Java Book/Ari/IntArrayList dependencies, with no Android replacement APIs or Parcel behavior claim.
- Builtin factory resolution reads actual serialized builtin declarations, recognizes supported declaration shapes, and lowers through shared vararg packing. Source shadowing and an ordinary vararg function returning an array prove that normal functions still execute their bodies. Full stdlib-directory extraction retains all 1,000 physical files including `.kotlin_builtins`, package metadata and versioned module descriptors; JAR/directory class bytes and runtime agree.
- Bounded source vararg declarations cover represented primitive arrays (Int/Byte/Char). Reference source varargs reject explicitly because their `Array<out T>` projection is not represented; compiled reference/generic Java/Kotlin varargs are covered. Compiled defaults are bounded to 64 parameters; source defaults retain their existing 32-parameter limit. These restrictions are not full Kotlin vararg/default coverage claims.
- The broad required selection still has unresolved acceptance work: its first C Rust run (`2026-10-26T093915534508Z-io-byte-arrays.json`) hits an old rejection expectation for mixed platform/String branches. Review found branch-order/nullability weaknesses in the new common-type join, which the lead is correcting with independent effect/nullability fixtures before final acceptance. The C ten-feature suite's Java/Kotlin behavior now passes the list-null regression and fails only its old nullable-Double boundary marker (`2026-10-26T094303055199Z-androidbible-ten-features.json`); task 24 will deliberately supersede that unsupported boundary with numeric evidence. These failures remain recorded and the whole A/B/C delivery remains open pending fresh broader checks.
- D tasks 16–20 are in implementation. Sol lead's syntax/metadata child exclusively owns `src/syntax.rs`, `src/metadata.rs`, and `src/classpath/metadata.rs` for functional-interface identity and concrete alias syntax/metadata; lead owns checking, source indexing, classpath integration and bytecode. E implementation and F acceptance remain pending. The final module-delivery entry point is prepared as `python3 tools/check_androidbible_delivery.py --frozen-root <final-snapshot> --capture-app-model`; it has not been run against the final compiler.
- Expanded Alkitab capture-only evidence is `reports/correctness/2026-10-26T093620071635Z-androidbible-app-input-capture.json`. It explicitly records incomplete generated inputs/33 missing outputs and does not claim an Alkitab app build. Complete selected-module inputs remain separate; KSP, Compose transformations and full-app compilation are still deferred.

### Owner-requested quota pause, 2026-10-26

- The existing coordinating chat relayed the owner's new restriction: stop when weekly quota remaining is 40% or less, and pause if actual remaining quota cannot be verified. This supersedes continuing implementation while quota visibility is unresolved. The original task, workspace, model choices, and all changes are preserved; no replacement chat or milestone was created.
- Available tools do not expose actual weekly Codex quota. The installed CLI protocol includes the supported read-only `account/rateLimits/read` method, but `codex app-server proxy` exits 1 because `/run/codex-environment/codex-home/app-server-control/app-server-control.sock` does not exist. No percentage was returned. Remaining quota is **unknown**, not inferred from model-capacity errors or assumed above 40%. No credentials were read or authentication/access restrictions bypassed.
- All workers have paused at safe boundaries. No new implementation or validation may start until actual weekly remaining quota is verified **above 40%**. Continuation is pending quota visibility, not denied by approval review. Existing completed reports and working files remain intact; no commit, push, PR, reset, or discard occurred.
- Current source builds in development, with an unused `LibraryMethod.abstract_` warning pending functional-interface integration. This is not a final formatting/Clippy/test acceptance claim. The last in-flight build completed with exit 0; no known compiler write/test command remains running. Persistent Gradle daemons are unrelated to new execution.
- A/B remain open as integrated checkpoints. Task 4's complete selected-module input model and task 10's frozen 18-case real module/rebuild proof pass. Batch differential evidence passes 38 cases. The 21-source required suite still needs a fresh final run after all repairs and deliberate boundary updates.
- Tasks 11–15 have frozen scoped acceptance: `2026-10-26T093913821458Z-compatibility-calls.json` passes 124 cases and all three unchanged selected app targets. Later common-platform join corrections and ordinary compiled-inline allocation support are saved but require fresh broader evidence. The completed C aggregate (`2026-10-26T093915319504Z-androidbible-regression.json`) remains failed: baseline hits the superseded platform-branch unit boundary; ten hits the old nullable-Double diagnostic; twenty exposes ordinary inline allocation in Ops. Simple, declarations, and library interop pass their scoped suites. Do not relabel these historical failures.
- The general platform join repair is saved: compatible nullable types take precedence, matching platform/concrete branches join a platform type, and expected `if` contexts preserve null or assert before later arguments as appropriate. Its updated Rust test passes. New independent platform/effect fixtures passed 54 official-only checks in `target/checks/2026-10-26T095004901675Z-compatibility-calls/evidence.json`; candidate runtime has not been run. Final C tools/fixtures differ intentionally from the older frozen 124-case snapshot and must be recopied for final validation.
- D tasks 16–20 are partially implemented: escaped/discard syntax, functional-interface/type-alias data structures, alias metadata/facades/index/private-file keys, and typed discard checks are saved. Child syntax/metadata tests passed (43 syntax, 20 metadata). Latest alias-import fix compiled but has not been revalidated. Functional-interface checker/SAM conversion remains unfinished; ordinary JUnit annotation retention is explicitly unsupported until emission hooks exist. Stronger official D oracle evidence passes 25 cases at `target/checks/2026-10-26T095014142724Z-compatibility-declarations/evidence.json`.
- Sol lead retains all shared compiler ownership. Its child returned `src/compiled_inline.rs`: ordinary external allocations now pass 11 narrow tests; inline-owner generated allocations still reject explicitly. Actual old Ops runtime has not been rerun. The generated-helper/JVM-owner collision preflight concern remains pending.
- F task 26 has a saved isolated serializer at `src/bytecode/annotations.rs`, owned by `build_model`. It implements attributes/member/parameter annotation merging and preserves existing attributes. Its five byte-level tests are **not run** because `mod annotations` and parent checked placement hooks are not integrated. Parent has defined the annotation value structures and retained annotation AST; checking/retention/placement/emission remains unfinished.
- E tasks 21–25 have official reference evidence only; enum/Double candidate implementation and acceptance remain pending. Remaining F tasks 27–28, final module task 29 and freeze/publication task 30 are not complete. The prepared final module command is `python3 tools/check_androidbible_delivery.py --frozen-root <final-snapshot> --capture-app-model`; it has not been executed against a final compiler. Source/reference snapshots remain unchanged, subject only to the documented five preexisting LFS-image setup gaps.
- Next action **only after verified quota clearance**: complete D functional interfaces/SAM/alias visibility plus the ordinary annotation prerequisite; fix generated-owner collision protection; then implement/validate E and remaining F semantics. Create fresh frozen snapshots, rerun required regression/C/D/E/F/module/corpus suites, resolve all failures, regenerate and verify HTML evidence, and finish all six checkpoints. Do not start deferred value classes or other future milestones.

### Owner directly resumed execution, 2026-10-26

- The owner's latest direct instruction is: “ok please just continue, disregarding checking for quota.” It supersedes the quota pause and its resume condition above. Resume the original all-30-task scope without further quota checks or renewed checkpoint approval. The pause report `reports/correctness/2026-10-26T100958619222Z-compatibility-quota-pause.json` remains historical evidence of the preserved state, not an active blocker.
- Sol compiler lead and annotation serializer worker have resumed their saved tasks with their existing model choices and exclusive file ownership. Continue D/annotation integration, then E and remaining F semantics, then fresh frozen whole-delivery validation. No work is discarded or restarted.

### Checkpoint D frozen validation and parallel E implementation

- Frozen `target/checkpoint-d-snapshot` contains 39 Rust source files with hashes in `compiler-sources.json`; the unhooked/incomplete `src/bytecode/annotation_files.rs` was deliberately excluded before validation. The settled `tests/d_sam_frontend.rs` was copied before any harness run. All snapshot inputs remain independent from active E/F edits.
- `reports/correctness/2026-10-26T104422171478Z-compatibility-declarations.json` completes with **64 passing cases and one explicit unsupported-language boundary**. Exact invocation inside the frozen copy: `source /workspace/korrust-setup/env.sh && python3 tools/check_compatibility_declarations.py --junit-jar /workspace/korrust/target/compatibility-dependencies/junit-4.13.2.jar`. This supplies scoped acceptance for tasks 16–20: real backtick JUnit methods, discard effects/types, Kotlin functional-interface identity/inherited defaults/captures, unchanged LogInterface runtime/Java/Kotlin consumers, concrete/function aliases and visibility/cycle boundaries. Generic aliases and implicit adaptation of arbitrary function values/references remain explicitly bounded rather than silently accepted.
- The official D options now explicitly use `-jvm-default=enable`, matching the established candidate default bodies plus compatibility helpers. Earlier disable-mode oracle reports remain unchanged. Exact ABI comparison and independent reflection retain abstract/default/static-helper distinctions, including default helper generic signatures. Two persisted SAM Rust integration tests also pass with fixed official-compiled Java callers and Kotlin consumers; log archived beside the D report.
- Latest C/platform fixtures also pass against the same frozen D compiler: `reports/correctness/2026-10-26T104423652411Z-compatibility-calls.json` has **136 derived passing cases**, including both platform/nullability branch orders and effect/assertion timing, all previous vararg/default/builtin checks, and all three unchanged app targets. Finished reports and a frozen D source/tool/fixture ZIP are published in the main report tree.
- Old twenty-feature validation advances past the ordinary inline allocation and Byte literal failures but still exposes a required numeric-context regression: `reports/correctness/2026-10-26T104425121244Z-androidbible-twenty-features.json` fails the existing direct erased `List<Int?>` read followed by `!!` case. Official Kotlin casts to Number and converts a Java-supplied Long there; materialized nullable return/local/Elvis paths must instead retain Integer casts. Existing nuanced assertions remain unchanged. The Double/core owner is implementing the contextual fix; fresh runtime evidence is required.
- Generated lambda/Companion/DefaultImpls JVM-owner collisions and alias/classifier duplicate declarations now reject explicitly. Actual lexical nesting comes from declaration AST rather than guessing from `$` in a name. Scoped collision, alias-private-name and generic Byte boundary tests pass in development; final broad checks remain mandatory.
- E ownership: `archive_policy` owns existing shared Rust core files for bounded Double integration after the enum AST handoff. Sol lead owns new `src/check_enums.rs`, `src/bytecode/enums.rs`, and new enum metadata helpers, with shared hooks deferred until the core owner returns. `build_model` exclusively owns the new annotation serializer/rewriter files. No simultaneous shared-file ownership is authorized.
- E development evidence now shows DoubleModel's Java matrix and unchanged TextColorUtil's 2,725 independent real-library color checks passing. These are development probes, not final E acceptance. Enum checking/emission/metadata helpers are saved and await shared integration. DoubleArray remains outside the required scalar scope unless an actual target establishes a need.
- F ordinary-annotation support is staged: six serializer tests and five raw-classfile rewriter tests pass, preserving original constant pools (including Double widths), metadata, nullability and unknown attributes while merging class/member/parameter annotations. Full checked retention/placement and runtime integration is still pending. The official same-file cross-JVM-owner private-access fixture passes Java/Kotlin assertions and establishes the required synthetic accessors; candidate task-27 integration is still pending.
- Remaining actions: finish/integrate E and F, rerun all affected existing regressions including the direct numeric `!!` case, complete task 29 with one final frozen compiler and real module/corpus inputs, then finish task 30 reports/HTML/link verification and the complete six-checkpoint record. No final all-30 completion is claimed yet.

### Preserved serialization boundary

- Review found a preexisting bounded empty `@Serializable` singleton lowering in `src/check_singletons.rs`, with required historical tests. The plan's deferred/general serialization-transform boundary does not authorize deleting previously supported behavior. Preserve that existing bounded path and its consumed marker; the new generic annotation collector must not duplicate or reinterpret it. Empty parentheses are ordinary equivalent annotation syntax within that old supported predicate, subject to its actual dependencies and semantic checks.
- General/argument-bearing serialization and Compose transformations remain unsupported/plugin-needed. No general serializer, Compose transformation, or other deferred milestone is introduced. Obsolete parser-only rejection expectations must become semantic/dependency/unsupported tests with fresh evidence rather than blocking valid general annotation syntax or removing unrelated old coverage.

### Checkpoint E frozen validation

- Frozen `target/checkpoint-e-snapshot` preserves 45 Rust source files plus settled tools/fixtures independently from active F work. Sol lead received the copy acknowledgment and resumed F hooks. The Double worker returned shared core ownership; enum checker/emitter/metadata helpers were integrated before the copy.
- Mandatory E evidence passes in `reports/correctness/2026-10-26T111923821382Z-compatibility-enums-numeric.json`: **26 positive cases, 20 expected rejections, three explicit unsupported outcomes**. The unsupported outcomes are enum entry-specific bodies, DoubleArray, and the separately assessed ReadingGuide stretch target. Original GotoTab and TextColorUtil compile and execute with fixed official Java callers, official Kotlin consumers, signature checks and JVM verification. The real AndroidX/framework color implementation supplies the 2,725 independent color checks; no fake APIs, device execution or full-app claim is made.
- Reproduction inside the frozen copy: `source /workspace/korrust-setup/env.sh && python3 tools/check_compatibility_enums_numeric.py --compiler target/release/korrust --colorutils-jar /workspace/korrust/target/setup/compatibility-enums-numeric/core-1.17.0.jar --android-runtime /workspace/korrust/target/setup/compatibility-enums-numeric/android-all-14-robolectric-10818077.jar`. The earlier `2026-10-26T111856952602Z-compatibility-enums-numeric.json` remains a setup failure caused by snapshot-relative dependency defaults; the fresh run passes explicit actual paths. Finished evidence and the snapshot's own source/tool/fixture ZIP are published in the main report tree.
- Tasks 21–25 have scoped mandatory acceptance. Enum user parameters remain distinct from JVM name/ordinal arguments; ordinary members/companions/initialization, defensive values/entries behavior and metadata consumers are covered. Double scalar/nullable/data/generic operations, constants, leading-dot literals, two-slot values/frames, IEEE cases, conversions and real Java calls are covered. New invalid implicit-widening/nullable/equality fixtures compare both compilers. No unrestricted enum inheritance, entry-body, DoubleArray or full numeric-language claim is established.
- ReadingGuide's whole unchanged source was reassessed against real original dependencies and selected aapt2-generated resources. Its next candidate blocker is line 13's `entries.firstOrNull { ... } ?: OFF` generic Elvis typing. It remains an explicitly unsupported stretch target; Preferences/application lifecycle/device behavior stays untested. This does not substitute for the mandatory GotoTab/TextColorUtil evidence.
- The older twenty-feature Java runtime now passes the nuanced direct erased numeric `!!` assertion and all previous runtime groups, reaching 78 passing checks before a changed nonproperty-constructor-parameter diagnostic marker fails (`2026-10-26T111858446898Z-androidbible-twenty-features.json`). The source is still correctly rejected at line 1 column 38; official reports unresolved `value`, candidate reports unsupported property `value`. Preserve rejection and refine diagnostics/expectations, then rerun fresh evidence.
- The older ten-feature run (`2026-10-26T111859957477Z-androidbible-ten-features.json`) now fails only because its former nullable-Double unsupported case succeeds. Promote that original case to a genuine positive regression with runtime/null/NaN/identity/signed-zero checks; do not delete unrelated assertions. Existing `tests/unsupported.json` also retains its original nullable-Double source as a supported regression. Final whole-delivery checks must confirm all promoted cases and consume no skipped tests.
- F26 ordinary annotation placement/metadata flags and F27/28 provenance, internal JVM names, file-private bridges/friend access and structured diagnostics are the active implementation stage. Task 29's prepared real three-module workflow and task 30's final frozen suites/corpus/reports/HTML/link audit remain pending. README still contains only the general compiler/CLI contract.

### Checkpoint F integration in progress

- User explicitly renewed the instruction to continue without quota checks. Preserve the earlier quota-pause evidence as history; it is no longer an active execution gate.
- Task 26's ordinary annotation serializer, classfile merger and checked placements are integrated. Official reductions established effective property/accessor/parameter metadata flags, including SOURCE-retention exclusions; persistent fixed Java metadata checks and a SOURCE-retention fixture are prepared. The final candidate annotation/visibility harness has not yet run.
- Tasks 27–28 remain active with Sol worker `library_calls` owning shared compiler files and coordinating separate new-helper ownership for private bridges, metadata and diagnostics. Compile options, friend-path provenance, source/JVM-name separation and structured diagnostic rendering are integrated; cross-owner same-file private bridges and final positive/negative visibility validation are still pending. The lead retains enum helper review. No task-26–28 acceptance is claimed from narrow unit tests alone.
- The README documents only general batch, friend-path, diagnostic-format and output-ownership contracts. Reference verification again confirms all application/specification sources, with the same five preexisting Kotlin documentation Git LFS pointers recorded in `target/final-preparation/reference-verification.log`. Task 29 and the final whole-compiler, corpus and report checks remain pending a settled source freeze.
- Eight Python tooling test groups pass in `target/final-preparation/python-tool-tests.log`. Six new persistent full-pipeline JSON diagnostic CLI tests are in `tests/test_diagnostic_cli.py`; an isolated development binary passes five and exposes an early header-resolution path still reporting an unclassified missing signature type at the function token. Preserve that failure and repair the token/code path before final acceptance; the development log is `target/final-preparation/diagnostic-cli-development.log`.
- Independent review exposed an imported generic factory incorrectly outranking a class instance member during expected-result inference. The reduced official program returns the member's `[9]`; the older frozen candidate returns the imported factory's `[]`. The shared compiler owner is repairing lexical priority generally, with persistent shadowing regressions, while preserving the established explicit-import priority over top-level source functions. Fresh final C/old-suite evidence is required after this repair.
- A metadata review bounds literal `$` characters in escaped classifier names as explicitly unsupported until logical classifier IDs can be preserved. Actual lexical nesting and escaped method/property names remain supported within their tested scope. Function-type alias parameter labels are not claimed byte-for-byte metadata equivalents; alias declaration/export/type semantics retain their official Kotlin consumer checks.
- Settled-source F development preflight now passes all 62 recorded cases in `target/checks/2026-10-26T124959114827Z-compatibility-annotations-visibility/evidence.json`: 24 passes, 12 invalid-Kotlin outcomes, 12 official accepted-boundary observations, 10 unsupported-language outcomes, two plugin-required outcomes and two dependency-failure observations. All frozen-input/reference/binary/source checks pass. Fixed official Java launcher bytecode runs each unchanged friend consumer's ordinary `main()` method; automatic Kotlin JVM launcher bridges remain unassessed.
- All 226 Rust library tests pass after repairing five interface/object regressions without weakening their assertions. The generic-import priority regression now covers accessible/inaccessible inherited members, local members, wrong-arity/type fallback, applicable wrong-result rejection and the established top-level import priority. Formatting, Clippy and six JSON CLI tests pass on the settled development compiler. Full integration tests and final frozen release evidence remain pending; an enum-finality fixture is being reduced to a plain method to isolate that rule, retaining the original unsupported-operator form as a separate rejection.

### Final frozen delivery run, in progress

- The settled compiler passes all **338 Rust tests, zero ignored**, formatting and all-target Clippy with warnings denied. Expanded F development evidence passes **68 cases**, now covering ordinary class internal methods as well as object methods, separate external rejection and both friend-consumer compilers. All eleven promoted historical forms were checked with the pinned oracle; invalid rows remain negative. The old nonproperty-constructor case still rejects with both compilers at line 1, column 38, with the accurate stable candidate code.
- Preserved the first delivery candidate under `target/final-compatibility-snapshot-v1`; its broad integration run exposed nested-interface mirror collision accounting, an escaped-template regression and nested default-package enum metadata. Repairs were made only in the original working tree, then revalidated. No accepted final evidence is attributed to that older candidate.
- Final source copy `target/final-compatibility-snapshot-v2` contains 483 frozen source/test/tool/document files, verified against the original tree at copy time. Manifest SHA-256: `395b80e21aff1533758c714c56b7970887f45d76724ec2c229a2ea7ca5306fb6`. Release compiler SHA-256: `0f6b30d7db79082d5869dded4973cfccde80af451ae521d2cb0bcc218472e8bc`.
- The final required regression aggregate, batch/C/D/E/F/older-next suites, three-module Gradle delivery and full limited-classpath corpus assessment are running independently from this frozen copy. Resolved corpus assessment follows the completed module-input model. The frozen Python discovery run already passes **153 tests**. Commands and lane logs are in `target/final-compatibility-snapshot-v2/target/final-validation/`; the reproducible lane driver is `target/final-preparation/run_lane.py`.
- Known bounded grammar limitation: adjacent `;;` is now rejected at the second enum-entry terminator, matching the pinned compiler's distinct lexical token. The preexisting ordinary class/member separator behavior was not extended into a general grammar milestone. This remains a disclosed non-target limitation, alongside the previously recorded classifier-name, alias-label and deferred-feature boundaries. Final task-29/task-30 acceptance remains pending actual results; no all-30 completion is yet claimed.

### Second frozen run findings and repair

- Preserved all finished v2 evidence plus its exact input ZIP in `reports/correctness/2026-10-26T152134491166Z-compatibility-v2-development.json`. Batch passes 38 cases; C passes 136; D passes 64 with one unsupported boundary; E passes 26 positives/20 expected rejections with three disclosed unsupported outcomes; expanded F passes 68 recorded cases; the older next suite passes. The real three-module delivery passes 146 checks, including its 127-check module proof and 18-check controlled rebuild. Python discovery passes 153 tests. These remain v2 evidence rather than final-compiler claims.
- The preserved required aggregate is incomplete: its baseline exposed two unused enum protected-access helpers, while old ten/twenty/interop tables still expected rejection of newly supported forms. The backend now emits protected bridges only for actual relevant calls. Unchanged RibkaEligibility has exact official private/generic JVM signatures; a new fixed official Java regression covers required deferred/default/conditional/inline protected calls and absence of unused helpers. No ABI assertion was relaxed.
- Every unpromoted row in the general and ten-feature boundary tables was audited. Ten newly supported cases retain their exact IDs/sources and now have pinned acceptance, fixed official Java runtime and ABI evidence. Twenty-feature empty-initializer/open-class cases likewise retain their sources and gain fixed Java checks; all twenty boundary rows were rechecked. Retained rejection markers were updated only to the actual precise current diagnostics. The interop allocating-inline case retains its source and is tested with explicitly recorded matching HotSpot hash options, 65 independent fixed Java checks and ABI comparison; generated/inline-owner allocation remains outside scope.
- The 400-file limited assessment completed with 26 emitted, 367 dependency failures, four explicit unsupported cases and three compiler-failure observations. All historical emission sources remain accepted. The three observations revealed unsupported-syntax diagnostic gaps rather than emission claims: newline-qualified continuation, parameterized supertype and accessor visibility. Structural guards now report those actual constructs with stable codes and exact original paths/spans; all three unchanged originals and nine reductions were accepted by the pinned oracle. Leading-dot decimal and adjacent supported syntax remain covered.
- Latest original-source validation passes **345 Rust tests, zero failed/ignored**, formatting and all-target Clippy. All source and harness owners are settled. A fresh v3 source freeze and complete rerun remain necessary before task-30 completion; v2 reports are immutable history.

### Final compiler and input-model freezes

- Preserved the complete third attempt in `reports/correctness/2026-10-26T163510600503Z-compatibility-v3-development.json`. Its required selection has all 21 source cases passing; only the baseline suite's stale compound-assignment diagnostic marker blocks the aggregate. All 73 retained diagnostic cases were independently audited with their actual dependency context: all reject without output, and only that one marker required updating. No source or case ID changed.
- Final compiler/language freeze: `target/final-compatibility-snapshot-v4`, 494 files, manifest SHA-256 `82ca11fdda8b7e28e0424b644e030a838a13dadaee16ae732fae25f4da0061d7`; native compiler SHA-256 `caa699dac6951e676fdd28375d0ab8488e978a4c5484e6a581caaef334dd42f5`. All **349 Rust tests pass, zero ignored**, with formatting and Clippy clean. Final language and required-regression validation is running from this immutable tree.
- Resolved test probes revealed an input-model omission: Gradle's actual friend outputs were not captured, so the isolated official `Yes2RoundTripTest` failed internal access under incomplete arguments. The pinned plugin's public `BaseKotlinCompile.getFriendPaths` and actual argument normalization were verified. Models now retain ordered absolute paths, provenance, artifact hashes, missing outputs and capture failures; no required path is silently dropped. A fresh three-module official build passed 161 Gradle tasks. The unchanged isolated test rejects with the same module name alone and accepts when only the captured friend outputs are added.
- Assessment now replays those authoritative friend inputs for both compilers, records their provenance/hashes, and reports missing test context as setup failure. A captured empty list grants no access. Sixteen build-helper, 23 assessment and six report tests pass. No compiler, language fixture, regression-manifest or language-harness change was needed for this model correction.
- Refreshed model/assessment freeze: `target/final-compatibility-snapshot-v5`, 494 files, manifest SHA-256 `49eb6125b7a7ca8ccb8324a62630f4a43df9f95184d14696ad1e0953f91c5b64`. Exactly four Python capture/replay/test files differ from v4. All Rust/Cargo sources, language fixtures and language harnesses are identical; the exact v4 compiler binary was copied and checksum-verified. Model/module/corpus/Python checks use v5, while language checks retain their exact v4 frozen inputs and the same final binary. The final audit enforces this separation and archives both input trees.
- Refreshed Python discovery passes **160 tests**. All final role results, derived counts, report links and the execution record still require completion before all-30 acceptance. No further implementation work or source writer remains active.

### Final acceptance boundary audit

- All six v4 feature-suite commands completed successfully: batch 38 passes, calls 136 passes, declarations 64 passes plus one unsupported case, enums/numeric 26 positives and 20 expected rejections plus three unsupported cases, annotations/visibility 68 classified cases, and the older next suite 121 passes with its disclosed unsupported/unassessed outcomes.
- Refreshed v5 module delivery passes all 146 checks. The resolved 400-file assessment completed with four emitted, 393 setup failures and three explicit unsupported-language cases; no unexpected compiler failure remains. Setup failures are incomplete app/generated dependency inputs, not intrinsic-invalidity or behavior claims. The limited assessment is still finishing.
- The v4 baseline reached its final interop table and exposed stale rejection expectations after bounded Java annotation support. All seven original IDs/sources are being audited; newly supported rows require paired pinned-Kotlin acceptance, independent fixed-Java verified execution and exact public/protected ABI checks. The compiler remains unchanged. A fresh baseline/Python freeze will replace only affected evidence; the final required aggregate will explicitly retain unchanged suites with their actual execution origins, report hashes and preserved per-dimension gaps.
- Read-only final evidence review confirmed mixed-origin aggregation preserves compile-evidence paths and all required acceptance conditions. Publication now rejects unexpected corpus failures/timeouts and keeps task claims unassessed until HTML generation and link/anchor audits succeed. Final all-30 acceptance remains pending.

### Final baseline expectation correction

- The complete seven-row Java interop audit found exactly one newly supported form: `java_annotated_method`. Its unchanged source gains independent annotation-presence, ordinary result, virtual override/effect-order, exception identity and null-receiver checks, executed by one official-compiled Java caller against both providers with JVM verification and exact facade ABI comparison. The other six entire row objects remain unchanged. Inherited `hashCode`, annotated-class and annotated-package cases retain pinned official acceptance and strict candidate rejection without output. Earlier focused audit failures are preserved under `target/final-preparation/2026-10-26-interop-boundary-promotions*`; no compiler behavior was changed for these boundaries.
- Both v5 corpus scans completed and their finished evidence was copied intact to `reports/`: limited `2026-10-26T164412809494Z-androidbible-limited-assessment.json` records 26 emitted/367 dependency failures/seven unsupported; resolved `2026-10-26T171122238892Z-androidbible-resolved-assessment.json` records four emitted/393 setup failures/three unsupported. Both retain all 400 unique source records and no unexpected compiler failures. Module delivery evidence is `2026-10-26T164412837221Z-androidbible-module-delivery.json` (146 passes).
- The original v4 simple, ten and twenty suites pass respectively 49, 60 and 93 cases, preserving their explicit unsupported boundaries. Remaining original suites, the fresh baseline/Python freeze and final publication checks still gate task-30 completion.

### Final affected-check freeze

- All seven interop boundary cases passed the focused final audit in `target/final-preparation/2026-10-26-interop-boundary-promotions-v3/result.json`: four supported rows (three preexisting plus the annotated method), three strict unsupported rows, and six helper/schema tests. Earlier failed attempts remain intact. All source ownership is returned; no compiler, fixture or tool writer remains active.
- Final baseline/Python freeze `target/final-compatibility-snapshot-v6` contains 495 files, manifest SHA-256 `8a019b3326c93b8f98689018ea8c5227085a278e2d4101683a596c7936574d30`. Exactly three baseline/helper/test files differ from v5, with all Rust/Cargo inputs unchanged. The exact final compiler binary SHA-256 `caa699dac6951e676fdd28375d0ab8488e978a4c5484e6a581caaef334dd42f5` is retained. Full baseline and Python checks are running; the baseline rebuild must reproduce that binary hash.
- Reproduction: source `/workspace/korrust-setup/env.sh`, then `python3 target/final-preparation/run_lane_v6.py required` and `python3 target/final-preparation/run_lane_v6.py python`. The required lane runs the entire baseline, then `refresh_required.py` computes all 21 source/dimension results from the fresh baseline and unchanged v4 suites using each suite's original compiler/source paths. It verifies actual recorded command exits, preserved source/tool manifests, shared classpath-helper semantics and the identical compiler hash, and records reuse explicitly. Final evidence archives all three frozen harness trees and their drivers; no reused command is represented as a rerun.

### Final frozen verification progress

- Fresh v6 Python discovery passes **166 tests**. The full v6 baseline independently passes **349 Rust tests, zero failed/ignored**, formatting, Clippy and report-generator tests. Its release rebuild reproduces the exact v4 compiler SHA-256, so retained feature/module/corpus evidence still refers to the same executable bytes. Kotlin/Java differential baseline checks and remaining v4 suite completion are still running.
- Fresh readable HTML generation and a complete local-link/anchor audit passed for the published evidence: **663 pages and 210,502 local links**. This is a preparatory audit; task-30 final publication will regenerate and audit the new delivery reports again before marking all tasks complete.
- Final source review records zero production app-target-name occurrences; six matches are confined to tests. Manual review confirms pinned bodyless array declarations are selected by structural metadata and use shared vararg lowering. Existing list/conversion fallback paths remain disclosed. Source-review evidence is `target/final-preparation/final-source-review.json` and will be copied into final delivery artifacts.

### Final accepted delivery, 2026-10-26

**All tasks 1–30 and checkpoints A–F are complete within this plan's stated scope.** The final evidence contains 30 supported task entries; successful assessments, unsupported cases and untested dimensions remain separate from behavior passes.

- [Final correctness, provenance, limits and next milestone](../correctness/2026-10-26T175243339825Z-androidbible-compatibility-delivery.json)
- [All 30 task entries and supporting evidence](./2026-10-26T175243339825Z-androidbible-compatibility-delivery.json)
- [Frozen sources, fixtures, three harness input trees, drivers and exact compiler binary](../correctness/2026-10-26T175243339825Z-androidbible-compatibility-delivery/frozen-inputs.zip)
- [Readable report index](../../target/reports-html/index.html)

| Checkpoint | Tasks | Accepted evidence |
| --- | --- | --- |
| A | 1–5 | [Matched PlaybackState repair](../correctness/2026-10-26T083849375925Z-playback-regression.json), [fresh 21-source regression aggregate](../correctness/2026-10-26T175224398764Z-androidbible-regression.json), [limited assessment](../correctness/2026-10-26T164412809494Z-androidbible-limited-assessment.json), [resolved assessment](../correctness/2026-10-26T171122238892Z-androidbible-resolved-assessment.json), and captured-input/module evidence below. |
| B | 6–10 | [Shared classpath/batch semantics](../correctness/2026-10-26T161828809544Z-batch-compilation.json) and [actual module integration/rebuild delivery](../correctness/2026-10-26T164412837221Z-androidbible-module-delivery.json). |
| C | 11–15 | [Generic calls, nullability, defaults, varargs and original pure utilities](../correctness/2026-10-26T162520115271Z-compatibility-calls.json): 136 passes. |
| D | 16–20 | [Escaped declarations, discards, functional interfaces and aliases](../correctness/2026-10-26T163942044589Z-compatibility-declarations.json): 64 passes and one unsupported boundary. |
| E | 21–25 | [Enums, Double, unchanged GotoTab/TextColorUtil and dependency-aware ReadingGuide assessment](../correctness/2026-10-26T164915244073Z-compatibility-enums-numeric.json): 26 positive checks, 20 expected rejections and three disclosed unsupported outcomes. |
| F | 26–30 | [Annotations/visibility/diagnostics](../correctness/2026-10-26T170157393700Z-compatibility-annotations-visibility.json): 68 classified cases; [module revalidation](../correctness/2026-10-26T164412837221Z-androidbible-module-delivery.json): 146 passes; final delivery and all-task evidence above. |

The fresh [full baseline](../correctness/2026-10-26T173229467527Z-io-byte-arrays.json) completes with **200 passes and 26 explicit unsupported cases**. The required aggregate has emission, independent behavior and ABI checks for all 21 unchanged sources, with official Kotlin consumers passing for 15; the six preexisting documented consumer gaps remain `not_run`. It transparently combines the fresh affected baseline with five unchanged original suites using their real execution roots, compiler identity, report hashes and recorded exit codes. The failed earlier aggregate and stale rejection expectation remain linked immutable history.

Final verification passes **349 Rust tests, zero failed/ignored; 166 Python tests; formatting; all-target Clippy with warnings denied; and report-generator checks**. An independent v6 release rebuild reproduces compiler SHA-256 `caa699dac6951e676fdd28375d0ab8488e978a4c5484e6a581caaef334dd42f5`. All 495 current frozen manifest inputs match the working tree. The final report-generation/local-link/anchor audits pass, covering 744 pages and 246,683 local links at delivery publication.

The three-module result is **batch-integration revalidation** of AlkitabModel/AlkitabIo/AlkitabYes2. Production Kotlin in the selected candidate modules uses Korrust; dependency/toolchain boundaries, official/candidate clean builds, downstream Java/Kotlin checks, module tests, AGP/D8/packaging and the 18-check changed-versus-independent-clean rebuild are recorded. No additional module coverage, complete Alkitab build, Android-device behavior or performance improvement is claimed. Resolved app capture remains partial for generated/KSP/R inputs; 393 resolved corpus setup failures and the limited-classpath dependency failures remain explicit observations rather than intrinsic invalidity or verified behavior. Five known preexisting documentation-image LFS pointers remain setup exceptions; app/spec/compiler source inventories were verified.

All implementation owners have returned their files. Acceptance used `master` at base revision `6244955d3504db548a673987cd00780c4d2d95f7` with the implementation/tests/reports and preserved workspace changes uncommitted. After acceptance, the owner explicitly requested committing and pushing the completed delivery. No tracked upstream snapshot was edited. README changes stay general CLI/output/friend/diagnostic instructions and retain Randy Sugianto (Yuku)'s attribution. Source/API-name review and earlier focused boundary-audit failures are archived with the delivery evidence.

**Next recommendation:** plan value-class JVM representation using unchanged Gid/Rpp, including boxed/nullable/generic forms and Java/Kotlin consumers. This delivery does not implement that next milestone. ReadingGuide remains the recorded unsupported stretch target; coroutines, Compose/general serialization transformations, KSP and full-app/device/performance work remain deferred.

### Git delivery authorized after acceptance

- The owner requested: “When finished, commit and push.” Commit/push authorization follows the completed 30-task acceptance and final HTML audit. The delivery commit includes the compiler, reusable harnesses, tests, general README updates, execution record and immutable dated evidence. Existing workspace changes remain preserved.
- Final post-record verification confirms all 495 frozen source inputs still match, all 30 task entries are supported within scope, the updated execution-record HTML is current, and all 744 pages/246,683 local links pass. Source/compiler/test inputs have not changed since acceptance.
- Publication targets `master` in `yukuku/korrust`, starting from remote revision `6244955d3504db548a673987cd00780c4d2d95f7`. Commit identity and confirmed push outcome are reported with the completed Git delivery.

Suggested prompt for a new Sol chat (also usable in this chat after switching):

> Execute all 30 tasks in `/workspace/korrust/reports/features/2026-10-26-androidbible-compatibility-execution-plan.md`. Use Sol as the lead for this task, overriding the standing Astra-main preference in AGENTS.md; use Sol 6.1 High for implementation subagents. Read the plan and repository instructions, preserve existing workspace changes and unchanged upstream snapshots, and work through all six checkpoints. Keep the README general. Validate each feature as specified, maintain the execution record, and continue through ordinary implementation decisions without asking me to reapprove each checkpoint. Do not start the deferred value-class or other future milestones. Do not commit, push, or create a PR unless I separately request it.
