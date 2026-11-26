#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;
#[test]
fn spread_calls_infer_their_component_from_a_concrete_vararg() {
    kotlin_java::compare("contextual_vararg_spreads", &["Bad.kt"]);
}
