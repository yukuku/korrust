#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn dependent_callback_parameters_preserve_generic_overload_specificity() {
    kotlin_java::compare("generic_callback_specificity", &["Bad.kt", "Ambiguous.kt"]);
}
