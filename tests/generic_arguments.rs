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

#[test]
fn abstract_generic_interface_varargs_dispatch_checked_renamed_overrides() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            compile_all(
                include_str!("generic_arguments/InterfaceVarargs.kt"),
                "InterfaceVarargs.kt",
            )
            .unwrap()
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn generic_interface_vararg_bodies_remain_explicitly_unsupported() {
    for source in [
        "interface Api { fun <T> count(vararg items:T):Int=items.size }",
        "interface Api<C> { fun count(vararg items:C):Int=items.size }",
        "interface Api<C> { fun put(value:C, vararg items:C) { items[0]=value } }",
    ] {
        let error = compile_all(source, "Boundary.kt").err().unwrap();
        assert_eq!(error.kind, korrust::DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "GENERIC_INTERFACE_VARARG_BODY_UNSUPPORTED");
    }
}

#[test]
fn nongeneric_interface_dispatches_generic_vararg_with_inferred_result() {
    compile_all("interface Api { fun <T> count(vararg x:T):Int }\nclass Impl:Api { override fun <T> count(vararg x:T)=x.size }\nfun use(api:Api):Int=api.count(1,2)", "Probe.kt").unwrap();
}
