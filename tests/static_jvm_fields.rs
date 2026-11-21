use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::path::PathBuf;
#[test]
fn static_jvm_fields_keep_storage_and_accessor_boundaries() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(check_fields)
        .unwrap()
        .join()
        .unwrap();
}
fn check_fields() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/static_jvm_fields");
    let cp = [root.join("references/kotlinc/lib/kotlin-stdlib.jar")];
    let sources = ["Models.kt", "Read.kt", "Extra.kt", "Shadow.kt"].map(|name| SourceInput {
        identity: name.into(),
        file_name: name.into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    });
    compile_batch_with_classpath(&sources, &cp, "staticfields").unwrap();
    for entry in std::fs::read_dir(&fixtures).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy();
        if name.starts_with("Invalid") || name.starts_with("Unsupported") {
            let source = std::fs::read_to_string(&path).unwrap();
            assert!(
                compile_outputs_with_classpath(&source, &name, &cp).is_err(),
                "{name}"
            );
        }
    }
}
