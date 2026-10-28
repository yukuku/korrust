use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const NAVIGATION_CASES: &[(&str, &str)] = &[
    (
        "fun length(text: String): Int = text\n    .length",
        ".length",
    ),
    (
        "fun length(text: String?): Int? = text\n    ?.length",
        "?.length",
    ),
];
const SUPERTYPE_CASES: &[&str] = &[
    "class Names : Comparator<String> { override fun compare(left: String, right: String): Int = 0 }",
    "object Names : Comparator<String> { override fun compare(left: String, right: String): Int = 0 }",
    "interface Names : Comparator<String>",
    "interface Names : kotlin.Comparator<String>",
];
const ACCESSOR_CASES: &[(&str, &str)] = &[
    (
        "class State { var value: Int = 0\n    private set }",
        "private",
    ),
    (
        "class State { var value: Int = 0\n    protected set }",
        "protected",
    ),
    (
        "class State { var value: Int = 0\n    internal set }",
        "internal",
    ),
];
const CLASS_LITERAL_CASES: &[&str] = &[
    "fun stringClass() = String::class",
    "class ClassLiteralTarget\nfun sourceClass() = ClassLiteralTarget::class",
    "@file:OptIn(ExperimentalUnsignedTypes::class)\nfun unsignedValues(): UIntArray = uintArrayOf(1u)",
    "@OptIn(ExperimentalUnsignedTypes::class)\nfun unsignedValue(): UInt = 1u",
];
const UNSIGNED_TEST_SOURCE: &str = include_str!(
    "../benchmarks/sources/androidbible/AlkitabYes2/src/test/java/yuku/alkitab/yes2/util/UnsignedBinarySearchKtTest.kt"
);
const ORIGINALS: &[(&str, &str, &str, &str, usize)] = &[(
    "AlphanumComparator.kt",
    include_str!(
        "../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/util/AlphanumComparator.kt"
    ),
    "<String>",
    "PARAMETERIZED_SUPERTYPE_UNSUPPORTED",
    35,
)];

fn assert_unsupported_at(source: &str, offset: usize, code: &str) {
    let identity = "/original/UnsupportedSyntax.kt";
    let error = compile_batch_with_classpath(
        &[SourceInput {
            identity: identity.into(),
            file_name: "UnsupportedSyntax.kt".into(),
            source: source.into(),
        }],
        &[],
        "syntax_diagnostics",
    )
    .err()
    .expect("Unsupported source must return a diagnostic without output artifacts");
    assert_eq!(
        error.kind,
        DiagnosticKind::UnsupportedLanguage,
        "{}",
        error.message
    );
    assert_eq!(error.code, code, "{}", error.message);
    assert_eq!(error.source_identity.as_deref(), Some(identity));
    assert_eq!(error.offset, offset, "{}", error.message);
    let line = source[..offset]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column = source[..offset]
        .rsplit('\n')
        .next()
        .unwrap()
        .chars()
        .count()
        + 1;
    assert!(
        error
            .render("UnsupportedSyntax.kt", source)
            .starts_with(&format!("UnsupportedSyntax.kt:{line}:{column}: error:"))
    );
}

#[test]
fn ordinary_class_literals_report_the_keyword_at_the_original_source() {
    for &source in CLASS_LITERAL_CASES
        .iter()
        .chain(std::iter::once(&UNSIGNED_TEST_SOURCE))
    {
        assert_unsupported_at(
            source,
            source.find("::class").unwrap() + 2,
            "CLASS_LITERAL_UNSUPPORTED",
        );
    }
}

#[test]
fn unsigned_test_cli_reports_the_first_line_class_literal_without_outputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = root.join("benchmarks/sources/androidbible/AlkitabYes2/src/test/java/yuku/alkitab/yes2/util/UnsignedBinarySearchKtTest.kt");
    let destination = std::env::temp_dir().join(format!(
        "korrust-class-literal-output-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let result = Command::new(env!("CARGO_BIN_EXE_korrust"))
        .arg(&input)
        .arg("-d")
        .arg(&destination)
        .arg("--diagnostic-format")
        .arg("json")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(result.stdout.is_empty());
    let diagnostic = String::from_utf8(result.stderr).unwrap();
    for expected in [
        "\"category\":\"unsupported_language\"",
        "\"code\":\"CLASS_LITERAL_UNSUPPORTED\"",
        "\"offset\":39",
        "\"line\":1",
        "\"column\":40",
    ] {
        assert!(diagnostic.contains(expected), "{diagnostic}");
    }
    let identity = fs::canonicalize(input)
        .unwrap()
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    assert!(
        diagnostic.contains(&format!("\"source_identity\":\"{identity}\"")),
        "{diagnostic}"
    );
    assert!(!destination.exists());
}

#[test]
fn newline_qualified_expressions_compile() {
    for &(source, _) in NAVIGATION_CASES {
        let output = compile_batch_with_classpath(
            &[SourceInput {
                identity: "/original/Navigation.kt".into(),
                file_name: "Navigation.kt".into(),
                source: source.into(),
            }],
            &[],
            "navigation",
        )
        .unwrap();
        assert!(!output.is_empty());
    }
}

#[test]
fn parameterized_class_and_interface_supertypes_require_their_library() {
    for &source in SUPERTYPE_CASES {
        if source.starts_with("object ") {
            assert_unsupported_at(
                source,
                source.find('<').unwrap(),
                "PARAMETERIZED_SUPERTYPE_UNSUPPORTED",
            );
        } else {
            assert_missing_comparator(source);
        }
    }
}

fn assert_missing_comparator(source: &str) {
    let identity = "/original/MissingComparator.kt";
    let error = compile_batch_with_classpath(
        &[SourceInput {
            identity: identity.into(),
            file_name: "MissingComparator.kt".into(),
            source: source.into(),
        }],
        &[],
        "missing_comparator",
    )
    .err()
    .expect("Missing library class must not compile");
    assert_eq!(
        error.kind,
        DiagnosticKind::MissingDependency,
        "{}",
        error.message
    );
    assert!(error.message.contains("Comparator"), "{}", error.message);
    assert_eq!(error.source_identity.as_deref(), Some(identity));
}

#[test]
fn supported_accessor_visibility_compiles() {
    for &(source, _) in ACCESSOR_CASES {
        let result = compile_batch_with_classpath(
            &[SourceInput {
                identity: "/original/Visibility.kt".into(),
                file_name: "Visibility.kt".into(),
                source: source.into(),
            }],
            &[],
            "visibility",
        );
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}

#[test]
fn unchanged_androidbible_generic_source_without_its_classpath_reports_missing_dependency() {
    for &(_, source, _, _, _) in ORIGINALS {
        assert_missing_comparator(source);
    }
}

#[test]
fn pinned_official_compiler_accepts_reductions_and_unchanged_sources() {
    struct Scratch(PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let scratch = Scratch(std::env::temp_dir().join(format!(
        "korrust-unsupported-syntax-oracle-{}-{}", std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
    )));
    fs::create_dir(&scratch.0).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let compiler = root.join(if cfg!(windows) {
        "references/kotlinc/bin/kotlinc.bat"
    } else {
        "references/kotlinc/bin/kotlinc"
    });
    let mut command = Command::new(compiler);
    for (index, source) in NAVIGATION_CASES
        .iter()
        .map(|case| case.0)
        .chain(SUPERTYPE_CASES.iter().copied())
        .chain(ACCESSOR_CASES.iter().map(|case| case.0))
        .enumerate()
    {
        let path = scratch.0.join(format!("Reduction{index}.kt"));
        fs::write(
            &path,
            format!("package unsupported.syntax.reduction{index}\n{source}"),
        )
        .unwrap();
        command.arg(path);
    }
    for &(file_name, source, _, _, _) in ORIGINALS {
        let path = scratch.0.join(file_name);
        fs::write(&path, source).unwrap();
        command.arg(path);
    }
    for (index, source) in CLASS_LITERAL_CASES.iter().enumerate() {
        let path = scratch.0.join(format!("ClassLiteralReduction{index}.kt"));
        fs::write(&path, source).unwrap();
        command.arg(path);
    }
    let result = command
        .arg("-no-stdlib")
        .arg("-classpath")
        .arg(root.join("references/kotlinc/lib/kotlin-stdlib.jar"))
        .arg("-d")
        .arg(scratch.0.join("official"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
