#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn qualified_generic_construction_uses_argument_and_result_constraints() {
    kotlin_java::compare("qualified_generic_constructors", &["Bad.kt"]);
}
