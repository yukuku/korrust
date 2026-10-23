use korrust::compile;

fn rejects(source: &str, message: &str) {
    let error = compile(source, "Test.kt")
        .err()
        .expect("expected rejection");
    assert!(error.message.contains(message), "{}", error.message);
}

#[test]
fn rejects_invalid_bindings_and_types() {
    for (source, message) in [
        (
            "fun f(): Int { val x = 1; x = 2; return x }",
            "Cannot reassign",
        ),
        ("fun f(x: Int): Int { x = 2; return x }", "Cannot reassign"),
        (
            "fun f(): Int { val x = 1; val x = 2; return x }",
            "Conflicting",
        ),
        ("fun f(): Int = missing", "Unresolved"),
        ("fun f(): Int = true + 1", "Type mismatch"),
        ("fun f(): Boolean = 1", "Type mismatch"),
        (
            "fun f(): Int { if (1) { return 1 }; return 2 }",
            "Type mismatch",
        ),
        (
            "fun f(): Int { while (false) { return 1 } }",
            "return on all paths",
        ),
        (
            "fun f(x: Int): Int = x\nfun g(): Int = f()",
            "argument count",
        ),
        (
            "fun f(): Int { if (true) { val x = 1 }; return x }",
            "Unresolved",
        ),
        (
            "fun f(Int: Int): Int = Int.MIN_VALUE",
            "Unsupported property",
        ),
    ] {
        rejects(source, message);
    }
}

#[test]
fn preserves_statement_boundaries_and_rejects_unsupported_syntax() {
    rejects(
        "fun f(): Int { val x = 1 val y = 2; return x }",
        "newline or semicolon",
    );
    rejects("fun f(): Int = 2147483648", "out of range");
    rejects("fun f(): Int = 01", "Leading zeroes");

    rejects("fun f(): Int = 1 /*", "Unterminated");
    rejects("fun f(): Int = 1\nfun f(x: Int): Int = x", "overloads");
}

#[test]
fn accepts_nested_comments_shadowing_and_minimum_int() {
    let source = "/* outer /* inner */ */\nfun f(x: Int): Int {\nval x = -2147483648\nif (true) { val x = 4 }\nreturn x\n}";
    assert!(compile(source, "Test.kt").is_ok());
}

#[test]
fn requires_legal_output_names_and_keeps_locations() {
    rejects("fun f(): Int = missing", "Unresolved");
    assert!(compile("fun f(): Int = 1", "../Test.kt").is_err());
    let source = "\nfun f(): Int = missing";
    let error = compile(source, "Test.kt").err().unwrap();
    assert!(error.render("Test.kt", source).starts_with("Test.kt:2:16:"));
}

#[test]
fn rejects_reserved_names_and_excessive_nesting_without_panicking() {
    rejects("fun f(): Int { val _ = 1; return _ }", "reserved");
    let long_sum = format!("fun f(): Int = {}", vec!["1"; 1000].join(" + "));
    rejects(&long_sum, "nesting");
    let nested = format!(
        "fun f(): Int {{ {}return 0 }}",
        "if (true) {} else ".repeat(1000)
    );
    rejects(&nested, "Nesting");
    let postfix = format!("fun f(a: IntArray): Int = a{}", ".size".repeat(1000));
    rejects(&postfix, "nesting");
}

#[test]
fn accepts_string_literals_calls_and_internal_functions() {
    let source = r#"internal fun f(value: String): String { var result = value; if (result.isEmpty()) { result = "\u0000\uD800😀" }; return result.removePrefix("x") + result.length + false }"#;
    assert!(compile(source, "Strings.kt").is_ok());
}

#[test]
fn rejects_invalid_strings_and_string_operations() {
    for source in [
        r#"fun f(): String = "unterminated"#,
        r#"fun f(): String = "\q""#,
        r#"fun f(): String = "\u12xz""#,
        "fun f(): String = \"line\nbreak\"",
        "fun f(x: String): Int = x.length()",
        "fun f(x: String): String = x.removePrefix()",
        "fun f(x: String): String = x.substring(true)",
        "fun f(x: String): Boolean = x.startsWith(1)",
        "fun f(x: String): String = x.substring(0, 1, 2)",
        "fun f(x: String): Boolean = x == 1",
    ] {
        assert!(compile(source, "Invalid.kt").is_err(), "accepted {source}");
    }
}

#[test]
fn checks_modified_utf8_constant_limit() {
    let accepted = format!("fun f(): String = \"{}\"", "\\u0000".repeat(32767));
    assert!(compile(&accepted, "Limit.kt").is_ok());
    let rejected = format!("fun f(): String = \"{}\"", "\\u0000".repeat(32768));
    assert!(compile(&rejected, "Limit.kt").is_err());
}

#[test]
fn accepts_unit_allocation_and_explicit_copy_ranges() {
    let source = "fun f(size: Int): IntArray { val result = IntArray(size); result.copyInto(result, 0, 0, 0); return result }\nfun g() { f(0); return }";
    assert!(compile(source, "Arrays.kt").is_ok());
    rejects("fun f(): Int = IntArray(1)", "Type mismatch");
    rejects("fun f() { val x = IntArray(true) }", "Type mismatch");
    rejects("fun f(a: IntArray): IntArray = a.copyInto(a)", "argument");
    rejects("fun f(): Int { return }", "Type mismatch");
}

#[test]
fn requires_classpath_for_imported_java_classes() {
    rejects(
        "import missing.Foreign\nfun f(value: Foreign): Int = 1",
        "not found",
    );
    rejects("fun f(value: Foreign): Int = 1", "explicit class import");
}

#[test]
fn extension_functions_require_receiver_syntax() {
    rejects(
        "fun IntArray.ext(): Int = 1\nfun bad(a: IntArray): Int = ext(a)",
        "extensions",
    );
}

#[test]
fn rejects_declared_extensions_shadowing_builtin_operations() {
    rejects(
        "fun String.removePrefix(prefix: String): String = \"custom\"\nfun use(s: String): String = s.removePrefix(\"x\")",
        "extensions",
    );
}

#[test]
fn accepts_chars_arrays_comparisons_and_extension_receivers() {
    for source in [
        "fun f(a: CharArray, i: Int): Char = a[i]",
        "fun f(a: CharArray): Int = a.size",
        "fun f(c: Char): Boolean = c >= '\\uD800' && c != 'x'",
        "fun f(c: Char): Char { var result: Char = 'a'; result = c; return result }",
        "fun Char.identity(): Char = this",
        "fun CharArray.count(): Int = this.size",
    ] {
        assert!(compile(source, "Chars.kt").is_ok(), "rejected {source}");
    }
    for literal in [
        r"'\t'",
        r"'\b'",
        r"'\n'",
        r"'\r'",
        r"'\''",
        r#"'\"'"#,
        r"'\\'",
        r"'\$'",
        "'$'",
        "'é'",
        r"'\u0000'",
        r"'\uD800'",
        r"'\uFFFF'",
    ] {
        let source = format!("fun f(): Char = {literal}");
        assert!(compile(&source, "Chars.kt").is_ok(), "rejected {source}");
    }
}

#[test]
fn rejects_malformed_chars_and_implicit_char_conversions() {
    for literal in [
        "''",
        "'ab'",
        "'😀'",
        r"'\q'",
        r"'\u12xz'",
        r"'\u12'",
        "'unterminated",
        "'\n'",
        "'\r'",
        r"'\uD800\uDC00'",
    ] {
        let source = format!("fun f(): Char = {literal}");
        assert!(compile(&source, "Invalid.kt").is_err(), "accepted {source}");
    }
    for source in [
        "fun f(): Int = 'a'",
        "fun f(): Char = 97",
        "fun f(): Char = 'a' + 1",
        "fun f(): Int = 'b' - 'a'",
        "fun f(): Char = -'a'",
        "fun f(): Boolean = 'a' == 97",
        "fun f(): Boolean = 97 < 'a'",
        "fun f(): Boolean = 'a' < 97",
        "fun f(): String = \"char\" + 'a'",
        "fun f(): String = \"x\".substring('a')",
        "fun f(a: CharArray): Char { var c = 'x'; c = 1; return c }",
        "fun f(a: CharArray): Char = a['a']",
        "fun f(a: CharArray, b: CharArray): Boolean = a == b",
        "fun f(a: CharArray) { a[0] = 'x' }",
        "fun f(): CharArray = CharArray(1)",
        "fun f(a: CharArray): CharArray = a.copyInto(a, 0, 0, 1)",
        "fun f(s: String): Char = s[0]",
    ] {
        assert!(compile(source, "Invalid.kt").is_err(), "accepted {source}");
    }
    for builtin in ["Char", "CharArray"] {
        rejects(
            &format!("import missing.Foreign as {builtin}\nfun f() {{}}"),
            "shadowing builtin",
        );
    }
}

#[test]
fn compound_addition_requires_mutable_int_locals() {
    assert!(
        compile(
            "fun f(x: Int): Int { var n = x; n += n + 2; return n }",
            "Add.kt"
        )
        .is_ok()
    );
    for (source, message) in [
        ("fun f() { val n = 1; n += 2 }", "Cannot reassign"),
        ("fun f(n: Int) { n += 2 }", "Cannot reassign"),
        ("fun f() { var n = 'a'; n += 2 }", "only mutable Int"),
        ("fun f() { var n = \"a\"; n += \"b\" }", "only mutable Int"),
        ("fun f() { var n = true; n += false }", "only mutable Int"),
        ("fun f() { var n = 1; n += 'a' }", "Type mismatch"),
        ("fun f() { missing += 1 }", "Unresolved"),
    ] {
        rejects(source, message);
    }
    for operator in ["-=", "*=", "/=", "%=", "+ =", "++", "+=="] {
        let source = format!("fun f() {{ var n = 1; n {operator} 2 }}");
        assert!(compile(&source, "Invalid.kt").is_err(), "accepted {source}");
    }
}

#[test]
fn accepts_statement_when_and_checks_return_paths_and_scopes() {
    for source in [
        "fun f(c: Char): Int { when (c) { 'a', 'b' -> return 1; else -> return 2 } }",
        "fun f(n: Int): Int { when (n) { 0 -> { return 1 }; 1 -> return 2; else -> { return 3 } } }",
        "fun f(n: Int): Int { var x = 0; when (n) { 0 -> x += 1; 1, 2 -> { val x = 5 }; else -> x += 2 }; return x }",
        "fun f(n: Int) { when (n) { 0 -> return } }",
        "fun f(n: Int) { when (n) {} }",
        "fun f(n: Int): Int { when (n) { else -> return 1 } }",
        "fun f(n: Int): Int { when (n) { 0 -> when (n) { else -> return 1 }; else -> return 2 } }",
    ] {
        assert!(compile(source, "When.kt").is_ok(), "rejected {source}");
    }
    for source in [
        "fun f(n: Int): Int { when (n) { 0 -> return 1 } }",
        "fun f(n: Int): Int { when (n) { 0 -> {}; else -> return 1 } }",
    ] {
        rejects(source, "return on all paths");
    }
    rejects(
        "fun f(n: Int): Int { when (n) { 0 -> { val x = 1 }; else -> { val x = 2 } }; return x }",
        "Unresolved",
    );
    rejects(
        "fun f(n: Int) { when (n) { 0 -> { val x = 1 }; 1 -> { x } } }",
        "Unresolved",
    );
    rejects(
        "fun f(n: Int) { when (n) { 0 -> { val x = 1 }; x -> {} } }",
        "Unresolved",
    );
    rejects(
        "fun f(n: Int): Int { when (n) { 0 -> return\n1 -> return 2; else -> return 3 } }",
        "Type mismatch",
    );
}

#[test]
fn rejects_unsupported_when_forms_and_malformed_entries() {
    for source in [
        "fun f(): Int = when (1) { else -> 1 }",
        "fun f() { when { true -> return } }",
        "fun f() { when (val n = 1) { else -> return } }",
        "fun f() { when (true) { true -> return } }",
        "fun f() { when (\"x\") { \"x\" -> return } }",
        "fun f(a: CharArray) { when (a) { else -> return } }",
        "fun f() { when (1) { 'a' -> return } }",
        "fun f() { when ('a') { 97 -> return } }",
        "fun f() { when (1) { in 0..2 -> return } }",
        "fun f() { when (1) { is Int -> return } }",
        "fun f() { when (1) { 1 if true -> return } }",
        "fun f() { when (1) { else -> return; 1 -> return } }",
        "fun f() { when (1) { else -> return; else -> return } }",
        "fun f() { when (1) { 1 -> {} 2 -> {} } }",
        "fun f() { when (1) { 1 -> {} else -> {} } }",
        "fun f() { when (1) { 1, -> return } }",
        "fun f() { when (1) { ,1 -> return } }",
        "fun f() { when (1) { 1 return } }",
        "fun f() { when (1) { 1 -> } }",
        "fun f() { when (1) { 1 -> return }",
    ] {
        assert!(compile(source, "Invalid.kt").is_err(), "accepted {source}");
    }
}

#[test]
fn when_generated_locals_and_nesting_obey_limits() {
    let branches = "0 -> {};".repeat(129);
    assert!(
        compile(
            &format!("fun f() {{ when (1) {{ {branches} }} }}"),
            "Limit.kt"
        )
        .err()
        .unwrap()
        .message
        .to_ascii_lowercase()
        .contains("nesting")
    );
    let conditions = vec!["0"; 129].join(",");
    rejects(
        &format!("fun f() {{ when (1) {{ {conditions} -> {{}} }} }}"),
        "nesting",
    );
    let mut nested = "return".to_string();
    for _ in 0..20 {
        nested = format!(
            "when (1) {{ {} else -> {{ {nested} }} }}",
            "0 -> {};".repeat(20)
        );
    }
    assert!(
        compile(&format!("fun f() {{ {nested} }}"), "Limit.kt")
            .err()
            .unwrap()
            .message
            .to_ascii_lowercase()
            .contains("nesting")
    );
    let params = (0..255)
        .map(|i| format!("p{i}: Int"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(compile(&format!("fun f({params}) {{ when (1) {{}} }}"), "Slots.kt").is_ok());
    rejects(
        &format!("fun f({params}) {{ when (1) {{}}; when (2) {{}} }}"),
        "256 local slots",
    );
}
