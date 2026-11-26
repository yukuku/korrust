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

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn qualified_classifier_values_preserve_writes_calls_and_lexical_shadowing() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_values)
        .unwrap()
        .join()
        .unwrap();
}

fn check_values() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/qualified_values");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-qualified-values-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new("javac").arg("-d").arg(&dependency).args(
        ["State.java", "Root.java", "Segment.java", "Holder.java"].map(|name| fixtures.join(name)),
    ));
    run(Command::new(&kotlin)
        .args(["Definitions.kt", "Nested.kt"].map(|name| fixtures.join(name)))
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&dependency));
    let official = root.join("official");
    let names = ["Uses.kt", "Alias.kt", "PropertyAlias.kt"];
    run(Command::new(&kotlin)
        .args(names.map(|name| fixtures.join(name)))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    let candidate = root.join("candidate");
    let dependencies = [dependency.clone(), standard.clone()];
    let inputs = names.map(|name| SourceInput {
        identity: fixtures.join(name).display().to_string(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    });
    for output in compile_batch_with_classpath(&inputs, &dependencies, "qualified_values")
        .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, provider, &dependency, &standard]))
            .arg("qualifiedcheck.Calls"));
        assert_eq!(output.stdout, b"qualified-values-ok\n");
    }
    for name in ["InvalidShadow.kt", "InvalidFinal.kt"] {
        let input = SourceInput {
            identity: fixtures.join(name).display().to_string(),
            file_name: name.into(),
            source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
        };
        assert!(
            compile_batch_with_classpath(&[input], &dependencies, "qualified_values").is_err(),
            "{name}"
        );
        let output = Command::new(&kotlin)
            .arg(fixtures.join(name))
            .arg("-cp")
            .arg(classpath(&[&dependency, &standard]))
            .arg("-d")
            .arg(root.join(name))
            .output()
            .unwrap();
        assert!(!output.status.success(), "{name}");
    }
    std::fs::remove_dir_all(root).unwrap();
}
