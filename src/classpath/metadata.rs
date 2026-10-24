use super::*;
use crate::syntax::{TypeArgument, TypeParameter, Variance};
use std::collections::BTreeMap;

fn strings(annotation: &Annotation, key: &str) -> Result<Vec<String>> {
    match annotation
        .values
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
    {
        Some(AnnotationValue::Array(values)) => values
            .iter()
            .map(|v| match v {
                AnnotationValue::String(s) => Ok(s.clone()),
                _ => Err(invalid()),
            })
            .collect(),
        None => Ok(Vec::new()),
        _ => Err(invalid()),
    }
}
fn integer(annotation: &Annotation, key: &str) -> Result<Option<u32>> {
    match annotation
        .values
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
    {
        Some(AnnotationValue::Integer(n)) => Ok(Some(*n)),
        None => Ok(None),
        _ => Err(invalid()),
    }
}
fn text(annotation: &Annotation, key: &str) -> Result<Option<String>> {
    match annotation
        .values
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
    {
        Some(AnnotationValue::String(s)) => Ok(Some(s.clone())),
        None => Ok(None),
        _ => Err(invalid()),
    }
}

fn decode(values: &[String]) -> Result<Vec<u8>> {
    let value = values.concat();
    let mut chars = value.chars();
    let marker = chars.next().ok_or_else(invalid)?;
    if marker == '\0' {
        return chars
            .map(|c| u8::try_from(u32::from(c)).map_err(|_| invalid()))
            .collect();
    }
    let encoded = if marker == '\u{ffff}' {
        chars.collect::<String>()
    } else {
        value
    };
    let chunks = encoded
        .chars()
        .map(|c| {
            u8::try_from(u32::from(c))
                .map(|v| v.wrapping_sub(1) & 127)
                .map_err(|_| invalid())
        })
        .collect::<Result<Vec<_>>>()?;
    let mut result = Vec::with_capacity(chunks.len() * 7 / 8);
    let mut acc = 0u32;
    let mut bits = 0;
    for byte in chunks {
        acc |= u32::from(byte) << bits;
        bits += 7;
        if bits >= 8 {
            result.push(acc as u8);
            acc >>= 8;
            bits -= 8;
        }
    }
    Ok(result)
}

#[derive(Clone)]
struct Context<'a> {
    strings: &'a [String],
    table: Vec<&'a [u8]>,
    first_nullable: Option<usize>,
    parameters: BTreeMap<u64, Type>,
}
impl<'a> Context<'a> {
    fn new(strings: &'a [String], proto: &Proto<'a>) -> Result<Self> {
        let mut result = Self {
            strings,
            table: Vec::new(),
            first_nullable: None,
            parameters: BTreeMap::new(),
        };
        result.set_table(proto)?;
        Ok(result)
    }
    fn set_table(&mut self, proto: &Proto<'a>) -> Result<()> {
        if let Some(bytes) = proto.bytes(30)? {
            let table = Proto::parse(bytes)?;
            self.table = table.messages(1)?;
            self.first_nullable = table
                .integer(2)?
                .map(usize::try_from)
                .transpose()
                .map_err(|_| invalid())?;
        }
        Ok(())
    }
    fn field(&self, proto: &Proto<'a>, inline: u32, index: u32) -> Result<Type> {
        self.field_at(proto, inline, index, 0)
    }
    fn field_at(&self, proto: &Proto<'a>, inline: u32, index: u32, depth: usize) -> Result<Type> {
        if let Some(bytes) = proto.bytes(inline)? {
            if proto.integer(index)?.is_some() {
                return Err(invalid());
            }
            self.ty(bytes, depth)
        } else {
            let i = usize::try_from(proto.integer(index)?.ok_or_else(invalid)?)
                .map_err(|_| invalid())?;
            let mut ty = self.ty(self.table.get(i).ok_or_else(invalid)?, depth)?;
            if self.first_nullable.is_some_and(|first| i >= first) && !ty.is_nullable() {
                ty = Type::Nullable(Box::new(ty));
            }
            Ok(ty)
        }
    }
    fn ty(&self, bytes: &'a [u8], depth: usize) -> Result<Type> {
        if depth > 64 {
            return Err(invalid());
        }
        let p = Proto::parse(bytes)?;
        p.allowed(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14])?;
        if p.integer(1)?.unwrap_or(0) != 0 || p.bytes(10)?.is_some() || p.integer(11)?.is_some() {
            return Err(invalid());
        }
        let mut ty = if let Some(id) = p.integer(7)? {
            self.parameters.get(&id).cloned().ok_or_else(invalid)?
        } else if let Some(name) = p.integer(9)? {
            let name = metadata_name(self.strings, Some(name))?;
            self.parameters
                .values()
                .find(|ty| matches!(ty,Type::Parameter{name:n,..}if n==name))
                .cloned()
                .ok_or_else(invalid)?
        } else {
            let name = metadata_name(self.strings, p.integer(6)?)?;
            let mut arguments = Vec::new();
            for bytes in p.messages(2)? {
                let a = Proto::parse(bytes)?;
                a.allowed(&[1, 2, 3])?;
                let projection = a.integer(1)?.unwrap_or(2);
                let variance = match projection {
                    0 => Variance::In,
                    1 | 3 => Variance::Out,
                    2 => Variance::Invariant,
                    _ => return Err(invalid()),
                };
                let inner = if projection == 3 {
                    None
                } else if let Some(bytes) = a.bytes(2)? {
                    Some(self.ty(bytes, depth + 1)?)
                } else {
                    Some(self.field_at(&a, 2, 3, depth + 1)?)
                };
                arguments.push(TypeArgument {
                    variance,
                    ty: inner,
                });
            }
            if !arguments.is_empty()
                && matches!(
                    name,
                    "kotlin/Int"
                        | "kotlin/Long"
                        | "kotlin/Byte"
                        | "kotlin/Char"
                        | "kotlin/Boolean"
                        | "kotlin/Float"
                        | "kotlin/String"
                        | "kotlin/Unit"
                        | "kotlin/IntArray"
                        | "kotlin/ByteArray"
                        | "kotlin/CharArray"
                )
            {
                return Err(invalid());
            }
            match name {
                "kotlin/Int" => Type::Int,
                "kotlin/Long" => Type::Long,
                "kotlin/Byte" => Type::Byte,
                "kotlin/Char" => Type::Char,
                "kotlin/Boolean" => Type::Boolean,
                "kotlin/Float" => Type::Float,
                "kotlin/String" => Type::String,
                "kotlin/Unit" => Type::Unit,
                "kotlin/IntArray" => Type::IntArray,
                "kotlin/ByteArray" => Type::ByteArray,
                "kotlin/CharArray" => Type::CharArray,
                "kotlin/Array" if arguments.len() == 1 && arguments[0].ty == Some(Type::String) => {
                    Type::StringArray
                }
                "kotlin/collections/List" | "kotlin/collections/MutableList"
                    if arguments.len() == 1
                        && arguments[0].ty.is_some()
                        && arguments[0].variance == Variance::Invariant =>
                {
                    let inner = Box::new(arguments.remove(0).ty.unwrap());
                    if name.ends_with("/MutableList") {
                        Type::MutableList(inner)
                    } else {
                        Type::List(inner)
                    }
                }
                _ if name.starts_with("kotlin/Function") && !arguments.is_empty() => {
                    let mut types = arguments
                        .into_iter()
                        .map(|a| a.ty.ok_or_else(invalid))
                        .collect::<Result<Vec<_>>>()?;
                    let result = Box::new(types.pop().ok_or_else(invalid)?);
                    Type::Function {
                        params: types,
                        result,
                    }
                }
                _ => {
                    let name =
                        match name {
                            "kotlin/Any" => "java/lang/Object",
                            "kotlin/Number" => "java/lang/Number",
                            "kotlin/CharSequence" => "java/lang/CharSequence",
                            "kotlin/Throwable" => "java/lang/Throwable",
                            "kotlin/Comparable" => "java/lang/Comparable",
                            "kotlin/Cloneable" => "java/lang/Cloneable",
                            "kotlin/collections/Iterable"
                            | "kotlin/collections/MutableIterable" => "java/lang/Iterable",
                            "kotlin/collections/List" | "kotlin/collections/MutableList" => {
                                "java/util/List"
                            }
                            "kotlin/collections/Iterator"
                            | "kotlin/collections/MutableIterator" => "java/util/Iterator",
                            "kotlin/collections/ListIterator"
                            | "kotlin/collections/MutableListIterator" => "java/util/ListIterator",
                            "kotlin/collections/Map.Entry"
                            | "kotlin/collections/MutableMap.MutableEntry" => "java/util/Map$Entry",
                            "kotlin/collections/Collection"
                            | "kotlin/collections/MutableCollection" => "java/util/Collection",
                            "kotlin/collections/Set" | "kotlin/collections/MutableSet" => {
                                "java/util/Set"
                            }
                            "kotlin/collections/Map" | "kotlin/collections/MutableMap" => {
                                "java/util/Map"
                            }
                            other => other,
                        }
                        .replace('.', "$");
                    if !valid_name(&name) {
                        return Err(invalid());
                    }
                    if arguments.is_empty() {
                        Type::Java(name)
                    } else {
                        Type::Parameterized { name, arguments }
                    }
                }
            }
        };
        if p.integer(3)?.unwrap_or(0) > 1 {
            return Err(invalid());
        }
        if p.integer(3)? == Some(1) {
            ty = Type::Nullable(Box::new(ty));
        }
        if p.integer(4)?.is_some() {
            if p.bytes(5)?.is_none() && p.integer(8)?.is_none() {
                return Err(invalid());
            }
            ty = Type::Platform(Box::new(ty));
        }
        Ok(ty)
    }
    fn type_parameters(&mut self, proto: &Proto<'a>, field: u32) -> Result<Vec<TypeParameter>> {
        let values = proto.messages(field)?;
        let mut result = Vec::new();
        for bytes in &values {
            let p = Proto::parse(bytes)?;
            let name = metadata_name(self.strings, p.integer(2)?)?.to_owned();
            self.parameters.insert(
                p.integer(1)?.ok_or_else(invalid)?,
                Type::Parameter {
                    name,
                    upper_bound: Box::new(Type::Java("java/lang/Object".into())),
                },
            );
        }
        for bytes in values {
            let p = Proto::parse(bytes)?;
            p.allowed(&[1, 2, 3, 4, 5, 6])?;
            if p.integer(3)?.unwrap_or(0) != 0 {
                return Err(invalid());
            }
            let name = metadata_name(self.strings, p.integer(2)?)?.to_owned();
            let variance = match p.integer(4)?.unwrap_or(2) {
                0 => Variance::In,
                1 => Variance::Out,
                2 => Variance::Invariant,
                _ => return Err(invalid()),
            };
            let mut bounds = p
                .messages(5)?
                .into_iter()
                .map(|b| self.ty(b, 0))
                .collect::<Result<Vec<_>>>()?;
            for i in numbers(&p, 6)? {
                bounds.push(self.ty(self.table.get(i as usize).ok_or_else(invalid)?, 0)?);
            }
            if bounds.is_empty() {
                bounds.push(Type::Nullable(Box::new(Type::Java(
                    "java/lang/Object".into(),
                ))));
            }
            self.parameters.insert(
                p.integer(1)?.ok_or_else(invalid)?,
                Type::Parameter {
                    name: name.clone(),
                    upper_bound: Box::new(bounds[0].clone()),
                },
            );
            result.push(TypeParameter {
                name,
                variance,
                bounds,
            });
        }
        Ok(result)
    }
    fn parameter(&self, bytes: &'a [u8]) -> Result<(String, Type, bool)> {
        let p = Proto::parse(bytes)?;
        p.allowed(&[1, 2, 3, 5])?;
        let flags = p.integer(1)?.unwrap_or(0);
        if flags & !3 != 0 {
            return Err(invalid());
        }
        Ok((
            metadata_name(self.strings, p.integer(2)?)?.into(),
            self.field(&p, 3, 5)?,
            flags & 2 != 0,
        ))
    }
}

pub(super) fn numbers(proto: &Proto<'_>, field: u32) -> Result<Vec<u64>> {
    let mut values = Vec::new();
    for (number, value) in &proto.0 {
        if *number != field {
            continue;
        }
        match value {
            ProtoValue::Integer(i) => values.push(*i),
            ProtoValue::Bytes(bytes) => {
                let mut r = Reader { bytes, offset: 0 };
                while r.offset < bytes.len() {
                    values.push(varint(&mut r)?);
                }
            }
        }
    }
    Ok(values)
}

pub(super) fn module_parts(bytes: &[u8], package: &str) -> Result<Vec<String>> {
    let mut r = Reader { bytes, offset: 0 };
    let count = r.u4()?;
    if !(1..=16).contains(&count) {
        return Err(invalid());
    }
    let mut version = Vec::new();
    for _ in 0..count {
        version.push(r.u4()?);
    }
    if version[0] > 2 {
        return Err(invalid());
    }
    if version[0] > 1 || version.get(1).copied().unwrap_or(0) >= 4 {
        r.u4()?;
    }
    let module = Proto::parse(r.take(bytes.len() - r.offset)?)?;
    let mut result = Vec::new();
    for bytes in module.messages(1)? {
        let p = Proto::parse(bytes)?;
        let name = std::str::from_utf8(p.bytes(1)?.ok_or_else(invalid)?)
            .map_err(|_| invalid())?
            .replace('.', "/");
        if name != package {
            continue;
        }
        for bytes in p.messages(2)? {
            let short = std::str::from_utf8(bytes).map_err(|_| invalid())?;
            let name = if package.is_empty() {
                short.into()
            } else {
                format!("{package}/{short}")
            };
            if !valid_name(&name) {
                return Err(invalid());
            }
            result.push(name);
        }
        let jvm_packages = module.messages(3)?;
        let package_ids = numbers(&p, 6)?;
        for (index, bytes) in p.messages(5)?.into_iter().enumerate() {
            let package_id = package_ids
                .get(index)
                .or_else(|| package_ids.last())
                .ok_or_else(invalid)?;
            let jvm_package =
                std::str::from_utf8(jvm_packages.get(*package_id as usize).ok_or_else(invalid)?)
                    .map_err(|_| invalid())?
                    .replace('.', "/");
            let short = std::str::from_utf8(bytes).map_err(|_| invalid())?;
            let name = if jvm_package.is_empty() {
                short.into()
            } else {
                format!("{jvm_package}/{short}")
            };
            if !valid_name(&name) {
                return Err(invalid());
            }
            result.push(name);
        }
    }
    Ok(result)
}

struct MemberTypes {
    params: Vec<(String, Type, bool)>,
    result: Type,
    receiver: Option<Type>,
    type_parameters: Vec<TypeParameter>,
    flags: u64,
}

fn attach_member(
    class: &mut JavaClass,
    attributes: &[Attributes],
    name: &str,
    types: MemberTypes,
    signature: Option<&[u8]>,
    strings: &[String],
    property: Option<String>,
) -> Result<()> {
    let MemberTypes {
        params,
        result,
        receiver,
        type_parameters,
        flags,
    } = types;
    let mut all = Vec::new();
    if let Some(ty) = &receiver {
        all.push(ty.descriptor());
    }
    all.extend(params.iter().map(|(_, ty, _)| ty.descriptor()));
    let descriptor = format!("({}){}", all.concat(), result.descriptor());
    let (jvm_name, jvm_descriptor) = if let Some(bytes) = signature {
        let p = Proto::parse(bytes)?;
        p.allowed(&[1, 2])?;
        (
            p.integer(1)?
                .map(|i| metadata_name(strings, Some(i)))
                .transpose()?
                .unwrap_or(name),
            p.integer(2)?
                .map(|i| metadata_name(strings, Some(i)))
                .transpose()?
                .unwrap_or(&descriptor),
        )
    } else {
        (name, descriptor.as_str())
    };
    let matching = class
        .methods
        .iter()
        .enumerate()
        .filter(|(_, m)| m.name == jvm_name && m.descriptor == jvm_descriptor)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let [index] = matching.as_slice() else {
        return Err(invalid());
    };
    if jvm_descriptor != descriptor {
        return Err(invalid());
    }
    if let Some(signature) = &attributes[*index].signature {
        let class_parameters = class
            .library
            .as_ref()
            .map_or(&[][..], |c| c.type_parameters.as_slice());
        let (_, signature_params, signature_result) =
            signature::method(signature, class_parameters)?;
        let erased = format!(
            "({}){}",
            signature_params
                .iter()
                .map(Type::descriptor)
                .collect::<String>(),
            signature_result.descriptor()
        );
        if erased != jvm_descriptor {
            return Err(invalid());
        }
    }
    if params.iter().any(|(_, _, default)| *default) {
        if params.len() > 32 {
            return Err(invalid());
        }
        let original = &class.methods[*index];
        let closing = descriptor.find(')').ok_or_else(invalid)?;
        let parameters = &descriptor[1..closing];
        let (bridge_name, bridge_descriptor) = if name == "<init>" {
            (
                "<init>".into(),
                format!("({parameters}ILkotlin/jvm/internal/DefaultConstructorMarker;)V"),
            )
        } else {
            let dispatch = if original.access & 8 == 0 {
                format!("L{};", class.name)
            } else {
                String::new()
            };
            (
                format!("{jvm_name}$default"),
                format!(
                    "({dispatch}{parameters}ILjava/lang/Object;){}",
                    &descriptor[closing + 1..]
                ),
            )
        };
        if !class.methods.iter().any(|m| {
            m.name == bridge_name && m.descriptor == bridge_descriptor && m.access & 0x1000 != 0
        }) {
            return Err(invalid());
        }
    }
    let method = &mut class.methods[*index];
    if flags & (1 << 13 | 1 << 14 | 1 << 12) != 0
        || flags & 14 == 2
        || attributes[*index].unsupported
    {
        return Err(invalid());
    }
    method.kotlin_params = Some(params);
    method.kotlin_result = Some(result);
    method.kotlin_internal = flags & 14 == 0;
    method.source_property = property;
    method.unsupported = false;
    method.library = Some(LibraryMethod {
        source_name: name.into(),
        type_parameters,
        receiver,
        inline: flags & 1024 != 0,
        code: attributes[*index].code.clone(),
    });
    Ok(())
}

fn remember_callable(
    class: &mut JavaClass,
    name: &str,
    member: &Proto<'_>,
    strings: &[String],
) -> Result<()> {
    let signature = member.bytes(100)?.map(Proto::parse).transpose()?;
    let jvm_name = signature
        .as_ref()
        .map(|s| s.integer(1))
        .transpose()?
        .flatten()
        .map(|i| metadata_name(strings, Some(i)))
        .transpose()?
        .unwrap_or(name);
    let descriptor = signature
        .as_ref()
        .map(|s| s.integer(2))
        .transpose()?
        .flatten()
        .map(|i| metadata_name(strings, Some(i)))
        .transpose()?;
    for method in &mut class.methods {
        if method.name == jvm_name
            && descriptor.is_none_or(|d| method.descriptor == d)
            && method.library.is_none()
        {
            method.library = Some(LibraryMethod {
                source_name: name.into(),
                type_parameters: Vec::new(),
                receiver: None,
                inline: false,
                code: None,
            });
        }
    }
    Ok(())
}

pub(super) fn attach(
    annotation: &Annotation,
    class: &mut JavaClass,
    attributes: &[Attributes],
) -> Result<()> {
    let kind = integer(annotation, "k")?.unwrap_or(1);
    if !matches!(kind, 1 | 2 | 5) {
        return Err(invalid());
    }
    let version = annotation
        .values
        .iter()
        .find(|(k, _)| k == "mv")
        .map(|(_, v)| v);
    let Some(AnnotationValue::Array(version)) = version else {
        return Err(invalid());
    };
    let version = version
        .iter()
        .map(|v| match v {
            AnnotationValue::Integer(i) => Ok(*i),
            _ => Err(invalid()),
        })
        .collect::<Result<Vec<_>>>()?;
    if version.first() != Some(&2) || version.get(1).copied().unwrap_or(0) > 3 {
        return Err(invalid());
    }
    let bytes = decode(&strings(annotation, "d1")?)?;
    let mut r = Reader {
        bytes: &bytes,
        offset: 0,
    };
    let length = usize::try_from(varint(&mut r)?).map_err(|_| invalid())?;
    let strings = metadata_strings(r.take(length)?, &strings(annotation, "d2")?)?;
    let proto = Proto::parse(r.take(bytes.len() - r.offset)?)?;
    let mut ctx = Context::new(&strings, &proto)?;
    if kind == 1 && metadata_name(&strings, proto.integer(3)?)?.replace('.', "$") != class.name {
        return Err(invalid());
    }
    let parameters = if kind == 1 {
        ctx.type_parameters(&proto, 5)?
    } else {
        Vec::new()
    };
    let mut supertypes = if kind == 1 {
        proto
            .messages(6)?
            .into_iter()
            .map(|b| ctx.ty(b, 0))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    if kind == 1 {
        for i in numbers(&proto, 2)? {
            supertypes.push(ctx.ty(ctx.table.get(i as usize).ok_or_else(invalid)?, 0)?);
        }
    }
    if kind == 1 {
        let declared = supertypes
            .iter()
            .map(Type::descriptor)
            .filter(|d| d != "Ljava/lang/Object;")
            .collect::<Vec<_>>();
        let actual = class
            .super_name
            .iter()
            .chain(class.interfaces.iter())
            .filter(|n| n.as_str() != "java/lang/Object")
            .map(|n| format!("L{n};"))
            .collect::<Vec<_>>();
        if declared != actual {
            return Err(invalid());
        }
    }
    let package = if kind == 1 {
        None
    } else {
        Some(
            text(annotation, "pn")?
                .unwrap_or_else(|| {
                    class
                        .name
                        .rsplit_once('/')
                        .map_or("", |(p, _)| p)
                        .replace('/', ".")
                })
                .replace('.', "/"),
        )
    };
    let facade = if kind == 5 {
        text(annotation, "xs")?
    } else {
        None
    };
    class.library = Some(LibraryClass {
        type_parameters: parameters,
        supertypes,
        package,
        multifile_facade: facade.clone(),
    });
    class.kotlin_class = true;
    class.kotlin_interface = class.access & 0x200 != 0;
    let class_flags = proto.integer(1)?.unwrap_or(6);
    class.unsupported = kind == 1
        && (class_flags & 14 == 0
            || class_flags & (512 | 2048 | 4096 | 8192) != 0
            || !matches!((class_flags >> 6) & 7, 0 | 1));
    for method in &mut class.methods {
        method.unsupported = true;
    }
    for (field, constructor) in [(if kind == 1 { 9 } else { 3 }, false), (8, true)] {
        if constructor && kind != 1 {
            continue;
        }
        for bytes in proto.messages(field)? {
            let member = Proto::parse(bytes)?;
            let mut context = ctx.clone();
            let source_name = if constructor {
                "<init>"
            } else {
                metadata_name(&strings, member.integer(2)?)?
            };
            remember_callable(class, source_name, &member, &strings)?;
            let parsed = (|| -> Result<()> {
                context.set_table(&member)?;
                let tps = if constructor {
                    Vec::new()
                } else {
                    context.type_parameters(&member, 4)?
                };
                if !member.messages(10)?.is_empty() || !member.messages(13)?.is_empty() {
                    return Err(invalid());
                }
                let name = if constructor {
                    "<init>"
                } else {
                    metadata_name(&strings, member.integer(2)?)?
                };
                let flags = member
                    .integer(if constructor { 1 } else { 9 })?
                    .unwrap_or(6);
                let params = member
                    .messages(if constructor { 2 } else { 6 })?
                    .into_iter()
                    .map(|p| context.parameter(p))
                    .collect::<Result<Vec<_>>>()?;
                let result = if constructor {
                    Type::Unit
                } else {
                    context.field(&member, 3, 7)?
                };
                let receiver = if !constructor
                    && (member.bytes(5)?.is_some() || member.integer(8)?.is_some())
                {
                    Some(context.field(&member, 5, 8)?)
                } else {
                    None
                };
                attach_member(
                    class,
                    attributes,
                    name,
                    MemberTypes {
                        params,
                        result,
                        receiver,
                        type_parameters: tps,
                        flags,
                    },
                    member.bytes(100)?,
                    &strings,
                    None,
                )
            })();
            let _ = parsed;
        }
    }
    for bytes in proto.messages(if kind == 1 { 10 } else { 4 })? {
        let property = Proto::parse(bytes)?;
        let parsed = (|| -> Result<()> {
            let flags = property.integer(11)?.unwrap_or(518);
            if flags & (1 << 15 | 1 << 12 | 1 << 16) != 0 {
                return Err(invalid());
            }
            let name = metadata_name(&strings, property.integer(2)?)?;
            if name.is_empty() || !name.is_ascii() {
                return Err(invalid());
            }
            let ty = ctx.field(&property, 3, 9)?;
            let receiver = if property.bytes(5)?.is_some() || property.integer(10)?.is_some() {
                Some(ctx.field(&property, 5, 10)?)
            } else {
                None
            };
            let sig = Proto::parse(property.bytes(100)?.ok_or_else(invalid)?)?;
            let accessor_flags = |field| -> Result<u64> {
                let accessor = property.integer(field)?.unwrap_or(flags & 63);
                if accessor & !(63 | 64 | 256) != 0 {
                    return Err(invalid());
                }
                Ok(accessor & 63 | if accessor & 256 != 0 { 1024 } else { 0 })
            };
            if flags & 512 != 0 {
                attach_member(
                    class,
                    attributes,
                    &crate::check::getter_name(name),
                    MemberTypes {
                        params: Vec::new(),
                        result: ty.clone(),
                        receiver: receiver.clone(),
                        type_parameters: Vec::new(),
                        flags: accessor_flags(7)?,
                    },
                    sig.bytes(3)?,
                    &strings,
                    Some(name.into()),
                )?;
            }
            if flags & 1024 != 0 {
                attach_member(
                    class,
                    attributes,
                    &crate::check::setter_name(name),
                    MemberTypes {
                        params: vec![("value".into(), ty, false)],
                        result: Type::Unit,
                        receiver,
                        type_parameters: Vec::new(),
                        flags: accessor_flags(8)?,
                    },
                    sig.bytes(4)?,
                    &strings,
                    Some(name.into()),
                )?;
            }
            Ok(())
        })();
        let _ = parsed;
    }
    if let Some(owner) = class
        .library
        .as_ref()
        .and_then(|library| library.multifile_facade.clone())
    {
        for method in &mut class.methods {
            method.owner = owner.clone();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn var(mut value: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        while value >= 128 {
            bytes.push(value as u8 | 128);
            value >>= 7;
        }
        bytes.push(value as u8);
        bytes
    }
    fn int(number: u32, value: u64) -> Vec<u8> {
        [var(u64::from(number) << 3), var(value)].concat()
    }
    fn blob(number: u32, value: &[u8]) -> Vec<u8> {
        [
            var((u64::from(number) << 3) | 2),
            var(value.len() as u64),
            value.to_vec(),
        ]
        .concat()
    }
    #[test]
    fn module_package_remapping_preserves_source_and_jvm_packages() {
        let parts = [
            blob(1, b"p"),
            blob(2, b"PlainKt"),
            blob(5, b"HiddenKt"),
            blob(6, &[0]),
        ]
        .concat();
        let module = [blob(1, &parts), blob(3, b"impl")].concat();
        let mut bytes = [3u32, 2, 3, 0, 0]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>();
        bytes.extend(module);
        assert_eq!(
            module_parts(&bytes, "p").unwrap(),
            ["p/PlainKt", "impl/HiddenKt"]
        );
        assert!(module_parts(&bytes, "other").unwrap().is_empty());
    }
    #[test]
    fn cyclic_type_tables_are_bounded() {
        let argument = blob(2, &blob(2, &int(6, 0)));
        let direct = [int(6, 0), argument].concat();
        let strings = vec!["sample/Box".into()];
        let parameter = blob(2, &int(3, 0));
        let recursive = [int(6, 0), parameter].concat();
        let context = Context {
            strings: &strings,
            table: vec![&recursive],
            first_nullable: None,
            parameters: BTreeMap::new(),
        };
        assert!(context.ty(&direct, 0).is_ok());
        let reference = int(7, 0);
        assert!(
            context
                .field(&Proto::parse(&reference).unwrap(), 3, 7)
                .is_err()
        );
    }
}
