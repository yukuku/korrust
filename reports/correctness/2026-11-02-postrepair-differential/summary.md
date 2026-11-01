The four requested differential suites completed against frozen compiler SHA-256 `58ee34b1866ca7a9a5cd0ce838730856bef745907e159f93a2a4a8a00249b784`. The effective 944-entry freeze remained unchanged: 943 root source/tool/fixture files plus the relocated frozen binary. See index.json for exact reproduction commands and each rebased report; original-report.json retains every original command argument and report field.

| Suite | Positive | Invalid rejected | Unsupported language | Unsupported harness |
| --- | ---: | ---: | ---: | ---: |
| Full selected original matrix | 19 | 0 | 12 | 5 |
| Full value-class suite | 31 | 11 | 32 | 2 |
| Diagnostic boundaries | 4 | 3 | 6 | 0 |
| Generic-value extension boundaries | 7 | 3 | 1 | 0 |

There were no correctness, dependency, runtime or timeout failures in these four suites. Unsupported outcomes remain separate from positive results. Counts describe the selected cases and do not add feature families or establish complete Kotlin, Android or performance compatibility. A separately discovered Java callee-collision failure is outside these suites and remains preserved by the parent task.

Publication contains source, text, command logs, original JSON and source archives. Compiler binaries, classfiles, JARs, Kotlin module outputs and other generated binary artifacts remain under target and are excluded here. The active root release binary differed during setup; all four commands explicitly used the separately verified frozen candidate, as recorded in identity.json.
