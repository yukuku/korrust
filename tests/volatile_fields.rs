use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;

#[test]
fn volatile_annotations_require_mutable_backing_fields() {
    let stdlib =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    let cp = [stdlib];
    for source in [
        "@Volatile val value: Int = 1",
        "class Box { companion object { @Volatile val value: Int = 1 } }",
        "class Box { @get:Volatile var value: Int = 1 }",
    ] {
        assert!(compile_outputs_with_classpath(source, "Invalid.kt", &cp).is_err());
    }
    compile_outputs_with_classpath(
        "class Box { companion object { @Volatile private var instance: Box? = null } }",
        "Valid.kt",
        &cp,
    )
    .unwrap();
}
