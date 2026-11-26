#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn enum_properties_resolve_on_instances_with_colliding_static_entries() {
    kotlin_java::compare("enum_instance_property", &["Bad.kt"]);
}
