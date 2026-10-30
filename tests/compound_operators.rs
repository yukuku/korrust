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

fn signatures(bytes: &[u8]) -> Vec<(String, String)> {
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    let lines = text.lines().collect::<Vec<_>>();
    let mut result = Vec::new();
    for pair in lines.windows(2) {
        if pair[1].trim_start().starts_with("descriptor:") {
            result.push((pair[0].trim().into(), pair[1].trim().into()));
        }
    }
    result.sort();
    result
}

#[test]
fn compound_operators_and_unchanged_history_match_the_pinned_runtime() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-compound-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    let library_file = root.join("Library.kt");
    std::fs::write(
        &library_file,
        r#"package foreign
class Box(var value: Int) { operator fun plusAssign(delta: Int) { value += delta } }
operator fun Box.minusAssign(delta: Int) { value = value - delta }
"#,
    )
    .unwrap();
    run(Command::new(&kotlin)
        .arg(&library_file)
        .arg("-d")
        .arg(&dependency));
    let java_file = root.join("JavaBox.java");
    std::fs::write(&java_file, "package foreign; public class JavaBox { public int value = 3; public void timesAssign(int delta) { value *= delta; } }").unwrap();
    let bad_file = root.join("BadBox.java");
    std::fs::write(&bad_file, "package foreign; public class BadBox { public int plusAssign(int delta) { return delta; } }").unwrap();
    let trace_file = root.join("Trace.java");
    std::fs::write(&trace_file, "package foreign; public class Trace { public static String value = \"\"; public static void record(String part) { value += part; } public static int rhs() { record(\"r\"); return 2; } public static String text() { record(\"r\"); return \"b\"; } }").unwrap();
    run(Command::new("javac")
        .arg("-d")
        .arg(&dependency)
        .arg(&java_file)
        .arg(&bad_file)
        .arg(&trace_file));
    let source = r#"package compounds
import foreign.Box
import foreign.minusAssign
import foreign.JavaBox
object Events {
    var trace: String = ""
    fun record(value: String) { trace = trace + value }
    fun rhs(): Int { record("r"); return 3 }
    fun failed(): Int { record("r"); throw IllegalStateException("rhs") }
}
class Mutating(var value: Int) {
    operator fun plusAssign(delta: Int) { Events.record("a"); value += delta; if (delta == 99) throw IllegalStateException("operator") }
    operator fun minusAssign(delta: Int) { value -= delta }
    operator fun timesAssign(delta: Int) { value *= delta }
    operator fun divAssign(delta: Int) { value /= delta }
    operator fun remAssign(delta: Int) { value %= delta }
}
class Replacing(val value: Int) {
    operator fun plus(delta: Int): Replacing { Events.record("p"); return Replacing(value + delta) }
    operator fun minus(delta: Int): Replacing = Replacing(value - delta)
    operator fun times(delta: Int): Replacing = Replacing(value * delta)
    operator fun div(delta: Int): Replacing = Replacing(value / delta)
    operator fun rem(delta: Int): Replacing = Replacing(value % delta)
}
class Both(var value: Int) {
    operator fun plusAssign(delta: Int) { value += delta }
    operator fun plus(delta: Int): Both = Both(value + delta)
}
class ReadProperty(val saved: Mutating) {
    val target: Mutating get() { Events.record("g"); return saved }
    fun apply(): Int { target += Events.rhs(); return saved.value }
    fun fail() { target += Events.failed() }
}
class WriteProperty(initial: Replacing) {
    var saved: Replacing = initial
    var target: Replacing
        get() { Events.record("g"); return saved }
        set(value) { Events.record("s"); saved = value }
    fun apply(): Int { target += Events.rhs(); return saved.value }
}
class ExtensionBox(var value: Int)
operator fun ExtensionBox.plusAssign(delta: Int) { this.value = this.value + delta }
fun mutating(): Int { val value = Mutating(20); value += Events.rhs(); value -= 2; value *= 3; value /= 7; value %= 5; return value.value }
fun replacing(): Int { var value = Replacing(20); value += Events.rhs(); value -= 2; value *= 3; value /= 7; value %= 5; return value.value }
fun immutableBoth(): Int { val value = Both(5); value += 1; return value.value }
fun extension(value: ExtensionBox): Int { value += 4; return value.value }
fun compiled(value: Box): Int { value += 3; value -= 1; return value.value }
fun javaOperator(value: JavaBox): Int { value *= 4; return value.value }
fun operatorFailure(value: Mutating) { value += 99 }
fun captured(): () -> Int { var value = Replacing(5); return { value += 2; value.value } }
fun rebindAssign(): Int { var value = Mutating(1); val original = value; fun rhs(): Int { value = Mutating(100); return 2 }; value += rhs(); return value.value * 1000 + original.value }
fun rebindReplace(): Int { var value = Replacing(1); val original = value; fun rhs(): Int { value = Replacing(100); return 2 }; value += rhs(); return value.value * 1000 + original.value }
"#;
    let source_file = root.join("Compound.kt");
    std::fs::write(&source_file, source).unwrap();
    let history = project.join("benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/BackForwardList.kt");
    let history_source = std::fs::read_to_string(&history).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_file)
        .arg(&history)
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    for (source, name) in [
        (source, "Compound.kt"),
        (history_source.as_str(), "BackForwardList.kt"),
    ] {
        for output in
            compile_outputs_with_classpath(source, name, &[dependency.clone(), standard.clone()])
                .unwrap()
        {
            let path = candidate.join(output.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, output.bytes).unwrap();
        }
    }
    let driver_source = root.join("Driver.java");
    std::fs::write(&driver_source, r#"public class Driver {
    public static void main(String[] args) {
        compounds.Events.INSTANCE.setTrace("");
        if (compounds.CompoundKt.mutating() != 4 || !compounds.Events.INSTANCE.getTrace().equals("ra")) throw new AssertionError("mutating conventions");
        compounds.Events.INSTANCE.setTrace("");
        if (compounds.CompoundKt.replacing() != 4 || !compounds.Events.INSTANCE.getTrace().equals("rp")) throw new AssertionError("replacing conventions");
        if (compounds.CompoundKt.immutableBoth() != 6) throw new AssertionError("val assignment operator");
        compounds.Events.INSTANCE.setTrace("");
        compounds.ReadProperty read = new compounds.ReadProperty(new compounds.Mutating(10));
        if (read.apply() != 13 || !compounds.Events.INSTANCE.getTrace().equals("gra")) throw new AssertionError("receiver RHS operator order");
        compounds.Events.INSTANCE.setTrace("");
        compounds.WriteProperty write = new compounds.WriteProperty(new compounds.Replacing(10));
        if (write.apply() != 13 || !compounds.Events.INSTANCE.getTrace().equals("grps")) throw new AssertionError("get RHS replace set order");
        compounds.Events.INSTANCE.setTrace("");
        try { read.fail(); throw new AssertionError("RHS failure"); } catch (IllegalStateException expected) { if (!expected.getMessage().equals("rhs")) throw new AssertionError(expected); }
        if (read.getSaved().getValue() != 13 || !compounds.Events.INSTANCE.getTrace().equals("gr")) throw new AssertionError("RHS failure effects");
        compounds.Mutating failed = new compounds.Mutating(1);
        try { compounds.CompoundKt.operatorFailure(failed); throw new AssertionError("operator failure"); } catch (IllegalStateException expected) { if (!expected.getMessage().equals("operator")) throw new AssertionError(expected); }
        if (failed.getValue() != 100) throw new AssertionError("mutation before exception");
        if (compounds.CompoundKt.extension(new compounds.ExtensionBox(5)) != 9) throw new AssertionError("source extension");
        if (compounds.CompoundKt.compiled(new foreign.Box(10)) != 12) throw new AssertionError("compiled member and extension");
        if (compounds.CompoundKt.javaOperator(new foreign.JavaBox()) != 12) throw new AssertionError("Java convention");
        kotlin.jvm.functions.Function0<Integer> captured = compounds.CompoundKt.captured();
        if (captured.invoke() != 7 || captured.invoke() != 9) throw new AssertionError("captured replacement");
        if (compounds.CompoundKt.rebindAssign() != 100003) throw new AssertionError("assignment receiver captured before RHS rebinding");
        if (compounds.CompoundKt.rebindReplace() != 3001) throw new AssertionError("replacement receiver captured before RHS rebinding");
        yuku.alkitab.base.util.BackForwardList history = new yuku.alkitab.base.util.BackForwardList();
        if (history.getSize() != 0 || history.getCurrentIndex() != -1 || history.getCurrentEntry() != null) throw new AssertionError("empty history");
        history.newEntry(10); history.newEntry(20);
        if (history.getSize() != 2 || history.getCurrentIndex() != 1 || history.getCurrentEntry().getInitialAri() != 20) throw new AssertionError("append history");
        history.moveTo(0); history.updateCurrentEntry(15);
        if (history.getCurrentEntry().getInitialAri() != 10 || history.getCurrentEntry().getCurrentAri() != 15) throw new AssertionError("update history");
        history.newEntry(30);
        if (history.getSize() != 2 || history.getCurrentIndex() != 1 || history.getCurrentEntry().getInitialAri() != 30 || history.getEntry(0).getCurrentAri() != 15) throw new AssertionError("truncate forward history");
        history.purgeOthers();
        if (history.getSize() != 1 || history.getCurrentIndex() != 0 || history.getCurrentEntry().getInitialAri() != 30) throw new AssertionError("purge history");
        if (history.getEntry(-1) != null || history.getEntry(1) != null) throw new AssertionError("safe history bounds");
        history.moveTo(1); history.updateCurrentEntry(99);
        if (history.getCurrentEntry() != null || history.getSize() != 1) throw new AssertionError("empty current position");
        try { history.moveTo(-1); throw new AssertionError("invalid history index"); } catch (IllegalArgumentException expected) { if (!expected.getMessage().equals("wrong index given -1, must be in 0..1")) throw new AssertionError(expected); }
        System.out.println("compound-and-history-ok");
    }
}"#).unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&driver_source));
    for owner in [
        "compounds.CompoundKt",
        "yuku.alkitab.base.util.BackForwardList",
        "yuku.alkitab.base.util.BackForwardList$Entry",
    ] {
        let signatures = [&official, &candidate].map(|provider| {
            run(Command::new("javap")
                .arg("-public")
                .arg("-s")
                .arg("-classpath")
                .arg(classpath(&[provider, &dependency, &standard]))
                .arg(owner))
            .stdout
        });
        assert_eq!(
            self::signatures(&signatures[0]),
            self::signatures(&signatures[1]),
            "{owner}"
        );
    }
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &dependency, &standard]))
            .arg("Driver"));
        assert_eq!(output.stdout, b"compound-and-history-ok\n");
    }
    let consumer_source = root.join("Consumer.kt");
    std::fs::write(
        &consumer_source,
        r#"package consumer
import compounds.Mutating
import compounds.Replacing
import compounds.ExtensionBox
import compounds.plusAssign
import yuku.alkitab.base.util.BackForwardList
fun verify(): Int {
    val first = Mutating(3)
    first += 2
    var second = Replacing(4)
    second *= 2
    val third = ExtensionBox(1)
    third += 4
    val history = BackForwardList()
    history.newEntry(17)
    return first.value + second.value + third.value + (history.currentEntry?.initialAri ?: 0)
}
"#,
    )
    .unwrap();
    let consumers = [
        root.join("consumer-official"),
        root.join("consumer-candidate"),
    ];
    for (provider, consumer) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new(&kotlin)
            .arg(&consumer_source)
            .arg("-cp")
            .arg(classpath(&[provider, &dependency, &standard]))
            .arg("-d")
            .arg(consumer));
    }
    let metadata_source = root.join("MetadataDriver.java");
    std::fs::write(&metadata_source, "public class MetadataDriver { public static void main(String[] args) { if (consumer.ConsumerKt.verify() != 35) throw new AssertionError(); } }").unwrap();
    let metadata_driver = root.join("metadata-driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[
            &consumers[0],
            &official,
            &dependency,
            &standard,
        ]))
        .arg("-d")
        .arg(&metadata_driver)
        .arg(&metadata_source));
    for (provider, consumer) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[
                &metadata_driver,
                consumer,
                provider,
                &dependency,
                &standard,
            ]))
            .arg("MetadataDriver"));
    }
    for (source, ambiguity) in [
        (
            "class Both { operator fun plusAssign(value: Int) {}\noperator fun plus(value: Int): Both = this }\nfun invalid() { var value = Both(); value += 1 }",
            true,
        ),
        (
            "class ReadOnly { operator fun plus(value: Int): ReadOnly = this }\nfun invalid() { val value = ReadOnly(); value += 1 }",
            false,
        ),
        (
            "class Invalid { operator fun plusAssign(value: Int): Int = value }",
            false,
        ),
        (
            "import foreign.BadBox\nfun invalid(value: BadBox) { value += 1 }",
            false,
        ),
        (
            "class Plain { fun plusAssign(value: Int) {} }\nfun invalid(value: Plain) { value += 1 }",
            false,
        ),
    ] {
        std::fs::write(&source_file, source).unwrap();
        let official = Command::new(&kotlin)
            .arg(&source_file)
            .arg("-cp")
            .arg(classpath(&[&dependency, &standard]))
            .arg("-d")
            .arg(root.join("rejected"))
            .output()
            .unwrap();
        assert!(!official.status.success(), "{source}");
        let error = compile_outputs_with_classpath(
            source,
            "Invalid.kt",
            &[dependency.clone(), standard.clone()],
        )
        .err()
        .expect("invalid compound operation");
        if ambiguity {
            assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
            assert_eq!(error.code, "ASSIGN_OPERATOR_AMBIGUITY");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn primitive_and_string_assignment_extensions_obey_mutability_and_ambiguity() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-primitive-assign-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    let trace = root.join("Trace.java");
    std::fs::write(&trace, "package foreign; public class Trace { public static String value = \"\"; public static void record(String part) { value += part; } public static int rhs() { record(\"r\"); return 2; } public static String text() { record(\"r\"); return \"b\"; } }").unwrap();
    run(Command::new("javac").arg("-d").arg(&dependency).arg(&trace));
    let source = r#"package primitives
import foreign.Trace
operator fun Int.plusAssign(value: Int) { Trace.record("i") }
operator fun String.plusAssign(value: String) { Trace.record("s") }
fun intVal(): Int { val value = 7; value += Trace.rhs(); return value }
fun stringVal(): String { val value = "a"; value += Trace.text(); return value }
"#;
    let source_file = root.join("Primitives.kt");
    std::fs::write(&source_file, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_file)
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    for output in compile_outputs_with_classpath(
        source,
        "Primitives.kt",
        &[dependency.clone(), standard.clone()],
    )
    .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let driver_source = root.join("PrimitiveDriver.java");
    std::fs::write(&driver_source, "public class PrimitiveDriver { public static void main(String[] args) { foreign.Trace.value = \"\"; if (primitives.PrimitivesKt.intVal() != 7 || !foreign.Trace.value.equals(\"ri\")) throw new AssertionError(\"Int extension\"); foreign.Trace.value = \"\"; if (!primitives.PrimitivesKt.stringVal().equals(\"a\") || !foreign.Trace.value.equals(\"rs\")) throw new AssertionError(\"String extension\"); } }").unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&driver_source));
    for provider in [&official, &candidate] {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &dependency, &standard]))
            .arg("PrimitiveDriver"));
    }
    for (initializer, right) in [("7", "2"), ("\"a\"", "\"b\"")] {
        let invalid = format!(
            "import primitives.plusAssign\nfun invalid() {{ var value = {initializer}; value += {right} }}"
        );
        std::fs::write(&source_file, &invalid).unwrap();
        let reference = Command::new(&kotlin)
            .arg(&source_file)
            .arg("-cp")
            .arg(classpath(&[&official, &dependency, &standard]))
            .arg("-d")
            .arg(root.join("rejected"))
            .output()
            .unwrap();
        assert!(!reference.status.success(), "{invalid}");
        let error = compile_outputs_with_classpath(
            &invalid,
            "Invalid.kt",
            &[official.clone(), dependency.clone(), standard.clone()],
        )
        .err()
        .expect("mutable target ambiguity");
        assert_eq!(error.kind, DiagnosticKind::InvalidKotlin, "{error:?}");
        assert_eq!(error.code, "ASSIGN_OPERATOR_AMBIGUITY");
    }
    std::fs::remove_dir_all(root).unwrap();
}
