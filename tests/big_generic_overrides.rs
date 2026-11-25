use korrust::compile_all;

#[test]
fn generic_overrides_match_parameter_positions_and_preserve_bridges() {
    let classes = compile_all(
        include_str!("big_generic_overrides/Overrides.kt"),
        "Overrides.kt",
    )
    .unwrap();
    for owner in [
        "genericdispatch/TextApi",
        "genericdispatch/IntApi",
        "genericdispatch/Child",
    ] {
        assert!(classes.iter().any(|class| class.internal_name == owner));
    }
}

#[test]
fn erased_descriptors_do_not_hide_invalid_generic_override_contracts() {
    for source in [
        "interface Api { fun <T> echo(x:T):T }\nclass Bad:Api { override fun <A,B> echo(x:A):A=x }",
        "interface Api { fun <T> echo(x:T):T }\nclass Bad:Api { override fun echo(x:Any?):Any?=x }",
        "interface Api { fun <A,B> echo(x:A,y:B):A }\nclass Bad:Api { override fun <X,Y> echo(x:X,y:Y):Y=y }",
        "interface Api { fun <T:Any> echo(x:T):T }\ninterface Bad:Api { override fun <R> echo(x:R):R }",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
