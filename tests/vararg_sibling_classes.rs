#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn vararg_siblings_retain_their_common_base_and_array_component() {
    kotlin_java::compare("vararg_sibling_classes", &["Bad.kt"]);
}
