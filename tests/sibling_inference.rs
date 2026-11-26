#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn nested_generic_calls_use_constraints_from_sibling_arguments() {
    kotlin_java::compare("sibling_inference", &["Ambiguous.kt", "Bad.kt"]);
}
