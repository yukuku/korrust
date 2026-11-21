use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn classifier_import_priority_preserves_explicit_package_wildcard_and_default_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/import_priority");
    let temporary =
        std::env::temp_dir().join(format!("korrust-import-priority-{}", std::process::id()));
    let dependency = temporary.join("dependency");
    std::fs::create_dir_all(&temporary).unwrap();
    let output = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .args([
            fixtures.join("One.kt"),
            fixtures.join("Two.kt"),
            fixtures.join("Local.kt"),
        ])
        .args(["-jvm-target", "1.8", "-d"])
        .arg(&dependency)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cp = [
        dependency,
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let sources = [
        "Wildcard.kt",
        "Explicit.kt",
        "SamePackage.kt",
        "Alias.kt",
        "Duplicate.kt",
    ]
    .map(|name| SourceInput {
        identity: name.into(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    });
    compile_batch_with_classpath(&sources, &cp, "priority").unwrap();
    for name in ["InvalidAmbiguous.kt", "InvalidAnnotation.kt"] {
        let source = std::fs::read_to_string(fixtures.join(name)).unwrap();
        let error = compile_outputs_with_classpath(&source, name, &cp)
            .err()
            .unwrap();
        assert_eq!(error.kind, korrust::DiagnosticKind::InvalidKotlin);
        assert_eq!(error.code, "INVALID_ANNOTATION_VALUE");
        assert!(
            error
                .message
                .contains("Ambiguous wildcard annotation classifier")
        );
    }
    std::fs::remove_dir_all(temporary).unwrap();
}
