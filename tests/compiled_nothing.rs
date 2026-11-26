#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn bottom_calls_terminate_in_value_contexts() {
    kotlin_java::compare("compiled_nothing", &["Bad.kt", "BadVoid.kt"]);
}
