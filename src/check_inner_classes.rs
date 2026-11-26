use super::*;

impl Checker<'_> {
    pub(super) fn enclosing_receiver(&self, offset: usize) -> Result<Option<Value>> {
        if let Some(receiver) = self.local("this$0") {
            return Ok(Some(Self::local_receiver(&receiver)));
        }
        let Ok(receiver) = self.dispatch_receiver_value(offset) else {
            return Ok(None);
        };
        let Some(owner) = library::type_owner(&receiver.ty).map(str::to_owned) else {
            return Ok(None);
        };
        let class = self
            .classpath
            .load(&owner)
            .or_else(|error| self.classes.get(&owner).cloned().ok_or(error))?;
        Ok(class.enclosing_instance().map(|outer| Value {
            ty: Type::Java(outer.into()),
            kind: ValueKind::JavaField {
                descriptor: None,
                owner,
                name: "this$0".into(),
                receiver: Some(Box::new(receiver)),
            },
        }))
    }

    pub(super) fn outer_assignment(
        &mut self,
        name: &str,
        value: &Expr,
        offset: usize,
    ) -> Result<Option<Statement>> {
        if self.local(name).is_some()
            || self.captured.contains_key(name)
            || self.fields.contains_key(name)
            || self
                .members
                .values()
                .any(|method| method.source_property.as_deref() == Some(name))
        {
            return Ok(None);
        }
        let Some(outer) = self.enclosing_receiver(offset)? else {
            return Ok(None);
        };
        let label = library::type_owner(&outer.ty)
            .unwrap()
            .rsplit(['/', '$'])
            .next()
            .unwrap();
        self.field_assignment(
            &Expr {
                offset,
                kind: ExprKind::Name(format!("this@{label}")),
            },
            name,
            value,
            offset,
        )
        .map(Some)
    }

    pub(super) fn java_inner_constructor(
        &mut self,
        class: &JavaClass,
        receiver: Value,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        self.classpath
            .check_java_import_visibility(&self.lambda_owner, &class.name, offset)?;
        let outer = self
            .classpath
            .lexical_parent(&class.name)
            .ok_or_else(|| Diagnostic::new(offset, "Missing Java inner class declaration"))?;
        if class.unsupported
            || class.access & 0x400 != 0
            || receiver.ty.is_nullable()
            || !self.subtype(&receiver.ty, &Type::Java(outer.clone()))
        {
            return Err(Diagnostic::new(
                offset,
                "Java inner constructor requires a concrete class and a non-null enclosing instance",
            ));
        }
        if class
            .library
            .as_ref()
            .is_some_and(|l| !l.type_parameters.is_empty())
            || self
                .classpath
                .load(&outer)?
                .library
                .as_ref()
                .is_some_and(|l| !l.type_parameters.is_empty())
        {
            return Err(Diagnostic::new(
                offset,
                "Generic Java inner classes require enclosing type argument substitution",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "INNER_CONSTRUCTOR_SHAPE_UNSUPPORTED",
            ));
        }
        let receiver = self.coerce(receiver, Type::Java(outer.clone()), offset)?;
        let prefix = format!("(L{outer};");
        let methods = class
            .methods
            .iter()
            .filter(|m| {
                m.name == "<init>"
                    && m.descriptor.starts_with(&prefix)
                    && self.classpath.constructor_visible(&self.lambda_owner, m)
            })
            .cloned()
            .collect::<Vec<_>>();
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let name = format!("<enclosing-{slot}>");
        let mut arguments = vec![Expr {
            offset,
            kind: ExprKind::Name(name.clone()),
        }];
        arguments.extend_from_slice(args);
        self.scopes.push(HashMap::from([(
            name,
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        )]));
        self.initialized.insert(slot);
        let result = (|| {
            let method = self.select_method(&methods, &arguments, offset)?;
            let (params, defaults) = Self::method_parameters(&method)?;
            let descriptor = format!(
                "({})V",
                params.iter().map(|p| p.1.descriptor()).collect::<String>()
            );
            if method.descriptor != descriptor
                || method
                    .library
                    .as_ref()
                    .is_some_and(|l| !l.type_parameters.is_empty())
            {
                return Err(Diagnostic::new(
                    offset,
                    "Java inner constructor has unsupported hidden or generic parameters",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "INNER_CONSTRUCTOR_SHAPE_UNSUPPORTED",
                ));
            }
            let arguments = self.vararg_arguments(&method, &arguments, offset)?;
            self.construct_value(&class.name, &params, &defaults, &arguments, false, offset)
        })();
        self.scopes.pop();
        self.initialized.remove(&slot);
        Ok(Self::wrap_lets(result?, vec![(slot, receiver)]))
    }

    pub(super) fn inner_constructor(
        &mut self,
        class: &JavaClass,
        receiver: Value,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        self.classpath
            .check_classifier_visibility(&self.lambda_owner, &class.name, offset)?;
        let outer = class
            .enclosing_instance()
            .ok_or_else(|| Diagnostic::new(offset, "Constructor has no enclosing instance"))?;
        if receiver.ty.is_nullable() || !self.subtype(&receiver.ty, &Type::Java(outer.into())) {
            return Err(Diagnostic::new(
                offset,
                "Inner constructor requires a non-null enclosing instance",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "INNER_CONSTRUCTOR_RECEIVER_INVALID",
            ));
        }
        if class.unsupported || class.access & 0x401 != 1 {
            return Err(
                Diagnostic::new(offset, "Unsupported inner constructor classifier").with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "INNER_CONSTRUCTOR_CLASS_UNSUPPORTED",
                ),
            );
        }
        let outer_class = self.classpath.load(outer)?;
        if !class.kotlin_class
            || class
                .library
                .as_ref()
                .is_some_and(|library| !library.type_parameters.is_empty())
            || outer_class
                .library
                .as_ref()
                .is_some_and(|library| !library.type_parameters.is_empty())
            || outer_class.enclosing_instance().is_some()
        {
            return Err(Diagnostic::new(
                offset,
                "Java, generic and chained compiled inner constructors are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "INNER_CONSTRUCTOR_SHAPE_UNSUPPORTED",
            ));
        }
        let methods = class
            .methods
            .iter()
            .filter(|method| {
                method.name == "<init>" && method.access & 1 != 0 && !method.unsupported
            })
            .cloned()
            .collect::<Vec<_>>();
        let method = self.select_method(&methods, args, offset)?;
        let normalized = self.vararg_arguments(&method, args, offset)?;
        let (params, defaults) = Self::method_parameters(&method)?;
        if defaults.iter().any(|default| *default)
            || !class
                .library
                .as_ref()
                .is_none_or(|library| library.type_parameters.is_empty())
        {
            return Err(Diagnostic::new(
                offset,
                "Generic and defaulted compiled inner constructors are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "INNER_CONSTRUCTOR_SHAPE_UNSUPPORTED",
            ));
        }
        let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
        let mut value = self.construct_value(
            &class.name,
            &params,
            &defaults,
            &normalized,
            self.method_named(&method),
            offset,
        )?;
        fn prepend(value: &mut Value, outer: &str, slot: u8) -> Result<()> {
            match &mut value.kind {
                ValueKind::Let { body, .. } => prepend(body, outer, slot),
                ValueKind::New {
                    descriptor, args, ..
                } => {
                    descriptor.insert_str(1, &format!("L{outer};"));
                    args.insert(
                        0,
                        Value {
                            ty: Type::Java(outer.into()),
                            kind: ValueKind::Local(slot),
                        },
                    );
                    Ok(())
                }
                _ => Err(Diagnostic::new(0, "Invalid inner constructor lowering")),
            }
        }
        prepend(&mut value, outer, slot)?;
        Ok(Self::wrap_lets(value, vec![(slot, receiver)]))
    }

    pub(super) fn implicit_inner_constructor(
        &mut self,
        owner: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Option<Value>> {
        let class = self
            .classpath
            .load(owner)
            .or_else(|error| self.classes.get(owner).cloned().ok_or(error))?;
        let Some(outer) = class.enclosing_instance() else {
            return Ok(None);
        };
        let receiver = self
            .implicit_receiver_values(offset)?
            .into_iter()
            .find(|receiver| self.subtype(&receiver.ty, &Type::Java(outer.into())))
            .ok_or_else(|| {
                Diagnostic::new(offset, "Inner constructor requires an enclosing instance")
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "INNER_CONSTRUCTOR_RECEIVER_REQUIRED",
                    )
            })?;
        self.inner_constructor(&class, receiver, args, offset)
            .map(Some)
    }
}
