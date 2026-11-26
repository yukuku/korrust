use super::*;

fn compose_effect(value: &Value) -> bool {
    matches!(&value.kind, ValueKind::Lambda { .. })
        || matches!(&value.kind, ValueKind::FunctionInvoke { function, .. } if function.ty.base() == &Type::ComposableUnitFunction)
        || matches!(&value.kind, ValueKind::JavaCall { method, .. }
        if method.library.as_ref().is_some_and(|library| library.compose.is_some()))
}

fn value_has_compose(value: &mut Value) -> bool {
    let mut found = false;
    ir_walk::value(value, "", &mut |value: &mut Value, _: &str| {
        found |= compose_effect(value);
    });
    found
}

#[derive(Default)]
struct Groups {
    calls: bool,
    conditional_call: bool,
    conditional_exit: bool,
}

impl ir_walk::Visitor for Groups {
    fn value(&mut self, value: &mut Value, _: &str) {
        self.calls |= compose_effect(value);
        if matches!(
            value.kind,
            ValueKind::If(..) | ValueKind::Try { .. } | ValueKind::Abrupt(_)
        ) {
            self.conditional_call |= value_has_compose(value);
        }
    }

    fn statement(&mut self, statement: &mut Statement, _: &str) {
        if matches!(
            statement,
            Statement::If(..)
                | Statement::While(..)
                | Statement::DoWhile(..)
                | Statement::ForUntil { .. }
                | Statement::ForEach { .. }
                | Statement::ForIterator { .. }
                | Statement::Labeled { .. }
        ) {
            let mut calls = false;
            struct Exit(bool);
            impl ir_walk::Visitor for Exit {
                fn value(&mut self, _: &mut Value, _: &str) {}
                fn statement(&mut self, statement: &mut Statement, _: &str) {
                    self.0 |= matches!(
                        statement,
                        Statement::Return(_)
                            | Statement::NonLocalReturn(_)
                            | Statement::Break
                            | Statement::Continue
                            | Statement::BreakLabel(_)
                            | Statement::ContinueLabel(_)
                    );
                }
            }
            ir_walk::statements(
                std::slice::from_mut(statement),
                "",
                &mut |value: &mut Value, _: &str| {
                    calls |= compose_effect(value);
                },
            );
            let mut exits = Exit(false);
            ir_walk::statements(std::slice::from_mut(statement), "", &mut exits);
            self.conditional_call |= calls;
            self.conditional_exit |= exits.0;
        }
    }
}

impl Checker<'_> {
    pub(super) fn validate_composable_groups(
        &self,
        body: &mut [Statement],
        offset: usize,
    ) -> Result<()> {
        let mut groups = Groups::default();
        ir_walk::statements(body, "", &mut groups);
        if groups.conditional_call || groups.calls && groups.conditional_exit {
            return Err(Diagnostic::new(
                offset,
                "Composable control flow requires unsupported group lowering",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "COMPOSE_CONTROL_FLOW_UNSUPPORTED",
            ));
        }
        Ok(())
    }

    fn compose_stable_owner(&self, owner: &str, depth: usize) -> bool {
        if depth > 16 {
            return false;
        }
        let Ok(class) = self.classpath.load(owner) else {
            return false;
        };
        if class.unsupported {
            return false;
        }
        class.library.as_ref().is_some_and(|library| {
            library.compose_stability == crate::classpath::compose_abi::Stability::Stable
        }) || class
            .interfaces
            .iter()
            .chain(class.super_name.iter())
            .any(|parent| {
                parent != "java/lang/Object" && self.compose_stable_owner(parent, depth + 1)
            })
    }

    fn compose_stable_type(&self, ty: &Type, depth: usize) -> bool {
        if depth > 16 {
            return false;
        }
        match ty.base() {
            Type::Nullable(inner) => self.compose_stable_type(inner, depth + 1),
            Type::Byte
            | Type::Short
            | Type::Int
            | Type::Long
            | Type::Float
            | Type::Double
            | Type::Boolean
            | Type::Char
            | Type::String
            | Type::Unit => true,
            Type::Java(owner) => self.compose_stable_owner(owner, 0),
            Type::Parameterized { name, .. } => self.compose_stable_owner(name, 0),
            _ => false,
        }
    }

    fn compose_static(&self, value: &Value, lets: &[(u8, Value)], depth: usize) -> bool {
        if depth > 32 {
            return false;
        }
        let static_ = |value| self.compose_static(value, lets, depth + 1);
        match &value.kind {
            ValueKind::Int(_)
            | ValueKind::Long(_)
            | ValueKind::Float(_)
            | ValueKind::Double(_)
            | ValueKind::Boolean(_)
            | ValueKind::String(_)
            | ValueKind::Null
            | ValueKind::Unit
            | ValueKind::ClassLiteral(_) => true,
            ValueKind::ObjectInstance(owner) => self.compose_stable_owner(owner, 0),
            ValueKind::Lambda { captures, .. } => captures.is_empty(),
            ValueKind::Let { slot, value, body } => {
                if !static_(value) {
                    return false;
                }
                let mut bindings = lets.to_vec();
                bindings.push((*slot, value.as_ref().clone()));
                self.compose_static(body, &bindings, depth + 1)
            }
            ValueKind::Local(slot) => lets
                .iter()
                .rev()
                .find(|(bound, _)| bound == slot)
                .is_some_and(|(_, value)| static_(value)),
            ValueKind::Cast { value, .. }
            | ValueKind::Box(value)
            | ValueKind::Unbox(value)
            | ValueKind::Convert(_, value)
            | ValueKind::Negate(value)
            | ValueKind::Not(value) => static_(value),
            ValueKind::Binary(_, left, right)
            | ValueKind::Compare(left, right)
            | ValueKind::StringConcat(left, right)
                if matches!(
                    left.ty.base(),
                    Type::Int
                        | Type::Long
                        | Type::Float
                        | Type::Double
                        | Type::Boolean
                        | Type::Char
                        | Type::String
                ) && matches!(
                    right.ty.base(),
                    Type::Int
                        | Type::Long
                        | Type::Float
                        | Type::Double
                        | Type::Boolean
                        | Type::Char
                        | Type::String
                ) =>
            {
                static_(left) && static_(right)
            }
            ValueKind::StaticField { owner, name }
            | ValueKind::JavaField {
                owner,
                name,
                receiver: None,
                ..
            } => self
                .classpath
                .field_candidates(owner, name)
                .is_ok_and(|fields| {
                    fields.iter().any(|field| {
                        field.access & 0x18 == 0x18
                            && (matches!(
                                field.descriptor.as_str(),
                                "B" | "S"
                                    | "I"
                                    | "J"
                                    | "F"
                                    | "D"
                                    | "Z"
                                    | "C"
                                    | "Ljava/lang/String;"
                            ) || field
                                .descriptor
                                .strip_prefix('L')
                                .and_then(|descriptor| descriptor.strip_suffix(';'))
                                .is_some_and(|owner| self.compose_stable_owner(owner, 0)))
                    })
                }),
            ValueKind::JavaCall { method, args } => {
                method
                    .library
                    .as_ref()
                    .is_some_and(|library| library.stable_result)
                    && self.compose_stable_type(&value.ty, 0)
                    && args.iter().all(static_)
            }
            ValueKind::StaticCall {
                owner,
                name,
                descriptor,
                args,
            } => {
                let Some(original) = name.strip_suffix("$default") else {
                    return false;
                };
                if !self.compose_stable_type(&value.ty, 0) {
                    return false;
                }
                self.classpath.methods(owner).is_ok_and(|methods| {
                    methods.iter().any(|method| {
                        if method.name != original
                            || method.access & 8 == 0
                            || !method
                                .library
                                .as_ref()
                                .is_some_and(|library| library.stable_result)
                        {
                            return false;
                        }
                        let Some(split) = method.descriptor.find(')') else {
                            return false;
                        };
                        let Some(params) = &method.kotlin_params else {
                            return false;
                        };
                        let expected = format!(
                            "({}{}Ljava/lang/Object;){}",
                            &method.descriptor[1..split],
                            "I".repeat(params.len().div_ceil(32)),
                            &method.descriptor[split + 1..]
                        );
                        &expected == descriptor && args.iter().all(static_)
                    })
                })
            }
            _ => false,
        }
    }

    fn compose_runtime_stability(
        &self,
        value: &Value,
        lets: &[(u8, Value)],
        depth: usize,
        offset: usize,
    ) -> Result<Option<Value>> {
        if depth > 32 {
            return Err(Diagnostic::new(
                offset,
                "Compose argument stability expression is too deep",
            ));
        }
        match &value.kind {
            ValueKind::Int(_)
            | ValueKind::Long(_)
            | ValueKind::Float(_)
            | ValueKind::Double(_)
            | ValueKind::Boolean(_)
            | ValueKind::String(_)
            | ValueKind::Null
            | ValueKind::Unit
            | ValueKind::Lambda { .. } => return Ok(None),
            ValueKind::Cast {
                value, safe: false, ..
            }
            | ValueKind::Box(value)
            | ValueKind::Unbox(value) => {
                return self.compose_runtime_stability(value, lets, depth + 1, offset);
            }
            ValueKind::Let { slot, value, body } => {
                let mut bindings = lets.to_vec();
                bindings.push((*slot, value.as_ref().clone()));
                return self.compose_runtime_stability(body, &bindings, depth + 1, offset);
            }
            ValueKind::Local(slot) => {
                if let Some((_, bound)) = lets.iter().rev().find(|(bound, _)| bound == slot) {
                    return self.compose_runtime_stability(bound, lets, depth + 1, offset);
                }
            }
            _ => {}
        }
        let mut ty = value.ty.base();
        while let Type::Nullable(inner) = ty {
            ty = inner.base();
        }
        let owner = match ty {
            Type::Java(owner) | Type::Parameterized { name: owner, .. } => owner,
            _ => return Ok(None),
        };
        if self.compose_stable_owner(owner, 0) {
            return Ok(None);
        }
        let class = self.classpath.load(owner)?;
        match class
            .library
            .as_ref()
            .map(|library| library.compose_stability)
        {
            Some(crate::classpath::compose_abi::Stability::Runtime) => Ok(Some(Value {
                ty: Type::Int,
                kind: ValueKind::StaticField {
                    owner: owner.clone(),
                    name: "$stable".into(),
                },
            })),
            Some(crate::classpath::compose_abi::Stability::UnsupportedRuntime) => {
                Err(Diagnostic::new(
                    offset,
                    "Unsupported compiled Compose stability parameter scheme",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "COMPOSE_ARGUMENT_STABILITY_UNSUPPORTED",
                ))
            }
            _ => Ok(None),
        }
    }

    pub(super) fn composable_call(
        &mut self,
        method: JavaMethod,
        mut values: Vec<Value>,
        lets: Vec<(u8, Value)>,
        mask: u64,
        offset: usize,
    ) -> Result<Value> {
        let composer = self.composer.clone().ok_or_else(|| {
            Diagnostic::new(
                offset,
                "Composable binary calls require a supported composable body",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "COMPOSE_CALL_CONTEXT_UNSUPPORTED",
            )
        })?;
        let abi = method.library.as_ref().unwrap().compose.as_ref().unwrap();
        if let Some(target) = &abi.target {
            if self
                .composable_target
                .as_ref()
                .is_some_and(|previous| previous != target)
            {
                return Err(Diagnostic::new(
                    offset,
                    "Composable calls require incompatible appliers",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "COMPOSE_APPLIER_MISMATCH",
                ));
            }
            self.composable_target = Some(target.clone());
        }
        if mask != 0 && abi.default_words == 0 {
            return Err(Diagnostic::new(
                offset,
                "Composable call does not expose default masks",
            ));
        }
        let mut changed = vec![0i32; abi.changed_words];
        let mut stability = vec![Vec::new(); abi.changed_words];
        for (index, value) in values.iter().enumerate() {
            if mask & (1u64 << index) == 0 && self.compose_static(value, &lets, 0) {
                changed[index / 10] |= 6 << (3 * (index % 10));
            }
            if let Some(mut flags) = self.compose_runtime_stability(value, &lets, 0, offset)? {
                let shift = 3 * (index % 10);
                if shift != 0 {
                    flags = Value {
                        ty: Type::Int,
                        kind: ValueKind::Binary(
                            "shl".into(),
                            Box::new(flags),
                            Box::new(Value {
                                ty: Type::Int,
                                kind: ValueKind::Int(shift as i32),
                            }),
                        ),
                    };
                }
                stability[index / 10].push(flags);
            }
        }
        values.push(Self::local_receiver(&composer));
        for (word, dynamic) in changed.into_iter().zip(stability) {
            let mut word = Value {
                ty: Type::Int,
                kind: ValueKind::Int(word),
            };
            for flags in dynamic {
                word = Value {
                    ty: Type::Int,
                    kind: ValueKind::Binary("or".into(), Box::new(word), Box::new(flags)),
                };
            }
            values.push(word);
        }
        for index in 0..abi.default_words {
            values.push(Value {
                ty: Type::Int,
                kind: ValueKind::Int((mask >> (32 * index)) as i32),
            });
        }
        Ok(Self::wrap_lets(
            Value {
                ty: Type::Unit,
                kind: ValueKind::JavaCall {
                    method,
                    args: values,
                },
            },
            lets,
        ))
    }
}
