use super::*;
use crate::classpath::AnnotationRetention;

pub(crate) fn unsupported_lowering(owner: &str) -> Option<&'static str> {
    match owner {
        "kotlinx/serialization/Serializable" | "androidx/compose/runtime/Composable" => {
            Some("COMPILER_PLUGIN_UNSUPPORTED")
        }
        "kotlin/jvm/JvmName"
        | "kotlin/jvm/JvmSynthetic"
        | "kotlin/jvm/Synchronized"
        | "kotlin/jvm/JvmDefault"
        | "kotlin/jvm/JvmDefaultWithCompatibility"
        | "kotlin/jvm/JvmDefaultWithoutCompatibility"
        | "kotlin/jvm/JvmMultifileClass"
        | "kotlin/jvm/JvmPackageName"
        | "kotlin/jvm/JvmSuppressWildcards"
        | "kotlin/jvm/JvmWildcard"
        | "kotlin/jvm/JvmInline"
        | "kotlin/jvm/JvmRecord"
        | "kotlin/jvm/JvmExposeBoxed"
        | "kotlin/jvm/Transient"
        | "kotlin/jvm/Strictfp"
        | "kotlin/jvm/JvmSerializableLambda" => Some("ANNOTATION_JVM_LOWERING_UNSUPPORTED"),
        "kotlin/PublishedApi"
        | "kotlin/UnsafeVariance"
        | "kotlin/BuilderInference"
        | "kotlin/OverloadResolutionByLambdaReturnType"
        | "kotlin/ConsistentCopyVisibility"
        | "kotlin/ExposedCopyVisibility"
        | "kotlin/internal/InlineOnly" => Some("ANNOTATION_KOTLIN_SEMANTICS_UNSUPPORTED"),
        _ => None,
    }
}

pub(super) fn annotation_owner(
    name: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> String {
    crate::annotation_declarations::owner(name, imports, classpath)
}

pub(super) fn declaration_annotations(
    annotations: &[syntax::AnnotationUse],
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    target: &str,
    use_site: Option<&str>,
    scope: Option<&str>,
) -> Result<Vec<DeclarationAnnotation>> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    for annotation in annotations {
        let owner = annotation_owner(&annotation.name, classpath, imports);
        if let Some(code) = unsupported_lowering(&owner) {
            let semantics = if code == "COMPILER_PLUGIN_UNSUPPORTED" {
                "compiler plugin transformations"
            } else {
                "Kotlin compiler annotation semantics"
            };
            return Err(Diagnostic::new(
                annotation.offset,
                format!(
                    "Annotation {} requires unsupported {semantics}",
                    annotation.name
                ),
            )
            .with_kind(crate::DiagnosticKind::UnsupportedLanguage, code));
        }
        if annotation.use_site.as_deref() != use_site {
            return Err(Diagnostic::new(
                annotation.offset,
                "Annotation use-site target is not valid for this declaration",
            ));
        }
        if matches!(
            owner.as_str(),
            "kotlin/jvm/JvmOverloads" | "kotlin/jvm/JvmStatic"
        ) {
            if target != "METHOD" || use_site.is_some() || !annotation.arguments.is_empty() {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "JVM method annotation requires a function without annotation arguments",
                ));
            }
            if !seen.insert(owner.clone()) {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Repeated JVM method annotation",
                ));
            }
            result.push(DeclarationAnnotation {
                descriptor: format!("L{owner};"),
                runtime: false,
                values: Vec::new(),
            });
            continue;
        }
        let class = classpath.load(&owner).map_err(|mut error| {
            error.offset = annotation.offset;
            error
        })?;
        let definition = class.annotation_definition.ok_or_else(|| {
            Diagnostic::new(
                annotation.offset,
                format!("Unsupported annotation definition: {}", annotation.name),
            )
            .with_kind(
                if class.access & 0x2000 != 0 {
                    crate::DiagnosticKind::UnsupportedLanguage
                } else {
                    crate::DiagnosticKind::InvalidKotlin
                },
                if class.access & 0x2000 != 0 {
                    "ANNOTATION_DEFINITION_UNSUPPORTED"
                } else {
                    "NOT_AN_ANNOTATION"
                },
            )
        })?;
        let allowed = if let Some(targets) = &definition.kotlin_targets {
            let logical = match (target, use_site) {
                ("PROPERTY", _) => "PROPERTY",
                ("FIELD", _) => "FIELD",
                ("PARAMETER", _) => "VALUE_PARAMETER",
                ("TYPE", _) => "CLASS",
                ("METHOD", Some("get")) => "PROPERTY_GETTER",
                ("METHOD", Some("set")) => "PROPERTY_SETTER",
                ("METHOD", _) => "FUNCTION",
                ("CONSTRUCTOR", _) => "CONSTRUCTOR",
                _ => target,
            };
            targets.iter().any(|allowed| allowed == logical)
        } else {
            definition.targets.iter().any(|allowed| allowed == target)
        };
        if !allowed {
            return Err(Diagnostic::new(
                annotation.offset,
                format!("Annotation {} cannot target {target}", annotation.name),
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ANNOTATION_TARGET_INVALID",
            ));
        }
        if !seen.insert(owner.clone()) {
            return Err(Diagnostic::new(
                annotation.offset,
                "Repeated annotations require a repeatable declaration",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "REPEATED_ANNOTATION_INVALID",
            ));
        }
        let checked =
            crate::annotation_declarations::checked_use(annotation, classpath, imports, scope)?;
        let values = checked.values;

        if definition.retention != AnnotationRetention::Source {
            result.push(DeclarationAnnotation {
                descriptor: format!("L{owner};"),
                runtime: definition.retention == AnnotationRetention::Runtime,
                values,
            });
        }
    }
    Ok(result)
}

pub(super) fn overload_descriptors(function: &Function) -> Vec<String> {
    if !function
        .annotations
        .iter()
        .any(|annotation| annotation.descriptor == "Lkotlin/jvm/JvmOverloads;")
    {
        return Vec::new();
    }
    let mut omitted = HashSet::new();
    let mut result = Vec::new();
    for index in (0..function.params.len()).rev() {
        if function.defaults.get(index).is_some_and(Option::is_some) {
            omitted.insert(index);
            result.push(format!(
                "({}){}",
                function
                    .params
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !omitted.contains(index))
                    .map(|(_, (_, ty))| ty.descriptor())
                    .collect::<String>(),
                function.result.descriptor()
            ));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_annotation_guards_use_canonical_identity() {
        for owner in [
            "kotlin/jvm/JvmName",
            "kotlin/jvm/JvmSynthetic",
            "kotlin/jvm/Synchronized",
            "kotlin/jvm/JvmDefaultWithCompatibility",
            "kotlin/jvm/Transient",
            "kotlin/jvm/Strictfp",
        ] {
            assert_eq!(
                unsupported_lowering(owner),
                Some("ANNOTATION_JVM_LOWERING_UNSUPPORTED")
            );
        }
        for name in [
            "JvmName",
            "custom/JvmName",
            "custom/Composable",
            "custom/Serializable",
            "kotlin/jvm/JvmStatic",
            "kotlin/jvm/JvmField",
            "kotlin/jvm/Volatile",
            "kotlin/jvm/Throws",
            "kotlin/Suppress",
        ] {
            assert_eq!(unsupported_lowering(name), None, "{name}");
        }
    }

    #[test]
    fn compiler_semantics_are_rejected_before_loading_annotation_definitions() {
        let classpath = Classpath::new(&[]).unwrap();
        for (name, code) in [
            ("kotlin/jvm/JvmName", "ANNOTATION_JVM_LOWERING_UNSUPPORTED"),
            (
                "androidx/compose/runtime/Composable",
                "COMPILER_PLUGIN_UNSUPPORTED",
            ),
            (
                "kotlinx/serialization/Serializable",
                "COMPILER_PLUGIN_UNSUPPORTED",
            ),
        ] {
            let error = declaration_annotations(
                &[syntax::AnnotationUse {
                    name: "Alias".into(),
                    offset: 23,
                    use_site: None,
                    arguments: Vec::new(),
                }],
                &classpath,
                &HashMap::from([("Alias".into(), name.into())]),
                "METHOD",
                None,
                None,
            )
            .err()
            .unwrap();
            assert_eq!(error.offset, 23);
            assert_eq!(error.kind, crate::DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.code, code);
        }
    }
}
