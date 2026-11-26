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
fn infer_parameters_from_nested_vararg_components() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = repo.join("tests/nested_vararg_inference");
    let temporary =
        std::env::temp_dir().join(format!("korrust-nested-vararg-{}", std::process::id()));
    fs::create_dir_all(&temporary).unwrap();
    let dependency = temporary.join("dependency");
    let official = temporary.join("official");
    let candidate = temporary.join("candidate");
    let caller = temporary.join("caller");
    let stdlib = repo.join("references/kotlinc/lib/kotlin-stdlib.jar");
    run(Command::new(repo.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Library.kt"))
        .arg("-d")
        .arg(&dependency));
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
        .arg(std::env::join_paths([&official, &dependency]).unwrap())
        .arg("-d")
        .arg(&caller)
        .arg(fixtures.join("Caller.java")));
    for output in [&official, &candidate] {
        run(Command::new("java")
            .args(["-Xverify:all", "-cp"])
            .arg(std::env::join_paths([&caller, output, &dependency, &stdlib]).unwrap())
            .arg("Caller"));
    }
    for compiler in [repo.join("references/kotlinc/bin/kotlinc"), binary] {
        assert!(
            !Command::new(compiler)
                .arg(fixtures.join("Bad.kt"))
                .arg("-cp")
                .arg(std::env::join_paths([&dependency, &stdlib]).unwrap())
                .arg("-d")
                .arg(temporary.join("bad"))
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    fs::remove_dir_all(temporary).unwrap();
}
