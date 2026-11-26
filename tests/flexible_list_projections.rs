use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn readonly_list_projections_keep_java_variance_and_write_restrictions() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_lists)
        .unwrap()
        .join()
        .unwrap();
}
fn check_lists() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/flexible_list_projections/java");
    let output =
        std::env::temp_dir().join(format!("korrust-flexible-lists-{}", std::process::id()));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(
            [
                "Lists.java",
                "android/annotation/NonNull.java",
                "android/annotation/Nullable.java",
                "org/jetbrains/annotations/ReadOnly.java",
                "org/jetbrains/annotations/Mutable.java",
            ]
            .map(|path| fixtures.join(path)),
        )
        .output()
        .unwrap();
    assert!(
        java.status.success(),
        "{}",
        String::from_utf8_lossy(&java.stderr)
    );
    let classpath = [output.clone()];
    compile_outputs_with_classpath(
        include_str!("flexible_list_projections/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "fun invalid(value: MutableList<out String>): Int = Lists.accept(value)",
        "fun invalid(value: List<out CharSequence>): Int = Lists.mutableArg(value)",
        "fun invalid(value: List<out String>) { value.add(\"x\") }",
        "fun invalid(value: List<out String>) { value[0] = \"x\" }",
        "fun invalid(value: MutableList<out String>) { value.add(\"x\") }",
    ] {
        assert!(
            compile_outputs_with_classpath(
                &format!("import Lists\n{source}"),
                "Invalid.kt",
                &classpath
            )
            .is_err(),
            "{source}"
        );
    }
    fs::remove_dir_all(output).unwrap();
}
