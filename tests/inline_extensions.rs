use korrust::{SourceInput, compile_batch_with_classpath};
use std::path::Path;
use std::process::Command;

fn success(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn source_inline_extension_receivers_preserve_slots_evaluation_and_metadata() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let reference = project.join("references/kotlinc");
    let standard = reference.join("lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-inline-extensions-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    let driver = root.join("driver");
    let sources = [
        (
            "Library.kt",
            include_str!("fixtures/inline_extensions/Library.kt"),
        ),
        ("Use.kt", include_str!("fixtures/inline_extensions/Use.kt")),
    ];
    let mut inputs = Vec::new();
    for (name, source) in sources {
        let input = root.join(name);
        std::fs::write(input, source).unwrap();
        inputs.push(SourceInput {
            identity: name.into(),
            file_name: name.into(),
            source: source.into(),
        });
    }
    success(
        Command::new(reference.join("bin/kotlinc"))
            .arg(root.join("Library.kt"))
            .arg(root.join("Use.kt"))
            .args(["-jvm-target", "1.8"])
            .arg("-d")
            .arg(&official),
    );
    let compiler_standard = standard.clone();
    let outputs = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_batch_with_classpath(&inputs, &[compiler_standard], "main").unwrap())
        .unwrap()
        .join()
        .unwrap();
    for output in outputs {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let input = root.join("Calls.java");
    std::fs::write(
        &input,
        include_str!("fixtures/inline_extensions/Calls.java"),
    )
    .unwrap();
    success(
        Command::new("javac")
            .arg("-cp")
            .arg(std::env::join_paths([&official, &standard]).unwrap())
            .arg("-d")
            .arg(&driver)
            .arg(input),
    );
    for provider in [&official, &candidate] {
        assert_eq!(
            success(
                Command::new("java")
                    .arg("-Xverify:all")
                    .arg("-cp")
                    .arg(std::env::join_paths([provider, &driver, &standard]).unwrap())
                    .arg("Calls"),
            ),
            "OK\n"
        );
    }
    let mut signatures = Vec::new();
    for provider in [&official, &candidate] {
        let listing = success(
            Command::new("javap")
                .args(["-public", "-s", "-classpath"])
                .arg(provider)
                .arg("extensions.LibraryKt"),
        );
        signatures.push(listing.lines().skip(1).collect::<Vec<_>>().join("\n"));
    }
    assert_eq!(signatures[0], signatures[1]);
    let listing = success(
        Command::new("javap")
            .args(["-c", "-p", "-classpath"])
            .arg(&candidate)
            .arg("usage.UseKt"),
    );
    assert!(!listing.contains("extensions/LibraryKt."), "{listing}");
    let input = root.join("Consumer.kt");
    std::fs::write(
        &input,
        include_str!("fixtures/inline_extensions/Consumer.kt"),
    )
    .unwrap();
    for (index, provider) in [&official, &candidate].iter().enumerate() {
        let consumer = root.join(format!("consumer-{index}"));
        success(
            Command::new(reference.join("bin/kotlinc"))
                .arg(&input)
                .arg("-cp")
                .arg(std::env::join_paths([*provider, &standard]).unwrap())
                .arg("-d")
                .arg(&consumer),
        );
        for runtime in [&official, &candidate] {
            assert_eq!(
                success(
                    Command::new("java")
                        .arg("-Xverify:all")
                        .arg("-cp")
                        .arg(std::env::join_paths([&consumer, runtime, &standard]).unwrap())
                        .arg("ConsumerKt"),
                ),
                "OK\n"
            );
        }
    }
    let forwarding = "inline fun String.invokeLength(block: (Int) -> Int): Int = block(length)\ninline fun String.forward(block: (Int) -> Int): Int = invokeLength(block)";
    let input = root.join("Forwarding.kt");
    std::fs::write(&input, forwarding).unwrap();
    success(
        Command::new(reference.join("bin/kotlinc"))
            .arg(&input)
            .arg("-d")
            .arg(root.join("forwarding-official")),
    );
    let error = korrust::compile_all_with_classpath(
        forwarding,
        "Forwarding.kt",
        std::slice::from_ref(&standard),
    )
    .err()
    .expect("Generated inline forwarding aliases remain unsupported");
    assert_eq!(error.kind, korrust::DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "SOURCE_INLINE_CONTEXT_UNSUPPORTED");
    let invalid = "inline fun String.escape(block: () -> Int): () -> Int = block";
    let error =
        korrust::compile_all_with_classpath(invalid, "Invalid.kt", std::slice::from_ref(&standard))
            .err()
            .expect("An inline parameter cannot escape");
    assert_eq!(error.kind, korrust::DiagnosticKind::InvalidKotlin);
    assert_eq!(error.code, "INLINE_PARAMETER_ESCAPES");
    std::fs::remove_dir_all(root).unwrap();
}
