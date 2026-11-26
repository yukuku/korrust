#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn reified_instance_defaults_preserve_the_dispatch_receiver_and_masks() {
    kotlin_java::compare("reified_member_defaults", &["Bad.kt"]);
}
