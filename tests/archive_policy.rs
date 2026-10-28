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

fn kotlin(source: &Path, output: &Path, standard: &Path, dependencies: &[&Path]) {
    let compiler = Path::new(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc");
    let mut libraries = fs::read_dir(compiler.join("lib"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "jar"))
        .collect::<Vec<_>>();
    libraries.sort();
    let mut dependencies = dependencies.to_vec();
    dependencies.push(standard);
    run(Command::new("java")
        .arg("-cp")
        .arg(std::env::join_paths(libraries).unwrap())
        .arg("org.jetbrains.kotlin.cli.jvm.K2JVMCompiler")
        .args(["-no-stdlib", "-no-reflect", "-jvm-target", "1.8"])
        .arg("-classpath")
        .arg(classpath(&dependencies))
        .arg("-d")
        .arg(output)
        .arg(source));
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
fn genuine_multi_release_overrides_reject_before_dependency_resolution() {
    let temp = Temp::new();
    let library = temp.0.join("library");
    let outputs = compile_outputs_with_classpath(
        "package dep\nclass Shared(val value: Int)",
        "Shared.kt",
        &[],
    )
    .unwrap();
    write_outputs(&library, outputs);
    let versioned = library.join("META-INF/versions/9/dep");
    fs::create_dir_all(&versioned).unwrap();
    let override_outputs = compile_outputs_with_classpath(
        "package dep\nclass Shared(val value: String)",
        "Shared.kt",
        &[],
    )
    .unwrap();
    let override_class = override_outputs
        .into_iter()
        .find(|output| output.relative_path == "dep/Shared.class")
        .unwrap();
    fs::write(versioned.join("Shared.class"), override_class.bytes).unwrap();
    fs::write(
        library.join("META-INF/MANIFEST.MF"),
        "Manifest-Version: 1.0\nMulti-Release: true\n\n",
    )
    .unwrap();
    let archive = temp.0.join("library.jar");
    jar(&library, &archive, zip::CompressionMethod::Deflated);
    for path in [&library, &archive] {
        let error = compile_all_with_classpath(
            "import dep.Shared\nfun probe(): Int = Shared(7).value",
            "Consumer.kt",
            std::slice::from_ref(path),
        )
        .err()
        .unwrap();
        assert!(
            error
                .message
                .ends_with("multi-release JAR class overrides are unsupported"),
            "{}",
            error.message
        );
    }
    assert!(versioned.join("Shared.class").is_file());
}
