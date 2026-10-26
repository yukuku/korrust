use super::*;

pub(super) struct DelegatedProperty {
    pub field: ClassField,
    pub methods: Vec<Function>,
    pub metadata_name: String,
    pub metadata: Value,
    pub property_type: Type,
    pub locals: u16,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check_property(
    source: &syntax::PropertyDecl,
    context: &declarations::DeclarationContext<'_>,
    owner: &str,
    storage_owner: &str,
    static_: bool,
    receiver: bool,
    constructor_params: &[(String, Type)],
    initial_locals: u16,
) -> Result<DelegatedProperty> {
    if source.initializer.is_some()
        || !source.body.is_empty()
        || source.setter.is_some()
        || source.lateinit
    {
        return Err(Diagnostic::new(
            source.offset,
            "Delegated properties cannot have initializers, custom accessors, or lateinit",
        ));
    }
    let delegate = source
        .delegate
        .as_ref()
        .ok_or_else(|| Diagnostic::new(source.offset, "Expected a delegated property"))?;
    let metadata_name = format!("{}$metadata", source.name);
    let metadata_type = Type::Parameterized {
        name: "kotlin/reflect/KProperty".into(),
        arguments: vec![syntax::TypeArgument {
            variance: syntax::Variance::Invariant,
            ty: None,
        }],
    };
    let metadata_initializer_read =
        static_read(storage_owner, &metadata_name, &metadata_type, false);
    let metadata_read = static_read(
        storage_owner,
        &metadata_name,
        &metadata_type,
        owner != storage_owner,
    );
    let mut initializer_checker = context.checker(
        receiver && !static_,
        u16::from(receiver && !static_),
        Type::Unit,
    );
    for (name, ty) in constructor_params {
        initializer_checker.declare(name, ty.clone(), false, source.offset)?;
    }
    initializer_checker.next = initializer_checker.next.max(initial_locals);
    initializer_checker.allow_return = false;
    let original = initializer_checker.value(delegate)?;
    if original.ty == Type::Unit || original.ty == Type::Null {
        return Err(Diagnostic::new(
            source.offset,
            "A property delegate requires a supported value type",
        ));
    }
    let original_next = initializer_checker.next;
    let provisioned = optional_operator(
        &mut initializer_checker,
        original.clone(),
        "provideDelegate",
        vec![
            this_reference(owner, receiver, static_),
            metadata_initializer_read,
        ],
        source.offset,
    )?;
    let initializer = provisioned.unwrap_or_else(|| {
        initializer_checker.next = original_next;
        original
    });
    if initializer.ty == Type::Unit || initializer.ty == Type::Null {
        return Err(Diagnostic::new(
            source.offset,
            "provideDelegate must return a supported delegate value",
        ));
    }
    let initializer_locals = initializer_checker.next;
    let declared_type = if source.inferred {
        None
    } else {
        Some(initializer_checker.resolve(&source.ty, source.offset)?)
    };
    let field_name = format!("{}$delegate", source.name);
    let delegate_type = initializer.ty.clone();
    let field_read = Value {
        ty: delegate_type.clone(),
        kind: if static_ {
            static_read(
                storage_owner,
                &field_name,
                &delegate_type,
                owner != storage_owner,
            )
            .kind
        } else {
            ValueKind::Field(
                field_name.clone(),
                Box::new(this_reference(owner, true, false)),
            )
        },
    };
    let mut getter_checker = context.checker(receiver, u16::from(receiver), Type::Unit);
    let result = invoke_operator(
        &mut getter_checker,
        field_read.clone(),
        "getValue",
        vec![
            this_reference(owner, receiver, false),
            metadata_read.clone(),
        ],
        source.offset,
    )?;
    let property_type = declared_type.unwrap_or_else(|| result.ty.clone());
    if matches!(property_type, Type::Unit | Type::Null) {
        return Err(Diagnostic::new(
            source.offset,
            "Delegated properties require a supported property type",
        ));
    }
    if source.inferred && has_platform(&property_type) {
        return Err(Diagnostic::new(
            source.offset,
            "Inferred delegated platform property types require an explicit supported type annotation",
        ));
    }
    let result = getter_checker.coerce(result, property_type.clone(), source.offset)?;
    let getter = Function {
        jvm_name: None,
        annotations: Vec::new(),
        vararg: false,
        type_params: Vec::new(),
        defaults: Vec::new(),
        name: getter_name(&source.name),
        private: source.private,
        is_override: source.is_override,
        params: Vec::new(),
        result: property_type.clone(),
        locals: getter_checker.next,
        body: vec![Statement::Return(result)],
    };
    let mut methods = vec![getter];
    if source.mutable {
        let mut setter_checker = context.checker(receiver, u16::from(receiver), Type::Unit);
        let value_slot =
            setter_checker.declare("value", property_type.clone(), false, source.offset)?;
        let result = invoke_operator(
            &mut setter_checker,
            field_read,
            "setValue",
            vec![
                this_reference(owner, receiver, false),
                metadata_read,
                Value {
                    ty: property_type.clone(),
                    kind: ValueKind::Local(value_slot),
                },
            ],
            source.offset,
        )?;
        setter_checker.require(&result, Type::Unit, source.offset)?;
        methods.push(Function {
            jvm_name: None,
            annotations: Vec::new(),
            vararg: false,
            type_params: Vec::new(),
            defaults: vec![None],
            name: setter_name(&source.name),
            private: source.private,
            is_override: source.is_override,
            params: vec![("value".into(), property_type.clone())],
            result: Type::Unit,
            locals: setter_checker.next,
            body: vec![
                Statement::Expression(result),
                Statement::Return(Value {
                    ty: Type::Unit,
                    kind: ValueKind::Unit,
                }),
            ],
        });
    }
    let reference_owner = format!(
        "kotlin/jvm/internal/{}PropertyReference{}Impl",
        if source.mutable { "Mutable" } else { "" },
        usize::from(receiver),
    );
    context.classpath.load(&reference_owner).map_err(|_| {
        Diagnostic::new(
            source.offset,
            "Delegated property metadata requires the Kotlin standard library",
        )
    })?;
    let metadata = Value {
        ty: metadata_type,
        kind: ValueKind::New {
            owner: reference_owner,
            descriptor: "(Ljava/lang/Class;Ljava/lang/String;Ljava/lang/String;I)V".into(),
            args: vec![
                Value {
                    ty: Type::Java("java/lang/Class".into()),
                    kind: ValueKind::ClassLiteral(Type::Java(owner.into())),
                },
                Value {
                    ty: Type::String,
                    kind: ValueKind::String(source.name.encode_utf16().collect()),
                },
                Value {
                    ty: Type::String,
                    kind: ValueKind::String(
                        format!(
                            "{}(){}",
                            getter_name(&source.name),
                            property_type.descriptor()
                        )
                        .encode_utf16()
                        .collect(),
                    ),
                },
                Value {
                    ty: Type::Int,
                    kind: ValueKind::Int(i32::from(!receiver)),
                },
            ],
        },
    };
    Ok(DelegatedProperty {
        field: ClassField {
            name: field_name,
            ty: delegate_type,
            private: true,
            mutable: false,
            param: None,
            initializer: Some(initializer),
        },
        methods,
        metadata_name,
        metadata,
        property_type,
        locals: initializer_locals,
    })
}

fn static_read(owner: &str, name: &str, ty: &Type, bridge: bool) -> Value {
    Value {
        ty: ty.clone(),
        kind: if bridge {
            ValueKind::StaticCall {
                owner: owner.into(),
                name: format!("access${}$cp", getter_name(name)),
                descriptor: format!("(){}", ty.descriptor()),
                args: Vec::new(),
            }
        } else {
            ValueKind::StaticField {
                owner: owner.into(),
                name: name.into(),
            }
        },
    }
}

fn this_reference(owner: &str, receiver: bool, static_initializer: bool) -> Value {
    if !receiver {
        return Value {
            ty: Type::Null,
            kind: ValueKind::Null,
        };
    }
    Value {
        ty: Type::Java(owner.into()),
        kind: if static_initializer {
            if let Some(outer) = owner.strip_suffix("$Companion") {
                ValueKind::StaticField {
                    owner: outer.into(),
                    name: "Companion".into(),
                }
            } else {
                ValueKind::ObjectInstance(owner.into())
            }
        } else {
            ValueKind::Local(0)
        },
    }
}

fn invoke_operator(
    checker: &mut Checker<'_>,
    receiver: Value,
    name: &str,
    arguments: Vec<Value>,
    offset: usize,
) -> Result<Value> {
    let receiver_slot = checker.allocate_typed_slot(&receiver.ty, offset)?;
    let receiver_type = receiver.ty.clone();
    let mut lets = vec![(receiver_slot, receiver)];
    let mut bindings = HashMap::new();
    let mut expressions = Vec::new();
    for argument in arguments {
        let slot = checker.allocate_typed_slot(&argument.ty, offset)?;
        let binding = format!("<delegate{slot}>");
        bindings.insert(
            binding.clone(),
            Local {
                slot,
                ty: argument.ty.clone(),
                mutable: false,
            },
        );
        checker.initialized.insert(slot);
        lets.push((slot, argument));
        expressions.push(Expr {
            offset,
            kind: ExprKind::Name(binding),
        });
    }
    checker.scopes.push(bindings);
    let result = checker.operator_call(
        Value {
            ty: receiver_type,
            kind: ValueKind::Local(receiver_slot),
        },
        name,
        &expressions,
        offset,
    );
    checker.scopes.pop();
    result.map(|value| Checker::wrap_lets(value, lets))
}

fn optional_operator(
    checker: &mut Checker<'_>,
    receiver: Value,
    name: &str,
    arguments: Vec<Value>,
    offset: usize,
) -> Result<Option<Value>> {
    let next = checker.next;
    let initialized = checker.initialized.clone();
    let refinements = checker.refinements.clone();
    let lambda_count = checker.lambdas.borrow().len();
    match invoke_operator(checker, receiver, name, arguments, offset) {
        Ok(value) => Ok(Some(value)),
        Err(error)
            if error
                .message
                .starts_with("Unresolved or unsupported applicable")
                || error.message.starts_with("Unresolved renamed declaration") =>
        {
            checker.next = next;
            checker.initialized = initialized;
            checker.refinements = refinements;
            checker.lambdas.borrow_mut().truncate(lambda_count);
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
