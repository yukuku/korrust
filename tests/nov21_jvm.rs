use korrust::{SourceInput, compile_batch_with_classpath, compile_outputs_with_classpath};
use std::path::PathBuf;

fn runtime() -> Vec<PathBuf> {
    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")]
}

#[test]
fn canonical_jvm_annotations_cover_methods_constructors_interfaces_and_aliases() {
    let source = r#"
import kotlin.jvm.Throws as Declares
import kotlin.jvm.Synchronized as Monitor
import java.io.IOException as Failure
@Declares(Failure::class, Throwable::class) fun top(): Int = 1
@Throws(exceptionClasses = [Failure::class]) fun named(): Int = 2
@Throws(*[Failure::class]) fun spread(): Int = 3
@Monitor fun locked(): Int = 4
class C @JvmOverloads @Throws(Failure::class) constructor(val n: Int = 5) {
    @Monitor @Throws(Failure::class) fun member(): Int = n
    @get:Monitor @get:Throws(Failure::class) val value: Int get() = n
}
interface I { @Throws(Throwable::class) fun run(); @Throws(Failure::class) fun body(): Int = 6 }
object O { @JvmStatic @JvmOverloads @Throws(Failure::class) fun value(n: Int = 7): Int = n }
"#;
    let output = compile_outputs_with_classpath(source, "Declarations.kt", &runtime()).unwrap();
    for owner in ["DeclarationsKt", "C", "I", "O"] {
        let class = output
            .iter()
            .find(|class| class.relative_path == format!("{owner}.class"))
            .unwrap();
        assert!(class.bytes.windows(10).any(|bytes| bytes == b"Exceptions"));
        assert!(
            !class
                .bytes
                .windows(b"Lkotlin/jvm/Throws;".len())
                .any(|bytes| bytes == b"Lkotlin/jvm/Throws;")
        );
        assert!(
            !class
                .bytes
                .windows(b"Lkotlin/jvm/Synchronized;".len())
                .any(|bytes| bytes == b"Lkotlin/jvm/Synchronized;")
        );
    }
}

#[test]
fn custom_same_named_annotations_keep_ordinary_identity() {
    let output = compile_batch_with_classpath(
        &[SourceInput {
            identity: "Shadow.kt".into(),
            file_name: "Shadow.kt".into(),
            source: include_str!("nov21_jvm/Shadow.kt").into(),
        }],
        &runtime(),
        "shadows",
    )
    .unwrap();
    assert!(
        output
            .iter()
            .any(|output| output.output.relative_path == "novjvm/shadow/ShadowKt.class")
    );
}

#[test]
fn invalid_and_unsupported_jvm_annotation_shapes_are_explicit() {
    for (source, code) in [
        (
            "@Throws(String::class) fun bad() {}",
            "JVM_ANNOTATION_PLACEMENT_INVALID",
        ),
        (
            "@Throws(Int::class) fun bad() {}",
            "JVM_ANNOTATION_PLACEMENT_INVALID",
        ),
        (
            "interface I { @Synchronized fun bad() {} }",
            "JVM_ANNOTATION_PLACEMENT_INVALID",
        ),
        (
            "abstract class C { @Synchronized abstract fun bad() }",
            "SYNCHRONIZED_ABSTRACT_INVALID",
        ),
        (
            "class C @Synchronized constructor()",
            "JVM_ANNOTATION_PLACEMENT_INVALID",
        ),
        (
            "@Synchronized fun bad(n: Int = 1): Int = n",
            "JVM_ANNOTATION_DEFAULTS_UNSUPPORTED",
        ),
        (
            "@Synchronized inline fun bad(): Int = 1",
            "SYNCHRONIZED_FUNCTION_SHAPE_UNSUPPORTED",
        ),
        (
            "@Throws(Exception::class) @Throws(Exception::class) fun bad() {}",
            "REPEATED_ANNOTATION_INVALID",
        ),
        (
            "interface Contract { @Throws(Exception::class) fun ready(): Int = 1 }; class Bad: Contract",
            "INHERITED_THROWS_DEFAULT_UNSUPPORTED",
        ),
        (
            "object Holder { private val Exception = ThreadLocal.withInitial { ByteArray(1) }; @Throws(Exception::class) fun bad() {} }",
            "INVALID_ANNOTATION_VALUE",
        ),
    ] {
        let error = compile_outputs_with_classpath(source, "Bad.kt", &runtime())
            .err()
            .unwrap();
        assert_eq!(error.code, code, "{source}: {error:?}");
    }
}
