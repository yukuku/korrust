#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn java_annotation_value_arrays_follow_vararg_and_default_rules() {
    kotlin_java::compare(
        "java_annotation_varargs",
        &["BadBound.kt", "BadOrder.kt", "Duplicate.kt"],
    );
}
