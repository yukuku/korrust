#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn implicit_extension_receiver_smart_casts_use_stable_source_getters() {
    kotlin_java::compare(
        "implicit_property_flow",
        &["Mutable.kt", "Custom.kt", "Open.kt"],
    );
}
