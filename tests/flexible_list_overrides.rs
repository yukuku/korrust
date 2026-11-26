#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn override_kotlin_choices_for_flexible_java_list_signatures() {
    kotlin_java::compare("flexible_list_overrides", &["WrongElement.kt", "Strict.kt"]);
}
