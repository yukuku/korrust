use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn passive_enum_annotations_preserve_field_and_semantic_metadata_boundaries() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_enums)
        .unwrap()
        .join()
        .unwrap();
}
fn check_enums() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/enum_field_annotations");
    let temporary = std::env::temp_dir().join(format!(
        "korrust-enum-field-annotations-{}",
        std::process::id()
    ));
    let dependency = temporary.join("dependency");
    std::fs::create_dir_all(&temporary).unwrap();
    let output = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Dependency.kt"))
        .arg("-Xemit-jvm-type-annotations")
        .arg("-d")
        .arg(&dependency)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let annotations = root.join("references/kotlinc/lib/annotations-13.0.jar");
    let javac = PathBuf::from(std::env::var_os("JAVA_HOME").unwrap()).join("bin/javac");
    let output = Command::new(javac)
        .arg("-cp")
        .arg(&annotations)
        .arg("-d")
        .arg(&dependency)
        .arg(fixtures.join("JavaMode.java"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let paths = [
        dependency,
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
        annotations,
    ];
    let source = std::fs::read_to_string(fixtures.join("Models.kt")).unwrap();
    compile_outputs_with_classpath(&source, "Models.kt", &paths).unwrap();
    for name in [
        "UnsupportedNullable.kt",
        "UnsupportedOrdinary.kt",
        "UnsupportedKotlinSemantic.kt",
        "InvalidOptIn.kt",
    ] {
        let source = std::fs::read_to_string(fixtures.join(name)).unwrap();
        assert!(
            compile_outputs_with_classpath(&source, name, &paths).is_err(),
            "{name}"
        );
    }
    std::fs::remove_dir_all(temporary).unwrap();
}
