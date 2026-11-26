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
fn java_nested_imports_preserve_owners_access_aliases_and_value_shadowing() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(verify_imports)
        .unwrap()
        .join()
        .unwrap();
}

fn verify_imports() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/java_nested_imports");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-java-nested-imports-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let java_classes = root.join("java_classes");
    run(Command::new("javac")
        .arg("-d")
        .arg(&java_classes)
        .arg(fixtures.join("Outer.java"))
        .arg(fixtures.join("Dollar.java"))
        .arg(fixtures.join("Dollar$Pretend.java")));
    let library = root.join("library");
    run(Command::new(&kotlin)
        .arg(fixtures.join("KotlinLibrary.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-cp",
        ])
        .arg(cp(&[&java_classes, &standard]))
        .arg("-d")
        .arg(&library));
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Consumer.kt"))
        .arg(fixtures.join("PackageConsumer.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-cp",
        ])
        .arg(cp(&[&java_classes, &library, &standard]))
        .arg("-d")
        .arg(&official));
    let inputs = ["Consumer.kt", "PackageConsumer.kt"].map(|name| SourceInput {
        identity: name.into(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    });
    for output in compile_batch_with_classpath(
        &inputs,
        &[java_classes.clone(), library.clone(), standard.clone()],
        "java_nested_imports",
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
        .arg(cp(&[&official, &java_classes, &library, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    let mut signatures = Vec::new();
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(cp(&[
                &callers,
                provider,
                &java_classes,
                &library,
                &standard,
            ]))
            .arg("nested.consumer.Calls"));
        assert_eq!(output.stdout, b"java-nested-imports-ok\n");
        let mut listing = Vec::new();
        for class in [
            "nested.consumer.ConsumerKt",
            "nested.library.PackageConsumerKt",
        ] {
            listing.extend(
                run(Command::new("javap")
                    .args(["-public", "-s", "-cp"])
                    .arg(provider)
                    .arg(class))
                .stdout,
            );
        }
        signatures.push(listing);
    }
    assert_eq!(signatures[0], signatures[1]);
    std::fs::remove_dir_all(root).unwrap();
}
