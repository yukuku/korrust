use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;
fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}
#[test]
fn reference_templates_accept_objects_nullable_values_and_array_references() {
    for source in [
        "fun render(v: Any?): String = \"[$v]\"",
        "fun render(v: CharArray): String = \"$v\"",
        "fun render(v: IntArray): String = \"$v\"",
        "fun render(v: Array<String>): String = \"$v\"",
    ] {
        compile_outputs_with_classpath(source, "Template.kt", &classpath()).unwrap();
    }
}
#[test]
fn compiled_inline_body_accepts_object_virtual_method_references() {
    compile_outputs_with_classpath(
        "fun clipped(s: String): String = s.trim()",
        "Trim.kt",
        &classpath(),
    )
    .unwrap();
}
