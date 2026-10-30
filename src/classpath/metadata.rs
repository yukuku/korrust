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
    qualified_names: Option<&'a [String]>,
    table: Vec<&'a [u8]>,
    first_nullable: Option<usize>,
    parameters: BTreeMap<u64, Type>,
}
impl<'a> Context<'a> {
    fn new(strings: &'a [String], proto: &Proto<'a>) -> Result<Self> {
        let mut result = Self {
            strings,
            qualified_names: None,
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
        p.allowed(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 14, 100])?;
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
            let name = metadata_name(self.qualified_names.unwrap_or(self.strings), p.integer(6)?)?;
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
                        | "kotlin/Double"
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
                "kotlin/Double" => Type::Double,
                "kotlin/String" => Type::String,
                "kotlin/Unit" => Type::Unit,
                "kotlin/IntArray" => Type::IntArray,
                "kotlin/ByteArray" => Type::ByteArray,
                "kotlin/CharArray" => Type::CharArray,
                "kotlin/LongArray" => Type::LongArray,
                "kotlin/BooleanArray" => Type::BooleanArray,
                "kotlin/FloatArray" => Type::FloatArray,
                "kotlin/DoubleArray" => Type::DoubleArray,

                "kotlin/Array" if arguments.len() == 1 && arguments[0].ty.is_some() => {
                    let inner = arguments.remove(0).ty.ok_or_else(invalid)?;
                    if inner == Type::String {
                        Type::StringArray
                    } else {
                        Type::Array(Box::new(inner))
                    }
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
                    let extension = p.messages(100)?.into_iter().any(|bytes| {
                        Proto::parse(bytes)
                            .ok()
                            .and_then(|annotation| annotation.integer(1).ok().flatten())
                            .and_then(|id| self.strings.get(id as usize))
                            .is_some_and(|name| name == "kotlin/ExtensionFunctionType")
                    });
                    if extension && !types.is_empty() {
                        Type::ReceiverFunction {
                            receiver: Box::new(types.remove(0)),
                            params: types,
                            result,
                        }
                    } else {
                        Type::Function {
                            params: types,
                            result,
                        }
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
        self.type_parameters_with_reified(proto, field, false)
    }
    fn type_parameters_with_reified(
        &mut self,
        proto: &Proto<'a>,
        field: u32,
        allow_reified: bool,
    ) -> Result<Vec<TypeParameter>> {
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
            let reified = p.integer(3)?.unwrap_or(0);
            if reified > 1 || (reified != 0 && !allow_reified) {
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
        p.allowed(&[1, 2, 3, 4, 5, 6])?;
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

fn builtin_qualified_names(strings: &[String], bytes: &[u8]) -> Result<Vec<String>> {
    let table = Proto::parse(bytes)?;
    table.allowed(&[1])?;
    let entries = table.messages(1)?;
    fn resolve(
        index: usize,
        entries: &[&[u8]],
        strings: &[String],
        depth: usize,
    ) -> Result<(String, u64)> {
        if depth > 64 {
            return Err(invalid());
        }
        let entry = Proto::parse(entries.get(index).ok_or_else(invalid)?)?;
        entry.allowed(&[1, 2, 3])?;
        let short = metadata_name(strings, entry.integer(2)?)?;
        if short.is_empty() || short.contains(['/', '.', '\0']) {
            return Err(invalid());
        }
        let kind = entry.integer(3)?.unwrap_or(1);
        if kind > 1 {
            return Err(invalid());
        }
        let name = match entry.integer(1)? {
            None | Some(u64::MAX) | Some(0xffff_ffff) => short.into(),
            Some(parent) => {
                let parent = usize::try_from(parent).map_err(|_| invalid())?;
                let (parent, parent_kind) = resolve(parent, entries, strings, depth + 1)?;
                if kind == 1 && parent_kind == 0 {
                    return Err(invalid());
                }
                format!(
                    "{parent}{}{short}",
                    if parent_kind == 0 { "." } else { "/" }
                )
            }
        };
        Ok((name, kind))
    }
    (0..entries.len())
        .map(|index| resolve(index, &entries, strings, 0).map(|(name, _)| name))
        .collect()
}

pub(super) fn builtin_array_factories(bytes: &[u8]) -> Result<Vec<JavaMethod>> {
    parse_builtin_array_factories(bytes).map_err(|error| {
        if error.message.contains("Kotlin builtins") {
            error
        } else {
            Diagnostic::new(
                error.offset,
                "Malformed or unsupported Kotlin builtins metadata resource",
            )
        }
    })
}

fn builtin_fragment(bytes: &[u8]) -> Result<(Vec<String>, Vec<String>, Proto<'_>)> {
    let mut reader = Reader { bytes, offset: 0 };
    let count = reader.u4()?;
    if count != 3 {
        return Err(Diagnostic::new(
            0,
            "Malformed or unsupported Kotlin builtins version header",
        ));
    }
    let version = [reader.u4()?, reader.u4()?, reader.u4()?];
    if version != [1, 0, 7] {
        return Err(Diagnostic::new(
            0,
            "Unsupported Kotlin builtins metadata version",
        ));
    }
    let fragment = Proto::parse(reader.take(bytes.len() - reader.offset)?)?;
    fragment.allowed(&[1, 2, 3, 4])?;
    let string_table = Proto::parse(fragment.bytes(1)?.ok_or_else(invalid)?)?;
    string_table.allowed(&[1])?;
    let strings = string_table
        .messages(1)?
        .into_iter()
        .map(|bytes| {
            std::str::from_utf8(bytes)
                .map(str::to_owned)
                .map_err(|_| invalid())
        })
        .collect::<Result<Vec<_>>>()?;
    let qualified_names =
        builtin_qualified_names(&strings, fragment.bytes(2)?.ok_or_else(invalid)?)?;
    Ok((strings, qualified_names, fragment))
}

fn parse_builtin_array_factories(bytes: &[u8]) -> Result<Vec<JavaMethod>> {
    let (strings, qualified_names, fragment) = builtin_fragment(bytes)?;
    let package = Proto::parse(fragment.bytes(3)?.ok_or_else(invalid)?)?;
    let mut context = Context::new(&strings, &package)?;
    context.qualified_names = Some(&qualified_names);
    let mut result = Vec::new();
    for bytes in package.messages(3)? {
        let function = Proto::parse(bytes)?;
        if function.bytes(5)?.is_some()
            || function.integer(8)?.is_some()
            || !function.messages(10)?.is_empty()
            || !numbers(&function, 11)?.is_empty()
            || !function.messages(13)?.is_empty()
        {
            continue;
        }
        let parameters = function.messages(6)?;
        let [parameter] = parameters.as_slice() else {
            continue;
        };
        let parameter = Proto::parse(parameter)?;
        if parameter.bytes(4)?.is_none() && parameter.integer(6)?.is_none() {
            continue;
        }
        let flags = function.integer(9)?.unwrap_or(6);
        if flags & 14 != 6
            || flags & !(65536 | 1024 | 6) != 0
            || parameter.integer(1)?.unwrap_or(0) != 0
        {
            continue;
        }
        let mut context = context.clone();
        context.set_table(&function)?;
        let type_parameters = context.type_parameters_with_reified(&function, 4, true)?;
        let array = context.field(&parameter, 3, 5)?;
        let element = context.field(&parameter, 4, 6)?;
        let returned = context.field(&function, 3, 7)?;
        let component = match &array {
            array if array.array_element().is_some() => array.array_element().unwrap(),
            _ => continue,
        };
        if component != element || returned != array {
            continue;
        }
        let name = metadata_name(&strings, function.integer(2)?)?.to_owned();
        let parameter_name = metadata_name(&strings, parameter.integer(2)?)?.to_owned();
        let descriptor = format!("({}){}", array.descriptor(), array.descriptor());
        result.push(JavaMethod {
            owner: "kotlin/<builtins>".into(),
            name: name.clone(),
            descriptor,
            access: 0x89,
            unsupported: false,
            source_property: None,
            kotlin_params: Some(vec![(parameter_name, array.clone(), false)]),
            kotlin_result: Some(array),
            kotlin_internal: false,
            library: Some(LibraryMethod {
                builtin_array: true,
                abstract_: false,
                kotlin_public: true,
                source_name: name,
                type_parameters,
                receiver: None,
                infix: false,
                operator: false,
                inline: flags & 1024 != 0,
                code: None,
            }),
        });
    }
    Ok(result)
}

pub(super) fn builtin_class_declarations(
    bytes: &[u8],
    classifier: &str,
) -> Result<Vec<JavaMethod>> {
    builtin_declarations(bytes, classifier, None)
}

pub(super) fn builtin_member_declarations(
    bytes: &[u8],
    classifier: &str,
    name: &str,
) -> Result<Vec<JavaMethod>> {
    builtin_declarations(bytes, classifier, Some(name))
}

fn builtin_declarations(
    bytes: &[u8],
    classifier: &str,
    requested_name: Option<&str>,
) -> Result<Vec<JavaMethod>> {
    let (strings, qualified_names, fragment) = builtin_fragment(bytes)?;
    let mut selected = None;
    for bytes in fragment.messages(4)? {
        let class = Proto::parse(bytes)?;
        let index =
            usize::try_from(class.integer(3)?.ok_or_else(invalid)?).map_err(|_| invalid())?;
        if qualified_names.get(index).ok_or_else(invalid)? != classifier {
            continue;
        }
        if selected.replace(class).is_some() {
            return Err(invalid());
        }
    }
    let class = selected.ok_or_else(|| {
        Diagnostic::new(0, format!("Builtin classifier not found: {classifier}")).with_kind(
            crate::DiagnosticKind::MissingDependency,
            "BUILTIN_CLASSIFIER_NOT_FOUND",
        )
    })?;
    if requested_name.is_none() && !class.messages(5)?.is_empty() {
        return Err(Diagnostic::new(
            0,
            "Generic builtin class member inventories are unsupported",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "BUILTIN_GENERIC_MEMBERS_UNSUPPORTED",
        ));
    }
    let mut context = Context::new(&strings, &class)?;
    context.qualified_names = Some(&qualified_names);
    context.type_parameters(&class, 5)?;
    let mut result = Vec::new();
    for bytes in class.messages(9)? {
        let function = Proto::parse(bytes)?;
        let name = metadata_name(&strings, function.integer(2)?)?;
        if requested_name.is_some_and(|requested| requested != name) {
            continue;
        }
        let flags = function.integer(9)?.unwrap_or(6);
        if flags & 14 != 6 {
            continue;
        }
        if !function.messages(4)?.is_empty()
            || function.bytes(5)?.is_some()
            || function.integer(8)?.is_some()
            || !function.messages(10)?.is_empty()
            || !numbers(&function, 11)?.is_empty()
            || !function.messages(13)?.is_empty()
        {
            return Err(Diagnostic::new(
                0,
                "Generic, extension, or context builtin member inventories are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "BUILTIN_MEMBER_SIGNATURE_UNSUPPORTED",
            ));
        }
        let mut context = context.clone();
        context.set_table(&function)?;
        let mut descriptor = String::from("(");
        let mut parameters = Vec::new();
        for parameter in function.messages(6)? {
            let parameter = Proto::parse(parameter)?;
            let ty = context.field(&parameter, 3, 5)?;
            if ty == Type::Unit {
                return Err(invalid());
            }
            descriptor.push_str(&ty.descriptor());
            parameters.push((
                metadata_name(&strings, parameter.integer(2)?)?.to_owned(),
                ty,
                parameter.integer(1)?.unwrap_or(0) & 2 != 0,
            ));
        }
        descriptor.push(')');
        let returned = context.field(&function, 3, 7)?;
        descriptor.push_str(&returned.descriptor());
        result.push(JavaMethod {
            owner: classifier.into(),
            name: name.to_owned(),
            descriptor,
            access: 1,
            unsupported: false,
            source_property: None,
            kotlin_params: Some(parameters),
            kotlin_result: Some(returned),
            kotlin_internal: false,
            library: requested_name.map(|_| LibraryMethod {
                abstract_: true,
                builtin_array: false,
                kotlin_public: true,
                source_name: name.to_owned(),
                type_parameters: Vec::new(),
                receiver: None,
                infix: flags & 512 != 0,
                operator: flags & 256 != 0,
                inline: false,
                code: None,
            }),
        });
    }
    result.sort_by(|a, b| (&a.name, &a.descriptor).cmp(&(&b.name, &b.descriptor)));
    if result
        .windows(2)
        .any(|pair| pair[0].name == pair[1].name && pair[0].descriptor == pair[1].descriptor)
    {
        return Err(invalid());
    }
    Ok(result)
}

#[cfg(test)]
pub(super) fn builtin_class_members(
    bytes: &[u8],
    classifier: &str,
) -> Result<Vec<(String, String)>> {
    Ok(builtin_class_declarations(bytes, classifier)?
        .into_iter()
        .map(|method| (method.name, method.descriptor))
        .collect())
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
        let parsed_signature = signature::method(signature, class_parameters);
        let (_, signature_params, signature_result) = parsed_signature?;
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
        if params.len() > 64 {
            return Err(invalid());
        }
        let original = &class.methods[*index];
        let closing = descriptor.find(')').ok_or_else(invalid)?;
        let parameters = &descriptor[1..closing];
        let (bridge_name, bridge_descriptor) = if name == "<init>" {
            (
                "<init>".into(),
                format!(
                    "({parameters}{}Lkotlin/jvm/internal/DefaultConstructorMarker;)V",
                    "I".repeat(params.len().div_ceil(32))
                ),
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
                    "({dispatch}{parameters}{}Ljava/lang/Object;){}",
                    "I".repeat((params.len() - usize::from(receiver.is_some())).div_ceil(32)),
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
        builtin_array: false,
        abstract_: flags & 0x30 == 32,
        kotlin_public: flags & 14 == 6,
        source_name: name.into(),
        type_parameters,
        receiver,
        infix: flags & 512 != 0,
        operator: flags & 256 != 0,
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
                builtin_array: false,
                abstract_: member
                    .integer(if name == "<init>" { 1 } else { 9 })?
                    .unwrap_or(6)
                    & 0x30
                    == 32,
                kotlin_public: member
                    .integer(if name == "<init>" { 1 } else { 9 })?
                    .unwrap_or(6)
                    & 14
                    == 6,
                source_name: name.into(),
                type_parameters: Vec::new(),
                receiver: None,
                infix: false,
                operator: false,
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
            .filter(|descriptor| {
                declared.contains(descriptor)
                    || !matches!(
                        descriptor.as_str(),
                        "Lkotlin/jvm/internal/markers/KMappedMarker;"
                            | "Lkotlin/jvm/internal/markers/KMutableIterable;"
                            | "Lkotlin/jvm/internal/markers/KMutableCollection;"
                            | "Lkotlin/jvm/internal/markers/KMutableList;"
                            | "Lkotlin/jvm/internal/markers/KMutableIterator;"
                            | "Lkotlin/jvm/internal/markers/KMutableListIterator;"
                            | "Lkotlin/jvm/internal/markers/KMutableMap;"
                            | "Lkotlin/jvm/internal/markers/KMutableMap$Entry;"
                            | "Lkotlin/jvm/internal/markers/KMutableSet;"
                    )
            })
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
    let mut type_aliases = Vec::new();
    if kind != 1 {
        for bytes in proto.messages(5)? {
            let alias = Proto::parse(bytes)?;
            let flags = alias.integer(1)?.unwrap_or(6);
            if flags & 14 != 6 {
                continue;
            }
            let mut context = ctx.clone();
            let parameters = context.type_parameters(&alias, 3)?;
            if let Ok(expanded) = context.field(&alias, 6, 7) {
                type_aliases.push(LibraryTypeAlias {
                    name: metadata_name(&strings, alias.integer(2)?)?.into(),
                    type_parameters: parameters,
                    expanded,
                });
            }
        }
    }
    let class_flags = proto.integer(1)?.unwrap_or(6);
    class.library = Some(LibraryClass {
        kotlin_visibility: (kind == 1).then_some(((class_flags >> 1) & 7) as u8),
        type_aliases,
        type_parameters: parameters,
        supertypes,
        package,
        multifile_facade: facade.clone(),
    });
    class.kotlin_class = true;
    class.kotlin_interface = class.access & 0x200 != 0;
    class.sealed_ = kind == 1 && class_flags & 48 == 48;
    class.sealed_subclasses = if class.sealed_ {
        numbers(&proto, 16)?
            .into_iter()
            .map(|id| {
                let name = metadata_name(&strings, Some(id))?.replace('.', "$");
                if !valid_name(&name) {
                    return Err(invalid());
                }
                Ok(name)
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    if class.sealed_subclasses.len() > 256 {
        return Err(invalid());
    }
    class.sealed_subclasses.sort();
    class.sealed_subclasses.dedup();

    class.kotlin_fun_interface = kind == 1 && class.kotlin_interface && class_flags & 16384 != 0;
    class.unsupported = kind == 1
        && (class_flags & 14 == 0
            || class_flags & (512 | 2048 | 4096 | 8192) != 0
            || !matches!((class_flags >> 6) & 7, 0 | 1 | 5 | 6));
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
        let mut context = ctx.clone();
        let parsed = (|| -> Result<()> {
            let type_parameters = context.type_parameters(&property, 4)?;
            let unsupported_parameter_shadowing = kind == 1
                && type_parameters.iter().any(|parameter| {
                    ctx.parameters.values().any(
                        |ty| matches!(ty, Type::Parameter { name, .. } if name == &parameter.name),
                    )
                });
            if unsupported_parameter_shadowing {
                return Err(invalid());
            }
            let flags = property.integer(11)?.unwrap_or(518);
            if flags & (1 << 16) != 0 {
                return Err(invalid());
            }
            let name = metadata_name(&strings, property.integer(2)?)?;
            if name.is_empty() || !name.is_ascii() {
                return Err(invalid());
            }
            let ty = context.field(&property, 3, 9)?;
            let receiver = if property.bytes(5)?.is_some() || property.integer(10)?.is_some() {
                Some(context.field(&property, 5, 10)?)
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
                        type_parameters: type_parameters.clone(),
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
                        type_parameters,
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
    fn builtin_fixture(name: &str, vararg: bool, matching_result: bool, receiver: bool) -> Vec<u8> {
        let strings = ["kotlin", "Int", "IntArray", name, "elements"]
            .into_iter()
            .flat_map(|text| blob(1, text.as_bytes()))
            .collect::<Vec<_>>();
        let package_name = int(2, 0);
        let int_name = [int(1, 0), int(2, 1), int(3, 0)].concat();
        let array_name = [int(1, 0), int(2, 2), int(3, 0)].concat();
        let names = [
            blob(1, &package_name),
            blob(1, &int_name),
            blob(1, &array_name),
        ]
        .concat();
        let element = int(6, 1);
        let array = int(6, 2);
        let mut parameter = [int(2, 4), blob(3, &array)].concat();
        if vararg {
            parameter.extend(blob(4, &element));
        }
        let mut function = [
            int(2, 3),
            blob(3, if matching_result { &array } else { &element }),
            blob(6, &parameter),
        ]
        .concat();
        if receiver {
            function.extend(blob(5, &element));
        }
        let fragment = [
            blob(1, &strings),
            blob(2, &names),
            blob(3, &blob(3, &function)),
        ]
        .concat();
        [
            [3u32, 1, 0, 7]
                .into_iter()
                .flat_map(u32::to_be_bytes)
                .collect::<Vec<_>>(),
            fragment,
        ]
        .concat()
    }

    #[test]
    fn builtin_class_inventory_uses_declared_names_and_signatures() {
        let strings = [
            "kotlin",
            "Probe",
            "Int",
            "Boolean",
            "argument",
            "arbitraryMember",
        ]
        .into_iter()
        .flat_map(|value| blob(1, value.as_bytes()))
        .collect::<Vec<_>>();
        let names = [
            blob(1, &int(2, 0)),
            blob(1, &[int(1, 0), int(2, 1), int(3, 0)].concat()),
            blob(1, &[int(1, 0), int(2, 2), int(3, 0)].concat()),
            blob(1, &[int(1, 0), int(2, 3), int(3, 0)].concat()),
        ]
        .concat();
        let function = [
            int(2, 5),
            blob(3, &int(6, 3)),
            blob(6, &[int(2, 4), blob(3, &int(6, 2))].concat()),
        ]
        .concat();
        let class = [int(3, 1), blob(9, &function)].concat();
        let header = [3u32, 1, 0, 7]
            .into_iter()
            .flat_map(u32::to_be_bytes)
            .collect::<Vec<_>>();
        let fixture = [
            header.clone(),
            blob(1, &strings),
            blob(2, &names),
            blob(4, &class),
        ]
        .concat();
        assert_eq!(
            builtin_class_members(&fixture, "kotlin/Probe").unwrap(),
            [("arbitraryMember".into(), "(I)Z".into())]
        );
        assert_eq!(
            builtin_class_members(&fixture, "kotlin/Missing")
                .unwrap_err()
                .code,
            "BUILTIN_CLASSIFIER_NOT_FOUND"
        );
        let generic = [
            header,
            blob(1, &strings),
            blob(2, &names),
            blob(4, &[class, blob(5, &[])].concat()),
        ]
        .concat();
        assert_eq!(
            builtin_class_members(&generic, "kotlin/Probe")
                .unwrap_err()
                .code,
            "BUILTIN_GENERIC_MEMBERS_UNSUPPORTED"
        );
        assert!(builtin_class_members(&fixture[..8], "kotlin/Probe").is_err());
    }

    #[test]
    fn pinned_any_member_inventory_matches_its_jvm_contract() {
        let standard = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        let bytes = archive::read(&standard, "kotlin/kotlin.kotlin_builtins")
            .unwrap()
            .unwrap();
        assert_eq!(
            builtin_class_members(&bytes, "kotlin/Any").unwrap(),
            [
                ("equals".into(), "(Ljava/lang/Object;)Z".into()),
                ("hashCode".into(), "()I".into()),
                ("toString".into(), "()Ljava/lang/String;".into()),
            ]
        );
        let declarations = builtin_class_declarations(&bytes, "kotlin/Any").unwrap();
        let equality = declarations
            .iter()
            .find(|method| method.name == "equals")
            .unwrap();
        assert_eq!(
            equality.kotlin_params.as_ref().unwrap(),
            &[(
                "other".into(),
                Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
                false
            )]
        );
        assert_eq!(
            declarations
                .iter()
                .find(|method| method.name == "toString")
                .unwrap()
                .kotlin_result,
            Some(Type::String)
        );
    }

    #[test]
    fn pinned_builtin_resource_exposes_supported_array_declarations() {
        let standard = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        let bytes = archive::read(&standard, "kotlin/kotlin.kotlin_builtins")
            .unwrap()
            .unwrap();
        let methods = builtin_array_factories(&bytes).unwrap();
        assert_eq!(
            methods
                .iter()
                .map(|method| method.name.as_str())
                .collect::<Vec<_>>(),
            [
                "arrayOf",
                "booleanArrayOf",
                "byteArrayOf",
                "charArrayOf",
                "doubleArrayOf",
                "floatArrayOf",
                "intArrayOf",
                "longArrayOf"
            ]
        );
        for method in &methods {
            assert_eq!(method.owner, "kotlin/<builtins>");
            assert_eq!(method.access, 0x89);
            assert!(method.library.as_ref().unwrap().builtin_array);
            assert_eq!(method.kotlin_params.as_ref().unwrap()[0].0, "elements");
            assert_eq!(
                method.kotlin_params.as_ref().unwrap()[0].1,
                *method.kotlin_result.as_ref().unwrap()
            );
        }
        assert_eq!(
            methods[0].library.as_ref().unwrap().type_parameters[0].name,
            "T"
        );
        assert!(
            matches!(methods[0].kotlin_result.as_ref().unwrap(), Type::Array(element)
            if matches!(element.as_ref(), Type::Parameter { name, .. } if name == "T"))
        );
        assert_eq!(
            methods
                .iter()
                .find(|method| method.name == "intArrayOf")
                .unwrap()
                .descriptor,
            "([I)[I"
        );
    }

    #[test]
    fn builtin_array_declarations_are_selected_by_shape_and_not_name() {
        let methods =
            builtin_array_factories(&builtin_fixture("renamedCollector", true, true, false))
                .unwrap();
        assert_eq!(methods.len(), 1);
        assert_eq!(methods[0].name, "renamedCollector");
        assert_eq!(methods[0].descriptor, "([I)[I");
        for (vararg, matching_result, receiver) in [
            (false, true, false),
            (true, false, false),
            (true, true, true),
        ] {
            assert!(
                builtin_array_factories(&builtin_fixture(
                    "arrayOf",
                    vararg,
                    matching_result,
                    receiver
                ))
                .unwrap()
                .is_empty()
            );
        }
    }

    #[test]
    fn builtin_metadata_headers_and_resources_are_checked() {
        let valid = builtin_fixture("renamed", true, true, false);
        for end in 0..valid.len() {
            if let Err(error) = builtin_array_factories(&valid[..end]) {
                assert!(error.message.contains("Kotlin builtins"));
            }
        }
        let mut wrong_version = valid.clone();
        wrong_version[15] = 8;
        assert!(builtin_array_factories(&wrong_version).is_err());
        let mut wrong_count = valid.clone();
        wrong_count[3] = 17;
        assert!(builtin_array_factories(&wrong_count).is_err());
        let mut wrong_wire = valid;
        wrong_wire[16] = 15;
        assert!(builtin_array_factories(&wrong_wire).is_err());
    }

    #[test]
    fn qualified_builtin_names_distinguish_packages_classes_and_cycles() {
        let strings = vec!["kotlin".into(), "Outer".into(), "Inner".into()];
        let table = [
            blob(1, &int(2, 0)),
            blob(1, &[int(1, 0), int(2, 1), int(3, 0)].concat()),
            blob(1, &[int(1, 1), int(2, 2), int(3, 0)].concat()),
        ]
        .concat();
        assert_eq!(
            builtin_qualified_names(&strings, &table).unwrap(),
            ["kotlin", "kotlin/Outer", "kotlin/Outer.Inner"]
        );
        let cycle = blob(1, &[int(1, 0), int(2, 0)].concat());
        assert!(builtin_qualified_names(&strings, &cycle).is_err());
        let missing = blob(1, &[int(1, 10), int(2, 0)].concat());
        assert!(builtin_qualified_names(&strings, &missing).is_err());
    }

    #[test]
    fn ordinary_metadata_still_rejects_reified_type_parameters() {
        let strings = vec!["T".into()];
        let bytes = blob(4, &[int(1, 0), int(2, 0), int(3, 1)].concat());
        let proto = Proto::parse(&bytes).unwrap();
        let mut context = Context::new(&strings, &proto).unwrap();
        assert!(context.type_parameters(&proto, 4).is_err());
    }

    fn metadata_annotation(kind: u32, strings: &[&str], proto: &[u8]) -> Annotation {
        let table = blob(1, &int(1, strings.len() as u64));
        let bytes = [var(table.len() as u64), table, proto.to_vec()].concat();
        let encoded = std::iter::once('\0')
            .chain(bytes.into_iter().map(char::from))
            .collect::<String>();
        Annotation {
            name: "Lkotlin/Metadata;".into(),
            values: vec![
                ("k".into(), AnnotationValue::Integer(kind)),
                (
                    "mv".into(),
                    AnnotationValue::Array(vec![
                        AnnotationValue::Integer(2),
                        AnnotationValue::Integer(3),
                        AnnotationValue::Integer(0),
                    ]),
                ),
                (
                    "d1".into(),
                    AnnotationValue::Array(vec![AnnotationValue::String(encoded)]),
                ),
                (
                    "d2".into(),
                    AnnotationValue::Array(
                        strings
                            .iter()
                            .map(|text| AnnotationValue::String((*text).into()))
                            .collect(),
                    ),
                ),
            ],
        }
    }

    fn bare_metadata_class(name: &str, access: u16) -> JavaClass {
        JavaClass {
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            library: None,
            name: name.into(),
            access,
            super_name: Some("java/lang/Object".into()),
            interfaces: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            kotlin_class: false,
            unsupported: false,
            kotlin_interface: false,
            kotlin_fun_interface: false,
        }
    }

    #[test]
    fn metadata_decodes_functional_interface_identity_and_abstract_member_modality() {
        for (fun, abstract_) in [(false, true), (true, true), (true, false)] {
            let function = [
                int(2, 2),
                blob(3, &int(6, 3)),
                int(9, 6 | if abstract_ { 32 } else { 16 }),
            ]
            .concat();
            let proto = [
                int(1, 102 | if fun { 16384 } else { 0 }),
                int(3, 0),
                blob(6, &int(6, 1)),
                blob(9, &function),
            ]
            .concat();
            let annotation = metadata_annotation(
                1,
                &["sample/Action", "kotlin/Any", "run", "kotlin/Unit"],
                &proto,
            );
            let mut class = bare_metadata_class("sample/Action", 0x601);
            class.methods.push(JavaMethod {
                library: None,
                owner: class.name.clone(),
                name: "run".into(),
                descriptor: "()V".into(),
                access: 0x401,
                unsupported: false,
                source_property: None,
                kotlin_params: None,
                kotlin_result: None,
                kotlin_internal: false,
            });
            attach(&annotation, &mut class, &[Attributes::default()]).unwrap();
            assert_eq!(class.kotlin_fun_interface, fun);
            assert_eq!(
                class.methods[0].library.as_ref().unwrap().abstract_,
                abstract_
            );
        }
    }

    #[test]
    fn metadata_exposes_public_aliases_and_keeps_private_aliases_out_of_library_index() {
        let visible = [
            int(1, 6),
            int(2, 0),
            blob(4, &int(6, 2)),
            blob(6, &int(6, 2)),
        ]
        .concat();
        let hidden = [
            int(1, 2),
            int(2, 1),
            blob(4, &int(6, 2)),
            blob(6, &int(6, 2)),
        ]
        .concat();
        let proto = [blob(5, &visible), blob(5, &hidden)].concat();
        let annotation = metadata_annotation(2, &["Visible", "Hidden", "kotlin/String"], &proto);
        let mut class = bare_metadata_class("sample/AliasesKt", 0x31);
        attach(&annotation, &mut class, &[]).unwrap();
        let aliases = &class.library.as_ref().unwrap().type_aliases;
        assert_eq!(aliases.len(), 1);
        assert_eq!(aliases[0].name, "Visible");
        assert_eq!(aliases[0].expanded, Type::String);
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
            qualified_names: None,
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
