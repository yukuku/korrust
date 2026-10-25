use korrust::{
    compile, compile_all, compile_all_with_classpath, compile_outputs_with_classpath,
    compile_with_classpath,
};

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
        ("fun f(): Int = true + 1", "Arithmetic requires"),
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
    rejects("fun f(): Int = 2147483648", "Type mismatch");
    rejects("fun f(): Int = 01", "Leading zeroes");

    rejects("fun f(): Int = 1 /*", "Unterminated");
    rejects(
        "fun f(): Int = 1\nfun f(): Int = 2",
        "Conflicting JVM function signature",
    );
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
    rejects("fun f(value: Foreign): Int = 1", "not found");
}

#[test]
fn extension_functions_require_receiver_syntax() {
    rejects(
        "fun IntArray.ext(): Int = 1\nfun bad(a: IntArray): Int = ext(a)",
        "extensions",
    );
}

#[test]
fn resolves_declared_extensions_before_implicit_library_extensions() {
    assert!(compile(
        "fun String.removePrefix(prefix: String): String = \"custom\"\nfun use(s: String): String = s.removePrefix(\"x\")",
        "Extensions.kt",
    ).is_ok());
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
        "fun f(a: CharArray): CharArray = a.copyInto(a, 0, 0, 1)",
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
    for operator in ["+ =", "++", "+=="] {
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
        "fun f() { when (val n = 1) { else -> return } }",
        "fun f() { when (1) { 'a' -> return } }",
        "fun f() { when ('a') { 97 -> return } }",
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

#[test]
fn accepts_bare_enums_and_uses_the_declaration_name() {
    for source in [
        "enum class Choice { First, Second }",
        "public enum class Choice { First, Second, }",
        "internal enum class Choice { First, Second; }",
        "enum class Choice { First, Second,; }",
        "enum class Choice {}",
        "enum class Choice {;}",
        "enum class Choice",
        "enum class Choice;",
        "enum class Choice\n",
        "public\nenum\nclass\nChoice\n{\nFirst /* entry */,\nSecond,\n}\n",
    ] {
        let class = compile(source, "DifferentFile.kt")
            .unwrap_or_else(|error| panic!("rejected {source}: {}", error.message));
        assert_eq!(class.internal_name, "Choice");
        assert_eq!(&class.bytes[..4], &[0xca, 0xfe, 0xba, 0xbe]);
    }
    let class = compile(
        "package example.nested\ninternal enum class Choice { First }",
        "differentFile.kt",
    )
    .unwrap();
    assert_eq!(class.internal_name, "example/nested/Choice");
    assert!(compile("enum class E { values, valueOf, E }", "Enum.kt").is_ok());
}

#[test]
fn rejects_enum_features_outside_the_bare_declaration_scope() {
    for source in [
        "private enum class E { A }",
        "protected enum class E { A }",
        "open enum class E { A }",
        "sealed enum class E { A }",
        "public internal enum class E { A }",
        "public public enum class E { A }",
        "enum class E() { A }",
        "enum class E private constructor() { A }",
        "enum class E(val n: Int) { A(1) }",
        "enum class E : Runnable { A }",
        "enum class E<T> { A }",
        "@Deprecated(\"x\") enum class E { A }",
        "enum class E { @Deprecated(\"x\") A }",
        "enum class E { A() }",
        "enum class E { A(1) }",
        "enum class E { A {} }",
        "enum class E { A { fun f() {} } }",
        "enum class E { A; fun f() {} }",
        "enum class E { A; val x = 1 }",
        "enum class E { A; companion object {} }",
        "enum class E { enum class Nested { A } }",
        "class Outer { enum class E { A } }",
        "enum class E { A }\nfun f() {}",
        "fun f() {}\nenum class E { A }",
        "enum class E { A }\nenum class F { B }",
        "enum class E\nfun f() {}",
        "enum class E { A B }",
        "enum class E { A\nB }",
        "enum class E { ,A }",
        "enum class E { A,, }",
        "enum class E { A;; }",
        "enum class E { A; B }",
        "enum class E { A",
        "enum class E { _ }",
        "enum class E { catch }",
        "enum class E { finally }",
        "enum class E { import }",
        "enum class E { name }",
        "enum class E { ordinal }",
        "enum class E { entries }",
        "enum class E { `A` }",
    ] {
        assert!(compile(source, "Invalid.kt").is_err(), "accepted {source}");
    }
    rejects("private enum class E { A }", "Unsupported enum visibility");
    rejects(
        "enum class E { A }\nenum class F { B }",
        "Only one enum class",
    );
}

#[test]
fn rejects_duplicate_enum_entries_at_the_second_declaration() {
    let source = "enum class E { First, Second, First }";
    let error = compile(source, "Enum.kt").err().unwrap();
    assert!(error.message.contains("Duplicate enum entry"));
    assert_eq!(error.offset, source.rfind("First").unwrap());
    assert!(error.render("Enum.kt", source).starts_with("Enum.kt:1:31:"));
}

#[test]
fn bounds_enum_entry_counts() {
    let entries = (0..1024)
        .map(|n| format!("Entry{n}"))
        .collect::<Vec<_>>()
        .join(",");
    assert!(compile(&format!("enum class E {{ {entries} }}"), "Enum.kt").is_ok());
    rejects(
        &format!("enum class E {{ {entries}, Entry1024 }}"),
        "1024-entry limit",
    );
}

#[test]
fn validates_even_unused_enum_imports() {
    rejects("import missing.Foreign\nenum class E { A }", "not found");
    rejects(
        "import missing.Foreign as Int\nenum class E { A }",
        "shadowing builtin",
    );
    rejects(
        "import missing.Foreign as E\nenum class E { A }",
        "conflicts with enum",
    );
    assert!(compile("import missing.*\nenum class E { A }", "Enum.kt").is_err());
}

#[test]
fn accepts_objects_with_typed_and_inferred_literal_constants() {
    for source in [
        "object Empty",
        "object Empty {}",
        "object Empty {;}",
        "public object Empty {}",
        "internal\nobject Empty {}",
        "object Empty { const val N = 0 }",
        "object Empty { const val Min = -2147483648; const val Max: Int = 2147483647; const val Plus = +1; const val MinusZero = -0 }",
        "object Empty { public const val Yes: Boolean = true; const internal val No = false }",
        "object Empty { const val C: Char = '\\uD800'; const val Other = 'é' }",
        r#"object Empty { const val S: String = "\u0000\uD800😀"; const val Empty = "" }"#,
        "object Empty { const val Empty = 1; const val name = 2; const val entries = 3 }",
    ] {
        let class = compile(source, "DifferentFile.kt")
            .unwrap_or_else(|error| panic!("rejected {source}: {}", error.message));
        assert_eq!(class.internal_name, "Empty");
        assert_eq!(&class.bytes[..4], &[0xca, 0xfe, 0xba, 0xbe]);
    }
}

#[test]
fn compiles_all_nested_object_classes_without_a_facade() {
    let source = "package test.objects\nobject Root { const val N=1; object Child { internal object Grandchild; const val C='a' }; object Root }";
    let classes = compile_all(source, "UnrelatedFile.kt").unwrap();
    let mut names = classes
        .iter()
        .map(|class| class.internal_name.as_str())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "test/objects/Root",
            "test/objects/Root$Child",
            "test/objects/Root$Child$Grandchild",
            "test/objects/Root$Root"
        ]
    );
    for class in classes {
        assert_eq!(&class.bytes[..4], &[0xca, 0xfe, 0xba, 0xbe]);
    }
    for result in [
        compile(source, "UnrelatedFile.kt"),
        compile_with_classpath(source, "UnrelatedFile.kt", &[]),
    ] {
        let error = result.err().unwrap();
        assert!(error.message.contains("multiple classes"));
        assert!(error.message.contains("compile_all"));
    }
    assert_eq!(
        compile_all("fun f(): Int = 1", "test.kt").unwrap()[0].internal_name,
        "TestKt"
    );
    assert_eq!(
        compile_all("enum class E { A }", "test.kt").unwrap()[0].internal_name,
        "E"
    );
    assert_eq!(
        compile_all_with_classpath("object E", "test.kt", &[]).unwrap()[0].internal_name,
        "E"
    );
    assert!(compile_all("object E", "../test.kt").is_err());
    assert_eq!(
        compile("object INSTANCE", "Singleton.kt")
            .unwrap()
            .internal_name,
        "INSTANCE"
    );
}

#[test]
fn rejects_unsupported_object_and_constant_forms() {
    for source in [
        "private object E",
        "protected object E",
        "data object E",
        "companion object E",
        "public internal object E",
        "object E()",
        "object E : Runnable",
        "object E<T>",
        "@Deprecated(\"x\") object E",
        "object E { init {} }",
        "object E { val N = 1 }",
        "object E { var N = 1 }",
        "object E { const var N = 1 }",
        "object E { const const val N = 1 }",
        "object E { public internal const val N = 1 }",
        "object E { private const val N = 1 }",
        "object E { protected object Nested }",
        "object E { data object Nested }",
        "object E { companion object Nested }",
        "object E { object Nested() }",
        "object E { const val INSTANCE = 1 }",
        "object E { object INSTANCE }",
        "object E { const val N = 1 + 2 }",
        "object E { const val N = (1) }",
        "object E { const val N = other }",
        "object E { const val N = -true }",
        "object E { const val N = !false }",
        "object E { const val N = null }",
        "object E { const val N: Boolean = 1 }",
        "object E { const val N: Char = 1 }",
        "object E { const val N: Int = 'a' }",
        "object E { const val N: String = false }",
        "object E { const val N: Long = 1 }",
        "object E { const val N = 2147483648 }",
        "object E { const val N = -2147483649 }",
        "object E { const val N = +2147483648 }",
        "object E { const val N = 9223372036854775808 }",
        "object E { const val N = 01 }",
        "object E { const val N = 1L }",
        "object E { const val N = 1_000 }",
        "object E { const val N = '😀' }",
        "object E { const val N = \"$value\" }",
        "object E { const val N = 1 const val Other = 2 }",
        "object E { const val N = 1",
        "object E { object Nested",
        "object E\nobject Other",
        "object E\nfun f() {}",
        "fun f() {}\nobject E",
        "enum class E { A }\nobject Other",
        "object E\nenum class Other { A }",
    ] {
        assert!(
            compile_all(source, "Invalid.kt").is_err(),
            "accepted {source}"
        );
    }
    rejects(
        "object E { const val INSTANCE = 1 }",
        "generated singleton field",
    );
    rejects(
        "object E { const val N: Boolean = 1 }",
        "Const literal type mismatch",
    );
    rejects("object E { const val N = -2147483649 }", "out of range");
    rejects("object E { const val N = 1", "Unterminated object body");
}

#[test]
fn object_member_names_share_one_namespace_and_preserve_locations() {
    for source in [
        "object E { const val N=1; const val N=2 }",
        "object E { object N; object N }",
        "object E { const val N=1; object N }",
        "object E { object N; const val N=1 }",
    ] {
        let error = compile_all(source, "Object.kt").err().unwrap();
        assert!(error.message.contains("Duplicate object member name"));
        assert_eq!(error.offset, source.rfind(';').unwrap() + 2);
    }
}

#[test]
fn object_depth_class_and_constant_counts_obey_limits() {
    let nested = |count: usize| {
        format!(
            "{}{}",
            (0..count)
                .map(|i| format!("object N{i} {{ "))
                .collect::<String>(),
            "}".repeat(count)
        )
    };
    assert_eq!(compile_all(&nested(16), "Depth.kt").unwrap().len(), 16);
    assert!(
        compile_all(&nested(17), "Depth.kt")
            .err()
            .unwrap()
            .message
            .contains("16-class depth limit")
    );
    let siblings = (0..255)
        .map(|i| format!("object N{i};"))
        .collect::<String>();
    assert_eq!(
        compile_all(&format!("object Root {{ {siblings} }}"), "Classes.kt")
            .unwrap()
            .len(),
        256
    );
    assert!(
        compile_all(
            &format!("object Root {{ {siblings} object Extra }}"),
            "Classes.kt"
        )
        .err()
        .unwrap()
        .message
        .contains("256-class limit")
    );
    let constants = (0..1024)
        .map(|i| format!("const val N{i}={i};"))
        .collect::<String>();
    assert!(compile(&format!("object Root {{ {constants} }}"), "Constants.kt").is_ok());
    rejects(
        &format!("object Root {{ {constants} const val Extra=0 }}"),
        "1024-constant limit",
    );
}

#[test]
fn object_string_constants_obey_modified_utf8_limits() {
    let accepted = format!(
        "object Root {{ const val Text = \"{}\" }}",
        "\\u0000".repeat(32767)
    );
    assert!(compile(&accepted, "Strings.kt").is_ok());
    let rejected = format!(
        "object Root {{ const val Text = \"{}\" }}",
        "\\u0000".repeat(32768)
    );
    assert!(compile(&rejected, "Strings.kt").is_err());
}

#[test]
fn object_imports_remain_fail_closed() {
    rejects("import missing.Foreign\nobject E", "not found");
    rejects(
        "import missing.Foreign as E\nobject E",
        "conflicts with object",
    );
    rejects(
        "import missing.Foreign as Int\nobject E",
        "shadowing builtin",
    );
    assert!(compile_all("import missing.*\nobject E", "Object.kt").is_err());
}

#[test]
fn accepts_ordinary_final_classes_with_constructor_fields_and_instance_members() {
    let source = r#"package sample
internal class Texts(private val texts: Array<String>, private val tag: Int) {
    val count: Int
        get() = texts.size
    public val marker: Int get() { return tag }
    fun text(index: Int): String = texts[index]
    fun identity(values: Array<String>): Array<String> = values
    private fun saved(): Int = tag
}"#;
    let class =
        compile(source, "DifferentFile.kt").unwrap_or_else(|error| panic!("{}", error.message));
    assert_eq!(class.internal_name, "sample/Texts");
    assert_eq!(&class.bytes[..4], &[0xca, 0xfe, 0xba, 0xbe]);
    assert!(
        class
            .bytes
            .windows(19)
            .any(|bytes| bytes == b"[Ljava/lang/String;")
    );
    for declaration in [
        "class Empty()",
        "public class Empty() {}",
        "final internal class Empty() {}",
        "internal final class Empty() {}",
    ] {
        let class = compile(declaration, "Unrelated.kt").unwrap();
        assert_eq!(class.internal_name, "Empty");
    }
}

#[test]
fn rejects_unsupported_class_surface_and_mixed_declarations() {
    for source in [
        "private class E()",
        "protected class E()",
        "open class E()",
        "abstract class E()",
        "data class E(private val x: Int)",
        "sealed class E()",
        "public internal class E()",
        "final final class E()",
        "class E<T>()",
        "class E private constructor()",
        "class E(private val x: Int, private val x: Int)",
        "class E() { constructor(x: Int) {} }",
        "class E() { init {} }",
        "class E() { var x: Int get() = 1 }",
        "class E() { internal fun x() {} }",
        "class E() { protected fun x() {} }",
        "class E() { private override fun x() {} }",
        "class E() { override override fun x() {} }",
        "class E() { fun String.extension(): Int = 1 }",
        "class E() { object Nested }",
        "class E() { companion object }",
        "class E() { class Nested() }",
        "class E() { val x: Int get() = 1; val x: Int get() = 2 }",
        "class E(private val x: Int) { val x: Int get() = 1 }",
        "class E(private val x: Array<Int>)",
        "class E(private val x: Array<Array<String>>)",
        "class E() { val x: Int get() = 1",
        "class E()\nfun f() {}",
        "fun f() {}\nclass E()",
        "class E()\nobject Other",
        "class E()\nenum class Other { A }",
        "class E()\nclass Other()",
    ] {
        assert!(
            compile_all(source, "Invalid.kt").is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn checks_constructor_fields_array_operations_and_getter_returns() {
    for (source, message) in [
        (
            "class E(private val value: Int) { fun f(): String = value }",
            "Type mismatch",
        ),
        (
            "class E(private val values: Array<String>) { fun f(): Int = values[0] }",
            "Type mismatch",
        ),
        (
            "class E(private val values: Array<String>) { fun f(): String = values[true] }",
            "Type mismatch",
        ),
        (
            "class E(private val value: Int) { fun f(): Int = missing }",
            "Unresolved",
        ),
        (
            "class E(private val value: Int) { fun f() { value = 1 } }",
            "Cannot reassign",
        ),
        ("class E() { val x: Int get() {} }", "return on all paths"),
        ("class E() { val x: Int get() = false }", "Type mismatch"),
    ] {
        rejects(source, message);
    }
}

#[test]
fn classes_require_valid_imports_even_when_unused() {
    rejects("import missing.Foreign\nclass E()", "not found");
    rejects(
        "import missing.Foreign as Array\nclass E()",
        "shadowing builtin",
    );
    rejects(
        "import missing.Foreign as E\nclass E()",
        "conflicts with class",
    );
    assert!(compile("class E() : Missing {}", "Class.kt").is_err());
}

#[test]
fn output_artifacts_include_module_mapping_only_for_function_facades() {
    let outputs =
        compile_outputs_with_classpath("package p\nfun f() {}", "sample.kt", &[]).unwrap();
    let mut paths = outputs
        .iter()
        .map(|output| output.relative_path.as_str())
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(
        paths,
        [
            "META-INF/korrust_23680438640a1a3c1f3140900a6ba17d.kotlin_module",
            "p/SampleKt.class"
        ]
    );
    assert!(outputs.iter().all(|output| !output.bytes.is_empty()));
    for source in ["object NameKt", "enum class NameKt { A }", "class NameKt()"] {
        let outputs = compile_outputs_with_classpath(source, "Other.kt", &[]).unwrap();
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].relative_path, "NameKt.class");
    }
    let outputs =
        compile_outputs_with_classpath("object Root { object Child }", "Other.kt", &[]).unwrap();
    assert_eq!(outputs.len(), 2);
    assert!(
        outputs
            .iter()
            .all(|output| output.relative_path.ends_with(".class"))
    );
}

#[test]
fn rejects_declared_classifiers_shadowing_builtin_types() {
    for source in [
        "class Array()",
        "class Int() { fun f(x: Int): Int = x }",
        "object Int { const val X: Int = 1 }",
        "object Root { object String; const val X: String = \"x\" }",
        "enum class Boolean { A }",
    ] {
        rejects(source, "shadowing builtin types");
    }
}

#[test]
fn module_mapping_filenames_stay_bounded_for_long_packages() {
    let package = vec!["longpackagename"; 24].join(".");
    let source = format!("package {package}\nfun f() {{}}");
    let outputs = compile_outputs_with_classpath(&source, "sample.kt", &[]).unwrap();
    let mapping = outputs
        .iter()
        .find(|output| output.relative_path.ends_with(".kotlin_module"))
        .unwrap();
    assert!(mapping.relative_path.rsplit('/').next().unwrap().len() <= 255);
    assert_eq!(
        mapping.relative_path.len(),
        "META-INF/korrust_".len() + 32 + ".kotlin_module".len()
    );
    let repeated = compile_outputs_with_classpath(&source, "sample.kt", &[]).unwrap();
    assert_eq!(
        mapping.relative_path,
        repeated
            .iter()
            .find(|output| output.relative_path.ends_with(".kotlin_module"))
            .unwrap()
            .relative_path
    );
}

#[test]
fn rejects_unsupported_inherited_any_and_object_members() {
    for source in [
        "class A() { fun toString(): String = \"x\" }",
        "class A() { override fun toString(): String = \"x\" }",
        "class A() { fun hashCode(): Int = 1 }",
        "class A() { fun notify() {} }",
    ] {
        rejects(source, "Any and Object inherited members are not supported");
    }
}

#[test]
fn accepts_abstract_and_default_interface_declarations() {
    for source in [
        "interface Empty",
        "interface Empty {}",
        "public interface Empty { val size: Int; fun at(index: Int): String }",
        "internal interface Empty { fun weight(index: Int): Float }",
        "interface Empty { val answer: Int get() = 42 }",
        "interface Empty { fun answer(): Int = 42 }",
    ] {
        let classes = compile_all(source, "Different.kt")
            .unwrap_or_else(|error| panic!("rejected {source}: {}", error.message));
        assert_eq!(classes[0].internal_name, "Empty");
        assert!(
            classes
                .iter()
                .all(|class| class.bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbe]))
        );
    }
}

#[test]
fn interfaces_emit_generic_nested_companion_and_anonymous_classes() {
    let source = r#"package sample
interface View {
    val size: Int
    fun at(index: Int): String
    fun label(index: Int): String {
        val next = index + 1
        return if (next < 2) { LABELS[next] } else { next.toString() }
    }
    interface Child : View { fun weight(index: Int): Float }
    companion object {
        private val LABELS = arrayOf("zero", "\u0000二")
        val EMPTY = object : View {
            override val size: Int get() = 0
            override fun at(index: Int): String {
                throw IllegalArgumentException("index=$index size=$size")
            }
        }
    }
}"#;
    let classes =
        compile_all(source, "Different.kt").unwrap_or_else(|error| panic!("{}", error.message));
    let mut names = classes
        .iter()
        .map(|class| class.internal_name.as_str())
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(
        names,
        [
            "sample/View",
            "sample/View$Child",
            "sample/View$Child$DefaultImpls",
            "sample/View$Companion",
            "sample/View$Companion$EMPTY$1",
            "sample/View$DefaultImpls",
        ]
    );
    assert!(
        compile(source, "Different.kt")
            .err()
            .unwrap()
            .message
            .contains("multiple classes")
    );
    let outputs = compile_outputs_with_classpath(source, "Different.kt", &[]).unwrap();
    assert_eq!(outputs.len(), 6);
    assert!(
        outputs
            .iter()
            .all(|output| output.relative_path.ends_with(".class"))
    );
}

#[test]
fn supports_if_expressions_integer_strings_templates_and_explicit_throws() {
    for source in [
        "fun f(n: Int): Int = if (n < 0) { -1 } else { 1 }",
        "fun f(n: Int): String = if (n < 0) \"negative\" else n.toString()",
        r#"fun f(n: Int, b: Boolean, s: String): String = "$s:$n:$b""#,
        r#"fun f(n: Int): String = "\$n/$n/\u0000\uD800""#,
        r#"fun f(n: Int): String { throw IllegalArgumentException("n=$n") }"#,
        r#"fun f(n: Int): String = "${n}""#,
    ] {
        assert!(
            compile(source, "Expressions.kt").is_ok(),
            "rejected {source}"
        );
    }
    rejects("fun f(): Int = if (1) 2 else 3", "Type mismatch");
    rejects("fun f(): Int = if (true) 1 else false", "Type mismatch");
    rejects(r#"fun f(): String = "$missing""#, "Unresolved");
    rejects(
        r#"fun f(c: Char): String = "$c""#,
        "String templates support only",
    );
    rejects(
        r#"fun f(n: Int): String = "$`n`""#,
        "Only simple ASCII name",
    );
    rejects(r#"fun f(n: Int): String = "$n二""#, "Non-ASCII characters");
    rejects(
        "fun f(): String { throw IllegalArgumentException(1) }",
        "Type mismatch",
    );
    rejects(
        "fun f(): String { throw RuntimeException(\"x\") }",
        "Only throwing supported standard exception constructors",
    );
    let source = r#"object E { const val S = "$missing" }"#;
    let error = compile(source, "Literal.kt").err().unwrap();
    assert!(error.message.contains("literal initializers"));
    assert_eq!(error.offset, source.find('"').unwrap());
}

#[test]
fn accepts_abstract_mutable_interface_properties() {
    for source in [
        "interface Surface { var count: Int; var isReady: Boolean; var isTitle: String }",
        "interface Surface { var bytes: ByteArray; var chars: CharArray; var names: Array<String> }",
        "interface Surface { val count: Int; interface Mutable : Surface { override var count: Int } }",
    ] {
        assert!(
            korrust::compile_all(source, "Surface.kt").is_ok(),
            "{source}"
        );
    }
}

#[test]
fn rejects_unsupported_interface_and_companion_forms() {
    for source in [
        "private interface E",
        "protected interface E",
        "sealed interface E",
        "fun interface E { fun f() }",
        "interface E<T>",
        "interface E()",
        "interface E { var x: Int get() = 1 }",
        "interface E { val x: Int = 1 }",
        "interface E { private fun f() {} }",
        "interface E { fun String.extension(): Int = 1 }",
        "interface E { object Named }",
        "interface E { class Nested() }",
        "interface E { companion object Named {} }",
        "interface E { companion object {}; companion object {} }",
        "interface E { companion object { var x = 1 } }",
        "interface E { companion object { const val x = 1 } }",
        "interface E { companion object { val x = arrayOf(\"x\") } }",
        "interface E { companion object { private val x = arrayOf(1) } }",
        "interface E { companion object { private val x = arrayOf() } }",
        "interface E { companion object { private val x: IntArray = arrayOf(\"x\") } }",
        "interface E { companion object { val x = 1 } }",
        "interface E { companion object { private val x = object : E {} } }",
        "interface E { companion object { val x: String = object : E {} } }",
        "interface E { val x: Int; val x: Int }",
        "interface E { interface Child; interface Child }",
        "interface E { companion object { private val x = arrayOf(\"a\"); private val x = arrayOf(\"b\") } }",
        "interface E { companion object { val INSTANCE = object : E {} } }",
        "interface E { fun f(): Int",
        "interface E { companion object { private val x = arrayOf(\"a\")",
        "interface E\nfun f() {}",
        "fun f() {}\ninterface E",
        "interface E\ninterface Other",
        "interface E\nclass Other()",
        "interface E\nobject Other",
        "interface E\nenum class Other { A }",
    ] {
        assert!(
            compile_all(source, "Unsupported.kt").is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn preserves_abstract_member_and_bodyless_nested_interface_boundaries() {
    let source = "interface Root { fun first()\nval count: Int\ninterface Child\nfun second(index: Int): String\n}";
    let mut names = compile_all(source, "Boundary.kt")
        .unwrap()
        .into_iter()
        .map(|class| class.internal_name)
        .collect::<Vec<_>>();
    names.sort();
    assert_eq!(names, ["Root", "Root$Child"]);
    assert!(
        compile(
            "interface Root { fun f(): Int val count: Int }",
            "Invalid.kt"
        )
        .is_err()
    );
}

#[test]
fn limits_interface_nesting_and_template_complexity() {
    let nested = |count: usize| {
        format!(
            "{}{}",
            (0..count)
                .map(|index| format!("interface N{index} {{ "))
                .collect::<String>(),
            "}".repeat(count)
        )
    };
    assert_eq!(compile_all(&nested(16), "Depth.kt").unwrap().len(), 16);
    assert!(
        compile_all(&nested(17), "Depth.kt")
            .err()
            .unwrap()
            .message
            .contains("16-declaration depth limit")
    );
    rejects(
        &format!("fun f(n: Int): String = \"{}\"", "$n ".repeat(129)),
        "128-name limit",
    );
    rejects("interface Float", "shadowing builtin types");
    rejects("import missing.Foreign\ninterface E", "not found");
}

#[test]
fn accepts_byte_arrays_char_array_construction_and_hexadecimal_ints() {
    for source in [
        "fun f(value: Byte): Int = value.toInt()",
        "fun f(value: Int): Byte = value.toByte()",
        "fun f(size: Int): ByteArray = ByteArray(size)",
        "fun f(): CharArray = CharArray(1)",
        "fun f(a: CharArray) { a[0] = 'x' }",
        "fun f(a: ByteArray, index: Int, value: Byte): Byte { a[index] = value; return a[index] }",
        "fun f(): Int = 0x7fffffff",
        "fun f(): Int = -0X80000000",
        "fun f(value: Byte): Int = value.toInt() and 0xff",
        "fun f(value: Int): Char = value.toChar()",
        "fun f(value: Char): Int = value.code",
    ] {
        assert!(compile(source, "Bytes.kt").is_ok(), "rejected {source}");
    }
    for source in [
        "fun f(): Int = 0x80000000",
        "fun f(): Int = -0x80000001",
        "fun f(): Int = 0x",
        "fun f(): Int = 0xGG",
        "fun f(): Int = 0xffL",
        "fun f(): Int = 0xff_ff",
        "fun f(): Int = 0xffffffffffffffffffffffffffffffff",
        "fun f(a: ByteArray) { a[0] = true }",
        "fun f(a: CharArray) { a[0] = 1 }",
    ] {
        assert!(compile(source, "Invalid.kt").is_err(), "accepted {source}");
    }
    assert!(compile("object E { const val Hex: Int = 0xff }", "Hex.kt").is_ok());
}

#[test]
fn accepts_annotated_overloaded_object_methods_and_lazy_array_fields() {
    let source = r#"package arbitrary
internal object Scratch {
    private val bytes = ThreadLocal.withInitial { ByteArray(7) }
    private val chars = ThreadLocal.withInitial { CharArray(9) }
    @JvmStatic fun choose(value: Byte): Int = value.toInt()
    @JvmStatic fun choose(value: Int): Int = value + 2
    @JvmStatic fun choose(value: ByteArray): Int = value.size
    @JvmStatic fun bytes(): ByteArray = bytes.get()!!
    @JvmStatic fun chars(): CharArray = chars.get()!!
    @Suppress("NOTHING_TO_INLINE") private inline fun low(value: Byte): Int = value.toInt() and 0xff
    private fun call(value: Byte): Int = choose(value)
}"#;
    let classes =
        compile_all(source, "Different.kt").unwrap_or_else(|error| panic!("{}", error.message));
    assert_eq!(classes.len(), 3);
    assert!(
        classes
            .iter()
            .any(|class| class.internal_name == "arbitrary/Scratch")
    );
    assert!(
        classes
            .iter()
            .all(|class| class.internal_name.starts_with("arbitrary/Scratch"))
    );
    assert!(
        compile(source, "Different.kt")
            .err()
            .unwrap()
            .message
            .contains("multiple classes")
    );
    assert!(compile("object E { fun f() {} }", "Methods.kt").is_ok());
    assert!(compile("object E { @JvmStatic @Throws(IllegalArgumentException::class) fun fail() { throw IllegalArgumentException() } }", "Throws.kt").is_ok());
}

#[test]
fn supports_ranges_when_expressions_loops_and_incremented_array_indices() {
    for source in [
        "fun f(): Int = when (1) { else -> 1 }",
        "fun f() { when (1) { in 0..2 -> return } }",
        "fun f(value: Int): Int = when (value) { in 0x10..0x1f -> 2; in 0x20..0x2f -> 3; else -> 1 }",
        "fun f(value: Int): Boolean = value in 'A'.code..'Z'.code",
        "fun f(value: Int): Boolean = value !in -2..2",
        "fun f(limit: Int): Int { var result = 0; for (index in 0 until limit) { if (index == 2) continue; result += index }; return result }",
        "fun f(limit: Int): Int { var result = 0; while (result < limit) { result++; if (result > 2) break }; return result }",
        "fun f(a: CharArray, value: Char): Int { var cursor = 0; a[cursor++] = value; return cursor }",
        "fun f(): Int { var n = 1; return ++n + n++ }",
        "fun f(a: ByteArray, index: Int): Boolean = index >= 0\n && index < a.size\n && (a[index].toInt() and 0xff) >= 0x80",
    ] {
        assert!(compile(source, "Control.kt").is_ok(), "rejected {source}");
    }
}

#[test]
fn requires_definite_assignment_before_every_local_read() {
    for source in [
        "fun f(flag: Boolean): Int { var value: Int; if (flag) { value = 1 } else { value = 2 }; return value }",
        "fun f(flag: Boolean): Int { var value: Int; if (flag) { throw IllegalArgumentException() } else { value = 2 }; return value }",
        "fun f(flag: Boolean): Int { var value: Int; if (flag) { value = 1 } else { return 2 }; return value }",
    ] {
        assert!(compile(source, "Assigned.kt").is_ok(), "rejected {source}");
    }
    for source in [
        "fun f(): Int { var value: Int; return value }",
        "fun f(flag: Boolean): Int { var value: Int; if (flag) { value = 1 }; return value }",
        "fun f(flag: Boolean): Int { var value: Int; while (flag) { value = 1 }; return value }",
        "fun f(limit: Int): Int { var value: Int; for (index in 0 until limit) { value = index }; return value }",
        "fun f(): Int { var value: Int; value++; return 0 }",
        "fun f(): Int { var value: Int; value += 1; return 0 }",
        "fun f(): Int { var value: Int; value = value + 1; return value }",
    ] {
        let error = compile(source, "Unassigned.kt").err().expect(source);
        assert!(error.message.contains("initialized"), "{}", error.message);
    }
    assert!(compile("fun f() { var value }", "Unassigned.kt").is_err());
}

#[test]
fn rejects_unsupported_object_annotations_initializers_and_overloads() {
    for source in [
        "object E { @Unknown fun f() {} }",
        "object E { @JvmStatic @JvmStatic fun f() {} }",
        "object E { @Throws() fun f() {} }",
        "object E { @Throws(String::class) fun f() {} }",
        "object E { @Suppress(1) fun f() {} }",
        "object E { @Suppress(\"x\") @Suppress(\"y\") fun f() {} }",
        "object E { inline fun f() {} }",
        "object E { private inline inline fun f() {} }",
        "object E { internal fun f() {} }",
        "object E { private val bytes: ThreadLocal<ByteArray> = ThreadLocal.withInitial { ByteArray(1) } }",
        "object E { private val bytes = ThreadLocal.withInitial { value -> ByteArray(value) } }",
        "object E { private val bytes = ThreadLocal.withInitial { val count = 1; ByteArray(count) } }",
        "object E { private val bytes = ThreadLocal.withInitial { IntArray(1) } }",
        "object E { private val bytes = ThreadLocal.withInitial { ByteArray(missing) } }",
        "object E { const val C = 1; fun f() {} }",
        "object E { object Nested; fun f() {} }",
        "object E { object Nested { fun f(): Int = 1 } }",
        "object E { object Nested { private val bytes = ThreadLocal.withInitial { ByteArray(1) } } }",
        "object E { fun f(value: Int): Int = value; fun f(value: Int): String = \"x\" }",
    ] {
        assert!(
            compile_all(source, "Unsupported.kt").is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn rejects_invalid_loop_membership_increment_and_when_boundaries() {
    for source in [
        "fun f() { continue }",
        "fun f() { break }",
        "fun f(): Int { val value = 0; return value++ }",
        "fun f(value: Int): Int = ++value",
        "fun f(): Int { var value: Char = 'a'; value++; return 0 }",
        "fun f() { for (index in 0 until 2) { index++ } }",
        "fun f(value: Int): Boolean = value in 0..true",
        "fun f(value: Int): Int = when (value) { in 0..2 -> 1 }",
        "fun f(value: Int): Int = when (value) { in 0..2 -> 1; else -> 2; else -> 3 }",
        "fun f(value: Int): Int = when (value) { in 0..2 -> 1; else -> 2; 3 -> 4 }",
        "fun f(value: Int): Int = when (value) { in 0..2 -> true; else -> 2 }",
        "fun f(value: Int): Int = when (value) { in 0..2 -> { val x = 1; x }; else -> 2 }",
        "fun f(a: ByteArray) { a[true] = a[0] }",
    ] {
        assert!(
            compile_all(source, "Invalid.kt").is_err(),
            "accepted {source}"
        );
    }
}

#[test]
fn accepts_ten_bounded_androidbible_language_additions() {
    for source in [
        "fun f(a: Long, b: Int): Long { var n: Long = a + 1L; n = n shl b; return n xor Long.MIN_VALUE }",
        "class Entry(val timestamp: Long, val text: String, private val hidden: Int)",
        "data class Entry(val timestamp: Long, val text: String)",
        "fun f(value: String?): String? = value\nfun absent(): String? = null",
        "fun f(value: String?): String? = value?.removePrefix(\"x\")",
        "fun f(value: String?): String = value ?: \"missing\"",
        "fun f(values: List<String?>, index: Int): String = values[index] ?: \"missing\"",
        "fun f(value: String): Int = when (value) { \"a\", \"b\" -> 1; else -> 2 }\nfun g(value: Boolean): Int = when (value) { true -> 1; else -> 0 }",
        "fun f(value: Int): Int = when { value < 0 -> -1; value == 0 -> 0; else -> 1 }",
        r#"fun f(value: Int): String = "${if (value > 0) "positive=${value + 1}" else "none"}""#,
    ] {
        let result = compile(source, "Ten.kt");
        assert!(result.is_ok(), "rejected {source}: {:?}", result.err());
    }
}

#[test]
fn rejects_unsupported_or_invalid_ten_feature_boundaries() {
    for source in [
        "fun f(): Long = -9223372036854775808L",
        "fun f(): String = null",
        "fun f(value: String?): String = value.substring(0)",
        "data class Entry(var text: String)",
        "fun f(value: Int): Int = when { value, 1 -> 1; else -> 2 }",
    ] {
        assert!(
            compile(source, "InvalidTen.kt").is_err(),
            "accepted {source}"
        );
    }
    let too_many = (0..33)
        .map(|index| format!("val p{index}: Int"))
        .collect::<Vec<_>>()
        .join(", ");
    assert!(compile(&format!("data class Entry({too_many})"), "TooMany.kt").is_err());
}

#[test]
fn long_parameters_and_locals_enforce_jvm_slot_widths() {
    let parameters = (0..127)
        .map(|index| format!("p{index}: Long"))
        .collect::<Vec<_>>()
        .join(", ");
    assert!(
        compile(
            &format!("fun f({parameters}, flag: Int): Long = p126"),
            "StaticSlots.kt"
        )
        .is_ok()
    );
    assert!(
        compile(
            &format!("fun f({parameters}, extra: Long): Long = extra"),
            "StaticOverflow.kt"
        )
        .is_err()
    );
    assert!(
        compile(
            &format!(
                "fun f({parameters}, flag: Int): Long {{ val extra: Long = 0L; return extra }}"
            ),
            "LocalOverflow.kt"
        )
        .is_err()
    );
    let fields = (0..127)
        .map(|index| format!("private val p{index}: Long"))
        .collect::<Vec<_>>()
        .join(", ");
    assert!(compile(&format!("class Wide({fields})"), "Wide.kt").is_ok());
    assert!(
        compile(
            &format!("class Wide({fields}, private val extra: Long)"),
            "WideOverflow.kt"
        )
        .is_err()
    );
    assert!(
        compile(
            &format!("class Wide() {{ fun f({parameters}): Long = p126 }}"),
            "WideMethod.kt"
        )
        .is_ok()
    );
    assert!(
        compile(
            &format!("class Wide() {{ fun f({parameters}, extra: Long): Long = extra }}"),
            "WideMethodOverflow.kt"
        )
        .is_err()
    );
}

#[test]
fn accepts_twenty_bounded_language_additions() {
    for source in [
        "fun f(): Int = 1\nfun f(x: Int): Int = x",
        "const val VALUE: Int = 3\nfun f(): Int = VALUE",
        "class E",
        "class E(x: Int) { val field: Int = x }",
        "class E(private var x: Int)",
        "class E(private val x: Int = 1)",
        "class E() { val x = 1 }",
        "class E() { private val x: Int get() = 1 }",
        "class E() { val x: Int = 1 }",
        "fun f(value: Int?): Int? = value",
        "fun f(value: String?): Int? = value?.length",
        "fun f(values: List<Int>): Int = values.size",
        "fun f(): List<String> = listOf(\"a\")",
        "data class Entry(val text: String?)",
        "data class Entry(val text: String = \"default\")",
        "class Entry(var text: String)",
        "fun f(value: String?): Int { if (value == null) { return -1 }; return value.length }",
        "fun f(value: String?): Int { val text = value ?: return -1; return text.length }",
    ] {
        let result = compile_all(source, "Twenty.kt");
        assert!(result.is_ok(), "rejected {source}: {:?}", result.err());
    }
}

#[test]
fn newly_supported_operations_replace_previous_rejection_boundaries() {
    for source in [
        "fun f(a: CharArray, b: CharArray): Boolean = a == b",
        "fun f(s: String): Char = s[0]",
        "fun f(a: CharArray) { when (a) { else -> return } }",
        "fun f() { when (1) { is Int -> return } }",
    ] {
        assert!(compile(source, "Supported.kt").is_ok(), "rejected {source}");
    }
    let standard = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("references/kotlinc/lib/kotlin-stdlib.jar");
    if standard.is_file() {
        korrust::compile_with_classpath(
            "fun f() { for (index in 0..2) {} }",
            "Supported.kt",
            &[standard],
        )
        .unwrap();
    }
    for operator in ["-=", "*=", "/=", "%="] {
        let source = format!("fun f(): Int {{ var n = 12; n {operator} 2; return n }}");
        assert!(
            compile(&source, "Supported.kt").is_ok(),
            "rejected {source}"
        );
    }
}
