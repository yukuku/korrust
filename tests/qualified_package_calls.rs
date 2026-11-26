#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn fully_qualified_package_calls_resolve_declared_overloads() {
    kotlin_java::compare("qualified_package_calls", &["Bad.kt"]);
}
