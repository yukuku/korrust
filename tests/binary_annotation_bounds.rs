#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn bounded_class_literals_in_binary_kotlin_annotations_match_the_declared_bounds() {
    kotlin_java::compare("binary_annotation_bounds", &["Bad.kt", "BadFlag.kt"]);
}
