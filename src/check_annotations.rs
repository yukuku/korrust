use super::*;
use crate::classpath::AnnotationRetention;

pub(crate) fn unsupported_lowering(owner: &str) -> Option<&'static str> {
    match owner {
        "kotlinx/serialization/Serializable"
        | "androidx/compose/runtime/Composable"
        | "kotlinx/parcelize/Parcelize" => Some("COMPILER_PLUGIN_UNSUPPORTED"),
        "kotlin/jvm/JvmName"
        | "kotlin/jvm/JvmSynthetic"
        | "kotlin/jvm/JvmDefault"
        | "kotlin/jvm/JvmDefaultWithCompatibility"
        | "kotlin/jvm/JvmDefaultWithoutCompatibility"
        | "kotlin/jvm/JvmMultifileClass"
        | "kotlin/jvm/JvmPackageName"
        | "kotlin/jvm/JvmSuppressWildcards"
        | "kotlin/jvm/JvmWildcard"
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
    let owner = crate::annotation_declarations::owner(name, imports, classpath);
    let owner = if matches!(owner.as_str(), "Throws" | "Synchronized")
        && !imports.contains_key(name)
        && classpath.load(&owner).is_err()
    {
        format!("kotlin/jvm/{owner}")
    } else {
        owner
    };
    if classpath.load(&owner).is_ok() {
        return owner;
    }
    match classpath.expand_type_alias(
        &Type::Java(owner.clone()),
        imports,
        &["kotlin", "kotlin/jvm"],
    ) {
        Ok(Type::Java(expanded)) => expanded,
        _ => owner,
    }
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
        if owner == "kotlin/jvm/JvmName" && target == "FILE" && use_site == Some("file") {
            if !seen.insert(owner.clone()) {
                return Err(
                    Diagnostic::new(annotation.offset, "Repeated JvmName annotation").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "REPEATED_ANNOTATION_INVALID",
                    ),
                );
            }
            let name = super::annotation_placements::validated_jvm_name(annotation)?;
            result.push(DeclarationAnnotation {
                descriptor: "Lkotlin/jvm/JvmName;".into(),
                runtime: false,
                values: vec![(
                    "name".into(),
                    DeclarationAnnotationValue::String(name.encode_utf16().collect()),
                )],
            });
            continue;
        }
        if matches!(
            owner.as_str(),
            "kotlin/jvm/Throws" | "kotlin/jvm/Synchronized"
        ) {
            if !seen.insert(owner.clone()) {
                return Err(
                    Diagnostic::new(annotation.offset, "Repeated JVM method annotation").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "REPEATED_ANNOTATION_INVALID",
                    ),
                );
            }
            result.push(jvm_method_annotation(
                annotation, &owner, classpath, imports, target, use_site, scope,
            )?);
            continue;
        }
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
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ANNOTATION_USE_SITE_INVALID",
            ));
        }
        if matches!(
            owner.as_str(),
            "kotlin/jvm/JvmOverloads" | "kotlin/jvm/JvmStatic"
        ) {
            if (target != "METHOD"
                && !(target == "CONSTRUCTOR" && owner == "kotlin/jvm/JvmOverloads"))
                || use_site.is_some()
                || !annotation.arguments.is_empty()
            {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "JVM method annotation requires a function without annotation arguments",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "JVM_ANNOTATION_PLACEMENT_INVALID",
                ));
            }
            if !seen.insert(owner.clone()) {
                return Err(
                    Diagnostic::new(annotation.offset, "Repeated JVM method annotation").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "REPEATED_ANNOTATION_INVALID",
                    ),
                );
            }
            result.push(DeclarationAnnotation {
                descriptor: format!("L{owner};"),
                runtime: owner == "kotlin/jvm/JvmStatic",
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

fn jvm_method_annotation(
    annotation: &syntax::AnnotationUse,
    owner: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    target: &str,
    use_site: Option<&str>,
    scope: Option<&str>,
) -> Result<DeclarationAnnotation> {
    let invalid = |message| {
        Diagnostic::new(annotation.offset, message).with_kind(
            crate::DiagnosticKind::InvalidKotlin,
            "JVM_ANNOTATION_PLACEMENT_INVALID",
        )
    };
    if annotation.use_site.as_deref() != use_site
        || !matches!(
            (target, use_site),
            ("METHOD", None | Some("get" | "set")) | ("CONSTRUCTOR", None)
        )
        || target == "CONSTRUCTOR" && owner == "kotlin/jvm/Synchronized"
    {
        return Err(invalid(
            "JVM annotation requires a function, accessor, or supported constructor",
        ));
    }
    let mut values = Vec::new();
    if owner == "kotlin/jvm/Synchronized" {
        if !annotation.arguments.is_empty() {
            return Err(invalid("Synchronized does not accept annotation arguments"));
        }
        if let Some(scope) = scope {
            let class = classpath.load(scope)?;
            if class.access & 0x0200 != 0 {
                return Err(invalid("Synchronized is not allowed on interface members"));
            }
            if classpath.value_classes().contains_key(scope) {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Synchronized is not allowed on value-class members",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "SYNCHRONIZED_VALUE_CLASS_INVALID",
                ));
            }
        }
    } else {
        let mut classes = Vec::new();
        let mut named = false;
        for (name, expression) in &annotation.arguments {
            let array = if let Some(name) = name {
                if name != "exceptionClasses" || named || !classes.is_empty() {
                    return Err(invalid(
                        "Throws requires exceptionClasses or positional class literals",
                    ));
                }
                named = true;
                Some(expression)
            } else {
                if named {
                    return Err(invalid(
                        "Positional Throws arguments cannot follow a named argument",
                    ));
                }
                match &expression.kind {
                    ExprKind::Unary(operator, value) if operator == "*" => Some(value.as_ref()),
                    _ => None,
                }
            };
            if let Some(array) = array {
                let DeclarationAnnotationValue::Array(items) =
                    crate::annotation_declarations::value(
                        array,
                        "[Ljava/lang/Class;",
                        imports,
                        classpath,
                        0,
                        scope,
                    )?
                else {
                    return Err(invalid("Throws requires an array of class literals"));
                };
                classes.extend(items);
            } else {
                classes.push(crate::annotation_declarations::value(
                    expression,
                    "Ljava/lang/Class;",
                    imports,
                    classpath,
                    0,
                    scope,
                )?);
            }
        }
        for value in &classes {
            let DeclarationAnnotationValue::Class { descriptor, .. } = value else {
                return Err(invalid("Throws requires Throwable class literals"));
            };
            let Some(mut current) = descriptor
                .strip_prefix('L')
                .and_then(|name| name.strip_suffix(';'))
                .map(str::to_owned)
            else {
                return Err(invalid("Throws requires Throwable class literals"));
            };
            let mut visited = HashSet::new();
            loop {
                if current == "java/lang/Throwable" {
                    break;
                }
                if !visited.insert(current.clone()) {
                    return Err(invalid("Throws requires a Throwable subtype"));
                }
                let class = classpath.load(&current)?;
                let Some(parent) = class.super_name else {
                    return Err(invalid("Throws requires a Throwable subtype"));
                };
                current = parent;
            }
        }
        values.push((
            "exceptionClasses".into(),
            DeclarationAnnotationValue::Array(classes),
        ));
    }
    Ok(DeclarationAnnotation {
        descriptor: format!("L{owner};"),
        runtime: false,
        values,
    })
}

pub(super) fn exception_classes(annotations: &[DeclarationAnnotation]) -> Vec<String> {
    annotations
        .iter()
        .filter(|annotation| annotation.descriptor == "Lkotlin/jvm/Throws;")
        .flat_map(|annotation| &annotation.values)
        .flat_map(|(_, value)| match value {
            DeclarationAnnotationValue::Array(values) => values.as_slice(),
            _ => &[],
        })
        .filter_map(|value| match value {
            DeclarationAnnotationValue::Class { descriptor, .. } => descriptor
                .strip_prefix('L')
                .and_then(|name| name.strip_suffix(';'))
                .map(str::to_owned),
            _ => None,
        })
        .collect()
}

pub(super) fn validate_jvm_function(
    function: &syntax::Function,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<()> {
    for annotation in &function.annotations {
        let owner = annotation_owner(&annotation.name, classpath, imports);
        if owner == "kotlin/jvm/Throws" && function.suspend_ {
            return Err(Diagnostic::new(
                annotation.offset,
                "Throws suspend functions are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "THROWS_SUSPEND_UNSUPPORTED",
            ));
        }
        if owner == "kotlin/jvm/Synchronized" && function.defaults.iter().any(Option::is_some) {
            return Err(Diagnostic::new(
                annotation.offset,
                "Synchronized functions with default arguments are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "JVM_ANNOTATION_DEFAULTS_UNSUPPORTED",
            ));
        }
        if owner == "kotlin/jvm/Synchronized" && function.suspend_ {
            return Err(Diagnostic::new(
                annotation.offset,
                "Synchronized is not allowed on suspend functions",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "SYNCHRONIZED_SUSPEND_INVALID",
            ));
        }
        if owner == "kotlin/jvm/Synchronized" && function.inline {
            return Err(Diagnostic::new(
                annotation.offset,
                "Synchronized inline functions are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "SYNCHRONIZED_FUNCTION_SHAPE_UNSUPPORTED",
            ));
        }
    }
    Ok(())
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
    fn jvm_method_markers_preserve_distinct_retention() {
        let classpath = Classpath::new(&[]).unwrap();
        for (owner, runtime) in [
            ("kotlin/jvm/JvmStatic", true),
            ("kotlin/jvm/JvmOverloads", false),
        ] {
            let annotations = declaration_annotations(
                &[syntax::AnnotationUse {
                    name: owner.into(),
                    offset: 5,
                    use_site: None,
                    arguments: Vec::new(),
                }],
                &classpath,
                &HashMap::new(),
                "METHOD",
                None,
                None,
            )
            .unwrap();
            assert_eq!(annotations.len(), 1);
            assert_eq!(annotations[0].descriptor, format!("L{owner};"));
            assert_eq!(annotations[0].runtime, runtime);
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
