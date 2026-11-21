use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn trailing_lambdas_map_to_last_parameter_after_defaults_and_named_arguments() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/trailing_lambdas");
    let temporary = std::env::temp_dir().join(format!("korrust-trailing-{}", std::process::id()));
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
    let cp = [
        dependency,
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let source = std::fs::read_to_string(fixtures.join("Models.kt")).unwrap();
    compile_outputs_with_classpath(&source, "Models.kt", &cp).unwrap();
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
    std::fs::remove_dir_all(temporary).unwrap();
}
