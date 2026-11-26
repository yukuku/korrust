#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn preserve_nominal_types_for_nested_compiled_value_carriers() {
    kotlin_java::compare("nested_value_carriers", &["Bad.kt", "Private.kt"]);
}
