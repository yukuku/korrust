use korrust::{CompileOptions, DiagnosticKind, SourceInput, compile_batch_with_options};
use std::path::PathBuf;
use std::process::Command;
fn run(command: &mut Command) {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
#[test]
fn nullable_value_coercions_preserve_producer_boxing_and_null_sentinels() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}
fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/nullable_value_coercions");
    let work = std::env::temp_dir().join(format!(
        "korrust-nullable-value-coercions-{}",
        std::process::id()
    ));
    let dependency = work.join("dependency");
    let official = work.join("official");
    let candidate = work.join("candidate");
    let callers = work.join("callers");
    let standard = root.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let compiler = root.join("references/kotlinc/bin/kotlinc");
    run(Command::new(&compiler)
        .arg(fixtures.join("Library.kt"))
        .args([
            "-jvm-target",
            "1.8",
            "-module-name",
            "nullable_values_library",
            "-d",
        ])
        .arg(&dependency));
    let paths = [dependency.clone(), standard.clone()];
    let cp = std::env::join_paths(&paths).unwrap();
    let input = |name: &str| SourceInput {
        identity: fixtures.join(format!("{name}.kt")).display().to_string(),
        file_name: format!("{name}.kt"),
        source: std::fs::read_to_string(fixtures.join(format!("{name}.kt"))).unwrap(),
    };
    let options = CompileOptions {
        module_name: "nullable_values_consumer".into(),
        friend_paths: Vec::new(),
    };
    run(Command::new(&compiler)
        .arg(fixtures.join("Consumer.kt"))
        .arg("-cp")
        .arg(&cp)
        .args(["-jvm-target", "1.8", "-d"])
        .arg(&official));
    for artifact in compile_batch_with_options(&[input("Consumer")], &paths, &options).unwrap() {
        let path = candidate.join(artifact.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, artifact.output.bytes).unwrap();
    }
    run(Command::new("javac")
        .arg("-cp")
        .arg(std::env::join_paths([&official, &dependency, &standard]).unwrap())
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for output in [&official, &candidate] {
        run(Command::new("java")
            .args(["-Xverify:all", "-cp"])
            .arg(std::env::join_paths([&callers, output, &dependency, &standard]).unwrap())
            .arg("nullablevalues.consumer.Calls"));
    }
    for name in ["WrongType"] {
        let error = compile_batch_with_options(&[input(name)], &paths, &options)
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {}",
            error.message
        );
    }
    std::fs::remove_dir_all(work).unwrap();
}
