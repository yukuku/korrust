#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn read_only_extension_calls_preserve_star_projections() {
    kotlin_java::compare("star_extension_receiver", &["Bad.kt", "BadRead.kt"]);
}
