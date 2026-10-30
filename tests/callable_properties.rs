use korrust::compile_all_with_classpath;
use std::path::PathBuf;

#[test]
fn top_level_function_properties_are_invoked_through_their_value() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let standard = root.join("references/kotlinc/lib/kotlin-stdlib.jar");
    for source in [
        "typealias S = String\ntypealias SF<T> = (T) -> S\nval f: SF<S> = { it }\nfun box(): S = f(\"OK\")",
        "var f: (Int) -> Int = { it + 1 }\nfun box(): Int = f(41)",
    ] {
        let result = compile_all_with_classpath(
            source,
            "CallableProperties.kt",
            std::slice::from_ref(&standard),
        );
        assert!(result.is_ok(), "{source}: {:?}", result.err());
    }
}
