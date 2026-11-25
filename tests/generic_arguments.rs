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
