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
fn protected_helpers_follow_deferred_calls_and_do_not_change_unused_enum_abi() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    assert!(kotlin.is_file() && standard.is_file());
    let root = std::env::temp_dir().join(format!(
        "korrust-protected-bridges-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let dependency = root.join("dependency");
    let java = root.join("Base.java");
    std::fs::write(
        &java,
        "package interop.protection; public class Base { public static int calls; protected int advance(int value) { calls++; return value + 9; } protected int unused(int value) { return value + 99; } }",
    ).unwrap();
    run(Command::new("javac").arg("-d").arg(&dependency).arg(&java));
    let source = r#"package interop.usage
import interop.protection.Base
class Direct : Base() { fun value(): Int = advance(4) }
class Deferred : Base() {
    fun task(): () -> Int = { advance(8) }
    fun conditional(flag: Boolean): () -> Int = { if (flag) advance(12) else advance(13) }
}
class Defaulted : Base() {
    protected fun helper(value: Int = 5): Int = advance(value)
    fun task(): () -> Int = { helper() }
    fun explicitTask(): () -> Int = { helper(6) }
}
class Inlined : Base() {
    fun value(): Int { run<Int> { advance(4); return 31 }; return 0 }
}
enum class Marker { FIRST, SECOND }
"#;
    let source_file = root.join("Protected.kt");
    std::fs::write(&source_file, source).unwrap();
    let official = root.join("official");
    let candidate = root.join("candidate");
    let driver = root.join("driver");
    run(Command::new(&kotlin)
        .arg(&source_file)
        .arg("-cp")
        .arg(classpath(&[&dependency, &standard]))
        .arg("-d")
        .arg(&official));
    for output in compile_outputs_with_classpath(
        source,
        "Protected.kt",
        &[dependency.clone(), standard.clone()],
    )
    .unwrap()
    {
        let path = candidate.join(&output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    for owner in [
        "interop.usage.Direct",
        "interop.usage.Inlined",
        "interop.usage.Marker",
    ] {
        let signatures = [&official, &candidate].map(|output| {
            run(Command::new("javap")
                .arg("-p")
                .arg("-s")
                .arg("-classpath")
                .arg(classpath(&[output, &dependency, &standard]))
                .arg(owner))
            .stdout
        });
        assert_eq!(signatures[0], signatures[1], "{owner}");
    }
    let check = root.join("ProtectedCheck.java");
    std::fs::write(&check, r#"public final class ProtectedCheck {
    public static void main(String[] args) {
        if (new interop.usage.Direct().value() != 13) throw new AssertionError("direct protected call");
        interop.usage.Deferred value = new interop.usage.Deferred();
        if (value.task().invoke() != 17) throw new AssertionError("deferred protected call");
        if (value.conditional(true).invoke() != 21 || value.conditional(false).invoke() != 22) throw new AssertionError("conditional protected call");
        interop.usage.Defaulted defaults = new interop.usage.Defaulted();
        if (defaults.task().invoke() != 14 || defaults.explicitTask().invoke() != 15) throw new AssertionError("protected source/default call");
        int beforeInline = interop.protection.Base.calls;
        if (new interop.usage.Inlined().value() != 31 || interop.protection.Base.calls != beforeInline + 1) throw new AssertionError("inlined protected call");
        if (interop.usage.Marker.values().length != 2 || interop.usage.Marker.SECOND.ordinal() != 1) throw new AssertionError("enum contract");
        if (args.length != 0) {
            int bridges = 0;
            for (java.lang.reflect.Method method : interop.usage.Deferred.class.getDeclaredMethods()) {
                if (method.getName().startsWith("access$protected$")) {
                    if (!method.getName().equals("access$protected$advance") || !method.isSynthetic()) throw new AssertionError(method);
                    bridges++;
                }
            }
            if (bridges != 1) throw new AssertionError("required protected bridges: " + bridges);
        }
        System.out.println("protected-bridges-ok");
    }
}"#).unwrap();
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &dependency, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&check));
    for (output, candidate_check) in [(&official, false), (&candidate, true)] {
        let mut java = Command::new("java");
        java.arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, output, &dependency, &standard]))
            .arg("ProtectedCheck");
        if candidate_check {
            java.arg("candidate");
        }
        let output = run(&mut java);
        assert_eq!(output.stdout, b"protected-bridges-ok\n");
    }
    std::fs::remove_dir_all(root).unwrap();
}
