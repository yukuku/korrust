use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::path::PathBuf;

#[test]
fn member_constants_compile_with_shared_source_references() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let cp = [root.join("references/kotlinc/lib/kotlin-stdlib.jar")];
            let sources = ["Models.kt", "Read.kt"].map(|name| SourceInput {
                identity: name.into(),
                file_name: name.into(),
                source: std::fs::read_to_string(root.join("tests/member_constants").join(name))
                    .unwrap(),
            });
            compile_batch_with_classpath(&sources, &cp, "memberconstants").unwrap();
            for source in [
                "class C { const val X = 1 }",
                "class C { companion object { const val X = 1 } }; fun bad(c: C): Int = c.X",
                "interface I { companion object { const val X = 1 } }; fun bad(i: I): Int = i.X",
                "import C.X\nclass C { companion object { const val X = 1 } }",
                "open class C { companion object { const val X = 1 } }; class D: C(); fun bad(): Int = D.X",
                "interface I { companion object { const val X = 1 } }; object O: I; fun bad(): Int = O.X",
                "class C { companion object { const var X = 1 } }",
                "class C { companion object { const val X: Int? = 1 } }",
                "class C { companion object { const val X = next(); fun next(): Int = 1 } }",
                "class C { companion object { const val X = 1; val X = 2 } }",
                "class C { companion object { const val X: String get() = \"x\" } }",
                "class C { companion object Named { const val X = 1 } }",
                "class C { val X = 1; companion object { const val X = 1 } }",
                "class C { companion object { const val Companion = 1 } }",
                "class C { companion object { @JvmField const val X = 1 } }",
            ] {
                assert!(
                    compile_outputs_with_classpath(source, "Invalid.kt", &cp).is_err(),
                    "{source}"
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

fn run(command: &mut std::process::Command) -> std::process::Output {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn cp(paths: &[&std::path::Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn member_constants_match_pinned_jvm_fields_and_independent_consumers() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            use std::process::Command;
            let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let fixtures = project.join("tests/member_constants");
            let root = project
                .join("target")
                .join(format!("member-constants-{}", std::process::id()));
            let official = root.join("official");
            let candidate = root.join("candidate");
            let callers = root.join("callers");
            let kotlin = project.join("references/kotlinc/bin/kotlinc");
            let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
            let options = [
                "-language-version",
                "2.3",
                "-api-version",
                "2.3",
                "-jvm-target",
                "1.8",
                "-no-stdlib",
                "-no-reflect",
            ];
            std::fs::create_dir_all(&root).unwrap();
            run(Command::new(&kotlin)
                .arg(fixtures.join("Models.kt"))
                .arg(fixtures.join("Read.kt"))
                .args(options)
                .arg("-cp")
                .arg(&standard)
                .arg("-d")
                .arg(&official));
            let sources = ["Models.kt", "Read.kt"].map(|name| SourceInput {
                identity: name.into(),
                file_name: name.into(),
                source: std::fs::read_to_string(fixtures.join(name)).unwrap(),
            });
            for result in compile_batch_with_classpath(
                &sources,
                std::slice::from_ref(&standard),
                "memberconstants",
            )
            .unwrap()
            {
                let path = candidate.join(result.output.relative_path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, result.output.bytes).unwrap();
            }
            run(Command::new("javac")
                .arg("-cp")
                .arg(cp(&[&official, &standard]))
                .arg("-d")
                .arg(&callers)
                .arg(fixtures.join("Calls.java")));
            for provider in [&official, &candidate] {
                let output = run(Command::new("java")
                    .args(["-Xverify:all", "-cp"])
                    .arg(cp(&[&callers, provider, &standard]))
                    .arg("constantprobe.Calls"));
                assert_eq!(output.stdout, b"OK\n");
            }
            let official_compiled = root.join("official-compiled");
            run(Command::new(&kotlin)
                .arg(fixtures.join("CompiledRead.kt"))
                .args(options)
                .arg("-cp")
                .arg(cp(&[&official, &standard]))
                .arg("-d")
                .arg(&official_compiled));
            run(Command::new("javac")
                .arg("-cp")
                .arg(cp(&[&official_compiled, &standard]))
                .arg("-d")
                .arg(&callers)
                .arg(fixtures.join("CompiledCalls.java")));
            let mut signatures = Vec::new();
            for provider in [&official, &candidate] {
                let mut rows = Vec::new();
                for owner in [
                    "constantprobe.Host",
                    "constantprobe.Host$Companion",
                    "constantprobe.Settings",
                    "constantprobe.Contract",
                    "constantprobe.Contract$Companion",
                    "constantprobe.Choice",
                    "constantprobe.Choice$Companion",
                ] {
                    let output = run(Command::new("javap")
                        .args(["-p", "-s", "-v", "-classpath"])
                        .arg(provider)
                        .arg(owner));
                    let mut field = String::new();
                    for line in String::from_utf8(output.stdout)
                        .unwrap()
                        .lines()
                        .map(str::trim)
                    {
                        if line.ends_with(';')
                            && !line.contains('(')
                            && ["public ", "private ", "protected "]
                                .iter()
                                .any(|prefix| line.starts_with(prefix))
                        {
                            field = line.into();
                        }
                        if line.starts_with("ConstantValue:") {
                            rows.push((owner.to_owned(), field.clone(), line.to_owned()));
                        }
                    }
                }
                rows.sort();
                signatures.push(rows);
            }
            assert_eq!(signatures[0], signatures[1]);
            for (label, provider) in [("official", &official), ("candidate", &candidate)] {
                for source in [
                    "const val BAD = constantprobe.Host.PRIVATE",
                    "const val BAD = constantprobe.Host.Companion.PRIVATE",
                    "const val BAD = constantprobe.Host.TIMEOUT",
                    "import constantprobe.Host.BASE\nconst val BAD = BASE",
                    "import constantprobe.Host\nfun bad(host: Host): Int = host.BASE",
                    "import constantprobe.Contract\nfun bad(value: Contract): Int = value.LIMIT",
                ] {
                    assert!(
                        compile_outputs_with_classpath(
                            source,
                            "InvalidCompiled.kt",
                            &[provider.to_path_buf(), standard.clone()]
                        )
                        .is_err(),
                        "{label}: {source}"
                    );
                }
                let consumer = root.join(format!("consumer-{label}"));
                run(Command::new(&kotlin)
                    .arg(fixtures.join("Consumer.kt"))
                    .args(options)
                    .arg("-cp")
                    .arg(cp(&[provider, &standard]))
                    .arg("-d")
                    .arg(&consumer));
                for runtime in [&official, &candidate] {
                    let output = run(Command::new("java")
                        .args(["-Xverify:all", "-cp"])
                        .arg(cp(&[&consumer, runtime, &standard]))
                        .arg("constantconsumer.ConsumerKt"));
                    assert_eq!(output.stdout, b"OK\n");
                }
                let compiled = root.join(format!("compiled-{label}"));
                for output in compile_outputs_with_classpath(
                    &std::fs::read_to_string(fixtures.join("CompiledRead.kt")).unwrap(),
                    "CompiledRead.kt",
                    &[provider.to_path_buf(), standard.clone()],
                )
                .unwrap()
                {
                    let path = compiled.join(output.relative_path);
                    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                    std::fs::write(path, output.bytes).unwrap();
                }
                let output = run(Command::new("java")
                    .args(["-Xverify:all", "-cp"])
                    .arg(cp(&[&callers, &compiled, provider, &standard]))
                    .arg("compiledconstantprobe.CompiledCalls"));
                assert_eq!(output.stdout, b"OK\n");
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
