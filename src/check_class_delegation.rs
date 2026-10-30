use super::*;

pub(super) struct Contract {
    pub declaration: JavaMethod,
    pub field: String,
    pub receiver: Type,
}

fn unsupported(offset: usize, code: &'static str, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(crate::DiagnosticKind::UnsupportedLanguage, code)
}

pub(super) fn normalize_contract(
    mut method: JavaMethod,
    classpath: &Classpath,
    offset: usize,
) -> Result<JavaMethod> {
    let owner = classpath.load(&method.owner)?;
    if owner.kotlin_class {
        return Ok(method);
    }
    if owner
        .library
        .as_ref()
        .is_some_and(|l| !l.type_parameters.is_empty())
        || method
            .library
            .as_ref()
            .is_some_and(|l| !l.type_parameters.is_empty())
    {
        return Err(unsupported(
            offset,
            "INTERFACE_DELEGATION_JAVA_GENERIC_UNSUPPORTED",
            "Generic Java interface delegation requires unsupported flexible parameter storage",
        ));
    }
    fn complex(ty: &Type, cp: &Classpath) -> bool {
        match ty {
            Type::Platform(inner) | Type::Nullable(inner) => complex(inner, cp),
            Type::Java(owner) => cp.load(owner).is_ok_and(|c| {
                c.library
                    .as_ref()
                    .is_some_and(|l| !l.type_parameters.is_empty())
            }),
            Type::Parameter { .. }
            | Type::Parameterized { .. }
            | Type::Array(_)
            | Type::StringArray
            | Type::List(_)
            | Type::MutableList(_)
            | Type::Function { .. }
            | Type::ReceiverFunction { .. } => true,
            _ => false,
        }
    }
    fn platform(ty: Type) -> Type {
        if ty.is_reference() && !ty.is_nullable() && !matches!(ty, Type::Platform(_)) {
            Type::Platform(Box::new(ty))
        } else {
            ty
        }
    }
    let (params, result) = class_method_types(&method)?;
    if params
        .iter()
        .chain(std::iter::once(&result))
        .any(|ty| complex(ty, classpath))
    {
        return Err(unsupported(
            offset,
            "INTERFACE_DELEGATION_JAVA_TYPE_UNSUPPORTED",
            "Delegation of Java generic, raw, function, or reference-array member types is unsupported",
        ));
    }
    method.kotlin_params = Some(
        params
            .into_iter()
            .enumerate()
            .map(|(index, ty)| {
                (
                    method
                        .kotlin_params
                        .as_ref()
                        .and_then(|p| p.get(index))
                        .map_or_else(|| format!("p{index}"), |p| p.0.clone()),
                    platform(ty),
                    false,
                )
            })
            .collect(),
    );
    method.kotlin_result = Some(platform(result));
    Ok(method)
}

pub(super) fn headers(classpath: &Classpath, target: &Type) -> Result<Vec<JavaMethod>> {
    let Some(owner) = library::type_owner(target) else {
        return Ok(Vec::new());
    };
    let class = classpath.load(owner)?;
    if class.access & 0x0200 == 0 {
        return Ok(Vec::new());
    }
    classpath
        .methods(owner)?
        .into_iter()
        .filter(|m| m.access & 9 == 1 && m.access & 0x40 == 0 && !m.name.starts_with('<'))
        .map(|method| {
            normalize_contract(
                library::specialize_receiver(method, target, &class, classpath),
                classpath,
                0,
            )
        })
        .collect()
}

fn internal_type(ty: &Type, classpath: &Classpath) -> bool {
    match ty {
        Type::Java(owner) => classpath.load(owner).is_ok_and(|c| {
            c.library
                .as_ref()
                .is_some_and(|l| l.kotlin_visibility == Some(0))
        }),
        Type::Parameterized { name, arguments } => {
            internal_type(&Type::Java(name.clone()), classpath)
                || arguments
                    .iter()
                    .filter_map(|a| a.ty.as_ref())
                    .any(|t| internal_type(t, classpath))
        }
        Type::Array(inner)
        | Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner)
        | Type::Parameter {
            upper_bound: inner, ..
        } => internal_type(inner, classpath),
        Type::Function { params, result } => {
            params.iter().any(|t| internal_type(t, classpath)) || internal_type(result, classpath)
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            internal_type(receiver, classpath)
                || params.iter().any(|t| internal_type(t, classpath))
                || internal_type(result, classpath)
        }
        _ => false,
    }
}

fn nested_type(ty: &Type) -> bool {
    match ty {
        Type::Java(owner) => owner.contains('$'),
        Type::Parameterized { name, arguments } => {
            name.contains('$')
                || arguments
                    .iter()
                    .filter_map(|a| a.ty.as_ref())
                    .any(nested_type)
        }
        Type::Array(inner)
        | Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner)
        | Type::Parameter {
            upper_bound: inner, ..
        } => nested_type(inner),
        Type::Function { params, result } => params.iter().any(nested_type) || nested_type(result),
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => nested_type(receiver) || params.iter().any(nested_type) || nested_type(result),
        _ => false,
    }
}

pub(super) fn prepare(
    source: &syntax::ClassDecl,
    checker: &mut Checker<'_>,
) -> Result<(Vec<InterfaceDelegateStorage>, Vec<Contract>)> {
    if source.interface_delegates.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    if source.data || !source.secondary_constructors.is_empty() || source.super_class.is_some() {
        return Err(unsupported(
            source.offset,
            "INTERFACE_DELEGATION_CLASS_SHAPE_UNSUPPORTED",
            "Delegating data classes, secondary constructors, and explicit superclass interactions are unsupported",
        ));
    }
    if checker.lambda_owner.contains('$') {
        return Err(unsupported(
            source.offset,
            "INTERFACE_DELEGATION_NESTED_UNSUPPORTED",
            "Nested or dollar-named delegating classifiers are unsupported",
        ));
    }
    for ty in source
        .params
        .iter()
        .map(|p| &p.1)
        .chain(source.super_types.iter())
    {
        if nested_type(&checker.resolve(ty, source.offset)?) {
            return Err(unsupported(
                source.offset,
                "INTERFACE_DELEGATION_NESTED_UNSUPPORTED",
                "Delegation with nested or dollar-named exposed type shapes is unsupported",
            ));
        }
    }
    if !source.internal {
        for ty in source
            .params
            .iter()
            .map(|p| &p.1)
            .chain(source.super_types.iter())
        {
            let ty = checker.resolve(ty, source.offset)?;
            if internal_type(&ty, checker.classpath) {
                return Err(Diagnostic::new(
                    source.offset,
                    "Public delegation exposes an internal constructor parameter or supertype",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "DELEGATION_EXPOSED_INTERNAL_TYPE",
                ));
            }
        }
    }
    for ty in &source.super_types {
        let ty = checker.resolve(ty, source.offset)?;
        if let Some(owner) = library::type_owner(&ty) {
            if checker.classpath.methods(owner)?.iter().any(|method| {
                method.access & 9 == 1
                    && !method.name.starts_with('<')
                    && method.access & 0x40 == 0
                    && (method.access & 0x0400 == 0
                        || method.library.as_ref().is_some_and(|l| !l.abstract_))
            }) {
                return Err(unsupported(
                    source.offset,
                    "INTERFACE_DELEGATION_DEFAULT_UNSUPPORTED",
                    "Delegation combined with interface default implementations is unsupported",
                ));
            }
        }
    }
    let mut storage = Vec::new();
    let mut contracts = Vec::new();
    let mut names = source
        .params
        .iter()
        .map(|p| p.0.clone())
        .chain(source.properties.iter().map(|p| p.name.clone()))
        .collect::<HashSet<_>>();
    let object_methods = checker.classpath.load("java/lang/Object")?.methods;
    for delegate in &source.interface_delegates {
        let ty = checker.resolve(&delegate.ty, delegate.offset)?;
        if matches!(&ty, Type::Parameterized { arguments, .. } if arguments.iter().any(|a| a.variance != syntax::Variance::Invariant || a.ty.is_none()))
        {
            return Err(unsupported(
                delegate.offset,
                "INTERFACE_DELEGATION_PROJECTION_UNSUPPORTED",
                "Projected interface delegation is unsupported",
            ));
        }
        let owner = library::type_owner(&ty).ok_or_else(|| {
            Diagnostic::new(delegate.offset, "Delegation requires an interface type").with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "INTERFACE_DELEGATION_TYPE_INVALID",
            )
        })?;
        let interface = checker.classpath.load(owner)?;
        if interface
            .library
            .as_ref()
            .and_then(|l| l.kotlin_visibility)
            .is_some_and(|v| v != 0 && v != 3)
        {
            return Err(Diagnostic::new(
                delegate.offset,
                "Delegated interface is inaccessible in Kotlin",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "INTERFACE_DELEGATION_VISIBILITY_INVALID",
            ));
        }
        if interface
            .library
            .as_ref()
            .is_some_and(|l| l.kotlin_visibility == Some(0))
        {
            return Err(unsupported(
                delegate.offset,
                "INTERFACE_DELEGATION_INTERNAL_UNSUPPORTED",
                "Delegation to internal interface contracts is unsupported",
            ));
        }
        if owner.contains('$') {
            return Err(unsupported(
                delegate.offset,
                "INTERFACE_DELEGATION_NESTED_UNSUPPORTED",
                "Nested or dollar-named interface delegation contracts are unsupported",
            ));
        }
        if ty.is_nullable() || interface.access & 0x0201 != 0x0201 {
            return Err(Diagnostic::new(
                delegate.offset,
                "Delegation requires an accessible non-null interface type",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "INTERFACE_DELEGATION_TYPE_INVALID",
            ));
        }
        let returns = checker.allow_return;
        checker.allow_return = false;
        let value = checker.value(&delegate.expression);
        checker.allow_return = returns;
        let value = value?;
        if !value.ty.is_reference() {
            return Err(unsupported(
                delegate.offset,
                "INTERFACE_DELEGATION_PRIMITIVE_UNSUPPORTED",
                "Primitive values as interface delegates are unsupported",
            ));
        }
        let initializer = checker
            .coerce(value, ty.clone(), delegate.offset)
            .map_err(|error| {
                if error.message.starts_with("Type mismatch") {
                    error.with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "INTERFACE_DELEGATION_VALUE_INVALID",
                    )
                } else {
                    error
                }
            })?;
        let mut ordinal = storage.len();
        let name = loop {
            let name = format!("$$delegate_{ordinal}");
            if names.insert(name.clone()) {
                break name;
            }
            ordinal += 1;
        };
        for raw in library::remove_overridden(checker.classpath.methods(owner)?, checker.classpath)
        {
            if raw.name.starts_with('<') || raw.access & 9 != 1 || raw.access & 0x0040 != 0 {
                continue;
            }
            if raw.library.as_ref().is_some_and(|l| l.has_annotations) {
                return Err(unsupported(
                    delegate.offset,
                    "INTERFACE_DELEGATION_ANNOTATION_UNSUPPORTED",
                    "Copying delegated declaration or parameter annotations is unsupported",
                ));
            }
            if raw.unsupported
                || raw.name != raw.source_name()
                || raw
                    .library
                    .as_ref()
                    .is_some_and(|l| !l.type_parameters.is_empty() || l.receiver.is_some())
            {
                return Err(unsupported(
                    delegate.offset,
                    "INTERFACE_DELEGATION_MEMBER_UNSUPPORTED",
                    "Generic, extension, renamed, or unsupported interface members cannot be delegated",
                ));
            }
            if raw.access & 0x0400 == 0 || raw.library.as_ref().is_some_and(|l| !l.abstract_) {
                return Err(unsupported(
                    delegate.offset,
                    "INTERFACE_DELEGATION_DEFAULT_UNSUPPORTED",
                    "Delegation of interface contracts containing default implementations is unsupported",
                ));
            }
            if raw.access & 0x0080 != 0
                || raw
                    .kotlin_params
                    .as_ref()
                    .is_some_and(|p| p.iter().any(|p| p.2))
            {
                return Err(unsupported(
                    delegate.offset,
                    "INTERFACE_DELEGATION_ARGUMENT_SHAPE_UNSUPPORTED",
                    "Delegation of vararg or default-argument interface contracts is unsupported",
                ));
            }
            if object_methods
                .iter()
                .any(|m| m.access & 9 == 1 && m.name == raw.name && m.descriptor == raw.descriptor)
            {
                return Err(unsupported(
                    delegate.offset,
                    "INTERFACE_DELEGATION_ANY_MEMBER_UNSUPPORTED",
                    "Delegation of explicitly redeclared Any members is unsupported",
                ));
            }
            contracts.push(Contract {
                declaration: normalize_contract(
                    library::specialize_receiver(raw, &ty, &interface, checker.classpath),
                    checker.classpath,
                    delegate.offset,
                )?,
                field: name.clone(),
                receiver: ty.clone(),
            });
        }
        let slot = checker.allocate_typed_slot(&ty, delegate.offset)?;
        storage.push(InterfaceDelegateStorage {
            slot,
            name,
            ty,
            initializer,
        });
    }
    Ok((storage, contracts))
}

pub(super) fn forwarders(
    contracts: &[Contract],
    methods: &[Function],
    owner: &str,
    supertypes: &[Type],
    classpath: &Classpath,
    offset: usize,
) -> Result<(Vec<Function>, Vec<InterfaceDelegatedMember>)> {
    let mut output = Vec::new();
    let mut declarations = Vec::new();
    let mut generated = HashSet::new();
    for contract in contracts {
        let declaration = &contract.declaration;
        let (parameters, result) = class_method_types(declaration)?;
        if methods.iter().any(|method| {
            method.name == declaration.name
                && declarations::override_signature(
                    declaration,
                    &method
                        .params
                        .iter()
                        .map(|p| p.1.clone())
                        .collect::<Vec<_>>(),
                    &method.result,
                )
                .unwrap_or(false)
        }) {
            continue;
        }
        let key = declarations::logical_member_key(declaration)?;
        if !generated.insert(key) {
            return Err(Diagnostic::new(
                offset,
                "Multiple delegated interfaces require an explicit override for a shared member",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "INTERFACE_DELEGATION_CONFLICT_INVALID",
            ));
        }
        let mut params = declaration
            .kotlin_params
            .as_ref()
            .map(|p| p.iter().map(|p| (p.0.clone(), p.1.clone())).collect())
            .unwrap_or_else(|| {
                parameters
                    .iter()
                    .enumerate()
                    .map(|(i, t)| (format!("p{i}"), java_parameter(t.clone())))
                    .collect::<Vec<_>>()
            });
        if declaration.source_property.is_some() && result == Type::Unit && params.len() == 1 {
            params[0].0 = "<set-?>".into();
        }
        let jvm_result = source_jvm_result(classpath, supertypes, declaration)?;
        let physical_result = jvm_result.as_ref().unwrap_or(&result).clone();
        let locals = 1 + params.iter().map(|p| p.1.slots()).sum::<usize>();
        if locals > 255 {
            return Err(unsupported(
                offset,
                "INTERFACE_DELEGATION_SLOT_LIMIT_UNSUPPORTED",
                "Delegated members exceed JVM parameter slot limits",
            ));
        }
        let mut next = 1u8;
        let mut arguments = params
            .iter()
            .map(|(_, ty)| {
                let slot = next;
                next += ty.slots() as u8;
                Value {
                    ty: ty.clone(),
                    kind: ValueKind::Local(slot),
                }
            })
            .collect::<Vec<_>>();
        arguments.insert(
            0,
            Value {
                ty: contract.receiver.clone(),
                kind: ValueKind::Field(
                    contract.field.clone(),
                    Box::new(Value {
                        ty: Type::Java(owner.into()),
                        kind: ValueKind::Local(0),
                    }),
                ),
            },
        );
        let invocation = Value {
            ty: physical_result,
            kind: ValueKind::MemberCall {
                method: declaration.clone(),
                interface: true,
                args: arguments,
            },
        };
        let body = if result == Type::Unit {
            vec![
                Statement::Expression(invocation),
                Statement::Return(Value {
                    ty: Type::Unit,
                    kind: ValueKind::Unit,
                }),
            ]
        } else {
            vec![Statement::Return(invocation)]
        };
        let function = Function {
            extension_receiver: false,
            class_type_params: Vec::new(),
            jvm_result,
            jvm_name: None,
            annotations: Vec::new(),
            vararg: false,
            type_params: Vec::new(),
            defaults: vec![None; params.len()],
            name: declaration.name.clone(),
            private: false,
            is_override: true,
            params,
            result,
            locals: next as u16,
            body,
        };
        declarations.push(InterfaceDelegatedMember {
            declaration: declaration.clone(),
            jvm_descriptor: function.descriptor(),
        });
        output.push(function);
    }
    Ok((output, declarations))
}
