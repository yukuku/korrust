use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;

fn classpath() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn inferred_expression_bodies_accept_validated_public_inline_only_calls() {
    for source in [
        "fun present(s: String) = s.isNotEmpty()",
        "fun token(s: String) = s.isNotEmpty() && s[0] == '+'",
    ] {
        assert!(
            compile_outputs_with_classpath(source, "Probe.kt", &classpath()).is_ok(),
            "{source}"
        );
    }
}

#[test]
fn inference_does_not_expose_private_inline_declarations() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let directory =
        std::env::temp_dir().join(format!("korrust-private-inline-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let library = directory.join("library");
    let source = directory.join("Library.kt");
    std::fs::write(
        &source,
        "package dependency\nprivate inline fun hidden(s: String): Int = s.length\ninline fun visible(s: String): Int = s.length\n",
    )
    .unwrap();
    let output = std::process::Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(&source)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(&classpath()[0])
        .arg("-d")
        .arg(&library)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut dependencies = classpath();
    dependencies.push(library);
    let public_source = "import dependency.visible\nfun probe(s: String) = visible(s)";
    assert!(compile_outputs_with_classpath(public_source, "PublicProbe.kt", &dependencies).is_ok());
    let source = "import dependency.hidden\nfun probe(s: String) = hidden(s)";
    let error = compile_outputs_with_classpath(source, "Probe.kt", &dependencies)
        .err()
        .unwrap();
    assert!(
        error.message.contains("No applicable function overload"),
        "{error:?}"
    );
    std::fs::remove_dir_all(directory).unwrap();
}
