use korrust::{DiagnosticKind, compile};

#[test]
fn anonymous_initializers_reject_unsupported_own_receiver_and_forward_reads() {
    for (members, code) in [
        (
            "val first: Int = next; val next: Int = 3",
            "ANONYMOUS_INITIALIZER_FORWARD_REFERENCE_UNSUPPORTED",
        ),
        (
            "val first: Int = first",
            "ANONYMOUS_INITIALIZER_FORWARD_REFERENCE_UNSUPPORTED",
        ),
        (
            "val first: Int = 3; val next: Int = this.first",
            "ANONYMOUS_INITIALIZER_RECEIVER_UNSUPPORTED",
        ),
    ] {
        let source = format!(
            "interface A {{ fun value(): Int }}; fun make(): A = object : A {{ {members}; override fun value(): Int = 1 }}"
        );
        let error = std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(move || compile(&source))
            .unwrap()
            .join()
            .unwrap()
            .unwrap_err();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, code);
    }
}
