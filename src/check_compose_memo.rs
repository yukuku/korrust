use super::*;

impl Checker<'_> {
    fn compose_identity_capture(&self, ty: &Type, depth: usize) -> Result<Option<bool>> {
        if depth > 16 {
            return Ok(None);
        }
        match ty.base() {
            Type::Nullable(inner) => self.compose_identity_capture(inner, depth + 1),
            Type::Byte
            | Type::Short
            | Type::Int
            | Type::Long
            | Type::Float
            | Type::Double
            | Type::Char
            | Type::Boolean
            | Type::String => Ok(Some(false)),
            Type::Java(owner) => {
                let class = self.classpath.load(owner)?;
                if class.unsupported
                    || !class.kotlin_interface
                    || !class.library.as_ref().is_some_and(|library| {
                        library.compose_stability
                            == crate::classpath::compose_abi::Stability::UnmarkedInterface
                    })
                {
                    return Ok(None);
                }
                for parent in &class.interfaces {
                    if self.compose_identity_capture(&Type::Java(parent.clone()), depth + 1)?
                        != Some(true)
                    {
                        return Ok(None);
                    }
                }
                Ok(Some(true))
            }
            _ => Ok(None),
        }
    }

    fn compose_runtime_call(
        &self,
        name: &str,
        descriptor: &str,
        args: Vec<Value>,
        ty: Type,
        offset: usize,
    ) -> Result<Value> {
        let owner = crate::classpath::compose_abi::COMPOSER;
        let method = self
            .classpath
            .methods(owner)?
            .into_iter()
            .find(|method| {
                method.name == name && method.descriptor == descriptor && method.access & 9 == 1
            })
            .ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    "Pinned Compose memoization runtime method is unavailable",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "COMPOSE_RUNTIME_ABI_UNSUPPORTED",
                )
            })?;
        Ok(Value {
            ty,
            kind: ValueKind::MemberCall {
                method,
                interface: true,
                args,
            },
        })
    }

    fn memoized_compose_function(&mut self, value: &Value, offset: usize) -> Result<Value> {
        let ValueKind::Lambda { owner, captures } = &value.kind else {
            unreachable!()
        };
        let valid = self
            .lambdas
            .borrow()
            .iter()
            .find(|lambda| lambda.owner == *owner)
            .is_some_and(|lambda| {
                lambda.composable_key.is_none()
                    && lambda.interface.is_none()
                    && lambda.anonymous_methods.is_none()
                    && lambda.anonymous_super.is_none()
                    && !lambda.function.suspend_
                    && !lambda.function.extension_receiver
                    && lambda.function.params.len() <= 1
                    && lambda.function.result == Type::Unit
            });
        let unsupported = || {
            Diagnostic::new(
                offset,
                "Unsupported function creation or capture stability in Compose callback",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "COMPOSE_BODY_LAMBDA_UNSUPPORTED",
            )
        };
        if !valid {
            return Err(unsupported());
        }
        let composer = Self::local_receiver(self.composer.as_ref().ok_or_else(unsupported)?);
        let mut lets = Vec::new();
        let mut arguments = Vec::new();
        let mut invalid = Value {
            ty: Type::Boolean,
            kind: ValueKind::Boolean(false),
        };
        for capture in captures {
            let identity = self
                .compose_identity_capture(&capture.ty, 0)?
                .ok_or_else(unsupported)?;
            let slot = self.allocate_typed_slot(&capture.ty, offset)?;
            lets.push((slot, capture.clone()));
            let argument = Value {
                ty: capture.ty.clone(),
                kind: ValueKind::Local(slot),
            };
            let descriptor = if identity || capture.ty.is_reference() {
                "(Ljava/lang/Object;)Z".into()
            } else {
                format!("({})Z", capture.ty.descriptor())
            };
            let changed = self.compose_runtime_call(
                if identity {
                    "changedInstance"
                } else {
                    "changed"
                },
                &descriptor,
                vec![composer.clone(), argument.clone()],
                Type::Boolean,
                offset,
            )?;
            invalid = Value {
                ty: Type::Boolean,
                kind: ValueKind::Binary("or".into(), Box::new(invalid), Box::new(changed)),
            };
            arguments.push(argument);
        }
        let invalid_slot = self.allocate_typed_slot(&Type::Boolean, offset)?;
        lets.push((invalid_slot, invalid));
        let object = Type::Nullable(Box::new(Type::Java("java/lang/Object".into())));
        let remembered = self.compose_runtime_call(
            "rememberedValue",
            "()Ljava/lang/Object;",
            vec![composer.clone()],
            object.clone(),
            offset,
        )?;
        let old_slot = self.allocate_typed_slot(&object, offset)?;
        lets.push((old_slot, remembered));
        let old = Value {
            ty: object.clone(),
            kind: ValueKind::Local(old_slot),
        };
        let runtime = || {
            Diagnostic::new(
                offset,
                "Pinned Compose memoization companion is unavailable",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "COMPOSE_RUNTIME_ABI_UNSUPPORTED",
            )
        };
        if !self
            .classpath
            .field_candidates(crate::classpath::compose_abi::COMPOSER, "Companion")?
            .iter()
            .any(|field| {
                field.access & 0x19 == 0x19
                    && field.descriptor == "Landroidx/compose/runtime/Composer$Companion;"
            })
        {
            return Err(runtime());
        }
        let empty_method = self
            .classpath
            .methods("androidx/compose/runtime/Composer$Companion")?
            .into_iter()
            .find(|method| {
                method.name == "getEmpty"
                    && method.descriptor == "()Ljava/lang/Object;"
                    && method.access & 9 == 1
            })
            .ok_or_else(runtime)?;
        let args = vec![Value {
            ty: Type::Java("androidx/compose/runtime/Composer$Companion".into()),
            kind: ValueKind::StaticField {
                owner: crate::classpath::compose_abi::COMPOSER.into(),
                name: "Companion".into(),
            },
        }];
        let empty = Value {
            ty: object,
            kind: ValueKind::JavaCall {
                method: empty_method,
                args,
            },
        };
        let condition = Value {
            ty: Type::Boolean,
            kind: ValueKind::Binary(
                "||".into(),
                Box::new(Value {
                    ty: Type::Boolean,
                    kind: ValueKind::Local(invalid_slot),
                }),
                Box::new(Value {
                    ty: Type::Boolean,
                    kind: ValueKind::Binary("===".into(), Box::new(old.clone()), Box::new(empty)),
                }),
            ),
        };
        let slot = self.allocate_typed_slot(&value.ty, offset)?;
        let fresh = Value {
            ty: value.ty.clone(),
            kind: ValueKind::Local(slot),
        };
        let update = self.compose_runtime_call(
            "updateRememberedValue",
            "(Ljava/lang/Object;)V",
            vec![composer, fresh.clone()],
            Type::Unit,
            offset,
        )?;
        let create = Value {
            ty: value.ty.clone(),
            kind: ValueKind::Let {
                slot,
                value: Box::new(Value {
                    ty: value.ty.clone(),
                    kind: ValueKind::Lambda {
                        owner: owner.clone(),
                        captures: arguments,
                    },
                }),
                body: Box::new(Value {
                    ty: value.ty.clone(),
                    kind: ValueKind::Block(vec![Statement::Expression(update)], Box::new(fresh)),
                }),
            },
        };
        let reuse = Value {
            ty: value.ty.clone(),
            kind: ValueKind::Cast {
                value: Box::new(old),
                target: value.ty.clone(),
                safe: false,
            },
        };
        Ok(Self::wrap_lets(
            Value {
                ty: value.ty.clone(),
                kind: ValueKind::If(Box::new(condition), Box::new(create), Box::new(reuse)),
            },
            lets,
        ))
    }

    pub(super) fn memoize_compose_functions(
        &mut self,
        body: &mut [Statement],
        owner: &str,
        offset: usize,
    ) -> Result<()> {
        let mut result = Ok(());
        ir_walk::statements(body, owner, &mut |value: &mut Value, _: &str| {
            if matches!(value.kind, ValueKind::Lambda { .. }) && result.is_ok() {
                match self.memoized_compose_function(value, offset) {
                    Ok(memo) => *value = memo,
                    Err(error) => result = Err(error),
                }
            }
        });
        result
    }
}
