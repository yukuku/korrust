use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::{Path, PathBuf};
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

fn java_files(path: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            java_files(&path, files);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            files.push(path);
        }
    }
}

#[test]
fn package_qualifiers_enhance_types_and_preserve_java_boundary_checks() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check)
        .unwrap()
        .join()
        .unwrap();
}

fn check() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/package_nullability");
    let work =
        std::env::temp_dir().join(format!("korrust-package-defaults-{}", std::process::id()));
    let dependency = work.join("dependency");
    let official = work.join("official");
    let candidate = work.join("candidate");
    let callers = work.join("callers");
    let standard = root.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let mut java = Vec::new();
    java_files(&fixtures.join("java"), &mut java);
    java.sort();
    run(Command::new("javac")
        .args(["--release", "8", "-d"])
        .arg(&dependency)
        .args(java));
    run(Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("KotlinLibrary.kt"))
        .arg("-cp")
        .arg(&dependency)
        .args([
            "-jvm-target",
            "1.8",
            "-module-name",
            "default_symbols",
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
    run(Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Consumer.kt"))
        .arg(fixtures.join("Broken.kt"))
        .arg(fixtures.join("Warnings.kt"))
        .arg("-cp")
        .arg(&cp)
        .args(["-jvm-target", "1.8", "-d"])
        .arg(&official));
    for artifact in compile_batch_with_classpath(
        &[input("Consumer"), input("Broken"), input("Warnings")],
        &paths,
        "package_defaults",
    )
    .unwrap()
    {
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
    for provider in [&official, &candidate] {
        run(Command::new("java")
            .args(["-Xverify:all", "-cp"])
            .arg(std::env::join_paths([&callers, provider, &dependency, &standard]).unwrap())
            .arg("defaults.consumer.Calls"));
    }
    for name in [
        "NullArgument",
        "NullableArgument",
        "ParameterDefault",
        "NullableResult",
        "ExplicitRequired",
        "NestedResult",
        "NestedArrayResult",
    ] {
        let error = compile_batch_with_classpath(&[input(name)], &paths, "negative")
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {}",
            error.message
        );
    }
    for name in ["GenericBoundary", "ScopeBoundary"] {
        let error = compile_batch_with_classpath(&[input(name)], &paths, "boundary")
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{name}: {}",
            error.message
        );
    }
    std::fs::remove_dir_all(work).unwrap();
}
