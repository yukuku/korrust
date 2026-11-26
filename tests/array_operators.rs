#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn array_operators_dispatch_to_kotlin_declarations() {
    kotlin_java::compare("array_operators", &["Bad.kt"]);
}
