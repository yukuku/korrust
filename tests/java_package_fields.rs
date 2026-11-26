#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn java_package_fields_follow_declaring_package_visibility() {
    kotlin_java::compare(
        "java_package_fields",
        &["Outside.kt", "Private.kt", "Final.kt"],
    );
}
