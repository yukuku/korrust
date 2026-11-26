#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn broad_expected_types_preserve_declared_result_bounds() {
    kotlin_java::compare("contextual_upper_bounds", &["Wrong.kt", "Explicit.kt"]);
}
