use korrust::{CompileOptions, DiagnosticKind, SourceInput, compile_batch_with_options};
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

fn public_signatures(output: Output) -> Vec<(String, String)> {
    let text = String::from_utf8(output.stdout).unwrap();
    let mut declaration = String::new();
    let mut signatures = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with("public ") && line.ends_with(';') {
            declaration = line.into();
        } else if let Some(descriptor) = line.strip_prefix("descriptor: ") {
            assert!(!declaration.is_empty());
            signatures.push((std::mem::take(&mut declaration), descriptor.into()));
        }
    }
    signatures.sort();
    signatures
}

#[test]
fn value_names_use_kotlin_import_priority_and_constant_visibility() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-value-names-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let library = root.join("library");
    let mut library_sources = Vec::new();
    for (name, source) in [
        (
            "Lib.kt",
            "const val four = 2 + 2\ninternal const val internalCount = 7\ninternal object InternalSignal { @JvmField val code: Int = 79 }\nprivate const val privateCount = 8\nconst val utf16 = \"a\\ud800b\"\nobject Marker { const val answer = 19; @JvmField val text: String = \"object-field\"; @JvmField val absent: String? = null }\n",
        ),
        (
            "Near.kt",
            "package near\nconst val selected = 17\nobject Signal { @JvmField val code: Int = 41 }\n",
        ),
        (
            "Imported.kt",
            "package imported\nconst val selected = 23\nconst val clash = 1\nobject Signal { @JvmField val code: Int = 43 }\n",
        ),
        (
            "Other.kt",
            "package other\nconst val selected = 31\nconst val clash = 2\nobject Signal { @JvmField val code: Int = 47 }\n",
        ),
    ] {
        let path = root.join(name);
        std::fs::write(&path, source).unwrap();
        library_sources.push(path);
    }
    run(Command::new(&kotlin)
        .args(&library_sources)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(&standard)
        .arg("-module-name")
        .arg("value_library")
        .arg("-d")
        .arg(&library));
    let sources = [
        (
            "Use.kt",
            r#"fun result(): Int = four
const val eight = four * 2
fun folded(): Int = eight
fun objectValue(): Int = Marker.answer
fun objectField(): String = Marker.text
fun nullableField(): String? = Marker.absent
fun charset(): String = String(byteArrayOf(65, 66), Charsets.US_ASCII)
fun utf16Value(): String = utf16
fun local(four: Int): Int { val capture: () -> Int = { four }; return capture() }
fun objectShadow(Marker: Int): Int = Marker
class Scope(val four: Int) { fun result(): Int = four }
class Holder(val code: Int)
fun localQualifier(Signal: Holder): Int = Signal.code
fun captureQualifier(Signal: Holder): Int { val capture: () -> Int = { Signal.code }; return capture() }
fun defaultShadow(Charsets: Holder): Int = Charsets.code
"#,
        ),
        (
            "Priority.kt",
            "package near\nimport imported.selected\nimport imported.Signal\nfun explicitValue(): Int = selected\nfun explicitObject(): Int = Signal.code\n",
        ),
        (
            "Star.kt",
            "package near\nimport imported.*\nfun samePackage(): Int = selected\nfun sameObject(): Int = Signal.code\n",
        ),
        (
            "Alias.kt",
            "package near\nimport imported.selected as alternate\nimport imported.Signal as OtherSignal\nfun renamed(): Int = alternate\nfun original(): Int = selected\nfun objectRenamed(): Int = OtherSignal.code\n",
        ),
        (
            "DefaultImport.kt",
            "package named\nimport four as count\nfun defaultPackageImport(): Int = count\n",
        ),
    ];
    let official = root.join("official");
    let candidate = root.join("candidate");
    let mut inputs = Vec::new();
    let mut files = Vec::new();
    for (name, source) in sources {
        let path = root.join(name);
        std::fs::write(&path, source).unwrap();
        files.push(path.clone());
        inputs.push(SourceInput {
            identity: path.display().to_string(),
            file_name: name.into(),
            source: source.into(),
        });
    }
    run(Command::new(&kotlin)
        .args(&files)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg("-module-name")
        .arg("value_consumer")
        .arg("-d")
        .arg(&official));
    let options = CompileOptions {
        module_name: "value_consumer".into(),
        friend_paths: Vec::new(),
    };
    for output in
        compile_batch_with_options(&inputs, &[library.clone(), standard.clone()], &options).unwrap()
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let driver = root.join("Driver.java");
    std::fs::write(&driver, r#"public class Driver {
 public static void main(String[] args) {
  if (UseKt.result() != 4 || UseKt.folded() != 8 || UseKt.objectValue() != 19) throw new AssertionError("constants");
  if (!UseKt.objectField().equals("object-field") || UseKt.nullableField() != null || !UseKt.charset().equals("AB")) throw new AssertionError("object fields");
  String text = UseKt.utf16Value(); if (text.length() != 3 || text.charAt(1) != '\ud800') throw new AssertionError("UTF16");
  if (UseKt.local(53) != 53 || UseKt.objectShadow(59) != 59 || new Scope(61).result() != 61) throw new AssertionError("lexical shadowing");
  if (UseKt.localQualifier(new Holder(67)) != 67 || UseKt.captureQualifier(new Holder(71)) != 71 || UseKt.defaultShadow(new Holder(73)) != 73) throw new AssertionError("qualifier shadowing");
  if (near.PriorityKt.explicitValue() != 23 || near.PriorityKt.explicitObject() != 43) throw new AssertionError("explicit imports");
  if (near.StarKt.samePackage() != 17 || near.StarKt.sameObject() != 41) throw new AssertionError("same package");
  if (near.AliasKt.renamed() != 23 || near.AliasKt.original() != 17 || near.AliasKt.objectRenamed() != 43 || named.DefaultImportKt.defaultPackageImport() != 4) throw new AssertionError("renamed imports");
  System.out.println("value-resolution-ok");
 }
}"#).unwrap();
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &library, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(&driver));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, provider, &library, &standard]))
            .arg("Driver"));
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "value-resolution-ok"
        );
    }
    for name in [
        "UseKt",
        "Scope",
        "Holder",
        "near.PriorityKt",
        "near.StarKt",
        "near.AliasKt",
        "named.DefaultImportKt",
    ] {
        let reference = run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-cp")
            .arg(&official)
            .arg(name));
        let actual = run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-cp")
            .arg(&candidate)
            .arg(name));
        assert_eq!(
            public_signatures(reference),
            public_signatures(actual),
            "{name}"
        );
    }
    let consumer = root.join("MetadataConsumer.kt");
    std::fs::write(&consumer, "fun main() { check(folded() == 8); check(eight == 8); check(Scope(four = 5).result() == 5); check(near.explicitValue() == 23) }\n").unwrap();
    for (index, provider) in [&official, &candidate].iter().enumerate() {
        let output = root.join(format!("consumer-{index}"));
        run(Command::new(&kotlin)
            .arg(&consumer)
            .arg("-no-stdlib")
            .arg("-cp")
            .arg(classpath(&[provider, &library, &standard]))
            .arg("-d")
            .arg(&output));
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&output, provider, &library, &standard]))
            .arg("MetadataConsumerKt"));
    }
    for (index, source) in [
        "fun value(): Int = internalCount",
        "fun value(): Int = privateCount",
        "fun value(): Any = InternalSignal",
        "package missing\nfun value(): Int = four",
        "package missing\nimport imported.*\nimport other.*\nfun value(): Int = clash",
        "package missing\nimport imported.*\nimport other.*\nfun value(): Int = Signal.code",
        "package near\nimport near.selected as alternate\nfun value(): Int = selected",
        "package near\nimport near.selected as alternate\nconst val value = selected",
        "package missing\nconst val value = four",
    ]
    .iter()
    .enumerate()
    {
        let file = root.join(format!("Invalid{index}.kt"));
        std::fs::write(&file, source).unwrap();
        let output = Command::new(&kotlin)
            .arg(&file)
            .arg("-no-stdlib")
            .arg("-cp")
            .arg(classpath(&[&library, &standard]))
            .arg("-d")
            .arg(root.join(format!("invalid-reference-{index}")))
            .output()
            .unwrap();
        assert!(!output.status.success(), "oracle accepted {source}");
        let error = compile_batch_with_options(
            &[SourceInput {
                identity: file.display().to_string(),
                file_name: format!("Invalid{index}.kt"),
                source: (*source).into(),
            }],
            &[library.clone(), standard.clone()],
            &options,
        )
        .err()
        .expect("candidate must reject invalid lookup");
        assert_ne!(error.kind, DiagnosticKind::Setup, "{source}: {error:?}");
    }
    let friend_source = "fun value(): Int = internalCount\nconst val sum = internalCount + 1\n";
    let file = root.join("Friend.kt");
    std::fs::write(&file, friend_source).unwrap();
    run(Command::new(&kotlin)
        .arg(&file)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg(format!("-Xfriend-paths={}", library.display()))
        .arg("-d")
        .arg(root.join("friend-reference")));
    let friend_candidate = root.join("friend-candidate");
    for output in compile_batch_with_options(
        &[SourceInput {
            identity: file.display().to_string(),
            file_name: "Friend.kt".into(),
            source: friend_source.into(),
        }],
        &[library.clone(), standard.clone()],
        &CompileOptions {
            module_name: "friend_consumer".into(),
            friend_paths: vec![library.clone()],
        },
    )
    .unwrap()
    {
        let path = friend_candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let friend_driver = root.join("FriendDriver.java");
    std::fs::write(&friend_driver, "public class FriendDriver { public static void main(String[] args) { if (FriendKt.value() != 7 || FriendKt.sum != 8) throw new AssertionError(); } }").unwrap();
    run(Command::new("javac")
        .arg("-cp")
        .arg(root.join("friend-reference"))
        .arg("-d")
        .arg(&callers)
        .arg(&friend_driver));
    for provider in [root.join("friend-reference"), friend_candidate] {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&callers, &provider, &library, &standard]))
            .arg("FriendDriver"));
    }
    let internal_object = "fun value(): Any = InternalSignal\n";
    let object_file = root.join("InternalObject.kt");
    std::fs::write(&object_file, internal_object).unwrap();
    run(Command::new(&kotlin)
        .arg(&object_file)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg(format!("-Xfriend-paths={}", library.display()))
        .arg("-d")
        .arg(root.join("internal-object-reference")));
    let error = compile_batch_with_options(
        &[SourceInput {
            identity: object_file.display().to_string(),
            file_name: "InternalObject.kt".into(),
            source: internal_object.into(),
        }],
        &[library.clone(), standard],
        &CompileOptions {
            module_name: "object_consumer".into(),
            friend_paths: vec![library],
        },
    )
    .err()
    .expect("Compiled internal object value is an explicit boundary");
    assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
    assert_eq!(error.code, "INTERNAL_CLASSIFIER_VALUE_UNSUPPORTED");
    std::fs::remove_dir_all(root).unwrap();
}
