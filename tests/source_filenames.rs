use korrust::{
    DiagnosticKind, SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath,
};
use std::path::{Path, PathBuf};
use std::process::Command;

const ASCII_NAMES: &[(&str, &str)] = &[
    ("1.kt", "_1Kt"),
    ("2.2.kt", "_2_2Kt"),
    ("foo-bar.kt", "Foo_barKt"),
    ("foo_bar.kt", "Foo_barKt"),
    ("a$b.kt", "A_bKt"),
    (".kt", "_Kt"),
    ("123.kt", "_123Kt"),
    ("_hidden.kt", "_hiddenKt"),
    ("foo...bar.kt", "Foo___barKt"),
    ("a b.kt", "A_bKt"),
];

fn run(command: &mut Command) -> String {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn ascii_source_names_use_kotlin_facade_sanitization() {
    for (name, facade) in ASCII_NAMES {
        let outputs = compile_outputs_with_classpath("fun answer(): Int = 42", name, &[]).unwrap();
        assert!(
            outputs
                .iter()
                .any(|output| output.relative_path == format!("{facade}.class")),
            "{name}"
        );
        let outputs = compile_outputs_with_classpath(
            "private typealias Number = Int\nfun answer(): Number = 42",
            name,
            &[],
        )
        .unwrap();
        assert!(
            outputs
                .iter()
                .any(|output| output.relative_path == format!("{facade}.class"))
        );
    }
}

#[test]
fn unsupported_filename_forms_and_file_annotations_reject_explicitly() {
    for name in ["élève.kt", "中文.kt", "ßeta.kt", "𝕒lpha.kt"] {
        let error = compile_outputs_with_classpath("fun answer(): Int = 42", name, &[])
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "SOURCE_FILENAME_UNSUPPORTED");
        assert_eq!(error.source_identity.as_deref(), Some(name));
    }
    for name in ["directory/1.kt", "directory\\1.kt", "bad\0name.kt"] {
        let error = compile_outputs_with_classpath("fun answer(): Int = 42", name, &[])
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::Setup);
        assert_eq!(error.code, "INVALID_SOURCE_FILENAME");
    }
    for source in [
        "@file:JvmName(\"Chosen\")\nfun answer(): Int = 42",
        "@file:JvmMultifileClass\n@file:JvmName(\"Chosen\")\nfun answer(): Int = 42",
    ] {
        let error = compile_outputs_with_classpath(source, "1.kt", &[])
            .err()
            .unwrap();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "FILE_ANNOTATION_UNSUPPORTED");
    }
}

#[test]
fn sanitized_facade_collisions_preserve_deterministic_source_identity() {
    let inputs = [
        SourceInput {
            identity: "first/foo-bar.kt".into(),
            file_name: "foo-bar.kt".into(),
            source: "fun first(): Int = 1".into(),
        },
        SourceInput {
            identity: "second/foo_bar.kt".into(),
            file_name: "foo_bar.kt".into(),
            source: "fun second(): Int = 2".into(),
        },
    ];
    for inputs in [inputs.to_vec(), inputs.into_iter().rev().collect()] {
        let error = compile_batch_with_classpath(&inputs, &[], "filename_collision")
            .err()
            .unwrap();
        assert_eq!(error.source_identity.as_deref(), Some("second/foo_bar.kt"));
        assert_eq!(error.message, "Duplicate source declaration: Foo_barKt");
    }
}

#[test]
fn official_filename_facades_match_java_signatures_and_kotlin_consumers() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "korrust-source-filenames-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    let mut source_paths = Vec::new();
    let mut inputs = Vec::new();
    let mut facades = Vec::new();
    let unicode_names = [
        ("élève.kt", "ÉlèveKt"),
        ("中文.kt", "中文Kt"),
        ("ßeta.kt", "SSetaKt"),
        ("𝕒lpha.kt", "_𝕒lphaKt"),
    ];
    for (index, (name, facade)) in ASCII_NAMES.iter().chain(unicode_names.iter()).enumerate() {
        let directory = root.join(index.to_string());
        std::fs::create_dir_all(&directory).unwrap();
        let source = format!(
            "package filenames.case{index}\nfun answer(): Int = {}\n",
            42 + index
        );
        let source_path = directory.join(name);
        std::fs::write(&source_path, &source).unwrap();
        source_paths.push(source_path);
        if index < ASCII_NAMES.len() {
            inputs.push(SourceInput {
                identity: format!("{index}/{name}"),
                file_name: (*name).into(),
                source,
            });
            facades.push(format!("filenames.case{index}.{facade}"));
        }
    }
    run(Command::new(&kotlin)
        .args(&source_paths)
        .arg("-jvm-target")
        .arg("1.8")
        .arg("-d")
        .arg(&official));
    for (index, (_, facade)) in ASCII_NAMES.iter().chain(unicode_names.iter()).enumerate() {
        assert!(
            official
                .join(format!("filenames/case{index}/{facade}.class"))
                .is_file()
        );
    }
    for output in
        compile_batch_with_classpath(&inputs, std::slice::from_ref(&standard), "main").unwrap()
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let official_signatures = run(Command::new("javap")
        .arg("-public")
        .arg("-s")
        .arg("-classpath")
        .arg(&official)
        .args(&facades));
    let candidate_signatures = run(Command::new("javap")
        .arg("-public")
        .arg("-s")
        .arg("-classpath")
        .arg(&candidate)
        .args(&facades));
    assert_eq!(official_signatures, candidate_signatures);
    let driver = root.join("driver");
    let java = root.join("FilenameCheck.java");
    let assertions = facades
        .iter()
        .enumerate()
        .map(|(index, facade)| {
            format!(
                "if ({facade}.answer() != {}) throw new AssertionError(\"{facade}\");",
                42 + index
            )
        })
        .collect::<String>();
    std::fs::write(&java, format!("public final class FilenameCheck {{ public static void main(String[] args) {{ {assertions} if (FilenameConsumerKt.result() != 465) throw new AssertionError(\"Kotlin metadata\"); }} }}")).unwrap();
    let consumer = root.join("FilenameConsumer.kt");
    let imports = (0..ASCII_NAMES.len())
        .map(|index| format!("import filenames.case{index}.answer as answer{index}\n"))
        .collect::<String>();
    let calls = (0..ASCII_NAMES.len())
        .map(|index| format!("answer{index}()"))
        .collect::<Vec<_>>()
        .join(" + ");
    std::fs::write(&consumer, format!("{imports}fun result(): Int = {calls}\n")).unwrap();
    let consumers = [
        root.join("consumer-official"),
        root.join("consumer-candidate"),
    ];
    for (provider, destination) in [&official, &candidate].into_iter().zip(&consumers) {
        run(Command::new(&kotlin)
            .arg(&consumer)
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-cp")
            .arg(classpath(&[provider, &standard]))
            .arg("-d")
            .arg(destination));
    }
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
            .arg("FilenameCheck"));
    }
    std::fs::remove_dir_all(root).unwrap();
}
