#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn array_element_types_contribute_to_only_input_types() {
    kotlin_java::compare("array_input_constraints", &["Bad.kt"]);
}
