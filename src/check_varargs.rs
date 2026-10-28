use super::*;

fn element_type(array: &Type) -> Option<Type> {
    array.array_element()
}

impl Checker<'_> {
    pub(super) fn vararg_arguments(
        &self,
        method: &JavaMethod,
        args: &[Expr],
        offset: usize,
    ) -> Result<Vec<Expr>> {
        if method.access & 0x80 == 0 {
            return Ok(args.to_vec());
        }
        let (params, _) = Self::method_parameters(method)?;
        let index = params.len().checked_sub(1).ok_or_else(|| {
            Diagnostic::new(offset, "Vararg declaration requires an array parameter")
        })?;
        let array = params[index].1.clone();
        if element_type(&array).is_none() {
            return Err(Diagnostic::new(
                offset,
                "Unsupported vararg array parameter type",
            ));
        }
        let mut normalized = Vec::new();
        let mut elements = Vec::new();
        let mut named_array = false;
        let mut seen_named = false;
        for (position, argument) in args.iter().enumerate() {
            match &argument.kind {
                ExprKind::NamedArgument(name, value) if name == &params[index].0 => {
                    if !self.method_named(method) {
                        return Err(Diagnostic::new(
                            argument.offset,
                            "Named arguments require a Kotlin declaration",
                        ));
                    }
                    if named_array || !elements.is_empty() {
                        return Err(Diagnostic::new(
                            argument.offset,
                            "Duplicate vararg argument",
                        ));
                    }
                    named_array = true;
                    seen_named = true;
                    elements.push(match &value.kind {
                        ExprKind::Unary(operator, _) if operator == "*" => value.as_ref().clone(),
                        _ => Expr {
                            offset: value.offset,
                            kind: ExprKind::Unary("*".into(), value.clone()),
                        },
                    });
                }
                ExprKind::NamedArgument(_, _) => {
                    seen_named = true;
                    normalized.push(argument.clone());
                }
                _ if seen_named => {
                    return Err(Diagnostic::new(
                        argument.offset,
                        "Positional arguments after named arguments are unsupported",
                    ));
                }
                _ if position < index => normalized.push(argument.clone()),
                _ => elements.push(argument.clone()),
            }
        }
        let packed = Expr {
            offset,
            kind: ExprKind::VarargArray {
                array: array.clone(),
                elements,
            },
        };
        normalized.push(
            if (seen_named || normalized.len() < index) && self.method_named(method) {
                Expr {
                    offset,
                    kind: ExprKind::NamedArgument(params[index].0.clone(), Box::new(packed)),
                }
            } else {
                packed
            },
        );
        Ok(normalized)
    }

    pub(super) fn vararg_array(
        &mut self,
        array: &Type,
        elements: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let java_component = matches!(array, Type::Platform(_));
        let array = array.base();
        let mut element = element_type(array)
            .ok_or_else(|| Diagnostic::new(offset, "Unsupported vararg array type"))?;
        if java_component && element.is_reference() && !matches!(element, Type::Platform(_)) {
            element = Type::Platform(Box::new(element));
        }
        let inferred = matches!(element.base(), Type::Parameter { .. });
        let mut checked = Vec::new();
        let mut inferred_element = None;
        for expr in elements {
            let (spread, expr) = match &expr.kind {
                ExprKind::Unary(operator, expr) if operator == "*" => (true, expr.as_ref()),
                _ => (false, expr),
            };
            let value = self.value(expr)?;
            let actual = if spread {
                element_type(&value.ty).ok_or_else(|| {
                    Diagnostic::new(expr.offset, "Spread argument requires a non-null array")
                })?
            } else {
                value.ty.clone()
            };
            if inferred {
                inferred_element = Some(match inferred_element {
                    None => actual,
                    Some(previous) if previous == actual => previous,
                    Some(Type::Null) => {
                        if actual.is_nullable() {
                            actual
                        } else {
                            Type::Nullable(Box::new(actual))
                        }
                    }
                    Some(previous) if actual == Type::Null => {
                        if previous.is_nullable() {
                            previous
                        } else {
                            Type::Nullable(Box::new(previous))
                        }
                    }
                    Some(previous) if self.subtype(&actual, &previous) => previous,
                    Some(previous) if self.subtype(&previous, &actual) => actual,
                    Some(_) => Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
                });
            }
            checked.push((spread, value, expr.offset));
        }
        if let Some(actual) = inferred_element {
            element = actual;
        }
        let array = if inferred {
            Type::Array(Box::new(element.clone()))
        } else {
            array.clone()
        };
        let mut values = Vec::new();
        for (spread, value, position) in checked {
            if spread {
                let actual = element_type(&value.ty).unwrap();
                let matching_array_kind = if array.primitive_array_element().is_some() {
                    &array == value.ty.base()
                } else {
                    matches!(value.ty.base(), Type::Array(_) | Type::StringArray)
                };
                if !matching_array_kind || !self.subtype(&actual, &element) {
                    return Err(Diagnostic::new(
                        position,
                        "Spread array component type mismatch",
                    ));
                }
                values.push((true, value));
            } else {
                values.push((false, self.coerce(value, element.clone(), position)?));
            }
        }
        if !values.iter().any(|(spread, _)| *spread) {
            let slot = self.allocate_typed_slot(&array, offset)?;
            let local = Value {
                ty: array.clone(),
                kind: ValueKind::Local(slot),
            };
            let mut body = vec![Statement::Store(
                slot,
                Value {
                    ty: array.clone(),
                    kind: ValueKind::NewArray(
                        array.clone(),
                        Box::new(Value {
                            ty: Type::Int,
                            kind: ValueKind::Int(values.len() as i32),
                        }),
                    ),
                },
            )];
            for (index, (_, value)) in values.into_iter().enumerate() {
                body.push(Statement::IndexStore(
                    local.clone(),
                    Value {
                        ty: Type::Int,
                        kind: ValueKind::Int(index as i32),
                    },
                    value,
                ));
            }
            return Ok(Value {
                ty: array,
                kind: ValueKind::Block(body, Box::new(local)),
            });
        }
        let primitive = array.primitive_array_name();
        let owner = primitive.map_or_else(
            || "kotlin/jvm/internal/SpreadBuilder".into(),
            |name| format!("kotlin/jvm/internal/{name}SpreadBuilder"),
        );
        let builder_type = Type::Java(owner.clone());
        let slot = self.allocate_typed_slot(&builder_type, offset)?;
        let builder = Value {
            ty: builder_type.clone(),
            kind: ValueKind::Local(slot),
        };
        let mut body = vec![Statement::Store(
            slot,
            Value {
                ty: builder_type,
                kind: ValueKind::New {
                    owner: owner.clone(),
                    descriptor: "(I)V".into(),
                    args: vec![Value {
                        ty: Type::Int,
                        kind: ValueKind::Int(values.len() as i32),
                    }],
                },
            },
        )];
        let method = |name: &str, descriptor: String| JavaMethod {
            library: None,
            owner: owner.clone(),
            name: name.into(),
            descriptor,
            access: 1,
            unsupported: false,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        };
        for (spread, value) in values {
            let value = if spread || primitive.is_some() || value.ty.is_reference() {
                value
            } else {
                Value {
                    ty: Type::Nullable(Box::new(value.ty.clone())),
                    kind: ValueKind::Box(Box::new(value)),
                }
            };
            let descriptor = if spread || primitive.is_none() {
                "(Ljava/lang/Object;)V".into()
            } else {
                format!("({})V", element.descriptor())
            };
            body.push(Statement::Expression(Value {
                ty: Type::Unit,
                kind: ValueKind::JavaCall {
                    method: method(if spread { "addSpread" } else { "add" }, descriptor),
                    args: vec![builder.clone(), value],
                },
            }));
        }
        let mut args = vec![builder];
        let descriptor = if primitive.is_some() {
            format!("(){}", array.descriptor())
        } else {
            args.push(Value {
                ty: array.clone(),
                kind: ValueKind::NewArray(
                    array.clone(),
                    Box::new(Value {
                        ty: Type::Int,
                        kind: ValueKind::Int(0),
                    }),
                ),
            });
            "([Ljava/lang/Object;)[Ljava/lang/Object;".into()
        };
        let result = Value {
            ty: array.clone(),
            kind: ValueKind::JavaCall {
                method: method("toArray", descriptor),
                args,
            },
        };
        Ok(Value {
            ty: array,
            kind: ValueKind::Block(body, Box::new(result)),
        })
    }
}
