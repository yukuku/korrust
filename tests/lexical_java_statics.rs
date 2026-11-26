#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn unqualified_inherited_java_static_calls_respect_lexical_scope() {
    kotlin_java::compare("lexical_java_statics", &["Hidden.kt", "InterfaceMember.kt"]);
}
