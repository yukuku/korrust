#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn java_array_fields_keep_descriptors_and_platform_elements() {
    kotlin_java::compare("java_array_fields", &["Bad.kt"]);
}
