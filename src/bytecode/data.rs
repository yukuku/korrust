use super::*;

fn code<'a>(pool: &'a mut Pool, owner: &'a str) -> Code<'a> {
    Code {
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
        max_stack: 4,
    }
}
fn field(code: &mut Code<'_>, owner: &str, receiver: u8, name: &str, ty: &Type) -> Result<()> {
    code.bytes.extend([0x19, receiver]);
    code.field_reference(0xb4, owner, name, &ty.descriptor())
}
fn finish(code: Code<'_>, locals: u16, max_stack: usize) -> Result<SimpleCode> {
    code.into_simple(
        locals,
        u16::try_from(max_stack).map_err(|_| failure("Data class stack exceeds JVM limit"))?,
    )
}
fn method(
    pool: &mut Pool,
    owner: &str,
    name: &str,
    descriptor: &str,
    flags: u16,
    code: SimpleCode,
    types: (&[(String, Type)], &Type),
) -> Result<Vec<u8>> {
    let (params, result) = types;
    let locals = code.locals;
    let mut method = simple_method(
        pool,
        flags,
        name,
        descriptor,
        code,
        generic_method(params, result).as_deref(),
        nonnull(result),
    )?;
    parameter_annotations(pool, &mut method, params)?;
    nullable_result(pool, &mut method, result)?;
    frames::attach(pool, &mut method, owner, params, locals, flags & 8 != 0)?;
    Ok(method)
}
pub(super) fn methods(
    pool: &mut Pool,
    class: &crate::check::ClassDecl,
    owner: &str,
) -> Result<Vec<Vec<u8>>> {
    let mut methods = Vec::new();
    let result = Type::Java(owner.into());
    let desc = format!(
        "({})L{owner};",
        class
            .params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let slots: usize = class.params.iter().map(|(_, ty)| ty.slots()).sum();
    for (index, (name, ty)) in class.params.iter().enumerate() {
        let mut body = code(pool, owner);
        field(&mut body, owner, 0, name, ty)?;
        body.bytes.push(return_opcode(ty));
        let body = finish(body, 1, ty.slots())?;
        methods.push(method(
            pool,
            owner,
            &format!("component{}", index + 1),
            &format!("(){}", ty.descriptor()),
            0x0011,
            body,
            (&[], ty),
        )?);
    }
    let mut body = code(pool, owner);
    for (slot, (name, ty)) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
    {
        if nonnull(ty) {
            body.bytes.extend([0x19, slot as u8]);
            let string = body.pool.string(name)?;
            body.reference(0x13, string);
            body.method_reference(
                0xb8,
                "kotlin/jvm/internal/Intrinsics",
                "checkNotNullParameter",
                "(Ljava/lang/Object;Ljava/lang/String;)V",
            )?;
        }
    }
    body.class_reference(0xbb, owner)?;
    body.bytes.push(0x59);
    for (slot, (_, ty)) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
    {
        body.bytes.extend([load_opcode(ty), slot as u8]);
    }
    body.method_reference(
        0xb7,
        owner,
        "<init>",
        &format!(
            "({})V",
            class
                .params
                .iter()
                .map(|(_, ty)| ty.descriptor())
                .collect::<String>()
        ),
    )?;
    body.bytes.push(0xb0);
    let body = finish(body, (slots + 1) as u16, slots + 2)?;
    methods.push(method(
        pool,
        owner,
        "copy",
        &desc,
        0x0011,
        body,
        (&class.params, &result),
    )?);
    let mut body = code(pool, owner);
    for (index, (slot, (name, ty))) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
        .enumerate()
    {
        let next = body.label();
        body.bytes.extend([0x15, (slots + 1) as u8]);
        body.integer((1u32 << index) as i32)?;
        body.bytes.push(0x7e);
        body.branch(0x99, next);
        field(&mut body, owner, 0, name, ty)?;
        body.bytes.extend([store_opcode(ty), slot as u8]);
        body.mark(next);
    }
    body.bytes.push(0x2a);
    for (slot, (_, ty)) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
    {
        body.bytes.extend([load_opcode(ty), slot as u8]);
    }
    body.method_reference(0xb6, owner, "copy", &desc)?;
    body.bytes.push(0xb0);
    let body = finish(body, (slots + 3) as u16, (slots + 1).max(2))?;
    let mut params = vec![("$this".into(), result.clone())];
    params.extend(class.params.clone());
    params.push(("$mask".into(), Type::Int));
    params.push((
        "$marker".into(),
        Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
    ));
    let default_desc = format!(
        "(L{owner};{}ILjava/lang/Object;)L{owner};",
        class
            .params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let mut default = simple_method(
        pool,
        0x1009,
        "copy$default",
        &default_desc,
        body,
        None,
        false,
    )?;
    frames::attach(pool, &mut default, owner, &params, (slots + 3) as u16, true)?;
    methods.push(default);
    let mut body = code(pool, owner);
    body.class_reference(0xbb, "java/lang/StringBuilder")?;
    body.bytes.push(0x59);
    body.method_reference(0xb7, "java/lang/StringBuilder", "<init>", "()V")?;
    let simple_name = owner.rsplit('/').next().unwrap_or(owner);
    for (index, (name, ty)) in class.params.iter().enumerate() {
        let prefix = if index == 0 {
            format!("{simple_name}({name}=")
        } else {
            format!(", {name}=")
        };
        let prefix = body.pool.string(&prefix)?;
        body.reference(0x13, prefix);
        body.method_reference(
            0xb6,
            "java/lang/StringBuilder",
            "append",
            "(Ljava/lang/String;)Ljava/lang/StringBuilder;",
        )?;
        field(&mut body, owner, 0, name, ty)?;
        let arg = if ty.is_reference() && ty.base() != &Type::String {
            "Ljava/lang/Object;".into()
        } else if ty == &Type::Byte {
            "I".into()
        } else {
            ty.descriptor()
        };
        body.method_reference(
            0xb6,
            "java/lang/StringBuilder",
            "append",
            &format!("({arg})Ljava/lang/StringBuilder;"),
        )?;
    }
    body.integer(41)?;
    body.method_reference(
        0xb6,
        "java/lang/StringBuilder",
        "append",
        "(C)Ljava/lang/StringBuilder;",
    )?;
    body.method_reference(
        0xb6,
        "java/lang/StringBuilder",
        "toString",
        "()Ljava/lang/String;",
    )?;
    body.bytes.push(0xb0);
    let body = finish(body, 1, 3)?;
    methods.push(method(
        pool,
        owner,
        "toString",
        "()Ljava/lang/String;",
        1,
        body,
        (&[], &Type::String),
    )?);
    let mut body = code(pool, owner);
    for (index, (name, ty)) in class.params.iter().enumerate() {
        if index != 0 {
            body.bytes.push(0x1b);
            body.integer(31)?;
            body.bytes.push(0x68);
        }
        field(&mut body, owner, 0, name, ty)?;
        match ty {
            Type::Long => body.method_reference(0xb8, "java/lang/Long", "hashCode", "(J)I")?,
            Type::Float => body.method_reference(0xb8, "java/lang/Float", "hashCode", "(F)I")?,
            Type::Double => body.method_reference(0xb8, "java/lang/Double", "hashCode", "(D)I")?,
            Type::Boolean => {
                body.method_reference(0xb8, "java/lang/Boolean", "hashCode", "(Z)I")?
            }
            ty if ty.is_reference() => {
                if ty.is_nullable() {
                    let present = body.label();
                    let end = body.label();
                    body.bytes.push(0x59);
                    body.branch(0xc7, present);
                    body.bytes.extend([0x57, 0x03]);
                    body.branch(0xa7, end);
                    body.mark(present);
                    body.method_reference(0xb6, "java/lang/Object", "hashCode", "()I")?;
                    body.mark(end);
                } else {
                    body.method_reference(0xb6, "java/lang/Object", "hashCode", "()I")?;
                }
            }
            _ => {}
        }
        if index != 0 {
            body.bytes.push(0x60);
        }
        body.bytes.extend([0x36, 1]);
    }
    body.bytes.extend([0x15, 1, 0xac]);
    let body = finish(body, 2, 4)?;
    methods.push(method(
        pool,
        owner,
        "hashCode",
        "()I",
        1,
        body,
        (&[], &Type::Int),
    )?);
    let mut body = code(pool, owner);
    let same = body.label();
    let different = body.label();
    body.bytes.extend([0x2a, 0x2b]);
    body.branch(0xa5, same);
    body.bytes.push(0x2b);
    body.class_reference(0xc1, owner)?;
    body.branch(0x99, different);
    body.bytes.push(0x2b);
    body.class_reference(0xc0, owner)?;
    body.bytes.extend([0x3a, 2]);
    for (name, ty) in &class.params {
        field(&mut body, owner, 0, name, ty)?;
        field(&mut body, owner, 2, name, ty)?;
        match ty {
            Type::Long => {
                body.bytes.push(0x94);
                body.branch(0x9a, different);
            }
            Type::Float | Type::Double => {
                let (owner, descriptor) = if ty == &Type::Double {
                    ("java/lang/Double", "(DD)I")
                } else {
                    ("java/lang/Float", "(FF)I")
                };
                body.method_reference(0xb8, owner, "compare", descriptor)?;
                body.branch(0x9a, different);
            }
            ty if ty.is_reference() => {
                body.method_reference(
                    0xb8,
                    "kotlin/jvm/internal/Intrinsics",
                    "areEqual",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                )?;
                body.branch(0x99, different);
            }
            _ => body.branch(0xa0, different),
        }
    }
    body.mark(same);
    body.bytes.extend([0x04, 0xac]);
    body.mark(different);
    body.bytes.extend([0x03, 0xac]);
    let body = finish(body, 3, 4)?;
    let params = vec![(
        "other".into(),
        Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
    )];
    methods.push(method(
        pool,
        owner,
        "equals",
        "(Ljava/lang/Object;)Z",
        1,
        body,
        (&params, &Type::Boolean),
    )?);
    Ok(methods)
}
