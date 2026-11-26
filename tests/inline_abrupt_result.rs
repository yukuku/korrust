#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn nonlocal_returns_in_generic_inline_calls_terminate_the_function() {
    kotlin_java::compare("inline_abrupt_result", &["Bad.kt", "Cross.kt"]);
}
