#[path = "fixture_support/kotlin_java.rs"]
mod kotlin_java;

#[test]
fn stable_locals_remain_nonnull_after_an_evaluated_assertion() {
    kotlin_java::compare(
        "assertion_flow",
        &["Skipped.kt", "Deferred.kt", "Caught.kt", "Mutable.kt"],
    );
}
