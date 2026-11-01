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

    pub(super) fn inner_constructor(
        &mut self,
        class: &JavaClass,
        receiver: Value,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
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
