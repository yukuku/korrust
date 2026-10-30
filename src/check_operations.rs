use super::*;

pub(super) enum ExtensionKind {
    Ordinary,
    Operator,
    Infix,
    Property,
    MemberFallback(Diagnostic),
}

impl Checker<'_> {
    pub(super) fn check_for_iteration(
        &mut self,
        name: &str,
        iterable: &Expr,
        source_body: &[syntax::Statement],
        offset: usize,
    ) -> Result<Statement> {
        let iterable = self.value(iterable)?;
        let iterator_element = match iterable.ty.base() {
            Type::List(element) | Type::MutableList(element) => Some(*element.clone()),
            _ => relations::as_supertype(self.classpath, &iterable.ty, "java/lang/Iterable")
                .and_then(|ty| match ty {
                    Type::Parameterized { arguments, .. }
                        if arguments.len() == 1
                            && arguments[0].variance != syntax::Variance::In =>
                    {
                        arguments[0].ty.clone()
                    }
                    _ => None,
                }),
        };
        if let Some(element) = iterator_element {
            if matches!(element, Type::Unit | Type::Null) || iterable.ty.is_nullable() {
                return Err(Diagnostic::new(
                    offset,
                    "Iteration requires a non-null Iterable with a supported element type",
                ));
            }
            let refinements = self.refinements.clone();
            let before = self.initialized.clone();
            self.scopes.push(HashMap::new());
            let slot = self.declare(name, element.clone(), false, offset)?;
            let iterator_slot = self.allocate_slot(offset)?;
            self.loop_depth += 1;
            let (body, _) = self.block(source_body)?;
            self.loop_depth -= 1;
            self.scopes.pop();
            self.initialized = before;
            self.refinements = refinements;
            return Ok(Statement::ForIterator {
                element,
                iterable,
                iterator_slot,
                slot,
                body,
            });
        }
        let element = match iterable.ty.base() {
            Type::String => Type::Char,
            array if array.array_element().is_some() => array.array_element().unwrap(),
            _ => {
                return Err(Diagnostic::new(
                    offset,
                    "Iteration supports primitive arrays, String, and non-null Iterable types with resolvable element metadata",
                ));
            }
        };
        if matches!(element, Type::Unit | Type::Null) {
            return Err(Diagnostic::new(offset, "Unsupported array element type"));
        }
        if unresolved_list(&iterable.ty) {
            return Err(Diagnostic::new(
                offset,
                "Cannot infer iteration element type",
            ));
        }
        let refinements = self.refinements.clone();
        let before = self.initialized.clone();
        self.scopes.push(HashMap::new());
        let slot = self.declare(name, element, false, offset)?;
        let iterable_slot = self.allocate_slot(offset)?;
        let index_slot = self.allocate_slot(offset)?;
        self.loop_depth += 1;
        let (body, _) = self.block(source_body)?;
        self.loop_depth -= 1;
        self.scopes.pop();
        self.initialized = before;
        self.refinements = refinements;
        Ok(Statement::ForEach {
            slot,
            iterable,
            iterable_slot,
            index_slot,
            body,
        })
    }

    pub(super) fn check_do_while(
        &mut self,
        condition: &Expr,
        source_body: &[syntax::Statement],
        offset: usize,
    ) -> Result<Statement> {
        let before = self.initialized.clone();
        let refinements = self.refinements.clone();
        self.loop_depth += 1;
        let (body, _) = self.block_scoped(source_body, false)?;
        self.refinements = refinements.clone();
        fn has_continue(body: &[syntax::Statement]) -> bool {
            body.iter().any(|s| match &s.kind {
                StatementKind::Continue => true,
                StatementKind::If(_, a, b) => has_continue(a) || has_continue(b),
                StatementKind::When(_, branches, tail) => {
                    branches.iter().any(|(_, b)| has_continue(b))
                        || tail.as_ref().is_some_and(|b| has_continue(b))
                }
                _ => false,
            })
        }
        if has_continue(source_body) {
            let mut names = HashSet::new();
            captures::expression_names(condition, &mut names);
            if names
                .iter()
                .any(|n| self.local(n).is_some_and(|l| !before.contains(&l.slot)))
            {
                return Err(Diagnostic::new(
                    offset,
                    "Do-while condition locals must be initialized before any continue path",
                ));
            }
        }
        let condition = self.value(condition)?;
        self.require(&condition, Type::Boolean, offset)?;
        self.scopes.pop();
        self.loop_depth -= 1;
        self.initialized = before;
        self.refinements = refinements;
        Ok(Statement::DoWhile(condition, body))
    }

    pub(super) fn check_compound_assignment(
        &mut self,
        name: &str,
        op: &str,
        right: &Expr,
        offset: usize,
    ) -> Result<Statement> {
        let receiver = self.value(&Expr {
            offset,
            kind: ExprKind::Name(name.into()),
        })?;
        self.check_compound_operator(name, op, receiver, right, offset)
    }

    fn check_compound_operator(
        &mut self,
        target: &str,
        op: &str,
        receiver: Value,
        right: &Expr,
        offset: usize,
    ) -> Result<Statement> {
        let (assignment, binary) = match op {
            "+=" => ("plusAssign", "plus"),
            "-=" => ("minusAssign", "minus"),
            "*=" => ("timesAssign", "times"),
            "/=" => ("divAssign", "div"),
            "%=" => ("remAssign", "rem"),
            _ => {
                return Err(
                    Diagnostic::new(offset, "Unsupported compound operator").with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "COMPOUND_OPERATOR_UNSUPPORTED",
                    ),
                );
            }
        };
        let assigning =
            self.compound_operator_candidate(receiver.clone(), assignment, right, None, offset);
        if assigning.as_ref().is_ok_and(|ty| *ty != Type::Unit) {
            return Err(
                Diagnostic::new(offset, "Assignment operators must return Unit").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "ASSIGN_OPERATOR_RETURN_TYPE",
                ),
            );
        }
        let replacing =
            self.compound_operator_candidate(receiver.clone(), binary, right, Some(target), offset);
        if assigning.is_ok() && replacing.is_ok() {
            return Err(Diagnostic::new(
                offset,
                "Both assignment and binary operators are applicable",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ASSIGN_OPERATOR_AMBIGUITY",
            ));
        }
        if assigning.is_ok() {
            if let Err(error) = &replacing {
                if error.kind == crate::DiagnosticKind::UnsupportedLanguage
                    || error.message.contains("Ambiguous")
                {
                    return Err(error.clone());
                }
            }
            return Ok(Statement::Expression(self.operator_call(
                receiver,
                assignment,
                std::slice::from_ref(right),
                offset,
            )?));
        }
        if replacing.is_ok() {
            if let Err(error) = &assigning {
                if error.kind == crate::DiagnosticKind::UnsupportedLanguage
                    || error.message.contains("Ambiguous")
                {
                    return Err(error.clone());
                }
            }
            let value = self.compound_binary_value(receiver, binary, right, offset)?;
            return self.compound_operator_store(target, value, offset);
        }
        let assigning = assigning.unwrap_err();
        let replacing = replacing.unwrap_err();
        if assigning.message.contains("Ambiguous") || replacing.message.contains("Ambiguous") {
            return Err(
                Diagnostic::new(offset, "Ambiguous compound operator").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "ASSIGN_OPERATOR_AMBIGUITY",
                ),
            );
        }
        if assigning.kind == crate::DiagnosticKind::UnsupportedLanguage {
            return Err(assigning);
        }
        if assigning.message.contains("operator modifier")
            || replacing.message.contains("operator modifier")
        {
            return Err(
                Diagnostic::new(offset, "The operator modifier is required").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "OPERATOR_MODIFIER_REQUIRED",
                ),
            );
        }
        if replacing.message.contains("Cannot reassign") {
            return Err(
                replacing.with_kind(crate::DiagnosticKind::InvalidKotlin, "VAL_REASSIGNMENT")
            );
        }
        if replacing.kind == crate::DiagnosticKind::InvalidKotlin
            || replacing.kind == crate::DiagnosticKind::UnsupportedLanguage
        {
            return Err(replacing);
        }
        Err(
            Diagnostic::new(offset, "No supported compound operator is applicable").with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "COMPOUND_OPERATOR_UNSUPPORTED",
            ),
        )
    }

    fn compound_operator_candidate(
        &mut self,
        receiver: Value,
        method: &str,
        right: &Expr,
        target: Option<&str>,
        offset: usize,
    ) -> Result<Type> {
        let next = self.next;
        let initialized = self.initialized.clone();
        let refinements = self.refinements.clone();
        let lambdas = self.lambdas.borrow().len();
        let result = (|| {
            let value = if target.is_some() {
                self.compound_binary_value(receiver, method, right, offset)?
            } else {
                self.operator_call(receiver, method, std::slice::from_ref(right), offset)?
            };
            let ty = value.ty.clone();
            if let Some(target) = target {
                self.compound_operator_store(target, value, offset)?;
            }
            Ok(ty)
        })();
        self.next = next;
        self.initialized = initialized;
        self.refinements = refinements;
        self.lambdas.borrow_mut().truncate(lambdas);
        result
    }

    fn compound_binary_value(
        &mut self,
        receiver: Value,
        method: &str,
        right: &Expr,
        offset: usize,
    ) -> Result<Value> {
        if !matches!(
            receiver.ty,
            Type::Int | Type::Long | Type::Float | Type::Double | Type::String
        ) {
            return self.operator_call(receiver, method, std::slice::from_ref(right), offset);
        }
        let symbol = match method {
            "plus" => "+",
            "minus" => "-",
            "times" => "*",
            "div" => "/",
            "rem" => "%",
            _ => {
                return Err(Diagnostic::new(
                    offset,
                    "Unsupported binary operator convention",
                ));
            }
        };
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let temporary = format!("<compoundReceiver{slot}>");
        self.scopes.push(HashMap::from([(
            temporary.clone(),
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let result = self.value(&Expr {
            offset,
            kind: ExprKind::Binary(
                symbol.into(),
                Box::new(Expr {
                    offset,
                    kind: ExprKind::Name(temporary),
                }),
                Box::new(right.clone()),
            ),
        });
        self.scopes.pop();
        result.map(|value| Self::wrap_lets(value, vec![(slot, receiver)]))
    }

    fn compound_operator_store(
        &mut self,
        target: &str,
        value: Value,
        offset: usize,
    ) -> Result<Statement> {
        let slot = self.allocate_typed_slot(&value.ty, offset)?;
        let temporary = format!("<compound{slot}>");
        self.scopes.push(HashMap::from([(
            temporary.clone(),
            Local {
                slot,
                ty: value.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let checked = self.block(&[syntax::Statement {
            offset,
            kind: StatementKind::Assign(
                target.into(),
                Expr {
                    offset,
                    kind: ExprKind::Name(temporary),
                },
            ),
        }]);
        self.scopes.pop();
        let (mut body, _) = checked?;
        if let Some(local) = self.local(target) {
            self.refinements.remove(&local.slot);
        }
        body.insert(0, Statement::Store(slot, value));
        Ok(Statement::Expression(Value {
            ty: Type::Unit,
            kind: ValueKind::Block(
                body,
                Box::new(Value {
                    ty: Type::Unit,
                    kind: ValueKind::Unit,
                }),
            ),
        }))
    }

    pub(super) fn check_decrement(
        &mut self,
        expr: &Expr,
        prefix: bool,
        offset: usize,
    ) -> Result<Value> {
        let ExprKind::Name(name) = &expr.kind else {
            return Err(Diagnostic::new(
                offset,
                "Decrement requires a mutable Int local",
            ));
        };
        if let Some((cell, ty)) = self.cell_binding(name) {
            if ty != Type::Int {
                return Err(Diagnostic::new(
                    offset,
                    "Decrement requires a mutable Int local",
                ));
            }
            if let Some(local) = self.local(name) {
                if !local.mutable || !self.initialized.contains(&local.slot) {
                    return Err(Diagnostic::new(
                        offset,
                        "Decrement requires an initialized mutable Int local",
                    ));
                }
            }
            return Ok(Value {
                ty: Type::Int,
                kind: ValueKind::CellDecrement {
                    cell: Box::new(cell),
                    prefix,
                },
            });
        }
        let local = self
            .local(name)
            .ok_or_else(|| Diagnostic::new(offset, "Decrement requires a local"))?;
        if !local.mutable || local.ty != Type::Int || !self.initialized.contains(&local.slot) {
            return Err(Diagnostic::new(
                offset,
                "Decrement requires an initialized mutable Int local",
            ));
        }
        Ok(Value {
            ty: Type::Int,
            kind: ValueKind::Decrement {
                slot: local.slot,
                prefix,
            },
        })
    }

    fn resolve_runtime_type(&self, target: &Type, offset: usize) -> Result<Type> {
        let target = self.resolve(target, offset)?;
        fn supported(ty: &Type) -> bool {
            match ty.base() {
                Type::Nullable(inner) => supported(inner),
                Type::Int
                | Type::Long
                | Type::Byte
                | Type::Char
                | Type::Boolean
                | Type::Float
                | Type::Double
                | Type::IntArray
                | Type::ByteArray
                | Type::CharArray
                | Type::LongArray
                | Type::BooleanArray
                | Type::FloatArray
                | Type::DoubleArray
                | Type::StringArray
                | Type::String
                | Type::Java(_) => true,
                _ => false,
            }
        }
        if !supported(&target) {
            return Err(Diagnostic::new(
                offset,
                "Type tests and casts require a supported runtime type; erased generic types are unsupported",
            ));
        }
        Ok(target)
    }

    pub(super) fn check_type_operation(
        &mut self,
        op: &str,
        source: &Expr,
        target: &Type,
        offset: usize,
    ) -> Result<Value> {
        let target = self.resolve_runtime_type(target, offset)?;
        let value = self.value(source)?;
        self.checked_type_operation(op, value, target, offset)
    }

    fn checked_type_operation(
        &self,
        op: &str,
        value: Value,
        target: Type,
        offset: usize,
    ) -> Result<Value> {
        if value.ty == Type::Unit {
            return Err(Diagnostic::new(
                offset,
                "Unit type operations are unsupported",
            ));
        }
        if matches!(op, "is" | "!is") && value.ty != Type::Null {
            let nonnull = |ty: &Type| {
                if let Type::Nullable(inner) = ty.base() {
                    *inner.clone()
                } else {
                    ty.base().clone()
                }
            };
            let source = nonnull(&value.ty);
            let tested = nonnull(&target);
            if !(self.subtype(&source, &tested)
                || self.subtype(&tested, &source)
                || value.ty.is_nullable() && target.is_nullable())
            {
                return Err(Diagnostic::new(
                    offset,
                    "Incompatible types in type test; unrelated runtime types are unsupported",
                ));
            }
        }
        let (ty, kind) = match op {
            "is" | "!is" => (
                Type::Boolean,
                ValueKind::TypeTest {
                    value: Box::new(value),
                    target,
                    negated: op == "!is",
                },
            ),
            "as" | "as?" => {
                let ty = if op == "as?" && !target.is_nullable() {
                    Type::Nullable(Box::new(target.clone()))
                } else {
                    target.clone()
                };
                (
                    ty,
                    ValueKind::Cast {
                        value: Box::new(value),
                        target,
                        safe: op == "as?",
                    },
                )
            }
            _ => return Err(Diagnostic::new(offset, "Unsupported type operation")),
        };
        Ok(Value { ty, kind })
    }

    pub(super) fn check_when_condition(&mut self, left: Value, condition: &Expr) -> Result<Value> {
        if let ExprKind::TypeOperation(op, subject, target) = &condition.kind {
            if matches!(&subject.kind, ExprKind::Name(name) if name == "$whenSubject") {
                let target = self.resolve_runtime_type(target, condition.offset)?;
                return self.checked_type_operation(op, left, target, condition.offset);
            }
        }
        if let ExprKind::Unary(op, range) = &condition.kind {
            if op == "in" {
                return self.check_contains(left, range, condition.offset);
            }
            return Err(Diagnostic::new(
                condition.offset,
                "Unsupported when range condition",
            ));
        }
        let right = self.value(condition)?;
        let right = self.coerce(right, left.ty.clone(), condition.offset)?;
        self.check_equality("==", left, right, condition.offset)
    }

    pub(super) fn refine_when_branch(&mut self, source: &Expr, conditions: &[Expr]) {
        if conditions.len() != 1 {
            return;
        }
        if let ExprKind::TypeOperation(op, subject, target) = &conditions[0].kind {
            if matches!(&subject.kind, ExprKind::Name(name) if name == "$whenSubject") {
                self.refine(
                    &Expr {
                        offset: source.offset,
                        kind: ExprKind::TypeOperation(
                            op.clone(),
                            Box::new(source.clone()),
                            target.clone(),
                        ),
                    },
                    true,
                );
                return;
            }
        }
        match &source.kind {
            ExprKind::Boolean(truth) => self.refine(&conditions[0], *truth),
            ExprKind::WhenSubjectless => self.refine(&conditions[0], true),
            _ => {}
        }
    }

    pub(super) fn check_float_binary(
        &self,
        op: &str,
        left: Value,
        right: Value,
        offset: usize,
    ) -> Result<Value> {
        if !matches!(op, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=") {
            return Err(Diagnostic::new(offset, "Unsupported Float operation"));
        }
        let numeric = if left.ty == Type::Double || right.ty == Type::Double {
            Type::Double
        } else {
            Type::Float
        };
        let promote = |value: Value| -> Result<Value> {
            if value.ty == numeric {
                return Ok(value);
            }
            if !matches!(value.ty, Type::Int | Type::Long | Type::Byte | Type::Float) {
                return Err(Diagnostic::new(
                    offset,
                    "Float operations require numeric operands",
                ));
            }
            Ok(Value {
                ty: numeric.clone(),
                kind: ValueKind::Convert(numeric.clone(), Box::new(value)),
            })
        };
        let left = promote(left)?;
        let right = promote(right)?;
        Ok(Value {
            ty: if matches!(op, "<" | ">" | "<=" | ">=") {
                Type::Boolean
            } else {
                numeric
            },
            kind: ValueKind::Binary(op.into(), Box::new(left), Box::new(right)),
        })
    }

    pub(super) fn check_equality(
        &mut self,
        op: &str,
        left: Value,
        right: Value,
        offset: usize,
    ) -> Result<Value> {
        if matches!(left.ty, Type::Unit) || matches!(right.ty, Type::Unit) {
            return Err(Diagnostic::new(offset, "Unit equality is unsupported"));
        }
        fn nonnull(ty: &Type) -> &Type {
            match ty.base() {
                Type::Nullable(inner) => inner,
                t => t,
            }
        }
        let compatible = left.ty == Type::Null
            || right.ty == Type::Null
            || self.subtype(nonnull(&left.ty), nonnull(&right.ty))
            || self.subtype(nonnull(&right.ty), nonnull(&left.ty));
        if op == "===" || op == "!==" {
            if !left.ty.is_reference() || !right.ty.is_reference() {
                return Err(Diagnostic::new(
                    offset,
                    "Reference identity requires reference operands",
                ));
            }
            if !compatible {
                return Err(Diagnostic::new(
                    offset,
                    "Identity equality operands have incompatible types",
                ));
            }
            return Ok(Value {
                ty: Type::Boolean,
                kind: ValueKind::Binary(op.into(), Box::new(left), Box::new(right)),
            });
        }
        if !left.ty.is_reference() && !right.ty.is_reference() {
            self.require(&right, left.ty.clone(), offset)?;
            return Ok(Value {
                ty: Type::Boolean,
                kind: ValueKind::Binary(op.into(), Box::new(left), Box::new(right)),
            });
        }
        if !compatible {
            return Err(Diagnostic::new(
                offset,
                "Equality operands have incompatible types",
            ));
        }
        let float = |ty: &Type| match ty.base() {
            Type::Float => true,
            Type::Nullable(t) => t.as_ref() == &Type::Float,
            _ => false,
        };
        let double = |ty: &Type| match ty.base() {
            Type::Double => true,
            Type::Nullable(inner) => inner.as_ref() == &Type::Double,
            _ => false,
        };
        let descriptor = if double(&left.ty) && double(&right.ty) {
            "(Ljava/lang/Double;Ljava/lang/Double;)Z"
        } else if float(&left.ty) && float(&right.ty) {
            "(Ljava/lang/Float;Ljava/lang/Float;)Z"
        } else {
            "(Ljava/lang/Object;Ljava/lang/Object;)Z"
        };
        let boxed = |v: Value| {
            if v.ty.is_reference() {
                v
            } else {
                Value {
                    ty: Type::Nullable(Box::new(v.ty.clone())),
                    kind: ValueKind::Box(Box::new(v)),
                }
            }
        };
        let equal = Value {
            ty: Type::Boolean,
            kind: ValueKind::StaticCall {
                owner: "kotlin/jvm/internal/Intrinsics".into(),
                name: "areEqual".into(),
                descriptor: descriptor.into(),
                args: vec![boxed(left), boxed(right)],
            },
        };
        Ok(if op == "!=" {
            Value {
                ty: Type::Boolean,
                kind: ValueKind::Not(Box::new(equal)),
            }
        } else {
            equal
        })
    }
}

impl Checker<'_> {
    fn callable_package(&self, owner: &str) -> String {
        if let Some(package) = self
            .classes
            .get(owner)
            .and_then(|class| class.library.as_ref())
            .and_then(|library| library.package.as_ref())
        {
            return package.clone();
        }
        self.classpath
            .load(owner)
            .ok()
            .and_then(|class| class.library.and_then(|library| library.package))
            .unwrap_or_else(|| {
                owner
                    .rsplit_once('/')
                    .map_or("", |(package, _)| package)
                    .into()
            })
    }

    pub(super) fn callable_name_was_renamed(&self, name: &str) -> bool {
        self.imports.iter().any(|(alias, binding)| {
            alias != name
                && binding
                    .rsplit_once("::")
                    .is_some_and(|(_, source)| source == name)
                && self.imports.get(name) != Some(binding)
        })
    }

    fn callable_is_renamed(&self, name: &str, owner: &str) -> bool {
        if !self.callable_name_was_renamed(name) {
            return false;
        }
        let package = self.callable_package(owner);
        if self
            .imports
            .get(name)
            .and_then(|binding| binding.rsplit_once("::"))
            .is_some_and(|(imported, source)| {
                source == name && self.callable_package(imported) == package
            })
        {
            return false;
        }
        self.imports.iter().any(|(alias, binding)| {
            alias != name
                && binding.rsplit_once("::").is_some_and(|(imported, source)| {
                    source == name && self.callable_package(imported) == package
                })
        })
    }

    fn extension_candidate_groups(
        &self,
        name: &str,
        property: bool,
    ) -> Result<Vec<Vec<JavaMethod>>> {
        let matches = |method: &&JavaMethod| {
            method.access & 8 != 0
                && (method.access & 1 != 0
                    || method
                        .library
                        .as_ref()
                        .is_some_and(|library| library.inline && library.kotlin_public))
                && method
                    .library
                    .as_ref()
                    .is_some_and(|library| library.receiver.is_some())
                && (if property {
                    method.source_property.as_deref() == Some(name)
                } else {
                    method.source_property.is_none()
                        && method
                            .library
                            .as_ref()
                            .is_some_and(|library| library.source_name == name)
                })
                && !self.callable_is_renamed(name, &method.owner)
        };
        let imported = self
            .library_candidates(name)
            .into_iter()
            .filter(|m| {
                m.library.as_ref().is_some_and(|l| l.receiver.is_some())
                    && m.source_property.is_some() == property
            })
            .collect::<Vec<_>>();
        let package = self
            .lambda_owner
            .rsplit_once('/')
            .map_or("", |(package, _)| package);
        let package_methods = self.classpath.package_methods(package, name)?;
        let local = package_methods
            .iter()
            .filter(matches)
            .cloned()
            .collect::<Vec<_>>();
        let mut implicit = Vec::new();
        for package in library::DEFAULT_PACKAGES {
            let methods = self.classpath.package_methods(package, name)?;
            implicit.extend(methods.iter().filter(matches).cloned());
        }
        let mut groups = vec![imported, local, implicit];
        for group in &mut groups {
            group.sort_by(|a, b| {
                (&a.owner, &a.name, &a.descriptor).cmp(&(&b.owner, &b.name, &b.descriptor))
            });
            group.dedup_by(|a, b| {
                a.owner == b.owner && a.name == b.name && a.descriptor == b.descriptor
            });
        }
        Ok(groups)
    }

    pub(super) fn extension_candidates(
        &self,
        name: &str,
        property: bool,
    ) -> Result<Vec<JavaMethod>> {
        Ok(self
            .extension_candidate_groups(name, property)?
            .into_iter()
            .flatten()
            .collect())
    }

    pub(super) fn resolved_extension_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        types: &[Type],
        kind: ExtensionKind,
        offset: usize,
    ) -> Result<Value> {
        let operator = matches!(&kind, ExtensionKind::Operator);
        let infix = matches!(&kind, ExtensionKind::Infix);
        let property = matches!(&kind, ExtensionKind::Property);
        let fallback = match kind {
            ExtensionKind::MemberFallback(error) => Some(error),
            _ => None,
        };
        let groups = self.extension_candidate_groups(name, property)?;
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let local_name = format!("<extension{slot}>");
        self.scopes.push(HashMap::from([(
            local_name.clone(),
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let mut all_args = vec![Expr {
            offset,
            kind: ExprKind::Name(local_name),
        }];
        all_args.extend_from_slice(args);
        let result = (|| {
            let mut applicable_groups = Vec::new();
            let mut failures = Vec::new();
            let mut non_infix_applicable = false;
            for group in groups {
                let mut applicable = Vec::new();
                for mut method in group {
                    if operator && !method.library.as_ref().is_some_and(|l| l.operator) {
                        continue;
                    }
                    let is_infix = method.library.as_ref().is_some_and(|library| library.infix);
                    let receiver = method
                        .library
                        .as_ref()
                        .unwrap()
                        .receiver
                        .as_ref()
                        .unwrap()
                        .clone();
                    if let Some(params) = &mut method.kotlin_params {
                        params.insert(0, ("<receiver>".into(), receiver, false));
                    }
                    match self.select_method_types(
                        std::slice::from_ref(&method),
                        &all_args,
                        types,
                        offset,
                    ) {
                        Ok(_) if infix && !is_infix => non_infix_applicable = true,
                        Ok(_) => applicable.push(method),
                        Err(error) => failures.push(error),
                    }
                }
                applicable_groups.push(applicable);
            }
            if !applicable_groups[0].is_empty()
                && applicable_groups[1].iter().any(|local| {
                    !applicable_groups[0].iter().any(|imported| {
                        imported.owner == local.owner
                            && imported.name == local.name
                            && imported.descriptor == local.descriptor
                    })
                })
            {
                return Err(Diagnostic::new(
                    offset,
                    "Competing imported and same-package extensions require import-origin precedence that is currently unsupported",
                ));
            }
            let Some(candidates) = applicable_groups.iter().find(|group| !group.is_empty()) else {
                if let Some(error) = fallback {
                    return Err(error);
                }
                if self.callable_name_was_renamed(name) {
                    return Err(Diagnostic::new(
                        offset,
                        format!("Unresolved renamed declaration: {name}"),
                    ));
                }
                let source_candidates = self
                    .function_names
                    .iter()
                    .enumerate()
                    .filter_map(|(id, candidate)| {
                        (candidate == name
                            && self.extensions.contains(&id)
                            && !self.source_infix.contains(&id))
                        .then_some(id)
                    })
                    .collect::<Vec<_>>();
                if infix
                    && (non_infix_applicable
                        || !source_candidates.is_empty()
                            && self
                                .select_function_types(&source_candidates, &all_args, types, offset)
                                .is_ok())
                {
                    return Err(Diagnostic::new(
                        offset,
                        format!("The infix modifier is required on {name}"),
                    ));
                }
                if failures.len() == 1 {
                    return Err(failures.remove(0));
                }
                return Err(Diagnostic::new(
                    offset,
                    format!(
                        "Unresolved or unsupported applicable {}extension: {name}",
                        if infix { "infix " } else { "" }
                    ),
                ));
            };
            let method = self.select_method_types(candidates, &all_args, types, offset)?;
            self.library_static_call(method, &all_args, offset)
        })();
        self.scopes.pop();
        result.map(|value| Self::wrap_lets(value, vec![(slot, receiver)]))
    }

    pub(super) fn member_extension_fallback(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        member_error: Diagnostic,
    ) -> Result<Value> {
        let offset = member_error.offset;
        let source_candidates = self
            .function_names
            .iter()
            .enumerate()
            .filter_map(|(id, candidate)| {
                (candidate == name && self.extensions.contains(&id)).then_some(id)
            })
            .collect::<Vec<_>>();
        if source_candidates.is_empty() {
            return self.resolved_extension_call(
                receiver,
                name,
                args,
                &[],
                ExtensionKind::MemberFallback(member_error),
                offset,
            );
        }
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let local_name = format!("<memberReceiver{slot}>");
        self.scopes.push(HashMap::from([(
            local_name.clone(),
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let mut all_args = vec![Expr {
            offset,
            kind: ExprKind::Name(local_name),
        }];
        all_args.extend_from_slice(args);
        let result = {
            let binding = Value {
                ty: receiver.ty.clone(),
                kind: ValueKind::Local(slot),
            };
            match self.select_function_types(&source_candidates, &all_args, &[], offset) {
                Ok(_) => self.source_extension_call(binding, name, args, &[], false, offset),
                Err(error) if error.message.contains("Ambiguous") => Err(error),
                Err(_) => self.resolved_extension_call(
                    binding,
                    name,
                    args,
                    &[],
                    ExtensionKind::MemberFallback(member_error),
                    offset,
                ),
            }
        };
        self.scopes.pop();
        result.map(|value| Self::wrap_lets(value, vec![(slot, receiver)]))
    }

    pub(super) fn source_extension_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        explicit: &[Type],
        infix: bool,
        offset: usize,
    ) -> Result<Value> {
        let candidates = self
            .function_names
            .iter()
            .enumerate()
            .filter_map(|(id, candidate)| {
                (candidate == name
                    && self.extensions.contains(&id)
                    && (!infix || self.source_infix.contains(&id)))
                .then_some(id)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Err(Diagnostic::new(
                offset,
                format!(
                    "No supported {}source extension: {name}",
                    if infix { "infix " } else { "" }
                ),
            ));
        }
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let local_name = format!("<receiver{slot}>");
        self.scopes.push(HashMap::from([(
            local_name.clone(),
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let mut all_args = vec![Expr {
            offset,
            kind: ExprKind::Name(local_name),
        }];
        all_args.extend_from_slice(args);
        let checked = (|| {
            let id = self.select_function_types(&candidates, &all_args, explicit, offset)?;
            let (params, result) = self.specialize_source(id, &all_args, explicit, offset)?;
            let (values, lets, mask) =
                self.arguments(&params, &vec![false; params.len()], &all_args, true, offset)?;
            if mask != 0 {
                return Err(Diagnostic::new(
                    offset,
                    "Default extension arguments are unsupported",
                ));
            }
            Ok(Self::wrap_lets(
                Value {
                    ty: result,
                    kind: ValueKind::Call(id, values),
                },
                lets,
            ))
        })();
        self.scopes.pop();
        checked.map(|value| Self::wrap_lets(value, vec![(slot, receiver)]))
    }

    pub(super) fn check_infix_call(
        &mut self,
        source: &Expr,
        name: &str,
        argument: &Expr,
        offset: usize,
    ) -> Result<Value> {
        let receiver = self.value(source)?;
        let mut methods = self
            .members
            .values()
            .filter(|m| {
                m.source_name() == name
                    && self
                        .receiver
                        .as_ref()
                        .is_some_and(|own| own.ty == receiver.ty)
                    && m.library.as_ref().is_some_and(|l| l.infix)
            })
            .cloned()
            .collect::<Vec<_>>();
        if self
            .object_owner
            .is_some_and(|owner| receiver.ty.base() == &Type::Java(owner.into()))
        {
            if let Some(methods) = self.object_methods.get(name) {
                let candidates = methods
                    .iter()
                    .filter(|method| method.library.as_ref().is_some_and(|library| library.infix))
                    .cloned()
                    .collect::<Vec<_>>();
                if !candidates.is_empty() {
                    self.select_method(&candidates, std::slice::from_ref(argument), offset)?;
                    return self.object_call(
                        name,
                        std::slice::from_ref(argument),
                        Some(receiver),
                        offset,
                    );
                }
            }
        }
        if let Some(owner) = library::type_owner(&receiver.ty) {
            if let Ok(inherited) = self.classpath.methods(owner) {
                if let Ok(class) = self.classpath.load(owner) {
                    methods.extend(
                        inherited
                            .into_iter()
                            .filter(|m| {
                                m.source_name() == name
                                    && m.access & 8 == 0
                                    && m.library.as_ref().is_some_and(|l| l.infix)
                                    && !relations::projected_input(
                                        m,
                                        &receiver.ty,
                                        &class,
                                        self.classpath,
                                    )
                            })
                            .map(|m| {
                                library::specialize_receiver(
                                    m,
                                    &receiver.ty,
                                    &class,
                                    self.classpath,
                                )
                            }),
                    );
                }
            }
        }
        let mut identities = HashSet::new();
        methods.retain(|method| {
            identities.insert((
                method.owner.clone(),
                method.name.clone(),
                method.descriptor.clone(),
            ))
        });
        if !methods.is_empty() {
            match self.select_method(&methods, std::slice::from_ref(argument), offset) {
                Ok(method) => {
                    return self.member_call(
                        method,
                        receiver,
                        std::slice::from_ref(argument),
                        offset,
                    );
                }
                Err(error) if error.message.contains("Ambiguous") => return Err(error),
                Err(_) => (),
            }
        }
        if self.function_names.iter().enumerate().any(|(id, n)| {
            n == name && self.extensions.contains(&id) && self.source_infix.contains(&id)
        }) {
            let attempt_receiver = self.value(source)?;
            let before = self.next;
            let count = self.lambdas.borrow().len();
            match self.source_extension_call(
                attempt_receiver,
                name,
                std::slice::from_ref(argument),
                &[],
                true,
                offset,
            ) {
                Ok(value) => {
                    if self.library_candidates(name).iter().any(|method| {
                        method
                            .library
                            .as_ref()
                            .is_some_and(|library| library.infix && library.receiver.is_some())
                    }) {
                        return Err(Diagnostic::new(
                            offset,
                            "Overload priority between source and imported infix declarations is unsupported",
                        ));
                    }
                    return Ok(value);
                }
                Err(error) if error.message.contains("Ambiguous") => return Err(error),
                Err(_) => {
                    self.next = before;
                    self.lambdas.borrow_mut().truncate(count);
                }
            }
        }
        self.resolved_extension_call(
            receiver,
            name,
            std::slice::from_ref(argument),
            &[],
            ExtensionKind::Infix,
            offset,
        )
    }
}

impl Checker<'_> {
    pub(super) fn operator_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        if receiver.ty.is_nullable() {
            return Err(Diagnostic::new(
                offset,
                "Operator receiver must be non-null",
            ));
        }
        let mut member_failure = None;
        let mut missing_operator = false;
        if let Some(owner) = library::type_owner(&receiver.ty) {
            if let Ok(class) = self
                .classes
                .get(owner)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| self.classpath.load(owner))
            {
                let inherited =
                    library::ordinary_member_methods(self.classpath, owner, name, args.len())
                        .unwrap_or_else(|_| class.methods.clone());
                let mut eligible = Vec::new();
                let mut ordinary = Vec::new();
                for method in inherited
                    .into_iter()
                    .filter(|m| m.source_name() == name && m.access & 9 == 1)
                {
                    let kotlin = self
                        .classes
                        .get(&method.owner)
                        .map(|c| c.kotlin_class)
                        .or_else(|| {
                            self.classpath
                                .load(&method.owner)
                                .ok()
                                .map(|c| c.kotlin_class)
                        })
                        .unwrap_or(class.kotlin_class);
                    if kotlin && !method.library.as_ref().is_some_and(|l| l.operator) {
                        ordinary.push(method);
                    } else {
                        eligible.push(method);
                    }
                }
                if !ordinary.is_empty() && self.select_method(&ordinary, args, offset).is_ok() {
                    missing_operator = true;
                }
                let methods = eligible
                    .into_iter()
                    .filter(|m| {
                        !relations::projected_input(m, &receiver.ty, &class, self.classpath)
                    })
                    .map(|m| library::specialize_receiver(m, &receiver.ty, &class, self.classpath))
                    .collect::<Vec<_>>();
                if !methods.is_empty() {
                    match self.select_method(&methods, args, offset) {
                        Ok(method) => return self.member_call(method, receiver, args, offset),
                        Err(error) if error.message.contains("Ambiguous") => return Err(error),
                        Err(error) => member_failure = Some(error),
                    }
                }
            }
        }
        if self
            .function_names
            .iter()
            .enumerate()
            .any(|(id, candidate)| {
                candidate == name
                    && self.extensions.contains(&id)
                    && self.source_operator.contains(&id)
            })
        {
            return self.source_operator_call(receiver, name, args, offset);
        }
        if self
            .function_names
            .iter()
            .enumerate()
            .any(|(id, candidate)| candidate == name && self.extensions.contains(&id))
        {
            let next = self.next;
            let initialized = self.initialized.clone();
            let refinements = self.refinements.clone();
            let count = self.lambdas.borrow().len();
            let ordinary =
                self.source_extension_call(receiver.clone(), name, args, &[], false, offset);
            self.next = next;
            self.initialized = initialized;
            self.refinements = refinements;
            self.lambdas.borrow_mut().truncate(count);
            missing_operator |= ordinary.is_ok();
        }
        let result = self.resolved_extension_call(
            receiver,
            name,
            args,
            &[],
            ExtensionKind::Operator,
            offset,
        );
        match result {
            Err(_) if missing_operator => Err(Diagnostic::new(
                offset,
                format!("The operator modifier is required on {name}"),
            )),
            Err(_) if member_failure.is_some() => Err(member_failure.unwrap()),
            result => result,
        }
    }

    fn source_operator_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let candidates = self
            .function_names
            .iter()
            .enumerate()
            .filter_map(|(id, n)| {
                (n == name && self.extensions.contains(&id) && self.source_operator.contains(&id))
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let binding = format!("<operator{slot}>");
        self.scopes.push(HashMap::from([(
            binding.clone(),
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let mut all = vec![Expr {
            offset,
            kind: ExprKind::Name(binding),
        }];
        all.extend_from_slice(args);
        let result = (|| {
            let id = self.select_function_types(&candidates, &all, &[], offset)?;
            let (params, result) = self.specialize_source(id, &all, &[], offset)?;
            let (values, lets, mask) =
                self.arguments(&params, &vec![false; params.len()], &all, true, offset)?;
            if mask != 0 {
                return Err(Diagnostic::new(
                    offset,
                    "Operator default arguments are unsupported",
                ));
            }
            Ok(Self::wrap_lets(
                Value {
                    ty: result,
                    kind: ValueKind::Call(id, values),
                },
                lets,
            ))
        })();
        self.scopes.pop();
        result.map(|value| Self::wrap_lets(value, vec![(slot, receiver)]))
    }

    pub(super) fn check_contains(
        &mut self,
        value: Value,
        container: &Expr,
        offset: usize,
    ) -> Result<Value> {
        let receiver = self.value(container)?;
        let slot = self.allocate_typed_slot(&value.ty, offset)?;
        let name = format!("<contains{slot}>");
        self.scopes.push(HashMap::from([(
            name.clone(),
            Local {
                slot,
                ty: value.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let argument = Expr {
            offset,
            kind: ExprKind::Name(name),
        };
        let receiver_slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let receiver_local = Value {
            ty: receiver.ty.clone(),
            kind: ValueKind::Local(receiver_slot),
        };
        let result = self.operator_call(receiver_local, "contains", &[argument], offset);
        self.scopes.pop();
        let result = result?;
        self.require(&result, Type::Boolean, offset)?;
        Ok(Self::wrap_lets(
            result,
            vec![(receiver_slot, receiver), (slot, value)],
        ))
    }

    pub(super) fn check_indices(
        &mut self,
        source: &Expr,
        indices: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let array = self.value(source)?;
        if array.ty.is_nullable() {
            return Err(Diagnostic::new(
                offset,
                "Nullable indexing requires a not-null assertion",
            ));
        }
        let element = match array.ty.base() {
            Type::String => Some(Type::Char),
            array if array.array_element().is_some() => array.array_element(),
            _ => None,
        };
        if let Some(element) = element {
            if matches!(element, Type::Unit | Type::Null) {
                return Err(Diagnostic::new(offset, "Unsupported array element type"));
            }
            if indices.len() != 1 {
                return Err(Diagnostic::new(offset, "Array indexing requires one index"));
            }
            let index = self.expected_value(&indices[0], &Type::Int)?;
            return Ok(Value {
                ty: element,
                kind: ValueKind::Index(Box::new(array), Box::new(index)),
            });
        }
        self.operator_call(array, "get", indices, offset)
    }

    pub(super) fn check_indices_assignment(
        &mut self,
        source: &Expr,
        indices: &[Expr],
        expression: &Expr,
        offset: usize,
    ) -> Result<Statement> {
        let array = self.value(source)?;
        if array.ty.is_nullable() {
            return Err(Diagnostic::new(
                offset,
                "Nullable indexing requires a not-null assertion",
            ));
        }
        let element = match array.ty.base() {
            array if array.array_element().is_some() => array.array_element(),
            Type::List(_) => {
                return Err(Diagnostic::new(
                    offset,
                    "Read-only List does not support indexed assignment",
                ));
            }
            _ => None,
        };
        if let Some(element) = element {
            if matches!(element, Type::Unit | Type::Null) {
                return Err(Diagnostic::new(offset, "Unsupported array element type"));
            }
            if indices.len() != 1 {
                return Err(Diagnostic::new(
                    offset,
                    "Array assignment requires one index",
                ));
            }
            let index = self.expected_value(&indices[0], &Type::Int)?;
            let value = self.expected_value(expression, &element)?;
            return Ok(Statement::IndexStore(array, index, value));
        }
        let mut args = indices.to_vec();
        args.push(expression.clone());
        Ok(Statement::Expression(
            self.operator_call(array, "set", &args, offset)?,
        ))
    }

    pub(super) fn array_constructor(
        &mut self,
        types: &[Type],
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        self.array_constructor_kind(types, args, offset, None)
    }

    pub(super) fn primitive_array_constructor(
        &mut self,
        array: Type,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let element = array
            .primitive_array_element()
            .ok_or_else(|| Diagnostic::new(offset, "Unsupported primitive array constructor"))?;
        self.array_constructor_kind(&[element], args, offset, Some(array))
    }

    fn array_constructor_kind(
        &mut self,
        types: &[Type],
        args: &[Expr],
        offset: usize,
        physical: Option<Type>,
    ) -> Result<Value> {
        if types.len() > 1 || args.len() != 2 {
            return Err(Diagnostic::new(
                offset,
                "Array construction requires size and initializer",
            ));
        }
        fn stable_initializer(checker: &Checker<'_>, expr: &Expr) -> bool {
            match &expr.kind {
                ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. } => true,
                ExprKind::Labeled(_, inner) => stable_initializer(checker, inner),
                ExprKind::Name(name) => checker.local(name).is_some_and(|local| {
                    !local.mutable
                        && matches!(
                            local.ty.base(),
                            Type::Function { .. } | Type::ReceiverFunction { .. }
                        )
                }),
                _ => false,
            }
        }
        if !stable_initializer(self, &args[1]) {
            return Err(Diagnostic::new(
                offset,
                "Array initializer evaluation order has a specification/oracle discrepancy; effectful initializer expressions are unsupported",
            ).with_kind(crate::DiagnosticKind::UnsupportedLanguage, "ARRAY_INITIALIZER_EVALUATION_UNSUPPORTED"));
        }
        let size = self.expected_value(&args[0], &Type::Int)?;
        let element = if let Some(ty) = types.first() {
            self.resolve(ty, offset)?
        } else {
            self.infer_array_element(&args[1], offset)?
        };
        if matches!(element, Type::Unit | Type::Null) {
            return Err(Diagnostic::new(
                offset,
                "Array element type requires an explicit supported type",
            ));
        }
        fn reifiable(ty: &Type) -> bool {
            match ty {
                Type::Parameter { .. } => false,
                Type::Array(inner) | Type::Nullable(inner) | Type::Platform(inner) => {
                    reifiable(inner)
                }
                _ => true,
            }
        }
        if !reifiable(&element) {
            return Err(Diagnostic::new(
                offset,
                "Array construction requires a reified element type",
            ));
        }
        let initializer = self.expected_value(
            &args[1],
            &Type::Function {
                params: vec![Type::Int],
                result: Box::new(element.clone()),
            },
        )?;
        let array = physical.unwrap_or_else(|| Type::Array(Box::new(element)));
        Ok(Value {
            ty: array.clone(),
            kind: ValueKind::ArrayInitialize {
                array,
                size: Box::new(size),
                initializer: Box::new(initializer),
                array_slot: self.allocate_slot(offset)?,
                index_slot: self.allocate_slot(offset)?,
                initializer_slot: self.allocate_slot(offset)?,
            },
        })
    }

    fn infer_array_element(&mut self, initializer: &Expr, offset: usize) -> Result<Type> {
        if let ExprKind::Labeled(_, inner) = &initializer.kind {
            return self.infer_array_element(inner, offset);
        }
        let (params, body) = match &initializer.kind {
            ExprKind::Lambda(value) => (
                None,
                vec![syntax::Statement {
                    offset,
                    kind: StatementKind::Expression(value.as_ref().clone()),
                }],
            ),
            ExprKind::LambdaWithParams { params, body } => (params.clone(), body.clone()),
            _ => {
                let next = self.next;
                let initialized = self.initialized.clone();
                let refinements = self.refinements.clone();
                let cells = self.cells.clone();
                let cell_names = self.cell_names.clone();
                let scopes = self.scopes.len();
                let loop_depth = self.loop_depth;
                let count = self.lambdas.borrow().len();
                let inferred = self.value(initializer).and_then(|value| match value.ty {
                    Type::Function { result, .. } | Type::ReceiverFunction { result, .. } => {
                        Ok(*result)
                    }
                    _ => Err(Diagnostic::new(
                        offset,
                        "Array initializer must be a function",
                    )),
                });
                self.next = next;
                self.initialized = initialized;
                self.refinements = refinements;
                self.cells = cells;
                self.cell_names = cell_names;
                self.scopes.truncate(scopes);
                self.loop_depth = loop_depth;
                self.lambdas.borrow_mut().truncate(count);
                return inferred;
            }
        };
        let scopes = self.scopes.len();
        let next = self.next;
        let loop_depth = self.loop_depth;
        let initialized = self.initialized.clone();
        let refinements = self.refinements.clone();
        let cells = self.cells.clone();
        let cell_names = self.cell_names.clone();
        let saved = self.lambdas.borrow().len();
        let allow_return = self.allow_return;
        let inline_result = self.inline_result.take();
        let nonlocal_result = self.nonlocal_result.take();
        self.allow_return = false;
        self.scopes.push(HashMap::new());
        let result = (|| {
            let name = params
                .as_ref()
                .and_then(|p| p.first())
                .map_or("it", |p| p.name.as_str());
            self.declare(name, Type::Int, false, offset)?;
            let Some((last, statements)) = body.split_last() else {
                return Err(Diagnostic::new(
                    offset,
                    "Array initializer must produce a value",
                ));
            };
            self.block_scoped(statements, false)?;
            if let StatementKind::Expression(expr) = &last.kind {
                self.value(expr).map(|v| v.ty)
            } else {
                Err(Diagnostic::new(
                    offset,
                    "Array initializer needs a final result expression",
                ))
            }
        })();
        self.scopes.truncate(scopes);
        self.next = next;
        self.loop_depth = loop_depth;
        self.initialized = initialized;
        self.refinements = refinements;
        self.cells = cells;
        self.cell_names = cell_names;
        self.lambdas.borrow_mut().truncate(saved);
        self.allow_return = allow_return;
        self.inline_result = inline_result;
        self.nonlocal_result = nonlocal_result;
        result
    }
}

impl Checker<'_> {
    pub(super) fn receiver_property(&mut self, name: &str, offset: usize) -> Result<Value> {
        if self.receiver.is_none() {
            return Err(Diagnostic::new(offset, format!("Unresolved name: {name}")));
        }
        self.value(&Expr {
            offset,
            kind: ExprKind::Property(
                Box::new(Expr {
                    offset,
                    kind: ExprKind::Name("this".into()),
                }),
                name.into(),
            ),
        })
    }

    pub(super) fn implicit_receiver_call(
        &mut self,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let receiver = self.receiver_value(offset)?;
        let next = self.next;
        let initialized = self.initialized.clone();
        let refinements = self.refinements.clone();
        let count = self.lambdas.borrow().len();
        let result = self.java_call(receiver, name, args, offset);
        if result.is_err() {
            self.next = next;
            self.initialized = initialized;
            self.refinements = refinements;
            self.lambdas.borrow_mut().truncate(count);
        }
        result
    }

    pub(super) fn receiver_function_call(
        &mut self,
        source: &Expr,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let function = self.value(&Expr {
            offset,
            kind: ExprKind::Name(name.into()),
        })?;
        let Type::ReceiverFunction {
            receiver,
            params,
            result,
        } = function.ty.base()
        else {
            return Err(Diagnostic::new(
                offset,
                "Local function value has no receiver",
            ));
        };
        let source = self.expected_value(source, receiver)?;
        if args.len() != params.len() {
            return Err(Diagnostic::new(
                offset,
                "Receiver function argument count mismatch",
            ));
        }
        let mut values = vec![source];
        for (arg, ty) in args.iter().zip(params) {
            values.push(self.expected_value(arg, ty)?);
        }
        let result = result.as_ref().clone();
        Ok(Value {
            ty: result,
            kind: ValueKind::FunctionInvoke {
                function: Box::new(function),
                args: values,
            },
        })
    }
}

impl Checker<'_> {
    pub(super) fn checked_throw(&mut self, expr: &Expr, offset: usize) -> Result<Statement> {
        let value = if let ExprKind::Call(target, args) = &expr.kind {
            if let ExprKind::Name(name) = &target.kind {
                if self.local(name).is_none()
                    && !self.functions.contains_key(name)
                    && !self.captured.contains_key(name)
                    && !self.constants.contains_key(name)
                    && !self.fields.contains_key(name)
                    && !self.companion_fields.contains_key(name)
                    && !self.object_fields.contains_key(name)
                    && !self.object_methods.contains_key(name)
                    && !self.members.values().any(|method| {
                        method.source_name() == name
                            || method.source_property.as_deref() == Some(name)
                    })
                    && !self
                        .receiver
                        .as_ref()
                        .and_then(|receiver| library::type_owner(&receiver.ty))
                        .is_some_and(|owner| {
                            self.classpath.methods(owner).is_ok_and(|methods| {
                                methods.iter().any(|method| {
                                    method.source_name() == name && method.access & 8 == 0
                                })
                            })
                        })
                {
                    if let Ok(ty) = self.resolve(&Type::Java(name.clone()), offset) {
                        if let Some(owner) = library::type_owner(&ty) {
                            let class = self
                                .classes
                                .get(owner)
                                .cloned()
                                .map(Ok)
                                .unwrap_or_else(|| self.classpath.load(owner))?;
                            let candidates = class
                                .methods
                                .iter()
                                .filter(|method| method.name == "<init>" && method.access & 1 != 0)
                                .cloned()
                                .collect::<Vec<_>>();
                            let method = self.select_method(&candidates, args, offset)?;
                            let (params, defaults) = Self::method_parameters(&method)?;
                            let value = self.constructor_call(
                                owner,
                                &params,
                                &defaults,
                                args,
                                self.method_named(&method),
                                offset,
                            )?;
                            return self.throw_value(value, offset);
                        }
                    }
                }
            }
            self.value(expr)?
        } else {
            self.value(expr)?
        };
        self.throw_value(value, offset)
    }

    fn throw_value(&self, value: Value, offset: usize) -> Result<Statement> {
        if value.ty.is_nullable()
            || !self.subtype(&value.ty, &Type::Java("java/lang/Throwable".into()))
        {
            return Err(Diagnostic::new(
                offset,
                "Thrown value must be a non-null Throwable",
            ));
        }
        Ok(Statement::ThrowValue(value))
    }
}

#[cfg(test)]
mod infix_tests {
    #[test]
    fn source_member_views_preserve_distinct_infix_overloads() {
        crate::compile("class C(val value:Int) { infix fun choose(other:Int):Int=value+other; infix fun choose(other:Long):Long=other; fun f():Int=this choose 3 }", "Infix.kt").unwrap();
        let error=crate::compile("class C { infix fun choose(other:String?):Int=1; infix fun choose(other:C?):Int=2; fun f():Int=this choose null }", "AmbiguousInfix.kt").err().unwrap();
        assert!(error.message.contains("Ambiguous"), "{}", error.message);
    }
}
