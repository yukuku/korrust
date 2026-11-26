use super::*;

fn unsupported(message: &str) -> Diagnostic {
    Diagnostic::new(0, message).with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "INLINE_DEFAULT_LAMBDA_UNSUPPORTED",
    )
}

fn encoded(text: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for unit in text.encode_utf16() {
        match unit {
            1..=127 => bytes.push(unit as u8),
            0..=2047 => bytes.extend([0xc0 | (unit >> 6) as u8, 0x80 | (unit & 63) as u8]),
            _ => bytes.extend([
                0xe0 | (unit >> 12) as u8,
                0x80 | ((unit >> 6) & 63) as u8,
                0x80 | (unit & 63) as u8,
            ]),
        }
    }
    let size = u16::try_from(bytes.len())
        .map_err(|_| unsupported("Regenerated lambda name exceeds classfile UTF-8 limit"))?;
    let mut result = vec![1];
    result.extend(size.to_be_bytes());
    result.extend(bytes);
    Ok(result)
}

fn rewrite(bytes: &[u8], owner: &str, renamed: &str, removals: &[Vec<usize>]) -> Result<Vec<u8>> {
    let mut reader = Reader { bytes, offset: 8 };
    let count = reader.u2()?;
    let mut pool = vec![Vec::new(); usize::from(count)];
    let mut names = std::collections::BTreeMap::new();
    let mut literals = std::collections::BTreeSet::new();
    let mut index = 1;
    while index < count {
        let start = reader.offset;
        let tag = reader.u1()?;
        match tag {
            1 => {
                let size = usize::from(reader.u2()?);
                let name = String::from_utf16_lossy(&modified_utf8_units(reader.take(size)?)?);
                names.insert(index, name);
            }
            3 | 4 | 9..=12 | 17 | 18 => {
                reader.take(4)?;
            }
            5 | 6 => {
                reader.take(8)?;
            }
            8 => {
                literals.insert(reader.u2()?);
            }
            7 | 16 | 19 | 20 => {
                reader.take(2)?;
            }
            15 => {
                reader.take(3)?;
            }
            _ => return Err(invalid()),
        }
        pool[usize::from(index)] = bytes[start..reader.offset].to_vec();
        index += if matches!(tag, 5 | 6) { 2 } else { 1 };
    }
    for (index, name) in &names {
        let new = if name == owner {
            renamed.to_owned()
        } else {
            name.replace(&format!("L{owner};"), &format!("L{renamed};"))
        };
        if new != *name {
            if literals.contains(index) {
                return Err(unsupported(
                    "Default lambda shares a class-name constant with a string literal",
                ));
            }
            pool[usize::from(*index)] = encoded(&new)?;
        }
    }
    let mut output = bytes[..10].to_vec();
    for entry in pool {
        output.extend(entry);
    }
    let start = reader.offset;
    reader.take(6)?;
    let interfaces = usize::from(reader.u2()?);
    reader.take(interfaces * 2)?;
    output.extend(&bytes[start..reader.offset]);
    for methods in [false, true] {
        let count = reader.u2()?;
        output.extend(count.to_be_bytes());
        let mut method_removals = removals.iter();
        for _ in 0..count {
            let empty = Vec::new();
            let removed = if methods {
                method_removals.next().ok_or_else(invalid)?
            } else {
                &empty
            };
            let start = reader.offset;
            reader.take(6)?;
            let attributes = reader.u2()?;
            output.extend(&bytes[start..reader.offset]);
            for _ in 0..attributes {
                let name = reader.u2()?;
                let size = usize::try_from(reader.u4()?).map_err(|_| invalid())?;
                let mut payload = reader.take(size)?.to_vec();
                if methods && names.get(&name).is_some_and(|name| name == "Code") {
                    for offset in removed {
                        payload[8 + offset..8 + offset + 3].fill(0);
                    }
                }
                output.extend(name.to_be_bytes());
                output.extend((size as u32).to_be_bytes());
                output.extend(payload);
            }
        }
    }
    let mut attributes = Vec::new();
    for _ in 0..reader.u2()? {
        let name = reader.u2()?;
        let size = usize::try_from(reader.u4()?).map_err(|_| invalid())?;
        let payload = reader.take(size)?;
        if names.get(&name).is_some_and(|name| {
            matches!(
                name.as_str(),
                "InnerClasses" | "EnclosingMethod" | "NestHost" | "NestMembers"
            )
        }) {
            continue;
        }
        attributes.push((name, payload));
    }
    output.extend((attributes.len() as u16).to_be_bytes());
    for (name, payload) in attributes {
        output.extend(name.to_be_bytes());
        output.extend((payload.len() as u32).to_be_bytes());
        output.extend(payload);
    }
    Ok(output)
}

impl Classpath {
    pub(crate) fn regenerate_inline_default_lambda(
        &self,
        owner: &str,
        caller: &str,
        offset: usize,
    ) -> Result<String> {
        use std::hash::{Hash, Hasher};
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        owner.hash(&mut hash);
        let renamed = format!("{caller}$inlineDefault${offset}${:016x}", hash.finish());
        if self.inline_default_classes.borrow().contains_key(&renamed) {
            return Ok(renamed);
        }
        let mut input = None;
        for path in &self.paths {
            if let Some(bytes) = self.read_resource(path, &format!("{owner}.class"))? {
                input = Some(bytes);
                break;
            }
        }
        let bytes = input.ok_or_else(|| unsupported("Default lambda class is unavailable"))?;
        let (class, _, attributes, _) = parse_details_impl(&bytes, owner, false)?;
        if class.access & 0x11 != 0x11
            || attributes
                .iter()
                .any(|attributes| attributes.code.is_none())
            || !matches!(
                class.super_name.as_deref(),
                Some("java/lang/Object" | "kotlin/jvm/internal/Lambda")
            )
            || class.fields.len() != 1
            || class.fields[0].name != "INSTANCE"
            || class.fields[0].descriptor != format!("L{owner};")
            || class.fields[0].access & 0x19 != 0x19
            || class.interfaces.len() != 1
            || !class.interfaces[0].starts_with("kotlin/jvm/functions/Function")
            || !class.methods.iter().any(|method| {
                method.name == "<init>" && method.descriptor == "()V" && method.access & 1 != 0
            })
            || class
                .methods
                .iter()
                .any(|method| !matches!(method.name.as_str(), "invoke" | "<init>" | "<clinit>"))
        {
            return Err(unsupported(
                "Default lambda regeneration requires a public capture-free function singleton",
            ));
        }
        let mut removals = Vec::new();
        for (method, attributes) in class.methods.iter().zip(attributes) {
            let mut removed = Vec::new();
            if let Some(code) = &attributes.code {
                for instruction in crate::compiled_inline::decode(code)?.instructions {
                    if let crate::compiled_inline::InlineOp::Reference {
                        opcode: 0xb8,
                        constant: InlineConstant::Method { owner, name, .. },
                        ..
                    } = instruction.kind
                        && owner == "kotlin/jvm/internal/Intrinsics"
                    {
                        if name == "reifiedOperationMarker"
                            || name == "needClassReification" && method.name != "<clinit>"
                        {
                            return Err(unsupported(
                                "Default lambda body requires reified class regeneration",
                            ));
                        }
                        if name == "needClassReification" {
                            removed.push(instruction.offset);
                        }
                    }
                }
                let mut validation = method.clone();
                validation.access |= 8;
                validation.library = Some(LibraryMethod {
                    default_code: None,
                    vararg_index: None,
                    contracts: Vec::new(),
                    value_dispatch: false,
                    suspend_: false,
                    kotlin_declared: false,
                    has_annotations: false,
                    parameter_modes: Vec::new(),
                    reified_arguments: Vec::new(),
                    abstract_: false,
                    builtin_array: false,
                    kotlin_public: true,
                    source_name: method.name.clone(),
                    type_parameters: Vec::new(),
                    receiver: None,
                    infix: false,
                    operator: false,
                    inline: true,
                    code: Some(code.clone()),
                });
                crate::compiled_inline::validate_access(&validation, self, &renamed)?;
            }
            removals.push(removed);
        }
        let output = rewrite(&bytes, owner, &renamed, &removals)?;
        let class = parse_details_impl(&output, &renamed, false)?.0;
        self.cache.borrow_mut().insert(renamed.clone(), class);
        self.inline_default_classes
            .borrow_mut()
            .insert(renamed.clone(), output);
        Ok(renamed)
    }

    pub(crate) fn inline_default_outputs(
        &self,
        files: &[crate::ClassFile],
    ) -> Result<Vec<crate::ClassFile>> {
        let generated = self.inline_default_classes.borrow();
        if generated.is_empty() {
            return Ok(Vec::new());
        }
        let mut used = std::collections::BTreeSet::new();
        for file in files {
            let (_, _, methods, _) = parse_details_impl(&file.bytes, &file.internal_name, false)?;
            for code in methods.into_iter().filter_map(|method| method.code) {
                for constant in code.constants.values() {
                    let owner = match constant {
                        InlineConstant::Class(owner)
                        | InlineConstant::Field { owner, .. }
                        | InlineConstant::Method { owner, .. } => owner,
                        _ => continue,
                    };
                    if generated.contains_key(owner) {
                        used.insert(owner.clone());
                    }
                }
            }
        }
        Ok(used
            .into_iter()
            .map(|owner| crate::ClassFile {
                bytes: generated[&owner].clone(),
                internal_name: owner,
            })
            .collect())
    }
}
