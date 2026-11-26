use korrust::{SourceInput, compile_batch_with_classpath};
use std::path::PathBuf;
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

#[test]
fn compiled_nonfinal_varargs_preserve_defaults_types_and_evaluation_order() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_compiled_nonfinal_varargs)
        .unwrap()
        .join()
        .unwrap();
}

fn check_compiled_nonfinal_varargs() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/nonfinal_varargs");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root =
        std::env::temp_dir().join(format!("korrust-nonfinal-varargs-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    let official = root.join("official");
    let candidate = root.join("candidate");
    let kotlinc = project.join("references/kotlinc/bin/kotlinc");
    run(Command::new(&kotlinc)
        .arg(fixtures.join("Library.kt"))
        .args(["-jvm-target", "1.8", "-Xmetadata-version=1.7.0", "-d"])
        .arg(&dependency));
    let classpath = std::env::join_paths([&dependency, &standard]).unwrap();
    run(Command::new(&kotlinc)
        .arg(fixtures.join("Consumer.kt"))
        .args(["-jvm-target", "1.8", "-cp"])
        .arg(&classpath)
        .arg("-d")
        .arg(&official));
    let input = SourceInput {
        identity: fixtures.join("Consumer.kt").display().to_string(),
        file_name: "Consumer.kt".into(),
        source: std::fs::read_to_string(fixtures.join("Consumer.kt")).unwrap(),
    };
    for output in compile_batch_with_classpath(
        &[input],
        &[dependency.clone(), standard.clone()],
        "nonfinal_varargs",
    )
    .unwrap()
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(std::env::join_paths([&official, &dependency, &standard]).unwrap())
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(std::env::join_paths([&callers, provider, &dependency, &standard]).unwrap())
            .arg("nonfinal.consumer.Calls"));
        assert_eq!(output.stdout, b"nonfinal-varargs-ok\n");
    }
    std::fs::remove_dir_all(root).unwrap();
}
