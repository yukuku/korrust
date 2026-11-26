use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};
#[test]
fn java_collection_variance_is_checked_after_overload_selection() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}
fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = std::env::temp_dir().join(format!("korrust-list-variance-{}", std::process::id()));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(["Bags.java", "Box.java", "GenericBox.java"].map(|file| {
            root.join("tests/java_list_variance/java/variance")
                .join(file)
        }))
        .output()
        .unwrap();
    assert!(
        java.status.success(),
        "{}",
        String::from_utf8_lossy(&java.stderr)
    );
    let classpath = [output.clone()];
    compile_outputs_with_classpath(
        include_str!("java_list_variance/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for body in [
        "fun invalid(value: MutableList<String>): Int = Bags.accept(value)",
        "fun invalid(value: MutableList<String>): Int = Bags.choose(value)",
        "fun invalid(holder: Bags, value: MutableList<String>): Int = holder.select(value)",
        "fun invalid(holder: Bags, value: MutableList<String>) { holder.setValue(value) }",
        "fun invalid(value: MutableList<String>) = Box(value)",
        "fun invalid(value: MutableList<String>) = GenericBox<CharSequence>(value)",
        "fun invalid(): Int = Bags.accept(Bags.strings())",
    ] {
        let source = format!(
            "import variance.Bags\nimport variance.Box\nimport variance.GenericBox\n{body}"
        );
        let error = compile_outputs_with_classpath(&source, "Invalid.kt", &classpath)
            .err()
            .unwrap();
        assert_eq!(error.code, "JAVA_TYPE_MISMATCH", "{body}: {error:?}");
    }
    fs::remove_dir_all(output).unwrap();
}
