#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn backward_jumps_to_method_entry_verify() {
    kotlin_java::compare("entry_loop_frame", &[]);
}
