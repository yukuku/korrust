#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn inferred_initializers_resolve_same_package_classifier_expressions() {
    kotlin_java::compare("inferred_package_qualifiers", &["Bad.kt"]);
}
