use super::*;

impl Checker<'_> {
    pub(super) fn argument_value(&mut self, expr: &Expr, expected: &Type) -> Result<Value> {
        if let ExprKind::TrailingLambda(inner) = &expr.kind {
            return self.argument_value(inner, expected);
        }
        let context = self.inline_result.clone();
        if !matches!(
            expr.kind,
            ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. } | ExprKind::Labeled(..)
        ) {
            self.inline_result = None;
        }
        let result = self.expected_value(expr, expected);
        self.inline_result = context;
        result
    }
    pub(super) fn expected_value(&mut self, expr: &Expr, expected: &Type) -> Result<Value> {
        if let ExprKind::TrailingLambda(inner) = &expr.kind {
            return self.expected_value(inner, expected);
        }
        if expected.base() == &Type::ComposableUnitFunction
            && !matches!(
                expr.kind,
                ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. } | ExprKind::Labeled(..)
            )
        {
            return Err(Diagnostic::new(
                expr.offset,
                "Compose callback parameters require a literal lambda",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "COMPOSE_LITERAL_CALLBACK_REQUIRED",
            ));
        }
        if let ExprKind::Unsigned { bits, long } = &expr.kind {
            let value = self.unsigned_literal(*bits, *long, Some(expected), expr.offset)?;
            return self.coerce(value, expected.clone(), expr.offset);
        }
        if let Type::Nullable(inner) = expected.base() {
            let callable = match &expr.kind {
                ExprKind::Lambda(_)
                | ExprKind::LambdaWithParams { .. }
                | ExprKind::CallableReference { .. } => true,
                ExprKind::Labeled(_, value) => matches!(
                    value.kind,
                    ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
                ),
                _ => false,
            };
            if callable
                && matches!(
                    inner.base(),
                    Type::Function { .. }
                        | Type::ReceiverFunction { .. }
                        | Type::SuspendFunction { .. }
                )
            {
                let value = self.expected_value(expr, inner)?;
                return self.coerce(value, expected.clone(), expr.offset);
            }
        }
        if let Some(labeled) = member_extensions::labeled_call(expr) {
            return self.expected_value(&labeled, expected);
        }
        if let ExprKind::VarargArray { elements, .. } = &expr.kind {
            return self.vararg_array(expected, elements, expr.offset);
        }
        if let ExprKind::Labeled(label, value) = &expr.kind {
            if matches!(
                value.kind,
                ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
            ) {
                return self.lambda_context(value, expected, None, false, Some(label));
            }
        }
        if matches!(
            expr.kind,
            ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
        ) {
            return self.lambda(expr, expected, None);
        }
        if matches!(expr.kind, ExprKind::CallableReference { .. }) {
            if !matches!(expected.base(), Type::Function { .. })
                && self.sam_function(expected, expr.offset)?.is_some()
            {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Callable-reference SAM conversion is not supported; use a literal lambda",
                ));
            }
            return self.callable_reference(expr, expected);
        }
        if matches!(expr.kind, ExprKind::Block(_) | ExprKind::Try { .. }) {
            let value = self.control_value_expected(expr, Some(expected))?;
            return self.coerce(value, expected.clone(), expr.offset);
        }
        if let ExprKind::If(condition, yes, no) = &expr.kind {
            let refinements = self.refinements.clone();
            let checked_condition = self.value(condition)?;
            self.require(&checked_condition, Type::Boolean, condition.offset)?;
            self.refine(condition, true);
            let checked_yes = self.expected_value(yes, expected);
            self.refinements = refinements.clone();
            let checked_yes = checked_yes?;
            self.refine(condition, false);
            let checked_no = self.expected_value(no, expected);
            self.refinements = refinements;
            return Ok(Value {
                ty: expected.clone(),
                kind: ValueKind::If(
                    Box::new(checked_condition),
                    Box::new(checked_yes),
                    Box::new(checked_no?),
                ),
            });
        }
        if let ExprKind::Binary(name, receiver, argument) = &expr.kind
            && operations::is_infix_name(name)
        {
            let value =
                self.check_infix_call(receiver, name, argument, expr.offset, Some(expected))?;
            return self.coerce(value, expected.clone(), expr.offset);
        }
        if let ExprKind::Call(target, args) = &expr.kind {
            let (target, explicit) = match &target.kind {
                ExprKind::Generic(target, types) => (target.as_ref(), types.as_slice()),
                _ => (target.as_ref(), &[][..]),
            };
            if let ExprKind::Property(receiver, name) = &target.kind {
                let classifier = match &receiver.kind {
                    ExprKind::Name(_) => self.value_classifier(receiver)?,
                    ExprKind::Property(..) => self
                        .qualified_java_classifier(receiver, expr.offset)?
                        .map(|(class, _)| class),
                    _ => None,
                };
                if let Some(class) = classifier
                    && !class.kotlin_class
                    && class.access & 1 != 0
                {
                    let methods = self.java_static_methods(&class.name, name)?;
                    if let [method] = methods.as_slice()
                        && method.owner == class.name
                        && method
                            .library
                            .as_ref()
                            .is_some_and(|m| !m.type_parameters.is_empty())
                        && self
                            .classpath
                            .java_member_class(&class.name, name)?
                            .is_none()
                    {
                        let method = self.select_method_expected(
                            &methods,
                            args,
                            explicit,
                            Some(expected),
                            expr.offset,
                        )?;
                        let value = self.library_static_call(method, args, expr.offset)?;
                        return self.coerce(value, expected.clone(), expr.offset);
                    }
                }
                if let Some(value) = self.contextual_extension_call(
                    receiver,
                    name,
                    args,
                    explicit,
                    expected,
                    expr.offset,
                )? {
                    return self.coerce(value, expected.clone(), expr.offset);
                }
            }
            if let ExprKind::Name(name) = &target.kind {
                if name == "Array" && explicit.is_empty() && self.unshadowed(name) {
                    if let Some(element) = expected.base().array_element() {
                        let value = self.array_constructor(&[element], args, expr.offset)?;
                        return self.coerce(value, expected.clone(), expr.offset);
                    }
                }
                if let Some(value) =
                    self.lexical_call(name, args, explicit, Some(expected), expr.offset)?
                {
                    return self.coerce(value, expected.clone(), expr.offset);
                }
                if explicit.is_empty()
                    && let Some(value) =
                        self.inferred_generic_constructor(name, args, Some(expected), expr.offset)?
                {
                    return self.coerce(value, expected.clone(), expr.offset);
                }
                let methods = self
                    .library_call_candidates(name)
                    .into_iter()
                    .filter(|method| {
                        method.source_property.is_none()
                            && method
                                .library
                                .as_ref()
                                .is_none_or(|library| library.receiver.is_none())
                    })
                    .collect::<Vec<_>>();
                if methods.iter().any(|method| {
                    method
                        .library
                        .as_ref()
                        .is_some_and(|library| !library.type_parameters.is_empty())
                }) {
                    let method = self.select_method_expected(
                        &methods,
                        args,
                        explicit,
                        Some(expected),
                        expr.offset,
                    )?;
                    let value = self.library_static_call(method, args, expr.offset)?;
                    return self.coerce(value, expected.clone(), expr.offset);
                }
            }
        }
        let mut value = self.value_expected(expr, Some(expected))?;
        if matches!(expected.base(), Type::SuspendFunction { .. })
            && matches!(
                value.ty.base(),
                Type::Function { .. } | Type::ReceiverFunction { .. }
            )
        {
            return Err(Diagnostic::new(
                expr.offset,
                "Ordinary-to-suspend function value adaptation is unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "SUSPEND_FUNCTION_ADAPTER_UNSUPPORTED",
            ));
        }
        if control::expression_terminates(expr) {
            value.ty = expected.clone();
            return Ok(value);
        }
        if matches!(
            value.ty.base(),
            Type::Function { .. } | Type::ReceiverFunction { .. }
        ) && !matches!(
            expected.base(),
            Type::Function { .. } | Type::ReceiverFunction { .. }
        ) && self.sam_function(expected, expr.offset)?.is_some()
        {
            return Err(Diagnostic::new(
                expr.offset,
                "Function-value SAM conversion is not supported; use a literal lambda",
            ));
        }
        self.coerce(value, expected.clone(), expr.offset)
    }

    pub(super) fn lambda(
        &mut self,
        expr: &Expr,
        expected: &Type,
        interface: Option<JavaMethod>,
    ) -> Result<Value> {
        self.lambda_context(expr, expected, interface, false, None)
    }

    pub(super) fn lambda_context(
        &mut self,
        expr: &Expr,
        expected: &Type,
        interface: Option<JavaMethod>,
        local_return: bool,
        label: Option<&str>,
    ) -> Result<Value> {
        if expected.base() == &Type::ComposableUnitFunction {
            if self.composable_body
                || self.public_inline_body
                || self.inline_analysis
                || interface.is_some()
            {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Compose callbacks in nested or inline contexts are unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "COMPOSE_NESTED_CALLBACK_UNSUPPORTED",
                ));
            }
            crate::classpath::compose_abi::validate_runtime(self.classpath, expr.offset)?;
            self.composable_body = true;
            let checked = self.lambda_context(
                expr,
                &Type::Function {
                    params: Vec::new(),
                    result: Box::new(Type::Unit),
                },
                None,
                local_return,
                label,
            );
            self.composable_body = false;
            let mut value = checked?;
            let ValueKind::Lambda { owner, .. } = &value.kind else {
                unreachable!()
            };
            let mut lambdas = self.lambdas.borrow_mut();
            let declaration = lambdas
                .iter_mut()
                .find(|lambda| &lambda.owner == owner)
                .unwrap();
            declaration.composable_key = Some(crate::classpath::compose_abi::key(owner));
            value.ty = expected.clone();
            return Ok(value);
        }
        if matches!(expected.base(), Type::SuspendFunction { .. }) && !local_return {
            return Err(
                Diagnostic::new(expr.offset, "Source suspend lambdas are unsupported").with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "SUSPEND_LAMBDA_UNSUPPORTED",
                ),
            );
        }
        let (params, result, receiver_type) = match expected.base() {
            Type::Function { params, result } | Type::SuspendFunction { params, result } => {
                (params, result, None)
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => (params, result, Some(receiver.as_ref().clone())),
            _ => {
                if let Some((method, function)) = self.sam_function(expected, expr.offset)? {
                    return self.lambda(expr, &function, Some(method)).map(|mut value| {
                        value.ty = expected.clone();
                        value
                    });
                }
                return Err(Diagnostic::new(
                    expr.offset,
                    "Lambda requires a function or SAM expected type",
                ));
            }
        };
        let (declared, mut statements) = match &expr.kind {
            ExprKind::Lambda(value) => (
                None,
                vec![syntax::Statement {
                    offset: value.offset,
                    kind: StatementKind::Expression(value.as_ref().clone()),
                }],
            ),
            ExprKind::LambdaWithParams { params, body } => (params.clone(), body.clone()),
            _ => unreachable!(),
        };
        let names = match declared {
            Some(declared) => {
                if declared.len() != params.len() {
                    return Err(Diagnostic::new(
                        expr.offset,
                        "Lambda parameter count differs from expected function type",
                    ));
                }
                let mut names = Vec::new();
                for (index, (param, ty)) in declared.into_iter().zip(params).enumerate() {
                    if let Some(explicit) = param.ty {
                        if self.resolve(&explicit, expr.offset)? != *ty {
                            return Err(Diagnostic::new(
                                expr.offset,
                                "Lambda parameter type differs from expected type",
                            ));
                        }
                    }
                    names.push(if param.discard {
                        format!("<unused{index}>")
                    } else {
                        param.name
                    });
                }
                names
            }
            None if params.len() == 1 => vec!["it".into()],
            None if params.is_empty() => Vec::new(),
            None => {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Multiple lambda parameters must be declared explicitly",
                ));
            }
        };
        let owner = format!(
            "{}$lambda${}${}",
            self.lambda_owner,
            expr.offset,
            self.lambdas.borrow().len()
        );
        let mut capture_names = captures::free_names(&statements, &names);
        let receiver_dependent = capture_names.iter().any(|name| {
            if name == "this" {
                return receiver_type.is_none();
            }
            name.starts_with("this@")
                || self.fields.contains_key(name)
                || self.members.values().any(|method| {
                    method.source_name() == name || method.source_property.as_ref() == Some(name)
                })
                || self.local(name).is_none()
                    && !self.captured.contains_key(name)
                    && !self.functions.contains_key(name)
                    && !self.constants.contains_key(name)
                    && !self.imports.contains_key(name)
        });
        capture_names.retain(|name| {
            name != "this"
                && !name.starts_with("this@")
                && (self.local(name).is_some() || self.captured.contains_key(name))
        });
        let mut capture_names = capture_names
            .into_iter()
            .filter(|name| !names.contains(name))
            .collect::<Vec<_>>();
        capture_names.sort();
        let mut captures = Vec::new();
        let mut fields = Vec::new();
        let mut captured = HashMap::new();
        let mut captured_cells = HashMap::new();
        let mut readonly_cells = Vec::new();
        for name in capture_names {
            if self
                .local(&name)
                .is_some_and(|local| !self.initialized.contains(&local.slot))
            {
                return Err(Diagnostic::new(
                    expr.offset,
                    format!("Variable {name} must be initialized before capture"),
                ));
            }
            let value = if let Some((cell, ty)) = self.cell_binding(&name) {
                if self.cell_is_readonly(&name) {
                    readonly_cells.push(name.clone());
                }
                captured_cells.insert(name.clone(), ty);
                cell
            } else {
                if self.local(&name).is_some_and(|local| local.mutable) {
                    return Err(Diagnostic::new(
                        expr.offset,
                        "Mutable capture must be initialized in the enclosing block",
                    ));
                }
                self.value(&Expr {
                    offset: expr.offset,
                    kind: ExprKind::Name(name.clone()),
                })?
            };
            captured.insert(name.clone(), (owner.clone(), value.ty.clone()));
            fields.push((name, value.ty.clone()));
            captures.push(value);
        }
        let mut receiver_captures = Vec::new();
        if receiver_dependent {
            let mut bindings = self
                .extension_receivers
                .iter()
                .map(|local| (false, Some(local.clone()), Self::local_receiver(local)))
                .collect::<Vec<_>>();
            if let Ok(dispatch) = self.dispatch_receiver_value(expr.offset) {
                bindings.push((true, self.dispatch_receiver.clone(), dispatch));
            }
            for (dispatch, binding, value) in bindings {
                let mut ordinal = receiver_captures.len();
                let field = loop {
                    let candidate = format!("$receiver${ordinal}");
                    if !fields.iter().any(|(name, _)| name == &candidate) {
                        break candidate;
                    }
                    ordinal += 1;
                };
                let labels = binding
                    .as_ref()
                    .map(|binding| {
                        self.receiver_labels
                            .iter()
                            .filter(|(_, local)| {
                                local.slot == binding.slot && local.ty == binding.ty
                            })
                            .map(|(name, _)| name.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                receiver_captures.push((dispatch, field.clone(), value.ty.clone(), labels));
                fields.push((field, value.ty.clone()));
                captures.push(value);
            }
        }
        let mut checker = Checker {
            composable_body: self.composable_body,
            inline_argument_modes: Vec::new(),
            inline_analysis: false,
            public_inline_body: self.public_inline_body,
            constructor_bridge_context: true,
            reified_parameters: self.reified_parameters.clone(),
            exhaustiveness_flow: false,
            extension_receivers: Vec::new(),
            receiver_labels: HashMap::new(),
            lambdas: self.lambdas.clone(),
            captured,
            captured_cells,
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: self.lambda_owner.clone(),
            inline_result: None,
            nonlocal_result: if local_return {
                None
            } else {
                self.inline_result.clone()
            },
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
            allow_return: local_return,
            contract_facts: HashMap::new(),
            contract_registration: None,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: 1,
            result: result.as_ref().clone(),
        };
        for name in readonly_cells {
            checker.scopes[0].insert(
                format!("<readonlyCell:{name}>"),
                Local {
                    slot: 0,
                    ty: Type::Unit,
                    mutable: false,
                },
            );
        }
        let mut function_params = names
            .into_iter()
            .zip(params.iter().cloned())
            .collect::<Vec<_>>();
        if let Some(receiver) = receiver_type {
            function_params.insert(0, ("this".into(), receiver));
        }
        if let Some(label) = label {
            checker.scopes[0].insert(
                format!("<return@{label}>"),
                Local {
                    slot: 0,
                    ty: Type::Unit,
                    mutable: false,
                },
            );
        }
        for (name, ty) in &function_params {
            let slot = checker.declare(name, ty.clone(), false, expr.offset)?;
            if name == "this" {
                let receiver = Local {
                    slot,
                    ty: ty.clone(),
                    mutable: false,
                };
                checker.extension_receivers.push(receiver.clone());
                if let Some(label) = label {
                    checker.receiver_labels.insert(label.into(), receiver);
                }
            }
        }
        let suspend_ = matches!(expected.base(), Type::SuspendFunction { .. });
        if suspend_ {
            checker.next += 1;
        }
        let mut receiver_setup = Vec::new();
        for (dispatch, name, ty, labels) in receiver_captures {
            let value = Value {
                ty: ty.clone(),
                kind: ValueKind::JavaField {
                    descriptor: None,
                    owner: owner.clone(),
                    name,
                    receiver: Some(Box::new(Value {
                        ty: Type::Java(owner.clone()),
                        kind: ValueKind::Local(0),
                    })),
                },
            };
            let slot = checker.allocate_typed_slot(&ty, expr.offset)?;
            checker.initialized.insert(slot);
            let receiver = Local {
                slot,
                ty,
                mutable: false,
            };
            if dispatch {
                checker.dispatch_receiver = Some(receiver.clone());
            } else {
                checker.extension_receivers.push(receiver.clone());
            }
            for label in labels {
                checker
                    .receiver_labels
                    .entry(label)
                    .or_insert_with(|| receiver.clone());
            }
            receiver_setup.push(Statement::Store(slot, value));
        }
        let last = statements.pop();
        let (mut body, returns) = checker.block_scoped(&statements, false)?;
        if returns && last.is_some() {
            return Err(Diagnostic::new(expr.offset, "Unreachable lambda result"));
        }
        match last {
            Some(syntax::Statement {
                kind: StatementKind::Expression(value),
                ..
            }) => {
                if result.as_ref() == &Type::Unit {
                    body.push(Statement::Expression(checker.value(&value)?));
                    body.push(Statement::Return(Value {
                        ty: Type::Unit,
                        kind: ValueKind::Unit,
                    }));
                } else {
                    body.push(Statement::Return(checker.expected_value(&value, result)?));
                }
            }
            Some(statement) => {
                let (more, returns) = checker.block(&[statement])?;
                body.extend(more);
                if !returns {
                    if result.as_ref() != &Type::Unit {
                        return Err(Diagnostic::new(
                            expr.offset,
                            "Lambda must end with a result expression",
                        ));
                    }
                    body.push(Statement::Return(Value {
                        ty: Type::Unit,
                        kind: ValueKind::Unit,
                    }));
                }
            }
            None if result.as_ref() == &Type::Unit => body.push(Statement::Return(Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            })),
            None => return Err(Diagnostic::new(expr.offset, "Lambda must produce a result")),
        }
        if self.composable_body {
            let mut used = HashSet::new();
            let mut creates_function = false;
            ir_walk::statements(&mut body, &owner, &mut |value: &mut Value, _: &str| {
                if let ValueKind::Local(slot) = &value.kind {
                    used.insert(*slot);
                }
                creates_function |= matches!(value.kind, ValueKind::Lambda { .. });
            });
            if creates_function {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Function creation in Compose callback bodies requires unsupported memoization",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "COMPOSE_BODY_LAMBDA_UNSUPPORTED",
                ));
            }
            let mut unused = HashSet::new();
            receiver_setup.retain(|statement| {
                let Statement::Store(slot, value) = statement else {
                    unreachable!()
                };
                if used.contains(slot) {
                    return true;
                }
                let ValueKind::JavaField { name, .. } = &value.kind else {
                    unreachable!()
                };
                unused.insert(name.clone());
                false
            });
            (fields, captures) = fields
                .into_iter()
                .zip(captures)
                .filter(|((name, _), _)| !unused.contains(name))
                .unzip();
        }
        receiver_setup.append(&mut body);
        let body = receiver_setup;
        let defaults = if let Some(method) = &interface {
            self.classpath
                .methods(&method.owner)?
                .into_iter()
                .filter(|candidate| {
                    candidate.access & 0x0408 == 0x0400
                        && candidate
                            .library
                            .as_ref()
                            .is_some_and(|library| !library.abstract_)
                })
                .collect()
        } else {
            Vec::new()
        };
        self.lambdas.borrow_mut().push(LambdaDecl {
            composable_key: None,
            anonymous_methods: None,
            anonymous_super: None,
            defaults,
            owner: owner.clone(),
            captures: fields,
            function: Function {
                suspend_,
                inline: false,
                extension_receiver: false,
                class_type_params: Vec::new(),
                jvm_result: suspend_
                    .then(|| Type::Nullable(Box::new(Type::Java("java/lang/Object".into())))),
                jvm_signature: None,
                jvm_name: None,
                annotations: Vec::new(),
                vararg: false,
                type_params: Vec::new(),
                defaults: (0..function_params.len()).map(|_| None).collect(),
                name: "invoke".into(),
                private: false,
                is_override: false,
                params: function_params,
                result: result.as_ref().clone(),
                locals: checker.next,
                body,
            },
            interface,
        });
        Ok(Value {
            ty: expected.clone(),
            kind: ValueKind::Lambda { owner, captures },
        })
    }

    pub(super) fn member_property_value(
        &mut self,
        receiver: Value,
        name: &str,
        offset: usize,
    ) -> Result<Option<Value>> {
        if self
            .dispatch_receiver
            .as_ref()
            .is_some_and(|this| this.ty == receiver.ty)
        {
            if let Some(method) = self.members.get(&format!("#get:{name}")) {
                return self
                    .member_call(method.clone(), receiver, &[], offset)
                    .map(Some);
            }
        }
        if self
            .dispatch_receiver
            .as_ref()
            .is_some_and(|this| this.ty == receiver.ty)
        {
            if let Some(ty) = self.fields.get(name) {
                return Ok(Some(Value {
                    ty: ty.clone(),
                    kind: ValueKind::Field(name.to_owned(), Box::new(receiver)),
                }));
            }
        }
        if self
            .dispatch_receiver
            .as_ref()
            .is_some_and(|this| this.ty == receiver.ty)
        {
            if let Some(method) = self.members.values().find(|method| {
                method.source_property.as_deref() == Some(name)
                    && method
                        .library
                        .as_ref()
                        .is_none_or(|library| library.receiver.is_none())
                    && method.source_name() == getter_name(name)
            }) {
                return self
                    .member_call(method.clone(), receiver, &[], offset)
                    .map(Some);
            }
        }
        if let Some(owner) = library::type_owner(&receiver.ty).map(str::to_owned) {
            let loaded = self.classpath.load(&owner).ok();
            if let Some(class) = self.classes.get(&owner).or(loaded.as_ref()) {
                if let Some(method) =
                    library::ordinary_member_methods(self.classpath, &owner, name, 0)
                        .unwrap_or_else(|_| class.methods.clone())
                        .into_iter()
                        .find(|m| {
                            m.source_property.as_deref() == Some(name)
                                && m.library
                                    .as_ref()
                                    .is_none_or(|library| library.receiver.is_none())
                                && m.kotlin_params.as_ref().is_some_and(Vec::is_empty)
                        })
                {
                    let method =
                        library::specialize_receiver(method, &receiver.ty, class, self.classpath);
                    return self.member_call(method, receiver, &[], offset).map(Some);
                }
                if self
                    .classpath
                    .field_candidates(&owner, name)?
                    .iter()
                    .any(|field| field.access & 1 != 0)
                {
                    return self
                        .java_field(&owner.clone(), name, Some(receiver), offset)
                        .map(Some);
                }
            }
        }
        if let Some(method) = self.java_property_getter(&receiver.ty, name, offset)? {
            return self.member_call(method, receiver, &[], offset).map(Some);
        }
        Ok(None)
    }

    pub(super) fn member_property_invoke(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        types: &[Type],
        offset: usize,
    ) -> Result<Option<Value>> {
        let next = self.next;
        let initialized = self.initialized.clone();
        let refinements = self.refinements.clone();
        let cells = self.cells.clone();
        let cell_names = self.cell_names.clone();
        let contract_registration = self.contract_registration;
        let contract_facts = self.contract_facts.clone();
        let count = self.lambdas.borrow().len();
        let result = (|| {
            let Some(function) = self.member_property_value(receiver, name, offset)? else {
                return Ok(None);
            };
            if !types.is_empty() {
                return Err(Diagnostic::new(
                    offset,
                    "Function properties do not accept explicit type arguments",
                ));
            }
            if !matches!(
                function.ty.base(),
                Type::Function { .. }
                    | Type::ReceiverFunction { .. }
                    | Type::SuspendFunction { .. }
                    | Type::Nullable(_)
            ) {
                return Err(
                    Diagnostic::new(offset, "Property is not callable").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "PROPERTY_NOT_CALLABLE",
                    ),
                );
            }
            self.function_invoke(function, args, offset).map(Some)
        })();
        if result.is_err() {
            self.next = next;
            self.initialized = initialized;
            self.refinements = refinements;
            self.cells = cells;
            self.cell_names = cell_names;
            self.contract_registration = contract_registration;
            self.contract_facts = contract_facts;
            self.lambdas.borrow_mut().truncate(count);
        }
        result
    }

    pub(super) fn function_invoke(
        &mut self,
        function: Value,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        if function.ty.is_nullable() {
            return Err(Diagnostic::new(
                offset,
                "Nullable function values require a safe call or not-null assertion",
            ));
        }
        let (params, result) = match function.ty.base() {
            Type::Function { params, result } | Type::SuspendFunction { params, result } => {
                (params.clone(), result.as_ref().clone())
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => {
                let mut all = vec![receiver.as_ref().clone()];
                all.extend(params.clone());
                (all, result.as_ref().clone())
            }
            _ => return Err(Diagnostic::new(offset, "Value is not a function")),
        };
        if args.len() != params.len() {
            return Err(Diagnostic::new(offset, "Function argument count mismatch"));
        }
        let values = args
            .iter()
            .zip(params)
            .map(|(arg, ty)| self.expected_value(arg, &ty))
            .collect::<Result<Vec<_>>>()?;
        Ok(Value {
            ty: result,
            kind: ValueKind::FunctionInvoke {
                function: Box::new(function),
                args: values,
            },
        })
    }

    pub(super) fn sam_constructor(
        &mut self,
        class: &crate::classpath::JavaClass,
        ty: Type,
        args: &[Expr],
        offset: usize,
    ) -> Result<Option<Value>> {
        if class.access & 0x0200 == 0 || class.kotlin_class && !class.kotlin_fun_interface {
            return Ok(None);
        }
        if args.len() != 1 {
            return Err(Diagnostic::new(
                offset,
                "Functional interface construction requires one function argument",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "SAM_CONSTRUCTOR_ARITY_INVALID",
            ));
        }
        if matches!(ty, Type::Java(_))
            && class
                .library
                .as_ref()
                .is_some_and(|library| !library.type_parameters.is_empty())
        {
            return Err(Diagnostic::new(
                offset,
                "Generic SAM construction requires explicit type arguments",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "GENERIC_SAM_INFERENCE_UNSUPPORTED",
            ));
        }
        let mut argument = &args[0];
        while let ExprKind::TrailingLambda(inner) | ExprKind::Labeled(_, inner) = &argument.kind {
            argument = inner;
        }
        if !matches!(
            argument.kind,
            ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
        ) {
            return Err(Diagnostic::new(
                offset,
                "SAM construction currently requires a literal lambda",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "SAM_CONSTRUCTOR_ARGUMENT_UNSUPPORTED",
            ));
        }
        self.expected_value(&args[0], &ty).map(Some)
    }

    pub(super) fn sam_function(
        &self,
        expected: &Type,
        offset: usize,
    ) -> Result<Option<(JavaMethod, Type)>> {
        let Some(owner) = library::type_owner(expected) else {
            return Ok(None);
        };
        let class = match self.classpath.load(owner) {
            Ok(class) => class,
            Err(_) => return Ok(None),
        };
        if class.access & 0x0200 == 0 || class.kotlin_class && !class.kotlin_fun_interface {
            return Ok(None);
        }
        let methods = self
            .classpath
            .methods(owner)?
            .into_iter()
            .filter(|m| {
                m.access & 0x0408 == 0x0400
                    && m.library.as_ref().is_none_or(|library| library.abstract_)
                    && !matches!(
                        (m.name.as_str(), m.descriptor.as_str()),
                        ("equals", "(Ljava/lang/Object;)Z")
                            | ("hashCode", "()I")
                            | ("toString", "()Ljava/lang/String;")
                    )
            })
            .collect::<Vec<_>>();
        if methods.len() != 1 {
            return Err(Diagnostic::new(
                offset,
                "SAM conversion requires one abstract method",
            ));
        }
        if methods[0].source_property.is_some() {
            return Err(Diagnostic::new(
                offset,
                "Functional interfaces cannot contain abstract properties",
            ));
        }
        if methods[0]
            .library
            .as_ref()
            .is_some_and(|library| !library.type_parameters.is_empty())
        {
            return Err(Diagnostic::new(
                offset,
                "Functional interface abstract methods cannot declare type parameters",
            ));
        }
        let mut method =
            library::specialize_receiver(methods[0].clone(), expected, &class, self.classpath);
        method.owner = owner.into();
        let (params, result) = class_method_types(&method)?;
        Ok(Some((
            method,
            Type::Function {
                params,
                result: Box::new(result),
            },
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checked(source: &str) -> Result<Module> {
        check(&syntax::parse(source)?, &Classpath::new(&[])?)
    }
    #[test]
    fn lambda_captures_only_free_initialized_names() {
        let module=checked("fun f(): Int { var unused: Int; val kept=4; val fn: () -> Int = { val kept=7; kept }; return fn() }").unwrap();
        assert!(module.lambdas[0].captures.is_empty());
        let module=checked("fun f(base: Int): Int { val fn: () -> Int = { val value=base+2; value }; return fn() }").unwrap();
        assert_eq!(module.lambdas[0].captures.len(), 1);
        assert_eq!(module.lambdas[0].captures[0].0, "base");
    }
    #[test]
    fn mutable_captures_share_cells_and_keep_parameter_slots() {
        let module=checked("fun f(): Int { var total=1; val fn: (Int) -> Int = { value -> total += value; total }; fn(2); return total }").unwrap();
        assert!(
            matches!(&module.lambdas[0].captures[0].1,Type::Java(n) if n=="kotlin/jvm/internal/Ref$IntRef")
        );
        assert_eq!(module.lambdas[0].function.params[0].1, Type::Int);
    }
    #[test]
    fn nonlocal_returns_require_inline_context() {
        let error = checked("fun f(): Int { val fn: () -> Int = { return 7 }; return fn() }")
            .err()
            .unwrap();
        assert!(error.message.contains("Non-local return"));
    }
    #[test]
    fn ordinary_methods_retain_lambdas_and_capture_receivers() {
        let module = checked(
            "class Holder { fun run(): Int { val n=4; val fn: () -> Int = { n+1 }; return fn() } }",
        )
        .unwrap();
        assert_eq!(module.lambdas.len(), 1);
        assert!(module.lambdas[0].owner.contains("$lambda$"));
        let module=checked("class Holder(val n: Int) { fun run(): Int { val fn: () -> Int = { n }; return fn() } }").unwrap();
        assert_eq!(module.lambdas[0].captures.len(), 1);
        assert_eq!(module.lambdas[0].captures[0].1, Type::Java("Holder".into()));
    }
    #[test]
    fn same_source_classifiers_validate_recursively_without_classpath_outputs() {
        checked("package local\nclass Recursive(val parent: Recursive?) { companion object { val INSTANCE: Recursive = Recursive(null) }; fun values(value: List<Recursive?>): List<Recursive?> = value; fun apply(block: (Recursive) -> Recursive): Recursive = block(this) }").unwrap();
        let error = resolve_type_checked(
            &Type::Java("missing/External".into()),
            &HashMap::new(),
            0,
            &Classpath::new(&[]).unwrap(),
            &HashMap::new(),
        )
        .err()
        .unwrap();
        assert!(error.message.contains("not found"));
    }
    #[test]
    fn source_generic_calls_substitute_explicit_and_inferred_parameters() {
        checked("fun <T> identity(value:T):T=value\nfun inferred():Int=identity(4)\nfun explicit():String=identity<String>(\"value\")").unwrap();
        assert!(
            checked("fun <T> identity(value:T):T=value\nfun invalid():String=identity<Int>(4)")
                .is_err()
        );
    }
}

#[cfg(test)]
mod sam_object_tests {
    #[test]
    fn only_exact_object_methods_are_excluded_from_sam_abstract_members() {
        let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let compiler = project.join("references/kotlinc/bin/kotlinc");
        let stdlib = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
        let root = std::env::temp_dir().join(format!(
            "korrust-sam-object-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let java = root.join("ObjectMethods.java");
        let dependencies = root.join("dependencies");
        std::fs::write(&java, "package sam; public final class ObjectMethods { public interface Good { boolean equals(Object value); int run(); } public interface Bad { boolean equals(String value); void run(); } public static int good(Good value) { return value.run(); } public static int bad(Bad value) { value.run(); return 1; } }").unwrap();
        let output = std::process::Command::new("javac")
            .arg("-d")
            .arg(&dependencies)
            .arg(&java)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let positive = "import sam.ObjectMethods\nfun good(): Int = ObjectMethods.good { 7 }";
        let negative = "import sam.ObjectMethods\nfun bad(): Int = ObjectMethods.bad { }";
        let classpath = [dependencies.clone(), stdlib];
        let classes =
            crate::compile_all_with_classpath(positive, "GoodSam.kt", &classpath).unwrap();
        assert_eq!(classes.len(), 2);
        let error = crate::compile_all_with_classpath(negative, "BadSam.kt", &classpath)
            .err()
            .unwrap();
        assert!(
            error
                .message
                .contains("SAM conversion requires one abstract method")
                || error.message.contains("No applicable member overload"),
            "{}",
            error.message
        );
        for (name, source, accepted) in [
            ("GoodSam.kt", positive, true),
            ("BadSam.kt", negative, false),
        ] {
            let path = root.join(name);
            std::fs::write(&path, source).unwrap();
            let output = std::process::Command::new(&compiler)
                .arg(&path)
                .arg("-cp")
                .arg(&dependencies)
                .arg("-d")
                .arg(root.join(name.trim_end_matches(".kt")))
                .output()
                .unwrap();
            assert_eq!(
                output.status.success(),
                accepted,
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod shadowing_tests {
    use super::*;
    #[test]
    fn captured_local_shadows_an_imported_property_getter() {
        let classpath = Classpath::new(&[]).unwrap();
        let empty = HashMap::new();
        let imports = HashMap::from([(
            "libraryNumber".into(),
            "foreign/Library::libraryNumber".into(),
        )]);
        let classes = HashMap::from([(
            "foreign/Library".into(),
            JavaClass {
                sealed_: false,
                sealed_subclasses: Vec::new(),
                annotation_definition: None,
                kotlin_fun_interface: false,
                library: Some(crate::classpath::LibraryClass {
                    value_class: None,
                    companion_name: None,
                    kotlin_visibility: None,
                    type_parameters: Vec::new(),
                    type_aliases: Vec::new(),
                    supertypes: Vec::new(),
                    package: Some("foreign".into()),
                    multifile_facade: None,
                }),
                name: "foreign/Library".into(),
                access: 1,
                super_name: None,
                interfaces: Vec::new(),
                fields: Vec::new(),
                kotlin_class: true,
                unsupported: false,
                kotlin_interface: false,
                methods: vec![JavaMethod {
                    library: Some(crate::classpath::LibraryMethod {
                        default_code: None,
                        vararg_index: None,
                        contracts: Vec::new(),
                        value_dispatch: false,
                        suspend_: false,
                        kotlin_declared: true,
                        has_annotations: false,
                        parameter_modes: Vec::new(),
                        reified_arguments: Vec::new(),
                        abstract_: false,
                        builtin_array: false,
                        source_name: "getLibraryNumber".into(),
                        type_parameters: Vec::new(),
                        receiver: None,
                        inline: false,
                        infix: false,
                        operator: false,
                        kotlin_public: true,
                        code: None,
                    }),
                    owner: "foreign/Library".into(),
                    name: "getLibraryNumber".into(),
                    descriptor: "()I".into(),
                    access: 9,
                    unsupported: false,
                    source_property: Some("libraryNumber".into()),
                    kotlin_params: Some(Vec::new()),
                    kotlin_result: Some(Type::Int),
                    kotlin_internal: false,
                }],
            },
        )]);
        let mut checker = Checker {
            composable_body: false,
            inline_argument_modes: Vec::new(),
            inline_analysis: false,
            public_inline_body: false,
            constructor_bridge_context: false,
            reified_parameters: Vec::new(),
            exhaustiveness_flow: false,
            extension_receivers: Vec::new(),
            receiver_labels: HashMap::new(),
            lambdas: Rc::new(RefCell::new(Vec::new())),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: "Shadowed".into(),
            inline_result: None,
            nonlocal_result: None,
            classpath: &classpath,
            constants: &empty,
            constructor: None,
            defaults: &Vec::new(),
            function_names: &[],
            mutable_fields: &HashSet::new(),
            extensions: &HashSet::new(),
            classes: &classes,
            imports: &imports,
            signatures: &Vec::new(),
            source_type_params: &[],
            source_infix: &HashSet::new(),
            source_operator: &HashSet::new(),
            dispatch_receiver: None,
            fields: &HashMap::new(),
            members: &HashMap::new(),
            companion_fields: &HashMap::new(),
            functions: &HashMap::new(),
            object_methods: &HashMap::new(),
            object_fields: &HashMap::new(),
            object_owner: None,
            allow_return: true,
            contract_facts: HashMap::new(),
            contract_registration: None,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: 0,
            result: Type::Int,
        };
        checker
            .declare("libraryNumber", Type::Int, false, 0)
            .unwrap();
        let expr = Expr {
            offset: 0,
            kind: ExprKind::Lambda(Box::new(Expr {
                offset: 0,
                kind: ExprKind::Name("libraryNumber".into()),
            })),
        };
        checker
            .expected_value(
                &expr,
                &Type::Function {
                    params: Vec::new(),
                    result: Box::new(Type::Int),
                },
            )
            .unwrap();
        let lambdas = checker.lambdas.borrow();
        assert!(
            matches!(&lambdas[0].function.body[0],Statement::Return(Value {kind:ValueKind::JavaField {owner,..},..}) if owner.starts_with("Shadowed$lambda$"))
        );
    }
}

impl Checker<'_> {
    pub(super) fn local_function(
        &mut self,
        function: &syntax::Function,
    ) -> Result<(Vec<Statement>, bool)> {
        if function.vararg {
            return Err(Diagnostic::new(
                function.offset,
                "Local vararg function call adaptation is unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "LOCAL_VARARG_UNSUPPORTED",
            ));
        }
        if function.suspend_
            && (!function.type_params.is_empty()
                || function.receiver.is_some()
                || function.defaults.iter().any(Option::is_some)
                || function.vararg
                || function.inline
                || !function.annotations.is_empty()
                || function
                    .param_annotations
                    .iter()
                    .any(|annotations| !annotations.is_empty()))
        {
            return Err(Diagnostic::new(function.offset, "Generic, extension, defaulted, vararg, inline, and annotated local suspend functions are unsupported")
                .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "SUSPEND_LOCAL_DECLARATION_SHAPE_UNSUPPORTED"));
        }
        if !function.type_params.is_empty()
            || function.receiver.is_some()
            || function.defaults.iter().any(Option::is_some)
        {
            return Err(Diagnostic::new(
                function.offset,
                "Generic local functions, local extension functions, and local default arguments are unsupported",
            ));
        }
        let params = function
            .params
            .iter()
            .map(|(_, ty)| self.resolve(ty, function.offset))
            .collect::<Result<Vec<_>>>()?;
        let result = if function.result_inferred {
            let scope_count = self.scopes.len();
            let saved_contract_registration = self.contract_registration;
            let saved_contract_facts = self.contract_facts.clone();
            let next = self.next;
            let initialized = self.initialized.clone();
            let refinements = self.refinements.clone();
            let cells = self.cells.clone();
            let cell_names = self.cell_names.clone();
            let loop_depth = self.loop_depth;
            let lambda_count = self.lambdas.borrow().len();
            self.scopes.push(HashMap::new());
            let inferred = (|| {
                for ((name, _), ty) in function.params.iter().zip(&params) {
                    self.declare(name, ty.clone(), false, function.offset)?;
                }
                let Some(syntax::Statement {
                    kind: StatementKind::Return(Some(expr)),
                    ..
                }) = function.body.first()
                else {
                    return Err(Diagnostic::new(
                        function.offset,
                        "Local function return type cannot be inferred",
                    ));
                };
                self.value(expr).map(|value| value.ty)
            })();
            self.scopes.truncate(scope_count);
            self.contract_facts = saved_contract_facts;
            self.contract_registration = saved_contract_registration;
            self.next = next;
            self.initialized = initialized;
            self.refinements = refinements;
            self.cells = cells;
            self.cell_names = cell_names;
            self.loop_depth = loop_depth;
            self.lambdas.borrow_mut().truncate(lambda_count);
            inferred?
        } else {
            self.resolve(&function.result, function.offset)?
        };
        let ty = if function.suspend_ {
            Type::SuspendFunction {
                params,
                result: Box::new(result),
            }
        } else {
            Type::Function {
                params,
                result: Box::new(result),
            }
        };
        let slot = self.declare(&function.name, ty.clone(), false, function.offset)?;
        let owner = Self::cell_owner(&ty);
        self.cells.insert(slot, owner.clone());
        let cell = Value {
            ty: Type::Java(owner.clone()),
            kind: ValueKind::Local(slot),
        };
        let mut body = vec![Statement::Store(
            slot,
            Value {
                ty: Type::Java(owner.clone()),
                kind: ValueKind::New {
                    owner,
                    descriptor: "()V".into(),
                    args: Vec::new(),
                },
            },
        )];
        let expr = Expr {
            offset: function.offset,
            kind: ExprKind::LambdaWithParams {
                params: Some(
                    function
                        .params
                        .iter()
                        .map(|(name, ty)| syntax::LambdaParameter {
                            discard: false,
                            name: name.clone(),
                            ty: Some(ty.clone()),
                        })
                        .collect(),
                ),
                body: function.body.clone(),
            },
        };
        let value = self.lambda_context(&expr, &ty, None, true, Some(&function.name))?;
        if function.suspend_
            && self.lambdas.borrow().last().is_some_and(|lambda| {
                lambda
                    .captures
                    .iter()
                    .any(|(name, _)| name == &function.name)
            })
        {
            return Err(Diagnostic::new(
                function.offset,
                "Recursive local suspend functions are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "SUSPEND_LOCAL_RECURSION_UNSUPPORTED",
            ));
        }
        body.push(self.cell_store(cell, ty, value, function.offset)?);
        Ok((body, false))
    }
}
