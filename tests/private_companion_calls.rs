#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn private_companion_methods_use_lexically_scoped_bridges() {
    kotlin_java::compare("private_companion_calls", &["Outside.kt"]);
}
