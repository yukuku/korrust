use super::*;

pub(super) fn owner(ty: &Type) -> Option<&str> {
    crate::value_abi::nominal_owner(ty).filter(|owner| {
        matches!(
            *owner,
            "kotlin/UInt" | "kotlin/ULong" | "kotlin/UByte" | "kotlin/UShort"
        )
    })
}

impl Checker<'_> {
    pub(super) fn unsigned_literal(
        &mut self,
        bits: u64,
        long: bool,
        expected: Option<&Type>,
        offset: usize,
    ) -> Result<Value> {
        let requested = expected.and_then(owner);
        if let Some(limit) = match requested {
            Some("kotlin/UByte") => Some(u8::MAX as u64),
            Some("kotlin/UShort") => Some(u16::MAX as u64),
            _ => None,
        } {
            if long || bits > limit {
                return Err(Diagnostic::new(
                    offset,
                    "Unsigned literal exceeds its contextual type",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "UNSIGNED_LITERAL_TYPE_MISMATCH",
                ));
            }
            return Err(Diagnostic::new(
                offset,
                "Contextual UByte and UShort literals are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "UNSIGNED_SMALL_LITERAL_UNSUPPORTED",
            ));
        }
        let wide = long || bits > u64::from(u32::MAX) || requested == Some("kotlin/ULong");
        if requested == Some("kotlin/UInt") && wide {
            return Err(Diagnostic::new(
                offset,
                "Unsigned literal cannot be represented by UInt in this context",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "UNSIGNED_LITERAL_TYPE_MISMATCH",
            ));
        }
        let (name, carrier, kind) = if wide {
            ("kotlin/ULong", Type::Long, ValueKind::Long(bits as i64))
        } else {
            ("kotlin/UInt", Type::Int, ValueKind::Int(bits as u32 as i32))
        };
        let class = self.classpath.load(name)?;
        let descriptor = carrier.descriptor();
        if !class
            .library
            .as_ref()
            .and_then(|library| library.value_class.as_ref())
            .is_some_and(|info| info.carrier == carrier)
            || !class.methods.iter().any(|method| {
                method.name == "constructor-impl"
                    && method.descriptor == format!("({descriptor}){descriptor}")
                    && method.access & 9 == 9
            })
        {
            return Err(Diagnostic::new(
                offset,
                "Unsigned literal requires a matching public value class construction ABI",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "UNSIGNED_VALUE_ABI_UNSUPPORTED",
            ));
        }
        let raw = Value {
            ty: carrier.clone(),
            kind,
        };
        let constructed = Value {
            ty: carrier,
            kind: ValueKind::StaticCall {
                owner: name.into(),
                name: "constructor-impl".into(),
                descriptor: format!("({descriptor}){descriptor}"),
                args: vec![raw],
            },
        };
        Ok(Value {
            ty: Type::Java(name.into()),
            kind: ValueKind::StaticCall {
                owner: name.into(),
                name: "box-impl".into(),
                descriptor: format!("({descriptor})L{name};"),
                args: vec![constructed],
            },
        })
    }
}

impl Checker<'_> {
    pub(super) fn compiled_value_member(
        &mut self,
        method: JavaMethod,
        args: Vec<Value>,
        offset: usize,
    ) -> Result<ValueKind> {
        crate::compiled_inline::validate_access(&method, self.classpath, &self.lambda_owner)?;
        let code = method
            .library
            .as_ref()
            .and_then(|library| library.code.as_ref())
            .ok_or_else(|| Diagnostic::new(offset, "Compiled value member bytecode unavailable"))?;
        if method
            .library
            .as_ref()
            .is_some_and(|library| !library.reified_arguments.is_empty())
            || args.iter().any(|arg| {
                matches!(
                    arg.ty.base(),
                    Type::Function { .. }
                        | Type::ReceiverFunction { .. }
                        | Type::SuspendFunction { .. }
                )
            })
        {
            return Err(Diagnostic::new(
                offset,
                "Compiled value members with reified or function parameters are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "VALUE_INLINE_PARAMETER_UNSUPPORTED",
            ));
        }
        let local_base = self.next;
        self.next = self
            .next
            .checked_add(code.max_locals)
            .filter(|next| *next <= 256)
            .ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    "Compiled value member locals exceed current JVM limit",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "COMPILED_INLINE_LOCALS_UNSUPPORTED",
                )
            })?;
        let lambda_bases = vec![None; args.len()];
        Ok(ValueKind::CompiledInline {
            method,
            args,
            local_base,
            lambda_bases,
            reified_temp: None,
        })
    }

    pub(super) fn nominal_binary(
        &mut self,
        op: &str,
        receiver: Value,
        argument: &Expr,
        offset: usize,
    ) -> Result<Value> {
        let (name, infix) = match op {
            "+" => ("plus", false),
            "-" => ("minus", false),
            "*" => ("times", false),
            "/" => ("div", false),
            "%" => ("rem", false),
            "<" | ">" | "<=" | ">=" => ("compareTo", false),
            "and" | "or" | "xor" | "shl" | "shr" | "ushr" => (op, true),
            _ => {
                return Err(
                    Diagnostic::new(offset, "Unsupported nominal operator").with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "NOMINAL_OPERATOR_UNSUPPORTED",
                    ),
                );
            }
        };
        if receiver.ty.is_nullable() {
            return Err(Diagnostic::new(
                offset,
                "Nullable nominal operator resolution is unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "NOMINAL_NULLABLE_OPERATOR_UNSUPPORTED",
            ));
        }
        let unsigned_receiver = super::unsigned::owner(&receiver.ty).is_some();
        let value = if infix {
            let owner = library::type_owner(&receiver.ty)
                .ok_or_else(|| Diagnostic::new(offset, "Operator receiver has no class"))?;
            let methods = self
                .classpath
                .methods(owner)?
                .into_iter()
                .filter(|method| {
                    method.source_name() == name
                        && method.instance_dispatch()
                        && method.library.as_ref().is_some_and(|library| library.infix)
                })
                .collect::<Vec<_>>();
            let method = self
                .select_method(&methods, std::slice::from_ref(argument), offset)
                .map_err(|error| {
                    if error.message == "No applicable member overload" {
                        error.with_kind(
                            crate::DiagnosticKind::UnsupportedLanguage,
                            "NOMINAL_INFIX_RESOLUTION_UNSUPPORTED",
                        )
                    } else {
                        error
                    }
                })?;
            self.member_call(method, receiver, std::slice::from_ref(argument), offset)?
        } else {
            self.operator_call(receiver, name, std::slice::from_ref(argument), offset)
                .map_err(|error| {
                    if unsigned_receiver && error.message == "No applicable member overload" {
                        error.with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "UNSIGNED_OPERATOR_TYPE_MISMATCH",
                        )
                    } else {
                        error
                    }
                })?
        };
        if matches!(op, "<" | ">" | "<=" | ">=") {
            self.require(&value, Type::Int, offset)?;
            Ok(Value {
                ty: Type::Boolean,
                kind: ValueKind::Binary(
                    op.into(),
                    Box::new(value),
                    Box::new(Value {
                        ty: Type::Int,
                        kind: ValueKind::Int(0),
                    }),
                ),
            })
        } else {
            Ok(value)
        }
    }
}
