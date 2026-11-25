use super::*;

fn unsupported(offset: usize, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "ANONYMOUS_OBJECT_SHAPE_UNSUPPORTED",
    )
}

pub(super) fn check_exposed_types(source: &syntax::Module, classpath: &Classpath) -> Result<()> {
    fn contains_anonymous(ty: &Type, classpath: &Classpath) -> bool {
        if let Some(owner) = library::type_owner(ty) {
            if classpath.is_source(owner)
                && classpath
                    .load(owner)
                    .is_ok_and(|class| class.access & 0x1000 != 0 && !class.kotlin_class)
            {
                return true;
            }
        }
        match ty {
            Type::Nullable(inner)
            | Type::Platform(inner)
            | Type::List(inner)
            | Type::MutableList(inner)
            | Type::ThreadLocal(inner)
            | Type::Array(inner) => contains_anonymous(inner, classpath),
            Type::Parameterized { arguments, .. } => arguments
                .iter()
                .filter_map(|arg| arg.ty.as_ref())
                .any(|ty| contains_anonymous(ty, classpath)),
            Type::Function { params, result } => params
                .iter()
                .chain(std::iter::once(result.as_ref()))
                .any(|ty| contains_anonymous(ty, classpath)),
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => params
                .iter()
                .chain([receiver.as_ref(), result.as_ref()])
                .any(|ty| contains_anonymous(ty, classpath)),
            _ => false,
        }
    }
    let check = |ty: &Type, private: bool, offset: usize| {
        if private || !contains_anonymous(ty, classpath) {
            return Ok(());
        }
        if matches!(ty, Type::Java(_)) {
            Err(Diagnostic::new(
                offset,
                "Public anonymous objects with multiple supertypes require an explicit result type",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ANONYMOUS_PUBLIC_RESULT_TYPE_REQUIRED",
            ))
        } else {
            Err(unsupported(
                offset,
                "Anonymous types nested in public inferred signatures require unsupported type approximation",
            ))
        }
    };
    let function =
        |function: &syntax::Function| check(&function.result, function.private, function.offset);
    for function_source in &source.functions {
        function(function_source)?;
    }
    for property in &source.properties {
        check(&property.ty, property.private, property.offset)?;
    }
    if let Some(class) = &source.class_decl {
        for method in &class.methods {
            function(&method.function)?;
        }
        for property in &class.properties {
            check(&property.ty, property.private, property.offset)?;
        }
        if let Some(companion) = &class.companion {
            for method in &companion.methods {
                function(&method.function)?;
            }
            for property in &companion.properties {
                if let Some(ty) = &property.ty {
                    check(ty, property.private, property.offset)?;
                }
            }
        }
    }
    if let Some(object) = &source.object_decl {
        for method in &object.methods {
            function(&method.function)?;
        }
        for field in &object.fields {
            if let Some(ty) = &field.ty {
                check(ty, field.private, field.offset)?;
            }
        }
    }
    if let Some(interface) = &source.interface_decl {
        for method in &interface.methods {
            function(&method.function)?;
        }
        if let Some(companion) = &interface.companion {
            for method in &companion.methods {
                function(&method.function)?;
            }
            for field in &companion.fields {
                if let Some(ty) = &field.ty {
                    check(ty, field.private, field.offset)?;
                }
            }
        }
    }
    Ok(())
}

fn enclosing_parameter(ty: &Type) -> bool {
    match ty {
        Type::Parameter { .. } => true,
        Type::Parameterized { arguments, .. } => arguments
            .iter()
            .filter_map(|argument| argument.ty.as_ref())
            .any(enclosing_parameter),
        Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::Array(inner)
        | Type::OutArray(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner) => enclosing_parameter(inner),
        Type::Function { params, result } | Type::SuspendFunction { params, result } => params
            .iter()
            .chain(std::iter::once(result.as_ref()))
            .any(enclosing_parameter),
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => params
            .iter()
            .chain([receiver.as_ref(), result.as_ref()])
            .any(enclosing_parameter),
        _ => false,
    }
}

fn override_type(actual: &Type, expected: &Type) -> bool {
    actual == expected
        || matches!(expected, Type::Platform(inner) if actual.base() == inner.base() || matches!(actual, Type::Nullable(nullable) if nullable.base() == inner.base()))
}

impl Checker<'_> {
    pub(super) fn anonymous_object(
        &mut self,
        supertypes: &[(Type, Option<Vec<Expr>>)],
        methods: &[syntax::Function],
        offset: usize,
    ) -> Result<Value> {
        let owner = format!("{}$object${offset}", self.lambda_owner);
        let mut parent = "java/lang/Object".to_string();
        let mut interfaces = Vec::new();
        let mut super_arguments = Vec::new();
        let mut super_descriptor = "()V".to_string();
        let mut inherited = Vec::new();
        let mut resolved_supertypes = Vec::new();
        let mut conflicts = HashSet::new();
        let mut has_parent = false;
        let mut primary = None;
        for (supertype, arguments) in supertypes {
            let ty = self.resolve(supertype, offset)?;
            let (Type::Java(super_owner)
            | Type::Parameterized {
                name: super_owner, ..
            }) = &ty
            else {
                return Err(unsupported(
                    offset,
                    "Anonymous supertype must be a class or interface",
                ));
            };
            if enclosing_parameter(&ty) {
                return Err(unsupported(
                    offset,
                    "Anonymous supertype signatures referencing enclosing type parameters require unsupported enclosing-method metadata",
                ));
            }
            let class = self.classpath.load(super_owner)?;
            if class.sealed_ {
                return Err(Diagnostic::new(
                    offset,
                    "Anonymous objects cannot inherit sealed classifiers",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "SEALED_ANONYMOUS_SUBCLASS_INVALID",
                ));
            }
            if class.access & 0x2000 != 0 {
                return Err(unsupported(
                    offset,
                    "Annotation anonymous supertypes are unsupported",
                ));
            }
            if let Type::Parameterized { arguments, .. } = &ty {
                if arguments.iter().any(|argument| {
                    argument.variance != syntax::Variance::Invariant || argument.ty.is_none()
                }) {
                    return Err(Diagnostic::new(
                        offset,
                        "Anonymous supertype arguments cannot be projected",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ANONYMOUS_SUPERTYPE_PROJECTION_INVALID",
                    ));
                }
            }
            resolved_supertypes.push(ty.clone());
            if !self
                .classpath
                .classifier_visible(&self.lambda_owner, super_owner)
            {
                return Err(
                    Diagnostic::new(offset, "Anonymous supertype is inaccessible").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "INACCESSIBLE_CLASSIFIER",
                    ),
                );
            }
            if class.access & 0x0200 != 0 {
                if arguments.is_some() || interfaces.iter().any(|owner| owner == super_owner) {
                    return Err(
                        Diagnostic::new(offset, "Invalid anonymous interface supertype").with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "ANONYMOUS_SUPERTYPE_INVALID",
                        ),
                    );
                }
                interfaces.push(super_owner.to_string());
            } else {
                if has_parent || arguments.is_none() || class.access & 0x0010 != 0 {
                    return Err(Diagnostic::new(
                        offset,
                        "Anonymous class requires one nonfinal superclass constructor",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ANONYMOUS_SUPERTYPE_INVALID",
                    ));
                }
                has_parent = true;
                parent = super_owner.to_string();
                let constructors = class
                    .methods
                    .iter()
                    .filter(|m| m.name == "<init>" && m.access & 5 != 0)
                    .cloned()
                    .map(|method| library::specialize_receiver(method, &ty, &class, self.classpath))
                    .collect::<Vec<_>>();
                let arguments = arguments.as_ref().unwrap();
                let method = self.select_method(&constructors, arguments, offset)?;
                let (types, _) = class_method_types(&method)?;
                let params = method
                    .kotlin_params
                    .as_ref()
                    .map(|params| {
                        params
                            .iter()
                            .map(|p| (p.0.clone(), p.1.clone()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| {
                        types
                            .iter()
                            .cloned()
                            .enumerate()
                            .map(|(i, t)| (format!("p{i}"), java_parameter(t)))
                            .collect()
                    });
                if arguments.len() < params.len()
                    && method
                        .kotlin_params
                        .as_ref()
                        .is_some_and(|params| params.iter().any(|p| p.2))
                {
                    return Err(unsupported(
                        offset,
                        "Default anonymous superclass arguments are unsupported",
                    ));
                }
                let (args, lets, mask) = self.arguments(
                    &params,
                    &vec![false; params.len()],
                    arguments,
                    method.kotlin_params.is_some(),
                    offset,
                )?;
                if !lets.is_empty() || mask != 0 {
                    return Err(unsupported(
                        offset,
                        "Reordered and default anonymous superclass arguments are unsupported",
                    ));
                }
                super_arguments = args
                    .into_iter()
                    .zip(crate::jvm_descriptor::method_types(&method.descriptor)?.0)
                    .map(|(value, ty)| self.coerce(value, java_parameter(ty), offset))
                    .collect::<Result<Vec<_>>>()?;
                super_descriptor = method.descriptor;
            }
            primary.get_or_insert(ty.clone());
            for method in self
                .classpath
                .ordinary_methods(super_owner)?
                .into_iter()
                .map(|method| library::specialize_receiver(method, &ty, &class, self.classpath))
                .filter(|method| {
                    method.access & 0x0048 == 0 && method.access & 5 != 0 && method.name != "<init>"
                })
            {
                if let Some(other) = inherited.iter().find(|other: &&JavaMethod| {
                    other.name == method.name && other.descriptor == method.descriptor
                }) {
                    let concrete = |method: &JavaMethod| {
                        method.access & 0x0400 == 0
                            || method
                                .library
                                .as_ref()
                                .is_some_and(|library| !library.abstract_)
                    };
                    if class_method_types(other)?.0 != class_method_types(&method)?.0
                        || class_method_types(other)?.1 != class_method_types(&method)?.1
                    {
                        if other.owner != method.owner {
                            return Err(unsupported(
                                offset,
                                "Anonymous inherited signatures requiring covariant or platform adaptation are unsupported",
                            ));
                        }
                        return Err(Diagnostic::new(
                            offset,
                            "Anonymous inherited generic specializations conflict",
                        )
                        .with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "ANONYMOUS_GENERIC_SUPERTYPE_CONFLICT",
                        ));
                    }
                    if other.owner != method.owner && (concrete(other) || concrete(&method)) {
                        conflicts.insert((method.name.clone(), method.descriptor.clone()));
                    }
                } else {
                    inherited.push(method);
                }
            }
        }
        if inherited.iter().any(|method| {
            method.access & 0x0400 != 0
                && (method.source_property.is_some()
                    || method
                        .library
                        .as_ref()
                        .is_some_and(|library| !library.type_parameters.is_empty()))
        }) {
            return Err(unsupported(
                offset,
                "Anonymous inherited properties and generic methods are unsupported",
            ));
        }
        self.classpath.register_source(crate::classpath::JavaClass {
            name: owner.clone(),
            access: 0x1031,
            super_name: Some(parent.clone()),
            interfaces: interfaces.clone(),
            methods: Vec::new(),
            fields: Vec::new(),
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            kotlin_fun_interface: false,
            library: Some(crate::classpath::LibraryClass {
                value_class: None,
                companion_name: None,
                kotlin_visibility: None,
                type_aliases: Vec::new(),
                type_parameters: Vec::new(),
                supertypes: resolved_supertypes.clone(),
                package: None,
                multifile_facade: None,
            }),
            kotlin_class: false,
            unsupported: false,
            kotlin_interface: false,
        });
        self.classpath
            .register_lexical_parent(&owner, &self.lambda_owner);
        let interface = primary.unwrap();
        let mut selected = Vec::new();
        let mut seen = HashSet::new();
        for source in methods {
            if source.suspend_
                || source.inline
                || source.vararg
                || source.receiver.is_some()
                || !source.type_params.is_empty()
                || !source.annotations.is_empty()
                || source
                    .param_annotations
                    .iter()
                    .any(|annotations| !annotations.is_empty())
                || source.defaults.iter().any(Option::is_some)
            {
                return Err(unsupported(
                    source.offset,
                    "Anonymous override modifiers, generics, annotations and defaults are unsupported",
                ));
            }
            let params = source
                .params
                .iter()
                .map(|(name, ty)| Ok((name.clone(), self.resolve(ty, source.offset)?)))
                .collect::<Result<Vec<_>>>()?;
            if params.iter().any(|(_, ty)| ty.base() == &Type::Unit) {
                return Err(unsupported(
                    source.offset,
                    "Anonymous Unit-valued parameters are unsupported",
                ));
            }
            let candidates = inherited
                .iter()
                .filter(|method| method.source_name() == source.name)
                .filter(|method| {
                    class_method_types(method).is_ok_and(|(types, _)| {
                        types.len() == params.len()
                            && types
                                .iter()
                                .zip(&params)
                                .all(|(expected, (_, actual))| override_type(actual, expected))
                    })
                })
                .collect::<Vec<_>>();
            if candidates.is_empty()
                || candidates.iter().any(|method| {
                    method.name != candidates[0].name
                        || class_method_types(method).ok() != class_method_types(candidates[0]).ok()
                })
            {
                let prefix = format!(
                    "({})",
                    params.iter().map(|p| p.1.descriptor()).collect::<String>()
                );
                if inherited.iter().any(|method| {
                    method.source_name() == source.name && method.descriptor.starts_with(&prefix)
                }) {
                    return Err(unsupported(
                        source.offset,
                        "Anonymous platform/nullability signature adaptation is unsupported",
                    ));
                }

                return Err(Diagnostic::new(
                    source.offset,
                    "Anonymous override requires an exact inherited method signature",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "ANONYMOUS_OVERRIDE_MISMATCH",
                ));
            }
            let method = candidates[0].clone();
            if candidates.iter().any(|method| method.access & 0x0010 != 0) {
                return Err(
                    Diagnostic::new(source.offset, "Cannot override a final member").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "FINAL_MEMBER_OVERRIDE",
                    ),
                );
            }
            if candidates
                .iter()
                .any(|method| seen.contains(&(method.name.clone(), method.descriptor.clone())))
            {
                return Err(
                    Diagnostic::new(source.offset, "Duplicate anonymous override").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ANONYMOUS_OVERRIDE_DUPLICATE",
                    ),
                );
            }
            for method in &candidates {
                seen.insert((method.name.clone(), method.descriptor.clone()));
            }
            let overrides = candidates
                .iter()
                .map(|method| (*method).clone())
                .collect::<Vec<_>>();
            let (_, mut result) = class_method_types(&method)?;
            if !source.result_inferred {
                let declared_result = self.resolve(&source.result, source.offset)?;
                if !override_type(&declared_result, &result) {
                    if method.kotlin_result.is_none()
                        && declared_result.descriptor() == result.descriptor()
                    {
                        return Err(unsupported(
                            source.offset,
                            "Anonymous Java platform result adaptation is unsupported",
                        ));
                    }
                    if self.subtype(&declared_result, &result)
                        || result == Type::Java("java/lang/Object".into())
                    {
                        return Err(unsupported(
                            source.offset,
                            "Covariant anonymous override results are unsupported",
                        ));
                    }
                    return Err(Diagnostic::new(
                        source.offset,
                        "Anonymous override result differs from inherited method",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ANONYMOUS_OVERRIDE_RESULT_MISMATCH",
                    ));
                }
                result = declared_result;
            }
            if result == Type::Unit
                && crate::jvm_descriptor::method_types(&method.descriptor)?
                    .1
                    .is_reference()
            {
                return Err(unsupported(
                    source.offset,
                    "Anonymous generic Unit results require unsupported boxed Unit overrides",
                ));
            }
            selected.push((source, params, result, method, overrides));
        }
        if conflicts.iter().any(|signature| !seen.contains(signature)) {
            return Err(unsupported(
                offset,
                "Conflicting anonymous inherited defaults require an explicit override",
            ));
        }
        for required in &inherited {
            let abstract_ = required.access & 0x0400 != 0
                && required
                    .library
                    .as_ref()
                    .is_none_or(|library| library.abstract_);
            if abstract_ && !seen.contains(&(required.name.clone(), required.descriptor.clone())) {
                return Err(Diagnostic::new(
                    offset,
                    "Anonymous object does not implement every abstract method",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "ANONYMOUS_ABSTRACT_MEMBER_MISSING",
                ));
            }
        }
        let mut names = HashSet::new();
        for source in methods {
            let parameters = source
                .params
                .iter()
                .map(|p| p.0.clone())
                .chain(std::iter::once("this".into()))
                .collect::<Vec<_>>();
            names.extend(captures::free_names(&source.body, &parameters));
        }
        if parent == "java/lang/Object"
            && names
                .iter()
                .any(|name| name == "super" || name.starts_with("super@"))
        {
            return Err(unsupported(
                offset,
                "Anonymous interface super calls are unsupported",
            ));
        }
        names.retain(|name| {
            !name.starts_with("this@")
                && (self.local(name).is_some() || self.captured.contains_key(name))
        });
        let mut names = names.into_iter().collect::<Vec<_>>();
        names.sort();
        let mut fields = super_arguments
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("$super${index}"), value.ty.clone()))
            .collect::<Vec<_>>();
        let super_types = super_arguments
            .iter()
            .map(|value| value.ty.clone())
            .collect();
        let mut values = super_arguments;
        let mut captured = HashMap::new();
        let mut captured_cells = HashMap::new();
        let mut readonly = Vec::new();
        for name in names {
            if self
                .local(&name)
                .is_some_and(|local| !self.initialized.contains(&local.slot))
            {
                return Err(Diagnostic::new(
                    offset,
                    "Variable must be initialized before anonymous capture",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "ANONYMOUS_CAPTURE_UNINITIALIZED",
                ));
            }
            let value = if let Some((cell, ty)) = self.cell_binding(&name) {
                if self.cell_is_readonly(&name) {
                    readonly.push(name.clone());
                }
                captured_cells.insert(name.clone(), ty);
                cell
            } else {
                if self.local(&name).is_some_and(|local| local.mutable) {
                    return Err(unsupported(
                        offset,
                        "Mutable anonymous capture requires an enclosing initialized local",
                    ));
                }
                self.value(&Expr {
                    offset,
                    kind: ExprKind::Name(name.clone()),
                })?
            };
            captured.insert(name.clone(), (owner.clone(), value.ty.clone()));
            fields.push((name, value.ty.clone()));
            values.push(value);
        }
        let mut receivers = self
            .extension_receivers
            .iter()
            .cloned()
            .map(|local| (false, Some(local.clone()), Self::local_receiver(&local)))
            .collect::<Vec<_>>();
        if self.dispatch_receiver.is_some()
            || self
                .object_owner
                .is_some_and(|owner| !self.classpath.is_source_facade(owner))
        {
            if let Ok(value) = self.dispatch_receiver_value(offset) {
                receivers.push((true, self.dispatch_receiver.clone(), value));
            }
        }
        let mut receiver_fields = Vec::new();
        for (index, (dispatch, local, value)) in receivers.into_iter().enumerate() {
            let mut ordinal = index;
            let name = loop {
                let candidate = format!("$outer${ordinal}");
                if !fields.iter().any(|(name, _)| name == &candidate) {
                    break candidate;
                }
                ordinal += 1;
            };
            let mut labels = local
                .as_ref()
                .map(|local| {
                    self.receiver_labels
                        .iter()
                        .filter(|(_, other)| local.slot == other.slot && local.ty == other.ty)
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if dispatch {
                if let Some(label) =
                    library::type_owner(&value.ty).and_then(|owner| owner.rsplit(['/', '$']).next())
                {
                    labels.push(label.to_string());
                }
            }
            receiver_fields.push((dispatch, name.clone(), value.ty.clone(), labels));
            fields.push((name, value.ty.clone()));
            values.push(value);
        }
        let mut implementations = Vec::new();
        let mut bridges = Vec::new();
        for (source, params, result, method, overrides) in selected {
            let mut checker = Checker {
                inline_argument_modes: Vec::new(),
                inline_analysis: false,
                public_inline_body: self.public_inline_body,
                constructor_bridge_context: true,
                reified_parameters: Vec::new(),
                exhaustiveness_flow: false,
                extension_receivers: vec![Local {
                    slot: 0,
                    ty: Type::Java(owner.clone()),
                    mutable: false,
                }],
                receiver_labels: HashMap::new(),
                lambdas: self.lambdas.clone(),
                captured: captured.clone(),
                captured_cells: captured_cells.clone(),
                cells: HashMap::new(),
                cell_names: HashSet::new(),
                lambda_owner: owner.clone(),
                inline_result: None,
                nonlocal_result: None,
                classpath: self.classpath,
                constants: self.constants,
                constructor: self.constructor,
                defaults: self.defaults,
                function_names: self.function_names,
                mutable_fields: self.mutable_fields,
                extensions: self.extensions,
                classes: self.classes,
                imports: self.imports,
                signatures: self.signatures,
                source_type_params: self.source_type_params,
                source_infix: self.source_infix,
                source_operator: self.source_operator,
                dispatch_receiver: if parent != "java/lang/Object" {
                    Some(Local {
                        slot: 0,
                        ty: Type::Java(owner.clone()),
                        mutable: false,
                    })
                } else {
                    None
                },
                fields: self.fields,
                members: self.members,
                companion_fields: self.companion_fields,
                functions: self.functions,
                object_methods: self.object_methods,
                object_fields: self.object_fields,
                object_owner: self.object_owner,
                allow_return: true,
                contract_facts: HashMap::new(),
                contract_registration: None,
                refinements: HashMap::new(),
                initialized: HashSet::from([0]),
                loop_depth: 0,
                scopes: vec![HashMap::new()],
                next: 1,
                result: result.clone(),
            };
            for name in &readonly {
                checker.scopes[0].insert(
                    format!("<readonlyCell:{name}>"),
                    Local {
                        slot: 0,
                        ty: Type::Unit,
                        mutable: false,
                    },
                );
            }
            for (name, ty) in &params {
                checker.declare(name, ty.clone(), false, source.offset)?;
            }
            let mut setup = Vec::new();
            for (dispatch, name, ty, labels) in &receiver_fields {
                let slot = checker.allocate_typed_slot(ty, offset)?;
                checker.initialized.insert(slot);
                let local = Local {
                    slot,
                    ty: ty.clone(),
                    mutable: false,
                };
                if *dispatch && parent == "java/lang/Object" {
                    checker.dispatch_receiver = Some(local.clone());
                } else {
                    checker.extension_receivers.push(local.clone());
                }
                for label in labels {
                    checker.receiver_labels.insert(label.clone(), local.clone());
                }
                setup.push(Statement::Store(
                    slot,
                    Value {
                        ty: ty.clone(),
                        kind: ValueKind::JavaField {
                            descriptor: None,
                            owner: owner.clone(),
                            name: name.clone(),
                            receiver: Some(Box::new(Value {
                                ty: Type::Java(owner.clone()),
                                kind: ValueKind::Local(0),
                            })),
                        },
                    },
                ));
            }
            if source.result_inferred {
                let [
                    syntax::Statement {
                        kind: StatementKind::Return(Some(expression)),
                        ..
                    },
                ] = source.body.as_slice()
                else {
                    return Err(unsupported(
                        source.offset,
                        "Anonymous inferred result shape is unsupported",
                    ));
                };
                if control::expression_terminates(expression) {
                    return Err(unsupported(
                        source.offset,
                        "Anonymous Nothing result inference is unsupported",
                    ));
                }
                let mut probe = checker.clone();
                let saved_count = checker.lambdas.borrow().len();
                let inferred = probe.value(expression);
                checker.lambdas.borrow_mut().truncate(saved_count);
                let inferred = inferred?;
                if !override_type(&inferred.ty, &result) {
                    if checker.subtype(&inferred.ty, &result)
                        || result == Type::Java("java/lang/Object".into())
                    {
                        return Err(unsupported(
                            source.offset,
                            "Covariant anonymous inferred results are unsupported",
                        ));
                    }
                    return Err(Diagnostic::new(
                        source.offset,
                        "Anonymous inferred result differs from inherited method",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ANONYMOUS_OVERRIDE_RESULT_MISMATCH",
                    ));
                }
            }
            let (mut body, returns) = checker.block(&source.body)?;
            if !returns {
                if result != Type::Unit {
                    return Err(Diagnostic::new(
                        source.offset,
                        "Anonymous method must return a value",
                    ));
                }
                body.push(Statement::Return(Value {
                    ty: Type::Unit,
                    kind: ValueKind::Unit,
                }));
            }
            setup.append(&mut body);
            let erased_result = crate::jvm_descriptor::method_types(&method.descriptor)?.1;
            let jvm_result =
                if !result.is_reference() && result != Type::Unit && erased_result.is_reference() {
                    let boxed = Type::Nullable(Box::new(result.clone())).descriptor();
                    Some(Type::Java(
                        boxed.trim_start_matches('L').trim_end_matches(';').into(),
                    ))
                } else {
                    None
                };
            if let Some(storage) = &jvm_result {
                struct Returns(Type);
                impl ir_walk::Visitor for Returns {
                    fn value(&mut self, _: &mut Value, _: &str) {}
                    fn statement(&mut self, statement: &mut Statement, _: &str) {
                        if let Statement::Return(value) = statement {
                            if !value.ty.is_reference() && value.ty != Type::Unit {
                                *value = Value {
                                    ty: self.0.clone(),
                                    kind: ValueKind::Box(Box::new(value.clone())),
                                };
                            }
                        }
                    }
                }
                ir_walk::statements(&mut setup, &owner, &mut Returns(storage.clone()));
            }
            let physical_result = jvm_result.as_ref().unwrap_or(&result);
            let descriptor = format!(
                "({}){}",
                params.iter().map(|p| p.1.descriptor()).collect::<String>(),
                physical_result.descriptor()
            );
            for inherited in overrides {
                if descriptor != inherited.descriptor {
                    bridges.push(GenericBridge {
                        erased: inherited,
                        target_params: params.iter().map(|p| p.1.clone()).collect(),
                        target_result: physical_result.clone(),
                    });
                }
            }
            implementations.push(Function {
                suspend_: false,
                inline: false,
                extension_receiver: false,
                class_type_params: Vec::new(),
                jvm_result,
                jvm_signature: None,
                jvm_name: Some(method.name),
                annotations: Vec::new(),
                vararg: false,
                type_params: Vec::new(),
                defaults: params.iter().map(|_| None).collect(),
                name: source.name.clone(),
                private: false,
                is_override: true,
                params,
                result,
                locals: checker.next,
                body: setup,
            });
        }
        let first = implementations.first().cloned().unwrap_or(Function {
            suspend_: false,
            inline: false,
            extension_receiver: false,
            class_type_params: Vec::new(),
            jvm_result: None,
            jvm_signature: None,
            jvm_name: None,
            annotations: Vec::new(),
            vararg: false,
            type_params: Vec::new(),
            defaults: Vec::new(),
            name: "<anonymous>".into(),
            private: false,
            is_override: false,
            params: Vec::new(),
            result: Type::Unit,
            locals: 1,
            body: Vec::new(),
        });
        let defaults = inherited
            .into_iter()
            .filter(|method| {
                !seen.contains(&(method.name.clone(), method.descriptor.clone()))
                    && method.access & 0x0400 != 0
                    && method
                        .library
                        .as_ref()
                        .is_some_and(|library| !library.abstract_)
            })
            .collect();
        self.lambdas.borrow_mut().push(LambdaDecl {
            anonymous_methods: Some(implementations),
            anonymous_super: Some(AnonymousSuper {
                supertypes: resolved_supertypes,
                bridges,
                owner: parent,
                descriptor: super_descriptor,
                params: super_types,
                interfaces,
            }),
            defaults,
            owner: owner.clone(),
            captures: fields,
            function: first,
            interface: None,
        });
        Ok(Value {
            ty: if supertypes.len() == 1 {
                interface
            } else {
                Type::Java(owner.clone())
            },
            kind: ValueKind::Lambda {
                owner,
                captures: values,
            },
        })
    }
}
