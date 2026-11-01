# Isolated suspend foundation handoff

Base: `2558a2e77dd67902d2b02262cd543d4adc2ed822`. Source and fixture paths/hashes: `suspend-handoff-manifest.json`. Review `suspend-handoff.patch` with three-way integration against the base. No root source/test/tool files were modified by this lane.

Final focused proof: `target/acceptance-v3/report.json`: complete, 9 positive groups, 5 invalid rejected, 11 valid-but-unsupported boundaries, unchanged inputs. Raw argv/output in `target/acceptance-v3/commands/`; frozen code/fixtures/harness/provenance in `target/acceptance-v3/inputs.zip`. Identical Java callers compiled only against official output execute both with JVM verification. Kotlin consumers compile against each output and execute both. Logical Kotlin metadata and public JVM generic signatures/descriptors match Kotlin 2.3.21. The 9 positives are five runtime groups, two consumer directions, logical metadata, public ABI; they are not a count of language features.

Runtime proof includes actual delayed suspension, code after two resumptions, saved reference/primitive/two-slot locals, nested source and ordinary-resolved compiled suspend calls, callback argument evaluation and mutable local spill order, null/Unit results, private bridge access, immediate/resumed/post-resume exception identity, and callback/result/ordinary-expression boxing identity. Pinned and candidate javap logs assert Boxing.boxInt inside boxingPositions and Integer.valueOf inside the separate ordinaryBox helper. Runtime assertions independently check these identity semantics.

Unchanged upstream proof: `target/upstream-v3/report.json`: complete, 1 pass and 3 unsupported, unchanged inputs. Both WITH_COROUTINES cases retain the complete unchanged helper and compile it with each provider; their first boundary is the general anonymous object expression at CoroutineHelpers.kt:14. Generic Result/helper support remains a prerequisite. kt49834 rejects nested suspension. inlineUnitFunction passes original box(), Kotlin consumers, and top-level signatures, but box() does not call its suspend function; it establishes source/ABI compatibility, not actual suspension coverage. Authored delayed-continuation fixtures supply the runtime proof. No helper was replaced by oracle bytecode or stripped.

Rust validation: `target/validation-v3/rust-checks.json`: 3 suspend frontend tests, 1 recursive value-class guard, 28 metadata tests, 70 parser tests; all-target Clippy -D warnings and formatting pass. Earlier `frontend.log` omitted JAVA_HOME and failed MissingDependency java.lang.Object; the explicit-JDK rerun is frontend-jdk.log and passes. Other early no-JDK logs are historical, not final validation.

Boundaries: top-level nongeneric straight-line suspend declarations, direct suspension stores/expressions/returns, at most eight points, ordinary primitive/reference types and suspend function parameters. Source suspend lambdas/references/conversions, member/extension/generic/default/vararg/inline declarations, nested suspension control flow/arguments/expressions, imported inline/default/member suspension, nullable Unit and Unit callback inputs are explicit unsupported. Source generic Result, value classes inside suspend IR, generated frame names/spill shape/DebugMetadata/debugger/stacktrace equivalence are unclaimed. Root must wire loaded value registry into validate_suspend; isolated baseline passes an empty set and the recursive guard test supplies a fake registry.

V1 and V2 evidence remains intact. V1 had no final ordinary-expression boxing proof and its upstream helper parser failures were unclassified. V2 had all 9 positives but nullable Unit was unclassified (1 correctness failure); V3 corrects only that rejection category and adds logged javap boxing assertions. Every focused version retains its full input source zip. Do not replace historical reports with V3 results.

## Reproduction

Run from `/Users/yuku/f/git/korrust/target/resume-suspend`, choosing new output directory names:

```sh
export JAVA_HOME=/Users/yuku/Library/Java/JavaVirtualMachines/jbr-21.0.8/Contents/Home
export RUSTUP_HOME=/Users/yuku/f/git/korrust/target/rustup
export CARGO_HOME=/Users/yuku/f/git/korrust/target/cargo-home
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo build --release --locked --offline
python3 tools/check_suspend_foundation.py --candidate target/frozen-v3/korrust --output target/acceptance-fresh
python3 tools/upstream_jvm.py --candidate target/frozen-v3/korrust --matrix tools/upstream_suspend_foundation.json --output target/upstream-fresh
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo test --release --locked --offline --test suspend_frontend
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo test --release --locked --offline --lib forbidden_carriers
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo test --release --locked --offline --lib metadata::
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo test --release --locked --offline --lib syntax::tests
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo clippy --all-targets --locked --offline -- -D warnings
/Users/yuku/f/git/korrust/target/cargo-home/bin/cargo fmt --check
```

The copied upstream adapter is root-owned and is excluded from the implementation patch. Final integrated acceptance must use root's current adapter and freeze the merged sources. The input source zip preserves the isolated harness but does not include compiler binaries.
