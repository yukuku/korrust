use super::*;

pub(crate) fn for_enum(
    source: &crate::syntax::EnumDecl,
    checked: &crate::check::enums::CheckedEnum,
    owner: &str,
    imports: Imports<'_>,
) -> Result<Metadata> {
    let mut bindings = imports.to_vec();
    bindings.push((source.name.clone(), owner.into(), source.offset));
    // A generated JVM self type can be a nested default-package binary name
    // without '/'. Bind that known identity as well as its source short name.
    bindings.push((owner.into(), owner.into(), source.offset));
    let ordinary = class_metadata(&source.body, owner, &bindings, false, Some(&checked.class))?;
    let bytes = ordinary
        .data1
        .iter()
        .flatten()
        .skip(1)
        .map(|unit| {
            u8::try_from(*unit)
                .map_err(|_| Diagnostic::new(source.offset, "Invalid generated enum metadata"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut cursor = 0;
    let table_len = read_varint(&bytes, &mut cursor)? as usize;
    cursor = cursor
        .checked_add(table_len)
        .filter(|cursor| *cursor <= bytes.len())
        .ok_or_else(|| Diagnostic::new(source.offset, "Invalid generated enum string table"))?;
    let mut payload = Proto::default();
    let mut class_flags = 6;
    while cursor < bytes.len() {
        let start = cursor;
        let key = read_varint(&bytes, &mut cursor)?;
        let field = key >> 3;
        let number = match key & 7 {
            0 => Some(read_varint(&bytes, &mut cursor)?),
            2 => {
                let length = read_varint(&bytes, &mut cursor)? as usize;
                cursor = cursor
                    .checked_add(length)
                    .filter(|cursor| *cursor <= bytes.len())
                    .ok_or_else(|| {
                        Diagnostic::new(source.offset, "Invalid generated enum metadata field")
                    })?;
                None
            }
            _ => {
                return Err(Diagnostic::new(
                    source.offset,
                    "Invalid generated enum metadata wire type",
                ));
            }
        };
        if field == 1 {
            class_flags = number.unwrap_or(6) as u32;
        }
        if !matches!(field, 1 | 6 | 8) {
            payload.0.extend(&bytes[start..cursor]);
        }
    }
    let mut strings = Strings {
        jvm_names: Default::default(),
        indices: ordinary
            .data2
            .iter()
            .enumerate()
            .map(|(index, value)| (value.clone(), index as u32))
            .collect(),
        values: ordinary.data2,
        local_names: Vec::new(),
        parameters: BTreeMap::new(),
        jvm_descriptors: BTreeMap::new(),
    };
    payload.number(1, class_flags | 128 | 32768);
    payload.message(
        6,
        &type_proto(
            &Type::Parameterized {
                name: "kotlin/Enum".into(),
                arguments: vec![crate::syntax::TypeArgument {
                    variance: Variance::Invariant,
                    ty: Some(Type::Java(owner.into())),
                }],
            },
            &bindings,
            &mut strings,
        )?,
    );
    let mut constructor = Proto::default();
    constructor.number(1, 2);
    for (name, ty) in &source.body.params {
        constructor.message(2, &parameter(name, ty, &bindings, &mut strings)?);
    }
    let descriptor = format!(
        "(Ljava/lang/String;I{})V",
        checked.class.params[2..]
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    constructor.message(100, &method_signature("<init>", &descriptor, &mut strings)?);
    payload.message(8, &constructor);
    for (name, _) in &source.entries {
        let mut entry = Proto::default();
        entry.number(1, strings.id(name)?);
        payload.message(13, &entry);
    }
    strings.finish(1, payload)
}

fn read_varint(bytes: &[u8], cursor: &mut usize) -> Result<u64> {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes
            .get(*cursor)
            .ok_or_else(|| Diagnostic::new(0, "Truncated generated enum metadata"))?;
        *cursor += 1;
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(Diagnostic::new(0, "Invalid generated enum metadata varint"))
}
