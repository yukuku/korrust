use super::*;
use crate::syntax::{TypeArgument, Variance};

pub(super) fn type_owner(ty: &Type) -> Option<&str> {
    match ty.base() {
        Type::Java(name) | Type::Parameterized { name, .. } => Some(name),
        Type::String => Some("java/lang/String"),
        Type::List(_) | Type::MutableList(_) => Some("java/util/List"),
        _ => None,
    }
}

fn relation_owner(ty: &Type) -> Option<&str> {
    type_owner(ty).or_else(|| match ty.base() {
        Type::Int => Some("java/lang/Integer"),
        Type::Long => Some("java/lang/Long"),
        Type::Byte => Some("java/lang/Byte"),
        Type::Char => Some("java/lang/Character"),
        Type::Boolean => Some("java/lang/Boolean"),
        Type::Float => Some("java/lang/Float"),
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
        Type::List(inner) => Type::List(Box::new(substitute(inner, substitutions))),
        Type::MutableList(inner) => Type::MutableList(Box::new(substitute(inner, substitutions))),
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
        Type::Function { params, result } => Type::Function {
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
        Type::List(inner) | Type::MutableList(inner) => Some(vec![TypeArgument {
            variance: Variance::Invariant,
            ty: Some(inner.as_ref().clone()),
        }]),
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
        Type::List(inner) | Type::MutableList(inner)
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

fn as_supertype(cp: &Classpath, ty: &Type, target: &str) -> Option<Type> {
    let mut pending = vec![(ty.clone(), 0)];
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
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner) => mentions(inner, name),
        Type::Parameterized { arguments, .. } => arguments
            .iter()
            .any(|argument| argument.ty.as_ref().is_some_and(|ty| mentions(ty, name))),
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
                && method.kotlin_params.as_ref().is_some_and(|params| {
                    params
                        .iter()
                        .any(|(_, ty, _)| mentions(ty, &parameter.name))
                })
        })
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
    if let Some(result) = &mut method.kotlin_result {
        *result = substitute(result, &readable);
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

fn subtype_at(cp: &Classpath, actual: &Type, expected: &Type, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    if actual == expected {
        return true;
    }
    if actual == &Type::Null {
        return expected.is_nullable() || matches!(expected, Type::Platform(_));
    }
    if let Type::Platform(inner) = actual {
        return subtype_at(cp, inner, expected, depth + 1);
    }
    if let Type::Platform(inner) = expected {
        let actual = if let Type::Nullable(inner) = actual {
            inner.as_ref()
        } else {
            actual
        };
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
    if let Type::Parameter { upper_bound, .. } = actual {
        return subtype_at(cp, upper_bound, expected, depth + 1);
    }
    if matches!(expected, Type::Java(name) if name == "java/lang/Object") {
        return actual != &Type::Unit;
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
            } else if matches!(expected, Type::List(_)) {
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
                    .unwrap_or_else(|| cp.load(name))?;
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
                    .unwrap_or_else(|| cp.load(name))?;
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
                            ));
                        }
                    }
                }
            }
            Type::Nullable(inner)
            | Type::List(inner)
            | Type::MutableList(inner)
            | Type::ThreadLocal(inner) => validate(inner, cp, classes, offset, depth + 1)?,
            Type::Platform(_) => {}
            Type::Parameter { .. } => {}
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
    ) -> Result<()> {
        let Type::Function { params, result } = pattern.base() else {
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
                        if parameter.name == "_" {
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
        let output = substitute(result, substitutions);
        if !parameters.iter().any(|name| mentions(&output, name)) {
            return Ok(());
        }
        let next = self.next;
        let initialized = self.initialized.clone();
        let refinements = self.refinements.clone();
        let cells = self.cells.clone();
        let cell_names = self.cell_names.clone();
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
                ));
            }
            match &last.kind {
                StatementKind::Expression(value) => Ok(self.value(value)?.ty),
                _ => Ok(Type::Unit),
            }
        })();
        self.scopes.truncate(scopes_len);
        self.allow_return = allow_return;
        self.next = next;
        self.initialized = initialized;
        self.refinements = refinements;
        self.cells = cells;
        self.cell_names = cell_names;
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

    pub(super) fn specialize_method(
        &mut self,
        method: &JavaMethod,
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
    ) -> Result<JavaMethod> {
        let named = self.method_named(method);
        self.specialize_method_named(method, args, explicit, offset, named)
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
                source_name: String::new(),
                type_parameters,
                receiver: None,
                inline: false,
                code: None,
            }),
        };
        let method = self.specialize_method_named(&method, args, explicit, offset, true)?;
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

    fn specialize_method_named(
        &mut self,
        method: &JavaMethod,
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
        named: bool,
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
        for (arg, index) in args.iter().zip(&mapping) {
            let arg = if let ExprKind::NamedArgument(_, expr) = &arg.kind {
                expr.as_ref()
            } else {
                arg
            };
            if matches!(
                arg.kind,
                ExprKind::Lambda(_)
                    | ExprKind::LambdaWithParams { .. }
                    | ExprKind::CallableReference { .. }
            ) {
                continue;
            }
            let value = self.value(arg)?;
            if !self.infer_constraints(&params[*index].1, &value.ty, &mut substitutions, &fixed, 0)
            {
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
            let arg = if let ExprKind::NamedArgument(_, expr) = &arg.kind {
                expr.as_ref()
            } else {
                arg
            };
            self.infer_lambda_result(
                &params[*index].1,
                arg,
                &mut substitutions,
                &fixed,
                &parameter_names,
            )?;
        }
        for parameter in &library.type_parameters {
            let inferred = substitutions.get(&parameter.name).ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    format!("Cannot infer type parameter {}", parameter.name),
                )
            })?;
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
        Ok(substitute_method(method.clone(), &substitutions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_checker(cp: &Classpath, action: impl FnOnce(&mut Checker<'_>)) {
        let mut checker = Checker {
            lambdas: Rc::new(RefCell::new(Vec::new())),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: "Relations".into(),
            inline_result: None,
            nonlocal_result: None,
            source_type_params: &[],
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
            receiver: None,
            fields: &HashMap::new(),
            members: &HashMap::new(),
            companion_fields: &HashMap::new(),
            functions: &HashMap::new(),
            object_methods: &HashMap::new(),
            object_fields: &HashMap::new(),
            object_owner: None,
            allow_return: false,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
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
