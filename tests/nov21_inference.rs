use korrust::compile_outputs_with_classpath;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn expected_results_and_metadata_contracts_preserve_stability_and_return_polarity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures = root.join("tests/nov21_inference");
    let temporary =
        std::env::temp_dir().join(format!("korrust-nov21-inference-{}", std::process::id()));
    let dependency = temporary.join("dependency");
    std::fs::create_dir_all(&temporary).unwrap();
    let output = Command::new(root.join("references/kotlinc/bin/kotlinc"))
        .arg(fixtures.join("Dependency.kt"))
        .args(["-jvm-target", "1.8", "-d"])
        .arg(&dependency)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let cp = [
        dependency,
        root.join("references/kotlinc/lib/kotlin-stdlib.jar"),
    ];
    let source = std::fs::read_to_string(fixtures.join("Models.kt")).unwrap();
    compile_outputs_with_classpath(&source, "Models.kt", &cp).unwrap();
    for name in [
        "InvalidNoContract",
        "InvalidOpposite",
        "InvalidOverload",
        "InvalidNullComplement",
        "InvalidDisjunction",
        "InvalidArray",
        "InvalidExplicitArray",
        "InvalidExplicitBound",
        "InvalidMutated",
        "InvalidCapturedMutated",
        "InvalidChain",
        "InvalidAnyNot",
        "InvalidAnyEqualsFalse",
        "InvalidAnyEqualsFalseReverse",
        "InvalidAnyEqualsTrue",
        "InvalidAnyEqualsTrueReverse",
        "InvalidNullableNot",
        "InvalidNullableComplementReverse",
        "InvalidNullableFalseComplementReverse",
        "InvalidShortCircuitOr",
        "InvalidShortCircuitAnd",
        "InvalidNullableFunction",
        "InvalidNullableReference",
        "InvalidNullableComplement",
        "InvalidNullableFalseComplement",
    ] {
        let source = std::fs::read_to_string(fixtures.join(format!("{name}.kt"))).unwrap();
        assert!(
            compile_outputs_with_classpath(&source, &format!("{name}.kt"), &cp).is_err(),
            "{name}"
        );
    }
    for name in [
        "UnsupportedMutable",
        "UnsupportedCaptured",
        "UnsupportedTrailing",
        "UnsupportedReifiedUnit",
        "UnsupportedReifiedNullableUnit",
        "NullReturn",
    ] {
        let source = std::fs::read_to_string(fixtures.join(format!("{name}.kt"))).unwrap();
        assert!(compile_outputs_with_classpath(&source, &format!("{name}.kt"), &cp).is_err());
    }
    std::fs::remove_dir_all(temporary).unwrap();
}
