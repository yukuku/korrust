use korrust::{SourceInput, compile_batch_with_classpath};
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

fn cp(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn inherited_classifiers_preserves_bounds_expected_types_matching_and_jvm_consumers() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(verify)
        .unwrap()
        .join()
        .unwrap();
}

fn verify() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = project.join("tests/inherited_classifiers");
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-inherited-classifiers-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let java = root.join("java");
    run(Command::new("javac")
        .arg("-d")
        .arg(&java)
        .arg(fixtures.join("JavaTypes.java")));
    let dependencies = root.join("dependencies");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Dependencies.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-cp",
        ])
        .arg(cp(&[&java, &standard]))
        .arg("-d")
        .arg(&dependencies));
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(fixtures.join("Declarations.kt"))
        .arg(fixtures.join("Explicit.kt"))
        .arg(fixtures.join("SamePackage.kt"))
        .arg(fixtures.join("PackageAccess.kt"))
        .args([
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            "1.8",
            "-cp",
        ])
        .arg(cp(&[&java, &dependencies, &standard]))
        .arg("-d")
        .arg(&official));
    let inputs = [
        "Declarations.kt",
        "Explicit.kt",
        "SamePackage.kt",
        "PackageAccess.kt",
    ]
    .iter()
    .map(|name| SourceInput {
        identity: (*name).into(),
        file_name: (*name).into(),
        source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
    })
    .collect::<Vec<_>>();
    for output in compile_batch_with_classpath(
        &inputs,
        &[java.clone(), dependencies.clone(), standard.clone()],
        "inherited_classifiers",
    )
    .unwrap_or_else(|error| panic!("{error:?}"))
    {
        let path = candidate.join(output.output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.output.bytes).unwrap();
    }
    let callers = root.join("callers");
    run(Command::new("javac")
        .arg("-cp")
        .arg(cp(&[&official, &java, &dependencies, &standard]))
        .arg("-d")
        .arg(&callers)
        .arg(fixtures.join("Calls.java")));
    let mut signatures = Vec::new();
    for (name, provider) in [("official", &official), ("candidate", &candidate)] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(cp(&[&callers, provider, &java, &dependencies, &standard]))
            .arg("inherited.consumer.Calls"));
        assert_eq!(output.stdout, b"inherited-classifiers-ok\n");
        signatures.push(
            run(Command::new("javap")
                .args(["-public", "-s", "-cp"])
                .arg(provider)
                .args([
                    "inherited.consumer.DeclarationsKt",
                    "inherited.consumer.Direct",
                    "inherited.consumer.SourceEnd",
                    "inherited.consumer.CompiledEnd",
                    "inherited.consumer.SuspendEnd",
                    "inherited.consumer.Closest",
                    "inherited.consumer.Own",
                    "inherited.consumer.TypeParameter",
                    "inherited.consumer.FunctionParameter",
                    "inherited.explicit.Derived",
                    "inherited.explicit.GenericDerived",
                    "inherited.same.Derived",
                ]))
            .stdout,
        );
        let consumer = root.join(format!("{name}-consumer"));
        run(Command::new(&kotlin)
            .arg(fixtures.join("Consumer.kt"))
            .arg("-cp")
            .arg(cp(&[provider, &java, &dependencies, &standard]))
            .arg("-d")
            .arg(&consumer));
        let runner = root.join(format!("{name}-runner"));
        run(Command::new("javac")
            .arg("-cp")
            .arg(cp(&[&consumer, &standard]))
            .arg("-d")
            .arg(&runner)
            .arg(fixtures.join("ConsumerCalls.java")));
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(cp(&[
                &runner,
                &consumer,
                provider,
                &java,
                &dependencies,
                &standard,
            ]))
            .arg("ConsumerCalls"));
        assert_eq!(output.stdout, b"consumer-ok\n");
    }
    assert_eq!(signatures[0], signatures[1]);
    std::fs::remove_dir_all(root).unwrap();
}
