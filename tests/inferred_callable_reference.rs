#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn bound_and_top_level_references_constrain_generic_results() {
    kotlin_java::compare("inferred_callable_reference", &["Bad.kt"]);
}
