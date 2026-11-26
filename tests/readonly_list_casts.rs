#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn unchecked_read_only_list_casts_test_only_the_runtime_classifier() {
    kotlin_java::compare("readonly_list_casts", &["Bad.kt"]);
}
