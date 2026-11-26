#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn nonlocal_return_contributes_no_lambda_result_constraint() {
    kotlin_java::compare(
        "nonlocal_result_inference",
        &["Bad.kt", "Cross.kt", "Prefix.kt"],
    );
}
