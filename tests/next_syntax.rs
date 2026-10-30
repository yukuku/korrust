use korrust::{compile, compile_with_classpath};
use std::path::PathBuf;

fn accepts(source: &str) {
    compile(source, "NextSyntax.kt").unwrap_or_else(|error| panic!("{}\n{source}", error.message));
}

fn rejects(source: &str, message: &str) {
    let error = compile(source, "NextSyntax.kt")
        .err()
        .expect("expected rejection");
    assert!(error.message.contains(message), "{}", error.message);
}

#[test]
fn accepts_numeric_updates_and_do_while() {
    accepts(
        "fun f(): Int { var x = 7; do { x -= 1; if (x == 4) { continue }; x *= 2; x /= 2; x %= 7 } while (x > 2); return --x + x-- }",
    );
    accepts("fun f(): Long { var x = 8L; x -= 2L; x *= 3L; x /= 2L; x %= 5L; return x }");
    accepts(
        "fun f(): Float { var x = 8.0f; x -= 2.0f; x *= 3.0f; x /= 2.0f; x %= 5.0f; return x }",
    );
}

#[test]
fn accepts_iteration_over_supported_values() {
    for (ty, expression) in [
        ("IntArray", "n"),
        ("ByteArray", "n.toInt()"),
        ("CharArray", "n.code"),
        ("String", "n.code"),
    ] {
        accepts(&format!(
            "fun f(a: {ty}): Int {{ var sum = 0; for (n in a) {{ sum += {expression} }}; return sum }}"
        ));
    }
    accepts("fun f(a: List<Int>): Int { var sum = 0; for (n in a) { sum += n }; return sum }");
}

#[test]
fn accepts_stdlib_progressions_with_an_explicit_classpath() {
    let stdlib =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !stdlib.is_file() {
        return;
    }
    for source in [
        "fun f(): Int { var sum = 0; for (n in 1..3) { sum += n }; return sum }",
        "fun f(): Int { var sum = 0; for (n in 5 downTo 1 step 2) { sum += n }; return sum }",
        "fun f(a: IntArray): Int { var sum = 0; for (n in a.indices) { sum += a[n] }; return sum }",
    ] {
        compile_with_classpath(source, "NextSyntax.kt", std::slice::from_ref(&stdlib))
            .unwrap_or_else(|error| panic!("{}\n{source}", error.message));
    }
}

#[test]
fn accepts_identity_casts_and_type_conditions() {
    accepts("fun f(a: String?, b: String?): Boolean = a === b || a !== b");
    accepts("fun f(a: Any?): String? = a as? String");
    accepts("fun f(a: Any?): String = a as String");
    accepts("fun f(a: Any?): Int = when (a) { is String -> a.length; !is Int -> 2; else -> 3 }");
    accepts("fun f(a: Any?): Int = when { a is String -> a.length; else -> 0 }");
    accepts("fun f(a: Any?): Int = when (true) { (a is String) -> 1; else -> 0 }");
}

#[test]
fn rejects_unsupported_update_targets_and_invalid_type_tokens() {
    rejects(
        "fun f(): Int { val n = 1; n -= 1; return n }",
        "Cannot reassign",
    );
    rejects(
        "fun f(a: IntArray): Unit { a[0] *= 2 }",
        "local variable target",
    );
    rejects("fun f(a: IntArray): Int = a[0]--", "mutable Int local");
    rejects(
        "fun f(a: Any): Boolean = a ! is String",
        "newline or semicolon",
    );
    rejects("fun f(a: Any): String? = a as ? String", "identifier");
    rejects(
        "fun f(): Int = when { is Int -> 1; else -> 2 }",
        "require a when subject",
    );
}

#[test]
fn source_infix_calls_follow_the_declaration_modifier() {
    accepts(
        "infix fun Int.combine(value: Int): Int = this * 10 + value\nfun f(): Int = 2 combine 3",
    );
    accepts("infix fun Int.downTo(value: Int): Int = this + value\nfun f(): Int = 2 downTo 3");
    accepts(
        "class C(val value: Int) { public infix fun combine(other: Int): Int = value + other; fun f(): Int = this combine 3 }",
    );
    for source in [
        "fun Int.combine(value: Int): Int = this + value\nfun f(): Int = 2 combine 3",
        "infix fun combine(value: Int): Int = value",
        "infix fun Int.combine(a: Int, b: Int): Int = a + b",
        "infix fun Int.combine(value: Int = 1): Int = value",
    ] {
        assert!(
            compile(source, "NextSyntax.kt").is_err(),
            "unexpected acceptance: {source}"
        );
    }
    accepts("fun Int.combine(value: Int): Int = this + value\nfun f(): Int = 2.combine(3)");
}
