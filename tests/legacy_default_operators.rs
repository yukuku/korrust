#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn legacy_class_forwarders_inherit_interface_default_signatures() {
    kotlin_java::compare("legacy_default_operators", &["Invalid.kt"]);
}
