use korrust::{DiagnosticKind, compile_all_with_classpath};
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn remaining_primitive_array_representations_compile_consistently() {
    for source in [
        "fun make(n: Int): LongArray = LongArray(n) { it.toLong() }",
        "fun make(n: Int): BooleanArray = BooleanArray(n) { it % 2 == 0 }",
        "fun make(n: Int): FloatArray = FloatArray(n) { it.toFloat() }",
        "fun make(n: Int): DoubleArray = DoubleArray(n) { it.toDouble() }",
        "fun pack(vararg values: Double): DoubleArray = values; fun copy(values: DoubleArray): DoubleArray = pack(1.0, *values)",
        "fun make(): LongArray = longArrayOf(1L, 2L)",
    ] {
        let result = compile_all_with_classpath(source, "PrimitiveArrays.kt", &classpath());
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn primitive_arrays_are_distinct_from_boxed_arrays_and_each_other() {
    for source in [
        "fun bad(values: LongArray): Array<Long> = values",
        "fun bad(values: Array<Long>): LongArray = values",
        "fun bad(values: BooleanArray): ByteArray = values",
        "fun bad(values: DoubleArray) { values[0] = 1 }",
        "fun bad(values: LongArray, value: Int) { values[0] = value }",
        "fun bad(values: BooleanArray) { values[0] = 1 }",
        "fun bad(values: DoubleArray?): Double = values[0]",
        "fun pack(vararg values: Long): LongArray = values; fun bad(value: IntArray): LongArray = pack(*value)",
    ] {
        assert!(
            compile_all_with_classpath(source, "InvalidArrays.kt", &classpath()).is_err(),
            "{source}"
        );
    }
}

#[test]
fn short_and_effectful_initializer_boundaries_are_explicit() {
    for source in [
        "fun make() = ShortArray(1)",
        "fun pass(value: Short): Int = 1",
    ] {
        let error = compile_all_with_classpath(source, "ShortBoundary.kt", &classpath())
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "SHORT_UNSUPPORTED");
    }
    let error = compile_all_with_classpath(
        "fun initializer(): (Int) -> Double = { it.toDouble() }; fun make() = DoubleArray(2, initializer())",
        "InitializerBoundary.kt",
        &classpath(),
    ).err().unwrap();
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "ARRAY_INITIALIZER_EVALUATION_UNSUPPORTED");
}
