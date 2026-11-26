#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn compare_overloads_using_each_vararg_parameter_position() {
    kotlin_java::compare("vararg_overload_positions", &["Bad.kt"]);
}
