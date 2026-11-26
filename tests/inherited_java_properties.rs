use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn kotlin_subclasses_keep_java_origin_property_contracts() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_kotlin_subclasses)
        .unwrap()
        .join()
        .unwrap();
}

fn check_kotlin_subclasses() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/inherited_java_properties/java");
    let output = std::env::temp_dir().join(format!(
        "korrust-inherited-java-properties-{}",
        std::process::id()
    ));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(
            [
                "beans/Base.java",
                "beans/GenericBase.java",
                "beans/Title.java",
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
        include_str!("inherited_java_properties/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "class Pure { fun getOnly(): String = \"pure\" }\nfun invalid(value: Pure): String = value.only",
        "class Child : beans.Base()\nfun invalid(value: Child) { value.name = null }",
        "class Child : beans.Base()\nfun invalid(value: Child): String = value.optional",
        "class Child : beans.Base()\nfun invalid(value: Child) { value.readOnly = \"new\" }",
    ] {
        assert!(compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err());
    }
    compile_outputs_with_classpath(
        "interface Child : beans.Title\nfun title(value: Child): String = value.title",
        "Interface.kt",
        &classpath,
    )
    .unwrap();
    fs::remove_dir_all(output).unwrap();
}
