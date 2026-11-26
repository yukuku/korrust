use korrust::compile_outputs_with_classpath;
use std::{fs, path::PathBuf, process::Command};
#[test]
fn nullable_sam_literals_preserve_callback_contracts() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}
fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = std::env::temp_dir().join(format!("korrust-nullable-sam-{}", std::process::id()));
    fs::create_dir_all(&output).unwrap();
    let library = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(root.join("tests/nullable_sam/Library.kt"))
        .args(["-jvm-target", "1.8", "-d"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        library.status.success(),
        "{}",
        String::from_utf8_lossy(&library.stderr)
    );
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .args([
            root.join("tests/nullable_sam/java/nullable/Callbacks.java"),
            root.join("tests/nullable_sam/java/org/jetbrains/annotations/Nullable.java"),
        ])
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
    compile_outputs_with_classpath(
        include_str!("nullable_sam/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "import nullable.Callbacks\nfun invalid(): Int = Callbacks.bad { 1 }",
        "import nullable.Callbacks\nfun invalid(): Int = Callbacks.call({ a, b -> a + b }, 4)",
        "import nullable.Callbacks\nfun invalid(): Int = Callbacks.call({ value -> \"wrong\" }, 4)",
        "import nullable.Callbacks\nfun invalid() { Callbacks.event({ value -> value.length }, null) }",
    ] {
        assert!(
            compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err(),
            "{source}"
        );
    }
    fs::remove_dir_all(output).unwrap();
}
