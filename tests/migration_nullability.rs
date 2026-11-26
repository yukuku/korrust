use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn warning_only_android_nullability_preserves_platform_calls_and_strict_markers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/migration_nullability/java");
    let output = std::env::temp_dir().join(format!(
        "korrust-migration-nullability-{}",
        std::process::id()
    ));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(
            [
                "Migration.java",
                "androidx/annotation/RecentlyNonNull.java",
                "androidx/annotation/RecentlyNullable.java",
                "androidx/annotation/NonNull.java",
                "androidx/annotation/Nullable.java",
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
        include_str!("migration_nullability/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "fun invalid(): String = Migration.strictNullable(true)",
        "fun invalid(): String? = Migration.strictAccept(null)",
        "fun invalid(value: Migration): String = value.strictNullableField",
        "fun invalid(value: Migration) { value.strictNonNullField = null }",
    ] {
        assert!(compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err());
    }
    compile_outputs_with_classpath(
        "fun accepted(value: Migration, items: List<String>) { value.genericField = items }",
        "Generic.kt",
        &classpath,
    )
    .unwrap();
    let error = compile_outputs_with_classpath(
        "fun invalid(): List<String>? = Migration.genericStaticField",
        "Generic.kt",
        &classpath,
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "GENERIC_JAVA_STATIC_FIELD_UNSUPPORTED");
    fs::remove_dir_all(output).unwrap();
}
