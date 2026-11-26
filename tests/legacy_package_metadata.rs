use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn legacy_compiled_package_declarations_resolve_by_kotlin_identity() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_legacy_compiled_package_declarations)
        .unwrap()
        .join()
        .unwrap();
}

fn check_legacy_compiled_package_declarations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/legacy_package_metadata");
    let temporary =
        std::env::temp_dir().join(format!("korrust-legacy-package-{}", std::process::id()));
    let dependency = temporary.join("dependency");
    std::fs::create_dir_all(&temporary).unwrap();
    let output = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .args([
            fixtures.join("Library.kt"),
            fixtures.join("First.kt"),
            fixtures.join("Second.kt"),
        ])
        .args(["-Xmetadata-version=1.7.0", "-jvm-target", "1.8", "-d"])
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
    let source = std::fs::read_to_string(fixtures.join("Consumer.kt")).unwrap();
    compile_outputs_with_classpath(&source, "Consumer.kt", &cp).unwrap();
    std::fs::remove_dir_all(temporary).unwrap();
}
