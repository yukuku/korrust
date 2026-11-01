use korrust::{DiagnosticKind, compile_all_with_classpath};
use std::path::PathBuf;

fn standard() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn suspend_entry_points_emit_continuation_frames_only_when_needed() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    let classes = compile_all_with_classpath(
        "package example\nsuspend fun immediate(value: String): String = value\nsuspend fun delayed(step: suspend () -> String): String { val value = step(); return value + \"!\" }",
        "Suspend.kt", std::slice::from_ref(&standard)).unwrap();
    assert!(
        classes
            .iter()
            .any(|class| class.internal_name == "example/SuspendKt")
    );
    assert_eq!(
        classes
            .iter()
            .filter(|class| class
                .internal_name
                .starts_with("example/SuspendKt$suspend$"))
            .count(),
        1
    );
}

#[test]
fn ordinary_call_context_and_suspend_argument_types_remain_invalid_kotlin() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "fun broken(step: suspend () -> String): String = step()",
        "suspend fun broken(step: suspend () -> Int): String = step()",
        "suspend fun broken(step: suspend (Int) -> Int): Int = step(\"wrong\")",
    ] {
        let error =
            compile_all_with_classpath(source, "Broken.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {}",
            error.message
        );
    }
}

#[test]
fn suspension_outside_the_bounded_state_machine_rejects_explicitly() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "suspend fun bounded(step: suspend () -> String): String = if (true) step() else \"x\"",
        "suspend fun bounded(step: suspend () -> String): String = try { step() } finally { }",
        "suspend fun <T> bounded(value: T): T = value",
        "suspend fun bounded(value: Int = 1): Int = value",
        "class Member { suspend fun bounded(): String = \"x\" }",
        "suspend fun original(): String = \"x\"\nfun bounded(): suspend () -> String = ::original",
    ] {
        let error =
            compile_all_with_classpath(source, "Bounded.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {}",
            error.message
        );
    }
}
