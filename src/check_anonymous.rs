use super::*;

fn unsupported(offset: usize, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "ANONYMOUS_OBJECT_SHAPE_UNSUPPORTED",
    )
}

impl Checker<'_> {
    pub(super) fn anonymous_object(
        &mut self,
        interface: &Type,
        methods: &[syntax::Function],
        offset: usize,
    ) -> Result<Value> {
        let interface = self.resolve(interface, offset)?;
        let Type::Java(interface_owner) = &interface else {
            return Err(unsupported(
                offset,
                "Anonymous objects require a nongeneric interface",
            ));
        };
        let class = self
            .classes
            .get(interface_owner)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.classpath.load(interface_owner))?;
        if class.sealed_ {
            return Err(Diagnostic::new(
                offset,
                "Anonymous objects cannot implement sealed interfaces",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "SEALED_ANONYMOUS_SUBCLASS_INVALID",
            ));
        }
        if class.access & 0x0200 == 0 || class.access & 0x2000 != 0 {
            return Err(unsupported(
                offset,
                "Anonymous class and annotation supertypes are unsupported",
            ));
        }
        if class
            .library
            .as_ref()
            .is_some_and(|library| !library.type_parameters.is_empty())
        {
            return Err(unsupported(
                offset,
                "Generic anonymous interface supertypes are unsupported",
            ));
        }
        let inherited = self
            .classpath
            .methods(interface_owner)?
            .into_iter()
            .filter(|method| method.access & 0x0009 == 0x0001 && method.name != "<init>")
            .collect::<Vec<_>>();
        if inherited.iter().any(|method| {
            method.source_property.is_some()
                || method
                    .library
                    .as_ref()
                    .is_some_and(|library| !library.type_parameters.is_empty())
        }) {
            return Err(unsupported(
                offset,
                "Anonymous interface properties and generic methods are unsupported",
            ));
        }
        if methods.is_empty() {
            return Err(unsupported(
                offset,
                "Anonymous marker implementations are unsupported",
            ));
        }
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
            let candidates = inherited
                .iter()
                .filter(|method| method.source_name() == source.name)
                .filter(|method| {
                    class_method_types(method).is_ok_and(|(types, _)| {
                        types == params.iter().map(|p| p.1.clone()).collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>();
            if candidates.len() != 1 {
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
            if !seen.insert((method.name.clone(), method.descriptor.clone())) {
                return Err(
                    Diagnostic::new(source.offset, "Duplicate anonymous override").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ANONYMOUS_OVERRIDE_DUPLICATE",
                    ),
                );
            }
            let (_, result) = class_method_types(&method)?;
            if !source.result_inferred {
                let declared_result = self.resolve(&source.result, source.offset)?;
                if declared_result != result {
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
            }
            if format!(
                "({}){}",
                params.iter().map(|p| p.1.descriptor()).collect::<String>(),
                result.descriptor()
            ) != method.descriptor
            {
                return Err(unsupported(
                    source.offset,
                    "Anonymous override requiring erased bridges is unsupported",
                ));
            }
            selected.push((source, params, result, method));
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
        let owner = format!(
            "{}$object${}${}",
            self.lambda_owner,
            offset,
            self.lambdas.borrow().len()
        );
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
        if names
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
        let mut fields = Vec::new();
        let mut values = Vec::new();
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
        if let Ok(value) = self.dispatch_receiver_value(offset) {
            receivers.push((true, self.dispatch_receiver.clone(), value));
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
            let labels = local
                .as_ref()
                .map(|local| {
                    self.receiver_labels
                        .iter()
                        .filter(|(_, other)| local.slot == other.slot && local.ty == other.ty)
                        .map(|(name, _)| name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            receiver_fields.push((dispatch, name.clone(), value.ty.clone(), labels));
            fields.push((name, value.ty.clone()));
            values.push(value);
        }
        let mut implementations = Vec::new();
        for (source, params, result, method) in selected {
            let mut checker = Checker {
                inline_argument_modes: Vec::new(),
                inline_analysis: false,
                public_inline_body: self.public_inline_body,
                constructor_bridge_context: true,
                reified_parameters: Vec::new(),
                exhaustiveness_flow: false,
                extension_receivers: vec![Local {
                    slot: 0,
                    ty: interface.clone(),
                    mutable: false,
                }],
                receiver_labels: HashMap::new(),
                lambdas: self.lambdas.clone(),
                captured: captured.clone(),
                captured_cells: captured_cells.clone(),
                cells: HashMap::new(),
                cell_names: HashSet::new(),
                lambda_owner: self.lambda_owner.clone(),
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
                dispatch_receiver: None,
                fields: self.fields,
                members: self.members,
                companion_fields: self.companion_fields,
                functions: self.functions,
                object_methods: self.object_methods,
                object_fields: self.object_fields,
                object_owner: self.object_owner,
                allow_return: true,
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
                if *dispatch {
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
                if inferred.ty != result {
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
            implementations.push(Function {
                suspend_: false,
                extension_receiver: false,
                class_type_params: Vec::new(),
                jvm_result: None,
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
        let mut first = implementations.remove(0);
        let mut interface_method = inherited
            .iter()
            .find(|method| {
                method.name == first.jvm_name() && method.descriptor == first.descriptor()
            })
            .unwrap()
            .clone();
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
        interface_method.owner = interface_owner.clone();
        first.is_override = true;
        self.lambdas.borrow_mut().push(LambdaDecl {
            anonymous_methods: Some(implementations),
            defaults,
            owner: owner.clone(),
            captures: fields,
            function: first,
            interface: Some(interface_method),
        });
        Ok(Value {
            ty: interface,
            kind: ValueKind::Lambda {
                owner,
                captures: values,
            },
        })
    }
}
