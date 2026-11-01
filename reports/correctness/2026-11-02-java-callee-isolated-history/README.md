# Java callee collision repair history

The accepted isolated repair combines applicable nested Java constructors and same-named static methods using the shared overload selector. It adds no language-family count. The combined final compiler is validated separately.

The [source and log archive](callee-text-and-source-history.zip) preserves the complete 12-pass, 12-invalid, two-unsupported isolated proof, exact command logs, frozen sources, oracle probes and V2 integration bases. It excludes generated executables, classes and JARs. Its SHA-256 is `fdd2d6b9d31286c62475d503ed22fd1ce90c31c790f330c38eb74954c1460dea`.

[Handoff](callee-handoff.json), [changed-file hashes](callee-changed-files.json), [patch](callee-final.patch) and [archive index](callee-history-index.json) retain original paths and provenance. Paths within those historical records identify the original work area; the ZIP is the durable source of those files. The earlier failed callee probe remains in [its original report](../2026-11-02-postrepair-java-nested-callee-probe.json).
