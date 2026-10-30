use korrust::{DiagnosticKind, compile};

#[test]
fn local_property_delegates_are_explicit_boundaries() {
    for source in [
        "fun f(): Int { val value by lazy { 1 }; return value }",
        "fun f(): Int { val value: Int by lazy { 1 }; return value }",
        "fun f(): Int { var value by kotlin.properties.Delegates.notNull<Int>(); value = 1; return value }",
    ] {
        let error = compile(source, "Local.kt").err().unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "LOCAL_DELEGATED_PROPERTY_UNSUPPORTED");
        assert_eq!(error.offset, source.find("by").unwrap());
    }
}
