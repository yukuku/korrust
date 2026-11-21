use korrust::{DiagnosticKind, compile_all};

fn contains_signature(classes: &[korrust::ClassFile], owner: &str, signature: &str) {
    let class = classes.iter().find(|c| c.internal_name == owner).unwrap();
    assert!(
        class
            .bytes
            .windows(signature.len())
            .any(|w| w == signature.as_bytes()),
        "{owner}: missing {signature}"
    );
}

#[test]
fn independent_member_parameters_share_classifier_scope_and_erase_interface_bounds() {
    let classes = compile_all("interface Label { fun text():String }\nclass Box<C>(val value:C) { fun <T:Label> echo(x:T):T=x; fun <T> choose(x:T):C=value }\ninterface Api<C> { fun <T:Label> echo(x:T):T }\nfun use(a:Api<String>, x:Label):Label=a.echo(x)", "Members.kt").unwrap();
    contains_signature(&classes, "Box", "<T::LLabel;>(TT;)TT;");
    contains_signature(&classes, "Box", "<T:Ljava/lang/Object;>(TT;)TC;");
    contains_signature(&classes, "Api", "<T::LLabel;>(TT;)TT;");
}

#[test]
fn wildcards_follow_declared_variance_finality_nested_modes_and_function_variance() {
    let classes = compile_all("class Cov<out T>\nclass Con<in T>\nclass Inv<T>\nopen class Open\nclass Final\nfun a(x:Cov<Open>):Int=1\nfun b(x:Cov<Final>):Int=1\nfun c(x:Con<Any>):Int=1\nfun d(x:Cov<Con<Final>>):Int=1\nfun e(x:Inv<Cov<Open>>):Int=1\nfun f(x:(Cov<Open>)->Cov<Open>):Int=1\nfun r():(Cov<Open>)->Cov<Open> = {x->x}", "Wild.kt").unwrap();
    for signature in [
        "(LCov<+LOpen;>;)I",
        "(LCov<LFinal;>;)I",
        "(LCon<Ljava/lang/Object;>;)I",
        "(LCov<+LCon<-LFinal;>;>;)I",
        "(LInv<LCov<LOpen;>;>;)I",
        "(Lkotlin/jvm/functions/Function1<-LCov<+LOpen;>;+LCov<+LOpen;>;>;)I",
        "()Lkotlin/jvm/functions/Function1<LCov<+LOpen;>;LCov<LOpen;>;>;",
    ] {
        contains_signature(&classes, "WildKt", signature);
    }
}

#[test]
fn unsupported_generic_member_complexities_are_explicit() {
    for (source, code) in [
        (
            "class Box<C> { fun <T:Comparable<T>> f(x:T):T=x }",
            "DEPENDENT_GENERIC_BOUND_UNSUPPORTED",
        ),
        (
            "class Box<C> { fun <T,U:T> f(x:U):U=x }",
            "DEPENDENT_GENERIC_BOUND_UNSUPPORTED",
        ),
        (
            "class Box<C> { fun <T> f(x:T= null):T=x }",
            "GENERIC_MEMBER_COMPLEXITY_UNSUPPORTED",
        ),
        (
            "class Box<C> { fun <T> f(x:T):()->T = { x } }",
            "GENERIC_MEMBER_LAMBDA_UNSUPPORTED",
        ),
        (
            "interface Api<C> { fun <T> f(x:T):T=x }",
            "GENERIC_INTERFACE_BODY_UNSUPPORTED",
        ),
        (
            "interface Api { fun <T> f(x:T):T=x }",
            "GENERIC_INTERFACE_METHOD_BODY_UNSUPPORTED",
        ),
        (
            "class Box<C> { fun <C> f(x:C):C=x }",
            "TYPE_PARAMETER_SHADOW_UNSUPPORTED",
        ),
    ] {
        let error = compile_all(source, "Boundary.kt").err().unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, code, "{source}: {error:?}");
    }
}

#[test]
fn member_bounds_reject_wrong_arguments() {
    let error = compile_all("interface Label\nclass Box<C> { fun <T:Label> f(x:T):T=x }\nfun bad():String=Box<String>().f(\"bad\")", "Invalid.kt").err().unwrap();
    assert!(
        error.message.contains("bound")
            || error.message.contains("overload")
            || error.message.contains("applicable"),
        "{error:?}"
    );
}

#[test]
fn inherited_default_bridges_keep_contextual_input_signatures() {
    let classes = compile_all("interface Cov<out T>\nopen class Open\ninterface Api { fun take(value:Cov<Open>):Int=1 }\nclass Impl:Api\nobject Obj:Api { fun own():Int=1 }", "Defaults.kt").unwrap();
    for owner in ["Impl", "Obj"] {
        contains_signature(&classes, owner, "(LCov<+LOpen;>;)I");
    }
    contains_signature(&classes, "Api$DefaultImpls", "(LApi;LCov<+LOpen;>;)I");
}

#[test]
fn secondary_constructors_keep_classifier_parameter_scope() {
    let classes = compile_all(
        "class Box<T>(val value:T) { constructor(value:T, ignored:Int):this(value) }",
        "Secondary.kt",
    )
    .unwrap();
    contains_signature(&classes, "Box", "(TT;I)V");
}
