#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn default_parameter_slots_keep_the_declared_reference_type() {
    kotlin_java::compare("default_reference_frames", &[]);
}
