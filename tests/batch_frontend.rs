use korrust::{SourceInput, compile_batch_with_classpath};
use std::collections::BTreeMap;

fn input(identity: &str, source: &str) -> SourceInput {
    SourceInput {
        identity: identity.into(),
        file_name: identity.rsplit('/').next().unwrap().into(),
        source: source.into(),
    }
}

fn outputs(
    inputs: &[SourceInput],
) -> Result<BTreeMap<(String, String), Vec<u8>>, korrust::Diagnostic> {
    Ok(compile_batch_with_classpath(inputs, &[], "batch_test")?
        .into_iter()
        .map(|output| {
            (
                (output.source_identity, output.output.relative_path),
                output.output.bytes,
            )
        })
        .collect())
}

#[test]
fn batch_discovers_forward_signatures_and_infers_across_files_independent_of_order() {
    let cases = [
        vec![
            input("Left.kt", "package sample\nclass Left(val right: Right?)"),
            input("Right.kt", "package sample\nclass Right(val left: Left?)"),
        ],
        vec![
            input("Use.kt", "package sample\nfun answer() = value() + 1"),
            input("Value.kt", "package sample\nfun value() = 41"),
        ],
        vec![
            input(
                "Use.kt",
                "package use\nimport values.answer\nfun use() = answer()",
            ),
            input("Value.kt", "package values\nfun answer() = 42"),
        ],
        vec![
            input(
                "Odd.kt",
                "fun odd(value: Int): Boolean = if(value == 0) false else even(value - 1)",
            ),
            input(
                "Even.kt",
                "fun even(value: Int): Boolean = if(value == 0) true else odd(value - 1)",
            ),
        ],
        vec![
            input("Use.kt", "package sample\nfun use(): Int = answer"),
            input("Value.kt", "package sample\nval answer = 42"),
        ],
    ];
    let mut cases = cases.to_vec();
    cases.push(vec![input("StringChoice.kt", "package sample\nfun choose(value: String): Int = value.length\nfun use(): Int = choose(4) + choose(\"abc\")"), input("IntChoice.kt", "package sample\nfun choose(value: Int): Int = value")]);
    cases.push(vec![input("Factory.kt", "package sample\nfun <T> fabricate(): List<T> = emptyList<T>()"), input("Snapshot.kt", "package sample\ndata class Snapshot(val logs: List<String>) { companion object { val EMPTY = Snapshot(logs = fabricate()) } }")]);
    cases.push(vec![
        input(
            "Hidden.kt",
            "package sample\nprivate fun hidden(): Int = 1\nfun useHidden(): Int = hidden()",
        ),
        input("Other.kt", "package sample\nfun other(): Int = useHidden()"),
    ]);
    cases.push(vec![
        input(
            "Internal.kt",
            "package sample\ninternal fun value(): Int = 42",
        ),
        input("Use.kt", "package sample\nfun use(): Int = value()"),
    ]);
    cases.push(vec![
        input("Const.kt", "package sample\nconst val NUMBER = 42"),
        input("UseConst.kt", "package sample\nfun use(): Int = NUMBER"),
    ]);
    cases.push(vec![
        input("Const.kt", "package values\nconst val NUMBER = 42"),
        input(
            "UseConst.kt",
            "package sample\nimport values.NUMBER\nfun use(): Int = NUMBER",
        ),
    ]);
    for mut sources in cases {
        let forward = outputs(&sources).unwrap_or_else(|error| {
            panic!(
                "{}: {}",
                error.source_identity.unwrap_or_default(),
                error.message
            )
        });
        sources.reverse();
        assert_eq!(forward, outputs(&sources).unwrap());
    }
}

#[test]
fn batch_scopes_names_by_package_and_reports_the_original_source() {
    let sources = [
        input("left/Value.kt", "package left\nclass Value"),
        input("right/Value.kt", "package right\nclass Value"),
    ];
    let compiled = outputs(&sources).unwrap();
    assert!(compiled.keys().any(|(_, path)| path == "left/Value.class"));
    assert!(compiled.keys().any(|(_, path)| path == "right/Value.class"));
    let error = outputs(&[
        input("Good.kt", "class Good"),
        input("Bad.kt", "fun fail(): Int = missing()"),
    ])
    .err()
    .unwrap();
    assert_eq!(error.source_identity.as_deref(), Some("Bad.kt"));
    assert!(error.offset > 0);
}

#[test]
fn batch_rejects_duplicate_owners_and_cyclic_inferred_signatures() {
    for sources in [
        vec![
            input("One.kt", "class Duplicate"),
            input("Two.kt", "class Duplicate"),
        ],
        vec![
            input("One.kt", "fun same(): Int = 1"),
            input("Two.kt", "fun same(): Int = 2"),
        ],
        vec![
            input("One.kt", "fun first() = second()"),
            input("Two.kt", "fun second() = first()"),
        ],
        vec![
            input("One.kt", "private fun hidden(): Int = 1"),
            input("Two.kt", "fun use(): Int = hidden()"),
        ],
    ] {
        assert!(outputs(&sources).is_err());
    }
    assert!(outputs(&[]).unwrap().is_empty());
}

#[test]
fn module_identity_is_stored_and_input_validation_keeps_provenance() {
    let sources = [input(
        "Public.kt",
        "package sample\nclass Public(val number: Int)\nfun value(): Int = 42",
    )];
    let first = compile_batch_with_classpath(&sources, &[], "first").unwrap();
    let second = compile_batch_with_classpath(&sources, &[], "second").unwrap();
    assert_ne!(
        first
            .iter()
            .map(|output| (&output.output.relative_path, &output.output.bytes))
            .collect::<Vec<_>>(),
        second
            .iter()
            .map(|output| (&output.output.relative_path, &output.output.bytes))
            .collect::<Vec<_>>()
    );
    assert!(
        first
            .iter()
            .any(|output| output.output.relative_path.starts_with("META-INF/korrust_"))
    );
    for source in [
        input("Invalid.txt", "class A"),
        input("NonAscii-é.kt", "class A"),
        input("TooLarge.kt", &" ".repeat(1_048_577)),
    ] {
        let identity = source.identity.clone();
        assert_eq!(
            outputs(&[source]).err().unwrap().source_identity.as_deref(),
            Some(identity.as_str())
        );
    }
}

#[test]
fn source_classifiers_and_generated_helpers_cannot_silently_replace_each_other() {
    for sources in [
        vec![input(
            "Aliases.kt",
            "typealias A = String\ntypealias A = Int",
        )],
        vec![input("Aliases.kt", "class A\ntypealias A = String")],
        vec![
            input("Left.kt", "package p\ntypealias A = String"),
            input("Right.kt", "package p\ntypealias A = Int"),
        ],
        vec![input(
            "Collision.kt",
            "interface Collision { class DefaultImpls; fun value(): Int = 1 }",
        )],
    ] {
        let error = outputs(&sources).unwrap_err();
        assert!(
            error.message.contains("Conflicting source classifier")
                || error.message.contains("Generated JVM class conflicts"),
            "{}",
            error.message
        );
    }
    let source = "fun run(): Int { val callback: () -> Int = { 3 }; return callback() }";
    let base = outputs(&[input("Collision.kt", source)]).unwrap();
    let helper = base
        .keys()
        .find_map(|(_, path)| {
            path.strip_suffix(".class")
                .filter(|owner| owner.contains("$lambda$"))
        })
        .unwrap();
    let collision = format!("{source}\nclass `{helper}`");
    let error = outputs(&[input("Collision.kt", &collision)]).unwrap_err();
    assert!(
        error.message.contains("Generated JVM class conflicts")
            || error.code == "CLASSIFIER_DOLLAR_NAME_UNSUPPORTED",
        "{}",
        error.message
    );
    outputs(&[
        input(
            "Left.kt",
            "package p\nprivate typealias A = String\nfun left(value: A): A = value",
        ),
        input(
            "Right.kt",
            "package p\nprivate typealias A = Int\nfun right(value: A): A = value",
        ),
    ])
    .unwrap();
}
