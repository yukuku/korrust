use korrust::compile_all_with_classpath;
use std::path::Path;
use std::process::Command;

fn run(command: &mut Command) -> Vec<u8> {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn compiled_regions_preserve_exceptions_finally_nonlocal_returns_and_reified_types() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let compiler = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let fixtures = project.join("tests/fixtures/compiled_inline_regions");
    let root = std::env::temp_dir().join(format!("korrust-inline-regions-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let library = root.join("library");
    let official = root.join("official");
    let candidate = root.join("candidate");
    let driver = root.join("driver");
    run(Command::new(&compiler)
        .arg(fixtures.join("Library.kt"))
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&library));
    let classpath = std::env::join_paths([&library, &standard]).unwrap();
    run(Command::new(&compiler)
        .arg(fixtures.join("Probe.kt"))
        .arg("-cp")
        .arg(&classpath)
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&official));
    for class in compile_all_with_classpath(
        include_str!("fixtures/compiled_inline_regions/Probe.kt"),
        "Probe.kt",
        &[library.clone(), standard.clone()],
    )
    .unwrap()
    {
        let path = candidate.join(format!("{}.class", class.internal_name));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, class.bytes).unwrap();
    }
    run(Command::new("javac")
        .arg("-cp")
        .arg(std::env::join_paths([&official, &library, &standard]).unwrap())
        .arg("-d")
        .arg(&driver)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        assert_eq!(
            run(Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(std::env::join_paths([provider, &driver, &library, &standard]).unwrap())
                .arg("Calls")),
            b"12 checks\n"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
