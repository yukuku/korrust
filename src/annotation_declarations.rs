use crate::check::{DeclarationAnnotation, DeclarationAnnotationValue as Value};
use crate::classpath::{AnnotationDefinition, AnnotationRetention, Classpath};
use crate::syntax::{AnnotationUse, Expr, ExprKind, Module, Type, TypeArgument, Variance};
use crate::{Diagnostic, DiagnosticKind, Result};
use std::collections::{HashMap, HashSet};

fn invalid(offset: usize, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(offset, message)
        .with_kind(DiagnosticKind::InvalidKotlin, "INVALID_ANNOTATION_VALUE")
}
fn unsupported(offset: usize, message: &str, code: &'static str) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(DiagnosticKind::UnsupportedLanguage, code)
}

pub(crate) fn path(expression: &Expr) -> Option<String> {
    match &expression.kind {
        ExprKind::Name(name) => Some(name.clone()),
        ExprKind::Property(receiver, name) => Some(format!("{}/{name}", path(receiver)?)),
        ExprKind::Generic(value, _) => path(value),
        _ => None,
    }
}

pub(crate) fn owner(name: &str, imports: &HashMap<String, String>, cp: &Classpath) -> String {
    if let Some(imported) = imports.get(name) {
        return imported.clone();
    }
    if let Some((first, rest)) = name.split_once('/') {
        if let Some(imported) = imports.get(first) {
            let direct = format!("{imported}/{rest}");
            if cp.load(&direct).is_ok() {
                return direct;
            }
            return format!("{imported}${}", rest.replace('/', "$"));
        }
        return name.into();
    }
    if cp.load(name).is_ok() {
        return name.into();
    }
    for package in [
        "kotlin",
        "kotlin/annotation",
        "kotlin/reflect",
        "kotlin/jvm",
        "java/lang",
    ] {
        let candidate = format!("{package}/{name}");
        if cp.load(&candidate).is_ok() {
            return candidate;
        }
    }
    name.into()
}

fn enum_entry(
    expression: &Expr,
    expected: &str,
    imports: &HashMap<String, String>,
    cp: &Classpath,
) -> Result<String> {
    let name =
        path(expression).ok_or_else(|| invalid(expression.offset, "Expected an enum entry"))?;
    let canonical = imports.get(&name).cloned().unwrap_or(name);
    let (classifier, entry) = canonical.rsplit_once('/').ok_or_else(|| {
        invalid(
            expression.offset,
            "Expected a qualified or imported enum entry",
        )
    })?;
    let classifier = owner(classifier, imports, cp);
    if classifier != expected {
        return Err(invalid(
            expression.offset,
            "Annotation enum argument has a different enum type",
        ));
    }
    let class = cp.load(expected)?;
    if class.access & 0x4000 == 0
        || !class.fields.iter().any(|field| {
            field.name == entry
                && field.descriptor == format!("L{expected};")
                && field.access & 0x4009 == 0x4009
        })
    {
        return Err(invalid(
            expression.offset,
            "Unresolved annotation enum entry",
        ));
    }
    Ok(entry.into())
}

pub(crate) fn logical_type(
    ty: &Type,
    imports: &HashMap<String, String>,
    cp: &Classpath,
    offset: usize,
) -> Result<Type> {
    let expanded = cp.expand_type_alias(ty, imports, &["kotlin", "java/lang"])?;
    Ok(match expanded {
        Type::Java(name) => Type::Java(owner(&name, imports, cp)),
        Type::Parameterized { name, arguments } => Type::Parameterized {
            name: owner(&name, imports, cp),
            arguments: arguments
                .into_iter()
                .map(|a| {
                    Ok(TypeArgument {
                        variance: a.variance,
                        ty: a
                            .ty
                            .map(|t| logical_type(&t, imports, cp, offset))
                            .transpose()?,
                    })
                })
                .collect::<Result<_>>()?,
        },
        Type::Array(inner) => Type::Array(Box::new(logical_type(&inner, imports, cp, offset)?)),
        Type::Nullable(_) | Type::Platform(_) => {
            return Err(invalid(
                offset,
                "Annotation elements cannot be nullable or platform types",
            ));
        }
        other => other,
    })
}

pub(crate) fn element_descriptor(ty: &Type, cp: &Classpath, offset: usize) -> Result<String> {
    match ty {
        Type::Int
        | Type::Long
        | Type::Float
        | Type::Double
        | Type::Boolean
        | Type::Char
        | Type::String => Ok(ty.descriptor()),
        Type::Byte | Type::Short | Type::ByteArray => Err(unsupported(
            offset,
            "Byte and Short annotation elements are unsupported",
            "ANNOTATION_BYTE_SHORT_UNSUPPORTED",
        )),
        Type::IntArray
        | Type::LongArray
        | Type::FloatArray
        | Type::DoubleArray
        | Type::BooleanArray
        | Type::CharArray
        | Type::StringArray => Ok(ty.descriptor()),
        Type::Array(inner) => {
            if !matches!(
                inner.as_ref(),
                Type::String | Type::Java(_) | Type::Parameterized { .. }
            ) {
                return Err(invalid(
                    offset,
                    "Annotation reference arrays require String, enum, class or annotation elements",
                ));
            }
            Ok(format!("[{}", element_descriptor(inner, cp, offset)?))
        }
        Type::Parameterized { name, arguments } if name == "kotlin/reflect/KClass" => {
            if arguments.len() != 1 || arguments[0].ty.is_some() {
                return Err(unsupported(
                    offset,
                    "Annotation KClass bounds beyond star projection are unsupported",
                    "ANNOTATION_KCLASS_BOUND_UNSUPPORTED",
                ));
            }
            Ok("Ljava/lang/Class;".into())
        }
        Type::Java(name)
            if matches!(
                name.as_str(),
                "Short" | "kotlin/Short" | "ShortArray" | "kotlin/ShortArray"
            ) =>
        {
            Err(unsupported(
                offset,
                "Short annotation elements are unsupported",
                "ANNOTATION_BYTE_SHORT_UNSUPPORTED",
            ))
        }
        Type::Java(name)
            if matches!(
                name.as_str(),
                "UInt"
                    | "ULong"
                    | "UByte"
                    | "UShort"
                    | "kotlin/UInt"
                    | "kotlin/ULong"
                    | "kotlin/UByte"
                    | "kotlin/UShort"
            ) =>
        {
            Err(unsupported(
                offset,
                "Unsigned annotation elements are unsupported",
                "ANNOTATION_UNSIGNED_UNSUPPORTED",
            ))
        }
        Type::Java(name) => {
            let class = cp.load(name)?;
            if class.access & (0x4000 | 0x2000) == 0 {
                return Err(invalid(
                    offset,
                    "Annotation elements require an enum or annotation type",
                ));
            }
            Ok(ty.descriptor())
        }
        _ => Err(invalid(offset, "Invalid annotation element type")),
    }
}

pub(crate) fn definition(
    module: &Module,
    name: &str,
    cp: &Classpath,
) -> Result<AnnotationDefinition> {
    let source = module
        .annotation_decl
        .as_ref()
        .ok_or_else(|| invalid(0, "Missing source annotation declaration"))?;
    let mut imports = imports(module, cp)?;
    imports.insert(source.name.clone(), name.into());
    let mut retention = AnnotationRetention::Runtime;
    let mut targets = None;
    let mut seen = HashSet::new();
    for annotation in &source.annotations {
        let canonical = owner(&annotation.name, &imports, cp);
        if !seen.insert(canonical.clone()) {
            return Err(invalid(
                annotation.offset,
                "Repeated annotation meta-annotation",
            ));
        }
        if canonical == "kotlin/annotation/Repeatable" || canonical == "kotlin/jvm/JvmRepeatable" {
            return Err(unsupported(
                annotation.offset,
                "Repeatable annotation declarations are unsupported",
                "REPEATABLE_ANNOTATION_UNSUPPORTED",
            ));
        }
        if canonical == "kotlin/annotation/Retention" {
            let [(_, expression)] = annotation.arguments.as_slice() else {
                return Err(invalid(
                    annotation.offset,
                    "Retention requires exactly one value",
                ));
            };
            retention = match enum_entry(
                expression,
                "kotlin/annotation/AnnotationRetention",
                &imports,
                cp,
            )?
            .as_str()
            {
                "SOURCE" => AnnotationRetention::Source,
                "BINARY" => AnnotationRetention::Class,
                "RUNTIME" => AnnotationRetention::Runtime,
                _ => return Err(invalid(expression.offset, "Unknown annotation retention")),
            };
        } else if canonical == "kotlin/annotation/Target" {
            let mut values = Vec::new();
            for (_, expression) in &annotation.arguments {
                let expressions = match &expression.kind {
                    ExprKind::AnnotationArray(values) => values.as_slice(),
                    _ => std::slice::from_ref(expression),
                };
                for expression in expressions {
                    values.push(enum_entry(
                        expression,
                        "kotlin/annotation/AnnotationTarget",
                        &imports,
                        cp,
                    )?);
                }
            }
            targets = Some(values);
        }
    }
    let kotlin_targets = targets.unwrap_or_else(|| {
        [
            "CLASS",
            "ANNOTATION_CLASS",
            "PROPERTY",
            "FIELD",
            "VALUE_PARAMETER",
            "CONSTRUCTOR",
            "FUNCTION",
            "PROPERTY_GETTER",
            "PROPERTY_SETTER",
            "LOCAL_VARIABLE",
            "TYPEALIAS",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    });
    let mut jvm_targets = Vec::new();
    for target in &kotlin_targets {
        let jvm = match target.as_str() {
            "CLASS" => "TYPE",
            "ANNOTATION_CLASS" => "ANNOTATION_TYPE",
            "FIELD" => "FIELD",
            "VALUE_PARAMETER" => "PARAMETER",
            "CONSTRUCTOR" => "CONSTRUCTOR",
            "FUNCTION" | "PROPERTY_GETTER" | "PROPERTY_SETTER" => "METHOD",
            "LOCAL_VARIABLE" => "LOCAL_VARIABLE",
            "TYPE" => "TYPE_USE",
            "TYPE_PARAMETER" => "TYPE_PARAMETER",
            _ => continue,
        };
        if !jvm_targets.iter().any(|value| value == jvm) {
            jvm_targets.push(jvm.into());
        }
    }
    let mut parameters = Vec::new();
    let mut vararg = None;
    for parameter in &source.parameters {
        let ty = logical_type(&parameter.ty, &imports, cp, parameter.offset)?;
        let mut descriptor = element_descriptor(&ty, cp, parameter.offset)?;
        if parameter.vararg {
            descriptor.insert(0, '[');
            vararg = Some(parameter.name.clone());
        }
        if matches!(
            (parameter.name.as_str(), descriptor.as_str()),
            ("hashCode", "I") | ("toString", "Ljava/lang/String;")
        ) {
            return Err(invalid(
                parameter.offset,
                "Annotation parameter conflicts with an inherited JVM method",
            ));
        }
        if matches!(
            parameter.name.as_str(),
            "hashCode" | "toString" | "annotationType"
        ) {
            return Err(unsupported(
                parameter.offset,
                "Annotation member conflicts with inherited JVM annotation proxy methods",
                "ANNOTATION_INHERITED_MEMBER_UNSUPPORTED",
            ));
        }
        parameters.push((
            parameter.name.clone(),
            descriptor,
            parameter.default.is_some(),
        ));
    }
    Ok(AnnotationDefinition {
        retention,
        targets: jvm_targets,
        parameters,
        kotlin_types: Vec::new(),
        kotlin_parameters: Some(source.parameters.iter().map(|p| p.name.clone()).collect()),
        kotlin_targets: Some(kotlin_targets),
        vararg,
    })
}

pub(crate) fn value(
    expression: &Expr,
    descriptor: &str,
    imports: &HashMap<String, String>,
    cp: &Classpath,
    depth: usize,
    scope: Option<&str>,
) -> Result<Value> {
    if depth > 32 {
        return Err(unsupported(
            expression.offset,
            "Annotation nesting exceeds the current limit",
            "ANNOTATION_DEPTH_UNSUPPORTED",
        ));
    }
    if let Some(component) = descriptor.strip_prefix('[') {
        let expressions = if let ExprKind::AnnotationArray(values) = &expression.kind {
            values.as_slice()
        } else if let ExprKind::Call(target, arguments) = &expression.kind {
            let name = path(target).ok_or_else(|| {
                invalid(
                    expression.offset,
                    "Expected an annotation array constructor",
                )
            })?;
            let canonical = imports.get(&name).cloned().unwrap_or_else(|| {
                if name.contains('/') {
                    name.clone()
                } else {
                    format!("kotlin/{name}")
                }
            });
            let (package, name) = canonical.rsplit_once('/').ok_or_else(|| {
                invalid(expression.offset, "Unresolved annotation array constructor")
            })?;
            let methods = cp.package_methods(package, name)?;
            let methods = methods
                .iter()
                .filter(|m| m.library.as_ref().is_some_and(|l| l.builtin_array))
                .collect::<Vec<_>>();
            if methods.is_empty() {
                return Err(invalid(
                    expression.offset,
                    "Annotation arrays require a resolved builtin array constructor",
                ));
            }
            if !methods.iter().any(|m| {
                m.descriptor.rsplit_once(')').is_some_and(|(_, result)| {
                    result == descriptor
                        || result == "[Ljava/lang/Object;" && component.starts_with('L')
                })
            }) {
                return Err(invalid(
                    expression.offset,
                    "Annotation array factory has a different array representation",
                ));
            }
            if let ExprKind::Generic(_, types) = &target.kind {
                if types.len() != 1
                    || !methods.iter().any(|m| {
                        m.library
                            .as_ref()
                            .is_some_and(|l| l.type_parameters.len() == 1)
                    })
                {
                    return Err(invalid(
                        expression.offset,
                        "Annotation array constructor has invalid type arguments",
                    ));
                }
                let explicit = logical_type(&types[0], imports, cp, expression.offset)?;
                if element_descriptor(&explicit, cp, expression.offset)? != component {
                    return Err(invalid(
                        expression.offset,
                        "Annotation array type argument mismatch",
                    ));
                }
            }
            arguments.as_slice()
        } else {
            return Err(invalid(
                expression.offset,
                "Expected an annotation array value",
            ));
        };
        let mut values = Vec::new();
        for expression in expressions {
            if let ExprKind::Unary(operator, inner) = &expression.kind
                && operator == "*"
            {
                let Value::Array(spread) = value(inner, descriptor, imports, cp, depth + 1, scope)?
                else {
                    return Err(invalid(
                        expression.offset,
                        "Annotation spread requires an array",
                    ));
                };
                values.extend(spread);
            } else {
                values.push(value(expression, component, imports, cp, depth + 1, scope)?);
            }
        }
        return Ok(Value::Array(values));
    }
    if descriptor == "Ljava/lang/Class;" {
        return class_literal_value(expression, imports, cp, scope);
    }
    if let Some(class_name) = descriptor
        .strip_prefix('L')
        .and_then(|name| name.strip_suffix(';'))
        && class_name != "java/lang/String"
    {
        let class = cp.load(class_name)?;
        if class.access & 0x4000 != 0 {
            return Ok(Value::Enum {
                descriptor: descriptor.into(),
                entry: enum_entry(expression, class_name, imports, cp)?,
            });
        }
        if class.access & 0x2000 != 0 {
            let ExprKind::Call(target, arguments) = &expression.kind else {
                return Err(invalid(
                    expression.offset,
                    "Expected a nested annotation constructor",
                ));
            };
            let target_name = path(target).ok_or_else(|| {
                invalid(expression.offset, "Expected a nested annotation classifier")
            })?;
            if owner(&target_name, imports, cp) != class_name {
                return Err(invalid(
                    expression.offset,
                    "Nested annotation has a different type",
                ));
            }
            let arguments = arguments
                .iter()
                .map(|argument| {
                    if let ExprKind::NamedArgument(name, expression) = &argument.kind {
                        (Some(name.clone()), expression.as_ref().clone())
                    } else {
                        (None, argument.clone())
                    }
                })
                .collect::<Vec<_>>();
            return Ok(Value::Nested(Box::new(arguments_value(
                class_name,
                &arguments,
                expression.offset,
                imports,
                cp,
                depth + 1,
                scope,
            )?)));
        }
        return Err(invalid(
            expression.offset,
            "Annotation value has a non-annotation reference type",
        ));
    }
    let folded = crate::const_eval::fold_expression(expression, &mut |_, offset| {
        Err(invalid(offset, "Unresolved annotation constant reference"))
    })?;
    use crate::syntax::ConstValue;
    match (folded, descriptor) {
        (ConstValue::Int(value), "I") => Ok(Value::Int(value)),
        (ConstValue::Int(value), "J")
            if crate::const_eval::integer_literal_expression(expression) =>
        {
            Ok(Value::Long(i64::from(value)))
        }
        (ConstValue::Long(value), "J") => Ok(Value::Long(value)),
        (ConstValue::Float(value), "F") => Ok(Value::Float(value)),
        (ConstValue::Double(value), "D") => Ok(Value::Double(value)),
        (ConstValue::Char(value), "C") => Ok(Value::Char(value)),
        (ConstValue::Boolean(value), "Z") => Ok(Value::Boolean(value)),
        (ConstValue::String(value), "Ljava/lang/String;") => Ok(Value::String(value)),
        (_, "B" | "S") => Err(unsupported(
            expression.offset,
            "Byte and Short annotation values are unsupported",
            "ANNOTATION_BYTE_SHORT_UNSUPPORTED",
        )),
        _ => Err(invalid(
            expression.offset,
            "Annotation argument type mismatch",
        )),
    }
}

fn class_literal_value(
    expression: &Expr,
    imports: &HashMap<String, String>,
    cp: &Classpath,
    scope: Option<&str>,
) -> Result<Value> {
    let ExprKind::ClassLiteral {
        receiver,
        classifier: Some(source),
    } = &expression.kind
    else {
        return Err(invalid(
            expression.offset,
            "Annotation class arguments require a pure classifier literal",
        ));
    };
    let source_path = path(receiver).ok_or_else(|| {
        invalid(
            expression.offset,
            "Annotation class arguments cannot use bound expressions",
        )
    })?;
    let head = source_path.split('/').next().unwrap();
    if let Some(scope) = scope {
        let class = cp.load(scope)?;
        if class.fields.iter().any(|field| field.name == head)
            || class
                .methods
                .iter()
                .any(|method| method.source_property.as_deref() == Some(head))
        {
            return Err(invalid(
                expression.offset,
                "Annotation class literal receiver is a lexical value",
            ));
        }
    }
    if let Some(binding) = imports.get(head) {
        if binding.contains("::") {
            return Err(invalid(
                expression.offset,
                "Annotation class arguments cannot use value bindings",
            ));
        }
        if let Some((package, name)) = binding.rsplit_once('/') {
            if !cp.package_constants(package, name)?.is_empty()
                || cp
                    .package_methods(package, name)?
                    .iter()
                    .any(|m| m.source_property.as_deref() == Some(name))
            {
                return Err(invalid(
                    expression.offset,
                    "Annotation class arguments cannot use value bindings",
                ));
            }
        }
    }
    fn normalize(
        source: &Type,
        imports: &HashMap<String, String>,
        cp: &Classpath,
        depth: usize,
        offset: usize,
    ) -> Result<Type> {
        if depth > 64 {
            return Err(unsupported(
                offset,
                "Annotation class nesting exceeds the current limit",
                "ANNOTATION_CLASS_DEPTH_UNSUPPORTED",
            ));
        }
        let primitive = match source {
            Type::Java(name) => match name.as_str() {
                "kotlin/Int" => Some(Type::Int),
                "kotlin/Long" => Some(Type::Long),
                "kotlin/Byte" => Some(Type::Byte),
                "kotlin/Short" => Some(Type::Short),
                "kotlin/Char" => Some(Type::Char),
                "kotlin/Boolean" => Some(Type::Boolean),
                "kotlin/Float" => Some(Type::Float),
                "kotlin/Double" => Some(Type::Double),
                "kotlin/String" => Some(Type::String),
                "kotlin/Unit" => Some(Type::Unit),
                "Any" | "kotlin/Any" => Some(Type::Java("java/lang/Object".into())),
                "Short" => Some(Type::Short),
                "ShortArray" | "kotlin/ShortArray" => {
                    return Err(unsupported(
                        offset,
                        "Short class literals require unsupported primitive distinctions",
                        "ANNOTATION_BYTE_SHORT_UNSUPPORTED",
                    ));
                }
                _ => Type::primitive_array_named(name),
            },
            _ => None,
        };
        if let Some(ty) = primitive {
            return Ok(ty);
        }
        match source {
            Type::Array(inner) => Ok(Type::Array(Box::new(normalize(
                inner,
                imports,
                cp,
                depth + 1,
                offset,
            )?))),
            Type::Nullable(inner) if depth != 0 => Ok(Type::Nullable(Box::new(normalize(
                inner,
                imports,
                cp,
                depth + 1,
                offset,
            )?))),
            Type::Nullable(_) => Err(invalid(
                offset,
                "Annotation class literal classifier cannot be nullable",
            )),
            Type::Java(name) => {
                let canonical = owner(name, imports, cp);
                let expanded = cp.expand_type_alias(
                    &Type::Java(canonical.clone()),
                    imports,
                    &["kotlin", "java/lang"],
                )?;
                if expanded != Type::Java(canonical.clone()) {
                    return normalize(&expanded, imports, cp, depth + 1, offset);
                }
                let class = cp.load(&canonical)?;
                if !class.kotlin_class && class.name.contains('$') {
                    return Err(unsupported(
                        offset,
                        "Java nested annotation class literals require complete enclosing visibility",
                        "ANNOTATION_JAVA_NESTED_CLASS_UNSUPPORTED",
                    ));
                }
                if class.access & 1 == 0
                    || class
                        .library
                        .as_ref()
                        .and_then(|l| l.kotlin_visibility)
                        .is_some_and(|v| {
                            v == 0 && !cp.internal_visible(&canonical) || !matches!(v, 0 | 3)
                        })
                {
                    return Err(invalid(
                        offset,
                        "Annotation class literal classifier is inaccessible",
                    ));
                }
                Ok(Type::Java(canonical))
            }
            Type::Parameterized { name, arguments }
                if name == "Array" || name == "kotlin/Array" =>
            {
                let [
                    TypeArgument {
                        variance: Variance::Invariant,
                        ty: Some(inner),
                    },
                ] = arguments.as_slice()
                else {
                    return Err(unsupported(
                        offset,
                        "Projected annotation array class literals are unsupported",
                        "ANNOTATION_ARRAY_CLASS_PROJECTION_UNSUPPORTED",
                    ));
                };
                Ok(Type::Array(Box::new(normalize(
                    inner,
                    imports,
                    cp,
                    depth + 1,
                    offset,
                )?)))
            }
            Type::Parameterized { .. } => Err(invalid(
                offset,
                "Parameterized non-array class literals are invalid",
            )),
            Type::Parameter { .. } => Err(unsupported(
                offset,
                "Type parameter annotation class literals require reification",
                "ANNOTATION_REIFIED_CLASS_UNSUPPORTED",
            )),
            other => Ok(other.clone()),
        }
    }
    let source = if let Some(binding) = imports
        .get(&source_path)
        .filter(|binding| cp.load(binding).is_ok())
    {
        Type::Java(binding.clone())
    } else {
        source.clone()
    };
    let logical = normalize(&source, imports, cp, 0, expression.offset)?;
    let descriptor = if logical == Type::Unit {
        "Lkotlin/Unit;".into()
    } else {
        logical.descriptor()
    };
    Ok(Value::Class {
        logical,
        descriptor,
    })
}

pub(crate) fn arguments_value(
    name: &str,
    arguments: &[(Option<String>, Expr)],
    offset: usize,
    imports: &HashMap<String, String>,
    cp: &Classpath,
    depth: usize,
    scope: Option<&str>,
) -> Result<DeclarationAnnotation> {
    let class = cp.load(name)?;
    let definition = class.annotation_definition.ok_or_else(|| {
        if class.access & 0x2000 != 0 {
            unsupported(
                offset,
                "Annotation declaration has unsupported parameter or metadata contracts",
                "ANNOTATION_DEFINITION_UNSUPPORTED",
            )
        } else {
            invalid(offset, "Expected an annotation declaration")
        }
    })?;
    let mut supplied = HashSet::new();
    let mut values = Vec::new();
    let order = definition.kotlin_parameters.as_ref();
    let mut variadic = Vec::new();
    let mut positional_vararg = false;
    if definition.vararg.as_ref().is_some_and(|name| {
        definition
            .parameters
            .last()
            .is_none_or(|(last, _, _)| last != name)
    }) {
        return Err(unsupported(
            offset,
            "Nonfinal annotation varargs are unsupported",
            "ANNOTATION_VARARG_POSITION_UNSUPPORTED",
        ));
    }
    for (position, (key, expression)) in arguments.iter().enumerate() {
        let key = key.clone().unwrap_or_else(|| {
            order
                .and_then(|names| names.get(position))
                .cloned()
                .or_else(|| definition.vararg.clone().filter(|_| order.is_some()))
                .unwrap_or_else(|| "value".into())
        });
        if order.is_none() && position != 0 && arguments[position].0.is_none() {
            return Err(invalid(
                expression.offset,
                "Java annotation arguments after value must be named",
            ));
        }
        let (_, descriptor, _) = definition
            .parameters
            .iter()
            .find(|(name, _, _)| name == &key)
            .ok_or_else(|| invalid(expression.offset, "Unknown annotation argument"))?;
        if definition.vararg.as_deref() == Some(&key) && arguments[position].0.is_none() {
            positional_vararg = true;
            if let ExprKind::Unary(operator, expression) = &expression.kind
                && operator == "*"
            {
                let Value::Array(items) =
                    value(expression, descriptor, imports, cp, depth + 1, scope)?
                else {
                    return Err(invalid(offset, "Expected spread annotation array"));
                };
                variadic.extend(items);
            } else {
                variadic.push(value(
                    expression,
                    &descriptor[1..],
                    imports,
                    cp,
                    depth + 1,
                    scope,
                )?);
            }
            continue;
        }
        if !supplied.insert(key.clone()) {
            return Err(invalid(expression.offset, "Duplicate annotation argument"));
        }
        let checked = if order.is_none() && key == "value" && descriptor.starts_with('[') {
            if let ExprKind::Unary(operator, expression) = &expression.kind
                && operator == "*"
            {
                value(expression, descriptor, imports, cp, depth + 1, scope)?
            } else if matches!(
                expression.kind,
                ExprKind::AnnotationArray(_) | ExprKind::Call(_, _)
            ) {
                value(expression, descriptor, imports, cp, depth + 1, scope)?
            } else {
                Value::Array(vec![value(
                    expression,
                    &descriptor[1..],
                    imports,
                    cp,
                    depth + 1,
                    scope,
                )?])
            }
        } else {
            value(expression, descriptor, imports, cp, depth + 1, scope)?
        };
        values.push((key, checked));
    }
    if let Some(name) = &definition.vararg {
        if positional_vararg && supplied.contains(name) {
            return Err(invalid(offset, "Duplicate annotation vararg argument"));
        }
        if !supplied.contains(name) {
            supplied.insert(name.clone());
            values.push((name.clone(), Value::Array(variadic)));
        }
    }
    if definition
        .parameters
        .iter()
        .any(|(name, _, default)| !default && !supplied.contains(name))
    {
        return Err(invalid(offset, "Missing required annotation argument"));
    }
    Ok(DeclarationAnnotation {
        descriptor: format!("L{name};"),
        runtime: definition.retention == AnnotationRetention::Runtime,
        values,
    })
}

pub(crate) fn checked_use(
    annotation: &AnnotationUse,
    cp: &Classpath,
    imports: &HashMap<String, String>,
    scope: Option<&str>,
) -> Result<DeclarationAnnotation> {
    arguments_value(
        &owner(&annotation.name, imports, cp),
        &annotation.arguments,
        annotation.offset,
        imports,
        cp,
        0,
        scope,
    )
}

pub(crate) struct CheckedParameter {
    pub name: String,
    pub ty: Type,
    pub descriptor: String,
    pub default: Option<Value>,
    pub vararg: bool,
}
pub(crate) struct CheckedAnnotation {
    pub parameters: Vec<CheckedParameter>,
    pub annotations: Vec<DeclarationAnnotation>,
    pub has_annotations: bool,
}

pub(crate) fn check(module: &Module, name: &str, cp: &Classpath) -> Result<CheckedAnnotation> {
    let source = module
        .annotation_decl
        .as_ref()
        .ok_or_else(|| invalid(0, "Missing annotation declaration"))?;
    let definition = definition(module, name, cp)?;
    let imports = imports(module, cp)?;
    validate_cycles(name, cp, &mut Vec::new(), false, source.offset)?;
    let mut parameters = Vec::new();
    for (parameter, (_, descriptor, _)) in source.parameters.iter().zip(&definition.parameters) {
        let element = logical_type(&parameter.ty, &imports, cp, parameter.offset)?;
        let ty = if parameter.vararg {
            Type::primitive_array_for(&element).unwrap_or_else(|| {
                if element == Type::String {
                    Type::StringArray
                } else {
                    Type::Array(Box::new(element.clone()))
                }
            })
        } else {
            element
        };
        let default = parameter
            .default
            .as_ref()
            .map(|expression| value(expression, descriptor, &imports, cp, 0, Some(name)))
            .transpose()?;
        parameters.push(CheckedParameter {
            name: parameter.name.clone(),
            ty,
            descriptor: descriptor.clone(),
            default,
            vararg: parameter.vararg,
        });
    }
    let mut annotations = Vec::new();
    for annotation in &source.annotations {
        let canonical = owner(&annotation.name, &imports, cp);
        let checked = if canonical == "kotlin/annotation/Retention" {
            let entry = match definition.retention {
                AnnotationRetention::Source => "SOURCE",
                AnnotationRetention::Class => "BINARY",
                AnnotationRetention::Runtime => "RUNTIME",
            };
            DeclarationAnnotation {
                descriptor: format!("L{canonical};"),
                runtime: true,
                values: vec![(
                    "value".into(),
                    Value::Enum {
                        descriptor: "Lkotlin/annotation/AnnotationRetention;".into(),
                        entry: entry.into(),
                    },
                )],
            }
        } else if canonical == "kotlin/annotation/Target" {
            DeclarationAnnotation {
                descriptor: format!("L{canonical};"),
                runtime: true,
                values: vec![(
                    "allowedTargets".into(),
                    Value::Array(
                        definition
                            .kotlin_targets
                            .as_ref()
                            .unwrap()
                            .iter()
                            .map(|entry| Value::Enum {
                                descriptor: "Lkotlin/annotation/AnnotationTarget;".into(),
                                entry: entry.clone(),
                            })
                            .collect(),
                    ),
                )],
            }
        } else {
            if let Some(code) = crate::check::annotation_unsupported_lowering(&canonical) {
                return Err(unsupported(
                    annotation.offset,
                    "Annotation requires unsupported compiler semantics",
                    code,
                ));
            }
            let class = cp.load(&canonical)?;
            let policy = class
                .annotation_definition
                .as_ref()
                .ok_or_else(|| invalid(annotation.offset, "Unsupported annotation definition"))?;
            if policy.kotlin_targets.as_ref().map_or_else(
                || {
                    !policy
                        .targets
                        .iter()
                        .any(|t| t == "ANNOTATION_TYPE" || t == "TYPE")
                },
                |targets| !targets.iter().any(|t| t == "ANNOTATION_CLASS"),
            ) {
                return Err(invalid(
                    annotation.offset,
                    "Annotation cannot target an annotation declaration",
                ));
            }
            let checked = checked_use(annotation, cp, &imports, Some(name))?;
            if policy.retention == AnnotationRetention::Source {
                continue;
            }
            checked
        };
        annotations.push(checked);
    }
    let has_annotations = !annotations.is_empty();
    let retention = match definition.retention {
        AnnotationRetention::Source => "SOURCE",
        AnnotationRetention::Class => "CLASS",
        AnnotationRetention::Runtime => "RUNTIME",
    };
    annotations.push(DeclarationAnnotation {
        descriptor: "Ljava/lang/annotation/Retention;".into(),
        runtime: true,
        values: vec![(
            "value".into(),
            Value::Enum {
                descriptor: "Ljava/lang/annotation/RetentionPolicy;".into(),
                entry: retention.into(),
            },
        )],
    });
    if source
        .annotations
        .iter()
        .any(|a| owner(&a.name, &imports, cp) == "kotlin/annotation/Target")
    {
        annotations.push(DeclarationAnnotation {
            descriptor: "Ljava/lang/annotation/Target;".into(),
            runtime: true,
            values: vec![(
                "value".into(),
                Value::Array(
                    definition
                        .targets
                        .iter()
                        .map(|entry| Value::Enum {
                            descriptor: "Ljava/lang/annotation/ElementType;".into(),
                            entry: entry.clone(),
                        })
                        .collect(),
                ),
            )],
        });
    }
    Ok(CheckedAnnotation {
        parameters,
        annotations,
        has_annotations,
    })
}

pub(crate) fn header(
    module: &Module,
    name: &str,
    cp: &Classpath,
) -> Result<crate::classpath::JavaClass> {
    use crate::classpath::{JavaClass, JavaMethod, LibraryClass, LibraryMethod};
    let definition = definition(module, name, cp)?;
    let imports = imports(module, cp)?;
    let methods = module
        .annotation_decl
        .as_ref()
        .unwrap()
        .parameters
        .iter()
        .zip(&definition.parameters)
        .map(|(parameter, (_, descriptor, _))| {
            let ty = logical_type(&parameter.ty, &imports, cp, parameter.offset)?;
            let logical = if parameter.vararg {
                Type::primitive_array_for(&ty).unwrap_or_else(|| Type::Array(Box::new(ty)))
            } else {
                ty
            };
            Ok(JavaMethod {
                owner: name.into(),
                name: parameter.name.clone(),
                descriptor: format!("(){descriptor}"),
                access: 0x401,
                unsupported: false,
                source_property: Some(parameter.name.clone()),
                kotlin_params: Some(Vec::new()),
                kotlin_result: Some(logical),
                kotlin_internal: false,
                library: Some(LibraryMethod {
                    parameter_modes: Vec::new(),
                    reified_arguments: Vec::new(),
                    has_annotations: false,
                    abstract_: true,
                    builtin_array: false,
                    kotlin_public: true,
                    source_name: parameter.name.clone(),
                    type_parameters: Vec::new(),
                    receiver: None,
                    infix: false,
                    operator: false,
                    inline: false,
                    code: None,
                }),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(JavaClass {
        sealed_: false,
        sealed_subclasses: Vec::new(),
        annotation_definition: Some(definition),
        kotlin_fun_interface: false,
        library: Some(LibraryClass {
            kotlin_visibility: Some(3),
            type_aliases: Vec::new(),
            type_parameters: Vec::new(),
            supertypes: vec![Type::Java("java/lang/annotation/Annotation".into())],
            package: None,
            multifile_facade: None,
        }),
        name: name.into(),
        access: 0x2601,
        super_name: Some("java/lang/Object".into()),
        interfaces: vec!["java/lang/annotation/Annotation".into()],
        methods,
        fields: Vec::new(),
        kotlin_class: true,
        unsupported: false,
        kotlin_interface: true,
    })
}

pub(crate) fn imports(module: &Module, cp: &Classpath) -> Result<HashMap<String, String>> {
    let mut bindings = module
        .imports
        .iter()
        .filter(|(name, _, _)| name != "*")
        .map(|(name, owner, _)| (name.clone(), owner.clone()))
        .collect::<HashMap<_, _>>();
    let mut wildcard = HashMap::<String, String>::new();
    for (_, path, offset) in module.imports.iter().filter(|(name, _, _)| name == "*") {
        let package = path
            .strip_suffix("/*")
            .ok_or_else(|| invalid(*offset, "Malformed annotation wildcard import"))?;
        for class in cp.package_classes(package)? {
            if class.access & 1 == 0 {
                continue;
            }
            let name = class.name.rsplit('/').next().unwrap().to_owned();
            if bindings.contains_key(&name) {
                continue;
            }
            if wildcard
                .get(&name)
                .is_some_and(|previous| previous != &class.name)
            {
                return Err(invalid(*offset, "Ambiguous wildcard annotation classifier"));
            }
            wildcard.insert(name, class.name);
        }
    }
    bindings.extend(wildcard);
    for class in cp.package_classes(&module.package.replace('.', "/"))? {
        let name = class.name.rsplit('/').next().unwrap().to_owned();
        bindings.entry(name).or_insert(class.name);
    }
    if let Some(annotation) = &module.annotation_decl {
        let name = if module.package.is_empty() {
            annotation.name.clone()
        } else {
            format!("{}/{}", module.package.replace('.', "/"), annotation.name)
        };
        bindings.insert(annotation.name.clone(), name);
    }
    Ok(bindings)
}

fn validate_cycles(
    name: &str,
    cp: &Classpath,
    active: &mut Vec<(String, bool)>,
    crossed_array: bool,
    offset: usize,
) -> Result<()> {
    if let Some(index) = active.iter().position(|(owner, _)| owner == name) {
        if crossed_array || active[index..].iter().any(|(_, array)| *array) {
            return Err(unsupported(
                offset,
                "Cyclic annotation parameter arrays remain an explicit specification/oracle boundary",
                "ANNOTATION_ARRAY_CYCLE_UNSUPPORTED",
            ));
        }
        return Err(invalid(
            offset,
            "Cyclic annotation parameter types are invalid",
        ));
    }
    if active.len() > 32 {
        return Err(unsupported(
            offset,
            "Annotation declaration nesting exceeds the current limit",
            "ANNOTATION_DEPTH_UNSUPPORTED",
        ));
    }
    let class = cp.load(name)?;
    let Some(definition) = class.annotation_definition else {
        return Ok(());
    };
    active.push((name.into(), crossed_array));
    for (_, descriptor, _) in &definition.parameters {
        let component = descriptor.trim_start_matches('[');
        if let Some(next) = component
            .strip_prefix('L')
            .and_then(|s| s.strip_suffix(';'))
            && cp.load(next)?.access & 0x2000 != 0
        {
            validate_cycles(next, cp, active, descriptor.starts_with('['), offset)?;
        }
    }
    active.pop();
    Ok(())
}
