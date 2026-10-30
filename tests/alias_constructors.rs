use korrust::{DiagnosticKind, compile_outputs_with_classpath};
use std::path::{Path, PathBuf};
use std::process::Command;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn alias_constructors_expand_before_loading_classifiers() {
    for source in [
        "class C(val x: String)\ntypealias Alias = C\nfun box(): String = Alias(\"OK\").x",
        "class Pair<A, B>(val first: A, val second: B)\ntypealias ST<T> = Pair<String, T>\nfun box(): String { val pair = ST<String>(\"O\", \"K\"); return pair.first + pair.second }",
        "class Box<T>(val value: T)\ntypealias Fixed = Box<String>\nfun box(): String = Fixed(\"OK\").value",
        "class C(val x: String)\ntypealias Nullable = C?\nfun box(): String = Nullable(\"OK\").x",
        "class Box<T>(val value: T)\ntypealias Nullable<T> = Box<T>?\nfun box(): String = Nullable<String>(\"OK\").value",
    ] {
        let result = compile_outputs_with_classpath(source, "Aliases.kt", &classpath());
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn alias_constructor_inference_remains_an_explicit_language_boundary() {
    let error = compile_outputs_with_classpath(
        "class Box<T>(val value: T)\ntypealias Alias<T> = Box<T>\nfun box(): String = Alias(\"OK\").value",
        "Aliases.kt",
        &classpath(),
    ).err().unwrap();
    assert_eq!(
        error.kind,
        DiagnosticKind::UnsupportedLanguage,
        "{}",
        error.message
    );
    assert_eq!(error.code, "GENERIC_CONSTRUCTOR_INFERENCE_UNSUPPORTED");
}

#[test]
fn aliases_preserve_callables_bounds_and_private_constructors() {
    for source in [
        "class C(val x: Int)\ntypealias Alias = C\nfun Alias(value: String): String = value\nfun box(): String = Alias(\"OK\")",
        "class C(val x: Int)\ntypealias Alias = C\nfun box(): Int { val Alias: (Int) -> Int = { it + 1 }; return Alias(41) }",
    ] {
        let result = compile_outputs_with_classpath(source, "Aliases.kt", &classpath());
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
    for source in [
        "class C private constructor(val x: String)\ntypealias Alias = C\nfun box(): String = Alias(\"OK\").x",
        "class Box<T: Number>(val value: T)\ntypealias Alias<T> = Box<T>\nfun box(): Alias<String> = Alias<String>(\"bad\")",
    ] {
        assert!(
            compile_outputs_with_classpath(source, "Aliases.kt", &classpath()).is_err(),
            "{source}"
        );
    }
}

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

fn joined(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn unchanged_upstream_alias_constructors_match_runtime_signatures_and_metadata() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "korrust-alias-constructors-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for (name, facade, classes, consumer_body, java_body) in [
        (
            "typeAliasConstructor.kt",
            "TypeAliasConstructorKt",
            vec!["C"],
            "fun construct(): Alias = Alias(\"OK\")\nfun result(): String = construct().x",
            "if (!new C(\"OK\").getX().equals(\"OK\")) throw new AssertionError(\"constructor\");",
        ),
        (
            "genericTypeAliasConstructor2.kt",
            "GenericTypeAliasConstructor2Kt",
            vec!["Pair"],
            "fun construct(): ST<String> = ST<String>(\"O\", \"K\")\nfun result(): String = construct().x1 + construct().x2",
            "Pair<String, String> pair = new Pair<>(\"O\", \"K\"); if (!(pair.getX1() + pair.getX2()).equals(\"OK\")) throw new AssertionError(\"constructor\");",
        ),
    ] {
        let directory = root.join(name);
        std::fs::create_dir_all(&directory).unwrap();
        let fixture = project
            .join("references/kotlin/compiler/testData/codegen/box/typealias")
            .join(name);
        let source = std::fs::read_to_string(&fixture).unwrap();
        let official = directory.join("official");
        let candidate = directory.join("candidate");
        run(Command::new(&kotlin)
            .arg(&fixture)
            .arg("-jvm-target")
            .arg("1.8")
            .arg("-module-name")
            .arg("main")
            .arg("-d")
            .arg(&official));
        for output in
            compile_outputs_with_classpath(&source, name, std::slice::from_ref(&standard)).unwrap()
        {
            let path = candidate.join(output.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, output.bytes).unwrap();
        }
        let inspected = classes
            .iter()
            .copied()
            .chain(std::iter::once(facade))
            .collect::<Vec<_>>();
        let expected = run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-classpath")
            .arg(&official)
            .args(&inspected));
        let actual = run(Command::new("javap")
            .arg("-public")
            .arg("-s")
            .arg("-classpath")
            .arg(&candidate)
            .args(&inspected));
        assert_eq!(expected, actual, "{name}");
        let consumer = directory.join("AliasConsumer.kt");
        std::fs::write(&consumer, consumer_body).unwrap();
        let consumers = [
            directory.join("consumer-official"),
            directory.join("consumer-candidate"),
        ];
        for (provider, destination) in [&official, &candidate].into_iter().zip(&consumers) {
            run(Command::new(&kotlin)
                .arg(&consumer)
                .arg("-jvm-target")
                .arg("1.8")
                .arg("-cp")
                .arg(joined(&[provider, &standard]))
                .arg("-d")
                .arg(destination));
        }
        let driver = directory.join("driver");
        let java = directory.join("AliasCheck.java");
        std::fs::write(&java, format!("public final class AliasCheck {{ public static void main(String[] args) {{ if (!{facade}.box().equals(\"OK\")) throw new AssertionError(\"upstream box\"); if (!AliasConsumerKt.result().equals(\"OK\")) throw new AssertionError(\"metadata consumer\"); {java_body} }} }}")).unwrap();
        run(Command::new("javac")
            .arg("-cp")
            .arg(joined(&[&official, &consumers[0], &standard]))
            .arg("-d")
            .arg(&driver)
            .arg(&java));
        for (provider, consumer) in [&official, &candidate].into_iter().zip(&consumers) {
            run(Command::new("java")
                .arg("-Xverify:all")
                .arg("-cp")
                .arg(joined(&[&driver, provider, consumer, &standard]))
                .arg("AliasCheck"));
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}
