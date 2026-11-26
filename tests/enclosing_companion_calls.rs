#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn unqualified_calls_search_enclosing_companions_in_scope_order() {
    kotlin_java::compare("enclosing_companion_calls", &["Outside.kt"]);
}
