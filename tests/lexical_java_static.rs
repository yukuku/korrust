use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn lexical_java_static_fields_preserve_scope_and_receiver_rules() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}

fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output =
        std::env::temp_dir().join(format!("korrust-nullable-string-{}", std::process::id()));
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(["Base.java", "Other.java", "Marks.java"].map(|name| {
            root.join("tests/lexical_java_static/java/fields")
                .join(name)
        }))
        .output()
        .unwrap();
    assert!(
        java.status.success(),
        "{}",
        String::from_utf8_lossy(&java.stderr)
    );
    let classpath = [
        output.clone(),
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let inputs = ["Effects", "Imported", "Interface", "Package", "Value"].map(|name| {
        let file_name = format!("{name}.kt");
        let source = root.join("tests/lexical_java_static").join(&file_name);
        SourceInput {
            identity: source.to_string_lossy().into(),
            file_name,
            source: fs::read_to_string(&source).unwrap(),
        }
    });
    compile_batch_with_classpath(&inputs, &classpath, "lexical_java_static").unwrap();
    for source in [
        "object Invalid { fun value(): Invalid = INSTANCE }",
        "import fields.Base\nfun invalid(value: Base): Int = value.CODE",
        "import fields.Marks\nclass Invalid : Marks { fun invalid(): Int = MARK }",
        "import fields.Base\nclass Invalid : Base() { fun invalid(): Int = SECRET }",
        "import fields.Base\nclass Invalid : Base() { fun invalid(): String = CODE }",
    ] {
        assert!(
            compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err(),
            "{source}"
        );
    }
    fs::remove_dir_all(output).unwrap();
}
