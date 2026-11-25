use korrust::compile_all_with_classpath;
use std::path::Path;
use std::process::Command;

fn success(command: &mut Command) -> Vec<u8> {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn compare(source: &str, caller: &str) {
    let project = Path::new(env!("CARGO_MANIFEST_DIR"));
    let compiler = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    assert!(compiler.is_file(), "Pinned Kotlin compiler required");
    let root = std::env::temp_dir().join(format!(
        "korrust-suspend-extended-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    let driver = root.join("driver");
    let input = root.join("Extended.kt");
    std::fs::write(&input, source).unwrap();
    success(
        Command::new(&compiler)
            .arg(&input)
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-d")
            .arg(&official),
    );
    let source = source.to_owned();
    let classpath = standard.clone();
    let classes = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || compile_all_with_classpath(&source, "Extended.kt", &[classpath]).unwrap())
        .unwrap()
        .join()
        .unwrap();
    for class in classes {
        let path = candidate.join(format!("{}.class", class.internal_name));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, class.bytes).unwrap();
    }
    let input = root.join("Calls.java");
    std::fs::write(&input, caller).unwrap();
    success(
        Command::new("javac")
            .arg("-cp")
            .arg(std::env::join_paths([&official, &standard]).unwrap())
            .arg("-d")
            .arg(&driver)
            .arg(&input),
    );
    for provider in [&official, &candidate] {
        let output = success(
            Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(std::env::join_paths([provider, &driver, &standard]).unwrap())
                .arg("Calls"),
        );
        assert_eq!(output, b"OK\n");
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn sixteen_suspensions_preserve_delayed_state_and_exception_identity() {
    let mut source = String::from(
        "suspend fun many(seed: Long, step: suspend (Int) -> Int): Long {\nval p0 = 0\n",
    );
    for index in 1..=16 {
        source.push_str(&format!("val p{index} = step(p{} + 1)\n", index - 1));
    }
    source.push_str("return seed + p16\n}");
    compare(
        &source,
        r#"
import kotlin.ResultKt;
import kotlin.coroutines.*;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function2;
public class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    static class Done implements Continuation<Long> {
        int count; Object result;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object value) { count++; result = value; }
    }
    static class Step implements Function2<Integer, Continuation<? super Integer>, Object> {
        int count; Continuation<? super Integer> pending; boolean immediate;
        public Object invoke(Integer value, Continuation<? super Integer> completion) {
            check(value == ++count); pending = completion;
            return immediate ? value : SUSPENDED;
        }
    }
    public static void main(String[] args) {
        Step quick = new Step(); quick.immediate = true; Done first = new Done();
        check(ExtendedKt.many(10000000000L, quick, first).equals(10000000016L));
        check(quick.count == 16 && first.count == 0);
        Step slow = new Step(); Done done = new Done();
        check(ExtendedKt.many(10000000000L, slow, done) == SUSPENDED);
        for (int i = 1; i <= 16; i++) { check(slow.count == i && done.count == 0); slow.pending.resumeWith(i); }
        check(done.count == 1 && done.result.equals(10000000016L));
        for (int failAt : new int[] {1, 9, 16}) {
            Step fail = new Step(); Done failed = new Done();
            check(ExtendedKt.many(10L, fail, failed) == SUSPENDED);
            for (int i = 1; i < failAt; i++) fail.pending.resumeWith(i);
            RuntimeException error = new RuntimeException("same");
            fail.pending.resumeWith(ResultKt.createFailure(error));
            check(failed.count == 1 && fail.count == failAt);
            try { ResultKt.throwOnFailure(failed.result); throw new AssertionError(); }
            catch (RuntimeException caught) { check(caught == error); }
        }
        System.out.println("OK");
    }
}
"#,
    );
}

#[test]
fn local_suspend_functions_keep_captures_continuations_and_failures() {
    compare(
        r#"
private suspend fun helper(step: suspend (Int) -> Int, value: Int): Int = step(value)
suspend fun local(seed: Int, step: suspend (Int) -> Int): Int {
    var saved = seed
    suspend fun next(value: Int): Int {
        val result = helper(step, value)
        saved += result
        return saved
    }
    val first = next(1)
    val second = next(first)
    return saved + second
}
suspend fun immediate(seed: Long, value: Long): Long {
    suspend fun identity(argument: Long): Long = argument + seed
    return identity(value)
}
abstract class Source { abstract fun read(): String }
class TextSource(val text: String): Source() { override fun read(): String = text }
suspend fun fetch(source: Source, step: suspend (String) -> String): String {
    suspend fun next(): String = step(source.read())
    return next()
}
class Host(val seed: Int) {
    suspend fun member(step: suspend (Int) -> Int): Int {
        suspend fun next(argument: Int): Int {
            val value = step(argument)
            return seed + value
        }
        return next(1)
    }
}
"#,
        r#"
import kotlin.ResultKt;
import kotlin.coroutines.*;
import kotlin.coroutines.intrinsics.IntrinsicsKt;
import kotlin.jvm.functions.Function2;
public class Calls {
    static final Object SUSPENDED = IntrinsicsKt.getCOROUTINE_SUSPENDED();
    static void check(boolean value) { if (!value) throw new AssertionError(); }
    static class Done implements Continuation<Integer> {
        int count; Object result;
        public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
        public void resumeWith(Object value) { count++; result = value; }
    }
    static class Step implements Function2<Integer, Continuation<? super Integer>, Object> {
        int count; Continuation<? super Integer> pending;
        public Object invoke(Integer value, Continuation<? super Integer> completion) {
            check(value == (count++ == 0 ? 1 : 13)); pending = completion; return SUSPENDED;
        }
    }
    public static void main(String[] args) {
        Step step = new Step(); Done done = new Done();
        check(ExtendedKt.local(10, step, done) == SUSPENDED);
        check(step.count == 1 && done.count == 0); step.pending.resumeWith(3);
        check(step.count == 2 && done.count == 0); step.pending.resumeWith(7);
        check(done.count == 1 && done.result.equals(40));
        Step fail = new Step(); Done failed = new Done();
        check(ExtendedKt.local(10, fail, failed) == SUSPENDED); fail.pending.resumeWith(3);
        RuntimeException error = new RuntimeException("local");
        fail.pending.resumeWith(ResultKt.createFailure(error));
        check(failed.count == 1 && fail.count == 2);
        try { ResultKt.throwOnFailure(failed.result); throw new AssertionError(); }
        catch (RuntimeException caught) { check(caught == error); }
        check(ExtendedKt.immediate(9L, 4L, null).equals(13L));
        Step memberStep = new Step(); Done memberDone = new Done();
        check(new Host(10).member(memberStep, memberDone) == SUSPENDED);
        memberStep.pending.resumeWith(3);
        check(memberDone.count == 1 && memberDone.result.equals(13));
        final Continuation<?>[] textPending = new Continuation<?>[1];
        final Object[] textResult = new Object[1];
        Continuation<String> textDone = new Continuation<String>() {
            public CoroutineContext getContext() { return EmptyCoroutineContext.INSTANCE; }
            public void resumeWith(Object value) { textResult[0] = value; }
        };
        check(ExtendedKt.fetch(new TextSource("verse"), (value, completion) -> {
            check(value.equals("verse")); textPending[0] = completion; return SUSPENDED;
        }, textDone) == SUSPENDED);
        textPending[0].resumeWith("OK"); check(textResult[0].equals("OK"));
        System.out.println("OK");
    }
}
"#,
    );
}

#[test]
fn local_suspend_boundaries_remain_explicit() {
    let standard =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    for source in [
        "fun broken(step: suspend () -> String): String { suspend fun inner(): String = step(); return inner() }",
        "fun broken(): String { suspend fun inner(): String = \"x\"; return inner() }",
    ] {
        let error =
            compile_all_with_classpath(source, "Broken.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(
            error.kind,
            korrust::DiagnosticKind::InvalidKotlin,
            "{source}: {}",
            error.message
        );
    }
}

#[test]
fn recursive_local_suspend_functions_reject_explicitly() {
    let standard =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    for source in [
        "suspend fun outer(): String { suspend fun again(): String = again(); return again() }",
        "suspend fun outer(): Int { suspend fun count(n: Int): Int { if (n == 0) return 0; return count(n - 1) }; return count(3) }",
    ] {
        let error =
            compile_all_with_classpath(source, "Recursive.kt", std::slice::from_ref(&standard))
                .err()
                .expect(source);
        assert_eq!(error.kind, korrust::DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "SUSPEND_LOCAL_RECURSION_UNSUPPORTED");
    }
}

#[test]
fn unsupported_local_suspend_declaration_shapes_reject_explicitly() {
    let standard =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    for source in [
        "fun outer(): Int { suspend fun inner(value: Int = 1): Int = value; return 1 }",
        "fun outer(): Int { suspend fun inner(vararg values: Int): Int = 1; return 1 }",
        "fun outer(): Int { suspend fun <T> inner(value: T): T = value; return 1 }",
        "fun outer(): Int { suspend fun String.inner(): String = this; return 1 }",
    ] {
        let error = compile_all_with_classpath(source, "Shape.kt", std::slice::from_ref(&standard))
            .err()
            .expect(source);
        assert_eq!(error.kind, korrust::DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "SUSPEND_LOCAL_DECLARATION_SHAPE_UNSUPPORTED");
    }
}

#[test]
fn anonymous_reader_uses_a_generic_capture_and_resumes_through_a_suspend_reference() {
    compare(
        include_str!("fixtures/language_interactions/Extended.kt"),
        include_str!("fixtures/language_interactions/Calls.java"),
    );
}
