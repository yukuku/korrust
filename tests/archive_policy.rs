use korrust::{compile_all_with_classpath, compile_outputs_with_classpath};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "korrust-archive-policy-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
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

fn kotlin_command(
    source: &Path,
    output: &Path,
    standard: &Path,
    dependencies: &[&Path],
    target: &str,
) -> Command {
    let compiler = Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc");
    let mut libraries = fs::read_dir(compiler.join("lib"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "jar"))
        .collect::<Vec<_>>();
    libraries.sort();
    let mut dependencies = dependencies.to_vec();
    dependencies.push(standard);
    let mut command = Command::new("java");
    command
        .arg("-cp")
        .arg(std::env::join_paths(libraries).unwrap())
        .arg("org.jetbrains.kotlin.cli.jvm.K2JVMCompiler")
        .args([
            "-no-stdlib",
            "-no-reflect",
            "-language-version",
            "2.3",
            "-api-version",
            "2.3",
            "-jvm-target",
            target,
        ])
        .arg("-classpath")
        .arg(classpath(&dependencies))
        .arg("-d")
        .arg(output)
        .arg(source);
    command
}

fn kotlin(source: &Path, output: &Path, standard: &Path, dependencies: &[&Path]) {
    run(&mut kotlin_command(
        source,
        output,
        standard,
        dependencies,
        "1.8",
    ));
}

fn files(directory: &Path, prefix: &Path, entries: &mut Vec<(String, Vec<u8>)>) {
    for entry in fs::read_dir(directory.join(prefix)).unwrap() {
        let entry = entry.unwrap();
        let relative = prefix.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            files(directory, &relative, entries);
        } else {
            entries.push((
                relative.to_str().unwrap().replace('\\', "/"),
                fs::read(entry.path()).unwrap(),
            ));
        }
    }
}

fn jar(directory: &Path, path: &Path, compression: zip::CompressionMethod) {
    let mut entries = Vec::new();
    files(directory, Path::new(""), &mut entries);
    entries.sort();
    let mut archive = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        archive
            .start_file(
                name,
                zip::write::FileOptions::default().compression_method(compression),
            )
            .unwrap();
        archive.write_all(&bytes).unwrap();
    }
    archive.finish().unwrap();
}

fn write_outputs(directory: &Path, outputs: Vec<korrust::OutputFile>) {
    for output in outputs {
        let path = directory.join(output.relative_path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, output.bytes).unwrap();
    }
}

#[test]
fn official_kotlin_library_is_equivalent_in_directories_and_ordinary_jars() {
    let standard =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !standard.is_file() {
        return;
    }
    let temp = Temp::new();
    let library = temp.0.join("library");
    let source = temp.0.join("Library.kt");
    fs::write(
        &source,
        "package dep\nclass Widget(val value: Int)\nfun answer(value: Int = 7): Int = value + 2\nval marker: Int get() = 3",
    )
    .unwrap();
    kotlin(&source, &library, &standard, &[]);
    let descriptor = temp.0.join("module-info.java");
    fs::write(&descriptor, "module archive.library { exports dep; }").unwrap();
    run(Command::new("javac")
        .arg("--patch-module")
        .arg(format!("archive.library={}", library.display()))
        .arg("-d")
        .arg(&library)
        .arg(&descriptor));
    let versioned = library.join("META-INF/versions/9");
    fs::create_dir_all(&versioned).unwrap();
    fs::copy(
        library.join("module-info.class"),
        versioned.join("module-info.class"),
    )
    .unwrap();
    assert!(library.join("META-INF/main.kotlin_module").is_file());
    let stored = temp.0.join("stored.jar");
    let deflated = temp.0.join("deflated.jar");
    jar(&library, &stored, zip::CompressionMethod::Stored);
    jar(&library, &deflated, zip::CompressionMethod::Deflated);
    let consumer = "import dep.Widget\nimport dep.answer\nimport dep.marker\nfun probe(): Int = Widget(answer()).value + marker";
    let consumer_source = temp.0.join("Consumer.kt");
    fs::write(&consumer_source, consumer).unwrap();
    let reference = temp.0.join("reference");
    kotlin(&consumer_source, &reference, &standard, &[&library]);
    let check = temp.0.join("Check.java");
    fs::write(&check, "public class Check { public static void main(String[] args) { System.out.print(ConsumerKt.probe()); } }").unwrap();
    let runner = temp.0.join("runner");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&reference, &library, &standard]))
        .arg("-d")
        .arg(&runner)
        .arg(check));
    let expected = run(Command::new("java")
        .args(["-Xverify:all", "-cp"])
        .arg(classpath(&[&runner, &reference, &library, &standard]))
        .arg("Check"));
    assert_eq!(expected, "12");
    let mut previous = None;
    for (index, dependency) in [&library, &stored, &deflated].into_iter().enumerate() {
        let outputs = compile_outputs_with_classpath(
            consumer,
            "Consumer.kt",
            &[dependency.to_path_buf(), standard.clone()],
        )
        .unwrap_or_else(|error| panic!("{}: {}", dependency.display(), error.message));
        let bytes = outputs
            .iter()
            .map(|output| (output.relative_path.clone(), output.bytes.clone()))
            .collect::<Vec<_>>();
        if let Some(previous) = &previous {
            assert_eq!(&bytes, previous);
        }
        previous = Some(bytes);
        let output = temp.0.join(format!("candidate-{index}"));
        write_outputs(&output, outputs);
        assert_eq!(
            run(Command::new("java")
                .args(["-Xverify:all", "-cp"])
                .arg(classpath(&[&runner, &output, dependency, &standard]))
                .arg("Check")),
            expected
        );
        let missing = compile_all_with_classpath(
            "import dep.Absent\nfun probe(): Int = 1",
            "Missing.kt",
            &[dependency.to_path_buf(), standard.clone()],
        )
        .err()
        .unwrap();
        assert!(missing.message.contains("Absent"), "{}", missing.message);
    }
}

#[test]
fn duplicate_classes_preserve_first_classpath_entry_precedence() {
    let temp = Temp::new();
    let first = temp.0.join("first");
    let second = temp.0.join("second");
    for (path, ty) in [(&first, "Int"), (&second, "String")] {
        let source = format!("package dep\nclass Shared(val value: {ty})");
        write_outputs(
            path,
            compile_outputs_with_classpath(&source, "Shared.kt", &[]).unwrap(),
        );
    }
    let first_jar = temp.0.join("first.jar");
    let second_jar = temp.0.join("second.jar");
    jar(&first, &first_jar, zip::CompressionMethod::Stored);
    jar(&second, &second_jar, zip::CompressionMethod::Deflated);
    let consumer = "import dep.Shared\nfun probe(): Int = Shared(7).value";
    for first in [&first, &first_jar] {
        for second in [&second, &second_jar] {
            assert!(
                compile_all_with_classpath(
                    consumer,
                    "Consumer.kt",
                    &[first.to_path_buf(), second.to_path_buf()]
                )
                .is_ok()
            );
            assert!(
                compile_all_with_classpath(
                    consumer,
                    "Consumer.kt",
                    &[second.to_path_buf(), first.to_path_buf()]
                )
                .is_err()
            );
        }
    }
}

#[test]
fn multi_release_jars_use_the_pinned_kotlin_base_view() {
    let standard =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !standard.is_file() {
        return;
    }
    let temp = Temp::new();
    let library = temp.0.join("library");
    for version in [7, 9, 21, 99] {
        let source_dir = temp.0.join(format!("source-{version}"));
        fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("Shared.java");
        fs::write(&source, format!("package dep; public class Shared {{ public static final int VALUE = {version}; public static int live() {{ return {version}; }} }}")).unwrap();
        let destination = if version == 7 {
            library.clone()
        } else {
            library.join(format!("META-INF/versions/{version}"))
        };
        run(Command::new("javac")
            .args(["--release", "8", "-d"])
            .arg(&destination)
            .arg(&source));
        if version == 9 {
            let only = source_dir.join("Only.java");
            fs::write(
                &only,
                "package dep; public class Only { public static final int VALUE = 4; }",
            )
            .unwrap();
            run(Command::new("javac")
                .args(["--release", "8", "-d"])
                .arg(&destination)
                .arg(only));
        }
    }
    let consumer = "import dep.Shared\nfun probe(): Int = Shared.VALUE";
    let source = temp.0.join("Consumer.kt");
    fs::write(&source, consumer).unwrap();
    let missing = "import dep.Only\nfun probe(): Int = Only.VALUE";
    let missing_source = temp.0.join("Missing.kt");
    fs::write(&missing_source, missing).unwrap();
    let check = temp.0.join("Check.java");
    fs::write(&check, "public class Check { public static void main(String[] args) { System.out.print(ConsumerKt.probe() + \":\" + dep.Shared.live()); } }").unwrap();
    let reference = temp.0.join("reference");
    kotlin(&source, &reference, &standard, &[&library]);
    let runner = temp.0.join("runner");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&reference, &library]))
        .arg("-d")
        .arg(&runner)
        .arg(&check));
    let reference_signature = run(Command::new("javap")
        .args(["-p", "-classpath"])
        .arg(&reference)
        .arg("ConsumerKt"));
    assert!(reference_signature.contains("public static final int probe();"));
    let manifest = library.join("META-INF/MANIFEST.MF");
    for (case, contents) in [
        ("absent", None),
        (
            "false",
            Some("Manifest-Version: 1.0\nMulti-Release: false\n\n"),
        ),
        (
            "true",
            Some("Manifest-Version: 1.0\nMulti-Release: true\n\n"),
        ),
    ] {
        if let Some(contents) = contents {
            fs::write(&manifest, contents).unwrap();
        }
        let archive = temp.0.join(format!("{case}.jar"));
        jar(&library, &archive, zip::CompressionMethod::Deflated);
        for (kind, dependency) in [("directory", &library), ("jar", &archive)] {
            let output = temp.0.join(format!("candidate-{case}-{kind}"));
            write_outputs(
                &output,
                compile_outputs_with_classpath(
                    consumer,
                    "Consumer.kt",
                    std::slice::from_ref(dependency),
                )
                .unwrap(),
            );
            let candidate_signature = run(Command::new("javap")
                .args(["-p", "-classpath"])
                .arg(&output)
                .arg("ConsumerKt"));
            assert!(candidate_signature.contains("public static final int probe();"));
            let error =
                compile_all_with_classpath(missing, "Missing.kt", std::slice::from_ref(dependency))
                    .err()
                    .unwrap();
            assert!(error.message.contains("Only"), "{}", error.message);
            let expected = if case == "true" && kind == "jar" {
                "7:21"
            } else {
                "7:7"
            };
            assert_eq!(
                run(Command::new("java")
                    .args(["-Xverify:all", "-cp"])
                    .arg(classpath(&[&runner, &output, dependency, &standard]))
                    .arg("Check")),
                expected
            );
            assert_eq!(
                run(Command::new("java")
                    .args(["-Xverify:all", "-cp"])
                    .arg(classpath(&[&runner, &reference, dependency, &standard]))
                    .arg("Check")),
                expected
            );
        }
        for target in ["1.8", "21"] {
            let official = temp.0.join(format!("official-{case}-{target}"));
            run(&mut kotlin_command(
                &source,
                &official,
                &standard,
                &[&archive],
                target,
            ));
            assert_eq!(
                run(Command::new("java")
                    .args(["-Xverify:all", "-cp"])
                    .arg(classpath(&[&runner, &official, &archive, &standard]))
                    .arg("Check")),
                if case == "true" { "7:21" } else { "7:7" }
            );
            let rejected =
                kotlin_command(&missing_source, &official, &standard, &[&archive], target)
                    .output()
                    .unwrap();
            assert!(!rejected.status.success());
            assert!(
                String::from_utf8_lossy(&rejected.stderr).contains("unresolved reference 'Only'")
            );
        }
    }
}
