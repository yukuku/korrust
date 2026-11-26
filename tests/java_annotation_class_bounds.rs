use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::PathBuf;

#[test]
fn binary_annotation_class_bounds_check_arguments_and_property_reads() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/java_annotation_class_bounds");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let output = std::env::temp_dir().join(format!(
        "korrust-class-bounds-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&output).unwrap();
    let javac = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .map(|path| path.join("bin/javac"))
        .unwrap_or_else(|| "javac".into());
    let status = std::process::Command::new(javac)
        .args(["--release", "8", "-d"])
        .arg(&output)
        .args(
            ["Bounds.java", "Upper.java", "Nested.java", "Complex.java"]
                .map(|name| fixtures.join(name)),
        )
        .output()
        .unwrap();
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let classpath = [
        output.clone(),
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let compile = |name: &str| {
        compile_batch_with_classpath(
            &[SourceInput {
                identity: name.into(),
                file_name: name.into(),
                source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
            }],
            &classpath,
            "annotationbounds",
        )
    };
    compile("Accepted.kt").unwrap();
    for name in [
        "WrongUpper.kt",
        "WrongExact.kt",
        "WrongLower.kt",
        "WrongArray.kt",
        "WrongArrayMarker.kt",
        "WrongArrayScalar.kt",
        "WrongArrayPositional.kt",
        "WrongNested.kt",
        "WrongReadUpper.kt",
        "WrongReadExact.kt",
        "WrongReadLower.kt",
        "WrongReadArray.kt",
    ] {
        let error = compile(name).err().unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {error:?}"
        );
        if !name.starts_with("WrongRead") {
            assert_eq!(error.code, "INVALID_ANNOTATION_VALUE", "{name}: {error:?}");
        }
    }
    let error = compile("Unsupported.kt").err().unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage, "{error:?}");
    assert_eq!(error.code, "ANNOTATION_DEFINITION_UNSUPPORTED");
    std::fs::remove_dir_all(output).unwrap();
}
