use std::process::Command;

#[test]
fn compose_callbacks_match_pinned_plugin_protocol_and_reject_other_shapes() {
    let result = Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/check_compose_callbacks.py"
        ))
        .args(["--candidate", env!("CARGO_BIN_EXE_korrust"), "--no-report"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
