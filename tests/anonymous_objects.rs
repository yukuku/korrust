use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::PathBuf;

fn compile(source: &str) -> Result<Vec<korrust::SourceOutput>, korrust::Diagnostic> {
    compile_batch_with_classpath(
        &[SourceInput {
            identity: "/original/Objects.kt".into(),
            file_name: "Objects.kt".into(),
            source: source.into(),
        }],
        &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar")],
        "anonymous_test",
    )
}

#[test]
fn anonymous_objects_emit_lexical_classes_and_shared_cells() {
    let outputs = compile(include_str!("anonymous_objects/Objects.kt")).unwrap();
    let lexical = outputs
        .iter()
        .filter(|output| output.output.relative_path.contains("$object$"))
        .collect::<Vec<_>>();
    assert!(lexical.len() >= 10);
    assert!(
        lexical
            .iter()
            .all(|output| output.source_identity == "/original/Objects.kt")
    );
    assert!(lexical.iter().any(|output| {
        output
            .output
            .bytes
            .windows(b"kotlin/jvm/internal/Ref$IntRef".len())
            .any(|value| value == b"kotlin/jvm/internal/Ref$IntRef")
    }));
}

#[test]
fn unsupported_anonymous_shapes_reject_explicitly() {
    for source in [
        include_str!("anonymous_objects/boundaries/ClassSupertype.kt"),
        include_str!("anonymous_objects/boundaries/Multiple.kt"),
        include_str!("anonymous_objects/boundaries/Property.kt"),
        include_str!("anonymous_objects/boundaries/Generic.kt"),
        include_str!("anonymous_objects/boundaries/ExtraMember.kt"),
        include_str!("anonymous_objects/boundaries/Marker.kt"),
    ] {
        let error = compile(source)
            .err()
            .expect("Unsupported anonymous shape must reject");
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{}",
            error.message
        );
    }
}

#[test]
fn invalid_anonymous_overrides_reject() {
    for source in [
        include_str!("anonymous_objects/boundaries/Missing.kt"),
        include_str!("anonymous_objects/boundaries/Duplicate.kt"),
        include_str!("anonymous_objects/boundaries/WrongParameter.kt"),
        include_str!("anonymous_objects/boundaries/WrongResult.kt"),
        include_str!("anonymous_objects/boundaries/NullResult.kt"),
    ] {
        let error = compile(source)
            .err()
            .expect("Invalid anonymous override must reject");
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{}",
            error.message
        );
    }
}

#[test]
fn anonymous_inline_returns_and_inferred_cells_execute_with_verification() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = project.join("target").join(format!(
        "anonymous-inline-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let candidate = root.join("candidate");
    let official = root.join("official");
    let caller = root.join("caller");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    std::fs::create_dir_all(&root).unwrap();
    let input = SourceInput {
        identity: "/original/InlineReturns.kt".into(),
        file_name: "InlineReturns.kt".into(),
        source: include_str!("anonymous_objects/InlineReturns.kt").into(),
    };
    for output in compile_batch_with_classpath(
        &[input],
        std::slice::from_ref(&standard),
        "anonymous_inline",
    )
    .unwrap()
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    fn success(command: &mut std::process::Command) -> Vec<u8> {
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }
    success(
        std::process::Command::new(project.join("references/kotlinc/bin/kotlinc"))
            .arg(project.join("tests/anonymous_objects/InlineReturns.kt"))
            .args([
                "-language-version",
                "2.3",
                "-api-version",
                "2.3",
                "-jvm-target",
                "1.8",
            ])
            .arg("-d")
            .arg(&official),
    );
    success(
        std::process::Command::new("javac")
            .arg("-cp")
            .arg(std::env::join_paths([&official, &standard]).unwrap())
            .arg("-d")
            .arg(&caller)
            .arg(project.join("tests/anonymous_objects/InlineReturnCheck.java")),
    );
    for provider in [&official, &candidate] {
        let output = success(
            std::process::Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(std::env::join_paths([provider, &caller, &standard]).unwrap())
                .arg("InlineReturnCheck"),
        );
        assert_eq!(output, b"OK\n");
    }
}
