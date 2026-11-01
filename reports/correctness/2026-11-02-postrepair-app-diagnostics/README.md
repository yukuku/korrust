# Post-repair app diagnostic checks

Frozen compiler `58ee34b1866ca7a9a5cd0ce838730856bef745907e159f93a2a4a8a00249b784` correctly returns two explicit unsupported-language diagnostics. Official Kotlin accepts both unchanged inputs. Neither candidate emits a class.

| Source | Explicit code | Location |
| --- | --- | --- |
| AudioHttp.kt | `SUSPEND_MEMBER_DECLARATION_UNSUPPORTED` | 22:5 |
| BackForwardListController.kt | `CLASS_VISIBILITY_UNSUPPORTED` | 49:13 |

The [input-equivalence record](input-equivalence.json) verifies identical source hashes, captured model, classpath/plugin/runtime/friend inputs and recorded options against the preserved earlier six-source assessment. Actual compiler argument arrays differ only in the intentionally changed candidate compiler path and output directories. Official class hashes match the earlier run. App probes preserve all 164 classpath entries, 11 plugins and both Compose options, including real precompiled sibling/generated outputs. No inputs were stripped.

Earlier AudioHttp and BackForward results remain `UNCLASSIFIED` diagnostics interpreted through the legacy heuristic. This report proves that the new compiler produces structured explicit codes; the old reports were not rewritten. No source-emission improvement or language-family count is claimed. DevotionActivity, the unsigned test, Gid and Rpp were not rerun.

The 127-case module contract and Gid/Rpp emission evidence remain identified with the earlier `362e12fd88ea493a1132d85a618a656f916242ef7ba4f48d002ea64fb054105b` compiler. Those workloads were not repeated with this repair compiler. This two-file probe does not establish their behavior on the new compiler, a fresh source batch, full app build or Android runtime.

The publication patch changes only relocated ROOT/work paths and suppresses the automatic HTML hook. Probe/launcher/classification logic matches the frozen original. The text/source archive preserves the 944-entry freeze manifest, source/tests/tools, both original app sources and AndroidBible license; binaries are omitted. No HTML generation was run.
