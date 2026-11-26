#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn infer_nonnull_generic_results_in_nullable_expected_positions() {
    kotlin_java::compare("nonnull_generic_result", &["Bad.kt"]);
}
