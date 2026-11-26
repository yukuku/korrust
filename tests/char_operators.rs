#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn char_offsets_wrap_and_differences_are_signed() {
    kotlin_java::compare("char_operators", &["Bad.kt"]);
}
