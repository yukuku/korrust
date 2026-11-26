use korrust::compile_all;

#[test]
fn safe_unit_results_keep_nullable_unit_jvm_descriptors() {
    let classes = compile_all(
        "class Receiver { fun touch() {} }\nfun safe(value: Receiver?): Unit? = value?.touch()\nfun force(value: Unit?): Unit = value!!\nclass Holder(val value: Unit?)",
        "Safe.kt",
    )
    .unwrap();
    assert!(classes.iter().any(|class| class.internal_name == "SafeKt"));
}

#[test]
fn nullable_unit_does_not_erase_type_errors() {
    for source in [
        "fun bad(value: Unit?): String? = value",
        "fun bad(value: String?): Unit? = value",
        "fun bad(value: Unit?): Unit = value",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
