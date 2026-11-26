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
fn source_composable_types_export_and_invoke_with_actual_composer() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_source_composable_types)
        .unwrap()
        .join()
        .unwrap();
}

fn check_source_composable_types() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/source_composable_types");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-source-composable-types-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let runtime = project.join("references/compose/runtime.jar");
    let annotations = project.join("references/compose/annotations.jar");
    let plugin = project.join("references/kotlinc/lib/compose-compiler-plugin.jar");
    let dependencies = root.join("dependencies");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Dependency.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-Xlambdas=class",
        ])
        .arg(format!("-Xplugin={}", plugin.display()))
        .args([
            "-P",
            "plugin:androidx.compose.compiler.plugins.kotlin:sourceInformation=false",
        ])
        .arg("-cp")
        .arg(classpath(&[&runtime, &annotations, &standard]))
        .arg("-d")
        .arg(&dependencies));
    let source = |name: &str| SourceInput {
        identity: fixtures.join(name).display().to_string(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    };
    let cp = [
        dependencies.clone(),
        runtime.clone(),
        annotations.clone(),
        standard.clone(),
    ];
    for name in [
        "InvalidOutside.kt",
        "InvalidSuspend.kt",
        "InvalidArity.kt",
        "InvalidPlainArgument.kt",
        "InvalidPlainResult.kt",
        "InvalidAnnotationArguments.kt",
    ] {
        let error = compile_batch_with_classpath(&[source(name)], &cp, "invalid")
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
            "UnsupportedArguments.kt",
            "COMPOSE_CALLBACK_SHAPE_UNSUPPORTED",
        ),
        ("UnsupportedResult.kt", "COMPOSE_CALLBACK_SHAPE_UNSUPPORTED"),
        (
            "UnsupportedReceiver.kt",
            "COMPOSE_CALLBACK_SHAPE_UNSUPPORTED",
        ),
        (
            "UnsupportedNullable.kt",
            "COMPOSE_CALLBACK_SHAPE_UNSUPPORTED",
        ),
        (
            "UnsupportedComposite.kt",
            "COMPOSE_CALLBACK_SHAPE_UNSUPPORTED",
        ),
        (
            "UnsupportedClass.kt",
            "COMPOSE_SOURCE_CLASS_STABILITY_UNSUPPORTED",
        ),
        (
            "UnsupportedNested.kt",
            "COMPOSE_NESTED_CALLBACK_UNSUPPORTED",
        ),
        (
            "UnsupportedConditional.kt",
            "COMPOSE_CONTROL_FLOW_UNSUPPORTED",
        ),
        ("UnsupportedDeclaration.kt", "COMPILER_PLUGIN_UNSUPPORTED"),
        ("UnsupportedStar.kt", "ANNOTATION_TYPE_USE_UNSUPPORTED"),
        ("UnsupportedLookalike.kt", "ANNOTATION_TYPE_USE_UNSUPPORTED"),
    ] {
        let error = compile_batch_with_classpath(&[source(name)], &cp, "unsupported")
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{name}: {error:?}"
        );
        assert_eq!(error.code, code, "{name}: {error:?}");
    }
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Callbacks.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
        ])
        .arg("-Xlambdas=class")
        .arg(format!("-Xplugin={}", plugin.display()))
        .args([
            "-P",
            "plugin:androidx.compose.compiler.plugins.kotlin:sourceInformation=false",
        ])
        .arg("-cp")
        .arg(classpath(&[
            &dependencies,
            &runtime,
            &annotations,
            &standard,
        ]))
        .arg("-d")
        .arg(&official));
    for output in compile_batch_with_classpath(
        &[source("Callbacks.kt")],
        &[
            dependencies.clone(),
            runtime.clone(),
            annotations.clone(),
            standard.clone(),
        ],
        "source_composable_types",
    )
    .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let compile_official = |provider: &Path, output: &Path| {
        run(Command::new(&kotlin)
            .arg(fixtures.join("Use.kt"))
            .args([
                "-no-stdlib",
                "-language-version",
                "2.3",
                "-api-version",
                "2.3",
                "-jvm-target",
                "1.8",
                "-Xlambdas=class",
            ])
            .arg(format!("-Xplugin={}", plugin.display()))
            .args([
                "-P",
                "plugin:androidx.compose.compiler.plugins.kotlin:sourceInformation=false",
            ])
            .arg("-cp")
            .arg(classpath(&[
                provider,
                &dependencies,
                &runtime,
                &annotations,
                &standard,
            ]))
            .arg("-d")
            .arg(output));
    };
    let official_use = root.join("official-use");
    compile_official(&official, &official_use);
    let callers = root.join("callers");
    run(Command::new("javac")
        .args(["--release", "8"])
        .arg("-cp")
        .arg(classpath(&[
            &official_use,
            &official,
            &dependencies,
            &runtime,
            &annotations,
            &standard,
        ]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for (index, provider) in [&official, &candidate].into_iter().enumerate() {
        let pinned_use = if index == 0 {
            official_use.clone()
        } else {
            let output = root.join("candidate-library-official-use");
            compile_official(provider, &output);
            output
        };
        let own_use = root.join(format!("candidate-use-{index}"));
        for class in compile_batch_with_classpath(
            &[source("Use.kt")],
            &[
                provider.clone(),
                dependencies.clone(),
                runtime.clone(),
                annotations.clone(),
                standard.clone(),
            ],
            "consumer",
        )
        .unwrap_or_else(|error| panic!("consumer {}: {}", error.offset, error.message))
        {
            let path = own_use.join(class.output.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, class.output.bytes).unwrap();
        }
        for consumer in [&pinned_use, &own_use] {
            let output = run(Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(classpath(&[
                    &callers,
                    consumer,
                    provider,
                    &dependencies,
                    &runtime,
                    &annotations,
                    &standard,
                ]))
                .arg("composetypeprobe.Calls"));
            assert_eq!(output.stdout, b"source-composable-types-ok\n");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
