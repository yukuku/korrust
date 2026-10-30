use korrust::{DiagnosticKind, compile};

#[test]
fn unchanged_upstream_reference_varargs_and_generic_interface_properties_are_explicit() {
    for (source, name, code) in [
        (
            include_str!(
                "../references/kotlin/compiler/testData/codegen/box/strings/rawStringsWithManyQuotes.kt"
            ),
            "rawStringsWithManyQuotes.kt",
            "SOURCE_REFERENCE_VARARG_UNSUPPORTED",
        ),
        (
            include_str!(
                "../references/kotlin/compiler/testData/codegen/box/delegation/genericProperty.kt"
            ),
            "genericProperty.kt",
            "INTERFACE_EXTENSION_UNSUPPORTED",
        ),
    ] {
        let error = compile(source, name).err().unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, code);
    }
}
