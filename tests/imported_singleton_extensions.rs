use korrust::{SourceInput, compile_batch_with_classpath};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn cp(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn imported_singleton_extensions_use_generic_metadata_defaults_properties_and_ordered_dispatch() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(verify_imports)
        .unwrap()
        .join()
        .unwrap();
}

fn verify_imports() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/imported_singleton_extensions");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-imported-singletons-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let events = root.join("events");
    run(Command::new("javac")
        .arg("-d")
        .arg(&events)
        .arg(fixtures.join("Events.java")));
    let library = root.join("library");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Library.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-cp",
        ])
        .arg(cp(&[&events, &standard]))
        .arg("-d")
        .arg(&library));
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Consumer.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-cp",
        ])
        .arg(cp(&[&events, &library, &standard]))
        .arg("-d")
        .arg(&official));
    let input = SourceInput {
        identity: "Consumer.kt".into(),
        file_name: "Consumer.kt".into(),
        source: std::fs::read_to_string(fixtures.join("Consumer.kt")).unwrap(),
    };
    for output in compile_batch_with_classpath(
        &[input],
        &[events.clone(), library.clone(), standard.clone()],
        "imported_singletons",
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(cp(&[&official, &events, &library, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    let mut signatures = Vec::new();
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(cp(&[&callers, provider, &events, &library, &standard]))
            .arg("singleton.consumer.Calls"));
        assert_eq!(output.stdout, b"singleton-extensions-ok\n");
        signatures.push(
            run(Command::new("javap")
                .args(["-public", "-s", "-cp"])
                .arg(provider)
                .arg("singleton.consumer.ConsumerKt"))
            .stdout,
        );
    }
    assert_eq!(signatures[0], signatures[1]);
    std::fs::remove_dir_all(root).unwrap();
}
