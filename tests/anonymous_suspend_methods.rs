use std::process::Command;

#[test]
fn anonymous_suspend_methods_preserve_dispatch_and_state_after_resumption() {
    let result = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/check_anonymous_suspend_methods.py"
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
