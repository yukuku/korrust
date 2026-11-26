use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn nested_class_leaf_type_annotations_preserve_nullable_contracts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/inner_type_nullability/java");
    let output = std::env::temp_dir().join(format!(
        "korrust-inner-type-nullability-{}",
        std::process::id()
    ));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(
            [
                "nestedapi/Outer.java",
                "nestedapi/Provider.java",
                "org/jspecify/annotations/NonNull.java",
                "org/jspecify/annotations/Nullable.java",
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
        include_str!("inner_type_nullability/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for body in [
        "fun invalid(): Outer.Leaf = Provider.nullable(true)",
        "fun invalid(): Int = Provider.strict(null)",
        "fun invalid(value: Provider): Outer.Leaf = value.field",
        "fun invalid(): Outer.Middle.Leaf = Provider.nested(true)",
    ] {
        let source = format!("import nestedapi.Outer\nimport nestedapi.Provider\n{body}");
        assert!(compile_outputs_with_classpath(&source, "Invalid.kt", &classpath).is_err());
    }
    for body in [
        "fun unsupported(): Array<Outer.Leaf?> = Provider.arrayElement()",
        "fun unsupported(): List<Outer.Leaf?>? = Provider.genericElement()",
    ] {
        let source = format!("import nestedapi.Outer\nimport nestedapi.Provider\n{body}");
        let error = compile_outputs_with_classpath(&source, "Unsupported.kt", &classpath)
            .err()
            .unwrap();
        assert_eq!(error.code, "LIBRARY_DECLARATION_UNSUPPORTED");
    }
    fs::remove_dir_all(output).unwrap();
}
