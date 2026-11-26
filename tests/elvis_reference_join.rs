#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn elvis_infers_a_common_reference_type() {
    kotlin_java::compare("elvis_reference_join", &["Bad.kt"]);
}
