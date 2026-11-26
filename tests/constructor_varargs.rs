use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
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
fn primary_constructor_varargs_preserve_calls_properties_and_metadata() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_constructor_varargs)
        .unwrap()
        .join()
        .unwrap();
}

fn check_constructor_varargs() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/constructor_varargs");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-constructor-varargs-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    let source = |name: &str| SourceInput {
        identity: fixtures.join(name).display().to_string(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    };
    for name in [
        "InvalidData.kt",
        "InvalidCollision.kt",
        "InvalidValue.kt",
        "InvalidMultiple.kt",
        "InvalidElementWrite.kt",
        "InvalidElementCall.kt",
        "InvalidPlainArray.kt",
        "InvalidSpread.kt",
        "InvalidSpreadArray.kt",
    ] {
        let error = compile_batch_with_classpath(
            &[source(name)],
            std::slice::from_ref(&standard),
            "negative",
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {error:?}"
        );
    }
    for (name, code) in [
        (
            "UnsupportedNonfinal.kt",
            "SOURCE_NONFINAL_VARARG_UNSUPPORTED",
        ),
        ("UnsupportedDefault.kt", "VARARG_DEFAULT_UNSUPPORTED"),
        ("UnsupportedUnit.kt", "SOURCE_VARARG_CARRIER_UNSUPPORTED"),
    ] {
        let error = compile_batch_with_classpath(
            &[source(name)],
            std::slice::from_ref(&standard),
            "negative",
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{name}: {error:?}"
        );
        assert_eq!(error.code, code);
    }
    run(Command::new(&kotlin)
        .arg(fixtures.join("Library.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-d")
        .arg(&official));
    for output in compile_batch_with_classpath(
        &[source("Library.kt")],
        std::slice::from_ref(&standard),
        "constructor_varargs",
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
        .arg(classpath(&[&official, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, provider, &standard]))
            .arg("constructorvarargscheck.Calls"));
        assert_eq!(output.stdout, b"constructor-varargs-ok\n");
    }
    for provider in [&official, &candidate] {
        let output = root.join(if provider == &official {
            "official-consumer"
        } else {
            "candidate-consumer"
        });
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
            .arg(&output));
        let observed = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&output, provider, &standard]))
            .arg("constructorvarargsconsumer.ConsumerKt"));
        assert_eq!(observed.stdout, b"constructor-consumer-ok\n");
        let candidate_consumer = output.with_extension("candidate");
        for class in compile_batch_with_classpath(
            &[source("Consumer.kt")],
            &[provider.clone(), standard.clone()],
            "consumer",
        )
        .unwrap_or_else(|error| panic!("consumer {}: {}", error.offset, error.message))
        {
            let path = candidate_consumer.join(class.output.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, class.output.bytes).unwrap();
        }
        let observed = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&candidate_consumer, provider, &standard]))
            .arg("constructorvarargsconsumer.ConsumerKt"));
        assert_eq!(observed.stdout, b"constructor-consumer-ok\n");
        let error = compile_batch_with_classpath(
            &[source("InvalidConsumer.kt")],
            &[provider.clone(), standard.clone()],
            "invalid_consumer",
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "projected binary property: {error:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
