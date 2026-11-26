#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn try_and_catch_keep_the_value_of_a_final_if_expression() {
    kotlin_java::compare("try_branch_results", &[]);
}
