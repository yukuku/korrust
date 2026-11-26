#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn infer_expression_getter_types_without_evaluating_getters() {
    kotlin_java::compare("getter_result_inference", &["Invalid.kt", "Block.kt"]);
}
