use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn annotated_java_and_mapped_builtin_calls_execute_actual_methods() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-library-contracts-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let dependency = root.join("dependency");
    std::fs::create_dir_all(&root).unwrap();
    let definitions = [
        (
            "contracts/Marker.java",
            "package contracts; import java.lang.annotation.*; @Retention(RetentionPolicy.CLASS) @Target({ElementType.TYPE,ElementType.METHOD,ElementType.CONSTRUCTOR,ElementType.PARAMETER}) public @interface Marker { String value(); }",
        ),
        (
            "android/annotation/NonNull.java",
            "package android.annotation; import java.lang.annotation.*; @Retention(RetentionPolicy.CLASS) @Target({ElementType.METHOD,ElementType.PARAMETER}) public @interface NonNull {}",
        ),
        (
            "android/annotation/Nullable.java",
            "package android.annotation; import java.lang.annotation.*; @Retention(RetentionPolicy.CLASS) @Target({ElementType.METHOD,ElementType.PARAMETER}) public @interface Nullable {}",
        ),
        (
            "contracts/Marked.java",
            "package contracts; @Marker(\"class\") public class Marked { public static int calls; @Marker(\"constructor\") public Marked() {} @Marker(\"method\") @android.annotation.Nullable public String echo(@Marker(\"parameter\") @android.annotation.Nullable String value) { calls++; return value; } @android.annotation.NonNull public String text(@android.annotation.NonNull String value) { calls++; return value; } }",
        ),
        (
            "contracts/Lookalike.java",
            "package contracts; public class Lookalike { public char charAt(int index) { return 'z'; } public String remove(int index) { return \"unrelated\"; } public int removeAt(int index) { return index + 70; } }",
        ),
        (
            "javax/annotation/meta/TypeQualifierNickname.java",
            "package javax.annotation.meta; import java.lang.annotation.*; @Retention(RetentionPolicy.RUNTIME) @Target(ElementType.ANNOTATION_TYPE) public @interface TypeQualifierNickname {}",
        ),
        (
            "javax/annotation/meta/TypeQualifierDefault.java",
            "package javax.annotation.meta; import java.lang.annotation.*; @Retention(RetentionPolicy.RUNTIME) @Target(ElementType.ANNOTATION_TYPE) public @interface TypeQualifierDefault { ElementType[] value(); }",
        ),
        (
            "contracts/Nickname.java",
            "package contracts; import java.lang.annotation.*; @javax.annotation.meta.TypeQualifierNickname @Retention(RetentionPolicy.CLASS) @Target(ElementType.METHOD) public @interface Nickname {}",
        ),
        (
            "contracts/Default.java",
            "package contracts; import java.lang.annotation.*; @javax.annotation.meta.TypeQualifierDefault({ElementType.PARAMETER}) @Retention(RetentionPolicy.CLASS) @Target(ElementType.TYPE) public @interface Default {}",
        ),
        (
            "contracts/Use.java",
            "package contracts; import java.lang.annotation.*; @Retention(RetentionPolicy.CLASS) @Target(ElementType.TYPE_USE) public @interface Use {}",
        ),
        (
            "contracts/Qualified.java",
            "package contracts; public class Qualified { @Nickname public String value() { return \"value\"; } public @Use String typed() { return \"typed\"; } }",
        ),
        (
            "contracts/Defaulted.java",
            "package contracts; @Default public class Defaulted { public String value(String value) { return value; } }",
        ),
    ];
    let mut javac = Command::new("javac");
    javac.arg("-d").arg(&dependency);
    for (name, source) in definitions {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, source).unwrap();
        javac.arg(path);
    }
    run(&mut javac);
    let source = r#"package contracts
import java.util.Random
import java.lang.StringBuilder
import contracts.Marked
import contracts.Lookalike
fun draw(seed: Long): Int = Random(seed).nextInt(10)
fun read(value: StringBuilder, index: Int): Char = value[index]
fun remove(values: MutableList<String>, index: Int): String = values.removeAt(index)
fun removeNullable(values: MutableList<String?>, index: Int): String? = values.removeAt(index)
fun removeInt(values: MutableList<Int>, index: Int): Int = values.removeAt(index)
fun removeWithEffects(receiver: () -> MutableList<String>, index: () -> Int): String = receiver().removeAt(index())
fun marker(value: String?): String? = Marked().echo(value)
fun text(value: Marked, input: String): String = value.text(input)
fun unrelated(value: Lookalike): Int = value.removeAt(3)
"#;
    let source_file = root.join("Calls.kt");
    std::fs::write(&source_file, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_file)
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    for output in
        compile_outputs_with_classpath(source, "Calls.kt", &[dependency.clone(), standard.clone()])
            .unwrap()
    {
        let path = candidate.join(&output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let driver_source = root.join("Driver.java");
    std::fs::write(&driver_source, r#"import java.util.*;
public class Driver {
    public static void main(String[] args) {
        if (contracts.CallsKt.draw(7L) != 6) throw new AssertionError("Random body");
        StringBuilder builder = new StringBuilder("a\ud83d\ude00z");
        if (contracts.CallsKt.read(builder, 1) != '\ud83d' || contracts.CallsKt.read(builder, 2) != '\ude00') throw new AssertionError("UTF16");
        ArrayList<String> values = new ArrayList<>(Arrays.asList("a", "b", "c"));
        if (!contracts.CallsKt.remove(values, 1).equals("b") || !values.equals(Arrays.asList("a", "c"))) throw new AssertionError("remove body");
        ArrayList<String> nullable = new ArrayList<>(Arrays.asList((String) null, "x"));
        if (contracts.CallsKt.removeNullable(nullable, 0) != null || !nullable.equals(Arrays.asList("x"))) throw new AssertionError("nullable removal");
        ArrayList<Integer> integers = new ArrayList<>(Arrays.asList(10, 20));
        if (contracts.CallsKt.removeInt(integers, 1) != 20 || !integers.equals(Arrays.asList(10))) throw new AssertionError("boxed removal");
        ArrayList<String> erasedNull = new ArrayList<>(Arrays.asList((String) null));
        if (contracts.CallsKt.remove(erasedNull, 0) != null || !erasedNull.isEmpty()) throw new AssertionError("erased null result");
        ArrayList<Integer> nullInteger = new ArrayList<>(Arrays.asList((Integer) null));
        try { contracts.CallsKt.removeInt(nullInteger, 0); throw new AssertionError("unbox null"); } catch (NullPointerException expected) {}
        if (!nullInteger.isEmpty()) throw new AssertionError("mutation before unbox failure");
        int[] dispatches = {0};
        ArrayList<String> overriding = new ArrayList<>(Arrays.asList("override")) {
            @Override public String remove(int index) { dispatches[0]++; return super.remove(index); }
        };
        StringBuilder effects = new StringBuilder();
        if (!contracts.CallsKt.removeWithEffects(() -> { effects.append('r'); return overriding; }, () -> { effects.append('i'); return 0; }).equals("override") || !effects.toString().equals("ri") || dispatches[0] != 1 || !overriding.isEmpty()) throw new AssertionError("dispatch and evaluation order");
        try { contracts.CallsKt.remove(values, 99); throw new AssertionError("index exception"); } catch (IndexOutOfBoundsException expected) {}
        try { contracts.CallsKt.read(builder, -1); throw new AssertionError("builder exception"); } catch (IndexOutOfBoundsException expected) {}
        contracts.Marked.calls = 0;
        if (contracts.CallsKt.marker(null) != null || !contracts.CallsKt.marker("a").equals("a")) throw new AssertionError("passive annotations");
        if (!contracts.CallsKt.text(new contracts.Marked(), "text").equals("text") || contracts.Marked.calls != 3) throw new AssertionError("annotated body calls");
        if (contracts.CallsKt.unrelated(new contracts.Lookalike()) != 73) throw new AssertionError("unrelated method");
        System.out.println("library-contracts-ok");
    }
}"#).unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&driver_source));
    let signatures = [&official, &candidate].map(|provider| {
        run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-classpath")
            .arg(classpath(&[provider, &dependency, &standard]))
            .arg("contracts.CallsKt"))
        .stdout
    });
    assert_eq!(signatures[0], signatures[1]);
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &dependency, &standard]))
            .arg("Driver"));
        assert_eq!(output.stdout, b"library-contracts-ok\n");
    }
    for source in [
        "import contracts.Qualified\nfun value(input: Qualified): String = input.value()",
        "import contracts.Qualified\nfun value(input: Qualified): String = input.typed()",
        "import contracts.Defaulted\nfun value(input: Defaulted): String = input.value(\"x\")",
    ] {
        let error = compile_outputs_with_classpath(
            source,
            "Boundary.kt",
            &[dependency.clone(), standard.clone()],
        )
        .err()
        .expect("Unhandled annotation semantics must be rejected");
        assert!(
            error.kind == DiagnosticKind::UnsupportedLanguage
                || error.message.contains("unsupported"),
            "{error:?}"
        );
    }
    for source in [
        "import contracts.Marked\nfun value(input: Marked): String = input.text(null)",
        "import contracts.Lookalike\nfun value(input: Lookalike): Char = input[0]",
        "import java.lang.CharSequence\nfun value(input: CharSequence): Char = input[0]",
    ] {
        std::fs::write(&source_file, source).unwrap();
        let rejected = Command::new(&kotlin)
            .arg(&source_file)
            .arg("-cp")
            .arg(classpath(&[&dependency, &standard]))
            .arg("-d")
            .arg(root.join("rejected"))
            .output()
            .unwrap();
        assert!(!rejected.status.success());
        assert!(
            compile_outputs_with_classpath(
                source,
                "Boundary.kt",
                &[dependency.clone(), standard.clone()],
            )
            .is_err()
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
