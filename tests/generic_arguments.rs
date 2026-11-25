use korrust::compile_all;

#[test]
fn generic_member_defaults_keep_class_and_method_parameter_scopes() {
    compile_all(include_str!("generic_arguments/Defaults.kt"), "Defaults.kt").unwrap();
}

#[test]
fn generic_member_defaults_are_type_checked() {
    for source in [
        "class Box<C> { fun <T> f(x:T=\"bad\"):T=x }",
        "class Box<C> { fun <T:Any> f(x:T=null):T=x }",
        "class Box<C> { fun <T> f(x:T, y:C=x):C=y }",
        "class Box<C> { fun <T> f(x:T=y, y:T):T=x }",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}

#[test]
fn generic_member_varargs_keep_projected_body_types() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| compile_all(include_str!("generic_arguments/Varargs.kt"), "Varargs.kt").unwrap())
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn generic_member_reference_varargs_cannot_be_written_or_escape_to_invariant_arrays() {
    for source in [
        "class Box<C> { fun <T> f(value:T, vararg items:T) { items[0]=value } }",
        "class Box<C> { fun <T> f(value:T, vararg items:T) { val alias=items; alias[0]=value } }",
        "fun <T> mutate(items:Array<T>, value:T) { items[0]=value }\nclass Box<C> { fun <T> f(value:T, vararg items:T) { mutate(items,value) } }",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
