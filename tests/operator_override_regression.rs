use korrust::{compile_all, compile_outputs_with_classpath};
use std::path::PathBuf;

#[test]
fn receiver_substitution_precedes_generic_operator_override_filtering() {
    let root = std::env::var_os("KORRUST_REFERENCE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    compile_outputs_with_classpath(
        include_str!("operator_override_regression/Use.kt"),
        "Use.kt",
        &[root.join("references/kotlinc/lib/kotlin-stdlib.jar")],
    )
    .unwrap();
}

#[test]
fn unrelated_operator_overloads_remain_ambiguous() {
    let error = compile_all(
        include_str!("operator_override_regression/Ambiguous.kt"),
        "Ambiguous.kt",
    )
    .err()
    .unwrap();
    assert!(error.message.contains("Ambiguous"), "{error:?}");
}
