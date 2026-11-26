#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn infer_implicit_lambda_inputs_from_the_expected_call_result() {
    kotlin_java::compare("lambda_input_from_result", &["Bad.kt"]);
}
