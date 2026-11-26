#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn unused_private_companion_does_not_block_extension_lookup_in_callbacks() {
    kotlin_java::compare("deferred_companion_receiver", &[]);
}
