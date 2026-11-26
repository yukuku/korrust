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
fn lexical_companion_properties_use_accessors_and_preserve_shadowing() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_companion_properties)
        .unwrap()
        .join()
        .unwrap();
}

fn check_companion_properties() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/companion_properties");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-companion-properties-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("State.kt"))
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
        &[source("State.kt")],
        std::slice::from_ref(&standard),
        "companion_properties",
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
            .arg("companionstatecheck.Calls"));
        assert_eq!(output.stdout, b"companion-properties-ok\n");
    }
    for (name, kind, code) in [
        (
            "InvalidVal.kt",
            DiagnosticKind::InvalidKotlin,
            "VAL_PROPERTY_REASSIGNMENT",
        ),
        (
            "InvalidType.kt",
            DiagnosticKind::InvalidKotlin,
            "TYPE_MISMATCH",
        ),
        (
            "UnsupportedFieldCollision.kt",
            DiagnosticKind::UnsupportedLanguage,
            "COMPANION_BACKING_FIELD_COLLISION_UNSUPPORTED",
        ),
        (
            "UnsupportedPrivateCustom.kt",
            DiagnosticKind::UnsupportedLanguage,
            "PRIVATE_COMPANION_ACCESSOR_SCOPE_UNSUPPORTED",
        ),
        (
            "UnsupportedPrivateCapture.kt",
            DiagnosticKind::UnsupportedLanguage,
            "PRIVATE_COMPANION_CAPTURE_UNSUPPORTED",
        ),
    ] {
        let error = compile_batch_with_classpath(
            &[source(name)],
            std::slice::from_ref(&standard),
            "consumer",
        )
        .err()
        .unwrap();
        assert_eq!(error.kind, kind, "{name}: {error:?}");
        assert_eq!(error.code, code, "{name}: {error:?}");
    }
    std::fs::remove_dir_all(root).unwrap();
}
