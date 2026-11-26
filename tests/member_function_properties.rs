use korrust::{SourceInput, compile_batch_with_classpath};
use std::{path::PathBuf, process::Command};

fn success(command: &mut Command) -> Vec<u8> {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

#[test]
fn function_properties_match_member_precedence_capture_and_evaluation_order() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp = std::env::temp_dir().join(format!(
        "korrust-member-function-properties-{}",
        std::process::id()
    ));
    let official = temp.join("official");
    let candidate = temp.join("candidate");
    let caller = temp.join("caller");
    let standard = root.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let source = root.join("tests/member_function_properties/Values.kt");
    let library = temp.join("library");
    success(
        Command::new(root.join("references/kotlinc/bin/kotlinc"))
            .arg(root.join("tests/member_function_properties/Library.kt"))
            .arg("-d")
            .arg(&library),
    );
    success(
        Command::new(root.join("references/kotlinc/bin/kotlinc"))
            .arg(&source)
            .arg("-cp")
            .arg(std::env::join_paths([&library, &standard]).unwrap())
            .args([
                "-language-version",
                "2.3",
                "-api-version",
                "2.3",
                "-jvm-target",
                "1.8",
            ])
            .arg("-d")
            .arg(&official),
    );
    let compile_classpath = vec![standard.clone(), library.clone()];
    let outputs = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            compile_batch_with_classpath(
                &[SourceInput {
                    identity: source.to_string_lossy().into(),
                    file_name: "Values.kt".into(),
                    source: std::fs::read_to_string(&source).unwrap(),
                }],
                &compile_classpath,
                "member_function_properties",
            )
        })
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
    for output in outputs {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    success(
        Command::new("javac")
            .arg("-cp")
            .arg(std::env::join_paths([&official, &library, &standard]).unwrap())
            .arg("-d")
            .arg(&caller)
            .arg(root.join("tests/member_function_properties/Check.java")),
    );
    for provider in [&official, &candidate] {
        assert_eq!(
            success(
                Command::new("java")
                    .arg("-Xverify:all")
                    .arg("-cp")
                    .arg(std::env::join_paths([provider, &caller, &library, &standard]).unwrap())
                    .arg("Check")
            ),
            b"OK\n"
        );
    }
    std::fs::remove_dir_all(temp).unwrap();
}
