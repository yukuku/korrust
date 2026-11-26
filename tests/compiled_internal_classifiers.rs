use korrust::{CompileOptions, DiagnosticKind, SourceInput, compile_batch_with_options};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn compiled_internal_classifier_visibility_follows_friend_origins() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_friend_classifiers)
        .unwrap()
        .join()
        .unwrap();
}

fn check_friend_classifiers() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = project.join("tests/compiled_internal_classifiers");
    let temporary = std::env::temp_dir().join(format!(
        "korrust-internal-classifiers-{}",
        std::process::id()
    ));
    let dependency = temporary.join("dependency");
    let output = Command::new(project.join("references/kotlinc/bin/kotlinc"))
        .arg(fixture.join("Library.kt"))
        .args([
            "-jvm-target",
            "1.8",
            "-module-name",
            "friends_production",
            "-d",
        ])
        .arg(&dependency)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let paths = [dependency.clone(), standard];
    let input = |name: &str| SourceInput {
        identity: fixture.join(format!("{name}.kt")).display().to_string(),
        file_name: format!("{name}.kt"),
        source: std::fs::read_to_string(fixture.join(format!("{name}.kt"))).unwrap(),
    };
    let friend = CompileOptions {
        module_name: "friends_consumer".into(),
        friend_paths: vec![dependency],
    };
    let unrelated = CompileOptions {
        module_name: "friends_production".into(),
        friend_paths: Vec::new(),
    };
    compile_batch_with_options(
        &[
            input("Consumer"),
            input("Explicit"),
            input("SamePackage"),
            input("AllowedSignatures"),
            input("SuperInterface"),
            input("PrivateAlias"),
        ],
        &paths,
        &friend,
    )
    .unwrap();
    compile_batch_with_options(
        &[input("PublicInheritance"), input("Shadow")],
        &paths,
        &unrelated,
    )
    .unwrap();
    for name in ["Explicit", "Consumer", "SamePackage", "NoFriendType"] {
        let error = compile_batch_with_options(&[input(name)], &paths, &unrelated)
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {}",
            error.message
        );
    }
    for name in [
        "PrivateFriend",
        "Exposed",
        "ExposedInferred",
        "ExposedProperty",
        "ExposedConstructor",
        "ExposedMember",
        "ExposedArray",
        "ExposedSuperInterface",
        "ExposedAlias",
        "ExposedAnnotation",
    ] {
        let error = compile_batch_with_options(&[input(name)], &paths, &friend)
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {}",
            error.message
        );
    }
    std::fs::remove_dir_all(temporary).unwrap();
}
