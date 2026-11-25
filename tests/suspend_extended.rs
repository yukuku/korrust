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
    for class in
        compile_all_with_classpath(source, "Extended.kt", std::slice::from_ref(&standard)).unwrap()
    {
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
