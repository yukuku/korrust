use super::interfaces::Builder;
use super::*;
use crate::check::{DynamicObject, ObjectField, ObjectInitializer, ObjectMethod};
use crate::metadata::Metadata;

fn add_attribute(method: &mut Vec<u8>, attribute: Vec<u8>) -> Result<()> {
    let count = u16::from_be_bytes([method[6], method[7]])
        .checked_add(1)
        .ok_or_else(|| failure("Too many method attributes"))?;
    method[6..8].copy_from_slice(&count.to_be_bytes());
    method.extend(attribute);
    Ok(())
}

fn exceptions(pool: &mut Pool, names: &[String]) -> Result<Vec<u8>> {
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("Exceptions")?);
    u4(&mut attribute, (2 + 2 * names.len()) as u32);
    u2(
        &mut attribute,
        u16::try_from(names.len()).map_err(|_| failure("Too many declared exceptions"))?,
    );
    for name in names {
        u2(&mut attribute, pool.class(name)?);
    }
    Ok(attribute)
}

fn jvm_static(pool: &mut Pool) -> Result<Vec<u8>> {
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("RuntimeVisibleAnnotations")?);
    u4(&mut attribute, 6);
    u2(&mut attribute, 1);
    u2(&mut attribute, pool.utf8("Lkotlin/jvm/JvmStatic;")?);
    u2(&mut attribute, 0);
    Ok(attribute)
}

fn method(pool: &mut Pool, owner: &str, declaration: &ObjectMethod) -> Result<Vec<u8>> {
    let function = &declaration.function;
    if declaration.inline && (!function.private || declaration.jvm_static) {
        return Err(failure(
            "Only private instance inline object methods are supported",
        ));
    }
    let mut code = Code {
        pool,
        functions: &[],
        owner,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        max_stack: 2,
    };
    if !function.private {
        for (index, (name, ty)) in function.params.iter().enumerate() {
            if ty.is_reference() {
                let slot = index + usize::from(!declaration.jvm_static);
                code.bytes.extend([0x19, slot as u8]);
                let constant = code.pool.string(name)?;
                code.reference(0x13, constant);
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
    let max_stack =
        u16::try_from(code.max_stack).map_err(|_| failure("JVM operand stack limit exceeded"))?;
    let code = SimpleCode {
        bytes: code.finish()?,
        max_stack,
        locals: function.locals,
    };
    let flags = (if function.private { 0x0002 } else { 0x0001 })
        | 0x0010
        | if declaration.jvm_static { 0x0008 } else { 0 };
    let mut method = simple_method(
        pool,
        flags,
        &function.name,
        &function.descriptor(),
        code,
        None,
        !function.private && function.result.is_reference(),
    )?;
    if !function.private {
        parameter_annotations(pool, &mut method, &function.params)?;
    }
    if declaration.jvm_static {
        add_attribute(&mut method, jvm_static(pool)?)?;
    }
    if !declaration.throws.is_empty() {
        add_attribute(&mut method, exceptions(pool, &declaration.throws)?)?;
    }
    Ok(method)
}

fn factory_name(field: &ObjectField) -> String {
    format!("{}$lambda$0", field.name)
}
fn accessor_name(field: &ObjectField) -> String {
    format!("access${}", factory_name(field))
}
fn supplier_name(owner: &str, field: &ObjectField) -> String {
    format!("{owner}${}$Supplier", field.name)
}

fn initializer_type(field: &ObjectField) -> &Type {
    match &field.initializer {
        ObjectInitializer::ThreadLocalArray { array_type, .. } => array_type,
    }
}

fn field(pool: &mut Pool, field: &ObjectField) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    u2(&mut bytes, 0x001a);
    u2(&mut bytes, pool.utf8(&field.name)?);
    u2(&mut bytes, pool.utf8(&field.ty.descriptor())?);
    u2(&mut bytes, 1);
    u2(&mut bytes, pool.utf8("Signature")?);
    u4(&mut bytes, 2);
    u2(
        &mut bytes,
        pool.utf8(&format!(
            "Ljava/lang/ThreadLocal<{}>;",
            initializer_type(field).descriptor()
        ))?,
    );
    Ok(bytes)
}

fn factory(pool: &mut Pool, owner: &str, field: &ObjectField) -> Result<Vec<u8>> {
    let ObjectInitializer::ThreadLocalArray {
        array_type,
        size,
        locals,
    } = &field.initializer;
    let mut code = Code {
        pool,
        functions: &[],
        owner,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        max_stack: 1,
    };
    code.value(size)?;
    code.bytes
        .extend([0xbc, primitive_array_type(array_type)?, 0xb0]);
    let max_stack =
        u16::try_from(code.max_stack).map_err(|_| failure("JVM operand stack limit exceeded"))?;
    let code = SimpleCode {
        bytes: code.finish()?,
        max_stack,
        locals: *locals,
    };
    simple_method(
        pool,
        0x001a,
        &factory_name(field),
        &format!("(){}", array_type.descriptor()),
        code,
        None,
        false,
    )
}

fn accessor(pool: &mut Pool, owner: &str, field: &ObjectField) -> Result<Vec<u8>> {
    let descriptor = format!("(){}", initializer_type(field).descriptor());
    let mut code = SimpleCode::new(1, 0);
    code.reference(0xb8, pool.method(owner, &factory_name(field), &descriptor)?);
    code.bytes.push(0xb0);
    simple_method(
        pool,
        0x1019,
        &accessor_name(field),
        &descriptor,
        code,
        None,
        false,
    )
}

fn supplier(owner: &str, field: &ObjectField, file: &str) -> Result<crate::ClassFile> {
    let name = supplier_name(owner, field);
    let mut builder = Builder::new(&name, 49, 0x1030, &["java/util/function/Supplier".into()])?;
    let mut signature = Vec::new();
    u2(&mut signature, builder.pool.utf8("Signature")?);
    u4(&mut signature, 2);
    u2(
        &mut signature,
        builder.pool.utf8(&format!(
            "Ljava/lang/Object;Ljava/util/function/Supplier<{}>;",
            initializer_type(field).descriptor()
        ))?,
    );
    builder.attributes.push(signature);
    let mut code = SimpleCode::new(1, 1);
    code.bytes.push(0x2a);
    code.reference(
        0xb7,
        builder.pool.method("java/lang/Object", "<init>", "()V")?,
    );
    code.bytes.push(0xb1);
    builder.methods.push(simple_method(
        &mut builder.pool,
        0,
        "<init>",
        "()V",
        code,
        None,
        false,
    )?);
    let mut code = SimpleCode::new(1, 1);
    code.reference(
        0xb8,
        builder.pool.method(
            owner,
            &accessor_name(field),
            &format!("(){}", initializer_type(field).descriptor()),
        )?,
    );
    code.bytes.push(0xb0);
    builder.methods.push(simple_method(
        &mut builder.pool,
        0x0011,
        "get",
        "()Ljava/lang/Object;",
        code,
        None,
        false,
    )?);
    builder.finish(
        file,
        &Metadata {
            kind: 3,
            data1: Vec::new(),
            data2: Vec::new(),
        },
        &[],
    )
}

pub fn emit_dynamic_object(
    object: &DynamicObject,
    owner: &str,
    file: &str,
    metadata: &Metadata,
) -> Result<Vec<crate::ClassFile>> {
    if object.owner != owner {
        return Err(failure("Object class identity mismatch"));
    }
    let mut builder = Builder::new(owner, 49, 0x0031, &[])?;
    builder.field("INSTANCE", &format!("L{owner};"), 0x0019, true)?;
    for property in &object.fields {
        builder.fields.push(field(&mut builder.pool, property)?);
    }
    let mut constructor = SimpleCode::new(1, 1);
    constructor.bytes.push(0x2a);
    constructor.reference(
        0xb7,
        builder.pool.method("java/lang/Object", "<init>", "()V")?,
    );
    constructor.bytes.push(0xb1);
    builder.methods.push(simple_method(
        &mut builder.pool,
        0x0002,
        "<init>",
        "()V",
        constructor,
        None,
        false,
    )?);
    for declaration in &object.methods {
        builder
            .methods
            .push(method(&mut builder.pool, owner, declaration)?);
    }
    for property in &object.fields {
        builder
            .methods
            .push(factory(&mut builder.pool, owner, property)?);
    }
    for property in &object.fields {
        builder
            .methods
            .push(accessor(&mut builder.pool, owner, property)?);
    }
    let mut initializer = SimpleCode::new(2, 0);
    initializer.reference(0xbb, builder.class);
    initializer.bytes.push(0x59);
    initializer.reference(0xb7, builder.pool.method(owner, "<init>", "()V")?);
    initializer.reference(
        0xb3,
        builder
            .pool
            .field(owner, "INSTANCE", &format!("L{owner};"))?,
    );
    for property in &object.fields {
        let supplier = supplier_name(owner, property);
        initializer.reference(0xbb, builder.pool.class(&supplier)?);
        initializer.bytes.push(0x59);
        initializer.reference(0xb7, builder.pool.method(&supplier, "<init>", "()V")?);
        initializer.reference(
            0xb8,
            builder.pool.method(
                "java/lang/ThreadLocal",
                "withInitial",
                "(Ljava/util/function/Supplier;)Ljava/lang/ThreadLocal;",
            )?,
        );
        initializer.reference(
            0xb3,
            builder
                .pool
                .field(owner, &property.name, &property.ty.descriptor())?,
        );
    }
    initializer.bytes.push(0xb1);
    builder.methods.push(simple_method(
        &mut builder.pool,
        0x0008,
        "<clinit>",
        "()V",
        initializer,
        None,
        false,
    )?);
    let mut classes = vec![builder.finish(file, metadata, &[])?];
    for property in &object.fields {
        classes.push(supplier(owner, property, file)?);
    }
    Ok(classes)
}
