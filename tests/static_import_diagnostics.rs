use korrust::{DiagnosticKind, SourceInput, compile_batch_with_classpath};
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn explicit_static_members_compile_and_missing_members_remain_dependencies() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    assert!(kotlin.is_file() && standard.is_file());
    let root = std::env::temp_dir().join(format!(
        "korrust-static-imports-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    let java = root.join("Utility.java");
    std::fs::write(&java, "package dep; public final class Utility { public static final int CONSTANT = 13; public static int twice(int value) { return value * 2; } public int instance(int value) { return value; } private static int hidden(int value) { return value; } }").unwrap();
    run(Command::new("javac").arg("-d").arg(&dependency).arg(&java));
    let library_source = root.join("Library.kt");
    let library = root.join("library");
    std::fs::write(
        &library_source,
        "package dep.library\ntypealias Count = Int\nfun doubleValue(value: Int): Int = value * 2\n",
    ).unwrap();
    run(Command::new(&kotlin)
        .arg(&library_source)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(&standard)
        .arg("-d")
        .arg(&library));
    let dependencies = [dependency.clone(), library.clone(), standard.clone()];
    let mut official_sources = Vec::new();
    for (index, source) in [
        "// λ🙂\n\nimport dep.Utility.twice\nfun value(): Int = twice(7)\n",
        "// λ🙂\n\nimport dep.Utility.twice as calculate\nfun value(): Int = calculate(7)\n",
        "// λ🙂\n\nimport dep.Utility.CONSTANT\nfun value(): Int = CONSTANT\n",
    ]
    .iter()
    .enumerate()
    {
        let input = SourceInput {
            identity: "/original/StaticImport.kt".into(),
            file_name: "StaticImport.kt".into(),
            source: (*source).into(),
        };
        let output = compile_batch_with_classpath(&[input], &dependencies, "static_imports")
            .unwrap_or_else(|error| panic!("{source}\n{}", error.message));
        assert!(!output.is_empty());
        let file = root.join(format!("StaticImport{index}.kt"));
        std::fs::write(&file, format!("package fixture.case{index}\n{source}")).unwrap();
        official_sources.push(file);
    }
    run(Command::new(&kotlin)
        .args(&official_sources)
        .arg("-no-stdlib")
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(root.join("official")));
    for path in ["absent.Library.twice", "dep.Utility.missing"] {
        let source = format!("import {path}\nfun value(): Int = 1\n");
        let error = compile_batch_with_classpath(
            &[SourceInput {
                identity: "/original/Missing.kt".into(),
                file_name: "Missing.kt".into(),
                source,
            }],
            &dependencies,
            "missing_imports",
        )
        .err()
        .expect("Unavailable import must reject");
        assert_eq!(
            error.kind,
            DiagnosticKind::MissingDependency,
            "{path}: {}",
            error.message
        );
        assert_eq!(error.code, "CLASS_NOT_FOUND");
        assert_eq!(error.offset, 7);
    }
    for member in ["instance", "hidden"] {
        let error = compile_batch_with_classpath(
            &[SourceInput {
                identity: "/original/Inaccessible.kt".into(),
                file_name: "Inaccessible.kt".into(),
                source: format!("import dep.Utility.{member}\nfun value(): Int = 1\n"),
            }],
            &dependencies,
            "inaccessible_imports",
        )
        .err()
        .expect("Inaccessible import must reject");
        assert_ne!(error.code, "STATIC_MEMBER_IMPORT_UNSUPPORTED");
    }
    for source in [
        "import dep.library.doubleValue\nfun value(): Int = doubleValue(7)",
        "import dep.library.Count\nfun value(value: Count): Int = value",
        "import dep.Utility\nfun value(): Int = Utility.twice(7)",
    ] {
        let output = compile_batch_with_classpath(
            &[SourceInput {
                identity: "/original/Supported.kt".into(),
                file_name: "Supported.kt".into(),
                source: source.into(),
            }],
            &dependencies,
            "supported_imports",
        )
        .unwrap_or_else(|error| panic!("{source}\n{}", error.message));
        assert!(!output.is_empty());
    }
    std::fs::remove_dir_all(root).unwrap();
}
