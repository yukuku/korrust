use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn compile(source: &str) -> korrust::Diagnostic {
    compile_outputs_with_classpath(
        source,
        "Boundary.kt",
        &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar")],
    )
    .err()
    .expect("rejection")
}

#[test]
fn update_declarations_and_lvalue_lowering_compile() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            compile_outputs_with_classpath(
                include_str!("nov21_operators/Operators.kt"),
                "Operators.kt",
                &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("references/kotlinc/lib/kotlin-stdlib.jar")],
            )
            .unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn update_signature_and_assignment_boundaries_reject() {
    for source in [
        "class Bad { operator fun dec() {} }",
        "class Bad { operator fun inc(): Int = 1 }",
        "class Bad { operator fun inc() = 1 }",
        "class Bad { fun inc(): Bad = this }; fun use() { var value = Bad(); value++ }",
        "class Bad { operator fun inc(): Bad = this }; fun use(value: Bad) { value++ }",
        "class Both { operator fun plusAssign(value: Int) {}\n operator fun plus(value: Int): Both = this }; class Holder(var value: Both); fun use(h: Holder) { h.value += 1 }",
    ] {
        let error = compile(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {error:?}"
        );
    }
}

#[test]
fn parameterized_update_declarations_are_explicitly_unsupported() {
    let error = compile("class Bad { operator fun inc(value: Int): Bad = this }");
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "PARAMETERIZED_UPDATE_OPERATOR_UNSUPPORTED");
}

#[test]
fn saved_index_literals_keep_their_standalone_numeric_type() {
    let error = compile(include_str!(
        "nov21_operators/boundaries/ContextualIndex.kt"
    ));
    assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
    assert_eq!(error.code, "UPDATE_INDEX_TYPE_MISMATCH");
}
