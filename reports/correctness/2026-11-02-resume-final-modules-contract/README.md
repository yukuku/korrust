# Final three-module Gradle and JVM contract proof

The frozen integrated compiler `362e12fd88ea493a1132d85a618a656f916242ef7ba4f48d002ea64fb054105b` passes all 127 contract cases. This is a fresh real Gradle build of unchanged AlkitabModel, AlkitabIo and AlkitabYes2: four production Kotlin files and 39 production Java files. Selected production Kotlin uses Korrust without fallback; nonselected dependencies and original test Kotlin use normal compilers. All three release AARs and the original 27 unit tests passed.

The contract checks source inventories, exact staged candidate class bytes, relevant ABI, independent fixed Java callers with `-Xverify:all`, official Kotlin consumers and negative visibility/nullability cases. The same original 27 official-compiled Io/Yes2 test bytecodes run against both module sets under verification. Model has no original tests.

Standalone actual AGP 9.2.1 embedded D8 9.2.14 accepted both complete three-module class-JAR sets with min API 26 and no warnings/errors. This standalone D8 evidence is separate from Gradle AAR packaging. No Android-device or full-app runtime is claimed. Configured Gradle target21 and Korrust classfile49/52 emission are disclosed in the JSON.

Official Gradle logs report 55 seconds and candidate Gradle logs report 14 minutes 23 seconds. These are raw run observations with different cache/contention conditions, not a controlled timing comparison or incremental speed claim.

The [launch record](launch.json) includes the required JBR and Python import path. The [publication patch](publication-adapter.patch) only changes report/work locations, keeps generated AAR/DEX copies under target, redirects generated feature JSON under target, and disables the automatic HTML hook. Contract assertions and commands are unchanged. No generated binaries are published in this evidence directory; their hashes and target paths remain in the report.

The separate [six-source assessment](../2026-11-02-resume-final-modules-six-source-assessment.json) uses precompiled sibling/generated dependencies and establishes per-file probe outcomes only. The final text/source snapshot and all six preserved source inputs are archived under `2026-11-02-resume-final-modules-inputs`.
