use super::*;

fn emitter<'a>(pool: &'a mut Pool, owner: &'a str) -> Code<'a> {
    Code {
        coroutine: false,
        pool,
        functions: &[],
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
        lambdas: &[],
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        in_lambda: false,
        max_stack: 16,
    }
}

fn text(pool: &Pool, index: u16) -> Result<String> {
    match &pool.entries[usize::from(index) - 1] {
        Constant::Utf8(bytes) => {
            String::from_utf8(bytes.clone()).map_err(|_| failure("Unsupported JVM member encoding"))
        }
        _ => Err(failure("Invalid JVM member string")),
    }
}

fn signature(pool: &Pool, method: &[u8]) -> Result<Option<String>> {
    let count = u16::from_be_bytes(method[6..8].try_into().unwrap());
    let mut cursor = 8;
    for _ in 0..count {
        let name = u16::from_be_bytes(method[cursor..cursor + 2].try_into().unwrap());
        let length =
            u32::from_be_bytes(method[cursor + 2..cursor + 6].try_into().unwrap()) as usize;
        cursor += 6;
        if text(pool, name)? == "Signature" {
            return text(
                pool,
                u16::from_be_bytes(method[cursor..cursor + 2].try_into().unwrap()),
            )
            .map(Some);
        }
        cursor += length;
    }
    Ok(None)
}

fn generic_prefix(signature: &str) -> &str {
    if !signature.starts_with('<') {
        return "";
    }
    let mut depth = 0;
    for (index, byte) in signature.bytes().enumerate() {
        if byte == b'<' {
            depth += 1;
        } else if byte == b'>' {
            depth -= 1;
            if depth == 0 {
                return &signature[..index + 1];
            }
        }
    }
    ""
}

pub(super) fn member_name(pool: &Pool, member: &[u8]) -> Result<String> {
    text(pool, u16::from_be_bytes(member[2..4].try_into().unwrap()))
}

pub(super) fn bridges(
    pool: &mut Pool,
    owner: &str,
    methods: &mut Vec<Vec<u8>>,
    class_member: bool,
    signatures: &HashMap<(String, String), (Vec<Type>, Type)>,
) -> Result<()> {
    let value_info = pool.value_classes.get(owner).cloned();
    let mut additions = Vec::new();
    for method in methods.iter_mut() {
        let flags = u16::from_be_bytes(method[..2].try_into().unwrap());
        let name = member_name(pool, method)?;
        if name.starts_with('<')
            || flags & (2 | 0x400) != 0
            || flags & 0x1000 != 0 && name != "copy$default"
        {
            continue;
        }
        let descriptor = text(pool, u16::from_be_bytes(method[4..6].try_into().unwrap()))?;
        let (parameters, result) = signatures
            .get(&(name.clone(), descriptor.clone()))
            .cloned()
            .map_or_else(|| invocation_types(&descriptor), Ok)?;
        let source_parameters = parameters.clone();
        let own_value = value_info
            .as_ref()
            .filter(|info| name != crate::check::getter_name(&info.property));
        if own_value.is_none()
            && !parameters
                .iter()
                .any(|ty| crate::value_abi::value_parameter(ty, &pool.value_classes))
            && !crate::value_abi::value_parameter(&result, &pool.value_classes)
        {
            continue;
        }
        let static_ = flags & 8 != 0;
        let mut actual_parameters = parameters
            .iter()
            .map(|ty| crate::value_abi::physical(ty, &pool.value_classes))
            .collect::<Vec<_>>();
        let actual_result = crate::value_abi::physical(&result, &pool.value_classes);
        let (logical_mangle_params, default_suffix) = if flags & 0x1000 != 0
            && name.ends_with("$default")
        {
            let start = usize::from(
                class_member && !parameters.is_empty() && parameters[0] == Type::Java(owner.into()),
            );
            (
                &source_parameters[start..source_parameters.len().saturating_sub(2)],
                true,
            )
        } else {
            (&source_parameters[..], false)
        };
        let base = name.strip_suffix("$default").unwrap_or(&name);
        let mut actual_name = crate::value_abi::mangled_name(
            base,
            logical_mangle_params,
            class_member.then_some(&result),
            |name| pool.value_classes.get(name).cloned(),
            own_value.is_some(),
        );
        if default_suffix {
            actual_name.push_str("$default");
        }
        if let Some(info) = own_value {
            actual_parameters.insert(0, info.carrier.clone());
        }
        let actual_descriptor = format!(
            "({}){}",
            actual_parameters
                .iter()
                .map(Type::descriptor)
                .collect::<String>(),
            actual_result.descriptor()
        );
        if actual_descriptor == descriptor && actual_name == name {
            continue;
        }
        let bridge_static = static_ || own_value.is_some();
        let start = usize::from(!bridge_static);
        let physical_params = actual_parameters
            .iter()
            .enumerate()
            .map(|(i, ty)| (format!("p{i}"), ty.clone()))
            .collect::<Vec<_>>();
        let slots = parameter_slots(&physical_params, start);
        let locals = start + actual_parameters.iter().map(Type::slots).sum::<usize>();
        let generic = signature(pool, method)?.map(|value| {
            format!(
                "{}{}",
                generic_prefix(&value),
                generic_method(&physical_params, &actual_result)
                    .unwrap_or_else(|| actual_descriptor.clone())
            )
        });
        let mut code = emitter(pool, owner);
        let shift = usize::from(own_value.is_some());
        if let Some(info) = own_value {
            code.bytes.extend([load_opcode(&info.carrier), 0]);
            code.method_reference(
                0xb8,
                owner,
                "box-impl",
                &format!("({})L{owner};", info.carrier.descriptor()),
            )?;
        } else if !static_ {
            code.bytes.push(0x2a);
        }
        for (index, logical) in parameters.iter().enumerate() {
            let physical = &actual_parameters[index + shift];
            code.bytes
                .extend([load_opcode(physical), slots[index + shift] as u8]);
            code.adapt_value(physical, logical)?;
        }
        let private_implementation = own_value.is_none()
            || flags & 0x10 != 0 && !matches!(name.as_str(), "toString" | "hashCode" | "equals");
        code.method_reference(
            if static_ {
                0xb8
            } else if private_implementation {
                0xb7
            } else {
                0xb6
            },
            owner,
            &name,
            &descriptor,
        )?;
        code.adapt_value(&result, &actual_result)?;
        code.bytes.push(return_opcode(&actual_result));
        let body = code.into_simple(locals as u16, 32)?;
        let bridge_flags = (flags & !8) | if bridge_static { 8 } else { 0 };
        let mut bridge = simple_method(
            pool,
            bridge_flags,
            &actual_name,
            &actual_descriptor,
            body,
            generic.as_deref(),
            nonnull(&actual_result),
        )?;
        frames::attach(
            pool,
            &mut bridge,
            owner,
            &physical_params,
            locals as u16,
            bridge_static,
        )?;
        additions.push(bridge);
        if private_implementation {
            method[..2].copy_from_slice(&((flags & !5) | 0x1002).to_be_bytes());
        }
    }
    methods.extend(additions);
    Ok(())
}

pub(super) fn helpers(
    pool: &mut Pool,
    owner: &str,
    info: &crate::value_abi::ValueClassInfo,
) -> Result<Vec<Vec<u8>>> {
    let carrier = &info.carrier;
    let physical = carrier.descriptor();
    let mut methods = Vec::new();
    let mut body = SimpleCode::new(3, (carrier.slots() + 1) as u16);
    body.bytes.push(0x2a);
    body.reference(0xb7, pool.method("java/lang/Object", "<init>", "()V")?);
    body.bytes.extend([0x2a, load_opcode(carrier), 1]);
    body.reference(0xb5, pool.field(owner, &info.property, &physical)?);
    body.bytes.push(0xb1);
    methods.push(simple_method(
        pool,
        0x1002,
        "<init>",
        &format!("({physical})V"),
        body,
        None,
        false,
    )?);
    let mut body = SimpleCode::new(5, carrier.slots() as u16);
    body.reference(0xbb, pool.class(owner)?);
    body.bytes.extend([0x59, load_opcode(carrier), 0, 0x03]);
    body.reference(
        0xb7,
        pool.method(owner, "<init>", &format!("({physical}Z)V"))?,
    );
    body.reference(
        0xb6,
        pool.method(owner, "unbox-impl", &format!("(){physical}"))?,
    );
    body.bytes.push(return_opcode(carrier));
    methods.push(simple_method(
        pool,
        9,
        "constructor-impl",
        &format!("({physical}){physical}"),
        body,
        None,
        false,
    )?);
    let mut body = SimpleCode::new(4, carrier.slots() as u16);
    body.reference(0xbb, pool.class(owner)?);
    body.bytes.extend([0x59, load_opcode(carrier), 0]);
    body.reference(
        0xb7,
        pool.method(owner, "<init>", &format!("({physical})V"))?,
    );
    body.bytes.push(0xb0);
    methods.push(simple_method(
        pool,
        0x1019,
        "box-impl",
        &format!("({physical})L{owner};"),
        body,
        None,
        true,
    )?);
    let mut body = SimpleCode::new(carrier.slots() as u16, 1);
    body.bytes.push(0x2a);
    body.reference(0xb4, pool.field(owner, &info.property, &physical)?);
    body.bytes.push(return_opcode(carrier));
    methods.push(simple_method(
        pool,
        0x1011,
        "unbox-impl",
        &format!("(){physical}"),
        body,
        None,
        nonnull(carrier),
    )?);
    let mut body = SimpleCode::new(4, (carrier.slots() * 2) as u16);
    for slot in [0, carrier.slots() as u8] {
        body.bytes.extend([load_opcode(carrier), slot]);
        body.reference(
            0xb8,
            pool.method(owner, "box-impl", &format!("({physical})L{owner};"))?,
        );
    }
    body.reference(
        0xb8,
        pool.method(
            "java/util/Objects",
            "equals",
            "(Ljava/lang/Object;Ljava/lang/Object;)Z",
        )?,
    );
    body.bytes.push(0xac);
    methods.push(simple_method(
        pool,
        0x19,
        "equals-impl0",
        &format!("({physical}{physical})Z"),
        body,
        None,
        false,
    )?);
    Ok(methods)
}

pub(super) fn consumer_constructor(
    pool: &mut Pool,
    owner: &str,
    class: &crate::check::ClassDecl,
) -> Result<Vec<Vec<u8>>> {
    if class.value_
        || !class
            .params
            .iter()
            .any(|(_, ty)| crate::value_abi::value_parameter(ty, &pool.value_classes))
    {
        return Ok(Vec::new());
    }
    if class.inner
        || class.super_class == "java/lang/Enum"
        || class.defaults.iter().any(Option::is_some)
        || !class.secondary_constructors.is_empty()
    {
        return Err(Diagnostic::new(
            0,
            "Value class consumer inner, enum, defaulted and secondary constructors are unsupported",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "VALUE_CONSUMER_CONSTRUCTOR_SHAPE_UNSUPPORTED",
        ));
    }
    let physical = class
        .params
        .iter()
        .map(|(name, ty)| {
            (
                name.clone(),
                crate::value_abi::physical(ty, &pool.value_classes),
            )
        })
        .collect::<Vec<_>>();
    let physical_descriptor = format!(
        "({})V",
        physical
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let boxed_descriptor = format!(
        "({})V",
        class
            .params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let slots = parameter_slots(&physical, 1);
    let locals = 1 + physical.iter().map(|(_, ty)| ty.slots()).sum::<usize>();
    let mut code = emitter(pool, owner);
    code.bytes.push(0x2a);
    for ((slot, (_, ty)), (_, logical)) in slots.iter().zip(&physical).zip(&class.params) {
        code.bytes.extend([load_opcode(ty), *slot as u8]);
        code.adapt_value(ty, logical)?;
    }
    code.method_reference(0xb7, owner, "<init>", &boxed_descriptor)?;
    code.bytes.push(0xb1);
    let body = code.into_simple(locals as u16, 32)?;
    let mut constructors = Vec::new();
    if physical_descriptor != boxed_descriptor {
        let mut constructor = simple_method(
            pool,
            2,
            "<init>",
            &physical_descriptor,
            body,
            generic_method(&physical, &Type::Unit).as_deref(),
            false,
        )?;
        frames::attach_constructor(pool, &mut constructor, owner, &physical, locals as u16)?;
        constructors.push(constructor);
    }
    let mut code = SimpleCode::new(32, (locals + 1) as u16);
    code.bytes.push(0x2a);
    for (slot, (_, ty)) in slots.into_iter().zip(&physical) {
        code.bytes.extend([load_opcode(ty), slot as u8]);
    }
    code.reference(0xb7, pool.method(owner, "<init>", &physical_descriptor)?);
    code.bytes.push(0xb1);
    let marker =
        physical_descriptor.replace(")V", "Lkotlin/jvm/internal/DefaultConstructorMarker;)V");
    let marker = simple_method(pool, 0x1001, "<init>", &marker, code, None, false)?;
    constructors.push(marker);
    Ok(constructors)
}

pub(super) fn storage_bridges(
    pool: &mut Pool,
    owner: &str,
    name: &str,
    ty: &Type,
    is_static: bool,
    mutable: bool,
) -> Result<Vec<Vec<u8>>> {
    let mut methods = Vec::new();
    for setter in [false, true] {
        if setter && !mutable {
            continue;
        }
        let mut parameters = Vec::new();
        if !is_static {
            parameters.push(("receiver".into(), Type::Java(owner.into())));
        }
        if setter {
            parameters.push(("value".into(), ty.clone()));
        }
        let result = if setter { Type::Unit } else { ty.clone() };
        let descriptor = format!(
            "({}){}",
            parameters
                .iter()
                .map(|(_, ty)| ty.descriptor())
                .collect::<String>(),
            result.descriptor()
        );
        let locals = parameters.iter().map(|(_, ty)| ty.slots()).sum::<usize>() as u16;
        let mut code = emitter(pool, owner);
        if !is_static {
            code.bytes.push(0x2a);
        }
        if setter {
            code.bytes.extend([load_opcode(ty), u8::from(!is_static)]);
        }
        code.storage_field(
            match (is_static, setter) {
                (false, false) => 0xb4,
                (false, true) => 0xb5,
                (true, false) => 0xb2,
                (true, true) => 0xb3,
            },
            owner,
            name,
            ty,
        )?;
        code.bytes.push(return_opcode(&result));
        let stack = code.max_stack as u16;
        let body = code.into_simple(locals, stack)?;
        let method_name = format!("access${}${name}$p", if setter { "set" } else { "get" });
        let mut method = simple_method(pool, 0x1008, &method_name, &descriptor, body, None, false)?;
        frames::attach(pool, &mut method, owner, &parameters, locals, true)?;
        methods.push(method);
    }
    Ok(methods)
}
