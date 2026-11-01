# Final AndroidBible library modules and six-source assessment

All results in this summary use frozen integrated compiler SHA-256 `362e12fd88ea493a1132d85a618a656f916242ef7ba4f48d002ea64fb054105b`, unless a row explicitly refers to official Kotlin. Source and tool hashes are preserved in the [final input manifest](2026-11-02-resume-final-modules-inputs/index.json) and its text/source archive. Later compiler repairs require separate evidence.

| Workload | Observed result | Evidence |
| --- | --- | --- |
| Fresh official Gradle modules | Three release AARs; 4 unchanged production Kotlin files and 39 Java files; all 27 original tests passed | [Official build](2026-11-02-resume-final-modules-official.json) |
| Fresh candidate Gradle and module contract | Real three-module build passed without selected production Kotlin fallback; all 127 contract checks passed | [Contract](2026-11-02-resume-final-modules-contract.json), [scope and launch notes](2026-11-02-resume-final-modules-contract/README.md) |
| Initial six-source probe | Official accepted all six; candidate classified four unsupported and timed out on Gid/Rpp at 120 seconds | [Preserved original assessment](2026-11-02-resume-final-modules-six-source-assessment.json) |
| Gid/Rpp single-job retry | Both candidate and official emitted one class for each unchanged source with a 600-second candidate limit | [Fresh retry](2026-11-02-resume-final-modules-six-source-assessment-gid-rpp-retry.json), [exact input equivalence](2026-11-02-resume-final-modules-six-source-assessment-gid-rpp-retry/input-equivalence.json) |

The module contract covers class inventories and exact staged bytes, relevant JVM ABI, independent Java callers under verification, official Kotlin consumers and negative visibility/nullability cases. The same 27 original official-compiled test bytecodes run against both module sets with `-Xverify:all`. Actual AGP 9.2.1 embedded D8 9.2.14 accepted both complete module sets without warnings/errors at min API 26. Gradle AAR packaging and standalone D8 are separate checks. Model has no original unit tests; Io has 8 and Yes2 has 19.

The six unchanged source probes remain separate from that module proof:

| Source | Latest observed outcome with this compiler | Scope |
| --- | --- | --- |
| Gid.kt | Emitted on the separately authorized retry; original timeout preserved | Emission only |
| Rpp.kt | Emitted on the separately authorized retry; original timeout preserved | Emission only |
| DevotionActivity.kt | Explicit `ANONYMOUS_OBJECT_EXPRESSION_UNSUPPORTED` at 57:32 | Official accepted; no candidate emission |
| BackForwardListController.kt | `UNCLASSIFIED` diagnostic at 49:5: unsupported class visibility | Legacy unsupported heuristic; not an explicit diagnostic code |
| AudioHttp.kt | `UNCLASSIFIED` diagnostic at 22:5: only supported interface declarations accepted | Legacy unsupported heuristic; not an explicit diagnostic code |
| UnsignedBinarySearchKtTest.kt | Explicit `FILE_ANNOTATION_UNSUPPORTED` at 1:1 | First blocker precedes unsigned-array semantics |

The five app sources use the original captured 164-entry classpath, all 11 plugins and both Compose options. Existing official sibling/generated outputs contain 1007 Kotlin classes and 293 Java classes and are real precompiled dependencies for both compilers. The test uses the actual fresh Gradle unit-test classpath and captured production friend JAR. All 189 unique selected input paths matched recorded hashes, including after the candidate Gradle build. The raw capture's documented optional-empty KSP omission and earlier setup limitations remain in provenance. No inputs or options were stripped. Candidate option gaps are recorded by the assessment.

The retry changed only scheduling and timeout: jobs 2 to 1 and candidate timeout 120 to 600 seconds, after Gradle and contract work finished. Source hashes, model, dependency bytes, options/plugins/friends, compiler runtime and actual compiler argument arrays match the initial run except output directories. No runtime, ABI or Kotlin-consumer check is inferred from these per-file emissions. These precompiled dependencies do not establish a fresh app source batch, full-app build or Android-device behavior.

Gradle logs report official 55 seconds and candidate 14 minutes 23 seconds. These are raw observations with different cache/contention conditions, not controlled timing or incremental speed evidence. Generated binaries remain under target; published evidence contains text, source and hashes only. No additional language-family count is assigned by this validation.
