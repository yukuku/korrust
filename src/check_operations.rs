use super::*;

pub(super) enum ExtensionKind {
    Ordinary,
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
            Type::IntArray => Type::Int,
            Type::ByteArray => Type::Byte,
            Type::CharArray | Type::String => Type::Char,
            Type::StringArray => Type::String,
            _ => {
                return Err(Diagnostic::new(
                    offset,
                    "Iteration supports primitive arrays, String, and non-null Iterable types with resolvable element metadata",
                ));
            }
        };
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
        let local = self.local(name);
        let cell = self.cell_binding(name);
        let ty = if let Some(local) = &local {
            if !local.mutable {
                return Err(Diagnostic::new(
                    offset,
                    format!("Cannot reassign val or parameter: {name}"),
                ));
            }
            if !self.initialized.contains(&local.slot) {
                return Err(Diagnostic::new(
                    offset,
                    "Variable must be initialized before use",
                ));
            }
            local.ty.clone()
        } else if let Some((_, ty)) = &cell {
            ty.clone()
        } else {
            if self
                .members
                .values()
                .any(|m| m.source_property.as_deref() == Some(name))
            {
                return Err(Diagnostic::new(
                    offset,
                    "Property assignments in method bodies are not supported",
                ));
            }
            return Err(Diagnostic::new(offset, format!("Unresolved local: {name}")));
        };
        if !matches!(ty, Type::Int | Type::Long | Type::Float) {
            return Err(Diagnostic::new(
                offset,
                "Numeric compound assignment supports only mutable Int, Long, and Float locals",
            ));
        }
        let binary = Expr {
            offset,
            kind: ExprKind::Binary(
                op.trim_end_matches('=').into(),
                Box::new(Expr {
                    offset,
                    kind: ExprKind::Name(name.into()),
                }),
                Box::new(right.clone()),
            ),
        };
        let value = self.value(&binary)?;
        let value = self.coerce(value, ty.clone(), offset)?;
        if let Some((cell, _)) = cell {
            self.cell_store(cell, ty, value, offset)
        } else {
            let slot = local.unwrap().slot;
            self.refinements.remove(&slot);
            Ok(Statement::Store(slot, value))
        }
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
                | Type::IntArray
                | Type::ByteArray
                | Type::CharArray
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
                return self.range_contains(left, range, condition.offset);
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
        if let ExprKind::Boolean(truth) = &source.kind {
            self.refine(&conditions[0], *truth);
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
        let promote = |value: Value| -> Result<Value> {
            if value.ty == Type::Float {
                return Ok(value);
            }
            if !matches!(value.ty, Type::Int | Type::Long | Type::Byte) {
                return Err(Diagnostic::new(
                    offset,
                    "Float operations require numeric operands",
                ));
            }
            Ok(Value {
                ty: Type::Float,
                kind: ValueKind::Convert(Type::Float, Box::new(value)),
            })
        };
        let left = promote(left)?;
        let right = promote(right)?;
        Ok(Value {
            ty: if matches!(op, "<" | ">" | "<=" | ">=") {
                Type::Boolean
            } else {
                Type::Float
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
        let descriptor = if float(&left.ty) && float(&right.ty) {
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
            method.access & 9 == 9
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
        let package_classes = self.classpath.package_classes(package)?;
        let local = package_classes
            .iter()
            .flat_map(|c| &c.methods)
            .filter(matches)
            .cloned()
            .collect::<Vec<_>>();
        let mut implicit = Vec::new();
        for package in ["kotlin/ranges", "kotlin/collections", "kotlin/text"] {
            let classes = self.classpath.package_classes(package)?;
            implicit.extend(
                classes
                    .iter()
                    .flat_map(|c| &c.methods)
                    .filter(matches)
                    .cloned(),
            );
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
                m.name == name
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
                                m.name == name
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
