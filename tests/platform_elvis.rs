#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn platform_elvis_fallback_preserves_nullability_and_lazy_evaluation() {
    kotlin_java::compare("platform_elvis", &["Bad.kt"]);
}
