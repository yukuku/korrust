use super::*;
use crate::syntax::{TypeArgument, Variance};
use std::collections::{HashMap, HashSet};

pub(super) fn expand(
    cp: &Classpath,
    ty: &Type,
    imports: &HashMap<String, String>,
    packages: &[&str],
) -> Result<Type> {
    expand_at(cp, ty, imports, packages, &mut HashSet::new(), 0)
}

fn alias(
    cp: &Classpath,
    name: &str,
    imports: &HashMap<String, String>,
    packages: &[&str],
) -> Result<Option<(String, LibraryTypeAlias)>> {
    if let Some(path) = imports.get(name) {
        if cp.load(path).is_ok() {
            return Ok(None);
        }
        if let Some((package, name)) = path.rsplit_once('/') {
            return Ok(cp
                .type_alias(package, name)?
                .map(|alias| (path.clone(), alias)));
        }
        return Ok(None);
    }
    if name.contains('/') {
        if cp.load(name).is_ok() {
            return Ok(None);
        }
        let Some((package, short)) = name.rsplit_once('/') else {
            return Ok(None);
        };
        return Ok(cp
            .type_alias(package, short)?
            .map(|alias| (name.into(), alias)));
    }
    let mut result = None;
    for package in packages {
        if let Some(alias) = cp.type_alias(package, name)? {
            if result.is_some() {
                return Err(Diagnostic::new(0, "Ambiguous default type alias"));
            }
            result = Some((format!("{package}/{name}"), alias));
        }
    }
    Ok(result)
}

fn expand_at(
    cp: &Classpath,
    ty: &Type,
    imports: &HashMap<String, String>,
    packages: &[&str],
    active: &mut HashSet<String>,
    depth: usize,
) -> Result<Type> {
    if depth > 64 {
        return Err(
            Diagnostic::new(0, "Type alias expansion exceeds the current depth limit").with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "TYPE_ALIAS_DEPTH_UNSUPPORTED",
            ),
        );
    }
    let (name, arguments) = match ty {
        Type::Java(name) => (Some(name), &[][..]),
        Type::Parameterized { name, arguments } => (Some(name), arguments.as_slice()),
        _ => (None, &[][..]),
    };
    if let Some(name) = name {
        if let Some((key, alias)) = alias(cp, name, imports, packages)? {
            if !active.insert(key.clone()) {
                return Err(
                    Diagnostic::new(0, format!("Cyclic type alias expansion: {key}"))
                        .with_kind(crate::DiagnosticKind::InvalidKotlin, "RECURSIVE_TYPE_ALIAS"),
                );
            }
            if alias.type_parameters.len() != arguments.len() {
                return Err(Diagnostic::new(
                    0,
                    format!(
                        "Type alias {} requires {} type arguments",
                        alias.name,
                        alias.type_parameters.len()
                    ),
                )
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "TYPE_ALIAS_ARITY"));
            }
            let mut substitutions = HashMap::new();
            for (parameter, argument) in alias.type_parameters.iter().zip(arguments) {
                if parameter.bounds.iter().any(|bound|!matches!(bound,Type::Nullable(inner) if matches!(inner.as_ref(),Type::Java(name) if name=="java/lang/Object"))) {
                    return Err(Diagnostic::new(0,"Library type aliases with constrained parameter bounds are unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "TYPE_ALIAS_BOUND_UNSUPPORTED"));
                }
                substitutions.insert(parameter.name.clone(), argument.clone());
            }
            let instantiated = substitute(&alias.expanded, &substitutions)?;
            let expanded = expand_at(cp, &instantiated, imports, packages, active, depth + 1)?;
            active.remove(&key);
            return Ok(expanded);
        }
    }
    let mut recurse = |ty: &Type| expand_at(cp, ty, imports, packages, active, depth + 1);
    Ok(match ty {
        Type::Parameterized { name, arguments } => Type::Parameterized {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|a| {
                    Ok(TypeArgument {
                        variance: a.variance,
                        ty: a.ty.as_ref().map(&mut recurse).transpose()?,
                    })
                })
                .collect::<Result<_>>()?,
        },
        Type::Nullable(t) => {
            let ty = recurse(t)?;
            if ty.is_nullable() {
                ty
            } else {
                Type::Nullable(Box::new(ty))
            }
        }
        Type::Platform(t) => Type::Platform(Box::new(recurse(t)?)),
        Type::Array(t) => Type::Array(Box::new(recurse(t)?)),
        Type::List(t) => Type::List(Box::new(recurse(t)?)),
        Type::MutableList(t) => Type::MutableList(Box::new(recurse(t)?)),
        Type::ThreadLocal(t) => Type::ThreadLocal(Box::new(recurse(t)?)),
        Type::Parameter { name, upper_bound } => Type::Parameter {
            name: name.clone(),
            upper_bound: Box::new(recurse(upper_bound)?),
        },
        Type::Function { params, result } => Type::Function {
            params: params.iter().map(&mut recurse).collect::<Result<_>>()?,
            result: Box::new(recurse(result)?),
        },
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => Type::ReceiverFunction {
            receiver: Box::new(recurse(receiver)?),
            params: params.iter().map(&mut recurse).collect::<Result<_>>()?,
            result: Box::new(recurse(result)?),
        },
        _ => ty.clone(),
    })
}

fn substitute(ty: &Type, arguments: &HashMap<String, TypeArgument>) -> Result<Type> {
    let recurse = |ty: &Type| substitute(ty, arguments);
    Ok(match ty {
        Type::Parameter { name, .. } if arguments.contains_key(name) => {
            let argument = &arguments[name];
            if argument.variance != Variance::Invariant {
                return Err(Diagnostic::new(
                    0,
                    "Projected type alias arguments at nested type positions are unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "TYPE_ALIAS_PROJECTION_UNSUPPORTED",
                ));
            }
            argument.ty.clone().ok_or_else(||Diagnostic::new(0,"Star-projected type alias expansion at a concrete type position is unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "TYPE_ALIAS_STAR_UNSUPPORTED"))?
        }
        Type::Parameterized {
            name,
            arguments: params,
        } => Type::Parameterized {
            name: name.clone(),
            arguments: params
                .iter()
                .map(|a| {
                    if let Some(Type::Parameter { name, .. }) = &a.ty {
                        if let Some(value) = arguments.get(name) {
                            let variance = match (a.variance, value.variance) {
                                (Variance::Invariant, v) | (v, Variance::Invariant) => v,
                                (a, b) if a == b => a,
                                _ => {
                                    return Err(Diagnostic::new(
                                        0,
                                        "Conflicting projections in type alias expansion",
                                    )
                                    .with_kind(
                                        crate::DiagnosticKind::InvalidKotlin,
                                        "TYPE_ALIAS_PROJECTION_CONFLICT",
                                    ));
                                }
                            };
                            return Ok(TypeArgument {
                                variance,
                                ty: value.ty.clone(),
                            });
                        }
                    }
                    Ok(TypeArgument {
                        variance: a.variance,
                        ty: a.ty.as_ref().map(recurse).transpose()?,
                    })
                })
                .collect::<Result<_>>()?,
        },
        Type::Nullable(t) => {
            let ty = recurse(t)?;
            if ty.is_nullable() {
                ty
            } else {
                Type::Nullable(Box::new(ty))
            }
        }
        Type::Platform(t) => Type::Platform(Box::new(recurse(t)?)),
        Type::Array(t) => Type::Array(Box::new(recurse(t)?)),
        Type::List(t) => Type::List(Box::new(recurse(t)?)),
        Type::MutableList(t) => Type::MutableList(Box::new(recurse(t)?)),
        Type::ThreadLocal(t) => Type::ThreadLocal(Box::new(recurse(t)?)),
        Type::Parameter { name, upper_bound } => Type::Parameter {
            name: name.clone(),
            upper_bound: Box::new(recurse(upper_bound)?),
        },
        Type::Function { params, result } => Type::Function {
            params: params.iter().map(recurse).collect::<Result<_>>()?,
            result: Box::new(recurse(result)?),
        },
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => Type::ReceiverFunction {
            receiver: Box::new(recurse(receiver)?),
            params: params.iter().map(recurse).collect::<Result<_>>()?,
            result: Box::new(recurse(result)?),
        },
        _ => ty.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn class(aliases: Vec<LibraryTypeAlias>) -> JavaClass {
        JavaClass {
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            kotlin_fun_interface: false,
            library: Some(LibraryClass {
                type_aliases: aliases,
                type_parameters: Vec::new(),
                supertypes: Vec::new(),
                package: Some("aliases".into()),
                multifile_facade: None,
            }),
            name: "aliases/DefinitionsKt".into(),
            access: 0x11,
            super_name: Some("java/lang/Object".into()),
            interfaces: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            kotlin_class: true,
            unsupported: false,
            kotlin_interface: false,
        }
    }
    #[test]
    fn expands_alias_arguments_and_nullability_and_detects_cycles() {
        let cp = Classpath::new(&[]).unwrap();
        cp.register_source(class(vec![
            LibraryTypeAlias {
                name: "Names".into(),
                type_parameters: vec![TypeParameter {
                    name: "T".into(),
                    variance: Variance::Invariant,
                    bounds: vec![Type::Nullable(Box::new(Type::Java(
                        "java/lang/Object".into(),
                    )))],
                }],
                expanded: Type::Array(Box::new(Type::Nullable(Box::new(Type::Parameter {
                    name: "T".into(),
                    upper_bound: Box::new(Type::Java("java/lang/Object".into())),
                })))),
            },
            LibraryTypeAlias {
                name: "Cycle".into(),
                type_parameters: Vec::new(),
                expanded: Type::Java("aliases/Cycle".into()),
            },
        ]));
        let value = Type::Parameterized {
            name: "Names".into(),
            arguments: vec![TypeArgument {
                variance: Variance::Invariant,
                ty: Some(Type::String),
            }],
        };
        assert_eq!(
            cp.expand_type_alias(&value, &HashMap::new(), &["aliases"])
                .unwrap(),
            Type::Array(Box::new(Type::Nullable(Box::new(Type::String))))
        );
        assert_eq!(
            cp.expand_type_alias(
                &Type::Nullable(Box::new(value.clone())),
                &HashMap::new(),
                &["aliases"]
            )
            .unwrap(),
            Type::Nullable(Box::new(Type::Array(Box::new(Type::Nullable(Box::new(
                Type::String
            ))))))
        );
        assert!(
            cp.expand_type_alias(&Type::Java("Cycle".into()), &HashMap::new(), &["aliases"])
                .err()
                .unwrap()
                .message
                .contains("Cyclic type alias expansion")
        );
    }
}
