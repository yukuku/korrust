use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn invalid_value_declarations_keep_distinct_diagnostic_codes() {
    for (source, code) in [
        ("value class Bad(val raw: Int)", "VALUE_JVM_INLINE_REQUIRED"),
        (
            "@JvmInline class Bad(val raw: Int)",
            "VALUE_JVM_INLINE_PLACEMENT_INVALID",
        ),
        (
            "annotation class JvmInline\n@JvmInline value class Bad(val raw:Int)",
            "VALUE_JVM_INLINE_REQUIRED",
        ),
        (
            "@JvmInline value class Bad(var raw: Int)",
            "VALUE_CARRIER_INVALID",
        ),
        (
            "@JvmInline value class Bad(raw: Int)",
            "VALUE_CARRIER_INVALID",
        ),
        (
            "@JvmInline value class Bad(val raw: Int, val second: Int)",
            "VALUE_CARRIER_INVALID",
        ),
        (
            "@JvmInline data value class Bad(val raw: Int)",
            "VALUE_MODIFIERS_INVALID",
        ),
        (
            "@JvmInline value class Bad(val raw: Int) { val other = 2 }",
            "VALUE_STORAGE_INVALID",
        ),
        (
            "@JvmInline value class Bad(val raw: Int) { override fun hashCode(): Int = 1 }",
            "VALUE_RESERVED_METHOD",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Invalid.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, code, "{source}");
    }
}

#[test]
fn valid_unsupported_value_shapes_are_explicit() {
    for (source, code) in [
        (
            "@JvmInline value class Boundary<T>(val raw: T)",
            "VALUE_DECLARATION_SHAPE_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: Int = 1)",
            "VALUE_DECLARATION_SHAPE_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: Int) { constructor(): this(1) }",
            "VALUE_DECLARATION_SHAPE_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: Int) { companion object {} }",
            "VALUE_DECLARATION_SHAPE_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: IntArray)",
            "VALUE_CARRIER_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: Float)",
            "VALUE_CARRIER_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: Int?)",
            "VALUE_CARRIER_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary private constructor(val raw: Int)",
            "VALUE_CONSTRUCTOR_VISIBILITY_UNSUPPORTED",
        ),
        (
            "@JvmInline value class Boundary(val raw: Int) { inline fun read(): Int = raw }",
            "SOURCE_INLINE_CONTEXT_UNSUPPORTED",
        ),
        (
            "interface I<T>\n@JvmInline value class Boundary(val raw: Int): I<String>",
            "VALUE_GENERIC_SUPERTYPE_UNSUPPORTED",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, code, "{source}");
    }
}

#[test]
fn value_class_reference_identity_is_invalid() {
    let source =
        "@JvmInline value class V(val raw: Int)\nfun invalid(a: V,b: V): Boolean = a === b";
    let error = compile_outputs_with_classpath(source, "Identity.kt", &classpath())
        .err()
        .unwrap();
    assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
    assert_eq!(error.code, "VALUE_IDENTITY_EQUALITY_INVALID");
}

#[test]
fn generic_bounds_on_value_classes_are_an_explicit_boundary() {
    let source = "@JvmInline value class V(val raw: Int)\nfun <T: V> identity(value:T):T = value";
    let error = compile_outputs_with_classpath(source, "Bounds.kt", &classpath())
        .err()
        .unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "VALUE_GENERIC_BOUND_UNSUPPORTED");
}
