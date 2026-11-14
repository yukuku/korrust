use korrust::{DiagnosticKind, compile_all_with_classpath};
use std::path::PathBuf;

fn check(source: &str) -> Result<Vec<korrust::ClassFile>, korrust::Diagnostic> {
    compile_all_with_classpath(
        source,
        "Shorts.kt",
        &[PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar")],
    )
}

#[test]
fn integral_operations_promote_without_changing_declared_types() {
    for source in [
        "fun sum(a: Short, b: Short): Int = a + b",
        "fun sum(a: Byte, b: Short): Int = a + b",
        "fun unary(a: Short): Int = -a",
        "fun mixed(a: Int, b: Long): Long = a + b",
        "fun mixed(a: Short, b: Long): Long = a / b",
        "fun ordered(a: Short, b: Long): Boolean = a < b",
        "fun ordered(a: Int, b: Long): Boolean = a >= b",
    ] {
        assert!(check(source).is_ok(), "{source}: {:?}", check(source).err());
    }
    for source in [
        "fun bad(a: Short, b: Short): Short = a + b",
        "fun bad(a: Int): Short = a",
        "fun bad(): Short = 32768",
        "fun bad(): Short = -32769",
        "fun bad(a: Short, b: Int): Boolean = a == b",
        "fun bad(a: Short): Int = a shl 1",
        "fun bad(a: Int, b: Long): Int = a + b",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
}

#[test]
fn short_arrays_use_primitive_representation_and_contextual_elements() {
    for source in [
        "fun make(n: Int): ShortArray = ShortArray(n) { it.toShort() }",
        "fun pack(vararg values: Short): ShortArray = values; fun copy(v: ShortArray): ShortArray = pack(-32768, *v, 32767)",
        "fun f(v: ShortArray): Int { v[0] = -32768; var n = 0; for (x in v) { n = n + x }; return n }",
        "fun f(): ShortArray = shortArrayOf(-32768, 32767)",
    ] {
        assert!(check(source).is_ok(), "{source}: {:?}", check(source).err());
    }
    for source in [
        "fun bad(v: ShortArray) { v[0] = 32768 }",
        "fun bad(v: ShortArray, n: Int) { v[0] = n }",
        "fun bad(v: ShortArray): Array<Short> = v",
        "fun bad(v: Array<Short>): ShortArray = v",
        "fun pack(vararg v: Short) {}; fun bad(v: IntArray) { pack(*v) }",
    ] {
        assert!(check(source).is_err(), "{source}");
    }
    let error =
        check("fun init(): (Int) -> Short = { it.toShort() }; fun make() = ShortArray(2, init())")
            .err()
            .unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "ARRAY_INITIALIZER_EVALUATION_UNSUPPORTED");
}
