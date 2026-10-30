use crate::{Diagnostic, DiagnosticKind, Result, check, classpath::Classpath, syntax};
use std::collections::{HashMap, HashSet};

fn unsupported(offset: usize, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(
        DiagnosticKind::UnsupportedLanguage,
        "SOURCE_INLINE_CONTEXT_UNSUPPORTED",
    )
}

pub(crate) fn stage(
    units: &[(String, syntax::Module)],
    classpath: &Classpath,
    module_name: &str,
) -> std::result::Result<(), (String, Diagnostic)> {
    let mut nodes = Vec::new();
    for (unit, (owner, module)) in units.iter().enumerate() {
        for (index, function) in module.functions.iter().enumerate() {
            if function.inline {
                if function.type_params.iter().filter(|parameter| parameter.reified).any(|parameter| parameter.bounds.iter().any(|bound| !matches!(bound, syntax::Type::Java(name) if name == "Any" || name == "java/lang/Object") && !matches!(bound, syntax::Type::Nullable(inner) if matches!(inner.as_ref(), syntax::Type::Java(name) if name == "Any" || name == "java/lang/Object")))) {
                    return Err((owner.clone(), unsupported(function.offset, "Reified source bounds other than Any are unsupported")));
                }
                if function.params.iter().zip(&function.parameter_modes).any(|((_, ty), mode)| matches!(ty, syntax::Type::Nullable(inner) if matches!(inner.as_ref(), syntax::Type::Function { .. } | syntax::Type::ReceiverFunction { .. })) && *mode != syntax::InlineParameterMode::Noinline) {
                    return Err((owner.clone(), Diagnostic::new(function.offset, "Nullable inline function parameters require noinline").with_kind(DiagnosticKind::InvalidKotlin, "NULLABLE_INLINE_PARAMETER")));
                }
                if function.receiver.is_some()
                    || function.vararg
                    || function.defaults.iter().any(Option::is_some)
                {
                    return Err((
                        owner.clone(),
                        unsupported(
                            function.offset,
                            "Source inline receivers, varargs and default arguments are unsupported",
                        ),
                    ));
                }
                nodes.push((unit, index));
            }
        }
        if module
            .class_decl
            .as_ref()
            .is_some_and(|class| class.methods.iter().any(|method| method.function.inline))
            || module
                .interface_decl
                .as_ref()
                .is_some_and(|class| class.methods.iter().any(|method| method.function.inline))
        {
            return Err((
                owner.clone(),
                unsupported(0, "Source member inline functions are unsupported"),
            ));
        }
    }
    if nodes.is_empty() {
        return Ok(());
    }
    for (owner, module) in units {
        let imports = crate::annotation_declarations::imports(module, classpath)
            .map_err(|error| (owner.clone(), error))?;
        for annotation in module
            .functions
            .iter()
            .flat_map(|function| &function.annotations)
        {
            let canonical =
                crate::annotation_declarations::owner(&annotation.name, &imports, classpath);
            if let Some(code) = check::unsupported_annotation_lowering(&canonical) {
                return Err((owner.clone(), Diagnostic::new(annotation.offset, "Source inline context contains an annotation requiring unsupported compiler semantics").with_kind(DiagnosticKind::UnsupportedLanguage, code)));
            }
        }
    }
    for (unit, index) in &nodes {
        let module = &units[*unit].1;
        let function = &module.functions[*index];
        let count = units
            .iter()
            .filter(|(_, source)| source.package == module.package)
            .flat_map(|(_, source)| &source.functions)
            .filter(|other| other.name == function.name)
            .count();
        if count > 1 {
            return Err((
                units[*unit].0.clone(),
                unsupported(
                    function.offset,
                    "Overloaded source inline declarations are unsupported",
                ),
            ));
        }
    }
    let mut dependencies = HashMap::<usize, Vec<usize>>::new();
    for (node, (unit, index)) in nodes.iter().copied().enumerate() {
        let (_, module) = &units[unit];
        let function = &module.functions[index];
        let mut names = HashSet::new();
        check::body_call_names(&function.body, &mut names);
        let free = check::body_free_names(
            &function.body,
            &function
                .params
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>(),
        );
        names.retain(|name| free.contains(name));
        for name in names {
            if function
                .params
                .iter()
                .any(|(parameter, _)| *parameter == name)
            {
                continue;
            }
            let imported = module
                .imports
                .iter()
                .find(|(alias, _, _)| *alias == name)
                .map(|(_, path, _)| path);
            let matches = nodes
                .iter()
                .enumerate()
                .filter_map(|(candidate, (candidate_unit, candidate_index))| {
                    let (_, source) = &units[*candidate_unit];
                    let other = &source.functions[*candidate_index];
                    let path = if source.package.is_empty() {
                        other.name.clone()
                    } else {
                        format!("{}/{}", source.package.replace('.', "/"), other.name)
                    };
                    ((imported == Some(&path))
                        || imported.is_none()
                            && other.name == name
                            && source.package == module.package)
                        .then_some(candidate)
                })
                .collect::<Vec<_>>();
            if !function.private
                && !function.internal
                && matches.iter().any(|candidate| {
                    let (unit, index) = nodes[*candidate];
                    units[unit].1.functions[index].private
                        || units[unit].1.functions[index].internal
                })
            {
                return Err((units[unit].0.clone(), Diagnostic::new(function.offset, "Public inline functions cannot access non-public source inline declarations").with_kind(DiagnosticKind::InvalidKotlin, "PUBLIC_INLINE_NON_PUBLIC_ACCESS")));
            }
            if matches.len() > 1 {
                return Err((
                    units[unit].0.clone(),
                    unsupported(
                        function.offset,
                        "Overloaded source inline staging requires signature-specific dependency discovery",
                    ),
                ));
            }
            dependencies.entry(node).or_default().extend(matches);
        }
    }
    fn visit(
        node: usize,
        dependencies: &HashMap<usize, Vec<usize>>,
        active: &mut HashSet<usize>,
        done: &mut HashSet<usize>,
        order: &mut Vec<usize>,
    ) -> Result<()> {
        if done.contains(&node) {
            return Ok(());
        }
        if active.len() >= 64 {
            return Err(unsupported(
                0,
                "Source inline dependency depth exceeds 64 declarations",
            ));
        }
        if !active.insert(node) {
            return Err(Diagnostic::new(
                0,
                "Inline functions cannot participate in a recursive source call cycle",
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "INLINE_RECURSION"));
        }
        for dependency in dependencies.get(&node).into_iter().flatten() {
            visit(*dependency, dependencies, active, done, order)?;
        }
        active.remove(&node);
        done.insert(node);
        order.push(node);
        Ok(())
    }
    let mut order = Vec::new();
    let mut done = HashSet::new();
    for node in 0..nodes.len() {
        visit(
            node,
            &dependencies,
            &mut HashSet::new(),
            &mut done,
            &mut order,
        )
        .map_err(|error| (units[nodes[node].0].0.clone(), error))?;
    }
    for node in order {
        let (unit, index) = nodes[node];
        let (owner, module) = &units[unit];
        let function = &module.functions[index];
        let mut source = module.clone();
        source.functions = vec![function.clone()];
        source.functions[0].private = false;
        let mut checked = check::check_with_owner(&source, classpath, owner)
            .map_err(|error| (owner.clone(), error))?;
        validate_parameter_uses(&mut checked, function, owner)
            .map_err(|error| (owner.clone(), error))?;
        let metadata = crate::metadata::for_checked_package(&source, &checked)
            .and_then(|metadata| metadata.in_module(module_name))
            .map_err(|error| (owner.clone(), error))?;
        let bytes =
            crate::bytecode::emit_with_bridges(&checked, owner, "InlineStage.kt", &metadata, &[])
                .map_err(|error| (owner.clone(), error))?;
        classpath
            .register_inline_source_output(
                &crate::ClassFile {
                    internal_name: owner.clone(),
                    bytes,
                },
                function,
            )
            .map_err(|mut error| {
                error.offset = function.offset;
                (owner.clone(), error)
            })?;
    }
    Ok(())
}

fn validate_parameter_uses(
    module: &mut check::Module,
    source: &syntax::Function,
    owner: &str,
) -> Result<()> {
    let mut parameters = HashSet::new();
    let mut slot = 0u16;
    for ((_, ty), mode) in source.params.iter().zip(&source.parameter_modes) {
        if matches!(
            ty,
            syntax::Type::Function { .. } | syntax::Type::ReceiverFunction { .. }
        ) && *mode != syntax::InlineParameterMode::Noinline
        {
            parameters.insert(slot as u8);
        }
        slot += ty.slots() as u16;
    }
    let mut all = HashMap::<u8, usize>::new();
    let mut allowed = HashMap::<u8, usize>::new();
    let mut generated = false;
    let mut generic_array = false;
    fn parameterized_array(ty: &syntax::Type) -> bool {
        match ty {
            syntax::Type::Array(inner) | syntax::Type::Nullable(inner) => {
                parameterized_array(inner)
            }
            syntax::Type::Parameter { .. } => true,
            _ => false,
        }
    }
    let mut visitor = |value: &mut check::Value, _: &str| match &value.kind {
        check::ValueKind::NewArray(array, _) | check::ValueKind::ArrayInitialize { array, .. }
            if parameterized_array(array) =>
        {
            generic_array = true;
        }
        check::ValueKind::Local(slot) if parameters.contains(slot) => {
            *all.entry(*slot).or_default() += 1;
        }
        check::ValueKind::FunctionInvoke { function, .. } => {
            if let check::ValueKind::Local(slot) = function.kind
                && parameters.contains(&slot)
            {
                *allowed.entry(slot).or_default() += 1;
            }
        }
        check::ValueKind::CompiledInline { method, args, .. } => {
            for (index, argument) in args.iter().enumerate() {
                if let check::ValueKind::Local(slot) = argument.kind
                    && parameters.contains(&slot)
                    && method
                        .library
                        .as_ref()
                        .and_then(|library| library.parameter_modes.get(index))
                        .is_none_or(|mode| *mode != syntax::InlineParameterMode::Noinline)
                {
                    *allowed.entry(slot).or_default() += 1;
                }
            }
        }
        check::ValueKind::Let { value, .. } => {
            generated |=
                matches!(value.kind, check::ValueKind::Local(slot) if parameters.contains(&slot));
        }
        check::ValueKind::Lambda { captures, .. } => {
            generated |= captures.iter().any(|value| matches!(value.kind, check::ValueKind::Local(slot) if parameters.contains(&slot)));
        }
        _ => {}
    };
    if let Some(function) = module
        .functions
        .iter_mut()
        .find(|function| function.name == source.name)
    {
        check::ir_walk::statements(&mut function.body, owner, &mut visitor);
    }
    if generic_array {
        return Err(unsupported(
            source.offset,
            "Array construction with type-parameter components requires unsupported array reification",
        ));
    }
    if all
        .iter()
        .any(|(slot, count)| allowed.get(slot).copied().unwrap_or(0) != *count)
    {
        return Err(if generated {
            unsupported(
                source.offset,
                "Inline parameters in generated closures or reordered forwarding are unsupported",
            )
        } else {
            Diagnostic::new(source.offset, "Inline function parameters may only be invoked directly or forwarded to an inline parameter").with_kind(DiagnosticKind::InvalidKotlin, "INLINE_PARAMETER_ESCAPES")
        });
    }
    Ok(())
}
