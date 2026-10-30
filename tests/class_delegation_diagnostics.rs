use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn generic_interface_delegation_builds_forwarders_properties_and_bridges() {
    let source = "interface I<T>{fun echo(value:T):T;var value:T}\nclass Generic<T>(delegate:I<T>):I<T> by delegate\nclass Scalar(delegate:I<Int>):I<Int> by delegate";
    let outputs = compile_outputs_with_classpath(source, "Delegation.kt", &classpath()).unwrap();
    for name in ["I.class", "Generic.class", "Scalar.class"] {
        assert!(outputs.iter().any(|o| o.relative_path == name));
    }
}

#[test]
fn invalid_delegation_contracts_do_not_return_artifacts() {
    for (source, code) in [
        (
            "interface I\nclass C:I by \"bad\"",
            "INTERFACE_DELEGATION_VALUE_INVALID",
        ),
        (
            "class I\nclass C(value:I):I by value",
            "INTERFACE_DELEGATION_TYPE_INVALID",
        ),
        (
            "interface I{fun f():Int}\ninterface J:I\nclass C(a:I,b:J):I by a,J by b",
            "INTERFACE_DELEGATION_CONFLICT_INVALID",
        ),
        (
            "interface I{fun f():Int}\ninterface J{fun g():Int}\nclass C(a:I):I by a,J",
            "MISSING_INTERFACE_IMPLEMENTATION",
        ),
        (
            "internal interface I{fun f():Int}\nclass C(value:I):I by value",
            "DELEGATION_EXPOSED_INTERNAL_TYPE",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Delegation.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{}",
            error.message
        );
        assert_eq!(error.code, code, "{}", error.message);
    }
}

#[test]
fn valid_unimplemented_delegation_shapes_have_explicit_boundaries() {
    for (source, code) in [
        (
            "interface I{fun f():Int=1}\nclass C(value:I):I by value",
            "INTERFACE_DELEGATION_DEFAULT_UNSUPPORTED",
        ),
        (
            "interface I{fun <T> f(value:T):T}\nclass C(value:I):I by value",
            "INTERFACE_DELEGATION_MEMBER_UNSUPPORTED",
        ),
        (
            "interface I\ndata class C(val value:I):I by value",
            "INTERFACE_DELEGATION_CLASS_SHAPE_UNSUPPORTED",
        ),
        (
            "internal interface I{fun f():Int}\ninternal class C(value:I):I by value",
            "INTERFACE_DELEGATION_INTERNAL_UNSUPPORTED",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Delegation.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{}",
            error.message
        );
        assert_eq!(error.code, code, "{}", error.message);
    }
}

#[test]
fn delegation_cannot_turn_a_var_override_into_a_val_or_hide_contract_members() {
    for (source, code) in [
        (
            "interface I{var value:Int}\nclass C(d:I):I by d{override val value:Int=1}",
            "VAR_OVERRIDDEN_BY_VAL",
        ),
        (
            "interface I{var value:Int}\nclass C(d:I,override val value:Int):I by d",
            "VAR_OVERRIDDEN_BY_VAL",
        ),
        (
            "interface I{fun f():Int}\nclass C(d:I):I by d{fun f():Int=1}",
            "OVERRIDE_REQUIRED",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Delegation.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{}",
            error.message
        );
        assert_eq!(error.code, code, "{}", error.message);
    }
}
