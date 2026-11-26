#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn java_member_constructors_use_the_declared_enclosing_instance() {
    kotlin_java::compare(
        "java_inner_constructors",
        &["Hidden.kt", "MissingOuter.kt", "Private.kt"],
    );
}
