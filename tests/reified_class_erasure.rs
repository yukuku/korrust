#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn parameterized_reified_class_literals_use_the_erased_classifier() {
    kotlin_java::compare("reified_class_erasure", &["Bad.kt"]);
}
