use korrust::{
    DiagnosticKind, SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath,
};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn sealed_source_batch_tracks_direct_children_across_files() {
    let sources = [
        SourceInput { identity: "root".into(), file_name: "Root.kt".into(), source: "package sample\nsealed interface Root".into() },
        SourceInput { identity: "children".into(), file_name: "Children.kt".into(), source: "package sample\nclass First:Root\nclass Second:Root\nfun selected(v:Root):Int=when(v){is First->1;is Second->2}".into() },
    ];
    let outputs = compile_batch_with_classpath(&sources, &classpath(), "test").unwrap();
    assert!(
        outputs
            .iter()
            .any(|o| o.output.relative_path == "sample/Root.class")
    );
    assert!(
        outputs
            .iter()
            .any(|o| o.output.relative_path == "sample/ChildrenKt.class")
    );
}

#[test]
fn invalid_closed_when_and_constructor_boundaries_are_structured() {
    for (source, code) in [
        (
            "sealed interface S\nclass A:S\nclass B:S\nfun f(v:S):Int=when(v){is A->1}",
            "NON_EXHAUSTIVE_WHEN",
        ),
        (
            "sealed interface S\nclass A:S\nfun f(v:S?):Int=when(v){is A->1}",
            "NON_EXHAUSTIVE_WHEN",
        ),
        ("fun f(v:Boolean){when(v){true->1}}", "NON_EXHAUSTIVE_WHEN"),
        (
            "sealed class S public constructor()",
            "SEALED_CONSTRUCTOR_VISIBILITY_INVALID",
        ),
        (
            "sealed class S\nfun f():S=S()",
            "ABSTRACT_CONSTRUCTION_INVALID",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{}",
            error.message
        );
        assert_eq!(error.code, code, "{}", error.message);
    }
}

#[test]
fn valid_unsupported_sealed_shapes_fail_explicitly_at_the_source() {
    for (source, code) in [
        ("sealed class S<T>", "SEALED_GENERIC_CLASSIFIER_UNSUPPORTED"),
        (
            "sealed class S\nprotected constructor()",
            "SEALED_CONSTRUCTOR_VISIBILITY_UNSUPPORTED",
        ),
        (
            "sealed class S(val value:Int=1)",
            "SEALED_CONSTRUCTOR_DEFAULTS_UNSUPPORTED",
        ),
        (
            "fun f():Int=try{1}catch(_:Exception){2}",
            "DISCARD_CATCH_BINDING_UNSUPPORTED",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{}",
            error.message
        );
        assert_eq!(error.code, code, "{}", error.message);
        if code == "DISCARD_CATCH_BINDING_UNSUPPORTED" {
            assert_eq!(error.offset, source.find('_').unwrap());
        }
    }
}

#[test]
fn source_batch_resource_limit_is_an_explicit_boundary() {
    let mut sources = vec![SourceInput {
        identity: "root".into(),
        file_name: "Root.kt".into(),
        source: "sealed interface S".into(),
    }];
    for part in 0..2 {
        sources.push(SourceInput {
            identity: format!("children{part}"),
            file_name: format!("Children{part}.kt"),
            source: (0..129)
                .map(|i| format!("class C{part}_{i}:S"))
                .collect::<Vec<_>>()
                .join("\n"),
        });
    }
    let error = compile_batch_with_classpath(&sources, &classpath(), "test")
        .err()
        .unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "SOURCE_DECLARATION_LIMIT_UNSUPPORTED");
}

#[test]
fn data_objects_close_sealed_interfaces() {
    let outputs = compile_outputs_with_classpath(
        "sealed interface S\ndata object A:S\nfun f(v:S):Int=when(v){A->1}",
        "Data.kt",
        &classpath(),
    )
    .unwrap();
    assert!(
        outputs
            .iter()
            .any(|output| output.relative_path == "A.class")
    );
}
