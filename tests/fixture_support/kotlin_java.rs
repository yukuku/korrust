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

pub fn compare(fixture: &str, invalid_sources: &[&str]) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixtures = repo.join("tests").join(fixture);
    let temporary = std::env::temp_dir().join(format!("korrust-{fixture}-{}", std::process::id()));
    fs::create_dir_all(&temporary).unwrap();
    let dependency = temporary.join("dependency");
    let caller = temporary.join("caller");
    let official = temporary.join("official");
    let candidate = temporary.join("candidate");
    let oracle = repo.join("references/kotlinc/bin/kotlinc");
    let native = PathBuf::from(env!("CARGO_BIN_EXE_korrust"));
    let stdlib = repo.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let library = fixtures.join("Library.kt");
    if library.exists() {
        run(Command::new(&oracle)
            .arg(library)
            .arg("-d")
            .arg(&dependency));
    } else {
        let mut sources: Vec<_> = fs::read_dir(&fixtures)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension().is_some_and(|ext| ext == "java") && !path.ends_with("Caller.java")
            })
            .collect();
        sources.sort();
        run(Command::new("javac")
            .arg("-d")
            .arg(&dependency)
            .args(sources));
    }
    let cp = std::env::join_paths([&dependency, &stdlib]).unwrap();
    let compile = |compiler: &Path, source: &str, output: &Path| {
        let mut command = Command::new(compiler);
        command
            .arg(fixtures.join(source))
            .arg("-cp")
            .arg(&cp)
            .arg("-d")
            .arg(output);
        command
    };
    run(&mut compile(&oracle, "Consumer.kt", &official));
    run(&mut compile(&native, "Consumer.kt", &candidate));
    run(Command::new("javac")
        .arg("-cp")
        .arg(std::env::join_paths([&official, &dependency, &stdlib]).unwrap())
        .arg("-d")
        .arg(&caller)
        .arg(fixtures.join("Caller.java")));
    for provider in [&official, &candidate] {
        run(Command::new("java")
            .args(["-Xverify:all", "-cp"])
            .arg(std::env::join_paths([&caller, provider, &dependency, &stdlib]).unwrap())
            .arg("Caller"));
    }
    for source in invalid_sources {
        for compiler in [&oracle, &native] {
            let result = compile(compiler, source, &temporary.join("invalid"))
                .output()
                .unwrap();
            assert_eq!(
                result.status.code(),
                Some(1),
                "{compiler:?} {source}\n{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
    fs::remove_dir_all(temporary).unwrap();
}
