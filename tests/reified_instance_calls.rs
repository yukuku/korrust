#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn inline_instance_methods_specialize_reified_operations_and_keep_the_receiver() {
    kotlin_java::compare("reified_instance_calls", &["Bad.kt"]);
}
