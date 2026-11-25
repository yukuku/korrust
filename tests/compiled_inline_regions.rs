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
        .arg(fixtures.join("Source.kt"))
        .arg("-cp")
        .arg(&classpath)
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&official));
    for (name, source) in [
        (
            "Probe.kt",
            include_str!("fixtures/compiled_inline_regions/Probe.kt"),
        ),
        (
            "Source.kt",
            include_str!("fixtures/compiled_inline_regions/Source.kt"),
        ),
    ] {
        let dependencies = [library.clone(), standard.clone()];
        let classes = std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .spawn(move || compile_all_with_classpath(source, name, &dependencies))
            .unwrap()
            .join()
            .unwrap()
            .unwrap();
        for class in classes {
            let path = candidate.join(format!("{}.class", class.internal_name));
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, class.bytes).unwrap();
        }
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
            b"15 checks\n"
        );
    }
    for (name, provider) in [("official", &official), ("candidate", &candidate)] {
        let consumer = root.join(format!("consumer-{name}"));
        run(Command::new(&compiler)
            .arg(fixtures.join("Consumer.kt"))
            .arg("-cp")
            .arg(std::env::join_paths([provider, &standard]).unwrap())
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-d")
            .arg(&consumer));
        assert_eq!(
            run(Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(std::env::join_paths([&consumer, provider, &standard]).unwrap())
                .arg("ConsumerKt")),
            b"2 consumer checks\n"
        );
    }
    for classifier in ["ProbeKt", "SourceKt"] {
        let expected = run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-cp")
            .arg(&official)
            .arg(classifier));
        let actual = run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-cp")
            .arg(&candidate)
            .arg(classifier));
        assert_eq!(actual, expected, "{classifier} signatures");
    }
    std::fs::remove_dir_all(root).unwrap();
}
