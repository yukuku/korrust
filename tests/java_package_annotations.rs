use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn package_annotations_respect_type_defaults_and_declaration_visibility() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = repo.join("tests/java_package_annotations");
    let temporary = std::env::temp_dir().join(format!(
        "korrust-package-annotations-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temporary).unwrap();
    let dependency = temporary.join("dependency");
    let official = temporary.join("official");
    let candidate = temporary.join("candidate");
    let caller = temporary.join("caller");
    let stdlib = repo.join("references/kotlinc/lib/kotlin-stdlib.jar");
    run(Command::new("javac").arg("-d").arg(&dependency).args(
        [
            "api/Names.java",
            "api/package-info.java",
            "marks/Audit.java",
        ]
        .map(|p| fixtures.join(p)),
    ));
    let compile = |compiler: PathBuf, destination: &Path| {
        let mut command = Command::new(compiler);
        command
            .arg(fixtures.join("Consumer.kt"))
            .arg("-cp")
            .arg(std::env::join_paths([&dependency, &stdlib]).unwrap())
            .arg("-d")
            .arg(destination);
        command
    };
    run(&mut compile(
        repo.join("references/kotlinc/bin/kotlinc"),
        &official,
    ));
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_korrust"));
    run(&mut compile(binary.clone(), &candidate));
    run(Command::new("javac")
        .arg("-cp")
        .arg(&official)
        .arg("-d")
        .arg(&caller)
        .arg(fixtures.join("Caller.java")));
    for output in [&official, &candidate] {
        run(Command::new("java")
            .args(["-Xverify:all", "-cp"])
            .arg(std::env::join_paths([&caller, output, &dependency, &stdlib]).unwrap())
            .arg("Caller"));
    }
    fs::remove_file(dependency.join("marks/Audit.class")).unwrap();
    for missing in [true, false] {
        if !missing {
            run(Command::new("javac").arg("-d").arg(&dependency).args(
                ["strict/package-info.java", "strict/NullMarked.java"].map(|p| fixtures.join(p)),
            ));
        }
        let output = compile(binary.clone(), &candidate).output().unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("may define nullability defaults"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::remove_dir_all(temporary).unwrap();
}
