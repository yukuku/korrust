use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};
#[test]
fn nullable_reference_assignments_preserve_only_stable_rhs_facts() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}
fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = std::env::temp_dir().join(format!(
        "korrust-assignment-smartcasts-{}",
        std::process::id()
    ));
    fs::create_dir_all(&output).unwrap();
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args(["Node.java", "Leaf.java", "Lookup.java"].map(|file| {
            root.join("tests/assignment_smartcasts/java/assign")
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
        include_str!("assignment_smartcasts/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
  "fun invalid(): Int { var value: String? = null; value = \"a\"; value = null; return value.length }",
  "fun invalid(input: String?): Int { var value: String? = null; value = \"a\"; value = input; return value.length }",
  "fun invalid(flag: Boolean): Int { var value: String? = null; value = \"a\"; if (flag) value = null; return value.length }",
  "fun invalid(flag: Boolean): Int { var value: String? = null; value = \"a\"; while (flag) { value.length; value = null }; return 0 }",
  "fun invalid(): Int { var value: String? = null; value = \"a\"; val change: () -> Unit = { value = null }; return value.length }",
  "fun invalid(): Int { var value: String? = null; val change: () -> Unit = { value = null }; value = \"a\"; return value.length }",
  "fun invalid(): Int { var value: String? = null; value = \"a\"; val change: () -> Unit = { value = null }; change(); return value.length }",
  "import assign.Lookup\nimport assign.Leaf\nfun invalid(lookup: Lookup): Int { var value: Leaf? = null; value = lookup.leaf(1); if (value == null) return value.inputType; return 0 }",
 ] { assert!(compile_outputs_with_classpath(source,"Invalid.kt",&classpath).is_err(),"{source}"); }
    fs::remove_dir_all(output).unwrap();
}
