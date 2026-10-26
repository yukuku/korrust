use korrust::{compile, compile_with_classpath};
use std::path::PathBuf;

fn rejects(source: &str, fragment: &str) {
    let error = compile(source, "Boundary.kt").err().expect(source);
    assert!(
        error.message.contains(fragment),
        "{}: {}",
        source,
        error.message
    );
}

#[test]
fn rejects_erased_type_tests_and_casts() {
    for op in ["is", "!is", "as", "as?"] {
        rejects(
            &format!("fun f(value: Any?) = value {op} List<String>"),
            "erased generic",
        );
    }
    rejects(
        "fun f(value: String): Boolean = value is Int",
        "Incompatible types",
    );
    rejects(
        "fun f(value: Int): Boolean = value is Long",
        "Incompatible types",
    );
}

#[test]
fn type_smartcasts_require_stable_locals_and_correct_branches() {
    for source in [
        "fun f(value: Any?): Int { if (value is String) return value.length; return 0 }",
        "fun f(value: Any?): Int { if (value !is String) return 0; return value.length }",
        "fun f(value: Any?): Int = when(value) { is String -> value.length; else -> 0 }",
        "fun f(value: Any?): Int = when { value is String -> value.length; else -> 0 }",
        "fun f(value: Any?): Int = when(true) { value is String -> value.length; else -> 0 }",
        "fun f(value: Any?): Int { while(value is String) { return value.length }; return 0 }",
    ] {
        assert!(compile(source, "Smartcast.kt").is_ok(), "{source}");
    }
    rejects(
        "fun f(input: Any?): Int { var value = input; if(value is String) return value.length; return 0 }",
        "Nullable receivers",
    );
    rejects(
        "fun f(value: Any?): Int { if(value !is String) return value.length; return 0 }",
        "Nullable receivers",
    );
    rejects(
        "fun f(value: Any?): Int = when(value) { is String, is Int -> value.length; else -> 0 }",
        "Nullable receivers",
    );
}

#[test]
fn nullable_and_boxed_type_operations_compile() {
    for source in [
        "fun f(value: Any?): Boolean = value is String?",
        "fun f(value: String?): Boolean = value is Int?",
        "fun f(value: Any?): String? = value as String?",
        "fun f(value: Any?): Int = value as Int",
        "fun f(value: Any?): Int? = value as? Int",
        "fun f(value: String?): Boolean = value === null",
        "fun f(a: Float?, b: Float?): Boolean = a == b",
        "fun f(a: Any?, b: Any?): Boolean = a != b",
    ] {
        assert!(compile(source, "Types.kt").is_ok(), "{source}");
    }
    rejects(
        "fun f(a: String?, b: Int?): Boolean = a === b",
        "incompatible types",
    );
    rejects(
        "fun f(a: Int, b: Int): Boolean = a === b",
        "Reference identity",
    );
    rejects(
        "fun f(a: String, b: Int): Boolean = a == b",
        "incompatible types",
    );
}

#[test]
fn numeric_operations_preserve_assignment_and_comparison_types() {
    for source in [
        "fun f(a: Float, b: Int): Float = a + b",
        "fun f(a: Long, b: Float): Boolean = a <= b",
        "fun f(a: Long): Float = a.toFloat()",
        "fun f(a: Float): Long = a.toLong()",
        "fun f(a: Float): Float { var n = a; n += 1; n -= 2.0f; n *= 3.0f; n /= 2.0f; n %= 5.0f; return n }",
        "fun f(a: Long): Long { var n = a; n -= 2L; n *= 3L; n /= 2L; n %= 5L; return n }",
        "fun f(): Int { var n = 3; val old = n--; return old + --n }",
    ] {
        assert!(compile(source, "Numeric.kt").is_ok(), "{source}");
    }
    rejects(
        "fun f(): Int { var n = 1; n += 1.0f; return n }",
        "Type mismatch",
    );
    rejects("fun f(a: Float, b: Int): Boolean = a == b", "Type mismatch");
    rejects("fun f(a: Float): Float = a and a", "Float operation");
    rejects(
        "fun f(): Int { val n = 1; return n-- }",
        "initialized mutable Int",
    );
}

#[test]
fn loop_scopes_and_continue_initialization_are_checked() {
    assert!(
        compile(
            "fun f(): Int { var n = 0; do { n += 1; if(n < 2) continue } while(n < 3); return n }",
            "Loop.kt"
        )
        .is_ok()
    );
    assert!(
        compile(
            "fun f(): Int { do { val n = 1 } while(n < 0); return 0 }",
            "Loop.kt"
        )
        .is_ok()
    );
    rejects(
        "fun f(flag: Boolean): Int { do { if(flag) continue; val n = 1 } while(n < 2); return 0 }",
        "before any continue",
    );
    rejects(
        "fun f(): Int { for(n in IntArray(2)) { n = 2 }; return 0 }",
        "Cannot reassign",
    );
    rejects(
        "fun f(): Int { for(n in IntArray(2)) {}; return n }",
        "Unresolved local",
    );
    rejects(
        "fun f(a: IntArray?) { for(n in a) {} }",
        "Iteration supports",
    );
}

#[test]
fn generic_iterable_element_types_follow_the_declared_signature() {
    for source in [
        "fun f(values: Iterable<String>): Int { var n = 0; for(value in values) { n += value.length }; return n }",
        "fun f(values: Iterable<Long>): Long { var n = 0L; for(value in values) { n += value }; return n }",
        "fun f(values: Iterable<String?>): Int { var n = 0; for(value in values) { if(value != null) n += value.length }; return n }",
        "object Adder { infix fun combine(other: Int): Int = other + 1; fun f(): Int = this combine 2 }",
    ] {
        assert!(compile(source, "Iterable.kt").is_ok(), "{source}");
    }
}

#[test]
fn generic_infix_calls_and_runtime_iteration_use_resolved_declarations() {
    rejects("fun f() { for(n in 2 downTo 1) {} }", "infix extension");
    rejects(
        "fun f(a: String) { for(n in a.indices) {} }",
        "Unsupported property",
    );
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !standard.is_file() {
        return;
    }
    let classpath = [standard];
    for source in [
        "fun f(a: String) { for(n in a.indices step 2) { a[n] } }",
        "fun f(a: IntArray) { for(n in a.indices step 2) {} }",
        "fun f(a: List<String>) { for(n in a.indices step 2) {} }",
        "fun f() { for(n in 3 downTo 1 step 2) {} }",
        "fun f() { for(n in 1 until 4 step 2) {} }",
    ] {
        let result = compile_with_classpath(source, "Ranges.kt", &classpath);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    for source in [
        "infix fun Int.downTo(other: Int): Int = this + other\nfun f(): Int = 3 downTo 1",
        "infix fun Int.combine(other: Int): Int = this + other\nfun f(): Int = 3 combine 1",
        "fun Int.combine(other: Int): Int = this + other\nfun f(): Int = 3.combine(1)",
    ] {
        assert!(
            compile_with_classpath(source, "Custom.kt", &classpath).is_ok(),
            "{source}"
        );
    }
    rejects(
        "fun Int.combine(other: Int): Int = this + other\nfun f(): Int = 3 combine 1",
        "infix modifier",
    );
}

#[test]
fn member_extension_fallback_preserves_diagnostics_and_selects_source_extensions() {
    for (body, message) in [
        ("value.missing()", "Unresolved member or imported extension"),
        ("value.addAndGet()", "argument"),
        ("value.addAndGet(true)", "Type mismatch"),
    ] {
        rejects(
            &format!(
                "import java.util.concurrent.atomic.AtomicInteger\nfun f(value: AtomicInteger): Int = {body}"
            ),
            message,
        );
    }
    rejects(
        "import java.lang.StringBuilder\nfun f(value: StringBuilder): Int = value.newCapacity(1)",
        "Unresolved member or imported extension",
    );
    for extension in ["addAndGet", "custom"] {
        let source = format!(
            "import java.util.concurrent.atomic.AtomicInteger\nfun AtomicInteger.{extension}(value: String): Int = value.length\nfun f(value: AtomicInteger): Int = value.{extension}(\"ok\")"
        );
        let result = compile(&source, "Fallback.kt");
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    rejects(
        "import java.util.concurrent.atomic.AtomicInteger\nfun AtomicInteger.addAndGet(value: String): Int = value.length\nfun f(value: AtomicInteger): Int = value.addAndGet(true)",
        "Type mismatch",
    );
    rejects(
        "import java.util.concurrent.atomic.AtomicInteger\nimport java.io.Serializable\nfun AtomicInteger.addAndGet(value: Serializable): Int = 1\nfun AtomicInteger.addAndGet(value: CharSequence): Int = 2\nfun f(value: AtomicInteger): Int = value.addAndGet(\"ok\")",
        "Ambiguous",
    );
}

#[test]
fn renamed_imports_hide_original_names_and_keep_explicit_original_imports() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !standard.is_file() {
        return;
    }
    let classpath = [standard];
    for imports in [
        "import kotlin.text.*\nimport kotlin.text.indices as positions",
        "import kotlin.text.indices as positions\nimport kotlin.text.*",
        "import kotlin.text.indices as positions",
    ] {
        let accepted = format!(
            "{imports}\nfun f(value: String): Int {{ var count = 0; for(index in value.positions) {{ count += index }}; return count }}"
        );
        assert!(
            compile_with_classpath(&accepted, "Alias.kt", &classpath).is_ok(),
            "{accepted}"
        );
        let rejected = format!(
            "{imports}\nfun f(value: String): Int {{ var count = 0; for(index in value.indices) {{ count += index }}; return count }}"
        );
        let error = compile_with_classpath(&rejected, "Alias.kt", &classpath)
            .err()
            .expect("renamed property original must be hidden");
        assert!(error.message.contains("Unresolved"), "{}", error.message);
    }
    for source in [
        "import kotlin.text.indices as positions\nimport kotlin.text.indices\nfun f(value: String): Int { var count = 0; for(index in value.indices) { count += index }; for(index in value.positions) { count += index }; return count }",
        "import kotlin.text.indices as positions\nfun f(value: IntArray): Int { var count = 0; for(index in value.indices) { count += index }; return count }",
        "import kotlin.ranges.downTo as descend\nimport kotlin.ranges.downTo\nfun f(): Int { var count = 0; for(value in 3 descend 1) { count += value }; for(value in 3 downTo 1) { count += value }; return count }",
    ] {
        let result = compile_with_classpath(source, "Alias.kt", &classpath);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    let outputs = korrust::compile_all(
        "package alias.library\ninfix fun Int.measure(other: Int): Int = this + other",
        "Library.kt",
    )
    .unwrap();
    let directory = std::env::temp_dir().join(format!(
        "korrust-alias-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for output in outputs {
        let path = directory.join(format!("{}.class", output.internal_name));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let aliased = "package alias.library\nimport alias.library.measure as counted\nfun f(): Int = 2 counted 3";
    let original = "package alias.library\nimport alias.library.measure as counted\nfun f(): Int = 2 measure 3";
    let accepted = compile_with_classpath(aliased, "Alias.kt", std::slice::from_ref(&directory));
    let rejected = compile_with_classpath(original, "Alias.kt", std::slice::from_ref(&directory));
    std::fs::remove_dir_all(directory).unwrap();
    assert!(accepted.is_ok(), "{:?}", accepted.err());
    assert!(
        rejected
            .err()
            .expect("same-package renamed original must be hidden")
            .message
            .contains("Unresolved renamed declaration")
    );
}

#[test]
fn imported_generic_results_use_constructor_and_function_expected_types() {
    let directory = std::env::temp_dir().join(format!(
        "korrust-expected-result-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for output in korrust::compile_outputs_with_classpath(
        "package inference.library\nfun <T> fabricate(): List<T> = emptyList<T>()\nfun <T> identity(value: T): T = value\nfun <T> nullable(): T? = null\nfun <T> nullableList(): List<T>? = null",
        "GenericLibrary.kt",
        &[],
    ).unwrap() {
        let path = directory.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let imports = "import inference.library.fabricate\nimport inference.library.identity\nimport inference.library.nullable as inferenceNullable\nimport inference.library.nullableList as inferenceNullableList\n";
    for source in [
        "data class Snapshot(val logs: List<String>) { companion object { val EMPTY = Snapshot(logs = fabricate()) } }",
        "fun use(): List<Int?> = fabricate()",
        "fun use(): List<String>? = fabricate()",
        "fun use(): List<Int?>? = fabricate()",
        "fun use(): List<String>? = inferenceNullableList()",
        "fun use(): String? = inferenceNullable()",
        "fun take(value: List<Long>): Int = value.size\nfun use(): Int = take(fabricate())",
        "fun use(): Any = identity(\"text\")",
        "fun use(fabricate: () -> Int): Int = fabricate()",
        "fun use(): Int { val fabricate: () -> Int = { 9 }; return fabricate() }",
        "fun use(): Int { fun fabricate(): Int = 9; return fabricate() }",
    ] {
        let result = korrust::compile_all_with_classpath(
            &format!("{imports}{source}"),
            "Expected.kt",
            std::slice::from_ref(&directory),
        );
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    for source in [
        "fun use(): List<String> = fabricate<Int>()",
        "data class Snapshot(val logs: List<String>) { companion object { val EMPTY = Snapshot(logs = fabricate<Int>()) } }",
        "fun use(): Int = identity(\"text\")",
        "fun fabricate(): Int = 7\nfun use(): Int = fabricate()",
        "fun use(): String = inferenceNullable()",
        "fun use(): List<String> = inferenceNullableList()",
        "val first = second\nval second = first",
    ] {
        assert!(
            korrust::compile_all_with_classpath(
                &format!("{imports}{source}"),
                "Rejected.kt",
                std::slice::from_ref(&directory)
            )
            .is_err(),
            "{source}"
        );
    }
    std::fs::remove_dir_all(directory).unwrap();
}
