use korrust::compile_outputs_with_classpath;
use std::path::Path;
use std::process::Command;

fn run(cmd: &mut Command) {
    let result = cmd.output().unwrap();
    assert!(
        result.status.success(),
        "{cmd:?}\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn loop_jumps_preserve_handlers_and_discard_pending_operands() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::env::temp_dir().join(format!("korrust-loop-jumps-{}", std::process::id()));
    let fixtures = root.join("tests/loop_jump_expressions");
    let stdlib = root.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let official = output.join("official");
    let candidate = output.join("candidate");
    let caller = output.join("caller");
    std::fs::create_dir_all(&candidate).unwrap();
    let names = ["Jumps.kt", "Pending.kt"];
    run(Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .args(names.map(|name| fixtures.join(name)))
        .arg("-d")
        .arg(&official));
    for name in names {
        let source = std::fs::read_to_string(fixtures.join(name)).unwrap();
        for file in
            compile_outputs_with_classpath(&source, name, std::slice::from_ref(&stdlib)).unwrap()
        {
            let path = candidate.join(file.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, file.bytes).unwrap();
        }
    }
    run(Command::new("javac")
        .arg("-cp")
        .arg(&official)
        .arg("-d")
        .arg(&caller)
        .arg(fixtures.join("Calls.java"))
        .arg(fixtures.join("PendingCheck.java")));
    for classes in [&official, &candidate] {
        for main in ["Calls", "PendingCheck"] {
            run(Command::new("java")
                .args(["-Xverify:all", "-cp"])
                .arg(std::env::join_paths([classes, &caller, &stdlib]).unwrap())
                .arg(main));
        }
    }
    std::fs::remove_dir_all(output).unwrap();
}
