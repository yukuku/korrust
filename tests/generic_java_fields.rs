use korrust::{DiagnosticKind, compile_outputs_with_classpath};
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
fn generic_java_field_reads_specialize_inherited_receivers_and_keep_erased_access() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/generic_java_fields");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-generic-fields-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new("javac")
        .arg("-cp")
        .arg(project.join("references/kotlinc/lib/annotations-13.0.jar"))
        .arg("-d")
        .arg(&dependency)
        .args(
            [
                "Cell.java",
                "StringCell.java",
                "NumberCell.java",
                "Provider.java",
            ]
            .map(|name| fixtures.join(name)),
        ));
    let dependencies = [dependency.clone(), standard.clone()];
    let outputs = compile_outputs_with_classpath(
        &std::fs::read_to_string(fixtures.join("Use.kt")).unwrap(),
        "Use.kt",
        &dependencies,
    )
    .unwrap();
    let candidate = root.join("candidate");
    for output in outputs {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let caller = root.join("caller");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&candidate, &dependency, &standard]))
        .arg("-d")
        .arg(&caller)
        .arg(fixtures.join("Driver.java")));
    let result = run(Command::new("java")
        .args(["-Xverify:all", "-cp"])
        .arg(classpath(&[&caller, &candidate, &dependency, &standard]))
        .arg("genericfields.Driver"));
    assert_eq!(String::from_utf8_lossy(&result.stdout), "OK\n");
    for (source, code) in [
        (
            "fun bad(x:genericfields.Cell<String>) { x.value = \"wrong\" }",
            "GENERIC_JAVA_FIELD_WRITE_UNSUPPORTED",
        ),
        (
            "fun bad(x:genericfields.Cell<out String>):Any?=x.list",
            "PROJECTED_JAVA_FIELD_UNSUPPORTED",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &dependencies)
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage, "{error:?}");
        assert_eq!(error.code, code, "{error:?}");
    }
    assert!(
        compile_outputs_with_classpath(
            "fun bad(x:genericfields.Cell<String>):String=x.optional",
            "Boundary.kt",
            &dependencies
        )
        .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
