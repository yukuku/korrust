#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn discarded_try_branches_preserve_effects_and_finally() {
    kotlin_java::compare("discarded_try", &["Invalid.kt"]);
}
