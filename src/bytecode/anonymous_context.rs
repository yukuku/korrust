use super::*;

#[derive(Clone)]
pub(super) struct Enclosing {
    pub owner: String,
    pub method: Option<(String, String)>,
    pub parameters: HashSet<String>,
    pub supported: bool,
}

fn scan(
    values: impl IntoIterator<Item = Value>,
    body: &[Statement],
    context: &Enclosing,
    result: &mut HashMap<String, Enclosing>,
) {
    let mut visitor = |value: &mut Value, _: &str| {
        if let ValueKind::Lambda { owner, .. } = &value.kind {
            result.insert(owner.clone(), context.clone());
        }
    };
    for mut value in values {
        crate::check::ir_walk::value(&mut value, &context.owner, &mut visitor);
    }
    crate::check::ir_walk::statements(&mut body.to_vec(), &context.owner, &mut visitor);
}

fn function(
    function: &crate::check::Function,
    owner: &str,
    parameters: &HashSet<String>,
    result: &mut HashMap<String, Enclosing>,
) {
    let unsupported = function.suspend_
        || function
            .type_params
            .iter()
            .any(|parameter| parameters.contains(&parameter.name));
    let mut parameters = parameters.clone();
    parameters.extend(function.class_type_params.iter().cloned());
    parameters.extend(
        function
            .type_params
            .iter()
            .map(|parameter| parameter.name.clone()),
    );
    scan(
        function.defaults.iter().flatten().cloned(),
        &function.body,
        &Enclosing {
            owner: owner.into(),
            method: Some((function.jvm_name().into(), function.descriptor())),
            parameters,
            supported: !unsupported,
        },
        result,
    );
}

fn companion(companion: &crate::check::ClassCompanion, result: &mut HashMap<String, Enclosing>) {
    let parameters = HashSet::new();
    scan(
        companion
            .properties
            .iter()
            .map(|property| property.value.clone()),
        &companion.initializers,
        &Enclosing {
            owner: companion.owner.clone(),
            method: None,
            parameters: parameters.clone(),
            supported: true,
        },
        result,
    );
    for method in &companion.methods {
        function(method, &companion.owner, &parameters, result);
    }
}

fn class(class: &crate::check::ClassDecl, owner: &str, result: &mut HashMap<String, Enclosing>) {
    let parameters: HashSet<_> = class
        .type_params
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect();
    scan(
        class
            .fields
            .iter()
            .filter_map(|field| field.initializer.clone())
            .chain(class.super_args.iter().cloned())
            .chain(class.defaults.iter().flatten().cloned())
            .chain(
                class
                    .delegated_properties
                    .iter()
                    .map(|(_, value)| value.clone()),
            )
            .chain(
                class
                    .interface_delegate_storage
                    .iter()
                    .map(|delegate| delegate.initializer.clone()),
            ),
        &class.constructor_body,
        &Enclosing {
            owner: owner.into(),
            method: class.primary_constructor.then(|| {
                (
                    "<init>".into(),
                    format!(
                        "({})V",
                        class
                            .params
                            .iter()
                            .map(|(_, ty)| ty.descriptor())
                            .collect::<String>()
                    ),
                )
            }),
            parameters: parameters.clone(),
            supported: true,
        },
        result,
    );
    for constructor in &class.secondary_constructors {
        function(&constructor.function, owner, &parameters, result);
    }
    for method in &class.methods {
        function(method, owner, &parameters, result);
    }
    if let Some(declaration) = &class.companion {
        companion(declaration, result);
    }
}

fn interface(interface: &crate::check::InterfaceDecl, result: &mut HashMap<String, Enclosing>) {
    let parameters = interface
        .type_params
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect();
    for method in &interface.methods {
        function(&method.function, &interface.owner, &parameters, result);
    }
    for nested in &interface.nested {
        self::interface(nested, result);
    }
    if let Some(companion) = &interface.companion {
        if let Some(state) = &companion.state {
            self::companion(state, result);
        }
        for property in &companion.properties {
            if let crate::check::CompanionValue::Object { owner, class_decl } = &property.value {
                class(class_decl, owner, result);
            }
        }
    }
}

pub(super) fn collect(module: &Module, owner: &str) -> HashMap<String, Enclosing> {
    let mut result = HashMap::new();
    let parameters = HashSet::new();
    scan(
        module
            .properties
            .iter()
            .map(|property| property.value.clone()),
        &module.initializers,
        &Enclosing {
            owner: owner.into(),
            method: None,
            parameters: parameters.clone(),
            supported: true,
        },
        &mut result,
    );
    for declaration in &module.functions {
        function(declaration, owner, &parameters, &mut result);
    }
    if let Some(declaration) = &module.class_decl {
        class(declaration, owner, &mut result);
    }
    if let Some(declaration) = &module.interface_decl {
        interface(declaration, &mut result);
    }
    if let Some(declaration) = &module.object_decl {
        scan(
            declaration
                .fields
                .iter()
                .map(|field| match &field.initializer {
                    crate::check::ObjectInitializer::Value { value, .. } => value.clone(),
                    crate::check::ObjectInitializer::ThreadLocalArray { size, .. } => size.clone(),
                }),
            &declaration.initializers,
            &Enclosing {
                owner: declaration.owner.clone(),
                method: None,
                parameters: parameters.clone(),
                supported: true,
            },
            &mut result,
        );
        for method in &declaration.methods {
            function(
                &method.function,
                &declaration.owner,
                &parameters,
                &mut result,
            );
        }
    }
    for _ in 0..module.lambdas.len() {
        let before = result.len();
        for lambda in &module.lambdas {
            if let Some(context) = result.get(&lambda.owner).cloned() {
                if let Some(methods) = &lambda.anonymous_methods {
                    for method in methods {
                        function(method, &lambda.owner, &context.parameters, &mut result);
                    }
                }
            }
        }
        if result.len() == before {
            break;
        }
    }
    result
}

pub(super) fn parameters(ty: &Type, result: &mut HashSet<String>) {
    match ty {
        Type::Parameter { name, .. } => {
            result.insert(name.clone());
        }
        Type::Parameterized { arguments, .. } => {
            for argument in arguments.iter().filter_map(|argument| argument.ty.as_ref()) {
                parameters(argument, result);
            }
        }
        Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::Array(inner)
        | Type::OutArray(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner) => parameters(inner, result),
        Type::Function {
            params,
            result: output,
        }
        | Type::SuspendFunction {
            params,
            result: output,
        } => {
            for parameter in params.iter().chain(std::iter::once(output.as_ref())) {
                parameters(parameter, result);
            }
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result: output,
        } => {
            for parameter in params.iter().chain([receiver.as_ref(), output.as_ref()]) {
                parameters(parameter, result);
            }
        }
        _ => {}
    }
}

pub(super) fn supertype(ty: &Type) -> Type {
    match ty {
        Type::List(inner) => Type::Parameterized {
            name: "java/util/List".into(),
            arguments: vec![crate::syntax::TypeArgument {
                variance: if covariant_parameter_type(inner) {
                    crate::syntax::Variance::Out
                } else {
                    crate::syntax::Variance::Invariant
                },
                ty: Some(supertype(inner)),
            }],
        },
        Type::Parameterized { name, arguments } => Type::Parameterized {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|argument| crate::syntax::TypeArgument {
                    variance: argument.variance,
                    ty: argument.ty.as_ref().map(supertype),
                })
                .collect(),
        },
        Type::Nullable(inner) => Type::Nullable(Box::new(supertype(inner))),
        Type::Platform(inner) => Type::Platform(Box::new(supertype(inner))),
        Type::Array(inner) => Type::Array(Box::new(supertype(inner))),
        Type::OutArray(inner) => Type::OutArray(Box::new(supertype(inner))),
        _ => ty.clone(),
    }
}
