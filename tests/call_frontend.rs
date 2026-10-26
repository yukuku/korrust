use korrust::{compile_all_with_classpath, compile_outputs_with_classpath};
use std::path::PathBuf;

#[test]
fn source_and_library_varargs_preserve_array_contracts() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !standard.is_file() {
        return;
    }
    for source in [
        "fun sum(vararg values: Int): Int { var total = 0; for(value in values) total += value; return total }\nfun use(): Int = sum() + sum(1, 2, 3)",
        "fun use(input: IntArray): IntArray = intArrayOf(1, *input, 2)",
        "fun use(): Array<Int> = arrayOf(1, 2)",
        "fun use(): List<Byte> = listOf<Byte>(-128, 127)",
        "fun use(): ByteArray = byteArrayOf(1, 2)",
        "fun use(): CharArray = charArrayOf('a', 'b')",
        "fun preserved(vararg values: Int): IntArray = values\nfun use(input: IntArray): IntArray = preserved(values = input)",
        "fun intArrayOf(value: Int): IntArray { val result = IntArray(1); result[0] = value + 100; return result }\nfun use(): IntArray = intArrayOf(3)",
        "fun use(): Int? = 3.let { if(it == 0) null else it }",
    ] {
        let result =
            compile_all_with_classpath(source, "Calls.kt", std::slice::from_ref(&standard));
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    for source in [
        "fun sum(vararg values: Int): Int = values.size\nfun use(): Int = sum(\"wrong\")",
        "fun use(input: Array<String>): IntArray = intArrayOf(*input)",
        "fun bad(): List<Byte> = listOf<Byte>(128)",
        "fun bad(): List<Byte> = listOf<Byte>(-129)",
        "fun use(input: IntArray?): IntArray = intArrayOf(*input)",
        "fun bad(vararg values: String) { values[0] = \"invalid\" }",
    ] {
        assert!(
            compile_all_with_classpath(source, "Rejected.kt", std::slice::from_ref(&standard))
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn same_package_compiled_classes_resolve_from_directories_and_jars() {
    let directory = std::env::temp_dir().join(format!(
        "korrust-package-default-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let outputs = compile_outputs_with_classpath(
        "package own.fixture\nclass Widget(val number: Int)",
        "Widget.kt",
        &[],
    )
    .unwrap();
    let jar = directory.join("dependency.jar");
    let mut archive = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
    for output in outputs {
        let path = directory.join(&output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &output.bytes).unwrap();
        archive
            .start_file(output.relative_path, zip::write::FileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, &output.bytes).unwrap();
    }
    archive.finish().unwrap();
    for dependency in [&directory, &jar] {
        let result = compile_all_with_classpath(
            "package own.fixture\nfun make(): Widget = Widget(42)",
            "Consumer.kt",
            std::slice::from_ref(dependency),
        );
        assert!(result.is_ok(), "{:?}", result.err());
    }
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn projected_builtin_collections_resolve_their_jvm_classifier() {
    let source =
        "package sample\nfun passthrough(value: List<out String>): List<out String> = value";
    assert!(compile_all_with_classpath(source, "Projected.kt", &[]).is_ok());
    assert!(
        compile_all_with_classpath(
            "fun bad(values: MutableList<out String>) { values.add(\"x\") }",
            "BadProjection.kt",
            &[]
        )
        .is_err()
    );
}

#[test]
fn builtin_array_metadata_resolves_identically_from_jar_and_directory() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !standard.is_file() {
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "korrust-builtin-resource-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let resource = "kotlin/kotlin.kotlin_builtins";
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&standard).unwrap()).unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut archive.by_name(resource).unwrap(), &mut bytes).unwrap();
    std::fs::create_dir_all(directory.join("kotlin")).unwrap();
    std::fs::write(directory.join(resource), bytes).unwrap();
    let source = "fun values(): IntArray = intArrayOf(1, 2, 3)";
    let from_jar = compile_all_with_classpath(source, "Builtin.kt", &[standard]).unwrap();
    let from_directory =
        compile_all_with_classpath(source, "Builtin.kt", std::slice::from_ref(&directory)).unwrap();
    assert_eq!(from_jar[0].bytes, from_directory[0].bytes);
    std::fs::remove_dir_all(directory).unwrap();
}
