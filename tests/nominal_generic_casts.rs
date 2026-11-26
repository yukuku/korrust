#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn checked_and_safe_nominal_casts_use_the_erased_class() {
    kotlin_java::compare("nominal_generic_casts", &["ErasedIs.kt", "WrongType.kt"]);
}
