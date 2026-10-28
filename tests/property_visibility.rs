use korrust::{DiagnosticKind, compile_all_with_classpath};
use std::path::PathBuf;
fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}
#[test]
fn ordinary_property_and_independent_setter_visibility_compile() {
    for source in [
        "class State { var value: Int = 0\nprivate set\nfun assign(other: State, value: Int) { other.value = value } }",
        "class State { var value: Long = 0L\nprivate set\nfun deferred(other: State, value: Long): () -> Unit = { other.value = value } }",
        "class State { internal var value: Int = 0 }",
        "class State { var value: Int = 0\ninternal set }",
        "open class State { var value: Int = 0\nprotected set }",
        "object State { var value: Int = 0\nprivate set\nfun assign(value: Int) { this.value = value } }",
        "internal var value: Int = 0\nprivate set",
    ] {
        assert!(
            compile_all_with_classpath(source, "Visibility.kt", &classpath()).is_ok(),
            "{source}"
        );
    }
}
#[test]
fn inaccessible_and_invalid_accessor_forms_reject() {
    for source in [
        "class State { var value: Int = 0\nprivate set }; fun bad(value: State) { value.value = 1 }",
        "class State { var value: Int = 0\nprivate set }; fun bad(value: State?) { value?.value = 1 }",
        "class State { var value: Int = 0\nprivate set }; fun bad() { val value: State? = null; value?.value = 1 }",
        "class State { var value: Int = 0\nprivate get }",
        "class State { private var value: Int = 0\npublic set }",
        "open class State { open var value: Int = 0\nprivate set }",
        "class State { var value: Int = 0\nprivate set\nfun bad() { this.setValue(1) } }",
    ] {
        let error = compile_all_with_classpath(source, "InvalidVisibility.kt", &classpath())
            .err()
            .expect(source);
        assert_ne!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
    }
}
#[test]
fn newly_modified_unmodeled_contexts_are_explicit() {
    for source in [
        "class State { lateinit var value: String\nprivate set }",
        "class State(internal val value: Int)",
        "enum class State { ONE; var value: Int = 0\nprivate set }",
        "object State { @JvmStatic var value: Int = 0\nprivate set }",
    ] {
        let error = compile_all_with_classpath(source, "BoundaryVisibility.kt", &classpath())
            .err()
            .expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
    }
}

#[test]
fn generic_default_storage_keeps_logical_types_and_bounds_physical_adaptation() {
    let supported = "class Dual<A, B>(initial: A) { private var value: A = initial; fun readAsB(other: Dual<B, B>): B = other.value }";
    assert!(compile_all_with_classpath(supported, "GenericStorage.kt", &classpath()).is_ok());
    let boundary = "class Cell<T>(initial: T) { var value: T = initial\n private set\nfun setInt(other: Cell<Int>, value: Int) { other.value = value } }";
    let error = compile_all_with_classpath(boundary, "GenericBoundary.kt", &classpath())
        .err()
        .unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "SPECIALIZED_PRIVATE_ACCESSOR_UNSUPPORTED");
}
