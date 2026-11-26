#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn expected_types_constrain_qualified_generic_java_calls() {
    kotlin_java::compare("contextual_static_calls", &["Bad.kt"]);
}
