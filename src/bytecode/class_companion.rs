use super::*;
use crate::check::ClassCompanion;

pub(super) fn inner_attribute(pool: &mut Pool, owner: &str, companion: &str) -> Result<Vec<u8>> {
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("InnerClasses")?);
    u4(&mut attribute, 10);
    u2(&mut attribute, 1);
    u2(&mut attribute, pool.class(companion)?);
    u2(&mut attribute, pool.class(owner)?);
    u2(&mut attribute, pool.utf8("Companion")?);
    u2(&mut attribute, 0x0019);
    Ok(attribute)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn outer(
    pool: &mut Pool,
    companion: &ClassCompanion,
    owner: &str,
    fields: &mut Vec<u8>,
    methods: &mut Vec<Vec<u8>>,
    lambdas: &[crate::check::LambdaDecl],
    delegated_properties: &[(String, Value)],
    enum_data: Option<&crate::check::enums::CheckedEnum>,
) -> Result<()> {
    fields.extend(class_field(
        pool,
        "Companion",
        &Type::Java(companion.owner.clone()),
        0x0019,
    )?);
    for property in &companion.properties {
        fields.extend(class_field(
            pool,
            &property.name,
            &property.ty,
            0x0008
                | companion
                    .exposed_fields
                    .get(&property.name)
                    .copied()
                    .unwrap_or(0x0002)
                | if property.mutable { 0 } else { 0x0010 },
        )?);
        if !lambdas.is_empty() {
            methods.extend(field_bridges(
                pool,
                owner,
                &property.name,
                &property.ty,
                true,
                property.mutable,
            )?);
        }
        let mut code = SimpleCode::new(property.ty.slots() as u16, 0);
        code.reference(
            0xb2,
            pool.field(owner, &property.name, &property.ty.descriptor())?,
        );
        code.bytes.push(return_opcode(&property.ty));
        methods.push(simple_method(
            pool,
            0x1019,
            &format!("access${}$cp", crate::check::getter_name(&property.name)),
            &format!("(){}", property.ty.descriptor()),
            code,
            None,
            false,
        )?);
        if property.mutable {
            let mut code = SimpleCode::new(property.ty.slots() as u16, property.ty.slots() as u16);
            code.bytes.extend([load_opcode(&property.ty), 0]);
            code.reference(
                0xb3,
                pool.field(owner, &property.name, &property.ty.descriptor())?,
            );
            code.bytes.push(0xb1);
            methods.push(simple_method(
                pool,
                0x1019,
                &format!("access${}$cp", crate::check::setter_name(&property.name)),
                &format!("({})V", property.ty.descriptor()),
                code,
                None,
                false,
            )?);
        }
    }
    for function in &companion.methods {
        if !defaults::annotated(function, "JvmStatic") {
            continue;
        }
        if function.private {
            return Err(failure(
                "Private companion JvmStatic methods are unsupported",
            ));
        }
        let mut omitted = HashSet::new();
        let mut variants = vec![function.params.clone()];
        if defaults::annotated(function, "JvmOverloads") {
            for index in (0..function.params.len()).rev() {
                if function.defaults.get(index).is_some_and(Option::is_some) {
                    omitted.insert(index);
                    variants.push(
                        function
                            .params
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| !omitted.contains(index))
                            .map(|(_, parameter)| parameter.clone())
                            .collect(),
                    );
                }
            }
        }
        for params in variants {
            let slots = params.iter().map(|(_, ty)| ty.slots()).sum::<usize>();
            let mut code = SimpleCode::new(
                (slots + 1).max(function.result.slots()) as u16,
                slots as u16,
            );
            code.reference(
                0xb2,
                pool.field(owner, "Companion", &format!("L{};", companion.owner))?,
            );
            for (slot, (_, ty)) in parameter_slots(&params, 0).into_iter().zip(&params) {
                code.bytes.extend([load_opcode(ty), slot as u8]);
            }
            let reduced = crate::check::Function {
                class_type_params: Vec::new(),
                jvm_result: None,
                jvm_name: function.jvm_name.clone(),
                annotations: function.annotations.clone(),
                vararg: function.vararg && params.last() == function.params.last(),
                type_params: function.type_params.clone(),
                defaults: Vec::new(),
                name: function.name.clone(),
                private: false,
                is_override: false,
                params,
                result: function.result.clone(),
                locals: 0,
                body: Vec::new(),
            };
            code.reference(
                0xb6,
                pool.method(&companion.owner, reduced.jvm_name(), &reduced.descriptor())?,
            );
            code.bytes.push(return_opcode(&function.result));
            let mut method = simple_method(
                pool,
                0x0019 | if reduced.vararg { 0x80 } else { 0 },
                reduced.jvm_name(),
                &reduced.descriptor(),
                code,
                generic_function(&reduced).as_deref(),
                nonnull(&reduced.result),
            )?;
            parameter_annotations(pool, &mut method, &reduced.params)?;
            annotations::append_annotations(pool, &mut method, &reduced.annotations)?;
            nullable_result(pool, &mut method, &reduced.result)?;
            methods.push(method);
        }
    }
    let mut code = Code {
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
        lambdas,
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        in_lambda: false,
        max_stack: 3,
    };
    if let Some(enumeration) = enum_data {
        enums::initialize(&mut code, owner, enumeration)?;
    }
    for (name, value) in delegated_properties {
        code.value(value)?;
        code.field_reference(0xb3, owner, name, &value.ty.descriptor())?;
    }
    code.class_reference(0xbb, &companion.owner)?;
    code.bytes.extend([0x59, 0x01]);
    code.method_reference(
        0xb7,
        &companion.owner,
        "<init>",
        "(Lkotlin/jvm/internal/DefaultConstructorMarker;)V",
    )?;
    code.field_reference(0xb3, owner, "Companion", &format!("L{};", companion.owner))?;
    if companion.initializers.is_empty() {
        for property in &companion.properties {
            code.value(&property.value)?;
            code.field_reference(0xb3, owner, &property.name, &property.ty.descriptor())?;
        }
    } else {
        code.block(&companion.initializers)?;
    }
    code.bytes.push(0xb1);
    let max_stack = code.max_stack as u16;
    let locals = companion
        .locals
        .max(enum_data.map_or(0, |enumeration| enumeration.initializer_locals));
    let code = code.into_simple(locals, max_stack)?;
    let mut initializer = simple_method(pool, 0x0008, "<clinit>", "()V", code, None, false)?;
    frames::attach(pool, &mut initializer, owner, &[], locals, true)?;
    methods.push(initializer);
    Ok(())
}

pub(super) fn emit(
    companion: &ClassCompanion,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<crate::ClassFile> {
    let mut pool = Pool::default();
    let this = pool.class(&companion.owner)?;
    let parent = pool.class("java/lang/Object")?;
    let mut methods = Vec::new();
    let mut code = SimpleCode::new(1, 1);
    code.bytes.push(0x2a);
    code.reference(0xb7, pool.method("java/lang/Object", "<init>", "()V")?);
    code.bytes.push(0xb1);
    methods.push(simple_method(
        &mut pool, 0x0002, "<init>", "()V", code, None, false,
    )?);
    for property in &companion.properties {
        if property.private {
            continue;
        }
        let has_getter = companion.methods.iter().any(|method| {
            method.name == crate::check::getter_name(&property.name) && method.params.is_empty()
        });
        if !has_getter {
            let mut code = SimpleCode::new(property.ty.slots() as u16, 1);
            code.reference(
                0xb8,
                pool.method(
                    owner,
                    &format!("access${}$cp", crate::check::getter_name(&property.name)),
                    &format!("(){}", property.ty.descriptor()),
                )?,
            );
            code.bytes.push(return_opcode(&property.ty));
            let mut getter = simple_method(
                &mut pool,
                0x0011,
                &crate::check::getter_name(&property.name),
                &format!("(){}", property.ty.descriptor()),
                code,
                generic_method(&[], &property.ty).as_deref(),
                nonnull(&property.ty),
            )?;
            nullable_result(&mut pool, &mut getter, &property.ty)?;
            methods.push(getter);
        }
        if property.mutable
            && !companion.methods.iter().any(|method| {
                method.name == crate::check::setter_name(&property.name) && method.params.len() == 1
            })
        {
            let mut code = SimpleCode::new(
                property.ty.slots().max(2) as u16,
                (property.ty.slots() + 1) as u16,
            );
            if nonnull(&property.ty) {
                code.bytes.extend([0x19, 1]);
                code.reference(0x13, pool.string("<set-?>")?);
                code.reference(
                    0xb8,
                    pool.method(
                        "kotlin/jvm/internal/Intrinsics",
                        "checkNotNullParameter",
                        "(Ljava/lang/Object;Ljava/lang/String;)V",
                    )?,
                );
            }
            code.bytes.extend([load_opcode(&property.ty), 1]);
            code.reference(
                0xb8,
                pool.method(
                    owner,
                    &format!("access${}$cp", crate::check::setter_name(&property.name)),
                    &format!("({})V", property.ty.descriptor()),
                )?,
            );
            code.bytes.push(0xb1);
            let params = vec![("<set-?>".into(), property.ty.clone())];
            let mut setter = simple_method(
                &mut pool,
                0x0011,
                &crate::check::setter_name(&property.name),
                &format!("({})V", property.ty.descriptor()),
                code,
                generic_method(&params, &Type::Unit).as_deref(),
                false,
            )?;
            parameter_annotations(&mut pool, &mut setter, &params)?;
            methods.push(setter);
        }
    }
    for function in &companion.methods {
        methods.extend(defaults::overloads(
            &mut pool,
            function,
            &companion.owner,
            false,
            if function.private { 0x0012 } else { 0x0011 },
        )?);
        if let Some(default) = defaults::function(
            &mut pool,
            &companion.methods,
            function,
            &companion.owner,
            false,
            lambdas,
        )? {
            methods.push(default);
        }
        if !lambdas.is_empty() && function.private {
            methods.push(private_method_bridge(
                &mut pool,
                &companion.owner,
                function,
                false,
            )?);
        }
        let mut code = Code {
            pool: &mut pool,
            functions: &companion.methods,
            owner: &companion.owner,
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
        if !function.private {
            for (slot, (name, ty)) in parameter_slots(&function.params, 1)
                .into_iter()
                .zip(&function.params)
            {
                if nonnull(ty) {
                    code.bytes.extend([0x19, slot as u8]);
                    let name = code.pool.string(name)?;
                    code.reference(0x13, name);
                    code.method_reference(
                        0xb8,
                        "kotlin/jvm/internal/Intrinsics",
                        "checkNotNullParameter",
                        "(Ljava/lang/Object;Ljava/lang/String;)V",
                    )?;
                }
            }
        }
        code.block(&function.body)?;
        let stack = code.max_stack as u16;
        let code = code.into_simple(function.locals, stack)?;
        let mut method = simple_method(
            &mut pool,
            if function.private { 0x0012 } else { 0x0011 },
            function.jvm_name(),
            &function.descriptor(),
            code,
            generic_function(function).as_deref(),
            nonnull(&function.result),
        )?;
        parameter_annotations(&mut pool, &mut method, &function.params)?;
        annotations::append_annotations(&mut pool, &mut method, &function.annotations)?;
        nullable_result(&mut pool, &mut method, &function.result)?;
        frames::attach(
            &mut pool,
            &mut method,
            &companion.owner,
            &function.params,
            function.locals,
            false,
        )?;
        methods.push(method);
    }
    let mut code = SimpleCode::new(1, 2);
    code.bytes.push(0x2a);
    code.reference(0xb7, pool.method(&companion.owner, "<init>", "()V")?);
    code.bytes.push(0xb1);
    methods.push(simple_method(
        &mut pool,
        0x1001,
        "<init>",
        "(Lkotlin/jvm/internal/DefaultConstructorMarker;)V",
        code,
        None,
        false,
    )?);
    let inner = inner_attribute(&mut pool, owner, &companion.owner)?;
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let metadata = metadata_attribute(&mut pool, metadata)?;
    let mut bytes = Vec::new();
    u4(&mut bytes, 0xcafebabe);
    u2(&mut bytes, 0);
    u2(&mut bytes, 52);
    defaults::unique_methods(&pool, &methods)?;
    pool.write(&mut bytes);
    u2(&mut bytes, 0x0031);
    u2(&mut bytes, this);
    u2(&mut bytes, parent);
    u2(&mut bytes, 0);
    u2(&mut bytes, 0);
    u2(&mut bytes, methods.len() as u16);
    for method in methods {
        bytes.extend(method);
    }
    u2(&mut bytes, 3);
    bytes.extend(inner);
    u2(&mut bytes, source_name);
    u4(&mut bytes, 2);
    u2(&mut bytes, source_file);
    bytes.extend(metadata);
    Ok(crate::ClassFile {
        internal_name: companion.owner.clone(),
        bytes,
    })
}
