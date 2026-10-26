use super::*;

impl Checker<'_> {
    pub(super) fn callable_reference(&mut self, expr: &Expr, expected: &Type) -> Result<Value> {
        let Type::Function { params, .. } = expected.base() else {
            return Err(Diagnostic::new(
                expr.offset,
                "Callable references require a supported function expected type",
            ));
        };
        let ExprKind::CallableReference { receiver, name } = &expr.kind else {
            unreachable!();
        };
        let offset = expr.offset;
        let names = (0..params.len())
            .map(|index| format!("<referenceArg{index}>"))
            .collect::<Vec<_>>();
        let arguments = names
            .iter()
            .map(|name| Expr {
                offset,
                kind: ExprKind::Name(name.clone()),
            })
            .collect::<Vec<_>>();
        let mut binding = None;
        let mut bound_scope = false;
        let (target, arguments) = if let Some(receiver) = receiver {
            let class = match &receiver.kind {
                ExprKind::Name(alias)
                    if self.local(alias).is_none() && !self.captured.contains_key(alias) =>
                {
                    self.imports
                        .get(alias)
                        .filter(|owner| self.classes.contains_key(*owner))
                }
                _ => None,
            };
            if let Some(owner) = class {
                let methods = self.classpath.methods(owner)?;
                if methods
                    .iter()
                    .any(|method| method.source_property.as_deref() == Some(name))
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Property callable references are unsupported",
                    ));
                }
                let has_static = methods
                    .iter()
                    .any(|method| method.source_name() == name && method.access & 0x0008 != 0);
                if has_static
                    && methods
                        .iter()
                        .any(|method| method.source_name() == name && method.access & 0x0008 == 0)
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Unsupported mixed static and instance callable reference overloads",
                    ));
                }
                if has_static {
                    (
                        Expr {
                            offset,
                            kind: ExprKind::Property(receiver.clone(), name.clone()),
                        },
                        arguments,
                    )
                } else {
                    let Some(first) = arguments.first() else {
                        return Err(Diagnostic::new(
                            offset,
                            "Unbound member reference requires a receiver parameter",
                        ));
                    };
                    if !self.subtype(&params[0], &Type::Java(owner.clone())) {
                        return Err(Diagnostic::new(
                            offset,
                            "Unbound callable reference receiver differs from its expected parameter",
                        ));
                    }
                    (
                        Expr {
                            offset,
                            kind: ExprKind::Property(Box::new(first.clone()), name.clone()),
                        },
                        arguments[1..].to_vec(),
                    )
                }
            } else {
                let value = self.value(receiver)?;
                if let Some(owner) = library::type_owner(&value.ty) {
                    if self
                        .classpath
                        .methods(owner)?
                        .iter()
                        .any(|method| method.source_property.as_deref() == Some(name))
                    {
                        return Err(Diagnostic::new(
                            offset,
                            "Property callable references are unsupported",
                        ));
                    }
                }
                let slot = self.allocate_typed_slot(&value.ty, offset)?;
                let local_name = format!("<referenceReceiver{slot}>");
                self.scopes.push(HashMap::from([(
                    local_name.clone(),
                    Local {
                        slot,
                        ty: value.ty.clone(),
                        mutable: false,
                    },
                )]));
                self.initialized.insert(slot);
                bound_scope = true;
                binding = Some((slot, value));
                (
                    Expr {
                        offset,
                        kind: ExprKind::Property(
                            Box::new(Expr {
                                offset,
                                kind: ExprKind::Name(local_name),
                            }),
                            name.clone(),
                        ),
                    },
                    arguments,
                )
            }
        } else {
            if self
                .imports
                .get(name)
                .is_some_and(|owner| self.classes.contains_key(owner))
            {
                return Err(Diagnostic::new(
                    offset,
                    "Constructor callable references are unsupported",
                ));
            }
            if self.constants.contains_key(name)
                || self
                    .library_candidates(name)
                    .iter()
                    .any(|method| method.source_property.is_some())
            {
                return Err(Diagnostic::new(
                    offset,
                    "Property callable references are unsupported",
                ));
            }
            (
                Expr {
                    offset,
                    kind: ExprKind::Name(name.clone()),
                },
                arguments,
            )
        };
        let lambda = Expr {
            offset,
            kind: ExprKind::LambdaWithParams {
                params: Some(
                    names
                        .into_iter()
                        .map(|name| syntax::LambdaParameter {
                            name,
                            ty: None,
                            discard: false,
                        })
                        .collect(),
                ),
                body: vec![syntax::Statement {
                    offset,
                    kind: StatementKind::Expression(Expr {
                        offset,
                        kind: ExprKind::Call(Box::new(target), arguments),
                    }),
                }],
            },
        };
        let result = self.lambda(&lambda, expected, None);
        if bound_scope {
            self.scopes.pop();
        }
        result.map(|value| Self::wrap_lets(value, binding.into_iter().collect()))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn top_level_reference_uses_expected_parameter_types() {
        let source = "fun plus(value: Int): Int = value + 1\nfun use(): Int { val block: (Int) -> Int = ::plus; return block(4) }";
        let classes = crate::compile_all(source, "References.kt").unwrap();
        assert_eq!(classes.len(), 2);
        assert!(
            classes
                .iter()
                .any(|class| class.internal_name.contains("$lambda$"))
        );
    }

    #[test]
    fn reference_arity_mismatch_rejects() {
        let source = "fun plus(value: Int): Int = value + 1\nfun use(): Int { val block: () -> Int = ::plus; return block() }";
        let error = match crate::compile_all(source, "References.kt") {
            Ok(_) => panic!("mismatched callable reference compiled"),
            Err(error) => error,
        };
        assert!(error.message.contains("argument"), "{}", error.message);
    }
}
