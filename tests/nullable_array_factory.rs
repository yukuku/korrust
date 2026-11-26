#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn nullable_arrays_allocate_the_reified_jvm_component() {
    kotlin_java::compare("nullable_array_factory", &["Bad.kt", "Erased.kt"]);
}
