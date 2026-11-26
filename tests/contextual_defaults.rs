#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn default_expressions_use_declared_parameter_types() {
    kotlin_java::compare("contextual_defaults", &["Bad.kt", "Later.kt"]);
}
