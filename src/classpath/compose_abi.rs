use super::*;

pub(crate) const COMPOSER: &str = "androidx/compose/runtime/Composer";
pub(crate) const FACTORY: &str = "androidx/compose/runtime/internal/ComposableLambdaKt";
pub(crate) const WRAPPER: &str = "androidx/compose/runtime/internal/ComposableLambda";

pub(crate) fn contains(ty: &Type) -> bool {
    match ty {
        Type::ComposableUnitFunction => true,
        Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::Array(inner)
        | Type::OutArray(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::FlexibleList(inner) => contains(inner),
        Type::Parameterized { arguments, .. } => {
            arguments.iter().filter_map(|a| a.ty.as_ref()).any(contains)
        }
        Type::Parameter { upper_bound, .. } => contains(upper_bound),
        Type::Function { params, result } | Type::SuspendFunction { params, result } => {
            params.iter().any(contains) || contains(result)
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
        } => contains(receiver) || params.iter().any(contains) || contains(result),
        _ => false,
    }
}

pub(super) fn validate_member(
    params: &[(String, Type, bool)],
    result: &Type,
    receiver: Option<&Type>,
    flags: u64,
    type_parameters: &[TypeParameter],
    signature: Option<&str>,
    class_parameters: &[TypeParameter],
) -> Result<()> {
    if !params.iter().any(|(_, ty, _)| contains(ty))
        && !contains(result)
        && !receiver.is_some_and(contains)
    {
        return Ok(());
    }
    let unsupported = || {
        Diagnostic::new(0, "Unsupported Compose callable ABI shape").with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "COMPOSE_CALLABLE_ABI_UNSUPPORTED",
        )
    };
    if contains(result)
        || receiver.is_some_and(contains)
        || flags & (1024 | (1 << 13)) != 0
        || !type_parameters.is_empty()
    {
        return Err(unsupported());
    }
    let signature = signature.ok_or_else(unsupported)?;
    let (_, physical, _) = super::signature::method(signature, class_parameters)?;
    let skip = usize::from(receiver.is_some());
    if physical.len() != params.len() + skip {
        return Err(unsupported());
    }
    for ((_, ty, _), physical) in params.iter().zip(&physical[skip..]) {
        if !contains(ty) {
            continue;
        }
        if ty != &Type::ComposableUnitFunction {
            return Err(unsupported());
        }
        let Type::Parameterized { name, arguments } = physical.base() else {
            return Err(unsupported());
        };
        if name != "kotlin/jvm/functions/Function2" || arguments.len() != 3 {
            return Err(unsupported());
        }
        for (argument, expected) in
            arguments
                .iter()
                .zip([COMPOSER, "java/lang/Integer", "kotlin/Unit"])
        {
            if !argument
                .ty
                .as_ref()
                .is_some_and(|ty| matches!(ty.base(), Type::Java(name) if name == expected))
            {
                return Err(unsupported());
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_runtime(classpath: &Classpath, offset: usize) -> Result<()> {
    let unsupported = || {
        Diagnostic::new(
            offset,
            "Compose runtime does not expose the supported callback protocol",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "COMPOSE_RUNTIME_ABI_UNSUPPORTED",
        )
    };
    let composer = classpath.load(COMPOSER).map_err(|_| unsupported())?;
    if composer.access & 0x0201 != 0x0201 {
        return Err(unsupported());
    }
    for (name, descriptor) in [
        ("shouldExecute", "(ZI)Z"),
        ("skipToGroupEnd", "()V"),
        (
            "startRestartGroup",
            "(I)Landroidx/compose/runtime/Composer;",
        ),
        (
            "endRestartGroup",
            "()Landroidx/compose/runtime/ScopeUpdateScope;",
        ),
    ] {
        if !composer
            .methods
            .iter()
            .any(|m| m.name == name && m.descriptor == descriptor && m.access & 9 == 1)
        {
            return Err(unsupported());
        }
    }
    let wrapper = classpath.load(WRAPPER).map_err(|_| unsupported())?;
    if wrapper.access & 0x0201 != 0x0201
        || !wrapper
            .interfaces
            .iter()
            .any(|name| name == "kotlin/jvm/functions/Function2")
    {
        return Err(unsupported());
    }
    let trace = classpath
        .load("androidx/compose/runtime/ComposerKt")
        .map_err(|_| unsupported())?;
    if trace.access & 1 == 0 {
        return Err(unsupported());
    }
    for (name, descriptor) in [
        ("isTraceInProgress", "()Z"),
        ("traceEventStart", "(IIILjava/lang/String;)V"),
        ("traceEventEnd", "()V"),
    ] {
        if !trace.methods.iter().any(|method| {
            method.name == name && method.descriptor == descriptor && method.access & 9 == 9
        }) {
            return Err(unsupported());
        }
    }
    let factory = classpath.load(FACTORY).map_err(|_| unsupported())?;
    if factory.access & 1 == 0
        || !factory.methods.iter().any(|m| {
            m.name == "composableLambdaInstance"
                && m.descriptor
                    == "(IZLjava/lang/Object;)Landroidx/compose/runtime/internal/ComposableLambda;"
                && m.access & 9 == 9
        })
    {
        return Err(unsupported());
    }
    Ok(())
}

pub(crate) fn key(owner: &str) -> i32 {
    owner.encode_utf16().fold(0i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}
