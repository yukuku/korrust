use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn data_object_contracts_are_emitted_with_nested_interface_implementations() {
    let source = include_str!("expanded_objects/Objects.kt");
    let outputs = compile_outputs_with_classpath(source, "Objects.kt", &classpath()).unwrap();
    for name in [
        "Start",
        "Stop",
        "Container$Nested",
        "Tracked",
        "SerializableEntry",
    ] {
        assert!(
            outputs
                .iter()
                .any(|output| output.relative_path == format!("objectcases/{name}.class"))
        );
    }
}

#[test]
fn data_object_equality_redefinitions_are_invalid() {
    for source in [
        "data object Bad { override fun hashCode(): Int = 1 }",
        "data object Bad { override fun equals(other: Any?): Boolean = false }",
        "data object Bad { fun equals(other: Any?): Boolean = false }",
    ] {
        let error = compile_outputs_with_classpath(source, "Bad.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
        assert_eq!(error.code, "DATA_OBJECT_CUSTOM_EQUALITY");
    }
}

#[test]
fn object_class_inheritance_is_an_explicit_boundary() {
    let error = compile_outputs_with_classpath(
        "open class Base\ndata object Entry : Base()",
        "Boundary.kt",
        &classpath(),
    )
    .err()
    .unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "OBJECT_SUPERCLASS_UNSUPPORTED");
}

#[test]
fn imported_input_type_constraints_are_checked_without_replacing_library_calls() {
    let mut dependencies = classpath();
    dependencies.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-test.jar"),
    );
    compile_outputs_with_classpath("import kotlin.test.*\ndata object Entry\nfun check(){assertEquals(Entry, Entry);assertSame<Any>(Entry, Entry)}", "Inputs.kt", &dependencies).unwrap();
    let error = compile_outputs_with_classpath(
        "import kotlin.test.assertSame\nfun check(){assertSame(1, \"x\")}",
        "Invalid.kt",
        &dependencies,
    )
    .err()
    .unwrap();
    assert_eq!(
        error.kind,
        DiagnosticKind::InvalidKotlin,
        "{}",
        error.message
    );
    assert_eq!(error.code, "TYPE_INFERENCE_ONLY_INPUT_TYPES_ERROR");
    let error = compile_outputs_with_classpath(
        "import kotlin.test.assertContains\nfun check(){assertContains(listOf(1), 1)}",
        "Boundary.kt",
        &dependencies,
    )
    .err()
    .unwrap();
    assert_eq!(
        error.kind,
        DiagnosticKind::UnsupportedLanguage,
        "{}",
        error.message
    );
}
