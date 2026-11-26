#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn reference_widening_preserves_the_field_descriptor() {
    kotlin_java::compare("reference_widening", &[]);
}
