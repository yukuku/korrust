#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn synthetic_writes_follow_the_kotlin_property_type() {
    kotlin_java::compare(
        "platform_accessor_types",
        &["BadWide.kt", "BadResult.kt", "BadParent.kt"],
    );
}
