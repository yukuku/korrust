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
fn ordinary_factories_and_builtin_constructor_contracts_match_the_pin() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-factories-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    let callbacks = root.join("Callbacks.java");
    std::fs::write(
        &callbacks,
        r#"package foreign;
public class Callbacks {
    public static char[] data;
    public static int offset;
    public static int count;
    public static String trace = "";
    public static char[] chars() { trace += "a"; return data; }
    public static int offset() { trace += "o"; return offset; }
    public static int count() { trace += "c"; return count; }
}"#,
    )
    .unwrap();
    run(Command::new("javac")
        .arg("-d")
        .arg(&dependency)
        .arg(&callbacks));
    let source = r#"package factories
import java.nio.charset.Charset
import java.lang.StringBuilder
import java.lang.StringBuffer
import java.lang.String as JavaString
import foreign.Callbacks
typealias Text = String
fun bytes(data: ByteArray, charset: Charset): String = String(data, charset)
fun byteSlice(data: ByteArray, offset: Int, length: Int, charset: Charset): String = String(data, offset, length, charset)
fun utf8(data: ByteArray): String = String(data)
fun utf8Slice(data: ByteArray, offset: Int, length: Int): String = String(data, offset, length)
fun chars(data: CharArray): String = String(data)
fun charSlice(data: CharArray, offset: Int, length: Int): String = String(data, offset, length)
fun points(data: IntArray, offset: Int, length: Int): String = String(data, offset, length)
fun builder(data: StringBuilder): String = String(data)
fun buffer(data: StringBuffer): String = String(data)
fun named(data: ByteArray, charset: Charset): String = String(bytes = data, charset = charset)
fun empty(): String = String()
fun aliasEmpty(): Text = Text()
fun javaCopy(data: String): JavaString = JavaString(data)
fun effects(bytes: () -> ByteArray, charset: () -> Charset): String = String(bytes(), charset())
fun platform(): String = String(Callbacks.chars(), Callbacks.offset(), Callbacks.count())
"#;
    let renamed = r#"package factory_aliases
import kotlin.text.String as makeText
import java.nio.charset.Charset
fun renamed(data: ByteArray, charset: Charset): String = makeText(data, charset)
"#;
    let source_file = root.join("Factories.kt");
    let renamed_file = root.join("Renamed.kt");
    std::fs::write(&source_file, source).unwrap();
    std::fs::write(&renamed_file, renamed).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_file)
        .arg(&renamed_file)
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    for (source, name) in [(source, "Factories.kt"), (renamed, "Renamed.kt")] {
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
    std::fs::write(&driver_source, r#"import java.nio.charset.StandardCharsets;
public class Driver {
    public static void main(String[] args) {
        byte[] bytes = {(byte) 0xc3, (byte) 0xa9, 32, 65};
        if (!factories.FactoriesKt.bytes(bytes, StandardCharsets.UTF_8).equals("\u00e9 A")) throw new AssertionError("UTF8 factory");
        if (!factories.FactoriesKt.byteSlice(bytes, 2, 2, StandardCharsets.US_ASCII).equals(" A")) throw new AssertionError("explicit charset slice");
        if (!factories.FactoriesKt.utf8(bytes).equals("\u00e9 A") || !factories.FactoriesKt.utf8Slice(bytes, 0, 2).equals("\u00e9")) throw new AssertionError("default UTF8");
        if (!factories.FactoriesKt.utf8(new byte[]{(byte) 0xc3, 0x28}).equals("\ufffd(")) throw new AssertionError("malformed UTF8");
        if (!factories.FactoriesKt.bytes(new byte[]{(byte) 0xff, 65}, StandardCharsets.US_ASCII).equals("\ufffdA")) throw new AssertionError("ASCII replacement");
        if (!factories.FactoriesKt.bytes(new byte[]{(byte) 0xff, 65}, StandardCharsets.ISO_8859_1).equals("\u00ffA")) throw new AssertionError("charset dispatch");
        char[] chars = {'a', '\ud83d', '\ude00', '\ud800', 'b'};
        if (!factories.FactoriesKt.chars(chars).equals("a\ud83d\ude00\ud800b") || !factories.FactoriesKt.charSlice(chars, 1, 3).equals("\ud83d\ude00\ud800")) throw new AssertionError("UTF16 preservation");
        if (!factories.FactoriesKt.points(new int[]{97, 0x1f600, 0xd800, 98}, 0, 4).equals("a\ud83d\ude00\ud800b")) throw new AssertionError("code points");
        try { factories.FactoriesKt.points(new int[]{0x110000}, 0, 1); throw new AssertionError("invalid code point"); } catch (IllegalArgumentException expected) {}
        try { factories.FactoriesKt.charSlice(chars, -1, 1); throw new AssertionError("char bounds"); } catch (StringIndexOutOfBoundsException expected) {}
        try { factories.FactoriesKt.byteSlice(bytes, 0, Integer.MAX_VALUE, StandardCharsets.UTF_8); throw new AssertionError("byte overflow"); } catch (StringIndexOutOfBoundsException expected) {}
        try { factories.FactoriesKt.bytes(null, StandardCharsets.UTF_8); throw new AssertionError("null bytes"); } catch (NullPointerException expected) {}
        try { factories.FactoriesKt.bytes(bytes, null); throw new AssertionError("null charset"); } catch (NullPointerException expected) {}
        StringBuilder builder = new StringBuilder("builder");
        StringBuffer buffer = new StringBuffer("buffer");
        String builderCopy = factories.FactoriesKt.builder(builder), bufferCopy = factories.FactoriesKt.buffer(buffer), charCopy = factories.FactoriesKt.chars(chars);
        builder.append('!'); buffer.append('!'); chars[0] = 'z';
        if (!builderCopy.equals("builder") || !bufferCopy.equals("buffer") || !charCopy.startsWith("a")) throw new AssertionError("copy snapshots");
        if (!factories.FactoriesKt.named(bytes, StandardCharsets.UTF_8).equals("\u00e9 A") || !factory_aliases.RenamedKt.renamed(bytes, StandardCharsets.UTF_8).equals("\u00e9 A")) throw new AssertionError("metadata names and renamed import");
        if (!factories.FactoriesKt.empty().isEmpty() || !factories.FactoriesKt.aliasEmpty().isEmpty()) throw new AssertionError("builtin constructors");
        String original = "copy";
        if (!factories.FactoriesKt.javaCopy(original).equals(original) || factories.FactoriesKt.javaCopy(original) == original) throw new AssertionError("explicit Java constructor");
        StringBuilder order = new StringBuilder();
        if (!factories.FactoriesKt.effects(() -> { order.append('b'); return bytes; }, () -> { order.append('c'); return StandardCharsets.UTF_8; }).equals("\u00e9 A") || !order.toString().equals("bc")) throw new AssertionError("factory argument effects");
        foreign.Callbacks.data = new char[]{'a', 'b'}; foreign.Callbacks.offset = 1; foreign.Callbacks.count = 1; foreign.Callbacks.trace = "";
        if (!factories.FactoriesKt.platform().equals("b") || !foreign.Callbacks.trace.equals("aoc")) throw new AssertionError("platform argument order");
        foreign.Callbacks.data = null; foreign.Callbacks.trace = "";
        try { factories.FactoriesKt.platform(); throw new AssertionError("platform assertion"); } catch (NullPointerException expected) { if (!expected.getMessage().equals("chars(...) must not be null")) throw new AssertionError(expected); }
        if (!foreign.Callbacks.trace.equals("a")) throw new AssertionError("platform null assertion ordering");
        System.out.println("factory-contracts-ok");
    }
}"#).unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&driver_source));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &dependency, &standard]))
            .arg("Driver"));
        assert_eq!(output.stdout, b"factory-contracts-ok\n");
    }
    for owner in ["factories.FactoriesKt", "factory_aliases.RenamedKt"] {
        let signatures = [&official, &candidate].map(|provider| {
            run(Command::new("javap")
                .arg("-public")
                .arg("-s")
                .arg("-classpath")
                .arg(classpath(&[provider, &dependency, &standard]))
                .arg(owner))
            .stdout
        });
        assert_eq!(signatures[0], signatures[1], "{owner}");
    }
    let consumer_source = root.join("Consumer.kt");
    std::fs::write(&consumer_source, "package consumer\nimport factories.Text\nimport factories.utf8\nfun value(bytes: ByteArray): String = utf8(bytes) + Text()\n").unwrap();
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
    std::fs::write(&metadata_source, "public class MetadataDriver { public static void main(String[] args) { if (!consumer.ConsumerKt.value(new byte[]{111, 107}).equals(\"ok\")) throw new AssertionError(); } }").unwrap();
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
    for source in [
        "fun invalid(): String = String(\"copy\")",
        "fun invalid(bytes: ByteArray): String = String(bytes, \"UTF-8\")",
        "fun invalid(bytes: ByteArray): String = String(bytes, 0)",
        "fun invalid(bytes: ByteArray): String = String(bytes, 0, 1, \"UTF-8\")",
        "typealias Text = String\nfun invalid(bytes: ByteArray): String = Text(bytes)",
        "fun invalid(): Boolean = Boolean(true)",
        "fun invalid(): Long = Long(1L)",
    ] {
        std::fs::write(&source_file, source).unwrap();
        let reference = Command::new(&kotlin)
            .arg(&source_file)
            .arg("-d")
            .arg(root.join("rejected"))
            .output()
            .unwrap();
        assert!(!reference.status.success(), "{source}");
        let error =
            compile_outputs_with_classpath(source, "Invalid.kt", std::slice::from_ref(&standard))
                .err()
                .expect("invalid Kotlin factory/constructor must reject");
        assert!(
            error.kind == DiagnosticKind::InvalidKotlin || error.message.contains("applicable"),
            "{error:?}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
