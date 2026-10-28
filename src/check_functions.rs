use super::*;

impl Checker<'_> {
    pub(super) fn argument_value(&mut self, expr: &Expr, expected: &Type) -> Result<Value> {
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
        if let ExprKind::Call(target, args) = &expr.kind {
            let (target, explicit) = match &target.kind {
                ExprKind::Generic(target, types) => (target.as_ref(), types.as_slice()),
                _ => (target.as_ref(), &[][..]),
            };
            if let ExprKind::Name(name) = &target.kind {
                if let Some(value) =
                    self.lexical_call(name, args, explicit, Some(expected), expr.offset)?
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
        let mut value = self.value(expr)?;
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
        let (params, result, receiver_type) = match expected.base() {
            Type::Function { params, result } => (params, result, None),
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
        let implicit_receiver = receiver_type.is_none()
            && self.receiver.is_some()
            && capture_names.iter().any(|name| {
                self.local(name).is_none()
                    && !self.captured.contains_key(name)
                    && (name == "this"
                        || self.receiver.is_some()
                            && !self.functions.contains_key(name)
                            && !self.constants.contains_key(name)
                            && !self.imports.contains_key(name)
                        || self.fields.contains_key(name)
                        || self.members.values().any(|m| {
                            m.access & 8 == 0
                                && (m.source_name() == name
                                    || m.source_property.as_deref() == Some(name))
                        }))
            });
        if implicit_receiver {
            capture_names.insert("this".into());
        }
        if receiver_type.is_some() {
            capture_names.remove("this");
        }
        capture_names.retain(|name| {
            self.local(name).is_some()
                || self.captured.contains_key(name)
                || name == "this" && self.receiver.is_some()
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
        let own_receiver = receiver_type.is_some();
        let receiver_fields = HashMap::new();
        let receiver_members = HashMap::new();
        let mut checker = Checker {
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
            receiver: None,
            fields: if own_receiver {
                &receiver_fields
            } else {
                self.fields
            },
            members: if own_receiver {
                &receiver_members
            } else {
                self.members
            },
            companion_fields: self.companion_fields,
            functions: self.functions,
            object_methods: self.object_methods,
            object_fields: self.object_fields,
            object_owner: self.object_owner,
            allow_return: local_return,
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
                checker.receiver = Some(Local {
                    slot,
                    ty: ty.clone(),
                    mutable: false,
                });
            }
        }
        let mut receiver_setup = Vec::new();
        if implicit_receiver {
            let (_, ty) = checker.captured.get("this").cloned().unwrap();
            let value = Value {
                ty: ty.clone(),
                kind: ValueKind::JavaField {
                    owner: owner.clone(),
                    name: "this".into(),
                    receiver: Some(Box::new(Value {
                        ty: Type::Java(owner.clone()),
                        kind: ValueKind::Local(0),
                    })),
                },
            };
            let slot = checker.allocate_typed_slot(&ty, expr.offset)?;
            checker.receiver = Some(Local {
                slot,
                ty,
                mutable: false,
            });
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
            defaults,
            owner: owner.clone(),
            captures: fields,
            function: Function {
                class_type_params: Vec::new(),
                jvm_result: None,
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
            Type::Function { params, result } => (params.clone(), result.as_ref().clone()),
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
        assert_eq!(module.lambdas[0].captures[0].0, "this");
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
                annotation_definition: None,
                kotlin_fun_interface: false,
                library: Some(crate::classpath::LibraryClass {
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
            receiver: None,
            fields: &HashMap::new(),
            members: &HashMap::new(),
            companion_fields: &HashMap::new(),
            functions: &HashMap::new(),
            object_methods: &HashMap::new(),
            object_fields: &HashMap::new(),
            object_owner: None,
            allow_return: true,
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
        let ty = Type::Function {
            params,
            result: Box::new(result),
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
        body.push(self.cell_store(cell, ty, value, function.offset)?);
        Ok((body, false))
    }
}
