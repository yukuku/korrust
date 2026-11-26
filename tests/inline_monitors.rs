use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;

#[test]
fn compiled_inline_monitor_instructions_have_verification_frames() {
    let stdlib =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    compile_outputs_with_classpath(
        "fun held(lock: Any): Boolean = synchronized(lock) { Thread.holdsLock(lock) }",
        "Monitor.kt",
        &[stdlib],
    )
    .unwrap();
}
