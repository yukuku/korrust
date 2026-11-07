use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::PathBuf;

fn input(name: &str, source: &str) -> SourceInput {
    SourceInput {
        identity: name.into(),
        file_name: name.into(),
        source: source.into(),
    }
}
fn runtime() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn annotations_belong_only_to_their_source_facade() {
    let sources = [
        input(
            "Definitions.kt",
            include_str!("file_annotations/Definitions.kt"),
        ),
        input("Uses.kt", include_str!("file_annotations/Uses.kt")),
        input(
            "ClassFile.kt",
            include_str!("file_annotations/ClassFile.kt"),
        ),
        input("Other.kt", include_str!("file_annotations/Other.kt")),
    ];
    let outputs = compile_batch_with_classpath(&sources, &[runtime()], "file_annotations").unwrap();
    let files = outputs
        .iter()
        .map(|output| &output.output)
        .collect::<Vec<_>>();
    let facade = files
        .iter()
        .find(|file| file.relative_path == "fileuses/UsesKt.class")
        .unwrap();
    assert!(
        facade
            .bytes
            .windows(b"Lfilemarks/Visible;".len())
            .any(|value| value == b"Lfilemarks/Visible;")
    );
    assert!(
        facade
            .bytes
            .windows(b"Lfilemarks/Binary;".len())
            .any(|value| value == b"Lfilemarks/Binary;")
    );
    assert!(
        !facade
            .bytes
            .windows(b"Lfilemarks/Source;".len())
            .any(|value| value == b"Lfilemarks/Source;")
    );
    assert!(
        !files
            .iter()
            .any(|file| file.relative_path == "fileuses/ClassFileKt.class")
    );
    for file in files.iter().filter(|file| {
        [
            "fileuses/Neighbor.class",
            "fileuses/Standalone.class",
            "fileuses/OtherKt.class",
        ]
        .contains(&file.relative_path.as_str())
    }) {
        assert!(
            !file
                .bytes
                .windows(b"Lfilemarks/Visible;".len())
                .any(|value| value == b"Lfilemarks/Visible;")
        );
    }
}

#[test]
fn source_retention_and_class_only_headers_still_check_arguments_and_targets() {
    for (name, source, code) in [
        (
            "WrongSourceArgument.kt",
            include_str!("file_annotations/invalid/WrongSourceArgument.kt"),
            "INVALID_ANNOTATION_VALUE",
        ),
        (
            "MissingArgument.kt",
            include_str!("file_annotations/invalid/MissingArgument.kt"),
            "INVALID_ANNOTATION_VALUE",
        ),
        (
            "WrongTarget.kt",
            include_str!("file_annotations/invalid/WrongTarget.kt"),
            "ANNOTATION_TARGET_INVALID",
        ),
        (
            "Duplicate.kt",
            include_str!("file_annotations/invalid/Duplicate.kt"),
            "REPEATED_ANNOTATION_INVALID",
        ),
    ] {
        let error = compile_batch_with_classpath(
            &[
                input(
                    "Definitions.kt",
                    include_str!("file_annotations/Definitions.kt"),
                ),
                input(name, source),
            ],
            &[runtime()],
            "file_annotations",
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{name}: {error:?}"
        );
        assert_eq!(error.code, code, "{name}: {error:?}");
        assert_eq!(error.source_identity.as_deref(), Some(name));
    }
}

#[test]
fn compiler_owned_file_annotations_do_not_emit_passively() {
    for name in [
        "kotlin.MustUseReturnValues",
        "kotlin.internal.JvmBuiltin",
        "kotlin.internal.SuppressBytecodeGeneration",
    ] {
        let source = format!("@file:{name}\nfun answer(): Int = 42");
        let error = compile_batch_with_classpath(
            &[input("Special.kt", &source)],
            &[runtime()],
            "file_annotations",
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{name}: {error:?}"
        );
        assert_eq!(
            error.code, "FILE_ANNOTATION_SEMANTICS_UNSUPPORTED",
            "{name}: {error:?}"
        );
    }
}

#[test]
fn unavailable_special_annotation_definitions_remain_unsupported() {
    for (name, source) in [
        (
            "OptIn.kt",
            include_str!("file_annotations/unsupported/OptIn.kt"),
        ),
        (
            "OptInAlias.kt",
            include_str!("file_annotations/unsupported/OptInAlias.kt"),
        ),
        (
            "OptInArgument.kt",
            include_str!("file_annotations/invalid/OptInArgument.kt"),
        ),
    ] {
        let error =
            compile_batch_with_classpath(&[input(name, source)], &[runtime()], "file_annotations")
                .err()
                .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{name}: {error:?}"
        );
        assert_eq!(
            error.code, "FILE_ANNOTATION_SEMANTICS_UNSUPPORTED",
            "{name}: {error:?}"
        );
    }
}

#[test]
fn misplaced_file_targets_have_an_invalid_placement_diagnostic() {
    let error = compile_batch_with_classpath(
        &[
            input(
                "Definitions.kt",
                include_str!("file_annotations/Definitions.kt"),
            ),
            input(
                "Misplaced.kt",
                include_str!("file_annotations/invalid/Misplaced.kt"),
            ),
        ],
        &[runtime()],
        "file_annotations",
    )
    .err()
    .unwrap();
    assert_eq!(error.kind, DiagnosticKind::InvalidKotlin, "{error:?}");
    assert_eq!(error.code, "ANNOTATION_USE_SITE_INVALID");
}
