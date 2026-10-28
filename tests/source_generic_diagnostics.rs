use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn generic_override_bridges_and_classifier_metadata_compile_together() {
    let outputs = compile_outputs_with_classpath(
        "package genericcheck\ninterface Echo<T> { fun echo(value: T): T }\nclass Value : Echo<Int> { override fun echo(value: Int): Int = value + 1 }\nclass Box<T>(var value: T)\nfun value(): Int = Box<Int>(Value().echo(4)).value",
        "Generic.kt",
        &classpath(),
    ).unwrap();
    for name in ["Echo.class", "Value.class", "Box.class", "GenericKt.class"] {
        assert!(
            outputs
                .iter()
                .any(|output| output.relative_path == format!("genericcheck/{name}"))
        );
    }
}

#[test]
fn classifier_bound_and_override_failures_do_not_return_artifacts() {
    for source in [
        "class Box<T : Number>(val value: T)\nfun invalid(): Box<String> = Box<String>(\"bad\")",
        "interface Echo<T> { fun echo(value: T): T }\nclass Value : Echo<String> { override fun echo(value: Int): Int = value }",
    ] {
        assert!(compile_outputs_with_classpath(source, "Generic.kt", &classpath()).is_err());
    }
}

#[test]
fn unsupported_classifier_shapes_have_structured_boundaries() {
    for (source, code) in [
        (
            "class Box<out T>(val value: T)",
            "TYPE_PARAMETER_MODIFIER_UNSUPPORTED",
        ),
        (
            "class Box<T>(val value: T)\nfun valid(): Box<String> = Box(\"x\")",
            "GENERIC_CONSTRUCTOR_INFERENCE_UNSUPPORTED",
        ),
        (
            "interface Echo<T> { fun echo(value: T): T = value }",
            "GENERIC_INTERFACE_BODY_UNSUPPORTED",
        ),
        (
            "class Box<T : Comparable<T>>(val value: T)",
            "DEPENDENT_GENERIC_BOUND_UNSUPPORTED",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Generic.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{}",
            error.message
        );
        assert_eq!(error.code, code, "{}", error.message);
    }
}
