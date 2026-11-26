use korrust::compile_outputs_with_classpath;
use std::{path::PathBuf, process::Command};
#[test]
fn lexical_labels_preserve_inline_cleanup_and_coroutine_returns() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}
fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/return_targets");
    let output =
        std::env::temp_dir().join(format!("korrust-return-targets-{}", std::process::id()));
    let provider = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Library.kt"))
        .arg("-d")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        provider.status.success(),
        "{}",
        String::from_utf8_lossy(&provider.stderr)
    );
    let java = Command::new("javac")
        .arg("-d")
        .arg(&output)
        .arg(fixtures.join("Probe.java"))
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
        root.join("target/reference-libs/kotlinx-coroutines-core-jvm-1.9.0.jar"),
    ];
    compile_outputs_with_classpath(
        include_str!("return_targets/Effects.kt"),
        "Effects.kt",
        &classpath,
    )
    .unwrap();
    for source in [
        "fun invalid(): Int = run { return@missing 1 }",
        "import targets.plain\nfun invalid(): Int { val action: () -> Int = out@{ plain { return@out 1 }; 2 }; return action() }",
        "import targets.cross\nfun invalid(): Int { val action: () -> Int = out@{ cross { return@out 1 }; 2 }; return action() }",
        "import targets.held\nfun invalid(): Int { val action: () -> Int = out@{ held { return@out 1 }; 2 }; return action() }",
    ] {
        assert!(
            compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err(),
            "{source}"
        );
    }
    std::fs::remove_dir_all(output).unwrap();
}
