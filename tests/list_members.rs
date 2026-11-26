#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn list_builtin_members_preserve_element_type_and_mutability() {
    kotlin_java::compare("list_members", &["Bad.kt"]);
}
