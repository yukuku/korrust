use korrust::{compile_all, compile_all_with_classpath};
use std::path::PathBuf;

fn rejects(source: &str) {
    assert!(
        compile_all(source, "Boundary.kt").is_err(),
        "Invalid Kotlin source accepted: {source}"
    );
}

#[test]
fn declaration_visibility_and_inheritance_boundaries() {
    for source in [
        "class Secret { private fun value(): Int = 1 }; fun bad(): Int = Secret().value()",
        "open class Secret { protected fun value(): Int = 1 }; fun bad(): Int = Secret().value()",
        "class Parent; class Child: Parent()",
        "open class Parent { fun value(): Int = 1 }; class Child: Parent() { override fun value(): Int = 2 }",
        "fun value(): Int = 1; fun value(): Int = 2",
    ] {
        rejects(source);
    }
}

#[test]
fn lateinit_restrictions_and_subject_scopes() {
    for source in [
        "class Bad { lateinit var value: Int }",
        "class Bad { lateinit val value: String }",
        "class Bad { lateinit var value: String = \"x\" }",
        "fun bad(): Int { when(val value = 1) { else -> value }; return value }",
        "fun outer() { fun hidden(): Int = 1 }; fun bad(): Int = hidden()",
        "fun bad(): Int {\n fun capture(): Int = later\n val later = 1\n return capture()\n}",
    ] {
        rejects(source);
    }
}

#[test]
fn operator_syntax_requires_operator_declarations() {
    for source in [
        "class Bag { fun contains(value: Int): Boolean = true }; fun bad(): Boolean = 1 in Bag()",
        "class Slots { fun get(index: Int): Int = index }; fun bad(): Int = Slots()[0]",
        "class Slots { fun set(index: Int, value: Int) {} }; fun bad() { Slots()[0] = 1 }",
    ] {
        rejects(source);
    }
}

#[test]
fn labels_and_backing_fields_obey_their_scope() {
    for source in [
        "fun bad() { while(true) { break@missing } }",
        "fun bad(): Int = field",
        "class Bad { val value: Int = 1\nget() { field = 2; return field } }",
    ] {
        rejects(source);
    }
}

#[test]
fn declaration_forms_compile_to_real_class_outputs() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    let classpath: Vec<PathBuf> = standard.is_file().then_some(standard).into_iter().collect();
    for source in [
        "fun literal(): Long = 0xff_ff80ab.toInt().toLong() + 0b1010_0101.toLong() + 1_000.toLong()",
        "fun before(): Int = Box(2).value; class Box(val value: Int); val top = 3; object State { val value = 4 }",
        "class Outer { class Nested(val value: Int); object Holder { val value = 8 } }",
        "class Initializer { val first = 1; init { first + 1 }; val second = 3; init { second + 1 } }",
        "fun local(seed: Int): Int { var calls = 0; fun recurse(n: Int): Int { calls += 1; if(n <= 1) return seed; return n * recurse(n - 1) }; return recurse(4) + calls }",
        "fun labeled(): Int { var total = 0; outer@ while(total < 10) { while(true) { total += 1; break@outer } }; return total }",
        "fun subject(input: Int): Int = when(val value = input) { 1 -> value + 10; else -> value + 20 }",
        "fun guarded(value: Int): Int = try { 20 / value } catch(e: ArithmeticException) { 7 } finally { value + 1 }",
        "class Accessor { var value: Int = 2; get() = field * 2; set(input) { field = input + 1 } }",
        "class Late { lateinit var text: String; fun size(): Int = text.length }",
        "open class Parent(val value: Int) { open fun compute(): Int = value + 1 }; class Child(value: Int): Parent(value) { override fun compute(): Int = super.compute() + 2 }",
        "class Secondary(val value: Int) { constructor(): this(4); constructor(text: String): this() }",
    ] {
        let result = compile_all_with_classpath(source, "Declarations.kt", &classpath);
        let outputs = result.unwrap_or_else(|error| panic!("{source}: {}", error.message));
        assert!(!outputs.is_empty(), "{source}");
        assert!(
            outputs
                .iter()
                .all(|output| output.bytes.starts_with(&[0xca, 0xfe, 0xba, 0xbe]))
        );
    }
}
