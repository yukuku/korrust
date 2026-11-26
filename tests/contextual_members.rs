use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};

#[test]
fn expected_result_types_reach_ordinary_member_inference() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_expected_members)
        .unwrap()
        .join()
        .unwrap();
}
fn check_expected_members() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/contextual_members/java/contextcalls");
    let output =
        std::env::temp_dir().join(format!("korrust-contextual-members-{}", std::process::id()));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(
            ["Item.java", "Leaf.java", "Other.java", "Service.java"]
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
        include_str!("contextual_members/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "fun invalid(value: contextcalls.Service): String = value.fixedLeaf()",
        "class Holder : contextcalls.Service() { fun invalid(): String = lookup<String>(1) }",
        "fun invalid(value: contextcalls.Service): contextcalls.Leaf = value.lookup(true)",
        "fun invalid(value: contextcalls.Service): String = value.lookup<String>(1)",
    ] {
        assert!(compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err());
    }
    for source in [
        "fun unsupported(value: contextcalls.Service): String = value.lookup(1)",
        "class Holder : contextcalls.Service() { fun unsupported(): String = lookup(1) }",
    ] {
        assert!(compile_outputs_with_classpath(source, "Unsupported.kt", &classpath).is_err());
    }
    fs::remove_dir_all(output).unwrap();
}
