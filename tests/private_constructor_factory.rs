#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn explicit_import_names_the_factory_even_for_a_generic_classifier() {
    kotlin_java::compare("private_constructor_factory", &["Bad.kt"]);
}
