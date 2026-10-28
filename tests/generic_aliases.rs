use korrust::{
    DiagnosticKind, SourceInput, compile_all_with_classpath, compile_batch_with_classpath,
};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn generic_source_aliases_substitute_function_lists_arrays_and_nullable_parameters() {
    for source in [
        "typealias Rows<T> = List<T>\nfun size(value: Rows<String>): Int = value.size",
        "typealias Callback<T> = (T?) -> T\nfun apply(value: Callback<String>): String = value(null)",
        "typealias Rows<T> = List<T>\ntypealias MaybeRows<T> = Rows<T>?\nfun size(value: MaybeRows<String>): Int = value?.size ?: -1",
        "typealias Slots<T> = Array<T?>\nfun first(value: Slots<String>): String? = value[0]",
        "typealias Ignored<T> = String\nfun value(input: Ignored<Int>): String = input",
        "typealias Pair<A, B> = (A) -> B\nfun apply(value: Pair<Int, String>): String = value(42)",
    ] {
        let result = compile_all_with_classpath(source, "GenericAliases.kt", &classpath());
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn generic_alias_arity_recursion_and_parameter_scope_reject_invalid_kotlin() {
    for source in [
        "typealias Rows<T> = List<T>\nfun bad(value: Rows): Int = 1",
        "typealias Rows<T> = List<T>\nfun bad(value: Rows<String, Int>): Int = 1",
        "typealias Cycle<T> = List<Cycle<T>>",
        "typealias Left<T> = List<Right<T>>\ntypealias Right<T> = List<Left<T>>",
        "typealias Rows<T> = List<T>\nfun bad(value: T): Int = 1",
        "typealias Identity<T> = T",
        "typealias Identity<T> = T?",
        "typealias Rows<T : Any> = List<T>",
        "typealias Rows<in T> = List<T>",
        "typealias Rows<reified T> = List<T>",
    ] {
        assert!(
            compile_all_with_classpath(source, "InvalidAlias.kt", &classpath()).is_err(),
            "Accepted invalid alias: {source}"
        );
    }
}

#[test]
fn source_generic_aliases_are_visible_across_files_but_private_aliases_are_not() {
    let alias = SourceInput {
        identity: "Definitions.kt".into(),
        file_name: "Definitions.kt".into(),
        source: "package sample\ntypealias Rows<T> = List<T>".into(),
    };
    let use_alias = SourceInput {
        identity: "Uses.kt".into(),
        file_name: "Uses.kt".into(),
        source: "package sample\nfun size(value: Rows<String>): Int = value.size".into(),
    };
    for sources in [
        vec![alias.clone(), use_alias.clone()],
        vec![use_alias.clone(), alias],
    ] {
        assert!(compile_batch_with_classpath(&sources, &classpath(), "aliases").is_ok());
    }
    let private = SourceInput {
        identity: "Private.kt".into(),
        file_name: "Private.kt".into(),
        source: "package sample\nprivate typealias Rows<T> = List<T>".into(),
    };
    assert!(compile_batch_with_classpath(&[private, use_alias], &classpath(), "aliases").is_err());
}

#[test]
fn projected_nested_alias_arguments_are_explicitly_unsupported() {
    let error = compile_all_with_classpath(
        "typealias Rows<T> = List<T>\nfun bad(value: Rows<out String>): Int = 1",
        "ProjectedAlias.kt",
        &classpath(),
    )
    .err()
    .unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "TYPE_ALIAS_PROJECTION_UNSUPPORTED");
}
