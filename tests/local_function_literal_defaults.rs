#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn local_default_arguments_survive_capture_and_shadowing() {
    kotlin_java::compare("local_function_literal_defaults", &["Bad.kt", "BadArgument.kt"]);
}
