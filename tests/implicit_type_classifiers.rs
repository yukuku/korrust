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
fn implicit_body_and_nested_signature_classifiers_preserve_interop_and_visibility() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let fixtures = project.join("tests/implicit_type_classifiers");
    let root = std::env::temp_dir().join(format!(
        "korrust-implicit-classifiers-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    run(Command::new("javac").arg("-d").arg(&dependency).args(
        [
            "Catalog.java",
            "Catalog$Phantom.java",
            "Marker.java",
            "PackageOuter.java",
        ]
        .map(|name| fixtures.join(name)),
    ));
    run(Command::new(&kotlin)
        .arg(fixtures.join("KotlinLibrary.kt"))
        .arg("-cp")
        .arg(&standard)
        .arg("-d")
        .arg(&dependency));
    let dependencies = [dependency.clone(), standard.clone()];
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Use.kt"))
        .arg(fixtures.join("Alias.kt"))
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
    for name in ["Use.kt", "Alias.kt"] {
        let outputs = compile_outputs_with_classpath(
            &std::fs::read_to_string(fixtures.join(name)).unwrap(),
            name,
            &dependencies,
        )
        .unwrap();
        for output in outputs {
            let path = candidate.join(output.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, output.bytes).unwrap();
        }
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Driver.java")));
    let mut signatures = Vec::new();
    for output in [&official, &candidate] {
        let result = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, output, &dependency, &standard]))
            .arg("classifierprobe.Driver"));
        assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "OK 13");
        let result = run(Command::new("javap")
            .args(["-public", "-s", "-classpath"])
            .arg(classpath(&[output, &dependency]))
            .args([
                "classifierprobe.Lookup",
                "classifierprobe.Observer",
                "classifierprobe.Holder",
                "aliasprobe.AliasKt",
            ]));
        signatures.push(String::from_utf8(result.stdout).unwrap());
    }
    assert_eq!(signatures[0], signatures[1]);
    for (name, code, kind, valid) in [
        (
            "Hidden",
            "INVISIBLE_REFERENCE",
            DiagnosticKind::InvalidKotlin,
            false,
        ),
        (
            "Guarded",
            "JAVA_NESTED_CLASS_UNSUPPORTED",
            DiagnosticKind::UnsupportedLanguage,
            true,
        ),
        (
            "Missing",
            "CLASS_NOT_FOUND",
            DiagnosticKind::MissingDependency,
            false,
        ),
        (
            "Phantom",
            "CLASS_NOT_FOUND",
            DiagnosticKind::MissingDependency,
            false,
        ),
    ] {
        for source in [
            format!("package classifierprobe\nprivate fun bad(value: Catalog.{name}): Int = 1\n"),
            format!(
                "package classifierprobe\nfun bad(value: Any): Boolean = value is Catalog.{name}\n"
            ),
        ] {
            let path = root.join("Boundary.kt");
            std::fs::write(&path, &source).unwrap();
            let oracle = Command::new(&kotlin)
                .arg(&path)
                .arg("-cp")
                .arg(classpath(&[&dependency, &standard]))
                .arg("-d")
                .arg(root.join(format!("official-{name}")))
                .output()
                .unwrap();
            assert_eq!(
                oracle.status.success(),
                valid,
                "{source}\n{}",
                String::from_utf8_lossy(&oracle.stderr)
            );
            let error = compile_outputs_with_classpath(&source, "Boundary.kt", &dependencies)
                .err()
                .expect(&source);
            assert_eq!(error.code, code, "{source}: {error:?}");
            assert_eq!(error.kind, kind, "{source}: {error:?}");
        }
    }
    for (package, name, code) in [
        (
            "outsider",
            "classifierprobe.PackageOuter.Visible",
            "INVISIBLE_REFERENCE",
        ),
        (
            "classifierprobe",
            "PrivateOuter.Visible",
            "INACCESSIBLE_CLASSIFIER",
        ),
        (
            "classifierprobe",
            "InternalOuter.Visible",
            "INACCESSIBLE_CLASSIFIER",
        ),
        ("classifierprobe", "KotlinOuter.Phantom", "CLASS_NOT_FOUND"),
    ] {
        for source in [
            format!("package {package}\nfun bad(value: {name}): Int = 1\n"),
            format!("package {package}\nfun bad(value: Any): Boolean = value is {name}\n"),
        ] {
            let path = root.join("Boundary.kt");
            std::fs::write(&path, &source).unwrap();
            let oracle = Command::new(&kotlin)
                .arg(&path)
                .arg("-cp")
                .arg(classpath(&[&dependency, &standard]))
                .arg("-d")
                .arg(root.join("oracle-outer"))
                .output()
                .unwrap();
            assert!(!oracle.status.success(), "{source}");
            let error = compile_outputs_with_classpath(&source, "Boundary.kt", &dependencies)
                .err()
                .expect(&source);
            assert_eq!(error.code, code, "{source}: {error:?}");
        }
    }
    for name in ["Catalog.class", "Catalog$Mode.class"] {
        let path = dependency.join("classifierprobe").join(name);
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, b"malformed").unwrap();
        let error = compile_outputs_with_classpath(
            "package outsider\nfun bad(value: classifierprobe.Catalog.Mode): Int = 1\n",
            "Malformed.kt",
            &dependencies,
        )
        .err()
        .expect(name);
        assert!(
            error
                .message
                .contains("Malformed or unsupported Java class file"),
            "{name}: {error:?}"
        );
        std::fs::write(path, original).unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}
