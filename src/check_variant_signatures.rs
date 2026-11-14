use super::*;

fn final_type(ty: &Type, cp: &Classpath) -> bool {
    match ty.base() {
        Type::Nullable(inner) => final_type(inner, cp),
        Type::Parameter { .. } => false,
        Type::Java(owner) | Type::Parameterized { name: owner, .. } => {
            cp.load(owner).is_ok_and(|c| c.access & 0x10 != 0)
        }
        Type::List(_)
        | Type::MutableList(_)
        | Type::ThreadLocal(_)
        | Type::Function { .. }
        | Type::SuspendFunction { .. }
        | Type::ReceiverFunction { .. } => false,
        _ => true,
    }
}
fn object_type(ty: &Type) -> bool {
    match ty.base() {
        Type::Nullable(inner) => object_type(inner),
        Type::Java(owner) => owner == "java/lang/Object",
        _ => false,
    }
}
fn signature_type(ty: &Type, cp: &Classpath, offset: usize, contextual: bool) -> Result<()> {
    match ty.base() {
        Type::Parameterized { name, arguments } => {
            let declaration = cp.load(name).ok().and_then(|c| c.library);
            for (index, argument) in arguments.iter().enumerate() {
                let Some(ty) = &argument.ty else { continue };
                let variance = declaration
                    .as_ref()
                    .and_then(|c| c.type_parameters.get(index))
                    .map_or(syntax::Variance::Invariant, |p| p.variance);
                if variance != syntax::Variance::Invariant
                    && (argument.variance != syntax::Variance::Invariant
                        || contextual
                            && match variance {
                                syntax::Variance::Out => !final_type(ty, cp),
                                syntax::Variance::In => !object_type(ty),
                                syntax::Variance::Invariant => false,
                            })
                {
                    return Err(Diagnostic::new(offset, "Exposed declaration variance requires unsupported JVM wildcard signature lowering").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "VARIANT_JVM_SIGNATURE_UNSUPPORTED"));
                }
                signature_type(ty, cp, offset, contextual)?;
            }
        }
        Type::Nullable(t)
        | Type::Array(t)
        | Type::List(t)
        | Type::MutableList(t)
        | Type::ThreadLocal(t) => signature_type(t, cp, offset, contextual)?,
        Type::Function { params, result } | Type::SuspendFunction { params, result } => {
            for ty in params {
                signature_type(ty, cp, offset, contextual)?;
            }
            signature_type(result, cp, offset, contextual)?;
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            signature_type(receiver, cp, offset, contextual)?;
            for ty in params {
                signature_type(ty, cp, offset, contextual)?;
            }
            signature_type(result, cp, offset, contextual)?;
        }
        _ => {}
    }
    Ok(())
}
fn function_types(
    function: &Function,
    cp: &Classpath,
    offset: usize,
    contextual: bool,
) -> Result<()> {
    if !function.private {
        for parameter in &function.type_params {
            for bound in &parameter.bounds {
                signature_type(bound, cp, offset, false)?;
            }
        }
        for (_, ty) in &function.params {
            signature_type(ty, cp, offset, contextual)?;
        }
        signature_type(&function.result, cp, offset, false)?;
    }
    Ok(())
}
fn function(function: &Function, cp: &Classpath, offset: usize) -> Result<()> {
    function_types(function, cp, offset, true)
}
fn methods(sources: &[syntax::ClassMethod], methods: &[Function], cp: &Classpath) -> Result<()> {
    for method in sources {
        for checked in methods.iter().filter(|f| f.name == method.function.name) {
            function(checked, cp, method.function.offset)?;
        }
    }
    Ok(())
}
pub(crate) fn validate_class(
    source: &syntax::ClassDecl,
    checked: &ClassDecl,
    cp: &Classpath,
) -> Result<()> {
    for parameter in &checked.type_params {
        for bound in &parameter.bounds {
            signature_type(bound, cp, source.offset, false)?;
        }
    }
    for ty in &checked.super_types {
        signature_type(ty, cp, source.offset, false)?;
    }
    for (_, ty) in &checked.params {
        signature_type(ty, cp, source.offset, true)?;
    }
    for constructor in &checked.secondary_constructors {
        function(&constructor.function, cp, source.offset)?;
    }
    methods(&source.methods, &checked.methods, cp)?;
    for method in &checked.methods {
        function_types(method, cp, source.offset, false)?;
    }
    if let (Some(source), Some(checked)) = (&source.companion, &checked.companion) {
        methods(&source.methods, &checked.methods, cp)?;
        for method in &checked.methods {
            function_types(method, cp, 0, false)?;
        }
        for property in &checked.properties {
            if !property.private {
                signature_type(&property.ty, cp, 0, false)?;
            }
        }
    }
    Ok(())
}
fn interface(
    source: &syntax::InterfaceDecl,
    checked: &InterfaceDecl,
    cp: &Classpath,
) -> Result<()> {
    for parameter in &checked.type_params {
        for bound in &parameter.bounds {
            signature_type(bound, cp, source.offset, false)?;
        }
    }
    for ty in &checked.super_types {
        signature_type(ty, cp, source.offset, false)?;
    }
    for method in &checked.methods {
        function_types(&method.function, cp, source.offset, false)?;
        if method.source_property.is_none() {
            function(&method.function, cp, source.offset)?;
        }
    }
    for (source, checked) in source.nested.iter().zip(&checked.nested) {
        interface(source, checked, cp)?;
    }
    if let (Some(source), Some(checked)) = (&source.companion, &checked.companion) {
        if let Some(state) = &checked.state {
            methods(&source.methods, &state.methods, cp)?;
            for method in &state.methods {
                function_types(method, cp, 0, false)?;
            }
            for property in &state.properties {
                if !property.private {
                    signature_type(&property.ty, cp, 0, false)?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_module(
    source: &syntax::Module,
    checked: &Module,
    classpath: &Classpath,
) -> Result<()> {
    for function in &checked.functions {
        function_types(function, classpath, 0, false)?;
    }
    for property in &checked.properties {
        if !property.private {
            signature_type(&property.ty, classpath, 0, false)?;
        }
    }
    for source in &source.functions {
        if source.extension_property.is_some() {
            continue;
        }
        for checked in checked.functions.iter().filter(|f| f.name == source.name) {
            function(checked, classpath, source.offset)?;
        }
    }
    if let (Some(source), Some(checked)) = (&source.class_decl, &checked.class_decl) {
        validate_class(source, checked, classpath)?;
    }
    if let (Some(source), Some(checked)) = (&source.interface_decl, &checked.interface_decl) {
        interface(source, checked, classpath)?;
    }
    if let (Some(source), Some(checked)) = (&source.object_decl, &checked.object_decl) {
        for method in &checked.methods {
            function_types(&method.function, classpath, source.offset, false)?;
        }
        for field in &checked.fields {
            signature_type(&field.ty, classpath, source.offset, false)?;
        }
        for method in &source.methods {
            for checked in checked
                .methods
                .iter()
                .filter(|f| f.function.name == method.function.name)
            {
                function(&checked.function, classpath, method.function.offset)?;
            }
        }
    }
    Ok(())
}
