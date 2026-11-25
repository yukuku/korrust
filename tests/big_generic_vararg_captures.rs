use korrust::compile_all;

#[test]
fn generic_reference_varargs_retain_projected_types_when_captured() {
    compile_all(
        include_str!("big_generic_vararg_captures/Captured.kt"),
        "Captured.kt",
    )
    .unwrap();
}

#[test]
fn captured_reference_varargs_reject_writes_and_invariant_escapes() {
    for source in [
        "class Box<C> { fun <T> bad(vararg values:T):()->Array<T> = {values} }",
        "class Box<C> { fun <T> bad(value:T,vararg values:T):()->Unit = {values[0]=value} }",
        "class Box<C> { fun <T> bad(value:T,vararg values:T):()->Unit = {val copy=values;copy[0]=value} }",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
