use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::PathBuf;

fn compile(source: &str) -> Result<Vec<korrust::SourceOutput>, korrust::Diagnostic> {
    compile_batch_with_classpath(
        &[SourceInput {
            identity: "/original/Private.kt".into(),
            file_name: "Private.kt".into(),
            source: source.into(),
        }],
        &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar")],
        "private_nested_test",
    )
}

#[test]
fn private_nested_classes_preserve_owned_outputs_and_lexical_access() {
    let source = format!(
        "{}\n{}",
        include_str!("private_nested/Vault.kt"),
        include_str!("private_nested/ObjectOwner.kt").replace("package privateprobe", "")
    );
    let outputs = compile(&source).unwrap();
    for name in [
        "privateprobe/Vault.class",
        "privateprobe/Vault$Entry.class",
        "privateprobe/Vault$Record.class",
        "privateprobe/Vault$Reader.class",
        "privateprobe/ObjectOwner$Reader.class",
        "privateprobe/Depth$Hidden.class",
        "privateprobe/Depth$Hidden$Public.class",
    ] {
        assert!(
            outputs.iter().any(|output| {
                output.source_identity == "/original/Private.kt"
                    && output.output.relative_path == name
            }),
            "missing {name}"
        );
    }
}

#[test]
fn inaccessible_private_classifiers_reject_constructor_and_type_uses() {
    for source in [
        "class Outer { private class Hidden(val x: Int) }\nfun bad(): Int = Outer.Hidden(1).x",
        "class Outer { private class Hidden }\nfun bad(value: Outer.Hidden): Int = 1",
        "class Outer { private class Hidden { class Public(val x: Int) } }\nfun bad(): Int = Outer.Hidden.Public(1).x",
        "class Outer { class First { private class Hidden }; class Second { fun bad(): Int = First.Hidden().hashCode() } }",
    ] {
        let error = compile(source).err().expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, "INACCESSIBLE_CLASSIFIER", "{source}: {error:?}");
        assert_eq!(
            error.source_identity.as_deref(),
            Some("/original/Private.kt")
        );
    }
}

#[test]
fn private_classifier_exposure_rejects_inferred_and_explicit_signatures() {
    for source in [
        "class Outer { private class Hidden; fun bad(): Hidden = Hidden() }",
        "class Outer { private class Hidden; fun bad() = Hidden() }",
        "class Outer { private class Hidden; fun bad(value: Hidden): Int = 1 }",
        "class Outer { private class Hidden; val bad: Hidden = Hidden() }",
        "class Outer { private class Hidden; class Public { fun bad(): Hidden = Hidden() } }",
    ] {
        let error = compile(source).err().expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {error:?}"
        );
        assert_eq!(
            error.code, "EXPOSED_PRIVATE_CLASSIFIER",
            "{source}: {error:?}"
        );
    }
}

#[test]
fn private_member_signatures_within_owning_scope_are_accepted() {
    compile("class Outer { private class Hidden(val x: Int); private val state: Hidden = Hidden(8); private fun make(): Hidden = state; fun read(): Int = make().x }").unwrap();
}

#[test]
fn unsupported_private_shapes_remain_explicit() {
    for source in [
        "class Outer { private inner class Hidden }",
        "class Outer { private open class Hidden }",
        "class Outer { private abstract class Hidden }",
        "class Outer { private sealed class Hidden }",
    ] {
        let error = compile(source).err().expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, "PRIVATE_NESTED_SHAPE_UNSUPPORTED");
    }
}

#[test]
fn generic_outer_and_dollar_classifier_boundaries_remain_explicit() {
    for (source, code) in [
        (
            "class Outer<T> { private data class Hidden(val x: Int); fun read(): Int = Hidden(4).x }",
            "GENERIC_CLASS_NESTED_UNSUPPORTED",
        ),
        (
            "class Outer { private class Hidden(val x: Int) }\nclass `Outer$Intruder` { fun bad(): Int = Outer.Hidden(1).x }",
            "CLASSIFIER_DOLLAR_NAME_UNSUPPORTED",
        ),
    ] {
        let error = compile(source).err().expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, code);
    }
}
