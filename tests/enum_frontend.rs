use korrust::{SourceInput, compile_all_with_classpath, compile_batch_with_classpath};
use std::path::PathBuf;

fn standard() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn enum_constructors_members_and_companions_use_ordinary_types() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    let source = include_str!("fixtures/compatibility_enums_numeric/EnumModel.kt");
    let outputs =
        compile_all_with_classpath(source, "EnumModel.kt", std::slice::from_ref(&standard))
            .unwrap();
    compile_all_with_classpath(
        "enum class Deferred(val callback: () -> Int) { A({ B.ordinal }), B({ 0 }) }",
        "Deferred.kt",
        std::slice::from_ref(&standard),
    )
    .unwrap();
    let nested = compile_all_with_classpath(
        "class Outer { enum class E(val code: Int) { A(3) } }",
        "Outer.kt",
        std::slice::from_ref(&standard),
    )
    .unwrap();
    assert!(nested.iter().any(|class| class.internal_name == "Outer$E"));
    for owner in [
        "compatibility/enumsnumeric/Signal",
        "compatibility/enumsnumeric/Signal$Companion",
        "compatibility/enumsnumeric/Empty",
    ] {
        assert!(
            outputs.iter().any(|class| class.internal_name == owner),
            "{owner}"
        );
    }
}

#[test]
fn enum_initialization_dependencies_and_argument_types_reject_at_original_source() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "enum class E(val number: Int) { A(\"wrong\") }",
        "enum class E(val number: Int) { A(1); companion object { fun bad(): E = E(2) } }",
        "enum class E(val number: Int) { A(1) }\nfun bad(): E = E(2)",
        "enum class E(val number: Int) { A(B.number), B(2) }",
        "enum class E(val number: Int) { A(if(true) E.B.number else 0), B(2) }",
        "enum class E(val number: Int) { A(when(1) { 1 -> E.B.number; else -> 0 }), B(2) }",
        "package p\nimport p.E as Alias\nenum class E(val number: Int) { A(Alias.B.number), B(2) }",
        "enum class E(val number: Int) { A(next()); companion object { fun next(): Int = 1 } }",
        "enum class E(val number: Int) { A(E.next()); companion object { fun next(): Int = 1 } }",
    ] {
        let input = SourceInput {
            identity: "/original/Enum.kt".into(),
            file_name: "Enum.kt".into(),
            source: source.into(),
        };
        let error =
            compile_batch_with_classpath(&[input], std::slice::from_ref(&standard), "enum_fixture")
                .err()
                .expect("Invalid enum must reject");
        assert_eq!(error.source_identity.as_deref(), Some("/original/Enum.kt"));
        assert!(
            !error.message.to_lowercase().contains("unsupported"),
            "{}",
            error.message
        );
        assert!(error.offset > 0, "{}", error.message);
    }
}

#[test]
fn enum_members_cannot_hide_the_final_core_contract() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "enum class E(val name: String) { A(\"bad\") }",
        "enum class E { A; fun compareTo(other: E): Int = 0 }",
    ] {
        let error = compile_all_with_classpath(source, "Enum.kt", std::slice::from_ref(&standard))
            .err()
            .expect("Invalid inherited enum member must reject");
        assert!(
            !error.message.to_lowercase().contains("unsupported")
                && error.message.to_lowercase().contains("final"),
            "{}",
            error.message
        );
    }
    // The operator form is also invalid Kotlin. The existing unsupported
    // operator-convention boundary rejects it before enum finality checking.
    let operator = "enum class E { A; operator fun compareTo(other: E): Int = 0 }";
    let error = compile_all_with_classpath(operator, "Enum.kt", std::slice::from_ref(&standard))
        .err()
        .expect("Unsupported operator convention must reject without outputs");
    assert!(
        error.message.contains("Unsupported operator convention"),
        "{}",
        error.message
    );
}
