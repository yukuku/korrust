#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn java_package_methods_are_visible_in_the_declaring_package() {
    kotlin_java::compare("java_package_methods", &["Outside.kt", "Private.kt"]);
}
