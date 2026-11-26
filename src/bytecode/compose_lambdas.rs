use super::*;
use crate::classpath::compose_abi::{COMPOSER, FACTORY};

pub(super) fn owner(body: &str) -> String {
    format!("{body}$composable")
}

fn interface(
    code: &mut SimpleCode,
    pool: &mut Pool,
    name: &str,
    descriptor: &str,
    count: u8,
) -> Result<()> {
    code.reference(0xb9, pool.interface_method(COMPOSER, name, descriptor)?);
    code.bytes.extend([count, 0]);
    Ok(())
}

fn jump(code: &mut SimpleCode, opcode: u8) -> usize {
    let start = code.bytes.len();
    code.bytes.extend([opcode, 0, 0]);
    start
}

fn bind(code: &mut SimpleCode, branch: usize, target: usize) -> Result<()> {
    let offset = i16::try_from(target as isize - branch as isize)
        .map_err(|_| failure("Compose callback branch exceeds JVM range"))?;
    code.bytes[branch + 1..branch + 3].copy_from_slice(&offset.to_be_bytes());
    Ok(())
}

pub(super) fn emit(
    lambda: &crate::check::LambdaDecl,
    key: i32,
    file: &str,
) -> Result<crate::ClassFile> {
    let owner = owner(&lambda.owner);
    let mut pool = Pool::default();
    let this = pool.class(&owner)?;
    let parent = pool.class("java/lang/Object")?;
    let function2 = pool.class("kotlin/jvm/functions/Function2")?;
    let body_type = Type::Function {
        params: vec![Type::Java(COMPOSER.into())],
        result: Box::new(Type::Unit),
    };
    let mut fields = class_field(&mut pool, "body", &body_type, 0x0012)?;
    let mut methods = Vec::new();
    let mut constructor = SimpleCode::new(2, 2);
    constructor.bytes.push(0x2a);
    constructor.reference(0xb7, pool.method("java/lang/Object", "<init>", "()V")?);
    constructor.bytes.extend([0x2a, 0x2b]);
    constructor.reference(
        0xb5,
        pool.field(&owner, "body", "Lkotlin/jvm/functions/Function1;")?,
    );
    constructor.bytes.push(0xb1);
    methods.push(simple_method(
        &mut pool,
        0x0001,
        "<init>",
        "(Lkotlin/jvm/functions/Function1;)V",
        constructor,
        None,
        false,
    )?);

    let mut invoke = SimpleCode::new(4, 4);
    invoke.bytes.extend([0x1c, 0x06, 0x7e, 0x05]);
    let unchanged = jump(&mut invoke, 0x9f);
    invoke.bytes.push(0x04);
    let boolean_done = jump(&mut invoke, 0xa7);
    let unchanged_target = invoke.bytes.len();
    invoke.bytes.push(0x03);
    let boolean_target = invoke.bytes.len();
    bind(&mut invoke, unchanged, unchanged_target)?;
    bind(&mut invoke, boolean_done, boolean_target)?;
    invoke.bytes.extend([0x3e, 0x2b, 0x1d, 0x1c, 0x04, 0x7e]);
    interface(&mut invoke, &mut pool, "shouldExecute", "(ZI)Z", 3)?;
    let skip = jump(&mut invoke, 0x99);
    invoke.reference(
        0xb8,
        pool.method(
            "androidx/compose/runtime/ComposerKt",
            "isTraceInProgress",
            "()Z",
        )?,
    );
    let no_trace_start = jump(&mut invoke, 0x99);
    invoke.reference(0x13, pool.add(Constant::Integer(key))?);
    invoke.bytes.extend([0x1c, 0x02]);
    invoke.reference(0x13, pool.string(&format!("{} ({file})", lambda.owner))?);
    invoke.reference(
        0xb8,
        pool.method(
            "androidx/compose/runtime/ComposerKt",
            "traceEventStart",
            "(IIILjava/lang/String;)V",
        )?,
    );
    let body_start = invoke.bytes.len();
    bind(&mut invoke, no_trace_start, body_start)?;
    invoke.bytes.push(0x2a);
    invoke.reference(
        0xb4,
        pool.field(&owner, "body", "Lkotlin/jvm/functions/Function1;")?,
    );
    invoke.bytes.push(0x2b);
    invoke.reference(
        0xb9,
        pool.interface_method(
            "kotlin/jvm/functions/Function1",
            "invoke",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
        )?,
    );
    invoke.bytes.extend([2, 0, 0x57]);
    invoke.reference(
        0xb8,
        pool.method(
            "androidx/compose/runtime/ComposerKt",
            "isTraceInProgress",
            "()Z",
        )?,
    );
    let no_trace_end = jump(&mut invoke, 0x99);
    invoke.reference(
        0xb8,
        pool.method(
            "androidx/compose/runtime/ComposerKt",
            "traceEventEnd",
            "()V",
        )?,
    );
    let completed = invoke.bytes.len();
    bind(&mut invoke, no_trace_end, completed)?;
    invoke.bytes.push(0xb1);
    let skip_target = invoke.bytes.len();
    bind(&mut invoke, skip, skip_target)?;
    invoke.bytes.push(0x2b);
    interface(&mut invoke, &mut pool, "skipToGroupEnd", "()V", 1)?;
    invoke.bytes.push(0xb1);
    methods.push(simple_method(
        &mut pool,
        0x0011,
        "invoke",
        "(Landroidx/compose/runtime/Composer;I)V",
        invoke,
        None,
        false,
    )?);

    let mut bridge = SimpleCode::new(3, 3);
    bridge.bytes.extend([0x2a, 0x2b]);
    bridge.reference(0xc0, pool.class(COMPOSER)?);
    bridge.bytes.push(0x2c);
    bridge.reference(0xc0, pool.class("java/lang/Number")?);
    bridge.reference(0xb6, pool.method("java/lang/Number", "intValue", "()I")?);
    bridge.reference(
        0xb6,
        pool.method(&owner, "invoke", "(Landroidx/compose/runtime/Composer;I)V")?,
    );
    bridge.reference(
        0xb2,
        pool.field("kotlin/Unit", "INSTANCE", "Lkotlin/Unit;")?,
    );
    bridge.bytes.push(0xb0);
    methods.push(simple_method(
        &mut pool,
        0x1041,
        "invoke",
        "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
        bridge,
        None,
        false,
    )?);

    let cached = lambda.captures.is_empty();
    if cached {
        fields.extend(class_field(
            &mut pool,
            "INSTANCE",
            &Type::Java("kotlin/jvm/functions/Function2".into()),
            0x1019,
        )?);
        let mut initialize = SimpleCode::new(8, 0);
        initialize.reference(0x13, pool.add(Constant::Integer(key))?);
        initialize.bytes.push(0x03);
        initialize.reference(0xbb, this);
        initialize.bytes.push(0x59);
        initialize.reference(0xbb, pool.class(&lambda.owner)?);
        initialize.bytes.push(0x59);
        initialize.reference(0xb7, pool.method(&lambda.owner, "<init>", "()V")?);
        initialize.reference(
            0xb7,
            pool.method(&owner, "<init>", "(Lkotlin/jvm/functions/Function1;)V")?,
        );
        initialize.reference(
            0xb8,
            pool.method(
                FACTORY,
                "composableLambdaInstance",
                "(IZLjava/lang/Object;)Landroidx/compose/runtime/internal/ComposableLambda;",
            )?,
        );
        initialize.reference(
            0xb3,
            pool.field(&owner, "INSTANCE", "Lkotlin/jvm/functions/Function2;")?,
        );
        initialize.bytes.push(0xb1);
        methods.push(simple_method(
            &mut pool, 0x0008, "<clinit>", "()V", initialize, None, false,
        )?);
    }
    let signature = signature_attribute(
        &mut pool,
        "Ljava/lang/Object;Lkotlin/jvm/functions/Function2<Landroidx/compose/runtime/Composer;Ljava/lang/Integer;Lkotlin/Unit;>;",
    )?;
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let mut bytes = Vec::new();
    u4(&mut bytes, 0xcafebabe);
    u2(&mut bytes, 0);
    u2(&mut bytes, 49);
    pool.write(&mut bytes);
    u2(&mut bytes, 0x1031);
    u2(&mut bytes, this);
    u2(&mut bytes, parent);
    u2(&mut bytes, 1);
    u2(&mut bytes, function2);
    u2(&mut bytes, 1 + u16::from(cached));
    bytes.extend(fields);
    u2(&mut bytes, methods.len() as u16);
    for method in methods {
        bytes.extend(method);
    }
    u2(&mut bytes, 2);
    bytes.extend(signature);
    u2(&mut bytes, source_name);
    u4(&mut bytes, 2);
    u2(&mut bytes, source_file);
    Ok(crate::ClassFile {
        internal_name: owner,
        bytes,
    })
}
