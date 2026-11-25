# AndroidBible module roadmap

The November 21 survey established compilation for 33 individual Kotlin files. The next milestone exercises the debug library build, where Gradle must combine Korrust output with Java classes and package the result for a consumer. [Survey evidence](reports/features/2026-11-21T195150057694Z-androidbible-survey-completed.json)

| Step | Work | Acceptance |
| --- | --- | --- |
| 1. Reproduce AlkitabModel | Build the unchanged module in independent official and Korrust project copies. | Account for every production Kotlin and Java source, verify the Kotlin provider, run Gradle Java compilation and inspect the debug AAR. |
| 2. Build AlkitabIo against Model | Select both modules in the same candidate build so Io consumes the rebuilt Model dependency. Keep Bintex as the project's declared dependency. | Apply the same source and packaging checks to Io. Run its original unit tests and compare outcomes with the official build. |
| 3. Verify downstream use | Compile a caller against official output and run that same caller against both pairs of libraries. | Verify JVM linkage and independently assert observable behavior from both modules. Record which consumer and paths ran. |
| 4. Check the survey and publish evidence | Run the fixed 33-file survey on the same candidate binary. Record source provenance, commands, compiler hash and per-case outcomes. | All 33 cases pass under the established classpaths. Link the module, consumer and regression evidence to this roadmap. |

If a build exposes a compiler defect, isolate the failing Kotlin construct, implement its general semantics, compare with the pinned compiler, and commit and push that feature separately. Rerun the affected build and the survey on the final binary.

Each selected module has one production Kotlin file. A module with several interacting Kotlin files is a subsequent milestone. The acceptance criteria above cover debug library builds and the exercised consumers.
