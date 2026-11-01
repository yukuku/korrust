use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::PathBuf;

fn standard() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn inner_instances_capture_outer_state_and_retain_source_identity() {
    let input = SourceInput {
        identity: "/original/Outer.kt".into(),
        file_name: "Outer.kt".into(),
        source: include_str!("inner_classes/Outer.kt").into(),
    };
    let outputs = compile_batch_with_classpath(&[input], &[standard()], "inner_test").unwrap();
    for name in [
        "innerprobe/Outer.class",
        "innerprobe/Outer$Inner.class",
        "innerprobe/OuterKt.class",
    ] {
        assert!(
            outputs
                .iter()
                .any(|output| output.source_identity == "/original/Outer.kt"
                    && output.output.relative_path == name),
            "{name}"
        );
    }
}

#[test]
fn inner_construction_requires_the_correct_nonnull_outer() {
    for source in [
        "class Outer { inner class Inner }\nfun bad(): Outer.Inner = Outer.Inner()",
        "class Outer { inner class Inner }\nfun bad(outer: Outer?): Outer.Inner = outer.Inner()",
        "import Outer.Inner\nclass Outer { inner class Inner }\nclass Other { fun bad(): Outer.Inner = Inner() }",
    ] {
        let input = SourceInput {
            identity: "/original/Invalid.kt".into(),
            file_name: "Invalid.kt".into(),
            source: source.into(),
        };
        let error = compile_batch_with_classpath(&[input], &[standard()], "inner_test")
            .err()
            .expect("Missing enclosing receiver must reject");
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {}",
            error.message
        );
        assert_eq!(
            error.source_identity.as_deref(),
            Some("/original/Invalid.kt")
        );
    }
}

#[test]
fn local_capture_and_generic_inner_boundaries_remain_explicit() {
    for source in [
        "class Outer<T> { inner class Inner }",
        "class Outer { inner class Inner<T> }",
        "class Outer { inner class Inner(val x: Int = 1) }",
        "fun f(): Int { class Local(val x: Int); return Local(3).x }",
    ] {
        let input = SourceInput {
            identity: "/original/Unsupported.kt".into(),
            file_name: "Unsupported.kt".into(),
            source: source.into(),
        };
        let error = compile_batch_with_classpath(&[input], &[standard()], "inner_test")
            .err()
            .expect("Unsupported captures must reject");
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{}",
            error.message
        );
    }
}
