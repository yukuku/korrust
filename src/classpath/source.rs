#[path = "source_enums.rs"]
mod enums;
use super::*;
use crate::syntax::{ConstValue, Function, Module, TypeArgument};

fn resolve(ty: &Type, module: &Module, cp: &Classpath) -> Type {
    fn name(name: &str, module: &Module, cp: &Classpath) -> String {
        if let Some((_, path, _)) = module.imports.iter().find(|(alias, _, _)| alias == name) {
            return path.clone();
        }
        if let Some((root, nested)) = name.split_once('/') {
            if let Some(owner) = module
                .imports
                .iter()
                .find(|(alias, _, _)| alias == root)
                .map(|(_, owner, _)| owner)
                .filter(|owner| !owner.contains("::"))
            {
                return format!("{owner}${}", nested.replace('/', "$"));
            }
            return name.into();
        }
        match name {
            "List" | "MutableList" => "java/util/List".into(),
            "Set" | "MutableSet" => "java/util/Set".into(),
            "Map" | "MutableMap" => "java/util/Map".into(),
            "Collection" | "MutableCollection" => "java/util/Collection".into(),
            "Iterable" | "MutableIterable" => "java/lang/Iterable".into(),
            "ThreadLocal" => "java/lang/ThreadLocal".into(),
            "Any" => "java/lang/Object".into(),
            "Throwable" => "java/lang/Throwable".into(),
            _ if cp
                .load(&format!("java/lang/{name}"))
                .is_ok_and(|class| class.access & 1 != 0) =>
            {
                format!("java/lang/{name}")
            }
            _ if module.package.is_empty() => name.into(),
            _ => format!("{}/{}", module.package.replace('.', "/"), name),
        }
    }
    match ty {
        Type::Java(n) => Type::Java(name(n, module, cp)),
        Type::Parameterized { name: n, arguments } => Type::Parameterized {
            name: name(n, module, cp),
            arguments: arguments
                .iter()
                .map(|a| TypeArgument {
                    variance: a.variance,
                    ty: a.ty.as_ref().map(|t| resolve(t, module, cp)),
                })
                .collect(),
        },
        Type::Nullable(t) => Type::Nullable(Box::new(resolve(t, module, cp))),
        Type::Platform(t) => Type::Platform(Box::new(resolve(t, module, cp))),
        Type::List(t) => Type::List(Box::new(resolve(t, module, cp))),
        Type::MutableList(t) => Type::MutableList(Box::new(resolve(t, module, cp))),
        Type::Array(t) => Type::Array(Box::new(resolve(t, module, cp))),
        Type::ThreadLocal(t) => Type::Parameterized {
            name: "java/lang/ThreadLocal".into(),
            arguments: vec![TypeArgument {
                variance: crate::syntax::Variance::Invariant,
                ty: Some(resolve(t, module, cp)),
            }],
        },
        Type::Parameter { name, upper_bound } => Type::Parameter {
            name: name.clone(),
            upper_bound: Box::new(resolve(upper_bound, module, cp)),
        },
        Type::Function { params, result } => Type::Function {
            params: params.iter().map(|t| resolve(t, module, cp)).collect(),
            result: Box::new(resolve(result, module, cp)),
        },
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => Type::ReceiverFunction {
            receiver: Box::new(resolve(receiver, module, cp)),
            params: params.iter().map(|t| resolve(t, module, cp)).collect(),
            result: Box::new(resolve(result, module, cp)),
        },
        _ => ty.clone(),
    }
}

fn method(
    function: &Function,
    module: &Module,
    owner: &str,
    access: u16,
    cp: &Classpath,
) -> JavaMethod {
    let params = function
        .params
        .iter()
        .enumerate()
        .map(|(i, (name, ty))| {
            (
                name.clone(),
                resolve(ty, module, cp),
                function.defaults.get(i).is_some_and(Option::is_some),
            )
        })
        .collect::<Vec<_>>();
    let result = resolve(&function.result, module, cp);
    let receiver = function.receiver.as_ref().map(|t| resolve(t, module, cp));
    let descriptor = format!(
        "({}{}){}",
        receiver.as_ref().map(Type::descriptor).unwrap_or_default(),
        params
            .iter()
            .map(|(_, t, _)| t.descriptor())
            .collect::<String>(),
        result.descriptor()
    );
    JavaMethod {
        library: Some(LibraryMethod {
            has_annotations: !function.annotations.is_empty()
                || function.param_annotations.iter().any(|a| !a.is_empty()),
            parameter_modes: function.parameter_modes.clone(),
            reified_arguments: Vec::new(),
            abstract_: access & 0x400 != 0,
            builtin_array: false,
            kotlin_public: !function.private && !function.protected && !function.internal,
            source_name: function.name.clone(),
            type_parameters: function
                .type_params
                .iter()
                .map(|parameter| crate::syntax::TypeParameter {
                    only_input_types: parameter.only_input_types,
                    reified: parameter.reified,
                    name: parameter.name.clone(),
                    variance: parameter.variance,
                    bounds: parameter
                        .bounds
                        .iter()
                        .map(|bound| resolve(bound, module, cp))
                        .collect(),
                })
                .collect(),
            receiver,
            infix: function.infix,
            operator: function.operator,
            inline: function.inline,
            code: None,
        }),
        owner: owner.into(),
        name: function.name.clone(),
        descriptor,
        access: (if function.private {
            (access & !1) | 2
        } else if function.protected {
            (access & !1) | 4
        } else {
            access
        }) | if function.vararg { 0x80 } else { 0 },
        unsupported: false,
        source_property: None,
        kotlin_params: Some(params),
        kotlin_result: Some(result),
        kotlin_internal: function.internal,
    }
}

fn accessor(owner: &str, name: &str, ty: Type, mutable: bool, private: bool) -> Vec<JavaMethod> {
    let make = |setter: bool| JavaMethod {
        library: None,
        owner: owner.into(),
        name: if setter {
            crate::check::setter_name(name)
        } else {
            crate::check::getter_name(name)
        },
        descriptor: if setter {
            format!("({})V", ty.descriptor())
        } else {
            format!("(){}", ty.descriptor())
        },
        access: if private { 2 } else { 1 },
        unsupported: false,
        source_property: Some(name.into()),
        kotlin_params: Some(if setter {
            vec![("value".into(), ty.clone(), false)]
        } else {
            Vec::new()
        }),
        kotlin_result: Some(if setter { Type::Unit } else { ty.clone() }),
        kotlin_internal: false,
    };
    let mut result = vec![make(false)];
    if mutable {
        result.push(make(true));
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn property_accessors(
    cp: &Classpath,
    owner: &str,
    storage_owner: &str,
    name: &str,
    ty: Type,
    mutable: bool,
    getter: crate::syntax::PropertyVisibility,
    setter: crate::syntax::PropertyVisibility,
    default_getter: bool,
    default_setter: bool,
    static_field: bool,
) -> Vec<JavaMethod> {
    use crate::syntax::PropertyVisibility;
    let mut methods = accessor(
        owner,
        name,
        ty.clone(),
        mutable,
        getter == PropertyVisibility::Private,
    );
    for (index, method) in methods.iter_mut().enumerate() {
        let visibility = if index == 0 { getter } else { setter };
        method.access = visibility.access();
        method.kotlin_internal = visibility == PropertyVisibility::Internal;
        if visibility == PropertyVisibility::Private
            && if index == 0 {
                default_getter
            } else {
                default_setter
            }
        {
            cp.source_default_accessors.borrow_mut().insert(
                (owner.into(), method.name.clone()),
                SourceBackingField {
                    owner: storage_owner.into(),
                    name: name.into(),
                    ty: ty.clone(),
                    is_static: static_field,
                    companion_bridge: owner != storage_owner,
                },
            );
        }
    }
    methods
}

fn empty(owner: &str) -> JavaClass {
    JavaClass {
        sealed_: false,
        sealed_subclasses: Vec::new(),
        annotation_definition: None,
        kotlin_fun_interface: false,
        library: None,
        name: owner.into(),
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

pub(super) fn header(module: &Module, owner: &str, cp: &Classpath) -> Result<JavaClass> {
    if module.annotation_decl.is_some() {
        return crate::annotation_declarations::header(module, owner, cp);
    }
    if module.enum_decl.is_some() {
        return enums::header(module, owner, cp);
    }
    let mut context = module.clone();
    let name = context
        .class_decl
        .as_ref()
        .map(|d| d.name.clone())
        .or_else(|| context.object_decl.as_ref().map(|d| d.name.clone()))
        .or_else(|| context.interface_decl.as_ref().map(|d| d.name.clone()))
        .or_else(|| context.enum_decl.as_ref().map(|d| d.name.clone()));
    if let Some(name) = name {
        context.imports.insert(0, (name, owner.into(), 0));
    }
    let module = &context;
    let mut class = empty(owner);
    class.sealed_ = module.class_decl.as_ref().is_some_and(|d| d.sealed_)
        || module.interface_decl.as_ref().is_some_and(|d| d.sealed_);
    let imports = module
        .imports
        .iter()
        .map(|(a, b, _)| (a.clone(), b.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let source_supertypes = module
        .class_decl
        .as_ref()
        .map(|d| d.super_types.as_slice())
        .or_else(|| {
            module
                .interface_decl
                .as_ref()
                .map(|d| d.super_types.as_slice())
        })
        .unwrap_or(&[]);
    let resolved_supertypes = source_supertypes
        .iter()
        .map(|ty| {
            let expanded = cp.expand_type_alias(ty, &imports, DEFAULT_KOTLIN_PACKAGES)?;
            let name = match ty {
                Type::Java(n) | Type::Parameterized { name: n, .. } => n.clone(),
                _ => return Err(invalid()),
            };
            Ok((name, resolve(&expanded, module, cp)))
        })
        .collect::<Result<Vec<_>>>()?;
    let super_owner = |name: &str| -> String {
        resolved_supertypes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| {
                t.descriptor()
                    .trim_start_matches('L')
                    .trim_end_matches(';')
                    .into()
            })
            .unwrap_or_else(|| {
                resolve(&Type::Java(name.into()), module, cp)
                    .descriptor()
                    .trim_start_matches('L')
                    .trim_end_matches(';')
                    .into()
            })
    };

    let generic = module
        .class_decl
        .as_ref()
        .map(|d| (&d.type_params, &d.super_types))
        .or_else(|| {
            module
                .interface_decl
                .as_ref()
                .map(|d| (&d.type_params, &d.super_types))
        });
    if let Some((parameters, _)) = generic {
        class.library = Some(LibraryClass {
            kotlin_visibility: None,
            type_aliases: Vec::new(),
            type_parameters: parameters
                .iter()
                .map(|p| crate::syntax::TypeParameter {
                    only_input_types: p.only_input_types,
                    reified: p.reified,
                    name: p.name.clone(),
                    variance: p.variance,
                    bounds: p.bounds.iter().map(|t| resolve(t, module, cp)).collect(),
                })
                .collect(),
            supertypes: resolved_supertypes.iter().map(|(_, t)| t.clone()).collect(),
            package: None,
            multifile_facade: None,
        });
    }

    if let Some(source) = &module.class_decl {
        if source.sealed_
            && (!source.secondary_constructors.is_empty()
                || source.defaults.iter().any(Option::is_some))
        {
            return Err(Diagnostic::new(
                source.offset,
                "Sealed secondary constructors and default constructor parameters are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "SEALED_CONSTRUCTOR_DEFAULTS_UNSUPPORTED",
            ));
        }
        class.access = if source.abstract_ {
            0x401
        } else if source.open {
            1
        } else {
            0x11
        };
        class.super_name = Some(
            source
                .super_class
                .as_ref()
                .map(|(n, _)| super_owner(n))
                .unwrap_or("java/lang/Object".into()),
        );
        class.interfaces = source.interfaces.iter().map(|n| super_owner(n)).collect();
        let params = source
            .params
            .iter()
            .enumerate()
            .map(|(i, (n, t))| {
                (
                    n.clone(),
                    resolve(t, module, cp),
                    source.defaults.get(i).is_some_and(Option::is_some),
                )
            })
            .collect::<Vec<_>>();
        if source.primary_constructor {
            class.methods.push(JavaMethod {
                library: None,
                owner: owner.into(),
                name: "<init>".into(),
                descriptor: format!(
                    "({})V",
                    params
                        .iter()
                        .map(|(_, t, _)| t.descriptor())
                        .collect::<String>()
                ),
                access: if source.sealed_ { 4 } else { 1 },
                unsupported: false,
                source_property: None,
                kotlin_params: Some(params),
                kotlin_result: Some(Type::Unit),
                kotlin_internal: source.internal,
            });
        }
        for ctor in &source.secondary_constructors {
            let params = ctor
                .params
                .iter()
                .enumerate()
                .map(|(i, (n, t))| {
                    (
                        n.clone(),
                        resolve(t, module, cp),
                        ctor.defaults.get(i).is_some_and(Option::is_some),
                    )
                })
                .collect::<Vec<_>>();
            class.methods.push(JavaMethod {
                library: None,
                owner: owner.into(),
                name: "<init>".into(),
                descriptor: format!(
                    "({})V",
                    params
                        .iter()
                        .map(|(_, t, _)| t.descriptor())
                        .collect::<String>()
                ),
                access: 1,
                unsupported: false,
                source_property: None,
                kotlin_params: Some(params),
                kotlin_result: Some(Type::Unit),
                kotlin_internal: false,
            });
        }
        for (name, ty) in &source.params {
            if source.param_properties.contains(name) {
                let mut methods = accessor(
                    owner,
                    name,
                    resolve(ty, module, cp),
                    source.mutable_params.contains(name),
                    !source.public_params.contains(name),
                );
                if !source.override_params.contains(name) {
                    for method in &mut methods {
                        method.access |= 0x10;
                    }
                }
                class.methods.extend(methods);
            }
        }
        for property in &source.properties {
            let ty = (!property.inferred).then(|| resolve(&property.ty, module, cp));
            if let Some(ty) = ty {
                let mut methods = property_accessors(
                    cp,
                    owner,
                    owner,
                    &property.name,
                    ty,
                    property.mutable,
                    property.getter_visibility(),
                    property.setter_visibility(),
                    property.body.is_empty() && property.delegate.is_none() && !property.lateinit,
                    property.setter.is_none() && property.delegate.is_none() && !property.lateinit,
                    false,
                );
                if !property.open {
                    for method in &mut methods {
                        method.access |= 0x10;
                    }
                }
                class.methods.extend(methods);
            }
        }
        class.methods.extend(
            source
                .methods
                .iter()
                .filter(|m| !m.function.result_inferred)
                .map(|m| {
                    method(
                        &m.function,
                        module,
                        owner,
                        if !m.has_body {
                            0x401
                        } else if m.open {
                            1
                        } else {
                            0x11
                        },
                        cp,
                    )
                }),
        );
        if source.data {
            for (index, (_, ty)) in source.params.iter().enumerate() {
                let ty = resolve(ty, module, cp);
                let name = format!("component{}", index + 1);
                class.methods.push(JavaMethod {
                    library: Some(LibraryMethod {
                        has_annotations: false,
                        parameter_modes: Vec::new(),
                        reified_arguments: Vec::new(),
                        abstract_: false,
                        builtin_array: false,
                        kotlin_public: true,
                        source_name: name.clone(),
                        type_parameters: Vec::new(),
                        receiver: None,
                        infix: false,
                        operator: true,
                        inline: false,
                        code: None,
                    }),
                    owner: owner.into(),
                    name,
                    descriptor: format!("(){}", ty.descriptor()),
                    access: 0x11,
                    unsupported: false,
                    source_property: None,
                    kotlin_params: Some(Vec::new()),
                    kotlin_result: Some(ty),
                    kotlin_internal: false,
                });
            }
            let params = source
                .params
                .iter()
                .map(|(n, t)| (n.clone(), resolve(t, module, cp), true))
                .collect::<Vec<_>>();
            class.methods.push(JavaMethod {
                library: None,
                owner: owner.into(),
                name: "copy".into(),
                descriptor: format!(
                    "({})L{owner};",
                    params
                        .iter()
                        .map(|(_, t, _)| t.descriptor())
                        .collect::<String>()
                ),
                access: 0x11,
                unsupported: false,
                source_property: None,
                kotlin_params: Some(params),
                kotlin_result: Some(Type::Java(owner.into())),
                kotlin_internal: false,
            });
        }
        if let Some(companion) = &source.companion {
            let companion_owner = format!("{owner}$Companion");
            let mut nested = empty(&companion_owner);
            nested.methods.extend(
                companion
                    .methods
                    .iter()
                    .filter(|m| !m.function.result_inferred)
                    .map(|m| method(&m.function, module, &companion_owner, 0x11, cp)),
            );
            for p in &companion.properties {
                if let Some(ty) = p.ty.as_ref().map(|t| resolve(t, module, cp)) {
                    nested.methods.extend(property_accessors(
                        cp,
                        &companion_owner,
                        owner,
                        &p.name,
                        ty,
                        p.mutable,
                        p.getter_visibility(),
                        p.setter_visibility(),
                        p.body.is_empty() && p.delegate.is_none() && !p.lateinit,
                        p.setter.is_none() && p.delegate.is_none() && !p.lateinit,
                        true,
                    ));
                }
            }
            cp.register_source(nested);
        }
        if source.companion.is_some() {
            class.fields.push(JavaField {
                kotlin: None,
                constant: None,
                owner: owner.into(),
                name: "Companion".into(),
                descriptor: format!("L{owner}$Companion;"),
                access: 0x19,
                unsupported: false,
            });
        }
    } else if let Some(source) = &module.interface_decl {
        class.kotlin_fun_interface = source.fun_interface;
        class.access = 0x601;
        class.kotlin_interface = true;
        class.interfaces = source.interfaces.iter().map(|n| super_owner(n)).collect();
        for p in &source.properties {
            let mut methods =
                accessor(owner, &p.name, resolve(&p.ty, module, cp), p.mutable, false);
            if p.body.is_none() {
                for method in &mut methods {
                    method.access |= 0x400;
                }
            }
            if !p.annotations.is_empty() {
                for method in &mut methods {
                    method.library = Some(LibraryMethod {
                        parameter_modes: Vec::new(),
                        reified_arguments: Vec::new(),
                        has_annotations: true,
                        abstract_: method.access & 0x400 != 0,
                        builtin_array: false,
                        kotlin_public: true,
                        source_name: method.name.clone(),
                        type_parameters: Vec::new(),
                        receiver: None,
                        infix: false,
                        operator: false,
                        inline: false,
                        code: None,
                    });
                }
            }
            class.methods.extend(methods);
        }
        class.methods.extend(
            source
                .methods
                .iter()
                .filter(|m| !m.function.result_inferred)
                .map(|m| {
                    method(
                        &m.function,
                        module,
                        owner,
                        if m.has_body { 1 } else { 0x401 },
                        cp,
                    )
                }),
        );
        if let Some(companion) = &source.companion {
            let companion_owner = format!("{owner}$Companion");
            let mut nested = empty(&companion_owner);
            nested.methods.extend(
                companion
                    .methods
                    .iter()
                    .filter(|m| !m.function.result_inferred)
                    .map(|m| method(&m.function, module, &companion_owner, 0x11, cp)),
            );
            for property in &companion.fields {
                if let Some(ty) = &property.ty {
                    nested.methods.extend(property_accessors(
                        cp,
                        &companion_owner,
                        &companion_owner,
                        &property.name,
                        resolve(ty, module, cp),
                        property.mutable,
                        property.getter_visibility(),
                        property.setter_visibility(),
                        property.body.is_empty()
                            && property.delegate.is_none()
                            && !property.lateinit,
                        property.setter.is_none()
                            && property.delegate.is_none()
                            && !property.lateinit,
                        true,
                    ));
                }
            }
            for property in &companion.properties {
                let ty = match &property.value {
                    crate::syntax::CompanionValue::StringArray(_) => Type::StringArray,
                    crate::syntax::CompanionValue::Object(class) => {
                        resolve(&Type::Java(class.interfaces[0].clone()), module, cp)
                    }
                };
                nested.methods.extend(accessor(
                    &companion_owner,
                    &property.name,
                    ty,
                    false,
                    property.private,
                ));
            }
            cp.register_source(nested);
            class.fields.push(JavaField {
                kotlin: None,
                constant: None,
                owner: owner.into(),
                name: "Companion".into(),
                descriptor: format!("L{companion_owner};"),
                access: 0x19,
                unsupported: false,
            });
        }
    } else if let Some(source) = &module.object_decl {
        class.interfaces = source
            .interfaces
            .iter()
            .map(
                |name| match resolve(&Type::Java(name.clone()), module, cp) {
                    Type::Java(name) => name,
                    _ => unreachable!(),
                },
            )
            .collect();
        class.fields.push(JavaField {
            kotlin: None,
            constant: None,
            owner: owner.into(),
            name: "INSTANCE".into(),
            descriptor: format!("L{owner};"),
            access: 0x19,
            unsupported: false,
        });
        class.methods.extend(
            source
                .methods
                .iter()
                .filter(|m| !m.function.result_inferred)
                .map(|m| {
                    method(
                        &m.function,
                        module,
                        owner,
                        if m.jvm_static { 0x19 } else { 0x11 },
                        cp,
                    )
                }),
        );
        for p in &source.fields {
            if let Some(ty) = p.ty.as_ref().map(|t| resolve(t, module, cp)) {
                class.methods.extend(property_accessors(
                    cp,
                    owner,
                    owner,
                    &p.name,
                    ty,
                    p.mutable,
                    p.getter_visibility(),
                    p.setter_visibility(),
                    p.body.is_empty() && p.delegate.is_none() && !p.lateinit,
                    p.setter.is_none() && p.delegate.is_none() && !p.lateinit,
                    true,
                ));
            }
        }
        constants(&mut class, &source.constants)?;
    } else {
        class.library = Some(LibraryClass {
            kotlin_visibility: None,
            type_aliases: module
                .type_aliases
                .iter()
                .map(|alias| LibraryTypeAlias {
                    name: if alias.private {
                        format!("{}#{}", owner.rsplit('/').next().unwrap(), alias.name)
                    } else {
                        alias.name.clone()
                    },
                    type_parameters: alias
                        .type_params
                        .iter()
                        .map(|parameter| crate::syntax::TypeParameter {
                            only_input_types: parameter.only_input_types,
                            reified: parameter.reified,
                            name: parameter.name.clone(),
                            variance: parameter.variance,
                            bounds: parameter
                                .bounds
                                .iter()
                                .map(|bound| resolve(bound, module, cp))
                                .collect(),
                        })
                        .collect(),
                    expanded: resolve(&alias.expanded, module, cp),
                })
                .collect(),
            type_parameters: Vec::new(),
            supertypes: Vec::new(),
            package: Some(module.package.clone()),
            multifile_facade: None,
        });
        class.methods.extend(
            module
                .functions
                .iter()
                .filter(|f| !f.result_inferred)
                .map(|f| method(f, module, owner, 9, cp)),
        );
        for property in &module.properties {
            if !property.inferred {
                let mut methods = property_accessors(
                    cp,
                    owner,
                    owner,
                    &property.name,
                    resolve(&property.ty, module, cp),
                    property.mutable,
                    property.getter_visibility(),
                    property.setter_visibility(),
                    property.body.is_empty() && property.delegate.is_none() && !property.lateinit,
                    property.setter.is_none() && property.delegate.is_none() && !property.lateinit,
                    true,
                );
                for method in &mut methods {
                    method.access |= 8;
                }
                class.methods.extend(methods);
            }
        }
        constants(&mut class, &module.constants)?;
    }
    let visibility = module
        .class_decl
        .as_ref()
        .map(|d| d.internal)
        .or_else(|| module.interface_decl.as_ref().map(|d| d.internal))
        .or_else(|| module.object_decl.as_ref().map(|d| d.internal));
    if let (Some(internal), Some(library)) = (visibility, &mut class.library) {
        library.kotlin_visibility = Some(if internal { 0 } else { 3 });
    }
    if let Some(source) = &module.class_decl {
        for delegate in &source.interface_delegates {
            let ty = resolve(&delegate.ty, module, cp);
            for mut method in crate::check::source_delegated_members(cp, &ty)? {
                let (params, result) = crate::check::class_method_types(&method)?;
                if class.methods.iter().any(|own| {
                    own.name == method.name
                        && crate::check::class_method_types(own).is_ok_and(|(a, _)| {
                            a.iter()
                                .map(Type::descriptor)
                                .eq(params.iter().map(Type::descriptor))
                        })
                }) {
                    continue;
                }
                method.owner = owner.into();
                method.access = 1;
                method.descriptor = format!(
                    "({}){}",
                    params.iter().map(Type::descriptor).collect::<String>(),
                    result.descriptor()
                );
                if let Some(library) = &mut method.library {
                    library.abstract_ = false;
                    library.inline = false;
                    library.code = None;
                } else {
                    method.library = Some(LibraryMethod {
                        parameter_modes: Vec::new(),
                        reified_arguments: Vec::new(),
                        has_annotations: false,
                        abstract_: false,
                        builtin_array: false,
                        kotlin_public: true,
                        source_name: method.name.clone(),
                        type_parameters: Vec::new(),
                        receiver: None,
                        infix: false,
                        operator: false,
                        inline: false,
                        code: None,
                    });
                }
                class.methods.push(method);
            }
        }
    }
    if module.class_decl.is_some() {
        let supertypes = class
            .library
            .as_ref()
            .map(|l| l.supertypes.clone())
            .unwrap_or_default();
        for method in &mut class.methods {
            if !method.name.starts_with('<') && method.access & 8 == 0 {
                if let Some(result) = crate::check::source_jvm_result(cp, &supertypes, method)? {
                    let prefix = method.descriptor.split(')').next().unwrap();
                    method.descriptor = format!("{prefix}){}", result.descriptor());
                }
            }
        }
    }
    if let Some(library) = &mut class.library {
        if library.supertypes.is_empty() {
            library.supertypes = class
                .super_name
                .iter()
                .chain(&class.interfaces)
                .map(|name| Type::Java(name.clone()))
                .collect();
        }
    }
    let declared = module
        .class_decl
        .as_ref()
        .map(|value| (&value.name, value.internal))
        .or_else(|| {
            module
                .object_decl
                .as_ref()
                .map(|value| (&value.name, value.internal))
        })
        .or_else(|| {
            module
                .interface_decl
                .as_ref()
                .map(|value| (&value.name, value.internal))
        });
    if let Some((name, internal)) = declared {
        let named = !name.contains('$') && owner.rsplit(['/', '$']).next() == Some(name.as_str());
        let supertypes = class
            .super_name
            .iter()
            .chain(&class.interfaces)
            .cloned()
            .map(Type::Java)
            .collect();
        let library = class.library.get_or_insert_with(|| LibraryClass {
            kotlin_visibility: None,
            type_aliases: Vec::new(),
            type_parameters: Vec::new(),
            supertypes,
            package: None,
            multifile_facade: None,
        });
        library.kotlin_visibility = named.then_some(if internal { 0 } else { 3 });
    }
    Ok(class)
}

fn constants(class: &mut JavaClass, constants: &[crate::syntax::ConstDecl]) -> Result<()> {
    for c in constants {
        let (ty, constant) = match &c.value {
            ConstValue::Int(n) => (Type::Int, JavaConstant::Int(*n)),
            ConstValue::Long(n) => (Type::Long, JavaConstant::Long(*n)),
            ConstValue::Float(n) => (Type::Float, JavaConstant::Float(*n)),
            ConstValue::Double(n) => (Type::Double, JavaConstant::Double(*n)),
            ConstValue::Boolean(n) => (Type::Boolean, JavaConstant::Int(i32::from(*n))),
            ConstValue::Char(n) => (Type::Char, JavaConstant::Int(i32::from(*n))),
            ConstValue::String(s) => (Type::String, JavaConstant::String(s.clone())),
            ConstValue::Expression { .. } => {
                return Err(Diagnostic::new(
                    c.offset,
                    "Constant expressions must be evaluated before source headers are built",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "UNEVALUATED_CONSTANT",
                ));
            }
        };
        class.fields.push(JavaField {
            kotlin: Some(KotlinField {
                name: c.name.clone(),
                ty: ty.clone(),
                visibility: if c.private {
                    1
                } else if c.internal {
                    0
                } else {
                    3
                },
                constant: true,
            }),
            constant: Some(constant),
            owner: class.name.clone(),
            name: c.name.clone(),
            descriptor: ty.descriptor(),
            access: if c.private { 0x1a } else { 0x19 },
            unsupported: false,
        });
    }
    Ok(())
}

pub(super) fn defaults(module: &mut Module, cp: &Classpath) {
    if let Some(mut enumeration) = module.enum_decl.take() {
        let mut context = module.clone();
        context.class_decl = Some(enumeration.body.clone());
        defaults(&mut context, cp);
        module.imports = context.imports;
        enumeration.body = context.class_decl.unwrap();
        module.enum_decl = Some(enumeration);
        return;
    }
    fn ty(value: &Type, names: &mut std::collections::BTreeSet<String>) {
        match value {
            Type::Java(name) => {
                names.insert(name.clone());
            }
            Type::Parameterized { name, arguments } => {
                names.insert(name.clone());
                for argument in arguments {
                    if let Some(value) = &argument.ty {
                        ty(value, names);
                    }
                }
            }
            Type::Array(value)
            | Type::Nullable(value)
            | Type::Platform(value)
            | Type::List(value)
            | Type::MutableList(value)
            | Type::ThreadLocal(value) => ty(value, names),
            Type::Parameter { upper_bound, .. } => ty(upper_bound, names),
            Type::Function { params, result } => {
                for value in params {
                    ty(value, names);
                }
                ty(result, names);
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => {
                ty(receiver, names);
                for value in params {
                    ty(value, names);
                }
                ty(result, names);
            }
            _ => {}
        }
    }
    fn annotations(
        values: &[crate::syntax::AnnotationUse],
        names: &mut std::collections::BTreeSet<String>,
    ) {
        names.extend(values.iter().map(|value| value.name.clone()));
    }
    fn function(value: &Function, names: &mut std::collections::BTreeSet<String>) {
        annotations(&value.annotations, names);
        for parameter in &value.param_annotations {
            annotations(parameter, names);
        }

        if let Some(receiver) = &value.receiver {
            ty(receiver, names);
        }
        for (_, value) in &value.params {
            ty(value, names);
        }
        ty(&value.result, names);
        for parameter in &value.type_params {
            for bound in &parameter.bounds {
                ty(bound, names);
            }
        }
    }
    fn property(
        value: &crate::syntax::ClassCompanionProperty,
        names: &mut std::collections::BTreeSet<String>,
    ) {
        annotations(&value.annotations, names);
        if let Some(value) = &value.ty {
            ty(value, names);
        }
    }
    fn declaration(
        value: &crate::syntax::Declaration,
        names: &mut std::collections::BTreeSet<String>,
    ) {
        match value {
            crate::syntax::Declaration::Class(value) => {
                for parameter in &value.type_params {
                    for bound in &parameter.bounds {
                        ty(bound, names);
                    }
                }
                for supertype in &value.super_types {
                    ty(supertype, names);
                }
                annotations(&value.annotations, names);
                for parameter in &value.param_annotations {
                    annotations(parameter, names);
                }
                if let Some((name, _)) = &value.super_class {
                    ty(&Type::Java(name.clone()), names);
                }
                for name in &value.interfaces {
                    ty(&Type::Java(name.clone()), names);
                }
                for (_, value) in &value.params {
                    ty(value, names);
                }
                for value in &value.properties {
                    annotations(&value.annotations, names);
                    ty(&value.ty, names);
                }
                for value in &value.methods {
                    function(&value.function, names);
                }
                for value in &value.secondary_constructors {
                    for (_, value) in &value.params {
                        ty(value, names);
                    }
                }
                if let Some(value) = &value.companion {
                    for value in &value.properties {
                        property(value, names);
                    }
                    for value in &value.methods {
                        function(&value.function, names);
                    }
                }
                for value in &value.nested {
                    declaration(value, names);
                }
            }
            crate::syntax::Declaration::Object(value) => {
                annotations(&value.annotation_uses, names);
                for name in &value.interfaces {
                    ty(&Type::Java(name.clone()), names);
                }
                for value in &value.fields {
                    annotations(&value.annotations, names);
                    if let Some(value) = &value.ty {
                        ty(value, names);
                    }
                }
                for value in &value.methods {
                    function(&value.function, names);
                }
                for value in &value.nested {
                    declaration(value, names);
                }
                for value in &value.objects {
                    declaration(&crate::syntax::Declaration::Object(value.clone()), names);
                }
            }
            crate::syntax::Declaration::Interface(value) => {
                for parameter in &value.type_params {
                    for bound in &parameter.bounds {
                        ty(bound, names);
                    }
                }
                for supertype in &value.super_types {
                    ty(supertype, names);
                }
                annotations(&value.annotations, names);
                for name in &value.interfaces {
                    ty(&Type::Java(name.clone()), names);
                }
                for value in &value.properties {
                    annotations(&value.annotations, names);
                    ty(&value.ty, names);
                }
                for value in &value.methods {
                    function(&value.function, names);
                }
                if let Some(value) = &value.companion {
                    for value in &value.fields {
                        property(value, names);
                    }
                    for value in &value.methods {
                        function(&value.function, names);
                    }
                }
                for value in &value.declarations {
                    declaration(value, names);
                }
                for value in &value.nested {
                    declaration(&crate::syntax::Declaration::Interface(value.clone()), names);
                }
            }
            crate::syntax::Declaration::Annotation(_) | crate::syntax::Declaration::Enum(_) => {}
        }
    }
    let mut names = std::collections::BTreeSet::new();
    for value in &module.functions {
        function(value, &mut names);
    }
    for value in &module.properties {
        annotations(&value.annotations, &mut names);
        ty(&value.ty, &mut names);
    }
    for value in &module.constants {
        annotations(&value.annotations, &mut names);
    }
    if let Some(value) = &module.class_decl {
        declaration(
            &crate::syntax::Declaration::Class(value.clone()),
            &mut names,
        );
    }
    if let Some(value) = &module.object_decl {
        declaration(
            &crate::syntax::Declaration::Object(value.clone()),
            &mut names,
        );
    }
    if let Some(value) = &module.interface_decl {
        declaration(
            &crate::syntax::Declaration::Interface(value.clone()),
            &mut names,
        );
    }
    for name in names {
        if name.contains('/')
            || name.contains('.')
            || name.contains('$')
            || module.imports.iter().any(|(alias, _, _)| alias == &name)
        {
            continue;
        }
        let own = module
            .class_decl
            .as_ref()
            .is_some_and(|value| value.name == name)
            || module
                .object_decl
                .as_ref()
                .is_some_and(|value| value.name == name)
            || module
                .interface_decl
                .as_ref()
                .is_some_and(|value| value.name == name);
        if own {
            continue;
        }
        let source_package = module.package.replace('.', "/");
        for package in std::iter::once(source_package.as_str())
            .chain(std::iter::once("java/lang"))
            .chain(DEFAULT_KOTLIN_PACKAGES.iter().copied())
        {
            let path = if package.is_empty() {
                name.clone()
            } else {
                format!("{package}/{name}")
            };
            if cp.load(&path).is_ok_and(|class| class.access & 1 != 0) {
                module.imports.push((name.clone(), path, 0));
                break;
            }
        }
    }
}

pub(super) fn expand_aliases(module: &mut Module, cp: &Classpath) -> Result<()> {
    if let Some(mut enumeration) = module.enum_decl.take() {
        let mut context = module.clone();
        context.class_decl = Some(enumeration.body.clone());
        let result = expand_aliases(&mut context, cp);
        module.imports = context.imports;
        module.type_aliases = context.type_aliases;
        enumeration.body = context.class_decl.unwrap();
        module.enum_decl = Some(enumeration);
        return result;
    }
    let imports = module
        .imports
        .iter()
        .map(|(alias, path, _)| (alias.clone(), path.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let expand = |ty: &mut Type| -> Result<()> {
        *ty = cp.expand_type_alias(ty, &imports, DEFAULT_KOTLIN_PACKAGES)?;
        Ok(())
    };
    fn function(value: &mut Function, expand: &impl Fn(&mut Type) -> Result<()>) -> Result<()> {
        if let Some(ty) = &mut value.receiver {
            expand(ty)?;
        }
        for (_, ty) in &mut value.params {
            expand(ty)?;
        }
        expand(&mut value.result)?;
        for parameter in &mut value.type_params {
            for bound in &mut parameter.bounds {
                expand(bound)?;
            }
        }
        Ok(())
    }
    fn state(
        value: &mut crate::syntax::ClassCompanion,
        expand: &impl Fn(&mut Type) -> Result<()>,
    ) -> Result<()> {
        for property in &mut value.properties {
            if let Some(ty) = &mut property.ty {
                expand(ty)?;
            }
        }
        for method in &mut value.methods {
            function(&mut method.function, expand)?;
        }
        Ok(())
    }
    fn declaration(
        value: &mut crate::syntax::Declaration,
        expand: &impl Fn(&mut Type) -> Result<()>,
    ) -> Result<()> {
        match value {
            crate::syntax::Declaration::Class(value) => {
                for (_, ty) in &mut value.params {
                    expand(ty)?;
                }
                for property in &mut value.properties {
                    expand(&mut property.ty)?;
                }
                for method in &mut value.methods {
                    function(&mut method.function, expand)?;
                }
                for constructor in &mut value.secondary_constructors {
                    for (_, ty) in &mut constructor.params {
                        expand(ty)?;
                    }
                }
                if let Some(companion) = &mut value.companion {
                    state(companion, expand)?;
                }
                for child in &mut value.nested {
                    declaration(child, expand)?;
                }
            }
            crate::syntax::Declaration::Object(value) => {
                for property in &mut value.fields {
                    if let Some(ty) = &mut property.ty {
                        expand(ty)?;
                    }
                }
                for method in &mut value.methods {
                    function(&mut method.function, expand)?;
                }
                for child in &mut value.nested {
                    declaration(child, expand)?;
                }
            }
            crate::syntax::Declaration::Interface(value) => {
                for property in &mut value.properties {
                    expand(&mut property.ty)?;
                }
                for method in &mut value.methods {
                    function(&mut method.function, expand)?;
                }
                if let Some(companion) = &mut value.companion {
                    for property in &mut companion.fields {
                        if let Some(ty) = &mut property.ty {
                            expand(ty)?;
                        }
                    }
                    for method in &mut companion.methods {
                        function(&mut method.function, expand)?;
                    }
                }
                for child in &mut value.declarations {
                    declaration(child, expand)?;
                }
            }
            crate::syntax::Declaration::Annotation(_) | crate::syntax::Declaration::Enum(_) => {}
        }
        Ok(())
    }
    for alias in &mut module.type_aliases {
        expand(&mut alias.expanded).map_err(|mut error| {
            error.offset = alias.offset;
            error
        })?;
    }
    for function_value in &mut module.functions {
        function(function_value, &expand)?;
    }
    for property in &mut module.properties {
        expand(&mut property.ty)?;
    }
    if let Some(value) = module.class_decl.take() {
        let mut value = crate::syntax::Declaration::Class(value);
        declaration(&mut value, &expand)?;
        if let crate::syntax::Declaration::Class(value) = value {
            module.class_decl = Some(value);
        }
    }
    if let Some(value) = module.object_decl.take() {
        let mut value = crate::syntax::Declaration::Object(value);
        declaration(&mut value, &expand)?;
        if let crate::syntax::Declaration::Object(value) = value {
            module.object_decl = Some(value);
        }
    }
    if let Some(value) = module.interface_decl.take() {
        let mut value = crate::syntax::Declaration::Interface(value);
        declaration(&mut value, &expand)?;
        if let crate::syntax::Declaration::Interface(value) = value {
            module.interface_decl = Some(value);
        }
    }
    Ok(())
}
