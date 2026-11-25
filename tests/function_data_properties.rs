use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    println!(
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success());
    output
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

fn public_signatures(output: &[u8]) -> Vec<String> {
    let mut signatures = Vec::new();
    let mut member = None;
    for line in String::from_utf8_lossy(output).lines() {
        if line.starts_with("public ") {
            signatures.push(line.to_owned());
        } else if line.starts_with("  public ") {
            member = Some(line.trim().to_owned());
        } else if let Some(descriptor) = line.trim().strip_prefix("descriptor: ")
            && let Some(member) = member.take()
        {
            signatures.push(format!("{member}\n{descriptor}"));
        }
    }
    signatures.sort();
    signatures
}

fn write_outputs(source: &Path, destination: &Path, dependencies: &[PathBuf]) {
    for output in compile_outputs_with_classpath(
        &std::fs::read_to_string(source).unwrap(),
        source.file_name().unwrap().to_str().unwrap(),
        dependencies,
    )
    .unwrap()
    {
        let path = destination.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
}

#[test]
fn function_data_properties_preserve_reference_and_data_contracts() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let fixtures = project.join("tests/function_data_properties");
    let root = std::env::temp_dir().join(format!(
        "korrust-data-functions-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Models.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-cp")
        .arg(&standard)
        .arg("-d")
        .arg(&official));
    write_outputs(
        &fixtures.join("Models.kt"),
        &candidate,
        std::slice::from_ref(&standard),
    );
    let signatures = [&official, &candidate].map(|provider| {
        public_signatures(
            &run(Command::new("javap")
                .args(["-public", "-s", "-classpath"])
                .arg(classpath(&[provider, &standard]))
                .arg("functiondata.Handlers"))
            .stdout,
        )
    });
    assert_eq!(signatures[0], signatures[1]);
    let mut consumers = Vec::new();
    for (provider_name, provider) in [("official", &official), ("candidate", &candidate)] {
        for consumer_name in ["official", "candidate"] {
            let consumer = root.join(format!("{provider_name}-{consumer_name}"));
            if consumer_name == "official" {
                run(Command::new(&kotlin)
                    .arg(fixtures.join("Consumer.kt"))
                    .args([
                        "-language-version",
                        "2.3",
                        "-api-version",
                        "2.3",
                        "-jvm-target",
                        "1.8",
                    ])
                    .arg("-cp")
                    .arg(classpath(&[provider, &standard]))
                    .arg("-d")
                    .arg(&consumer));
            } else {
                write_outputs(
                    &fixtures.join("local/Consumer.kt"),
                    &consumer,
                    &[provider.clone(), standard.clone()],
                );
            }
            consumers.push((provider.clone(), consumer));
        }
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &consumers[0].1, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Driver.java")));
    for (provider, consumer) in consumers {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, &provider, &consumer, &standard]))
            .arg("functiondata.Driver"));
        assert_eq!(output.stdout, b"function-data-checks=18\n");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn receiver_and_suspend_function_data_properties_remain_explicitly_unsupported() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-function-data-boundary-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    for (index, source) in [
        "data class Bad(val callback: String.() -> Int)",
        "data class Bad(val callback: suspend (Int) -> Int)",
        "data class Bad(val values: Array<String>)",
    ]
    .into_iter()
    .enumerate()
    {
        let path = root.join("Bad.kt");
        std::fs::write(&path, source).unwrap();
        run(Command::new(project.join("references/kotlinc/bin/kotlinc"))
            .arg(path)
            .args([
                "-language-version",
                "2.3",
                "-api-version",
                "2.3",
                "-jvm-target",
                "1.8",
            ])
            .arg("-cp")
            .arg(&standard)
            .arg("-d")
            .arg(root.join(index.to_string())));
        let error =
            compile_outputs_with_classpath(source, "Bad.kt", std::slice::from_ref(&standard))
                .err()
                .unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "DATA_PROPERTY_SHAPE_UNSUPPORTED");
    }
    std::fs::remove_dir_all(root).unwrap();
}
