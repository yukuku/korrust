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
fn passive_field_annotations_preserve_reference_types_and_semantic_boundaries() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_fields)
        .unwrap()
        .join()
        .unwrap();
}

fn check_fields() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/java_field_annotations");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let annotations = project.join("references/kotlinc/lib/annotations-13.0.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-java-field-annotations-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new("javac")
        .arg("-cp")
        .arg(&annotations)
        .arg("-d")
        .arg(&dependency)
        .args(
            [
                "Audit.java",
                "Fields.java",
                "Semantic.java",
                "TypeQualifierNickname.java",
            ]
            .map(|name| fixtures.join(name)),
        ));
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
        compile_batch_with_classpath(&[source("Uses.kt")], &dependencies, "field_annotations")
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
            .arg("fieldcheck.Calls"));
        assert_eq!(output.stdout, b"java-field-annotations-ok\n");
    }
    for name in [
        "InvalidNull.kt",
        "InvalidNullable.kt",
        "UnsupportedSemantic.kt",
        "UnsupportedPrimitive.kt",
    ] {
        let error =
            compile_batch_with_classpath(&[source(name)], &dependencies, "field_annotations")
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
            assert_eq!(error.code, "JAVA_FIELD_UNSUPPORTED", "{name}: {error:?}");
        } else {
            assert!(!official.status.success(), "{name}");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
