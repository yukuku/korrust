use korrust::{DiagnosticKind, compile_all_with_classpath};
use std::path::PathBuf;

fn standard() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn suspend_interface_and_final_member_bodies_keep_real_continuations() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    let classes = compile_all_with_classpath(
        "package sample\nfun interface Boundary { suspend fun read(value: String): String?; suspend fun retry(value: String): String? = read(value) }\nclass Reader(val prefix: String, val step: suspend (String) -> String): Boundary { override suspend fun read(value: String): String { val result = step(value); return prefix + result } }\nsuspend fun dispatch(reader: Boundary, value: String): String? = reader.retry(value)",
        "Members.kt", std::slice::from_ref(&standard)).unwrap();
    for owner in [
        "sample/Boundary",
        "sample/Reader",
        "sample/Boundary$DefaultImpls$suspend$1",
        "sample/Reader$suspend$0",
    ] {
        assert!(
            classes.iter().any(|class| class.internal_name == owner),
            "missing {owner}: {:?}",
            classes
                .iter()
                .map(|class| &class.internal_name)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn unsupported_suspend_member_shapes_and_control_flow_reject_explicitly() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "class C { private suspend fun read(): String = \"x\" }",
        "open class C { open suspend fun read(): String = \"x\" }",
        "class C { suspend fun <T> read(value: T): T = value }",
        "class C { suspend fun read(value: String = \"x\"): String = value }",
        "class C { suspend fun read(step: suspend () -> String): String = if (true) step() else \"x\" }",
    ] {
        let error =
            compile_all_with_classpath(source, "Unsupported.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {}",
            error.message
        );
    }
}

#[test]
fn ordinary_member_callers_cannot_invoke_suspend_members() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "interface R { suspend fun read(): String }; fun broken(reader: R): String = reader.read()",
        "class R { suspend fun read(): String = \"x\"; fun broken(): String = read() }",
        "interface R { suspend fun read(): String; fun broken(): String = read() }",
        "interface R { suspend fun read(): String }; class C: R { override fun read(): String = \"x\" }",
        "interface R { fun read(): String }; class C: R { override suspend fun read(): String = \"x\" }",
    ] {
        let error =
            compile_all_with_classpath(source, "Invalid.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::InvalidKotlin,
            "{source}: {}",
            error.message
        );
    }
}

#[test]
fn suspend_custom_parameter_annotations_reject_before_descriptor_rewriting() {
    let standard = standard();
    if !standard.is_file() {
        return;
    }
    for source in [
        "annotation class Mark; suspend fun read(@Mark value: String): String = value",
        "annotation class Mark; class C { suspend fun read(@Mark value: String): String = value }",
        "annotation class Mark; interface C { suspend fun read(@Mark value: String): String }",
        "annotation class Mark; abstract class C { abstract suspend fun read(@Mark value: String): String }",
    ] {
        let error =
            compile_all_with_classpath(source, "Annotated.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(
            error.kind,
            DiagnosticKind::UnsupportedLanguage,
            "{source}: {}",
            error.message
        );
        assert_eq!(
            error.code, "SUSPEND_PARAMETER_ANNOTATION_UNSUPPORTED",
            "{source}: {}",
            error.message
        );
    }
}
