use super::*;

pub(super) fn function(
    pool: &mut Pool,
    functions: &[crate::check::Function],
    function: &crate::check::Function,
    owner: &str,
    is_static: bool,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<Option<Vec<u8>>> {
    if !function.defaults.iter().any(Option::is_some) {
        return Ok(None);
    }
    let start = usize::from(!is_static);
    let slots: usize = start
        + function
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>();
    let locals = function.locals.max((slots + 2) as u16);
    let mut body = Code {
        pool,
        functions,
        owner,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        loop_finally_depths: Vec::new(),
        loop_names: Vec::new(),
        pending_loop_name: None,
        finalizers: Vec::new(),
        exceptions: Vec::new(),
        temporary_count: 0,
        temporary_patches: Vec::new(),
        lambdas,
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        in_lambda: false,
        max_stack: 2,
    };
    for (index, (slot, (_, ty))) in parameter_slots(&function.params, start)
        .into_iter()
        .zip(&function.params)
        .enumerate()
    {
        if let Some(value) = function.defaults.get(index).and_then(Option::as_ref) {
            let next = body.label();
            body.bytes.extend([0x15, slots as u8]);
            body.integer((1u32 << index) as i32)?;
            body.bytes.push(0x7e);
            body.branch(0x99, next);
            body.value(value)?;
            body.bytes.extend([store_opcode(ty), slot as u8]);
            body.mark(next);
        }
    }
    if !is_static {
        body.bytes.push(0x2a);
    }
    for (slot, (_, ty)) in parameter_slots(&function.params, start)
        .into_iter()
        .zip(&function.params)
    {
        body.bytes.extend([load_opcode(ty), slot as u8]);
    }
    body.max_stack = body.max_stack.max(slots.max(function.result.slots()));
    body.method_reference(
        if is_static {
            0xb8
        } else if function.private {
            0xb7
        } else {
            0xb6
        },
        owner,
        function.jvm_name(),
        &function.descriptor(),
    )?;
    body.bytes.push(return_opcode(&function.result));
    let max_stack = body.max_stack as u16;
    let code = body.into_simple(locals, max_stack)?;
    let mut params = Vec::new();
    if !is_static {
        params.push(("$this".into(), Type::Java(owner.into())));
    }
    params.extend(function.params.clone());
    params.push(("$mask".into(), Type::Int));
    params.push((
        "$marker".into(),
        Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
    ));
    let descriptor = format!(
        "({}){}",
        params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>(),
        function.result.descriptor()
    );
    let mut method = simple_method(
        pool,
        if function.private { 0x1008 } else { 0x1009 },
        &format!("{}$default", function.jvm_name()),
        &descriptor,
        code,
        None,
        false,
    )?;
    frames::attach(pool, &mut method, owner, &params, locals, true)?;
    Ok(Some(method))
}

pub(super) fn constructors(
    pool: &mut Pool,
    class: &crate::check::ClassDecl,
    owner: &str,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<Vec<Vec<u8>>> {
    if !class.defaults.iter().any(Option::is_some) {
        return Ok(Vec::new());
    }
    let slots = 1 + class.params.iter().map(|(_, ty)| ty.slots()).sum::<usize>();
    let locals = class.constructor_locals.max((slots + 2) as u16);
    let mut body = Code {
        pool,
        functions: &class.methods,
        owner,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        loop_finally_depths: Vec::new(),
        loop_names: Vec::new(),
        pending_loop_name: None,
        finalizers: Vec::new(),
        exceptions: Vec::new(),
        temporary_count: 0,
        temporary_patches: Vec::new(),
        lambdas,
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        in_lambda: false,
        max_stack: 2,
    };
    for (index, (slot, (_, ty))) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
        .enumerate()
    {
        if let Some(value) = class.defaults.get(index).and_then(Option::as_ref) {
            let next = body.label();
            body.bytes.extend([0x15, slots as u8]);
            body.integer((1u32 << index) as i32)?;
            body.bytes.push(0x7e);
            body.branch(0x99, next);
            body.value(value)?;
            body.bytes.extend([store_opcode(ty), slot as u8]);
            body.mark(next);
        }
    }
    body.bytes.push(0x2a);
    for (slot, (_, ty)) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
    {
        body.bytes.extend([load_opcode(ty), slot as u8]);
    }
    let descriptor = format!(
        "({})V",
        class
            .params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    body.method_reference(0xb7, owner, "<init>", &descriptor)?;
    body.bytes.push(0xb1);
    let max_stack = body.max_stack.max(slots) as u16;
    let code = body.into_simple(locals, max_stack)?;
    let mut params = class.params.clone();
    params.push(("$mask".into(), Type::Int));
    params.push((
        "$marker".into(),
        Type::Nullable(Box::new(Type::Java(
            "kotlin/jvm/internal/DefaultConstructorMarker".into(),
        ))),
    ));
    let descriptor = format!(
        "({})V",
        params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let mut method = simple_method(pool, 0x1001, "<init>", &descriptor, code, None, false)?;
    frames::attach_constructor(pool, &mut method, owner, &params, locals)?;
    let mut methods = vec![method];
    if class.defaults.len() == class.params.len() && class.defaults.iter().all(Option::is_some) {
        let mut body = Code {
            pool,
            functions: &[],
            owner,
            bytes: vec![0x2a],
            labels: Vec::new(),
            branches: Vec::new(),
            loops: Vec::new(),
            loop_finally_depths: Vec::new(),
            loop_names: Vec::new(),
            pending_loop_name: None,
            finalizers: Vec::new(),
            exceptions: Vec::new(),
            temporary_count: 0,
            temporary_patches: Vec::new(),
            lambdas,
            inline_local_maps: Vec::new(),
            inline_returns: Vec::new(),
            in_lambda: false,
            max_stack: slots + 2,
        };
        for (_, ty) in &class.params {
            body.bytes.push(if ty.is_reference() {
                0x01
            } else if ty == &Type::Long {
                0x09
            } else if ty == &Type::Double {
                0x0e
            } else if ty == &Type::Float {
                0x0b
            } else {
                0x03
            });
        }
        body.integer(if class.params.len() == 32 {
            -1
        } else {
            ((1u32 << class.params.len()) - 1) as i32
        })?;
        body.bytes.push(0x01);
        body.method_reference(0xb7, owner, "<init>", &descriptor)?;
        body.bytes.push(0xb1);
        let code = body.into_simple(1, (slots + 2) as u16)?;
        let mut method = simple_method(pool, 0x0001, "<init>", "()V", code, None, false)?;
        frames::attach_constructor(pool, &mut method, owner, &[], 1)?;
        methods.push(method);
    }
    Ok(methods)
}

pub(super) fn annotated(function: &crate::check::Function, name: &str) -> bool {
    function
        .annotations
        .iter()
        .any(|annotation| annotation.descriptor == format!("Lkotlin/jvm/{name};"))
}

pub(super) fn overloads(
    pool: &mut Pool,
    function: &crate::check::Function,
    owner: &str,
    is_static: bool,
    access: u16,
) -> Result<Vec<Vec<u8>>> {
    if !annotated(function, "JvmOverloads") {
        return Ok(Vec::new());
    }
    let mut omitted = HashSet::new();
    let mut methods = Vec::new();
    for index in (0..function.params.len()).rev() {
        if !function.defaults.get(index).is_some_and(Option::is_some) {
            continue;
        }
        omitted.insert(index);
        let reduced = crate::check::Function {
            jvm_name: function.jvm_name.clone(),
            annotations: function.annotations.clone(),
            vararg: function.vararg && !omitted.contains(&(function.params.len() - 1)),
            type_params: function.type_params.clone(),
            defaults: Vec::new(),
            name: function.name.clone(),
            private: function.private,
            is_override: false,
            params: function
                .params
                .iter()
                .enumerate()
                .filter(|(index, _)| !omitted.contains(index))
                .map(|(_, parameter)| parameter.clone())
                .collect(),
            result: function.result.clone(),
            locals: 0,
            body: Vec::new(),
        };
        let start = usize::from(!is_static);
        let locals = start
            + reduced
                .params
                .iter()
                .map(|(_, ty)| ty.slots())
                .sum::<usize>();
        let stack = start
            + function
                .params
                .iter()
                .map(|(_, ty)| ty.slots())
                .sum::<usize>()
            + 2;
        let mut code = SimpleCode::new(stack.max(function.result.slots()) as u16, locals as u16);
        if !function.private {
            for (slot, (name, ty)) in parameter_slots(&reduced.params, start)
                .into_iter()
                .zip(&reduced.params)
            {
                if nonnull(ty) {
                    code.bytes.extend([0x19, slot as u8]);
                    code.reference(0x13, pool.string(name)?);
                    code.reference(
                        0xb8,
                        pool.method(
                            "kotlin/jvm/internal/Intrinsics",
                            "checkNotNullParameter",
                            "(Ljava/lang/Object;Ljava/lang/String;)V",
                        )?,
                    );
                }
            }
        }
        if !is_static {
            code.bytes.push(0x2a);
        }
        let mut slot = start;
        for (index, (_, ty)) in function.params.iter().enumerate() {
            if omitted.contains(&index) {
                code.bytes.push(if ty.is_reference() {
                    0x01
                } else if ty == &Type::Long {
                    0x09
                } else if ty == &Type::Double {
                    0x0e
                } else if ty == &Type::Float {
                    0x0b
                } else {
                    0x03
                });
            } else {
                code.bytes.extend([load_opcode(ty), slot as u8]);
                slot += ty.slots();
            }
        }
        let mask = omitted
            .iter()
            .fold(0u32, |mask, index| mask | (1u32 << index));
        if mask <= 5 {
            code.bytes.push(0x03 + mask as u8);
        } else {
            code.reference(0x13, pool.add(Constant::Integer(mask as i32))?);
        }
        code.bytes.push(0x01);
        let descriptor = format!(
            "({}{}ILjava/lang/Object;){}",
            if is_static {
                String::new()
            } else {
                format!("L{owner};")
            },
            function
                .params
                .iter()
                .map(|(_, ty)| ty.descriptor())
                .collect::<String>(),
            function.result.descriptor()
        );
        code.reference(
            0xb8,
            pool.method(
                owner,
                &format!("{}$default", function.jvm_name()),
                &descriptor,
            )?,
        );
        code.bytes.push(return_opcode(&function.result));
        let mut method = simple_method(
            pool,
            access | if reduced.vararg { 0x80 } else { 0 },
            reduced.jvm_name(),
            &reduced.descriptor(),
            code,
            generic_function(&reduced).as_deref(),
            nonnull(&reduced.result),
        )?;
        parameter_annotations(pool, &mut method, &reduced.params)?;
        annotations::append_annotations(pool, &mut method, &reduced.annotations)?;
        nullable_result(pool, &mut method, &reduced.result)?;
        frames::attach(
            pool,
            &mut method,
            owner,
            &reduced.params,
            locals as u16,
            is_static,
        )?;
        methods.push(method);
    }
    Ok(methods)
}

pub(super) fn unique_methods(pool: &Pool, methods: &[Vec<u8>]) -> Result<()> {
    let mut seen = HashSet::new();
    for method in methods {
        let name = u16::from_be_bytes([method[2], method[3]]);
        let descriptor = u16::from_be_bytes([method[4], method[5]]);
        if !seen.insert((name, descriptor)) {
            let name = match &pool.entries[name as usize - 1] {
                Constant::Utf8(value) => String::from_utf8_lossy(value).into_owned(),
                _ => String::new(),
            };
            return Err(crate::Diagnostic::new(
                0,
                format!("Conflicting generated JVM method signature: {name}"),
            )
            .with_kind(crate::DiagnosticKind::InvalidKotlin, "JVM_SIGNATURE_CLASH"));
        }
    }
    Ok(())
}
