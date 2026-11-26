#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn reified_factories_preserve_static_arguments_without_runtime_type_operations() {
    kotlin_java::compare("reified_erased_factory", &["Bad.kt"]);
}
