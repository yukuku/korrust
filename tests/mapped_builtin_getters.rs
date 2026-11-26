#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn mapped_builtin_getters_use_kotlin_property_types() {
    kotlin_java::compare("mapped_builtin_getters", &["Nullable.kt", "ReadOnly.kt"]);
}
