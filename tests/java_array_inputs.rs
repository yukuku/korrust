#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn java_platform_array_inputs_preserve_jvm_covariance() {
    kotlin_java::compare(
        "java_array_inputs",
        &["Primitive.kt", "WrongElement.kt", "Invariant.kt"],
    );
}
