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
        "enum class E { A; operator fun compareTo(other: E): Int = 0 }",
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
}

#[test]
fn polymorphic_enum_entries_reuse_class_dispatch_and_default_arguments() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    let outputs = compile_all_with_classpath(
        include_str!("fixtures/enum_polymorphism/Models.kt"),
        "Models.kt",
        std::slice::from_ref(&standard),
    )
    .unwrap();
    for name in ["Task$LEFT", "Task$RIGHT", "AbstractAction$A", "PerEntry$B"] {
        assert!(
            outputs
                .iter()
                .any(|class| class.internal_name == format!("enums/polymorphism/{name}")),
            "{name}"
        );
    }
}

#[test]
fn invalid_enum_entry_implementations_reject_as_invalid_kotlin() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "enum class E { A; abstract fun run(): Int }",
        "enum class E { A {}; abstract fun run(): Int }",
        "enum class E { A { override fun run(): Int = 1 }; fun run(): Int = 0 }",
        "interface I { fun run(): Int }; enum class E : I { A {} }",
        "enum class E { A { override fun missing(): Int = 1 } }",
    ] {
        let error =
            compile_all_with_classpath(source, "Invalid.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert!(
            !error.message.to_lowercase().contains("unsupported"),
            "{source}: {}",
            error.message
        );
    }
}

#[test]
fn enum_entry_declaration_boundaries_are_explicitly_unsupported() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for (source, code) in [
        (
            "enum class E { A { class Nested } }",
            "ENUM_ENTRY_DECLARATION_UNSUPPORTED",
        ),
        (
            "enum class E { A { override val value: Int = 1 }; abstract val value: Int }",
            "ABSTRACT_CLASS_PROPERTY_UNSUPPORTED",
        ),
    ] {
        let error =
            compile_all_with_classpath(source, "Unsupported.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(error.kind, korrust::DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, code);
    }
}

#[test]
fn enum_entry_constructors_without_defaults_do_not_limit_parameter_mask_width() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    let params = (0..66)
        .map(|index| format!("p{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = (0..66)
        .map(|index| index.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let source =
        format!("enum class Wide({params}) {{ A({arguments}) {{ fun value(): Int = 1 }} }}");
    compile_all_with_classpath(&source, "Wide.kt", std::slice::from_ref(&standard)).unwrap();
}
