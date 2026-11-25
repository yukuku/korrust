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
fn independent_inline_numeric_and_switch_bodies_match_kotlin_with_verified_java_callers() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let fixtures = project.join("tests/compiled_inline_flow");
    let root = project
        .join("target")
        .join(format!("inline-flow-{}", std::process::id()));
    let dependency = root.join("dependency");
    let official = root.join("official");
    let candidate = root.join("candidate");
    let callers = root.join("callers");
    std::fs::create_dir_all(&root).unwrap();
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
        .arg("-cp")
        .arg(&standard)
        .arg("-d")
        .arg(&dependency));
    let disassembly = run(Command::new("javap")
        .args(["-c", "-classpath"])
        .arg(&dependency)
        .arg("flow.LibraryKt"));
    let instructions = String::from_utf8(disassembly.stdout).unwrap();
    for opcode in [
        "dload",
        "dstore",
        "dreturn",
        "dastore",
        "newarray",
        "dcmpg",
        "dcmpl",
        "daload",
        "dneg",
        "ddiv",
        "d2f",
        "f2d",
        "d2l",
        "l2d",
        "tableswitch",
        "lookupswitch",
    ] {
        assert!(
            instructions.contains(opcode),
            "missing proof opcode {opcode}"
        );
    }
    run(Command::new(&kotlin)
        .arg(fixtures.join("Use.kt"))
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
    for output in compile_outputs_with_classpath(
        &std::fs::read_to_string(fixtures.join("Use.kt")).unwrap(),
        "Use.kt",
        &[dependency.clone(), standard.clone()],
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
            .arg(classpath(&[&callers, output, &dependency, &standard]))
            .arg("flowuse.Driver"));
        assert_eq!(String::from_utf8_lossy(&result.stdout).trim(), "OK");
        signatures.push(
            run(Command::new("javap")
                .args(["-public", "-s", "-classpath"])
                .arg(output)
                .arg("flowuse.UseKt"))
            .stdout,
        );
    }
    assert_eq!(signatures[0], signatures[1]);
}
