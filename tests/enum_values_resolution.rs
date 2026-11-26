#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn kotlin_enum_values_resolves_before_the_companion() {
    kotlin_java::compare("enum_values_resolution", &["Bad.kt"]);
}
