use korrust::compile_all;

#[test]
fn inherited_generic_defaults_survive_subclass_and_implicit_receiver_lookup() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            compile_all(
                include_str!("big_generic_inherited/Inherited.kt"),
                "Inherited.kt",
            )
            .unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn inherited_defaults_do_not_allow_redeclarations_or_missing_required_arguments() {
    for source in [
        "open class Parent { open fun <T> echo(x:T, extra:Int=1):T=x }\nclass Bad:Parent() { override fun <R> echo(x:R,extra:Int=2):R=x }",
        "open class Parent { open fun <T> echo(x:T,extra:Int):T=x }\nclass Child:Parent() { override fun <R> echo(x:R,extra:Int):R=x }\nfun bad():String=Child().echo(\"x\")",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
