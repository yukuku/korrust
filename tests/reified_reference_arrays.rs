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
fn reified_reference_array_allocation_erases_classifier_arguments_only() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_arrays)
        .unwrap()
        .join()
        .unwrap();
}

fn check_arrays() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/reified_reference_arrays");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let annotations = project.join("references/kotlinc/lib/annotations-13.0.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-reified-reference-arrays-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Library.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-cp")
        .arg(&standard)
        .arg("-d")
        .arg(&dependency));
    let dependencies = [dependency.clone(), standard.clone(), annotations.clone()];
    let official = root.join("official");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Uses.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard, &annotations]))
        .arg("-d")
        .arg(&official));
    let candidate = root.join("candidate");
    let source = |name: &str| SourceInput {
        identity: fixtures.join(name).display().to_string(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    };
    for output in
        compile_batch_with_classpath(&[source("Uses.kt")], &dependencies, "reified_arrays")
            .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[
            &official,
            &dependency,
            &standard,
            &annotations,
        ]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[
                &callers,
                provider,
                &dependency,
                &standard,
                &annotations,
            ]))
            .arg("erasedcheck.Calls"));
        assert_eq!(output.stdout, b"reified-reference-arrays-ok\n");
    }
    for name in [
        "InvalidErased.kt",
        "UnsupportedTest.kt",
        "UnsupportedMixed.kt",
    ] {
        let error = compile_batch_with_classpath(&[source(name)], &dependencies, "reified_arrays")
            .err()
            .unwrap();
        let official = Command::new(&kotlin)
            .arg(fixtures.join(name))
            .arg("-cp")
            .arg(classpath(&[&dependency, &standard, &annotations]))
            .arg("-d")
            .arg(root.join(name))
            .output()
            .unwrap();
        if name.starts_with("Unsupported") {
            assert!(
                official.status.success(),
                "{name}: {}",
                String::from_utf8_lossy(&official.stderr)
            );
            assert_eq!(
                error.kind,
                DiagnosticKind::UnsupportedLanguage,
                "{name}: {error:?}"
            );
            assert_eq!(
                error.code, "REIFIED_ARGUMENT_CLASSIFIER_UNSUPPORTED",
                "{name}: {error:?}"
            );
        } else {
            assert!(!official.status.success(), "{name}");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
