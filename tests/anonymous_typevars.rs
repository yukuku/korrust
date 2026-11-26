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
fn anonymous_enclosing_type_variables_resolve_to_their_lexical_generic_declarations() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let fixtures = project.join("tests/anonymous_typevars");
    let root = project
        .join("target")
        .join(format!("anonymous-typevars-{}", std::process::id()));
    let official = root.join("official");
    let candidate = root.join("candidate");
    let callers = root.join("callers");
    std::fs::create_dir_all(&root).unwrap();
    run(Command::new(&kotlin)
        .arg(fixtures.join("Model.kt"))
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
    for output in compile_outputs_with_classpath(
        &std::fs::read_to_string(fixtures.join("Model.kt")).unwrap(),
        "Model.kt",
        std::slice::from_ref(&standard),
    )
    .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Driver.java")));
    let mut signatures = Vec::new();
    for output in [&official, &candidate] {
        let result = run(Command::new("java")
            .args(["-Xverify:all", "-cp"])
            .arg(classpath(&[&callers, output, &standard]))
            .arg("typevars.Driver"));
        assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "OK");
        for owner in ["typevars.ModelKt", "typevars.Holder"] {
            let verbose = run(Command::new("javap")
                .args(["-v", "-classpath"])
                .arg(output)
                .arg(owner));
            let listing = String::from_utf8(verbose.stdout).unwrap();
            let table = listing.split_once("InnerClasses:").unwrap().1;
            assert!(
                table
                    .lines()
                    .skip(1)
                    .take_while(|line| line.starts_with(' '))
                    .any(|line| line.contains(";") && line.contains("// class typevars/")),
                "{owner}"
            );
        }
        let abi = run(Command::new("javap")
            .args(["-public", "-s", "-classpath"])
            .arg(output)
            .args(["typevars.ModelKt", "typevars.Holder"]))
        .stdout;
        let mut declarations = String::from_utf8(abi)
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with("public ") || line.starts_with("descriptor:"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        declarations.sort();
        signatures.push(declarations);
    }
    assert_eq!(signatures[0], signatures[1]);
}

#[test]
fn unresolved_anonymous_lexical_scopes_reject_before_emission() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    for (source, code) in [
        (
            include_str!("anonymous_typevars/boundaries/Shadowed.kt"),
            "TYPE_PARAMETER_SHADOW_UNSUPPORTED",
        ),
        (
            include_str!("anonymous_typevars/boundaries/Lambda.kt"),
            "ANONYMOUS_ENCLOSING_TYPE_PARAMETERS_UNSUPPORTED",
        ),
        (
            include_str!("anonymous_typevars/boundaries/Suspend.kt"),
            "SUSPEND_DECLARATION_SHAPE_UNSUPPORTED",
        ),
    ] {
        let error =
            compile_outputs_with_classpath(source, "Boundary.kt", std::slice::from_ref(&standard))
                .err()
                .unwrap();
        assert_eq!(
            error.kind,
            korrust::DiagnosticKind::UnsupportedLanguage,
            "{error:?}"
        );
        assert_eq!(error.code, code, "{error:?}");
    }
}

#[test]
fn enclosing_type_parameters_still_obey_anonymous_supertype_bounds() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    let error = compile_outputs_with_classpath(
        include_str!("anonymous_typevars/boundaries/InvalidEnclosingBound.kt"),
        "Invalid.kt",
        &[standard],
    )
    .err()
    .unwrap();
    assert_eq!(
        error.kind,
        korrust::DiagnosticKind::InvalidKotlin,
        "{error:?}"
    );
}
