use korrust::{
    DiagnosticKind, SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath,
};
use std::path::PathBuf;

fn runtime() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}
fn source(name: &str, text: &str) -> SourceInput {
    SourceInput {
        identity: name.into(),
        file_name: name.into(),
        source: text.into(),
    }
}
#[test]
fn named_facades_follow_canonical_annotations_and_shared_source_imports() {
    let outputs = compile_batch_with_classpath(
        &[
            source("Named.kt", include_str!("next10_jvm/Named.kt")),
            source("Classes.kt", include_str!("next10_jvm/Classes.kt")),
            source("CrossFile.kt", include_str!("next10_jvm/CrossFile.kt")),
        ],
        &runtime(),
        "next_jvm",
    )
    .unwrap();
    assert!(
        outputs
            .iter()
            .any(|output| output.output.relative_path == "nextjvm/api/StableApi.class")
    );
    assert!(
        !outputs
            .iter()
            .any(|output| output.output.relative_path == "nextjvm/api/NamedKt.class")
    );
    assert!(outputs.iter().any(|output| {
        output.output.relative_path.ends_with(".kotlin_module")
            && output
                .output
                .bytes
                .windows(9)
                .any(|bytes| bytes == b"StableApi")
    }));
}
#[test]
fn shadow_annotations_do_not_rename_facades() {
    let outputs = compile_batch_with_classpath(
        &[
            source(
                "ShadowDefinition.kt",
                include_str!("next10_jvm/ShadowDefinition.kt"),
            ),
            source("ShadowUse.kt", include_str!("next10_jvm/ShadowUse.kt")),
            source("ShadowAlias.kt", include_str!("next10_jvm/ShadowAlias.kt")),
            source(
                "ShadowWildcard.kt",
                include_str!("next10_jvm/ShadowWildcard.kt"),
            ),
        ],
        &runtime(),
        "shadows",
    )
    .unwrap();
    assert!(
        outputs
            .iter()
            .any(|output| output.output.relative_path == "marks/ShadowUseKt.class")
    );
    assert!(
        outputs
            .iter()
            .any(|output| output.output.relative_path == "shadowalias/ShadowAliasKt.class")
    );
    assert!(
        !outputs
            .iter()
            .any(|output| output.output.relative_path.contains("NotAFacade"))
    );
}
#[test]
fn facade_collisions_are_rejected_before_emission() {
    let error = compile_batch_with_classpath(
        &[
            source("A.kt", "@file:JvmName(\"Same\")\nfun a(): Int = 1"),
            source("B.kt", "@file:JvmName(\"Same\")\nfun b(): Int = 2"),
        ],
        &runtime(),
        "collisions",
    )
    .err()
    .unwrap();
    assert!(error.message.contains("Duplicate source declaration: Same"));
}
#[test]
fn private_constructor_access_does_not_escape_lexical_scope() {
    for text in [
        "class C private constructor()\nfun bad(): C = C()",
        "class C private constructor()\nfun bad() { check(C() != null) }",
        "class C private constructor()\ntypealias Alias = C\nfun bad(): C = Alias()",
        "class Outer { class Nested private constructor(); fun bad(): Nested = Nested() }",
        "class C internal constructor()\ninline fun bad(): C = C()",
        "class C internal constructor()\ninline fun bad(): () -> C = { C() }",
    ] {
        let error = compile_outputs_with_classpath(text, "Bad.kt", &runtime())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{text}: {error:?}"
        );
        assert!(
            matches!(
                error.code,
                "INACCESSIBLE_CONSTRUCTOR" | "PUBLIC_INLINE_NON_PUBLIC_ACCESS"
            ),
            "{error:?}"
        );
    }
}
#[test]
fn nonpublic_constructor_boundaries_remain_explicit() {
    for text in [
        "class C private constructor(val value: Int = 1)",
        "class C internal constructor(val value: Int = 1)",
        "class C<T> private constructor(val value: T)",
        "data class C private constructor(val value: Int)",
        "open class C protected constructor()",
    ] {
        let error = compile_outputs_with_classpath(text, "Boundary.kt", &runtime())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{text}: {error:?}"
        );
    }
}
