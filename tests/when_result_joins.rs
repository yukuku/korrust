use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn when_reference_results_join_nullable_subtypes_and_abrupt_branches() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_when_joins)
        .unwrap()
        .join()
        .unwrap();
}

fn check_when_joins() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/when_result_joins");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root =
        std::env::temp_dir().join(format!("korrust-when-result-joins-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Joins.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-d")
        .arg(&official));
    let source = |name: &str| SourceInput {
        identity: fixtures.join(name).display().to_string(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    };
    for output in compile_batch_with_classpath(
        &[source("Joins.kt")],
        std::slice::from_ref(&standard),
        "when_result_joins",
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, provider, &standard]))
            .arg("whenjoinscheck.Calls"));
        assert_eq!(output.stdout, b"when-joins-ok\n");
    }
    for name in ["UnsupportedUnrelated.kt", "UnsupportedNumeric.kt"] {
        let error = compile_batch_with_classpath(
            &[source(name)],
            std::slice::from_ref(&standard),
            "consumer",
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{name}: {error:?}"
        );
        assert_eq!(
            error.code, "WHEN_RESULT_JOIN_UNSUPPORTED",
            "{name}: {error:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
