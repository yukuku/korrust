use super::*;

#[derive(Clone, Copy)]
enum Mode {
    Input,
    Output,
    Bound,
}
impl Mode {
    fn argument(self, variance: syntax::Variance) -> Self {
        match (self, variance) {
            (Self::Input, syntax::Variance::Invariant) => Self::Output,
            (Self::Output, syntax::Variance::In) => Self::Input,
            _ => self,
        }
    }
}
fn declaration_variances(owner: &str, cp: &Classpath) -> Vec<syntax::Variance> {
    cp.load(owner)
        .ok()
        .and_then(|c| c.library)
        .map(|c| c.type_parameters.into_iter().map(|p| p.variance).collect())
        .unwrap_or_default()
}
fn object_type(ty: &Type) -> bool {
    match ty.base() {
        Type::Nullable(inner) => object_type(inner),
        Type::Java(owner) => owner == "java/lang/Object",
        _ => false,
    }
}
fn final_type(ty: &Type, cp: &Classpath) -> bool {
    match ty.base() {
        Type::Nullable(inner) => final_type(inner, cp),
        Type::Parameter { .. } => false,
        Type::Java(owner) => cp.load(owner).is_ok_and(|c| c.access & 0x10 != 0),
        Type::Parameterized { name, arguments } => {
            cp.load(name).is_ok_and(|c| c.access & 0x10 != 0)
                && arguments.iter().enumerate().all(|(i, a)| {
                    let v = declaration_variances(name, cp)
                        .get(i)
                        .copied()
                        .unwrap_or(syntax::Variance::Invariant);
                    a.ty.as_ref()
                        .is_some_and(|ty| match effective(v, a.variance) {
                            syntax::Variance::Invariant => true,
                            syntax::Variance::Out => final_type(ty, cp),
                            syntax::Variance::In => object_type(ty),
                        })
                })
                && arguments.iter().all(|a| a.ty.is_some())
        }
        Type::List(_)
        | Type::MutableList(_)
        | Type::FlexibleList(_)
        | Type::Function { .. }
        | Type::ComposableUnitFunction
        | Type::SuspendFunction { .. }
        | Type::ReceiverFunction { .. }
        | Type::SuspendReceiverFunction { .. } => false,
        Type::ThreadLocal(_) => false,
        _ => true,
    }
}
fn effective(declared: syntax::Variance, projected: syntax::Variance) -> syntax::Variance {
    if projected == syntax::Variance::Invariant {
        declared
    } else {
        projected
    }
}
fn wildcard(
    declared: syntax::Variance,
    projected: syntax::Variance,
    ty: &Type,
    cp: &Classpath,
    mode: Mode,
) -> &'static str {
    use syntax::Variance::*;
    let variance = if declared == Invariant {
        projected
    } else if matches!(mode, Mode::Output)
        || !matches!(mode, Mode::Bound)
            && (declared == Out && final_type(ty, cp) || declared == In && object_type(ty))
    {
        Invariant
    } else {
        declared
    };
    match variance {
        Invariant => "",
        In => "-",
        Out => "+",
    }
}
fn boxed(ty: &Type) -> Option<&'static str> {
    Some(match ty.base() {
        Type::Int => "Ljava/lang/Integer;",
        Type::Long => "Ljava/lang/Long;",
        Type::Float => "Ljava/lang/Float;",
        Type::Double => "Ljava/lang/Double;",
        Type::Byte => "Ljava/lang/Byte;",
        Type::Short => "Ljava/lang/Short;",
        Type::Char => "Ljava/lang/Character;",
        Type::Boolean => "Ljava/lang/Boolean;",
        Type::Unit => "Lkotlin/Unit;",
        _ => return None,
    })
}
fn parameterized(
    owner: &str,
    args: &[syntax::TypeArgument],
    variances: &[syntax::Variance],
    cp: &Classpath,
    mode: Mode,
    names: &[&str],
) -> String {
    let args = args
        .iter()
        .enumerate()
        .map(|(i, a)| match &a.ty {
            None => "*".into(),
            Some(ty) => {
                let declared = variances
                    .get(i)
                    .copied()
                    .unwrap_or(syntax::Variance::Invariant);
                format!(
                    "{}{}",
                    wildcard(declared, a.variance, ty, cp, mode),
                    signature_type(
                        ty,
                        cp,
                        mode.argument(effective(declared, a.variance)),
                        names,
                        true
                    )
                )
            }
        })
        .collect::<String>();
    format!("L{owner}<{args}>;")
}
fn signature_type(ty: &Type, cp: &Classpath, mode: Mode, names: &[&str], argument: bool) -> String {
    if argument && let Some(boxed) = boxed(ty) {
        return boxed.into();
    }
    match ty {
        Type::ComposableUnitFunction => {
            signature_type(&Type::composable_runtime(), cp, mode, names, argument)
        }
        Type::Parameter { name, upper_bound } => {
            if names.contains(&name.as_str()) {
                format!("T{name};")
            } else {
                signature_type(upper_bound, cp, mode, names, argument)
            }
        }
        Type::Nullable(t) if !t.is_reference() => boxed(t).unwrap_or("Ljava/lang/Object;").into(),
        Type::Nullable(t) | Type::Platform(t) => signature_type(t, cp, mode, names, argument),
        Type::Parameterized { name, arguments } => parameterized(
            name,
            arguments,
            &declaration_variances(name, cp),
            cp,
            mode,
            names,
        ),
        Type::List(t) | Type::MutableList(t) | Type::FlexibleList(t) => parameterized(
            "java/util/List",
            &[syntax::TypeArgument {
                variance: syntax::Variance::Invariant,
                ty: Some(t.as_ref().clone()),
            }],
            &[if matches!(ty, Type::List(_)) {
                syntax::Variance::Out
            } else {
                syntax::Variance::Invariant
            }],
            cp,
            mode,
            names,
        ),
        Type::ThreadLocal(t) => parameterized(
            "java/lang/ThreadLocal",
            &[syntax::TypeArgument {
                variance: syntax::Variance::Invariant,
                ty: Some(t.as_ref().clone()),
            }],
            &[],
            cp,
            mode,
            names,
        ),
        Type::Array(t) | Type::OutArray(t) => format!(
            "[{}",
            signature_type(
                t,
                cp,
                mode.argument(syntax::Variance::Invariant),
                names,
                true
            )
        ),
        Type::SuspendFunction { params, result } => signature_type(
            &Type::suspend_runtime(params, result),
            cp,
            mode,
            names,
            argument,
        ),
        Type::SuspendReceiverFunction {
            receiver,
            params,
            result,
        } => {
            let mut all = vec![receiver.as_ref().clone()];
            all.extend(params.clone());
            signature_type(
                &Type::suspend_runtime(&all, result),
                cp,
                mode,
                names,
                argument,
            )
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            let mut all = vec![receiver.as_ref().clone()];
            all.extend(params.clone());
            signature_type(
                &Type::Function {
                    params: all,
                    result: result.clone(),
                },
                cp,
                mode,
                names,
                argument,
            )
        }
        Type::Function { params, result } => {
            let args = params
                .iter()
                .chain(std::iter::once(result.as_ref()))
                .map(|t| syntax::TypeArgument {
                    variance: syntax::Variance::Invariant,
                    ty: Some(t.clone()),
                })
                .collect::<Vec<_>>();
            let mut variances = vec![syntax::Variance::In; params.len()];
            variances.push(syntax::Variance::Out);
            parameterized(
                &format!("kotlin/jvm/functions/Function{}", params.len()),
                &args,
                &variances,
                cp,
                mode,
                names,
            )
        }
        _ => ty.descriptor(),
    }
}
fn declarations(parameters: &[syntax::TypeParameter], cp: &Classpath, names: &[&str]) -> String {
    if parameters.is_empty() {
        return String::new();
    }
    let content = parameters
        .iter()
        .map(|p| {
            let bound = p
                .bounds
                .first()
                .map(|t| signature_type(t, cp, Mode::Bound, names, true))
                .unwrap_or("Ljava/lang/Object;".into());
            let interface = p
                .bounds
                .first()
                .and_then(library::type_owner)
                .and_then(|o| cp.load(o).ok())
                .is_some_and(|c| c.access & 0x200 != 0);
            format!("{}:{}{bound}", p.name, if interface { ":" } else { "" })
        })
        .collect::<String>();
    format!("<{content}>")
}
fn function(function: &mut Function, cp: &Classpath, contextual: bool) {
    let names = function
        .class_type_params
        .iter()
        .map(String::as_str)
        .chain(function.type_params.iter().map(|p| p.name.as_str()))
        .collect::<Vec<_>>();
    let mut signature = declarations(&function.type_params, cp, &names);
    signature.push('(');
    for (_, ty) in &function.params {
        signature.push_str(&signature_type(
            ty,
            cp,
            if contextual {
                Mode::Input
            } else {
                Mode::Output
            },
            &names,
            false,
        ));
    }
    if function.suspend_ {
        signature.push_str("Lkotlin/coroutines/Continuation<-");
        signature.push_str(&signature_type(
            &function.result,
            cp,
            Mode::Output,
            &names,
            true,
        ));
        signature.push_str(">;");
    }
    signature.push(')');
    signature.push_str(&signature_type(
        function.physical_result(),
        cp,
        Mode::Output,
        &names,
        false,
    ));
    function.jvm_signature = (signature != function.descriptor()).then_some(signature);
}
fn projected_type(ty: &Type, cp: &Classpath, mode: Mode) -> Type {
    fn classifier(
        owner: &str,
        arguments: &[syntax::TypeArgument],
        variances: &[syntax::Variance],
        cp: &Classpath,
        mode: Mode,
    ) -> Type {
        Type::Parameterized {
            name: owner.into(),
            arguments: arguments
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let declared = variances
                        .get(i)
                        .copied()
                        .unwrap_or(syntax::Variance::Invariant);
                    syntax::TypeArgument {
                        variance: a.ty.as_ref().map_or(syntax::Variance::Invariant, |t| {
                            match wildcard(declared, a.variance, t, cp, mode) {
                                "+" => syntax::Variance::Out,
                                "-" => syntax::Variance::In,
                                _ => syntax::Variance::Invariant,
                            }
                        }),
                        ty: a.ty.as_ref().map(|t| {
                            projected_type(t, cp, mode.argument(effective(declared, a.variance)))
                        }),
                    }
                })
                .collect(),
        }
    }
    fn argument(ty: &Type) -> syntax::TypeArgument {
        syntax::TypeArgument {
            variance: syntax::Variance::Invariant,
            ty: Some(ty.clone()),
        }
    }
    match ty {
        Type::Nullable(t) => Type::Nullable(Box::new(projected_type(t, cp, mode))),
        Type::Platform(t) => Type::Platform(Box::new(projected_type(t, cp, mode))),
        Type::Array(t) => Type::Array(Box::new(projected_type(
            t,
            cp,
            mode.argument(syntax::Variance::Invariant),
        ))),
        Type::OutArray(t) => Type::OutArray(Box::new(projected_type(
            t,
            cp,
            mode.argument(syntax::Variance::Invariant),
        ))),
        Type::Parameterized { name, arguments } => {
            classifier(name, arguments, &declaration_variances(name, cp), cp, mode)
        }
        Type::List(t) | Type::MutableList(t) | Type::FlexibleList(t) => classifier(
            "java/util/List",
            &[argument(t)],
            &[if matches!(ty, Type::List(_)) {
                syntax::Variance::Out
            } else {
                syntax::Variance::Invariant
            }],
            cp,
            mode,
        ),
        Type::ThreadLocal(t) => classifier("java/lang/ThreadLocal", &[argument(t)], &[], cp, mode),
        Type::Function { params, result } => {
            let arguments = params
                .iter()
                .chain(std::iter::once(result.as_ref()))
                .map(argument)
                .collect::<Vec<_>>();
            let mut variances = vec![syntax::Variance::In; params.len()];
            variances.push(syntax::Variance::Out);
            classifier(
                &format!("kotlin/jvm/functions/Function{}", params.len()),
                &arguments,
                &variances,
                cp,
                mode,
            )
        }
        Type::SuspendReceiverFunction {
            receiver,
            params,
            result,
        } => {
            let mut all = vec![receiver.as_ref().clone()];
            all.extend(params.clone());
            projected_type(&Type::suspend_runtime(&all, result), cp, mode)
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            let mut all = vec![receiver.as_ref().clone()];
            all.extend(params.clone());
            projected_type(
                &Type::Function {
                    params: all,
                    result: result.clone(),
                },
                cp,
                mode,
            )
        }
        Type::SuspendFunction { params, result } => {
            projected_type(&Type::suspend_runtime(params, result), cp, mode)
        }
        _ => ty.clone(),
    }
}
fn bridge_types(bridges: &mut [JavaMethod], cp: &Classpath) {
    for bridge in bridges {
        let mode = if bridge.source_property.is_none() {
            Mode::Input
        } else {
            Mode::Output
        };
        if let Some(params) = &mut bridge.kotlin_params {
            for (_, ty, _) in params {
                *ty = projected_type(ty, cp, mode);
            }
        }
        if let Some(result) = &mut bridge.kotlin_result {
            *result = projected_type(result, cp, Mode::Output);
        }
    }
}
fn methods(sources: &[syntax::ClassMethod], methods: &mut [Function], cp: &Classpath) {
    for method in methods {
        function(
            method,
            cp,
            sources
                .iter()
                .any(|s| s.function.name == method.name && s.function.extension_property.is_none()),
        );
    }
}

fn jvm_field(
    uses: &[syntax::AnnotationUse],
    cp: &Classpath,
    imports: &HashMap<String, String>,
) -> bool {
    uses.iter()
        .any(|a| annotations::annotation_owner(&a.name, cp, imports) == "kotlin/jvm/JvmField")
}
fn exposed_field(ty: &Type, cp: &Classpath) -> Result<()> {
    match ty.base() {
        Type::Parameterized { name, arguments } => {
            let variances = declaration_variances(name, cp);
            for (i, a) in arguments.iter().enumerate() {
                if let Some(t) = &a.ty {
                    if a.variance != syntax::Variance::Invariant
                        && variances
                            .get(i)
                            .is_some_and(|v| *v != syntax::Variance::Invariant)
                    {
                        return Err(Diagnostic::new(0, "Projected variant JVM fields require unsupported field signature lowering").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "VARIANT_FIELD_SIGNATURE_UNSUPPORTED"));
                    }
                    exposed_field(t, cp)?;
                }
            }
        }
        Type::Nullable(t)
        | Type::Array(t)
        | Type::OutArray(t)
        | Type::List(t)
        | Type::MutableList(t)
        | Type::FlexibleList(t)
        | Type::ThreadLocal(t) => exposed_field(t, cp)?,
        Type::Function { params, result } | Type::SuspendFunction { params, result } => {
            for p in params {
                exposed_field(p, cp)?;
            }
            exposed_field(result, cp)?;
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
            exposed_field(receiver, cp)?;
            for p in params {
                exposed_field(p, cp)?;
            }
            exposed_field(result, cp)?;
        }
        _ => {}
    }
    Ok(())
}
fn classifier_context(ty: &Type, cp: &Classpath, bound: bool, offset: usize) -> Result<()> {
    match ty.base() {
        Type::Nullable(t)
        | Type::Array(t)
        | Type::OutArray(t)
        | Type::MutableList(t)
        | Type::FlexibleList(t)
        | Type::ThreadLocal(t) => classifier_context(t, cp, true, offset)?,
        Type::List(t) => {
            if bound {
                return Err(classifier_boundary(offset));
            }
            classifier_context(t, cp, true, offset)?;
        }
        Type::Parameterized { name, arguments } => {
            let variances = declaration_variances(name, cp);
            for (i, a) in arguments.iter().enumerate() {
                if let Some(t) = &a.ty {
                    if bound
                        && variances
                            .get(i)
                            .is_some_and(|v| *v != syntax::Variance::Invariant)
                    {
                        return Err(classifier_boundary(offset));
                    }
                    classifier_context(t, cp, true, offset)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
fn classifier_boundary(offset: usize) -> Diagnostic {
    Diagnostic::new(
        offset,
        "Contextual variant classifier bounds and nested supertypes are unsupported",
    )
    .with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "VARIANT_CLASSIFIER_SIGNATURE_UNSUPPORTED",
    )
}
fn classifier_types(
    parameters: &[syntax::TypeParameter],
    supertypes: &[Type],
    cp: &Classpath,
    offset: usize,
) -> Result<()> {
    for p in parameters {
        for bound in &p.bounds {
            classifier_context(bound, cp, true, offset)?;
        }
    }
    for ty in supertypes {
        classifier_context(ty, cp, false, offset)?;
    }
    Ok(())
}
pub(crate) fn validate_class(
    source: &syntax::ClassDecl,
    checked: &mut ClassDecl,
    cp: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<()> {
    classifier_types(
        &checked.type_params,
        &checked.super_types,
        cp,
        source.offset,
    )?;
    for field in &checked.fields {
        if checked.exposed_fields.contains_key(&field.name) {
            exposed_field(&field.ty, cp)?;
        }
    }
    for property in &source.properties {
        if jvm_field(&property.annotations, cp, imports)
            && let Some(field) = checked.fields.iter().find(|f| f.name == property.name)
        {
            exposed_field(&field.ty, cp)?;
        }
    }
    for ((name, _), uses) in source.params.iter().zip(&source.param_annotations) {
        if jvm_field(uses, cp, imports)
            && let Some(field) = checked.fields.iter().find(|f| f.name == *name)
        {
            exposed_field(&field.ty, cp)?;
        }
    }
    let names = checked
        .type_params
        .iter()
        .map(|p| p.name.as_str())
        .collect::<Vec<_>>();
    let hidden = usize::from(source.inner);
    let hidden = hidden.min(checked.params.len());
    let signature = format!(
        "({})V",
        checked.params[hidden..]
            .iter()
            .map(|(_, ty)| signature_type(ty, cp, Mode::Input, &names, false))
            .collect::<String>()
    );
    let descriptor = format!(
        "({})V",
        checked.params[hidden..]
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    checked.constructor_signature = (hidden != 0 || signature != descriptor).then_some(signature);
    for constructor in &mut checked.secondary_constructors {
        constructor.function.class_type_params =
            checked.type_params.iter().map(|p| p.name.clone()).collect();
        function(&mut constructor.function, cp, true);
    }
    bridge_types(&mut checked.bridges, cp);
    for default in &mut checked.default_functions {
        function(default, cp, true);
    }
    methods(&source.methods, &mut checked.methods, cp);
    if let (Some(source), Some(checked)) = (&source.companion, &mut checked.companion) {
        methods(&source.methods, &mut checked.methods, cp);
        for property in &source.properties {
            if jvm_field(&property.annotations, cp, imports)
                && let Some(field) = checked.properties.iter().find(|f| f.name == property.name)
            {
                exposed_field(&field.ty, cp)?;
            }
        }
        for field in &checked.properties {
            if checked.exposed_fields.contains_key(&field.name) {
                exposed_field(&field.ty, cp)?;
            }
        }
    }
    Ok(())
}
fn interface(
    source: &syntax::InterfaceDecl,
    checked: &mut InterfaceDecl,
    cp: &Classpath,
    imports: &HashMap<String, String>,
) -> Result<()> {
    classifier_types(
        &checked.type_params,
        &checked.super_types,
        cp,
        source.offset,
    )?;
    bridge_types(&mut checked.defaults, cp);
    for default in &mut checked.default_functions {
        function(
            default,
            cp,
            !checked
                .defaults
                .iter()
                .any(|m| m.name == default.name && m.source_property.is_some()),
        );
    }
    for method in &mut checked.methods {
        function(&mut method.function, cp, method.source_property.is_none());
    }
    for (source, checked) in source.nested.iter().zip(&mut checked.nested) {
        interface(source, checked, cp, imports)?;
    }
    if let (Some(source), Some(checked)) = (&source.companion, &mut checked.companion)
        && let Some(state) = &mut checked.state
    {
        methods(&source.methods, &mut state.methods, cp);
        for property in &source.fields {
            if jvm_field(&property.annotations, cp, imports)
                && let Some(field) = state.properties.iter().find(|f| f.name == property.name)
            {
                exposed_field(&field.ty, cp)?;
            }
        }
        for field in &state.properties {
            if state.exposed_fields.contains_key(&field.name) {
                exposed_field(&field.ty, cp)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_module(
    source: &syntax::Module,
    checked: &mut Module,
    cp: &Classpath,
) -> Result<()> {
    let imports = crate::annotation_declarations::imports(source, cp)?;
    for property in &source.properties {
        if jvm_field(&property.annotations, cp, &imports)
            && let Some(field) = checked.properties.iter().find(|f| f.name == property.name)
        {
            exposed_field(&field.ty, cp)?;
        }
    }
    for field in &checked.properties {
        if checked.exposed_fields.contains_key(&field.name) {
            exposed_field(&field.ty, cp)?;
        }
    }
    for checked in &mut checked.functions {
        let contextual = source
            .functions
            .iter()
            .any(|s| s.name == checked.name && s.extension_property.is_none());
        function(checked, cp, contextual);
    }
    if let (Some(source), Some(checked)) = (&source.class_decl, &mut checked.class_decl) {
        validate_class(source, checked, cp, &imports)?;
    }
    if let (Some(source), Some(checked)) = (&source.interface_decl, &mut checked.interface_decl) {
        interface(source, checked, cp, &imports)?;
    }
    if let Some(checked) = &mut checked.object_decl {
        if let Some(source) = &source.object_decl {
            for property in &source.fields {
                if jvm_field(&property.annotations, cp, &imports)
                    && let Some(field) = checked.fields.iter().find(|f| f.name == property.name)
                {
                    exposed_field(&field.ty, cp)?;
                }
            }
        }
        bridge_types(&mut checked.bridges, cp);
        for default in &mut checked.default_functions {
            function(default, cp, true);
        }
        for field in &checked.fields {
            if checked.exposed_fields.contains_key(&field.name) {
                exposed_field(&field.ty, cp)?;
            }
        }
        for method in &mut checked.methods {
            let contextual = source.object_decl.as_ref().is_some_and(|s| {
                s.methods.iter().any(|m| {
                    m.function.name == method.function.name
                        && m.function.extension_property.is_none()
                })
            });
            function(&mut method.function, cp, contextual);
        }
    }
    Ok(())
}
