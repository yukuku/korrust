#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn parameterized_java_bounds_can_reference_later_parameters() {
    kotlin_java::compare("forward_java_bounds", &["Bad.kt"]);
}
