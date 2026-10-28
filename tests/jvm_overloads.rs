use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn overload_collisions_reject_before_outputs_are_returned() {
    for source in [
        "@JvmOverloads fun value(a: Int = 1): Int = a\nfun value(): Int = 2",
        "@JvmOverloads fun value(a: Int = 1): Int = a\n@JvmOverloads fun value(a: Long = 2L): Int = 2",
        "class Value { @JvmOverloads fun value(a: Int = 1): Int = a\nfun value(): Int = 2 }",
        "object Value { @JvmOverloads fun value(a: Int = 1): Int = a\nfun value(): Int = 2 }",
        "class Value { fun value(): Int = 2\ncompanion object { @JvmStatic @JvmOverloads fun value(a: Int = 1): Int = a } }",
    ] {
        let error = compile_outputs_with_classpath(source, "Overloads.kt", &classpath())
            .err()
            .expect("Conflicting overloads must not return artifacts");
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{}",
            error.message
        );
        assert_eq!(error.code, "JVM_SIGNATURE_CLASH", "{}", error.message);
    }
}

#[test]
fn jvm_overloads_accepts_an_ordinary_method_without_defaults() {
    let outputs = compile_outputs_with_classpath(
        "@JvmOverloads fun value(a: Int): Int = a",
        "Overloads.kt",
        &classpath(),
    )
    .unwrap();
    assert!(
        outputs
            .iter()
            .any(|output| output.relative_path == "OverloadsKt.class")
    );
}

#[test]
fn static_annotations_reject_non_object_member_contexts() {
    for source in [
        "@JvmStatic @JvmOverloads fun value(a: Int = 1): Int = a",
        "class Value { @JvmStatic fun value(a: Int): Int = a }",
        "abstract class Value { @JvmStatic abstract fun value(a: Int): Int }",
        "interface Value { @JvmStatic fun value(a: Int): Int = a }",
    ] {
        let error = compile_outputs_with_classpath(source, "Overloads.kt", &classpath())
            .err()
            .expect("Invalid marker placement must not return artifacts");
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{}",
            error.message
        );
        assert_eq!(
            error.code, "JVM_STATIC_PLACEMENT_INVALID",
            "{}",
            error.message
        );
    }
}
