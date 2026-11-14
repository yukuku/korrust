use korrust::compile_all_with_classpath;
use std::path::PathBuf;

fn standard() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("references/kotlinc/lib/kotlin-stdlib.jar")
}

#[test]
fn custom_iteration_protocol_and_open_ranges_compile() {
    let standard = standard();
    assert!(standard.is_file(), "run tools/setup_reference.py");
    let source = "class Cursor(var n:Int) { operator fun hasNext():Boolean=n<3; operator fun next():Int { n+=1; return n } }; class Values { operator fun iterator():Cursor=Cursor(0) }; fun f():Int { var sum=0; loop@ for(n in Values()) { if(n==1) continue@loop; sum+=n; if(n==2) break@loop }; return sum }; fun range(a:Long,b:Long):LongRange=a..<b";
    let classes = compile_all_with_classpath(source, "Iteration.kt", &[standard]).unwrap();
    assert!(classes.iter().any(|class| class.internal_name == "Cursor"));
}

#[test]
fn protocol_rejects_missing_operator_and_nonboolean_condition() {
    for source in [
        "class C { fun iterator():C=this; operator fun hasNext():Boolean=false; operator fun next():Int=0 }; fun f() { for(n in C()) {} }",
        "class C { operator fun iterator():C=this; fun hasNext():Boolean=false; operator fun next():Int=0 }; fun f() { for(n in C()) {} }",
        "class C { operator fun iterator():C=this; operator fun hasNext():Boolean=false; fun next():Int=0 }; fun f() { for(n in C()) {} }",
        "class C { operator fun iterator():C=this; operator fun hasNext()=1; operator fun next():Int=0 }; fun f() { for(n in C()) {} }",
        "class C { operator fun iterator():C=this; operator fun hasNext():Boolean=false; operator fun next():Int=0 }; fun f(c:C?) { for(n in c) {} }",
        "class C { fun rangeUntil(other:C):Int=1 }; fun f():Int=C()..<C()",
    ] {
        assert!(
            korrust::compile_all(source, "Invalid.kt").is_err(),
            "{source}"
        );
    }
}

#[test]
fn unsupported_mixed_integral_open_range_rejects() {
    let error = korrust::compile("fun f(a:Int,b:Long):LongRange=a..<b", "Mixed.kt")
        .err()
        .unwrap();
    assert!(
        error.message.contains("type") || error.message.contains("Type"),
        "{}",
        error.message
    );
}

#[test]
fn labeled_protocol_loop_ignores_loops_in_container_evaluation() {
    let source = "class Cursor(var n:Int) { operator fun hasNext():Boolean=n<3; operator fun next():Int { val result=n; n+=1; return result } }; class Values { operator fun iterator():Cursor=Cursor(0) }; fun f(flag:Boolean):Int { var count=0; outer@ for(a in if(flag) { var n=0; while(n<2) { n+=1 }; Values() } else Values()) { count+=1; if(a<2) continue@outer }; return count }";
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || korrust::compile_all(source, "Initializer.kt").unwrap())
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn imported_operator_extension_is_not_rebound_to_a_package_class_member() {
    let library = "package imported\nclass Cursor(var n:Int) { operator fun hasNext():Boolean=n<3; operator fun next():Int { n+=1; return n } }; class ExtensionBox; class ZMember { operator fun iterator():Cursor=Cursor(0) }; operator fun ExtensionBox.iterator():Cursor=Cursor(0)";
    let root = std::env::temp_dir().join(format!(
        "korrust-iteration-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    for class in korrust::compile_all(library, "Library.kt").unwrap() {
        let path = root.join(format!("{}.class", class.internal_name));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, class.bytes).unwrap();
    }
    let result = compile_all_with_classpath(
        "import imported.ExtensionBox\nimport imported.iterator\nfun f():Int { var sum=0; for(n in ExtensionBox()) { sum+=n }; return sum }",
        "Consumer.kt",
        std::slice::from_ref(&root),
    );
    std::fs::remove_dir_all(root).unwrap();
    result.unwrap();
}
