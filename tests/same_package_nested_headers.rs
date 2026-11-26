#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn nested_classifiers_from_the_current_package_resolve_in_class_headers() {
    kotlin_java::compare("same_package_nested_headers", &["Bad.kt"]);
}
