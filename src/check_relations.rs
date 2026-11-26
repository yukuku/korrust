use super::*;
use crate::syntax::{TypeArgument, Variance};

pub(super) fn type_owner(ty: &Type) -> Option<&str> {
    match ty.base() {
        Type::Java(name) | Type::Parameterized { name, .. } => Some(name),
        Type::String => Some("java/lang/String"),
        Type::Parameter { upper_bound, .. } => type_owner(upper_bound),
        Type::List(_) | Type::MutableList(_) | Type::FlexibleList(_) => Some("java/util/List"),
        _ => None,
    }
}

fn relation_owner(ty: &Type) -> Option<&str> {
    type_owner(ty).or_else(|| match ty.base() {
        Type::Int => Some("java/lang/Integer"),
        Type::Long => Some("java/lang/Long"),
        Type::Byte => Some("java/lang/Byte"),
        Type::Short => Some("java/lang/Short"),
        Type::Char => Some("java/lang/Character"),
        Type::Boolean => Some("java/lang/Boolean"),
        Type::Float => Some("java/lang/Float"),
        Type::Double => Some("java/lang/Double"),
        _ => None,
    })
}

pub(super) fn substitute(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Parameter { name, .. } => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Nullable(inner) => {
            let inner = substitute(inner, substitutions);
            if inner.is_nullable() {
                inner
            } else {
                Type::Nullable(Box::new(inner))
            }
        }
        Type::Platform(inner) => {
            let inner = substitute(inner, substitutions);
            if matches!(inner, Type::Platform(_)) {
                inner
            } else {
                Type::Platform(Box::new(inner))
            }
        }
        Type::Array(inner) => Type::Array(Box::new(substitute(inner, substitutions))),
        Type::OutArray(inner) => Type::OutArray(Box::new(substitute(inner, substitutions))),
        Type::List(inner) => Type::List(Box::new(substitute(inner, substitutions))),
        Type::MutableList(inner) => Type::MutableList(Box::new(substitute(inner, substitutions))),
        Type::FlexibleList(inner) => Type::FlexibleList(Box::new(substitute(inner, substitutions))),
        Type::ThreadLocal(inner) => Type::ThreadLocal(Box::new(substitute(inner, substitutions))),
        Type::Parameterized { name, arguments } => Type::Parameterized {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|argument| TypeArgument {
                    variance: argument.variance,
                    ty: argument.ty.as_ref().map(|ty| substitute(ty, substitutions)),
                })
                .collect(),
        },
        Type::SuspendReceiverFunction {
            receiver,
            params,
            result,
        } => Type::SuspendReceiverFunction {
            receiver: Box::new(substitute(receiver, substitutions)),
            params: params
                .iter()
                .map(|ty| substitute(ty, substitutions))
                .collect(),
            result: Box::new(substitute(result, substitutions)),
        },
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => Type::ReceiverFunction {
            receiver: Box::new(substitute(receiver, substitutions)),
            params: params
                .iter()
                .map(|ty| substitute(ty, substitutions))
                .collect(),
            result: Box::new(substitute(result, substitutions)),
        },
        Type::Function { params, result } => Type::Function {
            params: params
                .iter()
                .map(|ty| substitute(ty, substitutions))
                .collect(),
            result: Box::new(substitute(result, substitutions)),
        },
        Type::SuspendFunction { params, result } => Type::SuspendFunction {
            params: params
                .iter()
                .map(|ty| substitute(ty, substitutions))
                .collect(),
            result: Box::new(substitute(result, substitutions)),
        },
        _ => ty.clone(),
    }
}

fn arguments(ty: &Type) -> Option<Vec<TypeArgument>> {
    match ty.base() {
        Type::Parameterized { arguments, .. } => Some(arguments.clone()),
        Type::List(inner) | Type::MutableList(inner) | Type::FlexibleList(inner) => {
            Some(vec![TypeArgument {
                variance: Variance::Invariant,
                ty: Some(inner.as_ref().clone()),
            }])
        }
        _ => None,
    }
}

fn class_substitutions(ty: &Type, class: &JavaClass) -> HashMap<String, Type> {
    let mut substitutions = HashMap::new();
    if let (Some(arguments), Some(library)) = (arguments(ty), &class.library) {
        for (parameter, argument) in library.type_parameters.iter().zip(arguments) {
            if let Some(ty) = argument.ty {
                substitutions.insert(parameter.name.clone(), ty);
            }
        }
    }
    substitutions
}

fn project_supertype(
    parent: &Type,
    substitutions: &HashMap<String, Type>,
    projections: &HashMap<String, TypeArgument>,
) -> Type {
    fn argument(
        pattern: &TypeArgument,
        substitutions: &HashMap<String, Type>,
        projections: &HashMap<String, TypeArgument>,
    ) -> TypeArgument {
        let Some(ty) = &pattern.ty else {
            return pattern.clone();
        };
        if let Type::Parameter { name, .. } = ty.base() {
            if let Some(projected) = projections.get(name) {
                if projected.ty.is_none() {
                    return TypeArgument {
                        variance: Variance::Invariant,
                        ty: None,
                    };
                }
                let variance = match (pattern.variance, projected.variance) {
                    (Variance::Invariant, variance) | (variance, Variance::Invariant) => variance,
                    (a, b) if a == b => a,
                    _ => {
                        return TypeArgument {
                            variance: Variance::Invariant,
                            ty: None,
                        };
                    }
                };
                return TypeArgument {
                    variance,
                    ty: projected.ty.clone(),
                };
            }
        }
        if projections.iter().any(|(name, projection)| {
            (projection.ty.is_none() || projection.variance != Variance::Invariant)
                && mentions(ty, name)
        }) {
            return TypeArgument {
                variance: Variance::Invariant,
                ty: None,
            };
        }
        TypeArgument {
            variance: pattern.variance,
            ty: Some(substitute(ty, substitutions)),
        }
    }
    match parent {
        Type::Parameterized { name, arguments } => Type::Parameterized {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|value| argument(value, substitutions, projections))
                .collect(),
        },
        Type::List(inner) | Type::MutableList(inner) | Type::FlexibleList(inner)
            if projections.iter().any(|(name, projection)| {
                (projection.ty.is_none() || projection.variance != Variance::Invariant)
                    && mentions(inner, name)
            }) =>
        {
            Type::Parameterized {
                name: "java/util/List".into(),
                arguments: vec![argument(
                    &TypeArgument {
                        variance: Variance::Invariant,
                        ty: Some(inner.as_ref().clone()),
                    },
                    substitutions,
                    projections,
                )],
            }
        }
        _ => substitute(parent, substitutions),
    }
}

fn direct_supertypes(cp: &Classpath, ty: &Type) -> Vec<Type> {
    let Some(owner) = relation_owner(ty) else {
        return Vec::new();
    };
    let Ok(class) = cp.load(owner) else {
        return Vec::new();
    };
    let substitutions = class_substitutions(ty, &class);
    if let Some(library) = &class.library {
        let projections = library
            .type_parameters
            .iter()
            .zip(arguments(ty).unwrap_or_default())
            .map(|(parameter, argument)| (parameter.name.clone(), argument))
            .collect::<HashMap<_, _>>();
        library
            .supertypes
            .iter()
            .map(|parent| project_supertype(parent, &substitutions, &projections))
            .collect()
    } else {
        class
            .super_name
            .iter()
            .chain(&class.interfaces)
            .map(|name| Type::Java(name.clone()))
            .collect()
    }
}

pub(super) fn as_supertype(cp: &Classpath, ty: &Type, target: &str) -> Option<Type> {
    let initial = match ty.base() {
        Type::Parameter { upper_bound, .. } => upper_bound.as_ref(),
        _ => ty,
    };
    let mut pending = vec![(initial.clone(), 0)];
    let mut seen = Vec::new();
    while let Some((ty, depth)) = pending.pop() {
        if depth > 64 || seen.len() >= 256 || seen.contains(&ty) {
            continue;
        }
        seen.push(ty.clone());
        if relation_owner(&ty) == Some(target) {
            return Some(ty);
        }
        pending.extend(
            direct_supertypes(cp, &ty)
                .into_iter()
                .map(|parent| (parent, depth + 1)),
        );
    }
    None
}

fn mentions(ty: &Type, name: &str) -> bool {
    match ty {
        Type::Parameter {
            name: parameter, ..
        } => parameter == name,
        Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::Array(inner)
        | Type::OutArray(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::FlexibleList(inner)
        | Type::ThreadLocal(inner) => mentions(inner, name),
        Type::Parameterized { arguments, .. } => arguments
            .iter()
            .any(|argument| argument.ty.as_ref().is_some_and(|ty| mentions(ty, name))),
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        }
        | Type::SuspendReceiverFunction {
            receiver,
            params,
            result,
        } => {
            mentions(receiver, name)
                || params.iter().any(|ty| mentions(ty, name))
                || mentions(result, name)
        }
        Type::Function { params, result } => {
            params.iter().any(|ty| mentions(ty, name)) || mentions(result, name)
        }
        _ => false,
    }
}

fn direct_parameter(ty: &Type, name: &str) -> bool {
    match ty {
        Type::Parameter {
            name: parameter, ..
        } => parameter == name,
        Type::Platform(inner) | Type::Nullable(inner) => direct_parameter(inner, name),
        _ => false,
    }
}

pub(super) fn projected_input(
    method: &JavaMethod,
    receiver: &Type,
    class: &JavaClass,
    cp: &Classpath,
) -> bool {
    let receiver = as_supertype(cp, receiver, &method.owner).unwrap_or_else(|| receiver.clone());
    let declaring = if method.owner == class.name {
        class.clone()
    } else {
        cp.load(&method.owner).unwrap_or_else(|_| class.clone())
    };
    let (Some(arguments), Some(library)) = (arguments(&receiver), &declaring.library) else {
        return false;
    };
    library
        .type_parameters
        .iter()
        .zip(arguments)
        .any(|(parameter, argument)| {
            if method.library.as_ref().is_some_and(|library| {
                library
                    .type_parameters
                    .iter()
                    .any(|own| own.name == parameter.name)
            }) {
                return false;
            }
            (argument.ty.is_none() || argument.variance == Variance::Out)
                && (method.kotlin_params.as_ref().is_some_and(|params| {
                    params
                        .iter()
                        .any(|(_, ty, _)| mentions(ty, &parameter.name))
                }) || method
                    .library
                    .as_ref()
                    .and_then(|library| library.receiver.as_ref())
                    .is_some_and(|receiver| mentions(receiver, &parameter.name)))
        })
}

pub(super) fn specialize_field(
    ty: &Type,
    receiver: &Type,
    owner: &str,
    cp: &Classpath,
    offset: usize,
) -> Result<Type> {
    let receiver = as_supertype(cp, receiver, owner).unwrap_or_else(|| receiver.clone());
    let class = cp.load(owner)?;
    let mut readable = class_substitutions(&receiver, &class);
    if let Some(library) = &class.library {
        let arguments = arguments(&receiver).unwrap_or_default();
        for (index, parameter) in library.type_parameters.iter().enumerate() {
            let argument = arguments.get(index);
            if argument.is_none_or(|arg| arg.ty.is_none() || arg.variance != Variance::Invariant)
                && mentions(ty, &parameter.name)
            {
                if !direct_parameter(ty, &parameter.name) {
                    return Err(Diagnostic::new(offset, "Nested generic Java field reads through projected receivers are unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "PROJECTED_JAVA_FIELD_UNSUPPORTED"));
                }
                if argument.is_none_or(|arg| arg.ty.is_none() || arg.variance == Variance::In) {
                    readable.insert(
                        parameter.name.clone(),
                        parameter
                            .bounds
                            .first()
                            .cloned()
                            .unwrap_or(Type::Java("java/lang/Object".into())),
                    );
                }
            }
        }
    }
    let mut result = substitute(ty, &readable);
    if matches!(ty, Type::Parameter { .. }) {
        while let Type::Nullable(inner) | Type::Platform(inner) = result {
            result = *inner;
        }
        if matches!(&result, Type::Parameter { upper_bound, .. } if upper_bound.is_nullable()) {
            return Err(Diagnostic::new(offset, "Non-null Java fields with nullable source type parameters require unsupported intersection types").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "NONNULL_JAVA_FIELD_PARAMETER_UNSUPPORTED"));
        }
    }
    Ok(result)
}

pub(super) fn specialize_receiver(
    mut method: JavaMethod,
    receiver: &Type,
    class: &JavaClass,
    cp: &Classpath,
) -> JavaMethod {
    let receiver = as_supertype(cp, receiver, &method.owner).unwrap_or_else(|| receiver.clone());
    let declaring = if method.owner == class.name {
        class.clone()
    } else {
        cp.load(&method.owner).unwrap_or_else(|_| class.clone())
    };
    let mut readable = class_substitutions(&receiver, &declaring);
    let mut writable = readable.clone();
    if let (Some(arguments), Some(library)) = (arguments(&receiver), &declaring.library) {
        for (parameter, argument) in library.type_parameters.iter().zip(arguments) {
            if method.library.as_ref().is_some_and(|library| {
                library
                    .type_parameters
                    .iter()
                    .any(|own| own.name == parameter.name)
            }) {
                continue;
            }
            if (argument.ty.is_none() || argument.variance != Variance::Invariant)
                && method.kotlin_result.as_ref().is_some_and(|ty| {
                    mentions(ty, &parameter.name) && !direct_parameter(ty, &parameter.name)
                })
            {
                method.unsupported = true;
            }
            if argument.ty.is_none() || argument.variance == Variance::Out {
                if method.kotlin_params.as_ref().is_some_and(|params| {
                    params
                        .iter()
                        .any(|(_, ty, _)| mentions(ty, &parameter.name))
                }) {
                    method.unsupported = true;
                }
                if argument.ty.is_none() {
                    readable.insert(
                        parameter.name.clone(),
                        parameter
                            .bounds
                            .first()
                            .cloned()
                            .unwrap_or(Type::Nullable(Box::new(Type::Java(
                                "java/lang/Object".into(),
                            )))),
                    );
                }
            } else if argument.variance == Variance::In {
                readable.insert(
                    parameter.name.clone(),
                    parameter
                        .bounds
                        .first()
                        .cloned()
                        .unwrap_or(Type::Nullable(Box::new(Type::Java(
                            "java/lang/Object".into(),
                        )))),
                );
            }
        }
    }
    if let Some(library) = &method.library {
        for parameter in &library.type_parameters {
            readable.remove(&parameter.name);
            writable.remove(&parameter.name);
        }
    }
    if let Some(params) = &mut method.kotlin_params {
        for (_, ty, _) in params {
            *ty = substitute(ty, &writable);
        }
    }
    if let Some(receiver) = method
        .library
        .as_mut()
        .and_then(|library| library.receiver.as_mut())
    {
        *receiver = substitute(receiver, &writable);
    }
    if let Some(result) = &mut method.kotlin_result {
        let precise_mapped_parameter = matches!(
            receiver.base(),
            Type::List(_) | Type::MutableList(_) | Type::FlexibleList(_)
        ) && matches!(result, Type::Platform(inner) if matches!(inner.as_ref(), Type::Parameter { .. }));
        if precise_mapped_parameter {
            if let Type::Platform(inner) = result {
                *result = substitute(inner, &readable);
            }
        } else {
            *result = substitute(result, &readable);
        }
    }
    method
}

fn substitute_method(mut method: JavaMethod, substitutions: &HashMap<String, Type>) -> JavaMethod {
    if let Some(params) = &mut method.kotlin_params {
        for (_, ty, _) in params {
            *ty = substitute(ty, substitutions);
        }
    }
    if let Some(result) = &mut method.kotlin_result {
        *result = substitute(result, substitutions);
    }
    if let Some(library) = &mut method.library {
        if let Some(receiver) = &mut library.receiver {
            *receiver = substitute(receiver, substitutions);
        }
    }
    method
}

pub(super) fn subtype_at(cp: &Classpath, actual: &Type, expected: &Type, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    if actual == expected || matches!(actual, Type::Java(name) if name == "kotlin/Nothing") {
        return true;
    }
    if actual == &Type::Null {
        return expected.is_nullable() || matches!(expected, Type::Platform(_));
    }
    if let Type::Platform(inner) = actual {
        return subtype_at(cp, inner, expected, depth + 1);
    }
    if let Type::Parameter { upper_bound, .. } = actual {
        if matches!(expected, Type::Platform(inner) if inner.as_ref() == actual) {
            return true;
        }
        return subtype_at(cp, upper_bound, expected, depth + 1);
    }
    if let Type::Platform(inner) = expected {
        let actual = if let Type::Nullable(inner) = actual {
            inner.as_ref()
        } else {
            actual
        };
        if matches!(inner.as_ref(), Type::Array(_) | Type::StringArray)
            && actual.primitive_array_element().is_none()
            && let (Some(actual), Some(expected)) = (actual.array_element(), inner.array_element())
        {
            return subtype_at(cp, &actual, &Type::Platform(Box::new(expected)), depth + 1);
        }
        return subtype_at(cp, actual, inner, depth + 1);
    }
    if let Type::Nullable(inner) = expected {
        let actual = if let Type::Nullable(inner) = actual {
            inner.as_ref()
        } else {
            actual
        };
        return subtype_at(cp, actual, inner, depth + 1);
    }
    if matches!(actual, Type::Nullable(_)) {
        return false;
    }
    if matches!(expected, Type::Java(name) if name == "java/lang/Object") {
        return true;
    }
    if actual.array_element().is_some()
        && matches!(expected, Type::Java(name) if matches!(name.as_str(), "java/lang/Cloneable" | "java/io/Serializable"))
    {
        return true;
    }
    if matches!((actual, expected), (Type::StringArray, Type::Array(element)) | (Type::Array(element), Type::StringArray) if element.as_ref() == &Type::String)
    {
        return true;
    }
    fn function_parts(ty: &Type) -> Option<(Vec<&Type>, &Type)> {
        match ty {
            Type::Function { params, result } | Type::SuspendFunction { params, result } => {
                Some((params.iter().collect(), result))
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            }
            | Type::SuspendReceiverFunction {
                receiver,
                params,
                result,
            } => {
                let mut all = vec![receiver.as_ref()];
                all.extend(params.iter());
                Some((all, result))
            }
            _ => None,
        }
    }
    if matches!(
        actual,
        Type::SuspendFunction { .. } | Type::SuspendReceiverFunction { .. }
    ) != matches!(
        expected,
        Type::SuspendFunction { .. } | Type::SuspendReceiverFunction { .. }
    ) {
        return false;
    }
    if let (Some((a, ar)), Some((e, er))) = (function_parts(actual), function_parts(expected)) {
        return a.len() == e.len()
            && a.iter()
                .zip(e)
                .all(|(a, e)| subtype_at(cp, e, a, depth + 1))
            && subtype_at(cp, ar, er, depth + 1);
    }
    if let Type::OutArray(expected) = expected {
        return actual.primitive_array_element().is_none()
            && actual
                .array_element()
                .is_some_and(|actual| subtype_at(cp, &actual, expected, depth + 1));
    }
    if matches!(actual, Type::OutArray(_)) && matches!(expected, Type::Array(_) | Type::StringArray)
    {
        return false;
    }
    if let (Type::Array(a), Type::Array(e)) = (actual, expected) {
        return a == e;
    }
    if let (
        Type::ReceiverFunction {
            receiver: ar,
            params: ap,
            result: av,
        },
        Type::ReceiverFunction {
            receiver: er,
            params: ep,
            result: ev,
        },
    ) = (actual, expected)
    {
        return subtype_at(cp, er, ar, depth + 1)
            && ap.len() == ep.len()
            && ap
                .iter()
                .zip(ep)
                .all(|(a, e)| subtype_at(cp, e, a, depth + 1))
            && subtype_at(cp, av, ev, depth + 1);
    }
    if let (
        Type::Function {
            params: actual,
            result: ar,
        },
        Type::Function {
            params: expected,
            result: er,
        },
    ) = (actual, expected)
    {
        return actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| subtype_at(cp, e, a, depth + 1))
            && subtype_at(cp, ar, er, depth + 1);
    }
    if matches!((actual, expected), (Type::List(_), Type::MutableList(_))) {
        return false;
    }
    let (Some(actual_owner), Some(expected_owner)) =
        (relation_owner(actual), relation_owner(expected))
    else {
        return false;
    };
    if actual_owner != expected_owner {
        return as_supertype(cp, actual, expected_owner)
            .is_some_and(|parent| subtype_at(cp, &parent, expected, depth + 1));
    }
    let Some(expected_arguments) = arguments(expected) else {
        return true;
    };
    let Some(actual_arguments) = arguments(actual) else {
        return false;
    };
    if actual_arguments.len() != expected_arguments.len() {
        return false;
    }
    let declared = cp.load(actual_owner).ok().and_then(|class| class.library);
    actual_arguments
        .iter()
        .zip(&expected_arguments)
        .enumerate()
        .all(|(index, (actual, expected_argument))| {
            let Some(expected_ty) = &expected_argument.ty else {
                return true;
            };
            let Some(actual_ty) = &actual.ty else {
                return false;
            };
            let variance = if expected_argument.variance != Variance::Invariant {
                expected_argument.variance
            } else if matches!(expected, Type::List(_)) || matches!(expected, Type::FlexibleList(_))
            {
                Variance::Out
            } else {
                declared
                    .as_ref()
                    .and_then(|library| library.type_parameters.get(index))
                    .map_or(Variance::Invariant, |parameter| parameter.variance)
            };
            match variance {
                Variance::Invariant => {
                    actual.variance == Variance::Invariant
                        && subtype_at(cp, actual_ty, expected_ty, depth + 1)
                        && subtype_at(cp, expected_ty, actual_ty, depth + 1)
                }
                Variance::Out => {
                    actual.variance != Variance::In
                        && subtype_at(cp, actual_ty, expected_ty, depth + 1)
                }
                Variance::In => {
                    actual.variance != Variance::Out
                        && subtype_at(cp, expected_ty, actual_ty, depth + 1)
                }
            }
        })
}

#[cfg(test)]
pub(super) fn validate_type_arguments(ty: &Type, cp: &Classpath, offset: usize) -> Result<()> {
    validate_type_arguments_with_classes(ty, cp, &HashMap::new(), offset)
}
pub(super) fn validate_type_arguments_with_classes(
    ty: &Type,
    cp: &Classpath,
    classes: &HashMap<String, JavaClass>,
    offset: usize,
) -> Result<()> {
    fn validate(
        ty: &Type,
        cp: &Classpath,
        classes: &HashMap<String, JavaClass>,
        offset: usize,
        depth: usize,
    ) -> Result<()> {
        if depth > 64 {
            return Err(Diagnostic::new(
                offset,
                "Generic type validation exceeds the current nesting limit",
            ));
        }
        match ty {
            Type::Java(name) => {
                let class = classes
                    .get(name)
                    .cloned()
                    .map(Ok)
                    .unwrap_or_else(|| cp.load(name))
                    .map_err(|error| error.for_type(name))?;
                if class
                    .library
                    .as_ref()
                    .is_some_and(|library| !library.type_parameters.is_empty())
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Source generic classifiers require explicit type arguments",
                    ));
                }
            }
            Type::Parameterized { name, arguments } => {
                let class = classes
                    .get(name)
                    .cloned()
                    .map(Ok)
                    .unwrap_or_else(|| cp.load(name))
                    .map_err(|error| error.for_type(name))?;
                let parameters = class
                    .library
                    .as_ref()
                    .map(|library| library.type_parameters.as_slice())
                    .unwrap_or(&[]);
                if parameters.len() != arguments.len() {
                    return Err(Diagnostic::new(
                        offset,
                        format!(
                            "Wrong number of type arguments for {name}: expected {}, found {}",
                            parameters.len(),
                            arguments.len()
                        ),
                    ));
                }
                let substitutions = parameters
                    .iter()
                    .zip(arguments)
                    .filter_map(|(parameter, argument)| {
                        argument
                            .ty
                            .as_ref()
                            .map(|ty| (parameter.name.clone(), ty.clone()))
                    })
                    .collect::<HashMap<_, _>>();
                for (parameter, argument) in parameters.iter().zip(arguments) {
                    let Some(ty) = &argument.ty else {
                        continue;
                    };
                    if matches!(
                        (parameter.variance, argument.variance),
                        (Variance::In, Variance::Out) | (Variance::Out, Variance::In)
                    ) {
                        return Err(Diagnostic::new(
                            offset,
                            "Type projection conflicts with declaration-site variance",
                        )
                        .with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "TYPE_PROJECTION_CONFLICT",
                        ));
                    }
                    validate(ty, cp, classes, offset, depth + 1)?;
                    for bound in &parameter.bounds {
                        let bound = substitute(bound, &substitutions);
                        let bound = if class.kotlin_class || matches!(bound, Type::Platform(_)) {
                            bound
                        } else {
                            Type::Platform(Box::new(bound))
                        };
                        if !subtype_at(cp, ty, &bound, 0) {
                            return Err(Diagnostic::new(
                                offset,
                                format!("Type argument violates upper bound of {}", parameter.name),
                            )
                            .with_kind(
                                crate::DiagnosticKind::InvalidKotlin,
                                "TYPE_ARGUMENT_BOUND_VIOLATED",
                            ));
                        }
                    }
                }
            }
            Type::Nullable(inner)
            | Type::Array(inner)
            | Type::OutArray(inner)
            | Type::List(inner)
            | Type::MutableList(inner)
            | Type::FlexibleList(inner)
            | Type::ThreadLocal(inner) => validate(inner, cp, classes, offset, depth + 1)?,
            Type::Platform(_) => {}
            Type::Parameter { .. } => {}
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            }
            | Type::SuspendReceiverFunction {
                receiver,
                params,
                result,
            } => {
                validate(receiver, cp, classes, offset, depth + 1)?;
                for parameter in params {
                    validate(parameter, cp, classes, offset, depth + 1)?;
                }
                validate(result, cp, classes, offset, depth + 1)?;
            }
            Type::Function { params, result } => {
                for parameter in params {
                    validate(parameter, cp, classes, offset, depth + 1)?;
                }
                validate(result, cp, classes, offset, depth + 1)?;
            }
            _ => {}
        }
        Ok(())
    }
    validate(ty, cp, classes, offset, 0)
}

impl Checker<'_> {
    pub(super) fn subtype(&self, actual: &Type, expected: &Type) -> bool {
        self.subtype_at(actual, expected, 0)
    }

    fn subtype_at(&self, actual: &Type, expected: &Type, depth: usize) -> bool {
        subtype_at(self.classpath, actual, expected, depth)
    }

    pub(super) fn validate_type_arguments(&self, ty: &Type, offset: usize) -> Result<()> {
        validate_type_arguments_with_classes(ty, self.classpath, self.classes, offset)
    }

    fn infer_constraints(
        &self,
        pattern: &Type,
        actual: &Type,
        substitutions: &mut HashMap<String, Type>,
        fixed: &HashSet<String>,
        depth: usize,
    ) -> bool {
        if depth > 64 {
            return false;
        }
        match pattern {
            Type::Parameter { name, .. } => {
                let actual = actual.base();
                if let Some(existing) = substitutions.get(name).cloned() {
                    if fixed.contains(name) {
                        return self.subtype(actual, &existing);
                    }
                    if existing == Type::Null || actual == &Type::Null {
                        let other = if existing == Type::Null {
                            actual.clone()
                        } else {
                            existing
                        };
                        substitutions.insert(
                            name.clone(),
                            if other.is_nullable() {
                                other
                            } else {
                                Type::Nullable(Box::new(other))
                            },
                        );
                        return true;
                    }
                    if self.subtype(actual, &existing) {
                        return true;
                    }
                    if self.subtype(&existing, actual) {
                        substitutions.insert(name.clone(), actual.clone());
                        return true;
                    }
                    let nullable =
                        existing.is_nullable() || actual.is_nullable() || actual == &Type::Null;
                    let mut common = Type::Java("java/lang/Object".into());
                    let mut candidates = direct_supertypes(self.classpath, existing.base());
                    for _ in 0..64 {
                        let Some(candidate) = candidates.pop() else {
                            break;
                        };
                        if self.subtype(actual, &candidate) {
                            common = candidate;
                            break;
                        }
                        candidates.extend(direct_supertypes(self.classpath, &candidate));
                    }
                    if nullable {
                        common = Type::Nullable(Box::new(common));
                    }
                    substitutions.insert(name.clone(), common);
                } else {
                    substitutions.insert(name.clone(), actual.clone());
                }
                true
            }
            Type::Nullable(inner) | Type::Platform(inner) => {
                let actual = if let Type::Nullable(actual) = actual.base() {
                    actual.as_ref()
                } else {
                    actual.base()
                };
                self.infer_constraints(inner, actual, substitutions, fixed, depth + 1)
            }
            Type::Array(element) | Type::OutArray(element) => match actual.base() {
                Type::Array(actual) | Type::OutArray(actual) => {
                    self.infer_constraints(element, actual, substitutions, fixed, depth + 1)
                }
                Type::StringArray => {
                    self.infer_constraints(element, &Type::String, substitutions, fixed, depth + 1)
                }
                _ => true,
            },
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            }
            | Type::SuspendReceiverFunction {
                receiver,
                params,
                result,
            } => {
                if let Type::ReceiverFunction {
                    receiver: ar,
                    params: ap,
                    result: av,
                }
                | Type::SuspendReceiverFunction {
                    receiver: ar,
                    params: ap,
                    result: av,
                } = actual.base()
                {
                    self.infer_constraints(receiver, ar, substitutions, fixed, depth + 1)
                        && params.len() == ap.len()
                        && params.iter().zip(ap).all(|(p, a)| {
                            self.infer_constraints(p, a, substitutions, fixed, depth + 1)
                        })
                        && self.infer_constraints(result, av, substitutions, fixed, depth + 1)
                } else {
                    true
                }
            }
            Type::Function { params, result } => {
                let Type::Function {
                    params: actual_params,
                    result: actual_result,
                } = actual.base()
                else {
                    return true;
                };
                params.len() == actual_params.len()
                    && params
                        .iter()
                        .zip(actual_params)
                        .all(|(p, a)| self.infer_constraints(p, a, substitutions, fixed, depth + 1))
                    && self.infer_constraints(
                        result,
                        actual_result,
                        substitutions,
                        fixed,
                        depth + 1,
                    )
            }
            _ => {
                let Some(pattern_args) = arguments(pattern) else {
                    return true;
                };
                let Some(owner) = type_owner(pattern) else {
                    return true;
                };
                let actual = as_supertype(self.classpath, actual.base(), owner)
                    .unwrap_or_else(|| actual.base().clone());
                let Some(actual_args) = arguments(&actual) else {
                    return true;
                };
                pattern_args.len() == actual_args.len()
                    && pattern_args
                        .iter()
                        .zip(&actual_args)
                        .all(|(p, a)| match (&p.ty, &a.ty) {
                            (Some(pattern), Some(actual)) => self.infer_constraints(
                                pattern,
                                actual,
                                substitutions,
                                fixed,
                                depth + 1,
                            ),
                            _ => true,
                        })
            }
        }
    }

    fn infer_lambda_result(
        &mut self,
        pattern: &Type,
        expr: &Expr,
        substitutions: &mut HashMap<String, Type>,
        fixed: &HashSet<String>,
        parameters: &HashSet<String>,
        label: Option<&str>,
    ) -> Result<()> {
        if let ExprKind::Labeled(label, inner) = &expr.kind {
            return self.infer_lambda_result(
                pattern,
                inner,
                substitutions,
                fixed,
                parameters,
                Some(label),
            );
        }
        if let Type::Nullable(inner) = pattern.base()
            && matches!(
                inner.base(),
                Type::Function { .. }
                    | Type::ReceiverFunction { .. }
                    | Type::SuspendReceiverFunction { .. }
            )
        {
            return self.infer_lambda_result(inner, expr, substitutions, fixed, parameters, label);
        }
        if matches!(expr.kind, ExprKind::CallableReference { .. })
            && let Type::Function { params, result } = pattern.base()
            && parameters
                .iter()
                .any(|name| mentions(&substitute(result, substitutions), name))
            && !params.iter().any(|param| {
                parameters
                    .iter()
                    .any(|name| mentions(&substitute(param, substitutions), name))
            })
        {
            let mut probe = self.clone();
            let count = self.lambdas.borrow().len();
            let inferred = probe.with_reference_target(expr, pattern, |checker, lambda, _| {
                checker.infer_lambda_result(
                    pattern,
                    lambda,
                    substitutions,
                    fixed,
                    parameters,
                    label,
                )
            });
            self.lambdas.borrow_mut().truncate(count);
            inferred?;
            return Ok(());
        }
        if !matches!(
            expr.kind,
            ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
        ) {
            return Ok(());
        }
        if let Type::ReceiverFunction {
            receiver,
            params,
            result,
        }
        | Type::SuspendReceiverFunction {
            receiver,
            params,
            result,
        } = pattern.base()
        {
            let receiver = substitute(receiver, substitutions);
            let old_receivers = self.extension_receivers.clone();
            let old_labels = self.receiver_labels.clone();
            let saved_contract_registration = self.contract_registration;
            let saved_contract_facts = self.contract_facts.clone();
            let next = self.next;
            let initialized = self.initialized.clone();
            let slot = self.allocate_typed_slot(&receiver, expr.offset)?;
            let receiver = Local {
                slot,
                ty: receiver,
                mutable: false,
            };
            self.extension_receivers.insert(0, receiver.clone());
            self.initialized.insert(slot);
            if let Some(label) = label {
                self.receiver_labels.insert(label.into(), receiver);
            }
            let function = Type::Function {
                params: params.clone(),
                result: result.clone(),
            };
            let inferred =
                self.infer_lambda_result(&function, expr, substitutions, fixed, parameters, label);
            self.extension_receivers = old_receivers;
            self.receiver_labels = old_labels;
            self.contract_facts = saved_contract_facts;
            self.contract_registration = saved_contract_registration;
            self.next = next;
            self.initialized = initialized;
            return inferred;
        }
        let Type::Function { params, result } = pattern.base() else {
            if let Some((_, function)) = self.sam_function(pattern, expr.offset)? {
                return self.infer_lambda_result(
                    &function,
                    expr,
                    substitutions,
                    fixed,
                    parameters,
                    label,
                );
            }
            return Ok(());
        };
        let (declared, body) = match &expr.kind {
            ExprKind::Lambda(value) => (
                None,
                vec![syntax::Statement {
                    offset: value.offset,
                    kind: StatementKind::Expression(value.as_ref().clone()),
                }],
            ),
            ExprKind::LambdaWithParams { params, body } => (params.clone(), body.clone()),
            _ => return Ok(()),
        };
        if self.nonlocal_result.is_some()
            && body
                .last()
                .is_some_and(|statement| matches!(statement.kind, StatementKind::Return(_)))
        {
            return Ok(());
        }
        let names = match declared {
            Some(declared) => {
                if declared.len() != params.len() {
                    return Err(Diagnostic::new(
                        expr.offset,
                        "Lambda parameter count mismatch",
                    ));
                }
                for (parameter, pattern) in declared.iter().zip(params) {
                    if let Some(ty) = &parameter.ty {
                        let actual = self.resolve(ty, expr.offset)?;
                        if !self.infer_constraints(pattern, &actual, substitutions, fixed, 0) {
                            return Err(Diagnostic::new(
                                expr.offset,
                                "Conflicting explicit lambda parameter constraints",
                            ));
                        }
                    }
                }
                declared
                    .into_iter()
                    .enumerate()
                    .map(|(index, parameter)| {
                        if parameter.discard {
                            format!("<unused{index}>")
                        } else {
                            parameter.name
                        }
                    })
                    .collect::<Vec<_>>()
            }
            None if params.len() == 1 => vec!["it".into()],
            None if params.is_empty() => Vec::new(),
            None => {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Multiple lambda parameters must be declared explicitly",
                ));
            }
        };
        let output = substitute(result, substitutions);
        if !parameters.iter().any(|name| mentions(&output, name)) {
            return Ok(());
        }
        let params = params
            .iter()
            .map(|ty| substitute(ty, substitutions))
            .collect::<Vec<_>>();
        if params
            .iter()
            .any(|ty| parameters.iter().any(|name| mentions(ty, name)))
        {
            return Err(Diagnostic::new(
                expr.offset,
                "Cannot infer lambda parameter types from the available constraints",
            ));
        }
        let saved_contract_registration = self.contract_registration;
        let saved_contract_facts = self.contract_facts.clone();
        let next = self.next;
        let initialized = self.initialized.clone();
        let refinements = self.refinements.clone();
        let cells = self.cells.clone();
        let cell_names = self.cell_names.clone();
        let changing_cells = self.changing_cells.clone();
        let count = self.lambdas.borrow().len();
        let allow_return = self.allow_return;
        self.allow_return = false;
        let scopes_len = self.scopes.len();
        self.scopes.push(HashMap::new());
        let inferred = (|| {
            for (name, ty) in names.iter().zip(params) {
                self.declare(name, ty, false, expr.offset)?;
            }
            let Some((last, prefix)) = body.split_last() else {
                return Ok(Type::Unit);
            };
            let (_, returns) = self.block_scoped(prefix, false)?;
            if returns {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Cannot infer a generic lambda result after unconditional termination",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "LAMBDA_ABRUPT_RESULT_INFERENCE_UNSUPPORTED",
                ));
            }
            match &last.kind {
                StatementKind::Expression(value) => Ok(self.value(value)?.ty),
                StatementKind::Return(_) => Err(Diagnostic::new(expr.offset, "Generic lambda results with unconditional nonlocal returns require unsupported Nothing inference").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "LAMBDA_ABRUPT_RESULT_INFERENCE_UNSUPPORTED")),
                _ => Ok(Type::Unit),
            }
        })();
        self.scopes.truncate(scopes_len);
        self.allow_return = allow_return;
        self.contract_facts = saved_contract_facts;
        self.contract_registration = saved_contract_registration;
        self.next = next;
        self.initialized = initialized;
        self.refinements = refinements;
        self.cells = cells;
        self.cell_names = cell_names;
        self.changing_cells = changing_cells;
        self.lambdas.borrow_mut().truncate(count);
        let inferred = inferred?;
        if !self.infer_constraints(result, &inferred, substitutions, fixed, 0) {
            return Err(Diagnostic::new(
                expr.offset,
                "Conflicting generic lambda result constraints",
            ));
        }
        Ok(())
    }

    pub(super) fn specialize_method_expected(
        &mut self,
        method: &JavaMethod,
        args: &[Expr],
        explicit: &[Type],
        expected: Option<&Type>,
        offset: usize,
    ) -> Result<JavaMethod> {
        self.specialize_method_named(
            method,
            args,
            explicit,
            offset,
            self.method_named(method),
            expected,
        )
    }

    pub(super) fn specialize_source(
        &mut self,
        id: usize,
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
    ) -> Result<(Vec<(String, Type)>, Type)> {
        let (params, result) = self
            .signatures
            .get(id)
            .ok_or_else(|| Diagnostic::new(offset, "Source function signature is unavailable"))?;
        let type_parameters = self.source_type_params.get(id).cloned().unwrap_or_default();
        if type_parameters.is_empty() {
            return if explicit.is_empty() {
                Ok((params.clone(), result.clone()))
            } else {
                Err(Diagnostic::new(
                    offset,
                    "Source function has no type parameters",
                ))
            };
        }
        let type_parameters = type_parameters
            .into_iter()
            .map(|mut parameter| {
                parameter.bounds = parameter
                    .bounds
                    .iter()
                    .map(|bound| self.resolve(bound, offset))
                    .collect::<Result<Vec<_>>>()?;
                Ok(parameter)
            })
            .collect::<Result<Vec<_>>>()?;
        let method = JavaMethod {
            owner: self.lambda_owner.clone(),
            name: self.function_names.get(id).cloned().unwrap_or_default(),
            descriptor: String::new(),
            access: 9,
            unsupported: false,
            source_property: None,
            kotlin_internal: false,
            kotlin_params: Some(
                params
                    .iter()
                    .enumerate()
                    .map(|(index, (name, ty))| {
                        (
                            name.clone(),
                            ty.clone(),
                            self.defaults
                                .get(id)
                                .and_then(|defaults| defaults.get(index))
                                .is_some_and(Option::is_some),
                        )
                    })
                    .collect(),
            ),
            kotlin_result: Some(result.clone()),
            library: Some(crate::classpath::LibraryMethod {
                compose: None,
                stable_result: false,
                default_code: None,
                vararg_index: None,
                contracts: Vec::new(),
                value_dispatch: false,
                suspend_: false,
                kotlin_declared: true,
                has_annotations: false,
                parameter_modes: Vec::new(),
                reified_arguments: Vec::new(),
                abstract_: false,
                builtin_array: false,
                source_name: self.function_names.get(id).cloned().unwrap_or_default(),
                type_parameters,
                receiver: None,
                inline: false,
                infix: false,
                operator: false,
                kotlin_public: true,
                code: None,
            }),
        };
        let method = self.specialize_method_named(&method, args, explicit, offset, true, None)?;
        Ok((
            method
                .kotlin_params
                .unwrap()
                .into_iter()
                .map(|(name, ty, _)| (name, ty))
                .collect(),
            method.kotlin_result.unwrap(),
        ))
    }

    fn argument_with_result_hint(
        &mut self,
        arg: &Expr,
        pattern: &Type,
        hints: &HashMap<String, Type>,
        parameters: &[syntax::TypeParameter],
    ) -> Result<Value> {
        if hints.is_empty() || !matches!(arg.kind, ExprKind::Call(..) | ExprKind::Binary(..)) {
            return self.value(arg);
        }
        let expected = substitute(pattern, hints);
        if expected == *pattern || parameters.iter().any(|p| mentions(&expected, &p.name)) {
            return self.value(arg);
        }
        let previous = self.clone();
        let lambda_count = self.lambdas.borrow().len();
        let original = self.value(arg);
        if original.as_ref().is_ok_and(|v| !unresolved_list(&v.ty)) {
            return original;
        }
        *self = previous;
        self.lambdas.borrow_mut().truncate(lambda_count);
        self.argument_value(arg, &expected)
    }
    fn specialize_method_named(
        &mut self,
        method: &JavaMethod,
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
        named: bool,
        expected: Option<&Type>,
    ) -> Result<JavaMethod> {
        let Some(library) = &method.library else {
            return if explicit.is_empty() {
                Ok(method.clone())
            } else {
                Err(Diagnostic::new(
                    offset,
                    "Declaration has no type parameters",
                ))
            };
        };
        let labeled = member_extensions::call_labels(&library.source_name, args);
        let args = labeled.as_slice();
        if library.type_parameters.is_empty() {
            return if explicit.is_empty() {
                Ok(method.clone())
            } else {
                Err(Diagnostic::new(
                    offset,
                    "Declaration has no type parameters",
                ))
            };
        }
        if !explicit.is_empty() && explicit.len() != library.type_parameters.len() {
            return Err(Diagnostic::new(offset, "Wrong number of type arguments"));
        }
        let mut substitutions = HashMap::new();
        for (parameter, ty) in library.type_parameters.iter().zip(explicit) {
            substitutions.insert(parameter.name.clone(), self.resolve(ty, offset)?);
        }
        let fixed = substitutions.keys().cloned().collect::<HashSet<_>>();
        let mut result_hints = HashMap::new();
        if method.name != "<init>"
            && let (Some(expected), Some(result)) = (expected, &method.kotlin_result)
        {
            let pattern = type_owner(expected)
                .and_then(|owner| as_supertype(self.classpath, result, owner))
                .unwrap_or_else(|| result.clone());
            if !self.infer_constraints(&pattern, expected, &mut result_hints, &HashSet::new(), 0) {
                result_hints.clear();
            }
        }
        let mut captures = HashMap::<String, TypeArgument>::new();
        let mut input_types = HashMap::<String, Vec<Type>>::new();
        let params = method
            .kotlin_params
            .as_ref()
            .ok_or_else(|| Diagnostic::new(offset, "Generic method lacks signature information"))?;
        let mapping = Self::argument_mapping(
            &params
                .iter()
                .map(|p| (p.0.clone(), p.1.clone()))
                .collect::<Vec<_>>(),
            &params.iter().map(|p| p.2).collect::<Vec<_>>(),
            args,
            named,
            offset,
        )?;
        let expected_result =
            expected
                .zip(method.kotlin_result.as_ref())
                .map(|(expected, result)| {
                    let expected = match (result.base(), expected.base()) {
                        (Type::Parameter { .. } | Type::Nullable(_), _) => expected,
                        (_, Type::Nullable(inner)) => inner.as_ref(),
                        _ => expected,
                    };
                    let pattern = if method.name == "<init>" {
                        type_owner(expected)
                            .and_then(|owner| as_supertype(self.classpath, result, owner))
                            .unwrap_or_else(|| result.clone())
                    } else {
                        result.clone()
                    };
                    (pattern, expected.clone())
                });
        if method.name == "<init>"
            && let Some((pattern, expected)) = &expected_result
        {
            let mut contextual = HashMap::new();
            if !self.infer_constraints(pattern, expected, &mut contextual, &HashSet::new(), 0) {
                return Err(Diagnostic::new(
                    offset,
                    "Conflicting expected constructor type",
                ));
            }
            for (name, ty) in contextual {
                substitutions.entry(name).or_insert(ty);
            }
        }
        let mut postponed = Vec::new();
        for (arg, index) in args.iter().zip(&mapping) {
            let arg = if let ExprKind::NamedArgument(_, expr) | ExprKind::TrailingLambda(expr) =
                &arg.kind
            {
                expr.as_ref()
            } else {
                arg
            };
            if matches!(
                arg.kind,
                ExprKind::Lambda(_)
                    | ExprKind::LambdaWithParams { .. }
                    | ExprKind::CallableReference { .. }
                    | ExprKind::Labeled(..)
            ) {
                continue;
            }
            if matches!(&arg.kind, ExprKind::VarargArray { elements, .. } if elements.is_empty()) {
                continue;
            }
            if let ExprKind::VarargArray { array, elements } = &arg.kind
                && let Some(pattern) = array.array_element()
                && !matches!(pattern.base(), Type::Parameter { .. })
                && library
                    .type_parameters
                    .iter()
                    .any(|p| !fixed.contains(&p.name) && mentions(&pattern, &p.name))
            {
                if library
                    .type_parameters
                    .iter()
                    .any(|p| p.only_input_types && mentions(&pattern, &p.name))
                {
                    return Err(Diagnostic::new(
                        offset,
                        "OnlyInputTypes inference through a nested vararg is unsupported",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "ONLY_INPUT_TYPES_SHAPE_UNSUPPORTED",
                    ));
                }
                for element in elements {
                    let actual = if let ExprKind::Unary(op, spread) = &element.kind
                        && op == "*"
                    {
                        self.value(spread)?.ty.array_element().ok_or_else(|| {
                            Diagnostic::new(spread.offset, "Spread argument requires an array")
                        })?
                    } else {
                        self.argument_with_result_hint(
                            element,
                            &pattern,
                            &result_hints,
                            &library.type_parameters,
                        )?
                        .ty
                    };
                    if !self.infer_constraints(&pattern, &actual, &mut substitutions, &fixed, 0) {
                        return Err(Diagnostic::new(
                            element.offset,
                            "Conflicting generic vararg element constraints",
                        ));
                    }
                }
                continue;
            }
            let specialized_array;
            let arg = if let ExprKind::VarargArray { array, elements } = &arg.kind {
                let fixed_types = substitutions
                    .iter()
                    .filter(|(name, _)| fixed.contains(*name))
                    .map(|(name, ty)| (name.clone(), ty.clone()))
                    .collect::<HashMap<_, _>>();
                specialized_array = Expr {
                    offset: arg.offset,
                    kind: ExprKind::VarargArray {
                        array: substitute(array, &fixed_types),
                        elements: elements.clone(),
                    },
                };
                &specialized_array
            } else {
                arg
            };
            let fixed_types = substitutions
                .iter()
                .filter(|(name, _)| {
                    fixed.contains(*name) || method.name == "<init>" && expected.is_some()
                })
                .map(|(name, ty)| (name.clone(), ty.clone()))
                .collect::<HashMap<_, _>>();
            let expected_argument = substitute(&params[*index].1, &fixed_types);
            let value = if (!fixed.is_empty() || method.name == "<init>")
                && !library.type_parameters.iter().any(|parameter| {
                    !fixed.contains(&parameter.name)
                        && mentions(&expected_argument, &parameter.name)
                }) {
                if method.name == "<init>" && fixed.is_empty() {
                    self.constructor_argument_with_context(arg, &expected_argument)?
                } else {
                    self.argument_value(arg, &expected_argument)?
                }
            } else if args.len() > 1
                && matches!(arg.kind, ExprKind::Call(..))
                && method.name != "<init>"
                && !library.type_parameters.iter().any(|p| p.only_input_types)
            {
                let saved = self.clone();
                let lambdas = self.lambdas.borrow().len();
                let checked = self.argument_with_result_hint(
                    arg,
                    &params[*index].1,
                    &result_hints,
                    &library.type_parameters,
                );
                let needs_context = match &checked {
                    Ok(value) => unresolved_list(&value.ty),
                    Err(error) => error.code == "TYPE_PARAMETER_INFERENCE_INCOMPLETE",
                };
                if needs_context {
                    *self = saved;
                    self.lambdas.borrow_mut().truncate(lambdas);
                    postponed.push((arg.clone(), *index));
                    continue;
                }
                checked?
            } else {
                self.argument_with_result_hint(
                    arg,
                    &params[*index].1,
                    &result_hints,
                    &library.type_parameters,
                )?
            };
            if *index == 0
                && library.receiver.is_some()
                && let (
                    Type::Parameterized {
                        name,
                        arguments: patterns,
                    },
                    Type::Parameterized {
                        name: actual_name,
                        arguments: actual,
                    },
                ) = (params[*index].1.base(), value.ty.base())
                && name == actual_name
            {
                for (pattern, projection) in patterns.iter().zip(actual) {
                    let Some(Type::Parameter { name, .. }) = pattern.ty.as_ref() else {
                        continue;
                    };
                    if projection.variance == Variance::Invariant && projection.ty.is_some() {
                        continue;
                    }
                    let star = projection.ty.is_none();
                    if (!star && projection.variance != Variance::Out)
                        || fixed.contains(name)
                        || star
                            && (patterns
                                .iter()
                                .filter(|pattern| {
                                    pattern.ty.as_ref().is_some_and(|ty| mentions(ty, name))
                                })
                                .count()
                                != 1
                                || library
                                    .type_parameters
                                    .iter()
                                    .any(|p| p.name == *name && p.reified))
                        || params.iter().skip(1).any(|(_, ty, _)| mentions(ty, name))
                        || captures
                            .get(name)
                            .is_some_and(|previous| previous != projection)
                    {
                        return Err(Diagnostic::new(offset, "Projected extension receiver requires unsupported writable or conflicting capture semantics")
                            .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "PROJECTED_EXTENSION_CAPTURE_UNSUPPORTED"));
                    }
                    if star {
                        substitutions.insert(
                            name.clone(),
                            Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
                        );
                    }
                    captures.insert(name.clone(), projection.clone());
                }
            }
            for parameter in library
                .type_parameters
                .iter()
                .filter(|p| p.only_input_types && !fixed.contains(&p.name))
            {
                let pattern = &params[*index].1;
                if !mentions(pattern, &parameter.name) {
                    continue;
                }
                let input = match pattern {
                    Type::Parameter { name, .. } if name == &parameter.name => {
                        Some(value.ty.clone())
                    }
                    Type::Array(inner) | Type::OutArray(inner)
                        if matches!(inner.as_ref(), Type::Parameter { name, .. } if name == &parameter.name)
                            && value.ty.base().primitive_array_element().is_none() =>
                    {
                        value.ty.base().array_element().map(|ty| ty.base().clone())
                    }
                    _ => None,
                };
                let Some(input) = input else {
                    return Err(Diagnostic::new(offset, "OnlyInputTypes inference through nested or captured parameter types is unsupported")
                        .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "ONLY_INPUT_TYPES_SHAPE_UNSUPPORTED"));
                };
                input_types
                    .entry(parameter.name.clone())
                    .or_default()
                    .push(input);
            }
            if method.name == "<init>"
                && expected.is_some()
                && value.ty == Type::Null
                && matches!(params[*index].1, Type::Nullable(_) | Type::Platform(_))
            {
                let contextual = substitute(&params[*index].1, &substitutions);
                if (contextual.is_nullable() || matches!(contextual, Type::Platform(_)))
                    && !library
                        .type_parameters
                        .iter()
                        .any(|parameter| mentions(&contextual, &parameter.name))
                {
                    continue;
                }
            }
            if !self.infer_constraints(&params[*index].1, &value.ty, &mut substitutions, &fixed, 0)
            {
                return Err(Diagnostic::new(
                    arg.offset,
                    "Conflicting generic type constraints",
                ));
            }
        }
        for (arg, index) in postponed {
            let mut hints = result_hints.clone();
            hints.extend(substitutions.clone());
            let parameter = &params[index].1;
            let value =
                self.argument_with_result_hint(&arg, parameter, &hints, &library.type_parameters)?;
            if !self.infer_constraints(parameter, &value.ty, &mut substitutions, &fixed, 0) {
                return Err(Diagnostic::new(
                    arg.offset,
                    "Conflicting generic type constraints",
                ));
            }
        }
        let parameter_names = library
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
        for (arg, index) in args.iter().zip(&mapping) {
            let arg = if let ExprKind::NamedArgument(_, expr) | ExprKind::TrailingLambda(expr) =
                &arg.kind
            {
                expr.as_ref()
            } else {
                arg
            };
            let previous_nonlocal = self.nonlocal_result.take();
            self.nonlocal_result = if library.inline
                && library
                    .parameter_modes
                    .get(*index)
                    .is_none_or(|mode| *mode == syntax::InlineParameterMode::Ordinary)
                && (self.allow_return || previous_nonlocal.is_some())
            {
                Some(
                    previous_nonlocal
                        .clone()
                        .unwrap_or_else(|| self.result.clone()),
                )
            } else {
                None
            };
            let inferred = self.infer_lambda_result(
                &params[*index].1,
                arg,
                &mut substitutions,
                &fixed,
                &parameter_names,
                None,
            );
            self.nonlocal_result = previous_nonlocal;
            inferred?;
        }
        if let Some((pattern, expected)) = &expected_result {
            let result = method.kotlin_result.as_ref().unwrap();
            let mut result_constraints = HashMap::new();
            if !self.infer_constraints(
                pattern,
                expected,
                &mut result_constraints,
                &HashSet::new(),
                0,
            ) {
                return Err(Diagnostic::new(
                    offset,
                    "Conflicting expected generic result type",
                ));
            }
            let result_needs_widening =
                !self.subtype(&substitute(result, &substitutions), expected);
            for (name, ty) in result_constraints {
                // An expected supertype bounds a bare result variable from above.
                // Its declared bound can be more specific than that expectation.
                let ty = if !substitutions.contains_key(&name)
                    && matches!(pattern.base(), Type::Parameter { name: result_name, .. } if *result_name == name)
                    && let Some(parameter) = library.type_parameters.iter().find(|p| p.name == name)
                {
                    let bounds = parameter
                        .bounds
                        .iter()
                        .map(|bound| substitute(bound, &substitutions))
                        .collect::<Vec<_>>();
                    if let Type::Nullable(inner) = &ty
                        && bounds.iter().any(|bound| !self.subtype(&ty, bound))
                        && bounds.iter().all(|bound| self.subtype(inner, bound))
                    {
                        inner.as_ref().clone()
                    } else {
                        bounds
                            .into_iter()
                            .find(|bound| {
                                matches!(bound.base(), Type::Java(_) | Type::String)
                                    && !self.subtype(&ty, bound)
                                    && self.subtype(bound, &ty)
                            })
                            .unwrap_or(ty)
                    }
                } else {
                    ty
                };
                if result_needs_widening
                    && !fixed.contains(&name)
                    && substitutions
                        .get(&name)
                        .is_some_and(|input| self.subtype(input, &ty))
                {
                    substitutions.insert(name, ty);
                } else {
                    substitutions.entry(name).or_insert(ty);
                }
            }
        }
        for _ in 0..library.type_parameters.len() {
            let previous = substitutions.clone();
            for parameter in &library.type_parameters {
                let Some(inferred) = substitutions.get(&parameter.name).cloned() else {
                    continue;
                };
                for bound in &parameter.bounds {
                    if matches!(bound, Type::Parameter { .. })
                        && !self.infer_constraints(bound, &inferred, &mut substitutions, &fixed, 0)
                    {
                        return Err(Diagnostic::new(
                            offset,
                            "Conflicting interdependent type parameter bounds",
                        ));
                    }
                }
            }
            if substitutions == previous {
                break;
            }
        }
        for parameter in &library.type_parameters {
            let inferred = substitutions.get(&parameter.name).ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    format!("Cannot infer type parameter {}", parameter.name),
                )
                .with_kind(
                    crate::DiagnosticKind::Unclassified,
                    "TYPE_PARAMETER_INFERENCE_INCOMPLETE",
                )
            })?;
            if parameter.reified
                && (inferred.base() == &Type::Unit
                    || matches!(inferred.base(), Type::Nullable(inner) if inner.base() == &Type::Unit))
            {
                return Err(Diagnostic::new(
                    offset,
                    "This reified argument classifier is unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "REIFIED_ARGUMENT_CLASSIFIER_UNSUPPORTED",
                ));
            }
            if parameter.only_input_types && !fixed.contains(&parameter.name) {
                let Some(inputs) = input_types.get(&parameter.name) else {
                    return Err(Diagnostic::new(
                        offset,
                        "OnlyInputTypes requires supported direct input constraints",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "ONLY_INPUT_TYPES_SHAPE_UNSUPPORTED",
                    ));
                };
                if !inputs.contains(inferred) {
                    return Err(
                        Diagnostic::new(offset, "Inferred type is not an input type").with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "TYPE_INFERENCE_ONLY_INPUT_TYPES_ERROR",
                        ),
                    );
                }
            }
            for bound in &parameter.bounds {
                let bound = substitute(bound, &substitutions);
                let bound = if named || matches!(bound, Type::Platform(_)) {
                    bound
                } else {
                    Type::Platform(Box::new(bound))
                };
                if !self.subtype(inferred, &bound) {
                    return Err(Diagnostic::new(
                        offset,
                        format!("Type argument violates upper bound of {}", parameter.name),
                    ));
                }
            }
        }
        let mut specialized = substitute_method(method.clone(), &substitutions);
        if let Some(metadata) = &mut specialized.library {
            metadata.reified_arguments = library
                .type_parameters
                .iter()
                .filter(|parameter| parameter.reified)
                .map(|parameter| {
                    (
                        parameter.name.clone(),
                        substitutions[&parameter.name].clone(),
                    )
                })
                .collect();
            for (parameter, ty) in &metadata.reified_arguments {
                fn nested_parameter(ty: &Type, nested: bool) -> Option<&str> {
                    match ty.base() {
                        Type::Parameter { name, .. } if nested => Some(name),
                        Type::Nullable(inner) => nested_parameter(inner, nested),
                        Type::Array(inner) | Type::OutArray(inner) => nested_parameter(inner, true),
                        _ => None,
                    }
                }
                if let Some(name) = nested_parameter(ty, false) {
                    return Err(Diagnostic::new(
                        offset,
                        "Nested type parameters require unsupported array reification",
                    )
                    .with_kind(
                        if self
                            .reified_parameters
                            .iter()
                            .any(|parameter| parameter.name == name && parameter.reified)
                        {
                            crate::DiagnosticKind::UnsupportedLanguage
                        } else {
                            crate::DiagnosticKind::InvalidKotlin
                        },
                        "NESTED_REIFIED_ARGUMENT_UNSUPPORTED",
                    ));
                }
                let actual = if let Type::Nullable(inner) = ty.base() {
                    inner.base()
                } else {
                    ty.base()
                };
                fn unsupported_classifier(ty: &Type) -> bool {
                    match ty {
                        Type::Nullable(inner) | Type::Array(inner) | Type::OutArray(inner) => {
                            unsupported_classifier(inner)
                        }
                        Type::Platform(_)
                        | Type::Parameterized { .. }
                        | Type::List(_)
                        | Type::MutableList(_)
                        | Type::FlexibleList(_)
                        | Type::Function { .. }
                        | Type::ReceiverFunction { .. }
                        | Type::SuspendReceiverFunction { .. }
                        | Type::Unit
                        | Type::Null => true,
                        _ => false,
                    }
                }
                fn erased_reference_classifier(ty: &Type) -> bool {
                    match ty.base() {
                        Type::Nullable(inner) | Type::Array(inner) | Type::OutArray(inner) => {
                            erased_reference_classifier(inner)
                        }
                        Type::Parameterized { .. }
                        | Type::List(_)
                        | Type::MutableList(_)
                        | Type::FlexibleList(_)
                        | Type::Java(_)
                        | Type::String => true,
                        _ => false,
                    }
                }
                if unsupported_classifier(ty)
                    && !(erased_reference_classifier(ty)
                        && (library.builtin_array
                            || crate::compiled_inline::reifies_only_array_allocation(
                                method, parameter,
                            )))
                {
                    return Err(Diagnostic::new(
                        offset,
                        "This reified argument classifier is unsupported",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "REIFIED_ARGUMENT_CLASSIFIER_UNSUPPORTED",
                    ));
                }
                if let Type::Parameter { name, .. } = actual
                    && !self
                        .reified_parameters
                        .iter()
                        .any(|parameter| parameter.name == *name && parameter.reified)
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Cannot use an erased type parameter as a reified argument",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ERASED_REIFIED_ARGUMENT",
                    ));
                }
            }
        }

        if !captures.is_empty() {
            if method.kotlin_result.as_ref().is_some_and(|result| {
                if !captures.keys().any(|name| mentions(result, name)) {
                    return false;
                }
                match result {
                    Type::Parameter { .. } => false,
                    Type::Parameterized { arguments, .. } => arguments.iter().any(|argument| {
                        argument.ty.as_ref().is_some_and(|ty| {
                            !matches!(ty, Type::Parameter { .. })
                                && captures.keys().any(|name| mentions(ty, name))
                        })
                    }),
                    _ => true,
                }
            }) {
                return Err(Diagnostic::new(offset, "Projected extension receiver result requires unsupported nested capture semantics")
                    .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "PROJECTED_EXTENSION_CAPTURE_UNSUPPORTED"));
            }
            if let (Some(original), Some(params)) =
                (&method.kotlin_params, &mut specialized.kotlin_params)
            {
                params[0].1 = project_supertype(&original[0].1, &substitutions, &captures);
            }
            if let Some(result) = &method.kotlin_result {
                specialized.kotlin_result =
                    Some(project_supertype(result, &substitutions, &captures));
            }
            if let Some(library) = &mut specialized.library
                && let Some(receiver) = &method.library.as_ref().unwrap().receiver
            {
                library.receiver = Some(project_supertype(receiver, &substitutions, &captures));
            }
        }
        Ok(specialized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_checker(cp: &Classpath, action: impl FnOnce(&mut Checker<'_>)) {
        let mut checker = Checker {
            composer: None,
            composable_target: None,
            composable_body: false,
            inline_argument_modes: Vec::new(),
            inline_analysis: false,
            public_inline_body: false,
            constructor_bridge_context: false,
            reified_parameters: Vec::new(),
            exhaustiveness_flow: false,
            extension_receivers: Vec::new(),
            receiver_labels: HashMap::new(),
            lambdas: Rc::new(RefCell::new(Vec::new())),
            captured: HashMap::new(),
            local_defaults: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            changing_cells: HashSet::new(),
            lambda_owner: "Relations".into(),
            inline_result: None,
            nonlocal_result: None,
            source_type_params: &[],
            source_infix: &HashSet::new(),
            source_operator: &HashSet::new(),
            classpath: cp,
            constants: &HashMap::new(),
            constructor: None,
            defaults: &Vec::new(),
            function_names: &[],
            mutable_fields: &HashSet::new(),
            extensions: &HashSet::new(),
            classes: &HashMap::new(),
            imports: &HashMap::new(),
            signatures: &Vec::new(),
            dispatch_receiver: None,
            fields: &HashMap::new(),
            members: &HashMap::new(),
            companion_fields: &HashMap::new(),
            functions: &HashMap::new(),
            object_methods: &HashMap::new(),
            object_fields: &HashMap::new(),
            object_owner: None,
            allow_return: false,
            contract_facts: HashMap::new(),
            contract_registration: None,
            refinements: HashMap::new(),
            initialized: InitializationState::default(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: 0,
            result: Type::Unit,
        };
        action(&mut checker);
    }

    fn box_type(variance: Variance, ty: Option<Type>) -> Type {
        Type::Parameterized {
            name: "example/Box".into(),
            arguments: vec![TypeArgument { variance, ty }],
        }
    }

    #[test]
    fn java_platform_bounds_preserve_parameter_upper_bound_flexibility() {
        let cp = Classpath::new(&[]).unwrap();
        let any = Type::Java("java/lang/Object".into());
        let number = Type::Java("java/lang/Number".into());
        let parameter = |bound| Type::Parameter {
            name: "T".into(),
            upper_bound: Box::new(Type::Nullable(Box::new(bound))),
        };
        let unconstrained = parameter(any.clone());
        let numeric = parameter(number.clone());
        let platform = |ty| Type::Platform(Box::new(ty));
        assert!(subtype_at(&cp, &unconstrained, &platform(any.clone()), 0));
        assert!(subtype_at(&cp, &numeric, &platform(number.clone()), 0));
        assert!(subtype_at(&cp, &numeric, &platform(any.clone()), 0));
        assert!(subtype_at(
            &cp,
            &unconstrained,
            &platform(unconstrained.clone()),
            0
        ));
        assert!(!subtype_at(&cp, &unconstrained, &any, 0));
        assert!(!subtype_at(&cp, &numeric, &number, 0));
        assert!(!subtype_at(&cp, &unconstrained, &platform(number), 0));
    }

    #[test]
    fn generic_variance_nullability_and_function_subtyping_are_checked() {
        with_checker(&Classpath::new(&[]).unwrap(), |checker| {
            let any = Type::Java("java/lang/Object".into());
            assert!(!checker.subtype(&Type::Nullable(Box::new(Type::String)), &any));
            assert!(checker.subtype(&Type::String, &Type::Nullable(Box::new(any.clone()))));
            assert!(checker.subtype(
                &Type::List(Box::new(Type::String)),
                &Type::List(Box::new(any.clone()))
            ));
            assert!(!checker.subtype(
                &Type::MutableList(Box::new(Type::String)),
                &Type::MutableList(Box::new(any.clone()))
            ));
            assert!(!checker.subtype(
                &box_type(Variance::Invariant, Some(Type::String)),
                &box_type(Variance::Invariant, Some(any.clone()))
            ));
            assert!(checker.subtype(
                &box_type(Variance::Invariant, Some(Type::String)),
                &box_type(Variance::Out, Some(any.clone()))
            ));
            assert!(checker.subtype(
                &box_type(Variance::Invariant, Some(any.clone())),
                &box_type(Variance::In, Some(Type::String))
            ));
            assert!(checker.subtype(
                &box_type(Variance::Out, Some(Type::String)),
                &box_type(Variance::Invariant, None)
            ));
            assert!(!checker.subtype(
                &box_type(Variance::Invariant, None),
                &box_type(Variance::Out, Some(Type::String))
            ));
            let broad = Type::Function {
                params: vec![any.clone()],
                result: Box::new(Type::String),
            };
            let narrow = Type::Function {
                params: vec![Type::String],
                result: Box::new(any),
            };
            assert!(checker.subtype(&broad, &narrow));
            assert!(!checker.subtype(&narrow, &broad));
        });
    }

    #[test]
    fn inference_preserves_explicit_constraints_and_nullable_candidates() {
        with_checker(&Classpath::new(&[]).unwrap(), |checker| {
            let parameter = Type::Parameter {
                name: "T".into(),
                upper_bound: Box::new(Type::Nullable(Box::new(Type::Java(
                    "java/lang/Object".into(),
                )))),
            };
            let mut substitutions = HashMap::from([("T".into(), Type::String)]);
            assert!(!checker.infer_constraints(
                &parameter,
                &Type::Int,
                &mut substitutions,
                &HashSet::from(["T".into()]),
                0
            ));
            assert_eq!(substitutions["T"], Type::String);
            substitutions.clear();
            assert!(checker.infer_constraints(
                &Type::List(Box::new(parameter.clone())),
                &Type::List(Box::new(Type::String)),
                &mut substitutions,
                &HashSet::new(),
                0
            ));
            assert!(checker.infer_constraints(
                &parameter,
                &Type::Null,
                &mut substitutions,
                &HashSet::new(),
                0
            ));
            assert_eq!(substitutions["T"], Type::Nullable(Box::new(Type::String)));
            assert_eq!(
                substitute(&Type::Nullable(Box::new(parameter)), &substitutions),
                Type::Nullable(Box::new(Type::String))
            );
        });
    }

    #[test]
    fn source_generic_classifiers_reject_invalid_arity_raw_types_and_bounds() {
        let cp = Classpath::new(&[]).unwrap();
        let map = |arguments| Type::Parameterized {
            name: "java/util/Map".into(),
            arguments,
        };
        let argument = |ty| TypeArgument {
            variance: Variance::Invariant,
            ty: Some(ty),
        };
        assert!(
            validate_type_arguments(&map(vec![argument(Type::String)]), &cp, 7)
                .unwrap_err()
                .message
                .contains("number of type arguments")
        );
        assert!(
            validate_type_arguments(&Type::Java("java/util/ArrayList".into()), &cp, 7)
                .unwrap_err()
                .message
                .contains("explicit type arguments")
        );
        let invalid_enum = Type::Parameterized {
            name: "java/lang/Enum".into(),
            arguments: vec![argument(Type::String)],
        };
        assert!(
            validate_type_arguments(&invalid_enum, &cp, 7)
                .unwrap_err()
                .message
                .contains("upper bound")
        );
        validate_type_arguments(
            &map(vec![argument(Type::String), argument(Type::Int)]),
            &cp,
            7,
        )
        .unwrap();
        validate_type_arguments(
            &map(vec![
                argument(Type::Nullable(Box::new(Type::String))),
                argument(Type::Int),
            ]),
            &cp,
            7,
        )
        .unwrap();
    }

    #[test]
    fn inherited_generic_receiver_arguments_reach_declaring_methods() {
        let directory = std::env::temp_dir().join(format!(
            "korrust-relations-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("Base.java"),"package relations; public class Base<T> { public T get() { return null; } public void put(T value) {} }").unwrap();
        std::fs::write(
            directory.join("Child.java"),
            "package relations; public class Child<T> extends Base<T> {}",
        )
        .unwrap();
        let output = std::process::Command::new("javac")
            .arg("-d")
            .arg(&directory)
            .arg(directory.join("Base.java"))
            .arg(directory.join("Child.java"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let cp = Classpath::new(std::slice::from_ref(&directory)).unwrap();
        let child = cp.load("relations/Child").unwrap();
        let receiver = Type::Parameterized {
            name: "relations/Child".into(),
            arguments: vec![TypeArgument {
                variance: Variance::Invariant,
                ty: Some(Type::String),
            }],
        };
        let parent = Type::Parameterized {
            name: "relations/Base".into(),
            arguments: vec![TypeArgument {
                variance: Variance::Invariant,
                ty: Some(Type::String),
            }],
        };
        with_checker(&cp, |checker| {
            assert!(checker.subtype(&receiver, &parent));
            let projected = Type::Parameterized {
                name: "relations/Child".into(),
                arguments: vec![TypeArgument {
                    variance: Variance::Out,
                    ty: Some(Type::String),
                }],
            };
            assert!(!checker.subtype(&projected, &parent));
            let put = cp
                .methods("relations/Child")
                .unwrap()
                .into_iter()
                .find(|method| method.name == "put")
                .unwrap();
            assert!(projected_input(&put, &projected, &child, &cp));
            assert!(specialize_receiver(put, &projected, &child, &cp).unsupported);
            assert!(!checker.subtype(
                &receiver,
                &Type::Parameterized {
                    name: "relations/Base".into(),
                    arguments: vec![TypeArgument {
                        variance: Variance::Invariant,
                        ty: Some(Type::Int)
                    }]
                }
            ));
        });
        let getter = cp
            .methods("relations/Child")
            .unwrap()
            .into_iter()
            .find(|method| method.name == "get")
            .unwrap();
        let getter = specialize_receiver(getter, &receiver, &child, &cp);
        assert_eq!(
            getter.kotlin_result,
            Some(Type::Platform(Box::new(Type::String)))
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
