use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;
fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}
#[test]
fn primary_overloads_support_interleaved_defaults_and_canonical_aliases() {
    for source in [
        "class C @JvmOverloads constructor(val a: Int = 1, val required: String, val b: Long = 2L)",
        "import kotlin.jvm.JvmOverloads as Overloads\nclass C @Overloads public constructor(val a: Int = 1)",
        "class C @kotlin.jvm.JvmOverloads constructor(val a: Int)",
        "class C public constructor(val a: Int)",
        "class Empty\npublic fun after(): Int = 1",
        "class Empty\nprivate fun after(): Int = 1",
        "class Empty\ninternal fun after(): Int = 1",
    ] {
        compile_outputs_with_classpath(source, "Constructors.kt", &classpath()).unwrap();
    }
}
#[test]
fn constructor_overload_collisions_are_invalid() {
    let source = "class C @JvmOverloads constructor(val a: Int, val b: Int = 1) { constructor(a: Int) : this(a, 2) }";
    let error = compile_outputs_with_classpath(source, "Constructors.kt", &classpath())
        .err()
        .unwrap();
    assert_eq!(
        error.kind,
        DiagnosticKind::InvalidKotlin,
        "{}",
        error.message
    );
    assert_eq!(error.code, "JVM_SIGNATURE_CLASH", "{}", error.message);
}
#[test]
fn unsupported_constructor_overload_shapes_reject_explicitly() {
    for source in [
        "class Outer { inner class C @JvmOverloads constructor(val a: Int = 1) }",
        "class C<T> @JvmOverloads constructor(val a: Int = 1)",
        "sealed class C @JvmOverloads constructor(val a: Int = 1)",
        "enum class C @JvmOverloads constructor(val a: Int = 1) { A }",
        "@JvmInline value class C @JvmOverloads constructor(val a: Int)",
    ] {
        let error = compile_outputs_with_classpath(source, "Constructors.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{}",
            error.message
        );
        assert_eq!(
            error.code,
            if source.contains("inner class") {
                "INNER_CLASS_SHAPE_UNSUPPORTED"
            } else if source.starts_with("sealed") {
                "SEALED_CONSTRUCTOR_DEFAULTS_UNSUPPORTED"
            } else {
                "CONSTRUCTOR_OVERLOAD_CLASS_SHAPE_UNSUPPORTED"
            },
            "{}",
            error.message
        );
    }
}
