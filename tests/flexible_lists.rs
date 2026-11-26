use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn java_list_mutability_is_independent_of_nullability() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_lists)
        .unwrap()
        .join()
        .unwrap();
}
fn check_lists() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/flexible_lists/java");
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
        include_str!("flexible_lists/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "fun invalid() { Lists.platformReadOnly().add(\"x\") }",
        "fun invalid() { Lists.accept(Lists.strings()) }",
        "fun invalid() { Lists.accept(null) }",
        "fun invalid(value: List<String>) { value.add(\"x\") }",
        "fun invalid() { Lists.readOnly().add(\"x\") }",
        "fun invalid() { val value: List<CharSequence> = Lists.nonNull(); value.add(\"x\") }",
        "fun invalid() { Lists.nullable(false).add(\"x\") }",
        "fun invalid(value: List<CharSequence>) { Lists.mutableArg(value) }",
        "fun invalid(value: MutableList<String>) { Lists.mutableArg(value) }",
        "fun invalid(value: MutableList<String>) { Lists.accept(value) }",
        "fun invalid() { val value: MutableList<CharSequence> = Lists.nonNull(); value.add(null) }",
        "fun invalid() { Lists.projected().add(\"x\") }",
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
