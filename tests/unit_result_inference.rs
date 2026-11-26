#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn inferred_unit_functions_have_void_jvm_returns() {
    kotlin_java::compare("unit_result_inference", &["Invalid.kt"]);
}
