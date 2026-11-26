#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn convert_numeric_const_initializers_with_jvm_semantics() {
    kotlin_java::compare("const_numeric_conversion", &["Bad.kt"]);
}
