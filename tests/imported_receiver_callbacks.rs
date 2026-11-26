use std::process::Command;

#[test]
fn imported_receiver_callbacks_and_suspended_unary_operands_execute() {
    let result = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/check_imported_receiver_callbacks.py"
        ))
        .args([
            "--candidate",
            env!("CARGO_BIN_EXE_korrust"),
            "--no-report",
            "--runtime-only",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
