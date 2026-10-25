use korrust::compile_all_with_classpath;
use std::path::PathBuf;

fn accepts(source: &str) {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    let classpath: Vec<PathBuf> = standard.is_file().then_some(standard).into_iter().collect();
    compile_all_with_classpath(source, "DeclarationsSyntax.kt", &classpath)
        .unwrap_or_else(|error| panic!("{}\n{source}", error.message));
}

#[test]
fn mixed_declarations_resolve_forward_references_and_inferred_results() {
    accepts(
        "fun before(): Int = Box(2).value + State.value + after()\nclass Box(val value: Int)\nval top = 3\nobject State { val value = 4 }\nfun after() = top",
    );
}

#[test]
fn initialized_state_and_nested_declarations_use_uniform_bodies() {
    accepts(
        "class Outer(val seed: Int) { val first = seed; init { first.toString() }; class Nested(val value: Int); object Holder { var value = 3 }; fun result(): Int = Nested(first).value + Holder.value; companion object { var count = 2; fun next(): Int { count += 1; return count } } }\nobject State { var count = 1; init { count += 1 }; fun next(): Int { count += 1; return count } }\nfun result(): Int = Outer(2).result() + State.next() + Outer.next()",
    );
}

#[test]
fn local_functions_destructuring_and_scoped_when_subjects_compile() {
    accepts(
        "data class Pair(val first: Int, val second: String)\nfun value(seed: Int): Int { var calls = 0; fun factorial(n: Int): Int { calls += 1; if (n <= 1) return seed; return n * factorial(n - 1) }; val (number, _) = Pair(factorial(3), \"unused\"); return when (val result = number + calls) { 10 -> result + 1; else -> result + 2 } }",
    );
}

#[test]
fn source_operator_calls_and_inheritance_preserve_declarations() {
    accepts(
        "open class Parent(val value: Int) { open fun answer(): Int = value }\nclass Child(value: Int): Parent(value) { override fun answer(): Int = super.answer() + 1; operator fun contains(other: Int): Boolean = other == value; operator fun get(index: Int): Int = value + index; operator fun set(index: Int, input: Int) {} }\nfun run(): Int { val child = Child(3); child[0] = 4; return if (3 in child) child[1] + child.answer() else 0 }",
    );
}

#[test]
fn custom_accessors_lateinit_and_secondary_constructors_compile() {
    accepts(
        "class Box(val initial: Int) { var value: Int = initial; get() = field * 2; set(input) { field = input + 1 }; lateinit var text: String; constructor(): this(2) { text = \"ready\" } }\nfun run(): Int { val box = Box(); box.value = 4; return box.value + box.text.length }",
    );
}

#[test]
fn labels_and_try_finally_compile_with_structured_blocks() {
    accepts(
        "fun run(): Int { var result = 0; outer@ while (true) { while (true) { result += 1; break@outer } }; return try { if (result < 0) throw IllegalStateException(); result } catch (error: IllegalStateException) { 2 } finally { result += 1 } }",
    );
}

#[test]
fn delegated_properties_resolve_source_operator_protocols() {
    accepts(
        "import kotlin.reflect.KProperty\nclass Cell(var value: Int) { operator fun provideDelegate(owner: Any?, property: KProperty<*>): Cell = this; operator fun getValue(owner: Any?, property: KProperty<*>): Int = value; operator fun setValue(owner: Any?, property: KProperty<*>, input: Int) { value = input } }\nclass Box { var score: Int by Cell(3) }\nfun run(): Int { val box = Box(); box.score = 8; return box.score }",
    );
}

#[test]
fn delegated_read_only_properties_do_not_require_provide_delegate() {
    accepts(
        "import kotlin.reflect.KProperty\nclass Cell(val value: Int) { operator fun getValue(owner: Any?, property: KProperty<*>): Int = value }\nclass Box { val score: Int by Cell(3) }\nfun run(): Int = Box().score",
    );
}

#[test]
fn invalid_delegate_operator_contracts_are_rejected() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    let classpath: Vec<PathBuf> = standard.is_file().then_some(standard).into_iter().collect();
    for source in [
        "import kotlin.reflect.KProperty\nclass Cell { fun getValue(owner: Any?, property: KProperty<*>): Int = 1 }\nclass Box { val score: Int by Cell() }",
        "import kotlin.reflect.KProperty\nclass Cell { operator fun getValue(owner: Any?, property: KProperty<*>): Int = 1; operator fun setValue(owner: Any?, property: KProperty<*>, input: Int): Int = input }\nclass Box { var score: Int by Cell() }",
        "import kotlin.reflect.KProperty\nclass Cell { operator fun getValue(owner: String, property: KProperty<*>): Int = 1 }\nclass Box { val score: Int by Cell() }",
        "import kotlin.reflect.KProperty\nclass Cell { operator fun provideDelegate(owner: Any?, property: KProperty<*>): Cell = this; operator fun provideDelegate(owner: Box, property: Any?): Cell = this; operator fun getValue(owner: Any?, property: KProperty<*>): Int = 1 }\nclass Box { val score: Int by Cell() }",
    ] {
        assert!(
            compile_all_with_classpath(source, "InvalidDelegate.kt", &classpath).is_err(),
            "Unexpected acceptance: {source}"
        );
    }
}

#[test]
fn when_subject_annotations_are_checked_before_binding() {
    accepts("fun value(): String = when(val subject: String = \"value\") { else -> subject }");
    assert!(
        compile_all_with_classpath(
            "fun value(): Int = when(val subject: String = 1) { else -> 2 }",
            "InvalidSubject.kt",
            &[],
        )
        .is_err()
    );
}
