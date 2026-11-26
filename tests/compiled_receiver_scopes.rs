use korrust::compile_outputs_with_classpath;
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

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn compiled_receiver_functions_preserve_identity_receivers_captures_and_abrupt_exits() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_compiled_receivers)
        .unwrap()
        .join()
        .unwrap();
}

fn check_compiled_receivers() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/compiled_receiver_scopes");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-compiled-receivers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Library.kt"))
        .args(["-no-stdlib", "-jvm-target", "1.8"])
        .arg("-cp")
        .arg(&standard)
        .arg("-d")
        .arg(&dependency));
    run(Command::new("javac")
        .arg("-d")
        .arg(&dependency)
        .arg(fixtures.join("Store.java"))
        .arg(fixtures.join("Generic.java")));
    let official = root.join("official");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Scopes.kt"))
        .args(["-no-stdlib", "-jvm-target", "1.8"])
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    let dependencies = [dependency.clone(), standard.clone()];
    let source = std::fs::read_to_string(fixtures.join("Scopes.kt")).unwrap();
    let candidate = root.join("candidate");
    for output in compile_outputs_with_classpath(&source, "Scopes.kt", &dependencies)
        .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let result = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, provider, &dependency, &standard]))
            .arg("receiverscopes.Calls"));
        assert_eq!(result.stdout, b"compiled-receivers-ok\n");
    }
    for invalid in [
        "fun bad(s: receiverlib.Store, x: List<Int>) { s.names = x }",
        "fun bad(s: receiverlib.Store, x: List<List<Int>>) { s.nested = x }",
        "fun bad(s: receiverlib.Store, x: Map<Int, String>) { s.counts = x }",
        "fun bad(s: receiverlib.Store, x: List<String>) { s.fixed = x }",
    ] {
        assert!(compile_outputs_with_classpath(invalid, "Invalid.kt", &dependencies).is_err());
    }
    for unsupported in [
        "fun bad(s: receiverlib.Generic<String>, x: String) { s.value = x }",
        "fun bad(s: receiverlib.Generic<String>, x: List<String>) { s.items = x }",
        "fun bad(s: receiverlib.Store, x: List<String>) { s.projected = x }",
        "fun bad(s: receiverlib.Store, x: List<String>) { s.star = x }",
    ] {
        let error = compile_outputs_with_classpath(unsupported, "Unsupported.kt", &dependencies)
            .err()
            .unwrap();
        assert_eq!(error.code, "GENERIC_JAVA_FIELD_WRITE_UNSUPPORTED");
    }
    std::fs::remove_dir_all(root).unwrap();
}
