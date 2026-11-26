#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn annotated_java_array_references_preserve_platform_elements() {
    kotlin_java::compare("java_annotated_array_input", &["Bad.kt", "BadType.kt"]);
}
