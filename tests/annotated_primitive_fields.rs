#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn passive_annotations_preserve_java_primitive_field_access() {
    kotlin_java::compare("annotated_primitive_fields", &["Bad.kt"]);
}
