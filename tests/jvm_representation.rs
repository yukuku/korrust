use korrust::compile_outputs_with_classpath;
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

#[test]
fn interface_default_bridges_preserve_physical_widths() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    let root = std::env::temp_dir().join(format!(
        "korrust-jvm-representation-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency_source = root.join("Contracts.kt");
    std::fs::write(
        &dependency_source,
        r#"package representation
interface IntegerContract { fun value(): Int = 42 }
interface LongContract { fun value(): Long = 9000000001L }
interface DoubleContract {
    fun value(): Double = 3.5
    fun sum(first: Long, second: Double, third: Int): Double = first.toDouble() + second + third
}
"#,
    )
    .unwrap();
    let dependency = root.join("dependency");
    run(Command::new(&kotlin)
        .arg(&dependency_source)
        .arg("-jvm-default=enable")
        .arg("-d")
        .arg(&dependency));
    let source = r#"package representation
import representation.IntegerContract
import representation.LongContract
import representation.DoubleContract
class IntegerValue : IntegerContract
class LongValue : LongContract
class DoubleValue : DoubleContract
"#;
    let source_file = root.join("Values.kt");
    std::fs::write(&source_file, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    run(Command::new(&kotlin)
        .arg(&source_file)
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    for output in
        compile_outputs_with_classpath(source, "Values.kt", &[dependency.clone(), standard.clone()])
            .unwrap()
    {
        let path = candidate.join(&output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    let driver_source = root.join("RepresentationCheck.java");
    std::fs::write(
        &driver_source,
        r#"public final class RepresentationCheck {
    public static void main(String[] args) {
        representation.IntegerValue integer = new representation.IntegerValue();
        representation.LongValue wide = new representation.LongValue();
        representation.DoubleValue floating = new representation.DoubleValue();
        if (integer.value() != 42) throw new AssertionError("integer");
        if (wide.value() != 9000000001L) throw new AssertionError("long");
        if (floating.value() != 3.5) throw new AssertionError("double");
        if (floating.sum(9000000001L, 3.5, -2) != 9000000002.5) throw new AssertionError("wide parameters");
        try {
            if (integer.getClass().getMethod("value").getReturnType() != int.class) throw new AssertionError("integer descriptor");
            if (wide.getClass().getMethod("value").getReturnType() != long.class) throw new AssertionError("long descriptor");
            if (floating.getClass().getMethod("sum", long.class, double.class, int.class).getReturnType() != double.class) throw new AssertionError("wide descriptor");
        } catch (ReflectiveOperationException exception) { throw new AssertionError(exception); }
        System.out.println("physical-default-bridges-ok");
    }
}"#,
    )
    .unwrap();
    let driver = root.join("driver");
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&driver_source));
    for provider in [&official, &candidate] {
        let output = run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, provider, &dependency, &standard]))
            .arg("RepresentationCheck"));
        assert_eq!(output.stdout, b"physical-default-bridges-ok\n");
    }
    std::fs::remove_dir_all(root).unwrap();
}
