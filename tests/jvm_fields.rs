use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn instance_jvm_fields_preserve_visibility_metadata_initialization_and_annotation_identity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/jvm_fields");
    let temporary = std::env::temp_dir().join(format!("korrust-jvm-fields-{}", std::process::id()));
    let dependency = temporary.join("dependency");
    std::fs::create_dir_all(&temporary).unwrap();
    let output = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Dependency.kt"))
        .args(["-jvm-target", "1.8", "-d"])
        .arg(&dependency)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut cp = vec![
        dependency,
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let sources = ["Models.kt", "Shadow.kt", "LocalShadow.kt"].map(|name| SourceInput {
        identity: name.into(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    });
    let outputs = compile_batch_with_classpath(&sources, &cp, "fields").unwrap();
    let provider = temporary.join("provider");
    for output in outputs {
        let path = provider.join(&output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    cp.insert(0, provider);
    let source = std::fs::read_to_string(fixtures.join("LibraryReader.kt")).unwrap();
    compile_outputs_with_classpath(&source, "LibraryReader.kt", &cp).unwrap();
    for entry in std::fs::read_dir(&fixtures).unwrap() {
        let path = entry.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Invalid")
        {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(
                compile_outputs_with_classpath(
                    &source,
                    path.file_name().unwrap().to_str().unwrap(),
                    &cp
                )
                .is_err(),
                "{}",
                path.display()
            );
        }
    }
    let source = std::fs::read_to_string(fixtures.join("OracleParameter.kt")).unwrap();
    assert!(compile_outputs_with_classpath(&source, "OracleParameter.kt", &cp).is_err());
    std::fs::remove_dir_all(temporary).unwrap();
}
