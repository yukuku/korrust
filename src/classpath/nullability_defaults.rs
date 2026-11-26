use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct JavaDefaults {
    result: Option<bool>,
    parameter: Option<bool>,
    field: Option<bool>,
    nested: Option<bool>,
}

fn unsupported(message: &str) -> Diagnostic {
    Diagnostic::new(0, message).with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "JAVA_NULLABILITY_DEFAULT_UNSUPPORTED",
    )
}

impl JavaDefaults {
    fn merge(&mut self, other: Self) -> Result<()> {
        for (old, new) in [
            (&mut self.result, other.result),
            (&mut self.parameter, other.parameter),
            (&mut self.field, other.field),
            (&mut self.nested, other.nested),
        ] {
            if let Some(new) = new {
                if old.is_some_and(|old| old != new) {
                    return Err(unsupported("Conflicting Java package nullability defaults"));
                }
                *old = Some(new);
            }
        }
        Ok(())
    }

    fn active(self) -> bool {
        self.result.is_some() || self.parameter.is_some() || self.field.is_some()
    }
}

impl Classpath {
    fn raw_annotation(&self, annotation: &Annotation) -> Result<Option<Attributes>> {
        let owner = annotation
            .name
            .strip_prefix('L')
            .and_then(|name| name.strip_suffix(';'))
            .filter(|name| valid_name(name))
            .ok_or_else(invalid)?;
        for path in &self.paths {
            if let Some(bytes) = self.read_resource(path, &format!("{owner}.class"))? {
                let (class, attrs, _, _) = parse_details_impl(&bytes, owner, false)?;
                if class.access & 0x2000 == 0 || attrs.unsupported {
                    return Err(unsupported("Malformed Java default qualifier annotation"));
                }
                return Ok(Some(attrs));
            }
        }
        Ok(None)
    }

    fn default_polarity(&self, annotation: &Annotation, depth: usize) -> Result<Option<bool>> {
        if depth > 32 {
            return Err(unsupported("Recursive Java type qualifier nickname"));
        }
        if matches!(annotation.name.as_str(), "Ljavax/annotation/Nonnull;") {
            if annotation.values.is_empty()
                || matches!(annotation.values.as_slice(), [(key, AnnotationValue::Enum { descriptor, name })]
                    if key == "when" && descriptor == "Ljavax/annotation/meta/When;" && name == "ALWAYS")
            {
                return Ok(Some(false));
            }
            return Err(unsupported(
                "Java default qualifier uses an unsupported nullness condition",
            ));
        }
        if matches!(annotation.name.as_str(), "Ljavax/annotation/CheckForNull;")
            && annotation.values.is_empty()
        {
            return Ok(Some(true));
        }
        let Some(attrs) = self.raw_annotation(annotation)? else {
            return Ok(None);
        };
        if !attrs
            .annotations
            .iter()
            .any(|annotation| annotation.name == "Ljavax/annotation/meta/TypeQualifierNickname;")
        {
            if attrs
                .annotations
                .iter()
                .any(|annotation| annotation.name == "Ljavax/annotation/meta/TypeQualifier;")
            {
                return Ok(match signature::nullability(Type::String, [annotation]) {
                    Ok(Type::Nullable(_)) => Some(true),
                    Ok(Type::String) => Some(false),
                    _ => None,
                });
            }
            return Ok(None);
        }
        let mut result = None;
        for annotation in &attrs.annotations {
            if let Some(polarity) = self.default_polarity(annotation, depth + 1)? {
                if result.is_some_and(|old| old != polarity) {
                    return Err(unsupported(
                        "Conflicting Java qualifier nickname nullability",
                    ));
                }
                result = Some(polarity);
            }
        }
        Ok(result)
    }

    fn resolve_default(&self, annotation: &Annotation) -> Result<Option<JavaDefaults>> {
        match annotation.name.as_str() {
            "Lorg/jspecify/annotations/NullMarked;" | "Lorg/jspecify/nullness/NullMarked;"
                if annotation.values.is_empty() =>
            {
                return Err(unsupported(
                    "JSpecify default bounds and declaration contexts are unsupported",
                ));
            }
            "Lorg/jspecify/annotations/NullUnmarked;" | "Lorg/jspecify/nullness/NullUnmarked;"
                if annotation.values.is_empty() =>
            {
                return Ok(Some(JavaDefaults::default()));
            }
            _ => {}
        }
        let Some(attrs) = self.raw_annotation(annotation)? else {
            return Ok(None);
        };
        let Some(targets) = attrs
            .annotations
            .iter()
            .find(|annotation| annotation.name == "Ljavax/annotation/meta/TypeQualifierDefault;")
        else {
            return Ok(None);
        };
        if !annotation.values.is_empty() {
            return Err(unsupported(
                "Parameterized Java default qualifier annotations are unsupported",
            ));
        }
        let migration = attrs
            .annotations
            .iter()
            .find(|annotation| annotation.name == "Lkotlin/annotations/jvm/UnderMigration;");
        let strict = match migration {
            None => false,
            Some(annotation) => match annotation.values.as_slice() {
                [(key, AnnotationValue::Enum { descriptor, name })]
                    if key == "status"
                        && descriptor == "Lkotlin/annotations/jvm/MigrationStatus;" =>
                {
                    match name.as_str() {
                        "STRICT" => true,
                        "WARN" | "IGNORE" => false,
                        _ => return Err(unsupported("Unknown Java qualifier migration status")),
                    }
                }
                _ => return Err(unsupported("Malformed Java qualifier migration status")),
            },
        };
        let mut polarity = None;
        for annotation in &attrs.annotations {
            if let Some(value) = self.default_polarity(annotation, 0)? {
                if polarity.is_some_and(|old| old != value) {
                    return Err(unsupported(
                        "Conflicting Java package nullability qualifiers",
                    ));
                }
                polarity = Some(value);
            }
        }
        let polarity = polarity.ok_or_else(|| {
            unsupported("Java default annotation lacks a supported nullability qualifier")
        })?;
        let [(key, AnnotationValue::Array(targets))] = targets.values.as_slice() else {
            return Err(unsupported("Malformed Java default qualifier targets"));
        };
        if key != "value" || targets.is_empty() {
            return Err(unsupported("Malformed Java default qualifier targets"));
        }
        let mut defaults = JavaDefaults::default();
        for target in targets {
            let AnnotationValue::Enum { descriptor, name } = target else {
                return Err(unsupported("Malformed Java default qualifier target"));
            };
            if descriptor != "Ljava/lang/annotation/ElementType;" {
                return Err(unsupported("Malformed Java default qualifier target type"));
            }
            match name.as_str() {
                "METHOD" => defaults.result = Some(polarity),
                "PARAMETER" => defaults.parameter = Some(polarity),
                "FIELD" => defaults.field = Some(polarity),
                "TYPE_USE" => {
                    defaults.result = Some(polarity);
                    defaults.parameter = Some(polarity);
                    defaults.field = Some(polarity);
                    defaults.nested = Some(polarity);
                }
                _ => return Err(unsupported("Java default qualifier target is unsupported")),
            }
        }
        Ok(Some(if strict {
            defaults
        } else {
            JavaDefaults::default()
        }))
    }

    pub(super) fn java_package_defaults(&self, owner: &str) -> Result<JavaDefaults> {
        let resource = owner
            .rsplit_once('/')
            .map(|(package, _)| format!("{package}/package-info.class"))
            .unwrap_or_else(|| "package-info.class".into());
        for path in &self.paths {
            let Some(bytes) = self.read_resource(path, &resource)? else {
                continue;
            };
            if self.passive_package_info(&bytes, &resource)? {
                return Ok(JavaDefaults::default());
            }
            let (class, attrs, _, _) = parse_details_impl(
                &bytes,
                resource.strip_suffix(".class").ok_or_else(invalid)?,
                false,
            )?;
            if class.access & 0x200 == 0
                || !class.fields.is_empty()
                || !class.methods.is_empty()
                || attrs.unsupported
                || !attrs.type_annotation_targets.is_empty()
            {
                return Err(unsupported("Unsupported Java package metadata shape"));
            }
            let mut defaults = JavaDefaults::default();
            for annotation in &attrs.annotations {
                if let Some(selected) = self.resolve_default(annotation)? {
                    defaults.merge(selected)?;
                } else if signature::type_semantic_annotation(&annotation.name)
                    || !self.passive_declaration_annotation(annotation)?
                {
                    return Err(unsupported("Unsupported Java package annotation semantics"));
                }
            }
            return Ok(defaults);
        }
        Ok(JavaDefaults::default())
    }

    pub(super) fn apply_java_package_defaults(
        &self,
        class: &mut JavaClass,
        attrs: &Attributes,
        method_attrs: &[Attributes],
        field_attrs: &[Attributes],
    ) -> Result<()> {
        if class.name.rsplit('/').next() == Some("package-info") {
            return Ok(());
        }
        let defaults = self.java_package_defaults(&class.name)?;
        if !defaults.active() {
            return Ok(());
        }
        for annotation in &attrs.annotations {
            if self.resolve_default(annotation)?.is_some() {
                return Err(unsupported(
                    "Declaration overrides of Java package defaults are unsupported",
                ));
            }
        }
        let mut parent = self.lexical_parents.borrow().get(&class.name).cloned();
        for _ in 0..64 {
            let Some(owner) = parent else { break };
            for path in &self.paths {
                if let Some(bytes) = self.read_resource(path, &format!("{owner}.class"))? {
                    let (_, attrs, _, _) = parse_details_impl(&bytes, &owner, false)?;
                    for annotation in &attrs.annotations {
                        if self.resolve_default(annotation)?.is_some() {
                            return Err(unsupported(
                                "Enclosing declaration Java nullability defaults are unsupported",
                            ));
                        }
                    }
                    break;
                }
            }
            parent = self.lexical_parents.borrow().get(&owner).cloned();
        }
        for (method, attributes) in class.methods.iter_mut().zip(method_attrs) {
            if attributes
                .annotations
                .iter()
                .any(|annotation| !matches!(self.resolve_default(annotation), Ok(None)))
            {
                method.unsupported = true;
            }
            if method.unsupported {
                continue;
            }
            let converted = (|| -> Result<()> {
                if let Some(params) = &mut method.kotlin_params {
                    for (_, ty, _) in params {
                        *ty = enhance(ty.clone(), defaults.parameter, defaults.nested, false)?;
                    }
                }
                if let Some(ty) = &mut method.kotlin_result {
                    *ty = enhance(ty.clone(), defaults.result, defaults.nested, false)?;
                }
                Ok(())
            })();
            if converted.is_err() {
                method.unsupported = true;
            }
        }
        for (field, attributes) in class.fields.iter_mut().zip(field_attrs) {
            if attributes
                .annotations
                .iter()
                .any(|annotation| !matches!(self.resolve_default(annotation), Ok(None)))
            {
                field.unsupported = true;
            }
            if field.unsupported {
                continue;
            }
            let ty = field.java_type.as_deref().cloned().or_else(|| {
                signature::field(&field.descriptor, &[])
                    .ok()
                    .map(|ty| Type::Platform(Box::new(ty)))
            });
            if let Some(ty) = ty {
                match enhance(ty, defaults.field, defaults.nested, false) {
                    Ok(ty) => field.java_type = Some(Box::new(ty)),
                    Err(_) => field.unsupported = true,
                }
            }
        }
        Ok(())
    }
}

fn enhance(ty: Type, root: Option<bool>, nested: Option<bool>, implicit: bool) -> Result<Type> {
    if matches!(ty, Type::Parameter { .. }) && root.is_some() {
        return Err(unsupported(
            "Java defaults on type parameters require definitely non-null type support",
        ));
    }
    let ty = match ty {
        Type::Platform(inner) => {
            if root.is_some() && matches!(inner.base(), Type::Parameter { .. }) {
                return Err(unsupported(
                    "Java defaults on type parameters require definitely non-null type support",
                ));
            }
            let inner = enhance(*inner, None, nested, false)?;
            return Ok(match root {
                Some(true) => Type::Nullable(Box::new(inner)),
                Some(false) => inner,
                None => Type::Platform(Box::new(inner)),
            });
        }
        Type::Nullable(inner) => {
            return Ok(Type::Nullable(Box::new(enhance(
                *inner, None, nested, false,
            )?)));
        }
        Type::Array(inner) => Type::Array(Box::new(enhance(*inner, nested, nested, true)?)),
        Type::OutArray(inner) => Type::OutArray(Box::new(enhance(*inner, nested, nested, true)?)),
        Type::StringArray if nested == Some(true) => {
            Type::Array(Box::new(enhance(Type::String, nested, nested, true)?))
        }
        Type::List(inner) => Type::List(Box::new(enhance(*inner, nested, nested, true)?)),
        Type::MutableList(inner) => {
            Type::MutableList(Box::new(enhance(*inner, nested, nested, true)?))
        }
        Type::FlexibleList(inner) => {
            Type::FlexibleList(Box::new(enhance(*inner, nested, nested, true)?))
        }
        Type::Parameterized {
            name,
            mut arguments,
        } => {
            for argument in &mut arguments {
                if let Some(ty) = &mut argument.ty {
                    *ty = enhance(ty.clone(), nested, nested, true)?;
                }
            }
            Type::Parameterized { name, arguments }
        }
        ty => ty,
    };
    if implicit && root == Some(true) && ty.is_reference() {
        Ok(Type::Nullable(Box::new(ty)))
    } else {
        Ok(ty)
    }
}
