use super::operations::ExtensionKind;
use super::*;

pub(super) fn call_labels(name: &str, arguments: &[Expr]) -> Vec<Expr> {
    arguments
        .iter()
        .map(|argument| {
            if let ExprKind::NamedArgument(parameter, value) = &argument.kind {
                return Expr {
                    offset: argument.offset,
                    kind: ExprKind::NamedArgument(
                        parameter.clone(),
                        Box::new(call_labels(name, std::slice::from_ref(value.as_ref())).remove(0)),
                    ),
                };
            }
            if matches!(
                argument.kind,
                ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
            ) {
                Expr {
                    offset: argument.offset,
                    kind: ExprKind::Labeled(name.into(), Box::new(argument.clone())),
                }
            } else {
                argument.clone()
            }
        })
        .collect()
}

pub(super) fn labeled_call(expr: &Expr) -> Option<Expr> {
    fn name(target: &Expr) -> Option<&str> {
        match &target.kind {
            ExprKind::Name(name) | ExprKind::Property(_, name) => Some(name),
            ExprKind::Generic(target, _) => name(target),
            _ => None,
        }
    }
    fn unlabeled(argument: &Expr) -> bool {
        match &argument.kind {
            ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. } => true,
            ExprKind::NamedArgument(_, value) => unlabeled(value),
            _ => false,
        }
    }
    let ExprKind::Call(target, arguments) = &expr.kind else {
        return None;
    };
    if !arguments.iter().any(unlabeled) {
        return None;
    }
    let name = name(target)?;
    Some(Expr {
        offset: expr.offset,
        kind: ExprKind::Call(target.clone(), call_labels(name, arguments)),
    })
}

impl Checker<'_> {
    pub(super) fn select_extension_accessor(
        &mut self,
        methods: &[JavaMethod],
        arguments: &[Expr],
        explicit: &[Type],
        setter: bool,
        offset: usize,
    ) -> Result<JavaMethod> {
        let getters = methods
            .iter()
            .filter(|method| {
                method
                    .kotlin_params
                    .as_ref()
                    .is_some_and(|params| params.len() == 1)
            })
            .cloned()
            .collect::<Vec<_>>();
        let getter = self.select_method_types(&getters, &arguments[..1], explicit, offset)?;
        if !setter {
            if getter.access & 8 != 0 {
                return methods
                    .iter()
                    .find(|method| {
                        method.owner == getter.owner
                            && method.name == getter.name
                            && method.descriptor == getter.descriptor
                    })
                    .cloned()
                    .ok_or_else(|| {
                        Diagnostic::new(
                            offset,
                            "Selected extension getter has no declaration candidate",
                        )
                    });
            }
            return Ok(getter);
        }
        let generic_setter = getter
            .library
            .as_ref()
            .is_some_and(|library| !library.type_parameters.is_empty())
            && methods
                .iter()
                .find(|method| {
                    method.owner == getter.owner
                        && method.name == getter.name
                        && method.descriptor == getter.descriptor
                })
                .is_some_and(|declaration| {
                    methods.iter().any(|method| {
                        method.source_property == declaration.source_property
                            && method.kotlin_params.as_ref().is_some_and(|params| {
                                params.len() == 2
                                    && Some(&params[0])
                                        == declaration
                                            .kotlin_params
                                            .as_ref()
                                            .and_then(|params| params.first())
                                    && Some(&params[1].1) == declaration.kotlin_result.as_ref()
                            })
                            && method.kotlin_result == Some(Type::Unit)
                    })
                });
        if generic_setter {
            return Err(Diagnostic::new(
                offset,
                "Generic extension property setters require unsupported type substitution",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "GENERIC_EXTENSION_SETTER_UNSUPPORTED",
            ));
        }
        let setters = methods
            .iter()
            .filter(|method| {
                method.kotlin_params.as_ref().is_some_and(|params| {
                    params.len() == 2
                        && Some(&params[0])
                            == getter
                                .kotlin_params
                                .as_ref()
                                .and_then(|params| params.first())
                        && method.kotlin_result == Some(Type::Unit)
                        && Some(&params[1].1) == getter.kotlin_result.as_ref()
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if setters.is_empty() {
            return Err(
                Diagnostic::new(offset, "Extension property has no accessible setter").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "EXTENSION_PROPERTY_SETTER_UNAVAILABLE",
                ),
            );
        }
        self.select_method_types(&setters, arguments, explicit, offset)
    }
    pub(super) fn extension_property_set(
        &mut self,
        receiver: Value,
        name: &str,
        expr: &Expr,
        offset: usize,
    ) -> Result<Option<Value>> {
        if let Some(owner) = library::type_owner(&receiver.ty) {
            let class = self
                .classpath
                .load(owner)
                .or_else(|error| self.classes.get(owner).cloned().ok_or(error))?;
            let fields = self
                .classpath
                .field_candidates(owner, name)
                .unwrap_or_else(|_| {
                    class
                        .fields
                        .iter()
                        .filter(|field| field.name == name)
                        .cloned()
                        .collect()
                });
            let methods = self
                .classpath
                .methods(owner)
                .unwrap_or_else(|_| class.methods.clone());
            if fields.iter().any(|field| field.access & 8 == 0)
                || methods.iter().any(|method| {
                    method.source_property.as_deref() == Some(name)
                        && method
                            .library
                            .as_ref()
                            .is_none_or(|library| library.receiver.is_none())
                })
            {
                return Ok(None);
            }
        }
        if let Some(value) = self.instance_extension_call(
            receiver.clone(),
            name,
            std::slice::from_ref(expr),
            &[],
            &ExtensionKind::PropertySet,
            offset,
        )? {
            return Ok(Some(value));
        }
        if !self.extension_candidates(name, true)?.is_empty() {
            return self
                .resolved_extension_call(
                    receiver,
                    name,
                    std::slice::from_ref(expr),
                    &[],
                    ExtensionKind::PropertySet,
                    offset,
                )
                .map(Some);
        }
        Ok(None)
    }

    pub(super) fn implicit_receiver_assignment(
        &mut self,
        name: &str,
        expr: &Expr,
        offset: usize,
    ) -> Result<Option<Statement>> {
        for receiver in self.extension_receivers.clone() {
            let next = self.next;
            let initialized = self.initialized.clone();
            let refinements = self.refinements.clone();
            let lambdas = self.lambdas.borrow().len();
            let local_name = format!("this@<implicit{}>", receiver.slot);
            self.scopes
                .push(HashMap::from([(local_name.clone(), receiver)]));
            let result = self.field_assignment(
                &Expr {
                    offset,
                    kind: ExprKind::Name(local_name),
                },
                name,
                expr,
                offset,
            );
            self.scopes.pop();
            match result {
                Ok(value) => return Ok(Some(value)),
                Err(error)
                    if error.message.starts_with("Unresolved")
                        || error.message == "Java field requires a class receiver" =>
                {
                    self.next = next;
                    self.initialized = initialized;
                    self.refinements = refinements;
                    self.lambdas.borrow_mut().truncate(lambdas);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }
    pub(super) fn companion_receiver(&self, owner: &str, offset: usize) -> Result<Option<Value>> {
        let class = self
            .classpath
            .load(owner)
            .or_else(|error| self.classes.get(owner).cloned().ok_or(error))?;
        let Some(name) = class
            .library
            .as_ref()
            .and_then(|library| library.companion_name.as_ref())
        else {
            return Ok(None);
        };
        let field = class
            .fields
            .iter()
            .find(|field| field.name == *name && field.access & 8 != 0)
            .ok_or_else(|| {
                Diagnostic::new(offset, "Companion metadata has no matching singleton field")
            })?;
        if field.access & 1 == 0
            && self.lambda_owner != owner
            && !self.lambda_owner.starts_with(&format!("{owner}$"))
        {
            return Err(
                Diagnostic::new(offset, "Companion is inaccessible from this declaration")
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "INACCESSIBLE_COMPANION",
                    ),
            );
        }
        let target = field
            .descriptor
            .strip_prefix('L')
            .and_then(|descriptor| descriptor.strip_suffix(';'))
            .ok_or_else(|| {
                Diagnostic::new(offset, "Companion field has a non-reference descriptor")
            })?;
        Ok(Some(Value {
            ty: Type::Java(target.into()),
            kind: ValueKind::StaticField {
                owner: owner.into(),
                name: name.clone(),
            },
        }))
    }
    pub(super) fn implicit_extension_receiver_call(
        &mut self,
        name: &str,
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
    ) -> Result<Option<Value>> {
        for receiver in self.extension_receivers.clone() {
            let next = self.next;
            let initialized = self.initialized.clone();
            let refinements = self.refinements.clone();
            let lambdas = self.lambdas.borrow().len();
            let local_name = format!("this@<implicit{}>", receiver.slot);
            self.scopes
                .push(HashMap::from([(local_name.clone(), receiver)]));
            let target = Expr {
                offset,
                kind: ExprKind::Property(
                    Box::new(Expr {
                        offset,
                        kind: ExprKind::Name(local_name),
                    }),
                    name.into(),
                ),
            };
            let target = if explicit.is_empty() {
                target
            } else {
                Expr {
                    offset,
                    kind: ExprKind::Generic(Box::new(target), explicit.to_vec()),
                }
            };
            let result = self.value(&Expr {
                offset,
                kind: ExprKind::Call(Box::new(target), args.to_vec()),
            });
            self.scopes.pop();
            match result {
                Ok(value) => return Ok(Some(value)),
                Err(error)
                    if error.message.starts_with("Unresolved member")
                        || error.message.starts_with("Unsupported member receiver")
                        || error.message.starts_with("No applicable")
                        || error.code == "TYPE_MISMATCH"
                        || error.code == "ARGUMENT_MAPPING_MISMATCH" =>
                {
                    self.next = next;
                    self.initialized = initialized;
                    self.refinements = refinements;
                    self.lambdas.borrow_mut().truncate(lambdas);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }

    pub(super) fn implicit_extension_receiver_property(
        &mut self,
        name: &str,
        offset: usize,
    ) -> Result<Option<Value>> {
        for receiver in self.extension_receivers.clone() {
            let next = self.next;
            let initialized = self.initialized.clone();
            let refinements = self.refinements.clone();
            let lambdas = self.lambdas.borrow().len();
            let local_name = format!("this@<implicit{}>", receiver.slot);
            self.scopes
                .push(HashMap::from([(local_name.clone(), receiver)]));
            let result = self.value(&Expr {
                offset,
                kind: ExprKind::Property(
                    Box::new(Expr {
                        offset,
                        kind: ExprKind::Name(local_name),
                    }),
                    name.into(),
                ),
            });
            self.scopes.pop();
            match result {
                Ok(value) => return Ok(Some(value)),
                Err(error)
                    if error.message.starts_with("Unresolved")
                        || error.message.starts_with("Unsupported property")
                        || error.message.starts_with("No applicable") =>
                {
                    self.next = next;
                    self.initialized = initialized;
                    self.refinements = refinements;
                    self.lambdas.borrow_mut().truncate(lambdas);
                }
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }
    pub(super) fn implicit_receiver_values(&self, offset: usize) -> Result<Vec<Value>> {
        let mut receivers = self
            .extension_receivers
            .iter()
            .map(Self::local_receiver)
            .collect::<Vec<_>>();
        if let Ok(dispatch) = self.dispatch_receiver_value(offset) {
            let companion = library::type_owner(&dispatch.ty)
                .map(|owner| self.companion_receiver(owner, offset))
                .transpose()?
                .flatten();
            receivers.push(dispatch);
            if let Some(companion) = companion {
                receivers.push(companion);
            }
        }
        Ok(receivers)
    }

    pub(super) fn instance_extension_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        explicit: &[Type],
        kind: &ExtensionKind,
        offset: usize,
    ) -> Result<Option<Value>> {
        let property = matches!(kind, ExtensionKind::Property | ExtensionKind::PropertySet);
        for dispatch in self.implicit_receiver_values(offset)? {
            let Some(owner) = library::type_owner(&dispatch.ty) else {
                continue;
            };
            let class = self
                .classpath
                .load(owner)
                .or_else(|error| self.classes.get(owner).cloned().ok_or(error))?;
            let mut seen = HashSet::new();
            let mut projected = false;
            let mut methods = self
                .classpath
                .methods(owner)
                .unwrap_or_else(|_| class.methods.clone())
                .into_iter()
                .filter(|method| {
                    method.access & 8 == 0
                        && self.lexical_method_visible(method)
                        && if property {
                            method.source_property.as_deref() == Some(name)
                        } else {
                            method.source_property.is_none() && method.source_name() == name
                        }
                })
                .filter(|method| {
                    method.library.as_ref().is_some_and(|library| {
                        library.receiver.is_some()
                            && (!matches!(kind, ExtensionKind::Operator) || library.operator)
                            && (!matches!(kind, ExtensionKind::Infix) || library.infix)
                    })
                })
                .filter(|method| {
                    seen.insert((
                        method.owner.clone(),
                        method.name.clone(),
                        method.descriptor.clone(),
                    ))
                })
                .filter(|method| {
                    let blocked =
                        relations::projected_input(method, &dispatch.ty, &class, self.classpath);
                    projected |= blocked;
                    !blocked
                })
                .map(|method| {
                    library::specialize_receiver(method, &dispatch.ty, &class, self.classpath)
                })
                .collect::<Vec<_>>();
            if methods.is_empty() {
                if projected {
                    return Err(Diagnostic::new(offset, "Projected dispatch receivers cannot consume their captured type as an extension receiver").with_kind(crate::DiagnosticKind::InvalidKotlin, "PROJECTED_MEMBER_EXTENSION_INPUT"));
                }
                continue;
            }
            let next = self.next;
            let initialized = self.initialized.clone();
            let refinements = self.refinements.clone();
            let lambdas = self.lambdas.borrow().len();
            for method in &mut methods {
                let library = method.library.as_mut().unwrap();
                let extension = library.receiver.take().unwrap();
                let params = method.kotlin_params.as_mut().ok_or_else(|| {
                    Diagnostic::new(offset, "Member extension metadata has no value parameters")
                })?;
                params.insert(0, ("<this>".into(), extension, false));
            }
            let dispatch_slot = self.allocate_typed_slot(&dispatch.ty, offset)?;
            let dispatch_value = Self::local_receiver(&Local {
                slot: dispatch_slot,
                ty: dispatch.ty.clone(),
                mutable: false,
            });
            let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
            let local_name = format!("<member-extension{slot}>");
            self.scopes.push(HashMap::from([(
                local_name.clone(),
                Local {
                    slot,
                    ty: receiver.ty.clone(),
                    mutable: false,
                },
            )]));
            self.initialized.insert(slot);
            let mut arguments = vec![Expr {
                offset,
                kind: ExprKind::Name(local_name),
            }];
            arguments.extend_from_slice(args);
            let selected = if property {
                self.select_extension_accessor(
                    &methods,
                    &arguments,
                    explicit,
                    matches!(kind, ExtensionKind::PropertySet),
                    offset,
                )
            } else {
                self.select_method_types(&methods, &arguments, explicit, offset)
            };
            let value = match selected {
                Ok(method) => self
                    .member_call_with_extension(method, dispatch_value, &arguments, true, offset)
                    .map(Some),
                Err(error)
                    if error.code == "TYPE_MISMATCH"
                        || error.code == "ARGUMENT_MAPPING_MISMATCH"
                        || error.message == "No applicable member overload" =>
                {
                    Ok(None)
                }
                Err(error) => Err(error),
            };
            self.scopes.pop();
            if let Some(value) = value? {
                return Ok(Some(Self::wrap_lets(
                    value,
                    vec![(dispatch_slot, dispatch), (slot, receiver)],
                )));
            }
            self.next = next;
            self.initialized = initialized;
            self.refinements = refinements;
            self.lambdas.borrow_mut().truncate(lambdas);
        }
        Ok(None)
    }
}
