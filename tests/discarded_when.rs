use korrust::compile_outputs_with_classpath;
#[test]
fn discarded_when_checks_effects_without_joining_branch_values() {
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let classpath = [std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("references/kotlinc/lib/kotlin-stdlib.jar")];
            compile_outputs_with_classpath(
                include_str!("discarded_when/Effects.kt"),
                "Effects.kt",
                &classpath,
            )
            .unwrap();
            for source in [
                r#"fun invalid(value: Int): String = when(value) { 0 -> 1; else -> "text" }"#,
                "fun invalid(value: Int) { when(value) { 0 -> unknown(); else -> Unit } }",
            ] {
                assert!(compile_outputs_with_classpath(source, "Invalid.kt", &classpath).is_err());
            }
        })
        .unwrap()
        .join()
        .unwrap();
}
