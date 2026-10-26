use korrust::compile_outputs_with_classpath;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(command: &mut Command) {
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

fn classpath(paths: &[&Path]) -> std::ffi::OsString {
    std::env::join_paths(paths).unwrap()
}

#[test]
fn lexical_members_shadow_explicit_generic_imports_without_changing_top_level_priority() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let kotlin = project.join("references/kotlinc/bin/kotlinc");
    let standard = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
    if !kotlin.is_file() || !standard.is_file() {
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "korrust-member-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let library_source = root.join("Library.kt");
    let library = root.join("library");
    std::fs::write(
        &library_source,
        "package interop.factories\nfun <T> constructSnapshot(): List<T> = emptyList<T>()\nfun <T> gather(value: T): List<T> = listOf(value)\n",
    )
    .unwrap();
    run(Command::new(&kotlin)
        .arg(&library_source)
        .arg("-d")
        .arg(&library));
    let source = r#"import interop.factories.constructSnapshot
import interop.factories.gather
import kotlin.collections.emptyList
class MemberShadow {
    fun constructSnapshot(): List<Int> = listOf(9)
    fun arbitrary(): List<Int> = constructSnapshot()
    fun emptyList(): List<Int> = listOf(7)
    fun standard(): List<Int> = emptyList()
    fun local(): List<Int> {
        fun constructSnapshot(): List<Int> = listOf(13)
        return constructSnapshot()
    }
}
open class Parent {
    fun constructSnapshot(): List<Int> = listOf(11)
}
class Child: Parent() {
    fun inherited(): List<Int> = constructSnapshot()
}
open class PrivateParent {
    private fun constructSnapshot(): List<Int> = listOf(23)
    private fun emptyList(): List<Int> = listOf(29)
}
class PrivateChild: PrivateParent() {
    fun arbitraryImported(): List<Int> = constructSnapshot()
    fun standardImported(): List<Int> = emptyList()
}
class InapplicableMember {
    fun constructSnapshot(required: Int): List<Int> = listOf(required)
    fun emptyList(required: Int): List<Int> = listOf(required)
    fun gather(value: String): List<String> = listOf(value)
    fun arbitraryArity(): List<Int> = constructSnapshot()
    fun standardArity(): List<Int> = emptyList()
    fun argumentType(): List<Int> = gather(17)
}
fun constructSnapshot(): List<Int> = listOf(99)
fun imported(): List<Int> = constructSnapshot()
"#;
    let source_path = root.join("MemberShadow.kt");
    let official = root.join("official");
    let candidate = root.join("candidate");
    let driver = root.join("driver");
    std::fs::write(&source_path, source).unwrap();
    run(Command::new(&kotlin)
        .arg(&source_path)
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg("-d")
        .arg(&official));
    let check = root.join("ShadowCheck.java");
    std::fs::write(&check, r#"public final class ShadowCheck {
    private static void value(java.util.List<Integer> actual, int expected) {
        if (actual.size() != 1 || actual.get(0) != expected) throw new AssertionError(actual);
    }
    public static void main(String[] args) {
        MemberShadow target = new MemberShadow();
        value(target.arbitrary(), 9);
        value(target.standard(), 7);
        value(target.local(), 13);
        value(new Child().inherited(), 11);
        PrivateChild privateChild = new PrivateChild();
        if (!privateChild.arbitraryImported().isEmpty()) throw new AssertionError("private member blocked generic import");
        if (!privateChild.standardImported().isEmpty()) throw new AssertionError("private member blocked standard import");
        InapplicableMember inapplicable = new InapplicableMember();
        if (!inapplicable.arbitraryArity().isEmpty()) throw new AssertionError("arity blocked generic import");
        if (!inapplicable.standardArity().isEmpty()) throw new AssertionError("arity blocked standard import");
        value(inapplicable.argumentType(), 17);
        if (!MemberShadowKt.imported().isEmpty()) throw new AssertionError("explicit import priority");
    }
}"#).unwrap();
    run(Command::new("javac")
        .arg("-cp")
        .arg(classpath(&[&official, &library, &standard]))
        .arg("-d")
        .arg(&driver)
        .arg(&check));
    for output in compile_outputs_with_classpath(
        source,
        "MemberShadow.kt",
        &[library.clone(), standard.clone()],
    )
    .unwrap()
    {
        let path = candidate.join(&output.relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, output.bytes).unwrap();
    }
    for output in [&official, &candidate] {
        run(Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath(&[&driver, output, &library, &standard]))
            .arg("ShadowCheck"));
    }
    // Return mismatch does not make an otherwise applicable member lose its
    // lexical priority to an imported function with a more convenient result.
    let invalid = "import interop.factories.constructSnapshot\nclass WrongResult { fun constructSnapshot(): String = \"member\"; fun result(): List<Int> = constructSnapshot() }\n";
    let invalid_path = root.join("WrongResult.kt");
    std::fs::write(&invalid_path, invalid).unwrap();
    let rejected = Command::new(&kotlin)
        .arg(&invalid_path)
        .arg("-cp")
        .arg(classpath(&[&library, &standard]))
        .arg("-d")
        .arg(root.join("official-invalid"))
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(
        String::from_utf8_lossy(&rejected.stderr)
            .to_lowercase()
            .contains("type mismatch")
    );
    assert!(
        compile_outputs_with_classpath(
            invalid,
            "WrongResult.kt",
            &[library.clone(), standard.clone()]
        )
        .is_err()
    );
    std::fs::remove_dir_all(root).unwrap();
}
