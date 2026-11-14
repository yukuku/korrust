use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn source_unary_and_compare_conventions_compile_through_ordinary_resolution() {
    let outputs = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            compile_outputs_with_classpath(
                include_str!("next10_operators/Operators.kt"),
                "Operators.kt",
                &classpath(),
            )
            .unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
    assert!(outputs.len() >= 5);
}

#[test]
fn operator_signatures_are_checked_even_when_unused_or_inferred() {
    for source in [
        include_str!("next10_operators/boundaries/UnaryArity.kt"),
        include_str!("next10_operators/boundaries/NotArity.kt"),
        include_str!("next10_operators/boundaries/CompareResult.kt"),
        include_str!("next10_operators/boundaries/CompareInferredResult.kt"),
        include_str!("next10_operators/boundaries/CompareArity.kt"),
        include_str!("next10_operators/boundaries/CompareDefault.kt"),
        include_str!("next10_operators/boundaries/CompareVararg.kt"),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {error:?}"
        );
        assert_eq!(
            error.code, "INVALID_OPERATOR_SIGNATURE",
            "{source}: {error:?}"
        );
    }
}

#[test]
fn unary_and_comparison_syntax_require_operator_modifier() {
    for source in [
        include_str!("next10_operators/boundaries/MissingUnaryModifier.kt"),
        include_str!("next10_operators/boundaries/MissingCompareModifier.kt"),
        include_str!("next10_operators/boundaries/MissingInterfaceModifier.kt"),
        include_str!("next10_operators/boundaries/UnrelatedOverload.kt"),
        include_str!("next10_operators/boundaries/PrivateAncestor.kt"),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert!(error.message.contains("operator modifier"), "{error:?}");
    }
}

#[test]
fn nullable_operator_extensions_are_an_explicit_boundary() {
    for source in [
        include_str!("next10_operators/boundaries/NullableUnary.kt"),
        include_str!("next10_operators/boundaries/NullableCompare.kt"),
        include_str!("next10_operators/boundaries/CovariantOverride.kt"),
    ] {
        let error = compile_outputs_with_classpath(source, "Boundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage, "{error:?}");
    }
}

#[test]
fn private_unary_and_comparison_operators_remain_inaccessible() {
    for source in [
        include_str!("next10_operators/boundaries/PrivateUnary.kt"),
        include_str!("next10_operators/boundaries/PrivateCompare.kt"),
    ] {
        assert!(compile_outputs_with_classpath(source, "Boundary.kt", &classpath()).is_err());
    }
}

#[test]
fn anonymous_companion_overrides_use_resolved_ancestor_headers() {
    for source in [
        "interface Root { fun value(): Int { return 7 }; interface Child : Root { fun other(): Int }; companion object { val EMPTY = object : Root { override fun value(): Int { return 0 } } } }",
        "interface Flag { operator fun not(): Int; companion object { val EMPTY = object : Flag { override fun not(): Int = 7 } } }; fun value(): Int = !Flag.EMPTY",
    ] {
        assert!(compile_outputs_with_classpath(source, "Anonymous.kt", &[]).is_ok());
    }
}
