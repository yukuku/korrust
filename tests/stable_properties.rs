use korrust::{CompileOptions, SourceInput, compile_batch_with_options};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn compiled_final_default_properties_preserve_receiver_identity_and_flow_scope() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_properties)
        .unwrap()
        .join()
        .unwrap();
}

fn check_properties() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/stable_properties");
    let temporary =
        std::env::temp_dir().join(format!("korrust-stable-properties-{}", std::process::id()));
    let dependency = temporary.join("dependency");
    std::fs::create_dir_all(&temporary).unwrap();
    let output = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Dependency.kt"))
        .arg("-d")
        .arg(&dependency)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let paths = [
        dependency.clone(),
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let compile = |name: &str, friends: bool| {
        compile_batch_with_options(
            &[SourceInput {
                identity: name.into(),
                file_name: name.into(),
                source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
            }],
            &paths,
            &CompileOptions {
                module_name: "stable".into(),
                friend_paths: if friends {
                    vec![dependency.clone()]
                } else {
                    Vec::new()
                },
            },
        )
    };
    compile("Models.kt", true).unwrap();
    assert!(compile("CrossModule.kt", false).is_err());
    for fixture in std::fs::read_dir(&fixtures).unwrap() {
        let name = fixture.unwrap().file_name().to_string_lossy().into_owned();
        if name.starts_with("Invalid") {
            assert!(compile(&name, true).is_err(), "{name}");
        }
    }
    std::fs::remove_dir_all(temporary).unwrap();
}
