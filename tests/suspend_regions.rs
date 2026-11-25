use korrust::compile_all_with_classpath;
use std::path::Path;
use std::process::Command;

fn success(command: &mut Command) -> Vec<u8> {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn protected_suspension_preserves_errors_returns_and_exactly_once_finalizers() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reference = project.join("references/kotlinc");
    let standard = reference.join("lib/kotlin-stdlib.jar");
    let source = include_str!("fixtures/suspend_regions/Regions.kt");
    let root = std::env::temp_dir().join(format!(
        "korrust-suspend-regions-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    let driver = root.join("driver");
    let input = root.join("Regions.kt");
    std::fs::write(&input, source).unwrap();
    success(
        Command::new(reference.join("bin/kotlinc"))
            .arg(&input)
            .args(["-jvm-target", "1.8"])
            .arg("-d")
            .arg(&official),
    );
    let standard_for_compiler = standard.clone();
    let classes = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            compile_all_with_classpath(source, "Regions.kt", &[standard_for_compiler]).unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    for class in classes {
        let path = candidate.join(format!("{}.class", class.internal_name));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, class.bytes).unwrap();
    }
    let input = root.join("Calls.java");
    std::fs::write(&input, include_str!("fixtures/suspend_regions/Calls.java")).unwrap();
    success(
        Command::new("javac")
            .arg("-cp")
            .arg(std::env::join_paths([&official, &standard]).unwrap())
            .arg("-d")
            .arg(&driver)
            .arg(input),
    );
    for provider in [&official, &candidate] {
        let output = success(
            Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(std::env::join_paths([provider, &driver, &standard]).unwrap())
                .arg("Calls"),
        );
        assert_eq!(output, b"OK\n");
    }
    std::fs::remove_dir_all(root).unwrap();
}
