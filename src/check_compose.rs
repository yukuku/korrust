use super::*;

fn compose_effect(value: &Value) -> bool {
    matches!(&value.kind, ValueKind::JavaCall { method, .. }
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
            | ValueKind::ObjectInstance(_)
            | ValueKind::ClassLiteral(_) => true,
            ValueKind::Local(slot) => lets
                .iter()
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
                .is_ok_and(|fields| fields.iter().any(|field| field.access & 0x18 == 0x18)),
            ValueKind::JavaCall { method, args } => {
                method
                    .library
                    .as_ref()
                    .is_some_and(|library| library.stable_result)
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
        for (index, value) in values.iter().enumerate() {
            if mask & (1u64 << index) == 0 && self.compose_static(value, &lets, 0) {
                changed[index / 10] |= 6 << (3 * (index % 10));
            }
        }
        values.push(Self::local_receiver(&composer));
        for word in changed {
            values.push(Value {
                ty: Type::Int,
                kind: ValueKind::Int(word),
            });
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
