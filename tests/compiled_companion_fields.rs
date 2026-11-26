#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn compiled_companion_field_signatures_bind_to_enclosing_storage() {
    kotlin_java::compare("compiled_companion_fields", &["Bad.kt"]);
}
