#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn implicit_stable_field_reads_keep_their_checked_receiver() {
    kotlin_java::compare("stable_field_refinements", &["Mutable.kt", "Custom.kt"]);
}
