#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn resolve_nested_generic_calls_with_an_infix_result_context() {
    kotlin_java::compare("contextual_infix", &["Bad.kt"]);
}
