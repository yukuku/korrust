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
fn collection_properties_preserve_generic_views_nullability_and_interface_dispatch() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_collection_properties)
        .unwrap()
        .join()
        .unwrap();
}

fn check_collection_properties() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/mapped_collection_properties");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-mapped-properties-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new("javac")
        .arg("-d")
        .arg(&dependency)
        .arg(fixtures.join("Renamed.java"))
        .arg(fixtures.join("Pretend.java")));
    let official = root.join("official");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Properties.kt"))
        .args(["-no-stdlib", "-jvm-target", "1.8"])
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    let dependencies = [dependency.clone(), standard.clone()];
    let source = std::fs::read_to_string(fixtures.join("Properties.kt")).unwrap();
    let candidate = root.join("candidate");
    for output in compile_outputs_with_classpath(&source, "Properties.kt", &dependencies)
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
            .arg("mapped.Calls"));
        assert_eq!(result.stdout, b"mapped-properties-ok\n");
    }
    for invalid in [
        "fun bad(input: Map<String, Int>): Set<Int> = input.keys",
        "fun bad(input: Map<String, Int?>): Collection<Int> = input.values",
        "fun bad(input: mapped.Pretend): Set<String> = input.keys",
        "fun bad(input: mapped.Pretend): Int = input.size",
    ] {
        assert!(compile_outputs_with_classpath(invalid, "Invalid.kt", &dependencies).is_err());
        let invalid_source = root.join("Invalid.kt");
        std::fs::write(&invalid_source, invalid).unwrap();
        let result = Command::new(&kotlin)
            .arg(invalid_source)
            .arg("-no-stdlib")
            .arg("-cp")
            .arg(classpath(&[&dependency, &standard]))
            .arg("-d")
            .arg(root.join("invalid"))
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
    std::fs::remove_dir_all(root).unwrap();
}
