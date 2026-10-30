use super::*;

fn literal_error(offset: usize, message: &str, invalid: bool) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(
        if invalid {
            crate::DiagnosticKind::InvalidKotlin
        } else {
            crate::DiagnosticKind::UnsupportedLanguage
        },
        if invalid {
            "INVALID_CLASS_LITERAL"
        } else {
            "CLASS_LITERAL_UNSUPPORTED"
        },
    )
}

fn source_path(expression: &Expr) -> Option<Vec<String>> {
    match &expression.kind {
        ExprKind::Name(name) => Some(vec![name.clone()]),
        ExprKind::Property(receiver, name) => {
            let mut names = source_path(receiver)?;
            names.push(name.clone());
            Some(names)
        }
        _ => None,
    }
}

fn source_type_supported(ty: &Type) -> bool {
    match ty {
        Type::Java(name) => !matches!(
            name.as_str(),
            "Nothing" | "kotlin/Nothing" | "ShortArray" | "kotlin/ShortArray"
        ),
        Type::Array(inner) | Type::Nullable(inner) => source_type_supported(inner),
        _ => true,
    }
}

fn bound_type_supported(ty: &Type) -> bool {
    match ty.base() {
        Type::Java(name) => !matches!(name.as_str(), "ShortArray" | "kotlin/ShortArray" | "[S"),
        Type::Array(inner) | Type::Nullable(inner) => bound_type_supported(inner),
        _ => true,
    }
}

fn nullable_parameter_bound(ty: &Type, active: &mut HashSet<String>, depth: usize) -> Option<bool> {
    if depth >= 64 {
        return None;
    }
    match ty {
        Type::Platform(_) => Some(false),
        Type::Parameter { name, upper_bound } => {
            if !active.insert(name.clone()) {
                return None;
            }
            let result = nullable_parameter_bound(upper_bound, active, depth + 1);
            active.remove(name);
            result
        }
        Type::Nullable(_) | Type::Null => Some(true),
        _ => Some(false),
    }
}

fn normalized_literal_type(ty: &Type) -> Type {
    match ty {
        Type::Java(name) => match name.as_str() {
            "kotlin/Int" => Type::Int,
            "kotlin/Long" => Type::Long,
            "kotlin/Byte" => Type::Byte,
            "kotlin/Short" => Type::Short,
            "kotlin/Char" => Type::Char,
            "kotlin/Boolean" => Type::Boolean,
            "kotlin/Float" => Type::Float,
            "kotlin/Double" => Type::Double,
            "kotlin/String" => Type::String,
            "kotlin/Unit" => Type::Unit,
            "kotlin/Any" => Type::Java("java/lang/Object".into()),
            "kotlin/CharSequence" => Type::Java("java/lang/CharSequence".into()),
            "kotlin/IntArray" => Type::IntArray,
            "kotlin/ByteArray" => Type::ByteArray,
            "kotlin/CharArray" => Type::CharArray,
            "kotlin/LongArray" => Type::LongArray,
            "kotlin/BooleanArray" => Type::BooleanArray,
            "kotlin/FloatArray" => Type::FloatArray,
            "kotlin/DoubleArray" => Type::DoubleArray,
            _ => ty.clone(),
        },
        Type::Array(inner) => Type::Array(Box::new(normalized_literal_type(inner))),
        Type::Nullable(inner) => Type::Nullable(Box::new(normalized_literal_type(inner))),
        Type::Parameterized { name, arguments } if name == "kotlin/Array" => {
            if let [
                syntax::TypeArgument {
                    variance: syntax::Variance::Invariant,
                    ty: Some(inner),
                },
            ] = arguments.as_slice()
            {
                Type::Array(Box::new(normalized_literal_type(inner)))
            } else {
                Type::Parameterized {
                    name: "Array".into(),
                    arguments: arguments.clone(),
                }
            }
        }
        _ => ty.clone(),
    }
}

fn array_literal_shape(ty: &Type, offset: usize, depth: usize) -> Result<()> {
    if depth >= 64 {
        return Err(literal_error(
            offset,
            "Class literal array nesting exceeds the supported depth",
            false,
        ));
    }
    match ty {
        Type::Array(inner) | Type::Nullable(inner) => array_literal_shape(inner, offset, depth + 1),
        Type::Parameterized { name, arguments } if name == "Array" => {
            let [
                syntax::TypeArgument {
                    ty: Some(inner), ..
                },
            ] = arguments.as_slice()
            else {
                return Err(literal_error(
                    offset,
                    "Array class literals require one non-star type argument at every array layer",
                    true,
                ));
            };
            array_literal_shape(inner, offset, depth + 1)?;
            Err(literal_error(
                offset,
                "Projected array class literal metadata is unsupported",
                false,
            ))
        }
        Type::List(_)
        | Type::MutableList(_)
        | Type::ThreadLocal(_)
        | Type::Parameterized { .. }
        | Type::Function { .. }
        | Type::ReceiverFunction { .. }
        | Type::Parameter { .. } => Err(literal_error(
            offset,
            "Array class literal components must be runtime-available classifiers without generic arguments",
            true,
        )),
        Type::Java(name) if name == "Array" || name == "kotlin/Array" => Err(literal_error(
            offset,
            "Array component classifiers require an explicit element type",
            true,
        )),
        _ => Ok(()),
    }
}

impl Checker<'_> {
    fn class_literal_type(&self, source: &Type, offset: usize) -> Result<Type> {
        if let Type::Array(inner) = source
            && matches!(inner.base(), Type::Parameter { name, .. } if self.reified_parameters.iter().any(|parameter| parameter.name == *name && parameter.reified))
        {
            return Err(literal_error(
                offset,
                "Nested reified array class literals are unsupported",
                false,
            ));
        }
        if let Type::Parameter { name, .. } = source
            && self
                .reified_parameters
                .iter()
                .any(|parameter| parameter.name == *name && parameter.reified)
        {
            return Ok(source.clone());
        }
        if !source_type_supported(source) {
            return Err(literal_error(
                offset,
                "Class literal uses an unsupported scalar or primitive-array type",
                false,
            ));
        }
        if source.is_nullable() {
            return Err(literal_error(
                offset,
                "Type in a class literal cannot be nullable",
                true,
            ));
        }
        let source = normalized_literal_type(source);
        let expanded = self.classpath.expand_type_alias(&source, self.imports, library::DEFAULT_PACKAGES)
            .map_err(|error| if error.code == "TYPE_ALIAS_ARITY" && matches!(source, Type::Java(_)) {
                literal_error(offset, "Bare generic type-alias class literals require unsupported star-argument inference", false)
            } else { error })?;
        if expanded.is_nullable() {
            return Err(literal_error(
                offset,
                "Type alias in a class literal expands to a nullable type",
                true,
            ));
        }
        if matches!(
            expanded,
            Type::Function { .. } | Type::ReceiverFunction { .. }
        ) {
            return Err(literal_error(
                offset,
                "Function-type alias class literals are unsupported",
                false,
            ));
        }
        if !source_type_supported(&expanded) {
            return Err(literal_error(
                offset,
                "Class literal alias expands to an unsupported scalar or primitive-array type",
                false,
            ));
        }
        if expanded.base() == &Type::Unit {
            return Err(literal_error(
                offset,
                "Unit class literals require unsupported Unit generic-value semantics",
                false,
            ));
        }
        if matches!(
            &expanded,
            Type::List(_)
                | Type::MutableList(_)
                | Type::ThreadLocal(_)
                | Type::Parameterized { .. }
        ) {
            if matches!(&expanded, Type::Parameterized { name, arguments } if name == "Array" && arguments.len() != 1)
            {
                return Err(literal_error(
                    offset,
                    "Array class literals require exactly one type argument",
                    true,
                ));
            }
            if matches!(source, Type::Java(_)) && expanded != source {
                return Err(literal_error(
                    offset,
                    "Concrete generic type-alias class literals require unsupported retained classifier arguments",
                    false,
                ));
            }
            if matches!(&expanded, Type::Parameterized { name, arguments } if name == "Array" && arguments.iter().any(|argument| argument.ty.is_none()))
            {
                return Err(literal_error(
                    offset,
                    "Star-projected Array class literal is not a runtime-available classifier",
                    true,
                ));
            }
            return Err(literal_error(
                offset,
                "Parameterized class literals are supported only for concrete invariant arrays",
                !matches!(&expanded, Type::Parameterized { name, .. } if name == "Array"),
            ));
        }
        if matches!(&expanded, Type::Java(name) if name == "Array" || name == "kotlin/Array") {
            return Err(literal_error(
                offset,
                "Bare Array class literals require unsupported array star-projection semantics",
                false,
            ));
        }
        if matches!(expanded, Type::Parameter { .. }) {
            return Err(literal_error(
                offset,
                "Class literals require runtime-available type arguments",
                true,
            ));
        }
        if matches!(expanded, Type::Array(_) | Type::StringArray) {
            array_literal_shape(&expanded, offset, 0)?;
        }
        let imports = library::default_type_imports(&expanded, self.imports, self.classpath);
        let ty = self.class_literal_classifier_type(
            resolve_type(&expanded, &imports, offset)?,
            true,
            offset,
            0,
        )?;
        if !bound_type_supported(&ty) {
            return Err(literal_error(
                offset,
                "Class literals involving Short require unsupported primitive/reference distinction",
                false,
            ));
        }
        self.validate_type_arguments(&ty, offset).map_err(|error| {
            if error.message == "Source generic classifiers require explicit type arguments" {
                literal_error(
                    offset,
                    "Array component generic classifiers require explicit arguments",
                    true,
                )
            } else {
                error
            }
        })?;
        Ok(ty)
    }

    fn class_literal_classifier_type(
        &self,
        ty: Type,
        root: bool,
        offset: usize,
        depth: usize,
    ) -> Result<Type> {
        if depth >= 64 {
            return Err(literal_error(
                offset,
                "Class literal classifier nesting exceeds the supported depth",
                false,
            ));
        }
        match ty {
            Type::Array(inner) => Ok(Type::Array(Box::new(self.class_literal_classifier_type(
                *inner,
                false,
                offset,
                depth + 1,
            )?))),
            Type::Nullable(inner) => Ok(Type::Nullable(Box::new(
                self.class_literal_classifier_type(*inner, false, offset, depth + 1)?,
            ))),
            Type::Java(owner) => {
                let owner = self.class_literal_owner(&owner, offset).unwrap_or(owner);
                let class = self
                    .classes
                    .get(&owner)
                    .cloned()
                    .map(Ok)
                    .unwrap_or_else(|| self.classpath.load(&owner))?;
                self.class_literal_visibility(&owner, offset)?;
                if class.unsupported && (class.kotlin_class || class.library.is_none()) {
                    return Err(literal_error(
                        offset,
                        "Class literal classifier carries unsupported metadata",
                        false,
                    ));
                }
                if let Some(library) = class.library
                    && !library.type_parameters.is_empty()
                {
                    if !root {
                        return Err(literal_error(
                            offset,
                            "Array component generic classifiers require explicit arguments",
                            true,
                        ));
                    }
                    return Ok(Type::Parameterized {
                        name: owner,
                        arguments: library
                            .type_parameters
                            .iter()
                            .map(|_| syntax::TypeArgument {
                                variance: syntax::Variance::Invariant,
                                ty: None,
                            })
                            .collect(),
                    });
                }
                Ok(Type::Java(owner))
            }
            ty => Ok(ty),
        }
    }

    fn class_literal_value_visibility(
        &self,
        ty: &Type,
        active: &mut HashSet<String>,
        offset: usize,
        depth: usize,
    ) -> Result<()> {
        if depth >= 64 {
            return Err(literal_error(
                offset,
                "Bound class literal type visibility exceeds the supported depth",
                false,
            ));
        }
        match ty {
            Type::Java(owner) => self.class_literal_visibility(owner, offset),
            Type::Parameterized { name, arguments } => {
                self.class_literal_visibility(name, offset)?;
                for argument in arguments {
                    if let Some(ty) = &argument.ty {
                        self.class_literal_value_visibility(ty, active, offset, depth + 1)?;
                    }
                }
                Ok(())
            }
            Type::Parameter { name, upper_bound } if active.insert(name.clone()) => {
                let result =
                    self.class_literal_value_visibility(upper_bound, active, offset, depth + 1);
                active.remove(name);
                result
            }
            Type::Nullable(inner)
            | Type::Platform(inner)
            | Type::Array(inner)
            | Type::List(inner)
            | Type::MutableList(inner)
            | Type::ThreadLocal(inner) => {
                self.class_literal_value_visibility(inner, active, offset, depth + 1)
            }
            Type::Function { params, result } => {
                for param in params {
                    self.class_literal_value_visibility(param, active, offset, depth + 1)?;
                }
                self.class_literal_value_visibility(result, active, offset, depth + 1)
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => {
                self.class_literal_value_visibility(receiver, active, offset, depth + 1)?;
                for param in params {
                    self.class_literal_value_visibility(param, active, offset, depth + 1)?;
                }
                self.class_literal_value_visibility(result, active, offset, depth + 1)
            }
            _ => Ok(()),
        }
    }

    fn class_literal_visibility(&self, owner: &str, offset: usize) -> Result<()> {
        let mut owner = owner;
        for _ in 0..64 {
            let class = self.classpath.load(owner)?;
            if class
                .library
                .as_ref()
                .is_some_and(|library| library.package.is_some())
            {
                return Err(literal_error(
                    offset,
                    "Kotlin file facades are not source class literal classifiers",
                    true,
                ));
            }
            if self.classpath.is_source(owner)
                && class.kotlin_class
                && class
                    .library
                    .as_ref()
                    .and_then(|library| library.kotlin_visibility)
                    != Some(3)
            {
                return Err(literal_error(offset, "Nonpublic or anonymous source classifier literals require unsupported public API exposure approximation", false)
                    .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "CLASS_LITERAL_VISIBILITY_UNSUPPORTED"));
            }
            if !self.classpath.is_source(owner) {
                if class.kotlin_class {
                    match class.library.as_ref().and_then(|library| library.kotlin_visibility) {
                        Some(3) => {},
                        Some(0) if self.classpath.internal_visible(owner) => {},
                        Some(0) => return Err(literal_error(offset, "Compiled internal Kotlin classifier is inaccessible", true)),
                        Some(1 | 4) if !owner.contains('$') => return Err(literal_error(offset, "Compiled file-private Kotlin classifier is inaccessible", true)),
                        Some(1 | 4) if owner.rsplit_once('$').is_some_and(|(outer, _)| self.lambda_owner != outer && !self.lambda_owner.starts_with(&format!("{outer}$"))) => return Err(literal_error(offset, "Compiled private nested Kotlin classifier is inaccessible", true)),
                        Some(2) if owner.rsplit_once('$').is_some_and(|(outer, _)| {
                            let caller = self.lambda_owner.split("$lambda$").next().unwrap();
                            self.classpath.load(caller).is_ok() && !self.subtype(&Type::Java(caller.into()), &Type::Java(outer.into()))
                        }) => return Err(literal_error(offset, "Compiled protected Kotlin classifier is inaccessible from this declaration", true)),
                        _ => return Err(literal_error(offset, "Nonpublic or unknown Kotlin classifier visibility requires unsupported lexical access semantics", false)
                            .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "CLASS_LITERAL_VISIBILITY_UNSUPPORTED")),
                    }
                }
                if !class.kotlin_class && owner.contains('$') {
                    return Err(literal_error(offset, "Java nested classifier literals require unsupported InnerClasses visibility provenance", false)
                        .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "CLASS_LITERAL_VISIBILITY_UNSUPPORTED"));
                }
                if class.access & 1 == 0 {
                    if owner.rsplit_once('/').map(|(package, _)| package)
                        == self
                            .lambda_owner
                            .rsplit_once('/')
                            .map(|(package, _)| package)
                    {
                        return Err(literal_error(
                            offset,
                            "Package-private classifier access in class literals is unsupported",
                            false,
                        )
                        .with_kind(
                            crate::DiagnosticKind::UnsupportedLanguage,
                            "CLASS_LITERAL_VISIBILITY_UNSUPPORTED",
                        ));
                    }
                    return Err(literal_error(
                        offset,
                        "Class literal names an inaccessible class",
                        true,
                    ));
                }
            }
            let Some((outer, _)) = owner.rsplit_once('$') else {
                return Ok(());
            };
            owner = outer;
        }
        Err(literal_error(
            offset,
            "Class literal containing-class chain exceeds the supported depth",
            false,
        ))
    }

    fn class_literal_owner(&self, owner: &str, offset: usize) -> Option<String> {
        if self.classpath.load(owner).is_ok() {
            return Some(owner.into());
        }
        let names = owner.split('/').collect::<Vec<_>>();
        for split in (1..names.len()).rev() {
            let prefix = names[..split].join("/");
            let source = Type::Java(prefix);
            let imports = library::default_type_imports(&source, self.imports, self.classpath);
            let Ok(Type::Java(prefix)) = resolve_type(&source, &imports, offset) else {
                continue;
            };
            let candidate = format!("{prefix}${}", names[split..].join("$"));
            if self.classpath.load(&candidate).is_ok() {
                return Some(candidate);
            }
        }
        None
    }

    fn class_literal_receiver(&self, value: Value, offset: usize) -> Result<Value> {
        if value.ty.is_reference() {
            return Ok(value);
        }
        let descriptor = match &value.kind {
            ValueKind::MemberCall { method, .. } | ValueKind::JavaCall { method, .. } => {
                Some(method.descriptor.clone())
            }
            ValueKind::StaticCall { descriptor, .. } => Some(descriptor.clone()),
            ValueKind::Call(id, _) => Some(format!("(){}", self.signatures[*id].1.descriptor())),
            _ => None,
        };
        if descriptor.as_ref().is_some_and(|descriptor| {
            descriptor
                .rsplit_once(')')
                .is_some_and(|(_, result)| result.starts_with('L') || result.starts_with('['))
        }) || matches!(
            value.kind,
            ValueKind::FunctionInvoke { .. } | ValueKind::ListGet(..) | ValueKind::ListSet(..)
        ) || matches!(&value.kind, ValueKind::Index(array, _) if matches!(array.ty.base(), Type::Array(_) | Type::StringArray))
        {
            return Ok(Value {
                ty: Type::Java("java/lang/Object".into()),
                kind: value.kind,
            });
        }
        match value.kind {
            ValueKind::CompiledInline { .. } => Err(literal_error(offset, "Primitive class literals on compiled inline results require unsupported physical result semantics", false)
                .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "CLASS_LITERAL_REPRESENTATION_UNSUPPORTED")),
            ValueKind::Unbox(_) | ValueKind::Cast { .. } => Ok(Value {
                ty: Type::Java("java/lang/Object".into()), kind: ValueKind::Box(Box::new(value)),
            }),
            ValueKind::Let { slot, value: initializer, body } => {
                let body = self.class_literal_receiver(*body, offset)?;
                Ok(Value { ty: body.ty.clone(), kind: ValueKind::Let { slot, value: initializer, body: Box::new(body) } })
            }
            ValueKind::Block(body, result) => {
                let result = self.class_literal_receiver(*result, offset)?;
                Ok(Value { ty: result.ty.clone(), kind: ValueKind::Block(body, Box::new(result)) })
            }
            kind => Ok(Value { ty: value.ty, kind }),
        }
    }

    pub(super) fn class_literal(
        &mut self,
        receiver: &Expr,
        classifier: Option<&Type>,
        offset: usize,
    ) -> Result<Value> {
        if classifier.is_some_and(Type::is_nullable) {
            return Err(literal_error(
                offset,
                "Nullable type syntax is invalid in a class literal",
                true,
            ));
        }
        let path = source_path(receiver);
        let qualified_value = path.as_ref().is_some_and(|path| {
            path.len() > 1
                && self.imports.get(&path[0]).is_some_and(|owner| {
                    self.classpath.load(owner).is_ok_and(|class| {
                        class.fields.iter().any(|field| field.name == path[1])
                            || self.classpath.methods(owner).is_ok_and(|methods| {
                                methods.iter().any(|method| {
                                    method.source_property.as_deref() == Some(&path[1])
                                })
                            })
                    })
                })
        });
        let bound = qualified_value
            || path.as_ref().is_none_or(|path| {
                let head = &path[0];
                self.local(head).is_some()
                    || self.captured.contains_key(head)
                    || self.constants.contains_key(head)
                    || self.fields.contains_key(head)
                    || self.object_fields.contains_key(head)
                    || self.companion_fields.contains_key(head)
                    || matches!(head.as_str(), "this" | "super")
                    || self
                        .members
                        .values()
                        .any(|method| method.source_property.as_deref() == Some(head))
            });
        if !bound
            && path
                .as_ref()
                .is_some_and(|path| path.iter().any(|name| name.contains('$')))
        {
            return Err(literal_error(
                offset,
                "Classifiers containing '$' require unsupported JVM nesting disambiguation",
                false,
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "CLASSIFIER_DOLLAR_NAME_UNSUPPORTED",
            ));
        }
        let (argument, token) = if !bound {
            let source = classifier
                .cloned()
                .or_else(|| {
                    path.as_ref().map(|names| match names.join("/").as_str() {
                        "Int" => Type::Int,
                        "Long" => Type::Long,
                        "Byte" => Type::Byte,
                        "Short" => Type::Short,
                        "Char" => Type::Char,
                        "Boolean" => Type::Boolean,
                        "Float" => Type::Float,
                        "Double" => Type::Double,
                        "String" => Type::String,
                        "Unit" => Type::Unit,
                        "Any" => Type::Java("java/lang/Object".into()),
                        "IntArray" => Type::IntArray,
                        "ByteArray" => Type::ByteArray,
                        "CharArray" => Type::CharArray,
                        "LongArray" => Type::LongArray,
                        "BooleanArray" => Type::BooleanArray,
                        "FloatArray" => Type::FloatArray,
                        "DoubleArray" => Type::DoubleArray,
                        name => Type::Java(name.into()),
                    })
                })
                .ok_or_else(|| {
                    literal_error(offset, "Unsupported class literal classifier", false)
                })?;
            let source = if let Some(path) = &path
                && path.len() == 1
                && let Some(owner) = self.imports.get(&path[0])
            {
                Type::Java(owner.clone())
            } else {
                source
            };
            let ty = self.class_literal_type(&source, offset)?;
            (
                syntax::TypeArgument {
                    variance: syntax::Variance::Invariant,
                    ty: Some(ty.clone()),
                },
                Value {
                    ty: Type::Java("java/lang/Class".into()),
                    kind: ValueKind::ClassLiteral(ty),
                },
            )
        } else {
            if classifier.is_some_and(|ty| {
                matches!(
                    ty,
                    Type::Parameterized { .. }
                        | Type::Array(_)
                        | Type::StringArray
                        | Type::List(_)
                        | Type::MutableList(_)
                        | Type::ThreadLocal(_)
                        | Type::Function { .. }
                        | Type::ReceiverFunction { .. }
                )
            }) {
                return Err(literal_error(
                    offset,
                    "Value class literals cannot have type arguments",
                    true,
                ));
            }
            if matches!(&receiver.kind, ExprKind::Call(target, _) if matches!(target.kind, ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }))
            {
                return Err(literal_error(offset, "Class literals on immediate lambda invocation require unsupported direct-invocation representation semantics", false)
                    .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "CLASS_LITERAL_REPRESENTATION_UNSUPPORTED"));
            }
            let value = self.value(receiver)?;
            if value.ty.is_nullable() || value.ty == Type::Null {
                return Err(literal_error(
                    offset,
                    "Expression in a class literal has nullable type",
                    true,
                ));
            }
            if matches!(value.ty, Type::Parameter { .. }) {
                match nullable_parameter_bound(&value.ty, &mut HashSet::new(), 0) {
                    Some(false) => {},
                    Some(true) => {
                        let refined = matches!(&receiver.kind, ExprKind::Name(name) if self.local(name).is_some_and(|local| self.refinements.contains_key(&local.slot)));
                        let unsupported = refined || matches!(receiver.kind, ExprKind::NonNull(_));
                        return Err(literal_error(offset, if unsupported { "Class literals require unsupported definitely non-null type-parameter semantics" } else { "Expression in a class literal has a possibly nullable type parameter" }, !unsupported));
                    },
                    None => return Err(literal_error(offset, "Class literal type-parameter bound lookup exceeds the supported depth or cycle boundary", false)
                        .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "CLASS_LITERAL_BOUND_DEPTH_UNSUPPORTED")),
                }
            }
            if value.ty == Type::Unit {
                return Err(literal_error(
                    offset,
                    "Unit value class literals require unsupported Unit generic-value semantics",
                    false,
                ));
            }
            if !bound_type_supported(&value.ty) {
                return Err(literal_error(
                    offset,
                    "Bound Short class literals require unsupported primitive Short semantics",
                    false,
                ));
            }
            let ty = value.ty.clone();
            self.class_literal_value_visibility(&ty, &mut HashSet::new(), offset, 0)?;
            let value = self.class_literal_receiver(value, offset)?;
            let token = if value.ty.is_reference() {
                let method = self
                    .classpath
                    .load("java/lang/Object")?
                    .methods
                    .into_iter()
                    .find(|method| {
                        method.name == "getClass"
                            && method.descriptor == "()Ljava/lang/Class;"
                            && method.access & 9 == 1
                    })
                    .ok_or_else(|| {
                        literal_error(offset, "Object.getClass declaration is unavailable", false)
                    })?;
                self.member_call(method, value, &[], offset)?
            } else {
                Value {
                    ty: Type::Java("java/lang/Class".into()),
                    kind: ValueKind::Block(
                        vec![Statement::Expression(value)],
                        Box::new(Value {
                            ty: Type::Java("java/lang/Class".into()),
                            kind: ValueKind::ClassLiteral(ty.clone()),
                        }),
                    ),
                }
            };
            (
                syntax::TypeArgument {
                    variance: syntax::Variance::Out,
                    ty: Some(ty),
                },
                token,
            )
        };
        let method = self
            .classpath
            .methods("kotlin/jvm/internal/Reflection")?
            .into_iter()
            .find(|method| {
                method.name == "getOrCreateKotlinClass"
                    && method.descriptor == "(Ljava/lang/Class;)Lkotlin/reflect/KClass;"
                    && method.access & 9 == 9
                    && !method.unsupported
            })
            .ok_or_else(|| {
                literal_error(
                    offset,
                    "Kotlin class literal runtime declaration is unavailable",
                    false,
                )
            })?;
        Ok(Value {
            ty: Type::Parameterized {
                name: "kotlin/reflect/KClass".into(),
                arguments: vec![argument],
            },
            kind: ValueKind::StaticCall {
                owner: method.owner,
                name: method.name,
                descriptor: method.descriptor,
                args: vec![token],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn class_literal_template_boundary_has_an_explicit_unsupported_diagnostic() {
        let error = crate::compile_with_classpath(
            "fun boundary(): String { val token = String::class; return \"$token\" }",
            "Boundary.kt",
            &[std::path::PathBuf::from(
                "references/kotlinc/lib/kotlin-stdlib.jar",
            )],
        )
        .err()
        .unwrap();
        assert_eq!(error.kind, crate::DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "STRING_TEMPLATE_VALUE_UNSUPPORTED");
    }

    #[test]
    fn parameter_bound_inspection_preserves_nullability_and_limits_resource_use() {
        fn chain(count: usize, nullable: bool) -> Type {
            let mut ty = Type::Java("java/lang/Object".into());
            if nullable {
                ty = Type::Nullable(Box::new(ty));
            }
            for index in 0..count {
                ty = Type::Parameter {
                    name: format!("T{index}"),
                    upper_bound: Box::new(ty),
                };
            }
            ty
        }
        assert_eq!(
            nullable_parameter_bound(&chain(8, false), &mut HashSet::new(), 0),
            Some(false)
        );
        assert_eq!(
            nullable_parameter_bound(&chain(8, true), &mut HashSet::new(), 0),
            Some(true)
        );
        assert_eq!(
            nullable_parameter_bound(&chain(80, false), &mut HashSet::new(), 0),
            None
        );
    }

    #[test]
    fn generic_extension_property_metadata_preserves_declaration_scope() {
        let cp = Classpath::new(&[std::path::PathBuf::from(
            "references/kotlinc/lib/kotlin-stdlib.jar",
        )])
        .unwrap();
        let methods = cp.package_methods("kotlin/jvm", "java").unwrap();
        let getter = methods
            .iter()
            .find(|method| method.source_property.as_deref() == Some("java"))
            .unwrap();
        assert!(!getter.unsupported);
        assert_eq!(getter.name, "getJavaClass");
        let library = getter.library.as_ref().unwrap();
        assert_eq!(library.type_parameters.len(), 1);
        let Type::Parameterized { name, arguments } = library.receiver.as_ref().unwrap() else {
            panic!("expected generic extension receiver");
        };
        assert_eq!(name, "kotlin/reflect/KClass");
        assert!(
            matches!(&arguments[0].ty, Some(Type::Parameter { name, .. }) if name == &library.type_parameters[0].name)
        );
    }
}
