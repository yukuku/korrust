use korrust::compile_outputs_with_classpath;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn compiled_generic_extension_properties_preserve_inference_and_write_boundaries() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-property-inference-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let producer = root.join("Producer.kt");
    std::fs::write(&producer, "package producer\nvar <T> T.echo: T get() = this; set(value) {}\nval <T> T.readOnly: T get() = this\n").unwrap();
    let library = root.join("library");
    run(Command::new(&kotlin).arg(&producer).arg("-d").arg(&library));
    let dependencies = [library.clone(), standard.clone()];
    let read = "import producer.echo\nfun read(): String = \"OK\".echo\nfun javaInt(): Class<Int> = Int::class.java\n";
    assert!(compile_outputs_with_classpath(read, "Read.kt", &dependencies).is_ok());
    let write = "import producer.echo\nfun write() { \"OK\".echo = \"new\" }\n";
    let consumer = root.join("Consumer.kt");
    std::fs::write(
        &consumer,
        format!("{read}\n{}", write.replace("import producer.echo\n", "")),
    )
    .unwrap();
    run(Command::new(&kotlin)
        .arg(&consumer)
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg("-d")
        .arg(root.join("official")));
    let error = compile_outputs_with_classpath(write, "Write.kt", &dependencies)
        .err()
        .unwrap();
    assert_eq!(error.kind, korrust::DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "GENERIC_EXTENSION_SETTER_UNSUPPORTED");
    let invalid = "import producer.readOnly\nfun write() { \"OK\".readOnly = \"new\" }\n";
    std::fs::write(&consumer, invalid).unwrap();
    let oracle = Command::new(&kotlin)
        .arg(&consumer)
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg("-d")
        .arg(root.join("invalid"))
        .output()
        .unwrap();
    assert!(!oracle.status.success());
    assert!(String::from_utf8_lossy(&oracle.stderr).contains("cannot be reassigned"));
    let error = compile_outputs_with_classpath(invalid, "Invalid.kt", &dependencies)
        .err()
        .unwrap();
    assert_eq!(error.kind, korrust::DiagnosticKind::InvalidKotlin);
    assert_eq!(error.code, "EXTENSION_PROPERTY_SETTER_UNAVAILABLE");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn class_extensions_keep_dispatch_and_extension_receivers_distinct() {
    for source in [
        "class Host(val suffix: String) { fun String.decorate(): String = this + suffix; fun value(): String = \"O\".decorate() }",
        "class Host(val value: Int) { fun Host.same(): Boolean = this.value == this@Host.value; fun result(other: Host): Boolean = other.same() }",
        "class Host { fun String?.sizeOrZero(): Int = this?.length ?: 0; fun result(): Int = null.sizeOrZero() }",
        "class Host { fun String.amount(): Int = length; fun result(): Int = \"OK\".amount() }",
        "class Host { private fun String.hidden(): String = this; fun result(): String = \"OK\".hidden() }",
    ] {
        let result = compile_outputs_with_classpath(source, "Extensions.kt", &[]);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn source_extension_declaration_identity_includes_the_receiver_type() {
    let source =
        "fun Int.record(value: Int): Int = value\nfun String.record(value: Int): Int = value";
    let result = compile_outputs_with_classpath(source, "Identities.kt", &[]);
    assert!(result.is_ok(), "{:?}", result.err());
}

fn run(command: &mut Command) -> String {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn member_extensions_execute_with_distinct_receivers_and_virtual_dispatch() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "korrust-member-extension-runtime-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = include_str!("member_extensions/Dispatch.kt");
    let source_path = root.join("Extensions.kt");
    std::fs::write(&source_path, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_path)
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&official));
    for output in
        compile_outputs_with_classpath(source, "Extensions.kt", std::slice::from_ref(&standard))
            .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let consumer_source = root.join("MemberConsumer.kt");
    std::fs::write(
        &consumer_source,
        include_str!("member_extensions/MemberConsumer.kt"),
    )
    .unwrap();
    let consumers = [
        root.join("consumer-official"),
        root.join("consumer-candidate"),
    ];
    for (provider, output) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new(&kotlin)
            .arg(&consumer_source)
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-cp")
            .arg(classpath(&[provider, &standard]))
            .arg("-d")
            .arg(output));
    }
    let java = root.join("MemberCheck.java");
    std::fs::write(&java, include_str!("member_extensions/MemberCheck.java")).unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &consumers[0], &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&java));
    for (provider, consumer) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, consumer, &standard]))
            .arg("MemberCheck"));
    }
    assert!(compile_outputs_with_classpath("class Host { fun String.extension(): String = this }\nfun invalid(): String = Host().extension(\"wrong\")", "Invalid.kt", std::slice::from_ref(&standard)).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn escaping_lambdas_capture_each_receiver_and_shared_mutable_locals() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "korrust-extension-captures-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = include_str!("member_extensions/Captures.kt");
    let source_path = root.join("Captures.kt");
    std::fs::write(&source_path, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_path)
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&official));
    for output in
        compile_outputs_with_classpath(source, "Captures.kt", std::slice::from_ref(&standard))
            .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let java = root.join("CaptureCheck.java");
    std::fs::write(&java, include_str!("member_extensions/CaptureCheck.java")).unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&java));
    for provider in [&official, &candidate] {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &standard]))
            .arg("CaptureCheck"));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn member_extension_properties_and_defaults_compile() {
    for source in [
        "class Host(val suffix: String) { val String.decorated: String get() = this + suffix; fun value(): String = \"O\".decorated }",
        "class Host { var stored: Int = 0; var String.amount: Int get() = length + stored; set(value) { stored = value }; fun value(): Int { \"O\".amount = 2; return \"O\".amount } }",
        "val String.amount: Int get() = length\nfun value(): Int = \"OK\".amount",
        "object Host { fun String.decorate(): String = this + \"K\"; fun value(): String = \"O\".decorate() }",
        "class Host { companion object { fun String.decorate(): String = this + \"K\" }; fun value(): String = \"O\".decorate() }",
        "class Host { fun String.decorate(value: String = this): String = value; fun value(): String = \"OK\".decorate() }",
    ] {
        let result = compile_outputs_with_classpath(source, "Properties.kt", &[]);
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn properties_objects_companions_and_default_masks_match_official_consumers() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "korrust-extension-properties-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let source = include_str!("member_extensions/Properties.kt");
    let source_path = root.join("Properties.kt");
    std::fs::write(&source_path, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_path)
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&official));
    for output in
        compile_outputs_with_classpath(source, "Properties.kt", std::slice::from_ref(&standard))
            .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let consumer_source = root.join("PropertyConsumer.kt");
    std::fs::write(
        &consumer_source,
        include_str!("member_extensions/PropertyConsumer.kt"),
    )
    .unwrap();
    let consumers = [
        root.join("consumer-official"),
        root.join("consumer-candidate"),
    ];
    for (provider, output) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new(&kotlin)
            .arg(&consumer_source)
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-cp")
            .arg(classpath(&[provider, &standard]))
            .arg("-d")
            .arg(output));
    }
    let java = root.join("PropertyCheck.java");
    std::fs::write(&java, include_str!("member_extensions/PropertyCheck.java")).unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &consumers[0], &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&java));
    for (provider, consumer) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, consumer, &standard]))
            .arg("PropertyCheck"));
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unchanged_upstream_member_extensions_and_properties_match_box_results() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    for relative in [
        "extensionProperties/inClass.kt",
        "extensionProperties/inClassWithSetter.kt",
        "extensionProperties/inClassWithPrivateSetter.kt",
        "extensionProperties/topLevelLongTypeInReceiver.kt",
        "defaultArguments/function/extentionFunctionInClassObject.kt",
    ] {
        let original = project
            .join("references/kotlin/compiler/testData/codegen/box")
            .join(relative);
        let source = std::fs::read_to_string(&original).unwrap();
        let name = original.file_name().unwrap().to_str().unwrap();
        let mut facade = name.strip_suffix(".kt").unwrap().to_owned();
        facade[0..1].make_ascii_uppercase();
        facade.push_str("Kt");
        let root = std::env::temp_dir().join(format!(
            "korrust-extension-original-{}-{}",
            std::process::id(),
            facade
        ));
        std::fs::create_dir_all(&root).unwrap();
        let official = root.join("official");
        let candidate = root.join("candidate");
        run(Command::new(&kotlin)
            .arg(&original)
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-d")
            .arg(&official));
        for output in compile_outputs_with_classpath(&source, name, std::slice::from_ref(&standard))
            .unwrap_or_else(|error| panic!("{relative}: {error:?}"))
        {
            let path = candidate.join(output.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, output.bytes).unwrap();
        }
        let java = root.join("OriginalCheck.java");
        std::fs::write(&java, format!("public final class OriginalCheck {{ public static void main(String[] args) {{ if (!{facade}.box().equals(\"OK\")) throw new AssertionError(); }} }}")).unwrap();
        let driver = root.join("driver");
        run(Command::new("javac")
            .arg("-cp")
            .arg(classpath(&[&official, &standard]))
            .arg("-d")
            .arg(&driver)
            .arg(&java));
        for provider in [&official, &candidate] {
            run(Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(classpath(&[&driver, provider, &standard]))
                .arg("OriginalCheck"));
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn receiver_tower_labels_and_implicit_mutation_compile() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let source = "class Value(var amount: Int)\nclass Host { var amount: Int = 99; fun Value.update(): Int { amount = amount + 1; return amount + this@Host.amount }; fun result(): Int = Value(1).update() }\nfun labeled(): Int = with(Value(3)) { this@with.amount }\nclass Props { val String.amount: Int get() = this@amount.length }";
    let result =
        compile_outputs_with_classpath(source, "Tower.kt", std::slice::from_ref(&standard));
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn imported_named_companion_uses_declared_metadata_name() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!("korrust-named-extension-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let producer = root.join("producer");
    run(Command::new(&kotlin)
        .arg(project.join("tests/member_extensions/Named.kt"))
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&producer));
    let source = include_str!("member_extensions/NamedConsumer.kt");
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(project.join("tests/member_extensions/NamedConsumer.kt"))
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-cp")
        .arg(classpath(&[&producer, &standard]))
        .arg("-d")
        .arg(&official));
    for output in compile_outputs_with_classpath(
        source,
        "NamedConsumer.kt",
        &[producer.clone(), standard.clone()],
    )
    .unwrap()
    {
        let path = candidate.join(output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let java = root.join("NamedCheck.java");
    std::fs::write(&java,"public final class NamedCheck { public static void main(String[] args) { if (!NamedConsumerKt.box().equals(\"OKOKOKOK\")) throw new AssertionError(); } }").unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &producer, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&java));
    for provider in [&official, &candidate] {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &producer, &standard]))
            .arg("NamedCheck"));
    }
    std::fs::remove_dir_all(root).unwrap();
}
