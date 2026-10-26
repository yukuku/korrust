use super::*;
use crate::bytecode::annotation_files::{ClassAnnotations, MemberAnnotations};

pub(crate) fn annotation_placements(
    module: &syntax::Module,
    classpath: &Classpath,
    owner: &str,
) -> Result<std::collections::BTreeMap<String, ClassAnnotations>> {
    let mut imports = module
        .imports
        .iter()
        .map(|(name, path, _)| (name.clone(), path.clone()))
        .collect::<HashMap<_, _>>();
    let mut result = std::collections::BTreeMap::new();
    let mut output = ClassAnnotations::default();
    if let Some(class) = &module.class_decl {
        imports.insert(class.name.clone(), owner.into());
        output.annotations = checked(&class.annotations, classpath, &imports, "TYPE", None)?;
        constructor_parameters(class, false, classpath, &imports, &mut output)?;
        for property in &class.properties {
            property_annotations(
                &property.annotations,
                &property.name,
                &property.ty,
                property.mutable,
                property.private,
                false,
                classpath,
                &imports,
                &mut output,
            )?;
        }
        for function in &class.methods {
            function_parameters(&function.function, classpath, &imports, owner, &mut output)?;
        }
        if let Some(companion) = &class.companion {
            let companion_owner = format!("{owner}$Companion");
            let mut companion_output = ClassAnnotations::default();
            for method in &companion.methods {
                function_parameters(
                    &method.function,
                    classpath,
                    &imports,
                    &companion_owner,
                    &mut companion_output,
                )?;
            }
            for property in &companion.properties {
                if !property.annotations.is_empty() {
                    return Err(Diagnostic::new(
                        property.offset,
                        "Annotations on companion properties require unsupported split backing-field placement",
                    ));
                }
            }
            result.insert(companion_owner, companion_output);
        }
    } else if let Some(object) = &module.object_decl {
        imports.insert(object.name.clone(), owner.into());
        // The established empty serializable-singleton lowering already emits
        // its consumed marker and serializer API. Preserve that bounded path;
        // general plugin annotations remain rejected by ordinary validation.
        let empty_singleton = object.fields.is_empty()
            && object.methods.is_empty()
            && object.constants.is_empty()
            && object.objects.is_empty()
            && object.nested.is_empty();
        let ordinary = object
            .annotation_uses
            .iter()
            .filter(|annotation| {
                !(empty_singleton
                    && annotation.arguments.is_empty()
                    && annotation.use_site.is_none()
                    && imports
                        .get(&annotation.name)
                        .map(String::as_str)
                        .unwrap_or(&annotation.name)
                        == "kotlinx/serialization/Serializable")
            })
            .cloned()
            .collect::<Vec<_>>();
        output.annotations = checked(&ordinary, classpath, &imports, "TYPE", None)?;
        for property in &object.fields {
            if let Some(ty) = &property.ty {
                property_annotations(
                    &property.annotations,
                    &property.name,
                    ty,
                    property.mutable,
                    property.private,
                    false,
                    classpath,
                    &imports,
                    &mut output,
                )?;
            } else if !property.annotations.is_empty() {
                return Err(Diagnostic::new(
                    property.offset,
                    "Cannot place annotations on unresolved property",
                ));
            }
        }
        for method in &object.methods {
            function_parameters(&method.function, classpath, &imports, owner, &mut output)?;
        }
    } else if let Some(interface) = &module.interface_decl {
        imports.insert(interface.name.clone(), owner.into());
        output.annotations = checked(&interface.annotations, classpath, &imports, "TYPE", None)?;
        for property in &interface.properties {
            if !property.annotations.is_empty() {
                return Err(Diagnostic::new(
                    property.offset,
                    "Annotations on interface properties are not supported",
                ));
            }
        }
        for method in &interface.methods {
            function_parameters(&method.function, classpath, &imports, owner, &mut output)?;
        }
        if let Some(companion) = &interface.companion {
            if companion
                .properties
                .iter()
                .any(|property| !property.annotations.is_empty())
                || companion
                    .fields
                    .iter()
                    .any(|property| !property.annotations.is_empty())
            {
                return Err(Diagnostic::new(
                    interface.offset,
                    "Annotations on interface companion properties are not supported",
                ));
            }
        }
    } else if let Some(enumeration) = &module.enum_decl {
        imports.insert(enumeration.name.clone(), owner.into());
        output.annotations = checked(&enumeration.annotations, classpath, &imports, "TYPE", None)?;
        constructor_parameters(&enumeration.body, true, classpath, &imports, &mut output)?;
        for method in &enumeration.body.methods {
            function_parameters(&method.function, classpath, &imports, owner, &mut output)?;
        }
        for property in &enumeration.body.properties {
            property_annotations(
                &property.annotations,
                &property.name,
                &property.ty,
                property.mutable,
                property.private,
                false,
                classpath,
                &imports,
                &mut output,
            )?;
        }
    } else {
        for property in &module.properties {
            property_annotations(
                &property.annotations,
                &property.name,
                &property.ty,
                property.mutable,
                property.private,
                true,
                classpath,
                &imports,
                &mut output,
            )?;
        }
        for function in &module.functions {
            function_parameters(function, classpath, &imports, owner, &mut output)?;
        }
    }
    for constant in &module.constants {
        let annotations = checked(&constant.annotations, classpath, &imports, "FIELD", None)?;
        if !annotations.is_empty() {
            let ty = match &constant.value {
                syntax::ConstValue::Int(_) => Type::Int,
                syntax::ConstValue::Long(_) => Type::Long,
                syntax::ConstValue::Float(_) => Type::Float,
                syntax::ConstValue::Double(_) => Type::Double,
                syntax::ConstValue::Boolean(_) => Type::Boolean,
                syntax::ConstValue::Char(_) => Type::Char,
                syntax::ConstValue::String(_) => Type::String,
            };
            member(&mut output.fields, &constant.name, &ty.descriptor())
                .annotations
                .extend(annotations);
        }
    }
    result.insert(owner.into(), output);
    Ok(result)
}

fn checked(
    annotations: &[syntax::AnnotationUse],
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    target: &str,
    use_site: Option<&str>,
) -> Result<Vec<DeclarationAnnotation>> {
    annotations::declaration_annotations(annotations, classpath, imports, target, use_site)
}

fn member<'a>(
    members: &'a mut Vec<MemberAnnotations>,
    name: &str,
    descriptor: &str,
) -> &'a mut MemberAnnotations {
    let index = members
        .iter()
        .position(|member| member.name == name && member.descriptor == descriptor)
        .unwrap_or_else(|| {
            members.push(MemberAnnotations {
                name: name.into(),
                descriptor: descriptor.into(),
                annotations: Vec::new(),
                parameters: Vec::new(),
            });
            members.len() - 1
        });
    &mut members[index]
}

fn constructor_parameters(
    class: &syntax::ClassDecl,
    enumeration: bool,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    output: &mut ClassAnnotations,
) -> Result<()> {
    let mut parameters = vec![Vec::new(); class.params.len() + if enumeration { 2 } else { 0 }];
    for (index, ((name, ty), annotations)) in class
        .params
        .iter()
        .zip(&class.param_annotations)
        .enumerate()
    {
        let is_property = class.param_properties.contains(name);
        let mutable = class.mutable_params.contains(name);
        let mut parameter = Vec::new();
        let mut property = Vec::new();
        for annotation in annotations {
            match annotation.use_site.as_deref() {
                Some("param") => parameter.push(annotation.clone()),
                Some(_) if is_property => property.push(annotation.clone()),
                Some(_) => {
                    return Err(Diagnostic::new(
                        annotation.offset,
                        "Annotation use-site target requires a constructor property",
                    ));
                }
                None => {
                    let owner = imports.get(&annotation.name).unwrap_or(&annotation.name);
                    let definition =
                        classpath
                            .load(owner)?
                            .annotation_definition
                            .ok_or_else(|| {
                                Diagnostic::new(
                                    annotation.offset,
                                    "Unsupported annotation definition",
                                )
                            })?;
                    if definition
                        .targets
                        .iter()
                        .any(|target| target == "PARAMETER")
                    {
                        parameter.push(annotation.clone());
                    } else if is_property {
                        property.push(annotation.clone());
                    } else {
                        return Err(Diagnostic::new(
                            annotation.offset,
                            "Annotation cannot target a constructor parameter",
                        ));
                    }
                }
            }
        }
        let mut checked_parameter = Vec::new();
        let mut seen_parameters = HashSet::new();
        for annotation in parameter {
            let identity = imports.get(&annotation.name).unwrap_or(&annotation.name);
            if !seen_parameters.insert(identity.clone()) {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Repeated parameter annotations require a repeatable declaration",
                ));
            }
            let site = annotation.use_site.clone();
            checked_parameter.extend(checked(
                &[annotation],
                classpath,
                imports,
                "PARAMETER",
                site.as_deref(),
            )?);
        }
        parameters[index + if enumeration { 2 } else { 0 }] = checked_parameter;
        if is_property {
            property_annotations(
                &property,
                name,
                ty,
                mutable,
                !class.public_params.contains(name),
                false,
                classpath,
                imports,
                output,
            )?;
        }
    }
    if parameters.iter().any(|parameter| !parameter.is_empty()) {
        let descriptor = format!(
            "({}{})V",
            if enumeration {
                "Ljava/lang/String;I"
            } else {
                ""
            },
            class
                .params
                .iter()
                .map(|(_, ty)| resolve_type(ty, imports, class.offset).map(|ty| ty.descriptor()))
                .collect::<Result<Vec<_>>>()?
                .concat()
        );
        member(&mut output.methods, "<init>", &descriptor).parameters = parameters;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn property_annotations(
    annotations: &[syntax::AnnotationUse],
    name: &str,
    ty: &Type,
    mutable: bool,
    private: bool,
    _static_: bool,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    output: &mut ClassAnnotations,
) -> Result<()> {
    let ty = resolve_type(
        ty,
        imports,
        annotations
            .first()
            .map_or(0, |annotation| annotation.offset),
    )?;
    let mut seen = HashSet::new();
    for annotation in annotations {
        let site = annotation.use_site.clone();
        let target = site.as_deref().unwrap_or("field");
        let identity = imports.get(&annotation.name).unwrap_or(&annotation.name);
        if !seen.insert((target.to_owned(), identity.clone())) {
            return Err(Diagnostic::new(
                annotation.offset,
                "Repeated property annotations require a repeatable declaration",
            ));
        }
        match site.as_deref() {
            Some("property") => {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Kotlin property-target annotations are not supported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "ANNOTATION_PROPERTY_TARGET_UNSUPPORTED",
                ));
            }
            Some("field") | None => {
                let values = checked(
                    std::slice::from_ref(annotation),
                    classpath,
                    imports,
                    "FIELD",
                    site.as_deref(),
                )?;
                if !values.is_empty() {
                    member(&mut output.fields, name, &ty.descriptor())
                        .annotations
                        .extend(values);
                }
            }
            Some("get") | Some("set") | Some("setparam") => {
                if private {
                    return Err(Diagnostic::new(
                        annotation.offset,
                        "Annotations on private property accessors are not supported",
                    ));
                }
                if site.as_deref() != Some("get") && !mutable {
                    return Err(Diagnostic::new(
                        annotation.offset,
                        "Setter annotation requires a mutable property",
                    ));
                }
                let getter = site.as_deref() == Some("get");
                let descriptor = if getter {
                    format!("(){}", ty.descriptor())
                } else {
                    format!("({})V", ty.descriptor())
                };
                let method_name = if getter {
                    getter_name(name)
                } else {
                    setter_name(name)
                };
                let parameter = site.as_deref() == Some("setparam");
                let values = checked(
                    std::slice::from_ref(annotation),
                    classpath,
                    imports,
                    if parameter { "PARAMETER" } else { "METHOD" },
                    site.as_deref(),
                )?;
                if !values.is_empty() {
                    let method = member(&mut output.methods, &method_name, &descriptor);
                    if parameter {
                        if method.parameters.is_empty() {
                            method.parameters.push(Vec::new());
                        }
                        method.parameters[0].extend(values);
                    } else {
                        method.annotations.extend(values);
                    }
                }
            }
            _ => {
                return Err(Diagnostic::new(
                    annotation.offset,
                    "Invalid annotation use-site target for property",
                ));
            }
        }
    }
    Ok(())
}

fn function_parameters(
    function: &syntax::Function,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    owner: &str,
    output: &mut ClassAnnotations,
) -> Result<()> {
    if function.param_annotations.iter().all(Vec::is_empty) {
        return Ok(());
    }
    let mut params = Vec::new();
    let mut descriptor = String::from("(");
    if let Some(receiver) = &function.receiver {
        descriptor.push_str(&resolve_type(receiver, imports, function.offset)?.descriptor());
        params.push(Vec::new());
    }
    for ((_, ty), annotations) in function.params.iter().zip(&function.param_annotations) {
        descriptor.push_str(&resolve_type(ty, imports, function.offset)?.descriptor());
        params.push(checked(annotations, classpath, imports, "PARAMETER", None)?);
    }
    descriptor.push(')');
    descriptor.push_str(&resolve_type(&function.result, imports, function.offset)?.descriptor());
    let name = classpath
        .load(owner)?
        .methods
        .iter()
        .find(|method| {
            method.descriptor == descriptor
                && (method.name == function.name
                    || method
                        .library
                        .as_ref()
                        .is_some_and(|library| library.source_name == function.name))
        })
        .map(|method| method.name.clone())
        .unwrap_or_else(|| function.name.clone());
    member(&mut output.methods, &name, &descriptor).parameters = params;
    Ok(())
}
