use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn scalar_unsigned_fixture_compiles_with_real_library_metadata() {
    let outputs = compile_outputs_with_classpath(
        include_str!("unsigned/Scalars.kt"),
        "Scalars.kt",
        &classpath(),
    )
    .unwrap();
    assert!(outputs.len() >= 3);
}

#[test]
fn unsigned_rejections_distinguish_valid_boundaries_from_invalid_literals() {
    for (source, kind, code) in [
        (
            "const val x = 1u",
            DiagnosticKind::UnsupportedLanguage,
            "UNSIGNED_CONSTANT_UNSUPPORTED",
        ),
        (
            "fun f(x: UIntArray): UIntArray = x",
            DiagnosticKind::UnsupportedLanguage,
            "UNSIGNED_ARRAY_UNSUPPORTED",
        ),
        (
            "fun f() = 1u..2u",
            DiagnosticKind::UnsupportedLanguage,
            "UNSIGNED_RANGE_UNSUPPORTED",
        ),
        (
            "fun f(): UByte = 255u",
            DiagnosticKind::UnsupportedLanguage,
            "UNSIGNED_SMALL_LITERAL_UNSUPPORTED",
        ),
        (
            "fun f(): UInt = 18446744073709551615u",
            DiagnosticKind::InvalidKotlin,
            "UNSIGNED_LITERAL_TYPE_MISMATCH",
        ),
        (
            "fun f(): UInt = 1uL",
            DiagnosticKind::InvalidKotlin,
            "UNSIGNED_LITERAL_TYPE_MISMATCH",
        ),
        (
            "fun f() = 18446744073709551616uL",
            DiagnosticKind::InvalidKotlin,
            "UNSIGNED_LITERAL_OUT_OF_RANGE",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(error.kind, kind, "{source}: {error:?}");
        assert_eq!(error.code, code, "{source}: {error:?}");
    }
}
