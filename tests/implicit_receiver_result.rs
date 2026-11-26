#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn implicit_receiver_calls_infer_types_from_argument_context() {
    kotlin_java::compare("implicit_receiver_result", &["Bad.kt"]);
}
