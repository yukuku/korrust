use korrust::compile_all;

#[test]
fn generic_classifier_and_method_captures_keep_erased_runtime_storage() {
    let classes = compile_all(include_str!("big_generic/Capture.kt"), "Capture.kt").unwrap();
    assert_eq!(
        classes
            .iter()
            .filter(|c| c.internal_name.contains("$lambda$"))
            .count(),
        6
    );
}

#[test]
fn generic_captures_do_not_weaken_result_type_checks() {
    for source in [
        "class Box<C>(val seed:C) { fun capture():()->String = { seed } }",
        "class Box<C> { fun <T> capture(x:T):()->String = { x } }",
        "class Box<C> { fun <T> capture(x:T):()->T = { 1 } }",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
