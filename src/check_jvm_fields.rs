use super::*;

pub(crate) fn has_jvm_field(
    uses: &[syntax::AnnotationUse],
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> bool {
    uses.iter().any(|annotation| {
        annotations::annotation_owner(&annotation.name, classpath, imports) == "kotlin/jvm/JvmField"
    })
}

fn marker<'a>(
    uses: &'a [syntax::AnnotationUse],
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<Option<&'a syntax::AnnotationUse>> {
    let Some(annotation) = uses.iter().find(|annotation| {
        annotations::annotation_owner(&annotation.name, classpath, imports) == "kotlin/jvm/JvmField"
    }) else {
        return Ok(None);
    };
    if annotation
        .use_site
        .as_deref()
        .is_some_and(|site| site != "field")
    {
        return Err(invalid(
            annotation.offset,
            "JvmField requires a property field target",
        ));
    }
    if uses.iter().any(|other| {
        matches!(
            annotations::annotation_owner(&other.name, classpath, imports).as_str(),
            "kotlin/jvm/Volatile" | "kotlin/concurrent/Volatile"
        )
    }) {
        return Err(Diagnostic::new(
            annotation.offset,
            "Volatile JvmField properties require unsupported field modifier lowering",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "JVM_FIELD_VOLATILE_UNSUPPORTED",
        ));
    }
    Ok(Some(annotation))
}

fn invalid(offset: usize, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message)
        .with_kind(crate::DiagnosticKind::InvalidKotlin, "JVM_FIELD_INVALID")
}

fn validate_property(
    property: &syntax::PropertyDecl,
    annotation: &syntax::AnnotationUse,
) -> Result<()> {
    let message = if property.receiver.is_some() {
        Some("JvmField cannot expose an extension property")
    } else if property.private {
        Some("JvmField cannot expose a private property")
    } else if property.open || property.is_override {
        Some("JvmField requires a final non-overriding property")
    } else if property.lateinit {
        Some("JvmField cannot be combined with lateinit")
    } else if property.delegate.is_some() {
        Some("JvmField cannot expose a delegated property")
    } else if !property.body.is_empty() || property.setter.is_some() {
        Some("JvmField cannot expose a property with custom accessors")
    } else if property.setter_visibility() != property.visibility() {
        Some("JvmField cannot expose a property with a restricted setter")
    } else {
        None
    };
    if let Some(message) = message {
        return Err(invalid(annotation.offset, message));
    }
    if property.annotations.iter().any(|annotation| {
        matches!(
            annotation.use_site.as_deref(),
            Some("get" | "set" | "setparam")
        )
    }) {
        return Err(Diagnostic::new(
            annotation.offset,
            "Accessor annotations on JvmField properties are unsupported",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "JVM_FIELD_ACCESSOR_ANNOTATION_UNSUPPORTED",
        ));
    }
    Ok(())
}

pub(super) fn class_fields(
    source: &syntax::ClassDecl,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<HashSet<String>> {
    let mut fields = HashSet::new();
    for ((name, _), uses) in source.params.iter().zip(&source.param_annotations) {
        let Some(annotation) = marker(uses, classpath, imports)? else {
            continue;
        };
        if !source.param_properties.contains(name) {
            return Err(invalid(
                annotation.offset,
                "JvmField requires a constructor property",
            ));
        }
        if !source.public_params.contains(name) {
            return Err(invalid(
                annotation.offset,
                "JvmField cannot expose a private property",
            ));
        }
        if source.override_params.contains(name) {
            return Err(invalid(
                annotation.offset,
                "JvmField cannot expose an overriding property",
            ));
        }
        if source.value_ {
            return Err(Diagnostic::new(
                annotation.offset,
                "JvmField value class carrier properties are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "JVM_FIELD_VALUE_CLASS_UNSUPPORTED",
            ));
        }
        fields.insert(name.clone());
    }
    for property in &source.properties {
        let Some(annotation) = marker(&property.annotations, classpath, imports)? else {
            continue;
        };
        validate_property(property, annotation)?;
        fields.insert(property.name.clone());
    }
    Ok(fields)
}

pub(super) fn static_fields(
    properties: &[syntax::ClassCompanionProperty],
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<HashSet<String>> {
    let mut fields = HashSet::new();
    for property in properties {
        let Some(annotation) = marker(&property.annotations, classpath, imports)? else {
            continue;
        };
        let source = syntax::PropertyDecl {
            receiver: None,
            annotations: property.annotations.clone(),
            name: property.name.clone(),
            private: property.private,
            internal: property.internal,
            getter_visibility: property.getter_visibility,
            setter_visibility: property.setter_visibility,
            protected: false,
            mutable: property.mutable,
            inferred: property.ty.is_none(),
            initializer: property.initializer.clone(),
            offset: property.offset,
            ty: property.ty.clone().unwrap_or(Type::Unit),
            body: property.body.clone(),
            is_override: false,
            setter: property.setter.clone(),
            delegate: property.delegate.clone(),
            lateinit: property.lateinit,
            open: false,
        };
        validate_property(&source, annotation)?;
        fields.insert(property.name.clone());
    }
    Ok(fields)
}

pub(super) fn object_fields(
    source: &syntax::ObjectDecl,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<HashSet<String>> {
    let properties = source
        .fields
        .iter()
        .map(|p| syntax::ClassCompanionProperty {
            annotations: p.annotations.clone(),
            name: p.name.clone(),
            offset: p.offset,
            private: p.private,
            internal: p.internal,
            getter_visibility: p.getter_visibility,
            setter_visibility: p.setter_visibility,
            mutable: p.mutable,
            ty: p.ty.clone(),
            initializer: p.initializer.clone(),
            body: p.body.clone(),
            setter: p.setter.clone(),
            delegate: p.delegate.clone(),
            lateinit: p.lateinit,
        })
        .collect::<Vec<_>>();
    let fields = static_fields(&properties, classpath, imports)?;
    if fields.contains("INSTANCE") {
        return Err(Diagnostic::new(source.offset, "JvmField names colliding with singleton storage require unsupported field disambiguation").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "JVM_FIELD_STATIC_COLLISION_UNSUPPORTED"));
    }
    for property in &source.fields {
        if fields.contains(&property.name) && property.is_override {
            return Err(invalid(
                property.offset,
                "JvmField cannot expose an overriding property",
            ));
        }
    }
    Ok(fields)
}

pub(super) fn preflight(source: &syntax::Module, classpath: &Classpath) -> Result<()> {
    let imports = crate::annotation_declarations::imports(source, classpath)?;
    let constants = |values: &[syntax::ConstDecl]| -> Result<()> {
        for value in values {
            if let Some(annotation) = marker(&value.annotations, classpath, &imports)? {
                return Err(invalid(
                    annotation.offset,
                    "JvmField cannot be combined with const",
                ));
            }
        }
        Ok(())
    };
    let static_properties = |values: Vec<&[syntax::AnnotationUse]>| -> Result<()> {
        for uses in values {
            if let Some(annotation) = marker(uses, classpath, &imports)? {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Static JvmField properties are unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "JVM_FIELD_STATIC_UNSUPPORTED",
                ));
            }
        }
        Ok(())
    };
    constants(&source.constants)?;
    static_properties(
        source
            .properties
            .iter()
            .map(|p| p.annotations.as_slice())
            .collect(),
    )?;
    if let Some(class) = &source.class_decl {
        class_fields(class, classpath, &imports)?;
        if let Some(companion) = &class.companion {
            let exposed = static_fields(&companion.properties, classpath, &imports)?;
            if exposed.iter().any(|name| {
                name == "Companion"
                    || class.param_properties.contains(name)
                    || class.properties.iter().any(|p| &p.name == name)
            }) {
                return Err(Diagnostic::new(class.offset, "JvmField names colliding with class storage require unsupported field disambiguation").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "JVM_FIELD_STATIC_COLLISION_UNSUPPORTED"));
            }
        }
    }
    if let Some(object) = &source.object_decl {
        constants(&object.constants)?;
        object_fields(object, classpath, &imports)?;
    }
    if let Some(interface) = &source.interface_decl {
        for property in &interface.properties {
            if let Some(annotation) = marker(&property.annotations, classpath, &imports)? {
                return Err(invalid(
                    annotation.offset,
                    "JvmField requires a property backing field",
                ));
            }
        }
        if let Some(companion) = &interface.companion {
            static_properties(
                companion
                    .fields
                    .iter()
                    .map(|p| p.annotations.as_slice())
                    .chain(
                        companion
                            .properties
                            .iter()
                            .map(|p| p.annotations.as_slice()),
                    )
                    .collect(),
            )?;
        }
    }
    if let Some(enumeration) = &source.enum_decl {
        if let Some(companion) = &enumeration.body.companion {
            for property in &companion.properties {
                if let Some(annotation) = marker(&property.annotations, classpath, &imports)? {
                    return Err(Diagnostic::new(
                        annotation.offset,
                        "Enum companion JvmField properties remain unsupported",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "JVM_FIELD_ENUM_UNSUPPORTED",
                    ));
                }
            }
        }
        for uses in &enumeration.body.param_annotations {
            if let Some(annotation) = marker(uses, classpath, &imports)? {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Enum JvmField properties are unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "JVM_FIELD_ENUM_UNSUPPORTED",
                ));
            }
        }
        for property in &enumeration.body.properties {
            if let Some(annotation) = marker(&property.annotations, classpath, &imports)? {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Enum JvmField properties are unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "JVM_FIELD_ENUM_UNSUPPORTED",
                ));
            }
        }
    }
    Ok(())
}
