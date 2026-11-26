use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn nullable_to_string_uses_builtin_contract_and_scoped_extensions() {
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
        .arg(root.join("tests/nullable_to_string/java/stringify/Probe.java"))
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
    let inputs = ["Effects", "Local", "Member", "Alias"].map(|name| {
        let file_name = format!("{name}.kt");
        let source = root.join("tests/nullable_to_string").join(&file_name);
        SourceInput {
            identity: source.to_string_lossy().into(),
            file_name,
            source: fs::read_to_string(&source).unwrap(),
        }
    });
    compile_batch_with_classpath(&inputs, &classpath, "nullable_to_string").unwrap();
    for source in [
        "fun invalid(value: Any?): String = value.toString(1)",
        "fun invalid(value: Any?): Int = value.toString()",
        "import stringify.Probe\nfun invalid(value: Probe?): Int = value.calls",
    ] {
        assert!(
            compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err(),
            "{source}"
        );
    }
    fs::remove_dir_all(output).unwrap();
}
