# Working on Korrust

## Models and ownership

- Use Astra (`gpt-6-astra`) for the main agent. Keep architecture, milestone selection, integration, and correctness review there.
- Delegate implementation to Sol 6.1 (`gpt-6.1-sol`) subagents with **High** reasoning effort explicitly selected. Do not inherit a Low default or silently substitute another model.
- Assign nonoverlapping file ownership. Give subagents the agreed interfaces, supported scope, rejection boundaries, and validation requirements. The main agent reviews their work before accepting it.
- Finish the requested milestone and any explicitly requested delivery steps; do not start another milestone without a request.

## Context and scope

Before compiler work, read the project contract in [README.md](README.md) and the evidence conventions in [reports/README.md](reports/README.md). Before choosing a milestone, inspect the latest successful feature/correctness reports and actual unsupported AndroidBible source files.

- Use AndroidBible as the first compatibility target and Now in Android as a secondary benchmark. Prefer a bounded milestone that compiles an unchanged app file and adds reusable language support.
- Keep the language specification, reference compiler sources, and app snapshots tracked inside this project and unchanged. Their manifests and checksums provide provenance; preserve upstream licenses. Compiler binaries and generated build outputs remain ignored. Do not edit the original AndroidBible repository for compiler work.
- Follow the [reference policy](references/README.md): the specification defines covered language rules, while the pinned official compiler and standard library supply version-specific and JVM evidence. Record discrepancies and suspected oracle bugs; do not silently adopt incorrect behavior or special-case each library API.
- Keep documentation minimal. Current support, limitations, test counts, benchmarks, and progress belong in dated reports and executable tests, not another prose feature checklist.
- Preserve the author's attribution: Randy Sugianto (Yuku), GitHub `yukuku`.

## Correctness and delivery

- Compare against the pinned official Kotlin compiler with matching inputs, classpath, and options. Include independent assertions where practical, especially boundaries, UTF-16, exceptions, nullability, and evaluation order.
- For JVM changes, compile Java callers against official output, run them against both outputs with verification enabled, and compare relevant signatures. Emitted classes or matching Java signatures alone do not establish Kotlin metadata, Android, or full-app compatibility.
- Reject unsupported language and interop explicitly. Preserve upstream directives; distinguish unsupported language from unsupported harness directives and correctness failures.
- Freeze code and fixtures before the final harness run. Record hashes of the final inputs, command logs, and per-case outcomes. Derive counts from cases. Fixes require a fresh report; leave historical reports intact.
- Run appropriate Rust tests, formatting, and Clippy; run report-generator tests when reporting changes. Use the commands in the README and reports README rather than maintaining duplicate commands here.
- Reports must regenerate readable HTML after changes. Keep the existing report watcher working when renaming or moving the project. Verify the generated index and evidence links.
- Treat incremental speedup as unmeasured until controlled benchmarks exist. Scope claims to the tested workload and separate incremental, clean, and full edit-to-running-app timings.

## Communication

The user is not a native English speaker. Before work, minimally correct grammar only when needed: **bold** additions, ~~strikethrough~~ removals, *italic* replacements. Then restate the understood request in one or two sentences and proceed. Ask and wait before touching files only when the intent is linguistically ambiguous; the user will otherwise interrupt if the interpretation is wrong.

Explain consequential choices and report observed evidence, limitations, and remaining work precisely. Keep routine updates concise.

## Editing conventions

- Default to no code comments. Add one only for a non-obvious reason, hidden constraint, or surprising invariant. Avoid descriptions of what code does, section dividers, progress narration, and references to the current task.
- In Kotlin, avoid default parameter values unless required; keep call sites explicit.
- For Gradle, never use `--no-daemon`. Pipe output through `2>&1 | tail -60`.
- Use `master` for this repository; the requested remote is `git@github.com:yukuku/korrust.git`.
- Use `dorooong` instead of `git push`, with the same arguments. Use `ghooost` instead of `gh`.
- Do not add AI-generated attribution footers to commits or pull requests, including `Co-Authored-By: Codex <noreply@anthropic.com>` or `Generated with Codex`.
- Before using the `gws` CLI for Google Workspace, read `~/.Codex/docs/google-workspace-gws.md`.
