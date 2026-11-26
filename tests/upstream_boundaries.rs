use korrust::{DiagnosticKind, compile};

#[test]
fn unchanged_upstream_generic_interface_properties_reject_explicitly() {
    for (source, name, code) in [(
        include_str!(
            "../references/kotlin/compiler/testData/codegen/box/delegation/genericProperty.kt"
        ),
        "genericProperty.kt",
        "INTERFACE_EXTENSION_UNSUPPORTED",
    )] {
        let error = compile(source, name).err().unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, code);
    }
}

#[test]
fn unchanged_upstream_reference_varargs_compile() {
    korrust::compile_all_with_classpath(
        include_str!(
            "../references/kotlin/compiler/testData/codegen/box/strings/rawStringsWithManyQuotes.kt"
        ),
        "rawStringsWithManyQuotes.kt",
        &[std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar")],
    )
    .unwrap();
}
