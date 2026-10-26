use korrust::compile_all_with_classpath;
use std::path::PathBuf;

#[test]
fn function_value_and_callable_reference_sam_adapters_reject_explicitly() {
    let standard =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    for (source, diagnostic) in [
        (
            "fun interface Mapper { fun apply(value: Int): Int }\nfun adapt(block: (Int) -> Int): Mapper = Mapper(block)",
            "Function-value SAM conversion is not supported",
        ),
        (
            "fun interface Mapper { fun apply(value: Int): Int }\nfun increment(value: Int): Int = value + 1\nfun adapt(): Mapper = Mapper(::increment)",
            "Callable-reference SAM conversion is not supported",
        ),
    ] {
        let error =
            compile_all_with_classpath(source, "SamBoundary.kt", std::slice::from_ref(&standard))
                .err()
                .expect("SAM adapters outside the literal-lambda scope must reject");
        assert!(error.message.contains(diagnostic), "{}", error.message);
    }
}

#[test]
fn inherited_sam_defaults_preserve_captures_for_java_and_kotlin_consumers() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let compiler = project.join("references/kotlinc/bin/kotlinc");
    let work = std::env::temp_dir().join(format!(
        "korrust-sam-inherited-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&work).unwrap();
    let source = r#"package d.sam
interface Parent { fun apply(value: Int): Int; fun inherited(value: Int): Int = apply(value) * 3 }
fun interface Child : Parent { fun own(value: Int): Int = apply(value) + 1 }
interface TextParent { fun apply(value: String): String; fun inherited(value: String): String = apply(value) + "!" }
fun interface TextChild : TextParent { fun own(value: String): String = apply(value) + "?" }
fun counter(seed: Int): Child { var total = seed; return Child { value -> total += value; total } }
fun prefixed(prefix: String): TextChild = TextChild { value -> prefix + value }
"#;
    let kotlin = work.join("Inherited.kt");
    std::fs::write(&kotlin, source).unwrap();
    let candidate = work.join("candidate");
    for output in korrust::compile_outputs_with_classpath(
        source,
        "Inherited.kt",
        std::slice::from_ref(&standard),
    )
    .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    fn run(command: &mut std::process::Command) -> std::process::Output {
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{command:?}\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
    let official = work.join("official");
    run(std::process::Command::new(&compiler)
        .arg(&kotlin)
        .args(["-language-version", "2.3", "-api-version", "2.3"])
        .args(["-jvm-target", "1.8", "-jvm-default=enable"])
        .arg("-d")
        .arg(&official));
    let java = work.join("InheritedCheck.java");
    std::fs::write(
        &java,
        r#"import d.sam.*;
public final class InheritedCheck {
 public static void main(String[] args) {
  Child counter=InheritedKt.counter(2);
  if(counter.apply(3)!=5 || counter.own(1)!=7 || counter.inherited(1)!=21 || counter.apply(0)!=7) throw new AssertionError("counter");
  TextChild text=InheritedKt.prefixed("P");
  if(!text.apply("x").equals("Px") || !text.own("x").equals("Px?") || !text.inherited("x").equals("Px!")) throw new AssertionError("text");
  System.out.println("OK");
 }
}
"#,
    )
    .unwrap();
    let callers = work.join("callers");
    let classpath = |paths: &[&std::path::Path]| std::env::join_paths(paths).unwrap();
    run(std::process::Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(&java));
    let consumer = work.join("Consumer.kt");
    std::fs::write(
        &consumer,
        r#"import d.sam.*
fun main() {
 val count = Child { value -> value * 2 }
 check(count.inherited(3) == 18 && count.own(3) == 7)
 val text = TextChild { value -> value + "K" }
 check(text.inherited("x") == "xK!" && text.own("x") == "xK?")
 println("OK")
}
"#,
    )
    .unwrap();
    for (name, provider) in [("official", &official), ("candidate", &candidate)] {
        let output = run(std::process::Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, provider, &standard]))
            .arg("InheritedCheck"));
        assert_eq!(output.stdout, b"OK\n");
        let destination = work.join(format!("{name}-consumer"));
        run(std::process::Command::new(&compiler)
            .arg(&consumer)
            .args(["-language-version", "2.3", "-api-version", "2.3"])
            .args(["-jvm-target", "1.8", "-jvm-default=enable"])
            .arg("-cp")
            .arg(classpath(&[provider, &standard]))
            .arg("-d")
            .arg(&destination));
        let output = run(std::process::Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&destination, provider, &standard]))
            .arg("ConsumerKt"));
        assert_eq!(output.stdout, b"OK\n");
    }
    std::fs::remove_dir_all(work).unwrap();
}
