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
fn default_jdk_modules_match_pinned_kotlin_exports_and_fixed_java_caller() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/jdk_modules");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!("korrust-jdk-modules-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Exports.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-d",
        ])
        .arg(&official));
    let source = std::fs::read_to_string(fixtures.join("Exports.kt")).unwrap();
    let input = SourceInput {
        identity: "Exports.kt".into(),
        file_name: "Exports.kt".into(),
        source,
    };
    for output in
        compile_batch_with_classpath(&[input], std::slice::from_ref(&standard), "jdk_modules")
            .unwrap_or_else(|error| panic!("{}: {}", error.offset, error.message))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(cp(&[&official, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(cp(&[&callers, provider, &standard]))
            .arg("modules.Calls"));
        assert_eq!(output.stdout, b"jdk-modules-ok\n");
    }
    for (id, owner) in [
        ("base_internal", "jdk.internal.misc.Unsafe"),
        ("http_internal", "sun.net.httpserver.HttpServerImpl"),
        ("qualified_export", "sun.security.x509.X500Name"),
    ] {
        let short = owner.rsplit('.').next().unwrap();
        let source = format!("import {owner}\nfun value(): {short}? = null\n");
        let path = root.join(format!("{id}.kt"));
        std::fs::write(&path, &source).unwrap();
        let output = Command::new(&kotlin)
            .arg(&path)
            .arg("-d")
            .arg(root.join(id))
            .output()
            .unwrap();
        assert!(!output.status.success(), "official accepted {owner}");
        let input = SourceInput {
            identity: path.display().to_string(),
            file_name: format!("{id}.kt"),
            source,
        };
        assert!(
            compile_batch_with_classpath(
                &[input],
                std::slice::from_ref(&standard),
                "jdk_modules_invalid"
            )
            .is_err(),
            "candidate accepted {owner}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
