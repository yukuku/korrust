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

fn annotation_owner(
    name: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> String {
    if let Some(owner) = imports.get(name) {
        return owner.clone();
    }
    if name.contains('/') || classpath.load(name).is_ok() {
        return name.into();
    }
    for package in ["kotlin", "kotlin/annotation", "kotlin/jvm"] {
        let owner = format!("{package}/{name}");
        if classpath.load(&owner).is_ok() {
            return owner;
        }
    }
    name.into()
}

pub(super) fn declaration_annotations(
    annotations: &[syntax::AnnotationUse],
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    target: &str,
    use_site: Option<&str>,
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
        })?;
        if !definition.targets.iter().any(|allowed| allowed == target) {
            return Err(Diagnostic::new(
                annotation.offset,
                format!("Annotation {} cannot target {target}", annotation.name),
            ));
        }
        if !seen.insert(owner.clone()) {
            return Err(Diagnostic::new(
                annotation.offset,
                "Repeated annotations require a repeatable declaration",
            ));
        }
        let mut supplied = HashSet::new();
        let mut values = Vec::new();
        for (position, (name, expression)) in annotation.arguments.iter().enumerate() {
            let name = match name {
                Some(name) => name.as_str(),
                None if position == 0 => "value",
                None => {
                    return Err(Diagnostic::new(
                        expression.offset,
                        "Java annotation arguments after value must be named",
                    ));
                }
            };
            let (_, descriptor, _) = definition
                .parameters
                .iter()
                .find(|(parameter, _, _)| parameter == name)
                .ok_or_else(|| {
                    Diagnostic::new(
                        expression.offset,
                        format!("Unknown annotation argument: {name}"),
                    )
                })?;
            if !supplied.insert(name.to_owned()) {
                return Err(Diagnostic::new(
                    expression.offset,
                    "Duplicate annotation argument",
                ));
            }
            let value = match (&expression.kind, descriptor.as_str()) {
                (ExprKind::Int(value), "I") => {
                    DeclarationAnnotationValue::Int(i32::try_from(*value).map_err(|_| {
                        Diagnostic::new(
                            expression.offset,
                            "Annotation integer argument is out of range",
                        )
                    })?)
                }
                (ExprKind::Unary(operator, value), "I") if operator == "-" => match value.kind {
                    ExprKind::Int(value) => {
                        DeclarationAnnotationValue::Int(i32::try_from(-value).map_err(|_| {
                            Diagnostic::new(
                                expression.offset,
                                "Annotation integer argument is out of range",
                            )
                        })?)
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            expression.offset,
                            "Unsupported annotation constant expression",
                        ));
                    }
                },
                (ExprKind::Boolean(value), "Z") => DeclarationAnnotationValue::Boolean(*value),
                (ExprKind::String(value), "Ljava/lang/String;") => {
                    DeclarationAnnotationValue::String(String::from_utf16(value).map_err(|_| {
                        Diagnostic::new(
                            expression.offset,
                            "Unsupported unpaired UTF-16 annotation string",
                        )
                    })?)
                }
                (_, "I" | "Z" | "Ljava/lang/String;") => {
                    return Err(Diagnostic::new(
                        expression.offset,
                        format!("Annotation argument type mismatch for {name}"),
                    ));
                }
                _ => {
                    return Err(Diagnostic::new(
                        expression.offset,
                        "Unsupported annotation array, class, enum, or numeric argument type",
                    ));
                }
            };
            values.push((name.to_owned(), value));
        }
        if let Some((name, _, _)) = definition
            .parameters
            .iter()
            .find(|(name, _, default)| !default && !supplied.contains(name))
        {
            return Err(Diagnostic::new(
                annotation.offset,
                format!("Missing required annotation argument: {name}"),
            ));
        }
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
            )
            .err()
            .unwrap();
            assert_eq!(error.offset, 23);
            assert_eq!(error.kind, crate::DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.code, code);
        }
    }
}
