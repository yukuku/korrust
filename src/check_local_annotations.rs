use super::*;

fn literal(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Int(_)
        | ExprKind::Long(_)
        | ExprKind::Float(_)
        | ExprKind::Double(_)
        | ExprKind::Char(_)
        | ExprKind::Boolean(_)
        | ExprKind::String(_) => true,
        ExprKind::Unary(_, value) | ExprKind::AnnotationConstant { value, .. } => literal(value),
        ExprKind::Binary(_, left, right) => literal(left) && literal(right),
        ExprKind::AnnotationArray(values) | ExprKind::Template(values) => {
            values.iter().all(literal)
        }
        _ => false,
    }
}

fn lexical_classifier(
    classpath: &Classpath,
    scope: &str,
    name: &str,
    offset: usize,
) -> Result<Option<String>> {
    let mut level = vec![scope.to_owned()];
    let mut seen = HashSet::new();
    while !level.is_empty() && seen.len() < 128 {
        let mut matches = std::collections::BTreeSet::new();
        let mut next = Vec::new();
        for owner in level {
            if !seen.insert(owner.clone()) {
                continue;
            }
            if let Some((class, _)) = classpath.java_member_class(&owner, name)? {
                matches.insert(class.name);
            }
            if let Ok(class) = classpath.load(&owner) {
                next.extend(class.super_name);
                next.extend(class.interfaces);
            }
        }
        if matches.len() > 1 {
            return Err(
                Diagnostic::new(offset, "Ambiguous local annotation classifier").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "LOCAL_ANNOTATION_CLASSIFIER_AMBIGUOUS",
                ),
            );
        }
        if let Some(owner) = matches.into_iter().next() {
            return Ok(Some(owner));
        }
        level = next;
    }
    Ok(None)
}

pub(crate) fn validate(source: &syntax::Module, classpath: &Classpath, scope: &str) -> Result<()> {
    if source.local_annotations.is_empty() {
        return Ok(());
    }
    let mut imports = crate::annotation_declarations::imports(source, classpath)?;
    for (annotations, target) in &source.local_annotations {
        for annotation in annotations {
            if !annotation.name.contains('/') {
                if let Some(owner) =
                    lexical_classifier(classpath, scope, &annotation.name, annotation.offset)?
                {
                    imports.insert(annotation.name.clone(), owner);
                } else if let Some(crate::classpath::ValueSymbol::Classifier(class)) =
                    classpath.value_symbol(scope, &annotation.name, annotation.offset)?
                {
                    imports.insert(annotation.name.clone(), class.name);
                }
            }
        }
        for annotation in annotations {
            if annotation
                .arguments
                .iter()
                .any(|(_, value)| !literal(value))
            {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Local annotation arguments currently require literal constant expressions",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "LOCAL_ANNOTATION_ARGUMENT_CONTEXT_UNSUPPORTED",
                ));
            }
        }
        let emitted = annotations::declaration_annotations(
            annotations,
            classpath,
            &imports,
            target,
            None,
            Some(scope),
        )?;
        if !emitted.is_empty() {
            return Err(Diagnostic::new(
                annotations[0].offset,
                "Nonsource local and expression annotation retention is unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "LOCAL_ANNOTATION_RETENTION_UNSUPPORTED",
            ));
        }
        for annotation in annotations {
            if annotations::annotation_owner(&annotation.name, classpath, &imports)
                != "kotlin/Suppress"
            {
                continue;
            }
            let checked = crate::annotation_declarations::checked_use(
                annotation,
                classpath,
                &imports,
                Some(scope),
            )?;
            for (_, value) in &checked.values {
                let DeclarationAnnotationValue::Array(values) = value else {
                    return Err(Diagnostic::new(
                        annotation.offset,
                        "Suppress requires String names",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "SUPPRESS_ARGUMENT_INVALID",
                    ));
                };
                for value in values {
                    let DeclarationAnnotationValue::String(units) = value else {
                        return Err(Diagnostic::new(
                            annotation.offset,
                            "Suppress requires String names",
                        )
                        .with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "SUPPRESS_ARGUMENT_INVALID",
                        ));
                    };
                    let name = String::from_utf16_lossy(units);
                    if !matches!(name.as_str(), "UNCHECKED_CAST" | "RestrictedApi") {
                        return Err(Diagnostic::new(
                            annotation.offset,
                            format!("Local suppression {name} may require unsupported diagnostic semantics"),
                        )
                        .with_kind(
                            crate::DiagnosticKind::UnsupportedLanguage,
                            "SUPPRESSION_DIAGNOSTIC_UNSUPPORTED",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
