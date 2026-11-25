# AndroidBible module roadmap

The November 21 survey established compilation for 33 individual Kotlin files. This milestone checks whether Gradle can compile Java against Korrust output, pass debug unit tests and package release libraries that downstream callers can use. [Survey evidence](reports/features/2026-11-21T195150057694Z-androidbible-survey-completed.json)

| Step | Work | Acceptance |
| --- | --- | --- |
| 1. Reproduce AlkitabModel | Build the unchanged module in independent official and Korrust project copies. | Account for every production Kotlin and Java source, verify the Kotlin provider, run Gradle Java compilation and inspect the release AAR. |
| 2. Build AlkitabIo against Model | Select both modules in the same candidate build so Io consumes the rebuilt Model dependency. Keep Bintex as the project's declared dependency. | Apply the same source and packaging checks to Io. Run its original unit tests and compare outcomes with the official build. |
| 3. Verify downstream use | Compile a caller against official output and run that same caller against both pairs of libraries. | Verify JVM linkage and independently assert observable behavior from both modules. Record which consumer and paths ran. |
| 4. Check the survey and publish evidence | Run the fixed 33-file survey on the same candidate binary. Record source provenance, commands, compiler hash and per-case outcomes. | All 33 cases pass under the established classpaths. Link the module, consumer and regression evidence to this roadmap. |

If a build exposes a compiler defect, isolate the failing Kotlin construct, implement its general semantics, compare with the pinned compiler, and commit and push that feature separately. Rerun the affected build and the survey on the final binary.

Each selected module has one production Kotlin file. An earlier run also covered AlkitabYes2, which has two. For new compiler coverage after this milestone, take an interacting set of Kotlin files from the Alkitab app, reproduce its first failure with the app’s resolved classpath, and use that failure to select the next feature. [Earlier three-module evidence](reports/features/2026-10-30T083348667439Z-androidbible-model-io-yes2.json)

## November 25 results

| Step | Result |
| --- | --- |
| Model and Io builds | Both providers packaged release AARs from all 2 Kotlin and 17 Java sources. Candidate Io consumed candidate Model in debug and release. All 8 original Io tests passed per provider. [Gradle report](reports/correctness/2026-11-25T063155244021Z-androidbible-model-io-gradle.json) |
| Downstream use | The same official-compiled Java and Kotlin callers ran on both library pairs. The checks exercised the unchanged Yes2 chapter implementation, matched 122 Model and 2,135 Io runtime observations per provider, and verified class origins. [Consumer report](reports/correctness/2026-11-25T064030368731Z-model-io-downstream-consumers.json) |
| Survey | Both providers compiled all 33 unchanged files. The candidate binary matched the one used in the module builds. [Survey report](reports/correctness/2026-11-25T062653522540Z-androidbible-survey-dependencies.json) |

The existing compiler passed this milestone. The new consumer check can be rerun against another pair of build receipts using the commands in the linked reports.
