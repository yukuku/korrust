use korrust::compile_all;

#[test]
fn generic_interface_bodies_emit_parameterized_helpers_and_specialized_forwarders() {
    let classes = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| compile_all(include_str!("interface_bodies/Bodies.kt"), "Bodies.kt").unwrap())
        .unwrap()
        .join()
        .unwrap();
    for (owner, signature) in [
        (
            "interfacebodies/Generic$DefaultImpls",
            "<C:Ljava/lang/Object;T:Ljava/lang/Object;>(Linterfacebodies/Generic<TC;>;TT;)TT;",
        ),
        (
            "interfacebodies/Text",
            "<T::Linterfacebodies/Label;>(TT;)TT;",
        ),
    ] {
        let class = classes.iter().find(|c| c.internal_name == owner).unwrap();
        assert!(
            class
                .bytes
                .windows(signature.len())
                .any(|w| w == signature.as_bytes()),
            "{owner}: {signature}"
        );
    }
}

#[test]
fn generic_interface_bodies_keep_independent_parameters_and_bounds_checked() {
    for source in [
        "interface Api<C> { fun <T> bad(value:T):C=value }",
        "interface Api { fun <T> bad(value:T):T=1 }",
        "interface Label\ninterface Api<C> { fun <T:Label> echo(value:T):T=value }\nfun bad(api:Api<String>):String=api.echo(\"bad\")",
    ] {
        assert!(compile_all(source, "Invalid.kt").is_err(), "{source}");
    }
}
