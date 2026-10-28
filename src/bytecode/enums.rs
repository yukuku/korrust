use super::*;
use crate::check::enums::CheckedEnum;

pub(super) struct Members {
    pub fields: Vec<u8>,
    pub field_count: usize,
    pub methods: Vec<Vec<u8>>,
    pub signature: Vec<u8>,
}

pub(super) fn members(pool: &mut Pool, owner: &str, checked: &CheckedEnum) -> Result<Members> {
    let descriptor = format!("L{owner};");
    let array_descriptor = format!("[{descriptor}");
    let entries_descriptor = "Lkotlin/enums/EnumEntries;";
    let mut fields = Vec::new();
    for (name, _) in &checked.entries {
        fields.extend(class_field(pool, name, &Type::Java(owner.into()), 0x4019)?);
    }
    for (name, descriptor) in [
        ("$VALUES", array_descriptor.as_str()),
        ("$ENTRIES", entries_descriptor),
    ] {
        u2(&mut fields, 0x101a);
        u2(&mut fields, pool.utf8(name)?);
        u2(&mut fields, pool.utf8(descriptor)?);
        u2(&mut fields, 0);
    }
    let class = pool.class(owner)?;
    let array_class = pool.class(&array_descriptor)?;
    let mut methods = Vec::new();
    let mut code = SimpleCode::new(1, 0);
    code.reference(0xb2, pool.field(owner, "$VALUES", &array_descriptor)?);
    code.reference(
        0xb6,
        pool.method(&array_descriptor, "clone", "()Ljava/lang/Object;")?,
    );
    code.reference(0xc0, array_class);
    code.bytes.push(0xb0);
    methods.push(simple_method(
        pool,
        0x0009,
        "values",
        &format!("(){array_descriptor}"),
        code,
        None,
        false,
    )?);
    let mut code = SimpleCode::new(2, 1);
    code.reference(0x13, class);
    code.bytes.push(0x2a);
    code.reference(
        0xb8,
        pool.method(
            "java/lang/Enum",
            "valueOf",
            "(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;",
        )?,
    );
    code.reference(0xc0, class);
    code.bytes.push(0xb0);
    methods.push(simple_method(
        pool,
        0x0009,
        "valueOf",
        &format!("(Ljava/lang/String;){descriptor}"),
        code,
        None,
        false,
    )?);
    let mut code = SimpleCode::new(1, 0);
    code.reference(0xb2, pool.field(owner, "$ENTRIES", entries_descriptor)?);
    code.bytes.push(0xb0);
    methods.push(simple_method(
        pool,
        0x0009,
        "getEntries",
        &format!("(){entries_descriptor}"),
        code,
        Some(&format!("()Lkotlin/enums/EnumEntries<{descriptor}>;")),
        true,
    )?);
    let mut code = SimpleCode::new(if checked.entries.is_empty() { 1 } else { 4 }, 0);
    code.integer(checked.entries.len())?;
    code.reference(0xbd, class);
    for (ordinal, (name, _)) in checked.entries.iter().enumerate() {
        code.bytes.push(0x59);
        code.integer(ordinal)?;
        code.reference(0xb2, pool.field(owner, name, &descriptor)?);
        code.bytes.push(0x53);
    }
    code.bytes.push(0xb0);
    methods.push(simple_method(
        pool,
        0x101a,
        "$values",
        &format!("(){array_descriptor}"),
        code,
        None,
        false,
    )?);
    let mut signature = Vec::new();
    u2(&mut signature, pool.utf8("Signature")?);
    u4(&mut signature, 2);
    let interfaces = checked.class.interfaces.iter().map(|owner| {
        checked.class.super_types.iter().find(|ty| matches!(ty, Type::Java(name) | Type::Parameterized { name, .. } if name == owner)).map(generic_type).unwrap_or_else(|| format!("L{owner};"))
    }).collect::<String>();
    u2(
        &mut signature,
        pool.utf8(&format!("Ljava/lang/Enum<{descriptor}>;{interfaces}"))?,
    );
    Ok(Members {
        fields,
        field_count: checked.entries.len() + 2,
        methods,
        signature,
    })
}

/// Entries initialize in source order, followed by values and entries caches.
/// Companion initialization is emitted by the ordinary class machinery after
/// this prefix, preserving Kotlin's enum initialization dependency ordering.
pub(super) fn initialize(code: &mut Code<'_>, owner: &str, checked: &CheckedEnum) -> Result<()> {
    let descriptor = format!("L{owner};");
    let array = format!("[{descriptor}");
    for (name, value) in &checked.entries {
        code.value(value)?;
        code.field_reference(0xb3, owner, name, &descriptor)?;
    }
    code.method_reference(0xb8, owner, "$values", &format!("(){array}"))?;
    code.field_reference(0xb3, owner, "$VALUES", &array)?;
    code.field_reference(0xb2, owner, "$VALUES", &array)?;
    code.method_reference(
        0xb8,
        "kotlin/enums/EnumEntriesKt",
        "enumEntries",
        "([Ljava/lang/Enum;)Lkotlin/enums/EnumEntries;",
    )?;
    code.field_reference(0xb3, owner, "$ENTRIES", "Lkotlin/enums/EnumEntries;")?;
    Ok(())
}

pub(super) fn constructor_signature(checked: &CheckedEnum) -> String {
    generic_method(&checked.class.params[2..], &Type::Unit).unwrap_or_else(|| {
        format!(
            "({})V",
            checked.class.params[2..]
                .iter()
                .map(|(_, ty)| ty.descriptor())
                .collect::<String>()
        )
    })
}

pub(crate) fn emit(
    checked: &CheckedEnum,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
    companion_metadata: Option<&crate::metadata::Metadata>,
) -> Result<Vec<crate::ClassFile>> {
    let mut files = vec![crate::ClassFile {
        internal_name: owner.into(),
        bytes: emit_class_with(
            &checked.class,
            owner,
            file,
            metadata,
            0x0002,
            &checked.lambdas,
            Some(checked),
            |_| Ok(Vec::new()),
        )?,
    }];
    if let Some(companion) = &checked.class.companion {
        files.push(class_companion::emit(
            companion,
            owner,
            file,
            companion_metadata.ok_or_else(|| failure("Missing enum companion metadata"))?,
            &checked.lambdas,
        )?);
    }
    for lambda in &checked.lambdas {
        files.push(lambdas::emit(lambda, &[], owner, file, &checked.lambdas)?);
    }
    Ok(files)
}
