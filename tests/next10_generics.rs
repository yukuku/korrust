use korrust::{DiagnosticKind, compile_all};

#[test]
fn declaration_variance_checks_public_polarity_and_inferred_results() {
    for source in [
        "interface Read<out T> { fun get(): T }",
        "interface Write<in T> { fun put(value:T): Int }",
        "interface Valid<in I,out O> { fun transform(f:(O)->I):(I)->O }",
        "class Box<out T>(val value:T) { fun get() = value }",
    ] {
        compile_all(source, "Variance.kt").unwrap();
    }
    for source in [
        "interface Bad<out T> { fun put(value:T): Int }",
        "interface Bad<in T> { val value:T }",
        "class Bad<out T>(var value:T)",
        "interface Inv<T>\ninterface Bad<out T> { fun get():Inv<T> }",
        "interface Bad<out T> { fun bad(f:()->T):Int }",
        "class Bad<in T> { fun echo(value:T) = value }",
    ] {
        let error = compile_all(source, "Variance.kt").err().unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {error:?}"
        );
        assert_eq!(error.code, "TYPE_VARIANCE_CONFLICT", "{source}: {error:?}");
    }
}

#[test]
fn interface_classifier_bound_supports_member_resolution_and_checks_arguments() {
    compile_all("interface Label { fun text():String }\nclass Named:Label { override fun text():String = \"OK\" }\ninterface Bounded<T:Label> { fun get():T }\nclass Holder<T:Label>(val value:T):Bounded<T> { override fun get():T=value; fun text():String=value.text() }\nfun use():String=Holder<Named>(Named()).text()", "Bounds.kt").unwrap();
    for source in [
        "interface Label\nclass Holder<T:Label>(val value:T)\nfun bad():Holder<String> = Holder<String>(\"bad\")",
        "class Holder<T:CharSequence>(val value:T)\nfun bad():Holder<String?> = Holder<String?>(null)",
    ] {
        let error = compile_all(source, "Bounds.kt").err().unwrap();
        assert!(error.message.contains("upper bound"), "{error:?}");
    }
}

#[test]
fn private_variance_and_recursive_bounds_remain_explicit_unsupported() {
    for (source, code) in [
        (
            "class Holder<out T>(private var value:T)",
            "PRIVATE_VARIANCE_UNSUPPORTED",
        ),
        (
            "interface Rec<T:Comparable<T>>",
            "DEPENDENT_GENERIC_BOUND_UNSUPPORTED",
        ),
    ] {
        let error = compile_all(source, "Bounds.kt").err().unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage, "{error:?}");
        assert_eq!(error.code, code, "{error:?}");
    }
}

#[test]
fn exposed_variant_inputs_requiring_contextual_wildcards_are_explicit_boundaries() {
    for source in [
        "interface P<out T>\nfun use(value:P<Any>):Int=1",
        "interface C<in T>\nfun use(value:C<String>):Int=1",
        "interface P<out T>\nfun use(value:P<out String>):Int=1",
        "interface P<out T>\nfun value(p:P<String>):P<out String> = p",
        "interface P<out T>\ninterface Api { val value:P<out String> }",
        "interface P<out T>\nclass Box<out T>(val value:T):P<T>\nclass Api { val value:P<out String> get()=Box<String>(\"x\") }",
        "interface P<out T>\nclass Holder(val value:P<Any>)",
        "interface P<out T>\ninterface Api { fun use(value:P<Any>):Int }",
        "interface P<out T>\nenum class Api { A; fun use(value:P<Any>):Int=1 }",
    ] {
        let error = compile_all(source, "Boundary.kt").err().unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {error:?}"
        );
        assert_eq!(
            error.code, "VARIANT_JVM_SIGNATURE_UNSUPPORTED",
            "{source}: {error:?}"
        );
    }
    compile_all("interface P<out T>\ninterface C<in T>\nfun use(value:P<String>):P<Any> = value\nfun useInput(value:C<Any>):C<String> = value", "Exact.kt").unwrap();
}
