# Final six-source resolved-input assessment

This completed all six requested probes with frozen compiler `362e12fd88ea493a1132d85a618a656f916242ef7ba4f48d002ea64fb054105b`. Official Kotlin accepted all six unchanged sources. Candidate outcomes are four unsupported-language classifications and two 120-second timeouts. No candidate source emitted classes in this run.

DevotionActivity reports the explicit anonymous-object-expression boundary at 57:32. The unchanged unsigned test reports the explicit file-annotation boundary at 1:1, before reaching unsigned-array semantics. AudioHttp at 22:5 and BackForwardListController at 49:5 retain `UNCLASSIFIED` diagnostics; their unsupported outcomes come from the preserved legacy heuristic. These are not silently promoted to explicit unsupported diagnostic codes.

Gid and Rpp timed out without candidate diagnostics or emitted classes. Timeouts do not establish language failures. This historical run remains unchanged; the separately authorized single-job 600-second retry is recorded in a fresh report.

App probes use the original captured 164-entry classpath, all 11 plugins and both Compose options, including real precompiled sibling/generated outputs with 1007 Kotlin and 293 Java classes. The test uses the fresh official Gradle unit-test task's 12-entry classpath and captured production friend JAR. Both compilers receive identical dependency bytes. No inputs or options were stripped.

This is per-file emission assessment. Behavior, ABI and Kotlin consumers are not run here; precompiled dependencies do not establish a fresh source batch, full app build, or Android runtime compatibility. The independent three-module Gradle and contract proof is reported separately.
