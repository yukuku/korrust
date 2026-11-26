#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn infer_parameters_from_nested_vararg_components() {
    kotlin_java::compare("nested_vararg_inference", &["Bad.kt"]);
}
