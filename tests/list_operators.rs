#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn list_operators_resolve_to_applicable_operator_declarations() {
    kotlin_java::compare("list_operators", &["Bad.kt"]);
}
