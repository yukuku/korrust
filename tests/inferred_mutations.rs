use korrust::compile_all;

#[test]
fn inferred_member_bodies_can_mutate_mutable_properties() {
    for source in [
        "class Counter { private var value = false\nset(next) { field = next }\nfun update(flag: Boolean) = when { flag -> { value = true; 1 }; else -> { value = false; 0 } } }",
        "class Counter(var value: Int) { fun update(flag: Boolean) = when { flag -> { value = 2; value }; else -> value } }",
    ] {
        compile_all(source, "Counter.kt").unwrap();
    }
}

#[test]
fn inferred_member_bodies_reject_immutable_property_writes() {
    for source in [
        "class Counter { private val value = 1; fun update(flag: Boolean) = when { flag -> { value = 2; 1 }; else -> 0 } }",
        "class Counter(val value: Int) { fun update(flag: Boolean) = when { flag -> { value = 2; 1 }; else -> 0 } }",
    ] {
        assert!(compile_all(source, "Counter.kt").is_err());
    }
}
