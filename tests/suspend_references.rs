use korrust::{DiagnosticKind, compile_all_with_classpath};
use std::path::PathBuf;

fn standard() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn suspend_references_keep_logical_parameter_and_result_types() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    let classes = compile_all_with_classpath(
        include_str!("fixtures/suspend_references/Models.kt"),
        "Models.kt",
        std::slice::from_ref(&standard),
    )
    .unwrap();
    assert_eq!(
        classes
            .iter()
            .filter(|class| class.internal_name.contains("$lambda$"))
            .filter(|class| !class.internal_name.contains("$suspend$"))
            .count(),
        19
    );
}

#[test]
fn suspend_references_reject_ordinary_invocation_and_mismatched_types() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "suspend fun read(): String = \"x\"; fun broken(): () -> String = ::read",
        "suspend fun read(): String = \"x\"; fun broken(): String { val block: suspend () -> String = ::read; return block() }",
        "suspend fun read(value: Int): Int = value; fun broken(): suspend (String) -> Int = ::read",
        "suspend fun read(value: Int): Int = value; fun broken(): suspend () -> Int = ::read",
        "suspend fun read(): Int = 1; fun broken(): suspend () -> String = ::read",
        "class Reader { suspend fun read(): String = \"x\" }; fun broken(): suspend () -> String = Reader::read",
    ] {
        let error =
            compile_all_with_classpath(source, "Invalid.kt", std::slice::from_ref(&standard))
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
