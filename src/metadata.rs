use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Diagnostic, Result,
    syntax::{ClassDecl, CompanionDecl, CompanionValue, Function, InterfaceDecl, Module, Type},
};

pub(crate) struct Metadata {
    pub kind: i32,
    pub data1: Vec<Vec<u16>>,
    pub data2: Vec<String>,
}

#[derive(Clone, Default)]
struct Proto(Vec<u8>);

impl Proto {
    fn varint(&mut self, mut value: u64) {
        while value >= 128 {
            self.0.push(value as u8 | 128);
            value >>= 7;
        }
        self.0.push(value as u8);
    }

    fn number(&mut self, field: u32, value: u32) {
        self.varint(u64::from(field) << 3);
        self.varint(u64::from(value));
    }

    fn bytes(&mut self, field: u32, value: &[u8]) {
        self.varint((u64::from(field) << 3) | 2);
        self.varint(value.len() as u64);
        self.0.extend_from_slice(value);
    }

    fn message(&mut self, field: u32, value: &Self) {
        self.bytes(field, &value.0);
    }
}

#[derive(Default)]
struct Strings {
    values: Vec<String>,
    indices: BTreeMap<String, u32>,
    local_names: Vec<u32>,
}

impl Strings {
    fn id(&mut self, value: &str) -> Result<u32> {
        if let Some(id) = self.indices.get(value) {
            return Ok(*id);
        }
        let length: usize = value
            .encode_utf16()
            .map(|unit| match unit {
                1..=127 => 1,
                0..=2047 => 2,
                _ => 3,
            })
            .sum();
        if length > u16::MAX as usize {
            return Err(Diagnostic::new(
                0,
                "Kotlin metadata string exceeds JVM UTF-8 limit",
            ));
        }
        if self.values.len() >= u16::MAX as usize {
            return Err(Diagnostic::new(
                0,
                "Kotlin metadata string table exceeds JVM annotation limit",
            ));
        }
        let id = self.values.len() as u32;
        self.values.push(value.into());
        self.indices.insert(value.into(), id);
        Ok(id)
    }

    fn finish(self, kind: i32, proto: Proto) -> Result<Metadata> {
        let mut record = Proto::default();
        record.number(1, self.values.len() as u32);
        let mut table = Proto::default();
        if !self.values.is_empty() {
            table.message(1, &record);
        }
        if !self.local_names.is_empty() {
            let mut names = Proto::default();
            for index in &self.local_names {
                names.varint(u64::from(*index));
            }
            table.bytes(5, &names.0);
        }
        let mut data = Proto::default();
        data.varint(table.0.len() as u64);
        data.0.extend(table.0);
        data.0.extend(proto.0);
        let data1 = encode_bytes(&data.0);
        if data1.len() > u16::MAX as usize {
            return Err(Diagnostic::new(
                0,
                "Kotlin metadata exceeds JVM annotation limit",
            ));
        }
        Ok(Metadata {
            kind,
            data1,
            data2: self.values,
        })
    }
}

fn encode_bytes(bytes: &[u8]) -> Vec<Vec<u16>> {
    let mut chunks = Vec::new();
    let mut chunk = vec![0];
    let mut encoded_length = 2;
    for &byte in bytes {
        let length = if (1..=127).contains(&byte) { 1 } else { 2 };
        if encoded_length + length > u16::MAX as usize {
            chunks.push(std::mem::take(&mut chunk));
            encoded_length = 0;
        }
        chunk.push(u16::from(byte));
        encoded_length += length;
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}

type Imports<'a> = &'a [(String, String, usize)];

fn imported_name<'a>(name: &str, imports: Imports<'a>) -> Result<&'a str> {
    imports
        .iter()
        .find(|(alias, _, _)| alias == name)
        .map(|(_, name, _)| name.as_str())
        .ok_or_else(|| {
            Diagnostic::new(
                0,
                format!("Metadata type requires an explicit import: {name}"),
            )
        })
}

fn descriptor(ty: &Type, imports: Imports<'_>) -> Result<String> {
    match ty {
        Type::Nullable(inner) => descriptor(inner, imports),
        Type::Java(name) => Ok(format!("L{};", imported_name(name, imports)?)),
        Type::Platform(_) => Err(Diagnostic::new(
            0,
            "Platform types in source metadata are unsupported",
        )),
        _ => Ok(ty.descriptor()),
    }
}

fn type_proto(ty: &Type, imports: Imports<'_>, strings: &mut Strings) -> Result<Proto> {
    if let Type::Nullable(inner) = ty {
        let mut proto = type_proto(inner, imports, strings)?;
        proto.number(3, 1);
        return Ok(proto);
    }
    let mut proto = Proto::default();
    let name = match ty {
        Type::Int => "kotlin/Int",
        Type::Long => "kotlin/Long",
        Type::Null => "kotlin/Nothing",
        Type::Nullable(_) => unreachable!(),
        Type::List(inner) => {
            let mut argument = Proto::default();
            argument.message(2, &type_proto(inner, imports, strings)?);
            proto.message(2, &argument);
            "kotlin/collections/List"
        }
        Type::Byte => "kotlin/Byte",
        Type::Char => "kotlin/Char",
        Type::Boolean => "kotlin/Boolean",
        Type::Float => "kotlin/Float",
        Type::IntArray => "kotlin/IntArray",
        Type::ByteArray => "kotlin/ByteArray",
        Type::CharArray => "kotlin/CharArray",
        Type::String => "kotlin/String",
        Type::Unit => "kotlin/Unit",
        Type::StringArray => {
            let mut argument = Proto::default();
            argument.message(2, &type_proto(&Type::String, imports, strings)?);
            proto.message(2, &argument);
            "kotlin/Array"
        }
        Type::ThreadLocal(inner) => {
            let mut argument = Proto::default();
            argument.message(2, &type_proto(inner, imports, strings)?);
            proto.message(2, &argument);
            "java/lang/ThreadLocal"
        }
        Type::Java(name) => {
            let name = imported_name(name, imports)?.replace('$', ".");
            proto.number(6, strings.id(&name)?);
            return Ok(proto);
        }
        Type::Platform(_) => {
            return Err(Diagnostic::new(
                0,
                "Platform types in source metadata are unsupported",
            ));
        }
    };
    proto.number(6, strings.id(name)?);
    Ok(proto)
}

fn parameter(name: &str, ty: &Type, imports: Imports<'_>, strings: &mut Strings) -> Result<Proto> {
    let mut parameter = Proto::default();
    parameter.number(2, strings.id(name)?);
    parameter.message(3, &type_proto(ty, imports, strings)?);
    Ok(parameter)
}

fn method_signature(name: &str, desc: &str, strings: &mut Strings) -> Result<Proto> {
    let mut signature = Proto::default();
    signature.number(1, strings.id(name)?);
    signature.number(2, strings.id(desc)?);
    Ok(signature)
}

fn function_proto(
    function: &Function,
    is_override: bool,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<Proto> {
    function_with_modality(function, if is_override { 16 } else { 0 }, imports, strings)
}

fn function_with_modality(
    function: &Function,
    modality: u32,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<Proto> {
    function_with_flags(function, modality, 0, imports, strings)
}

fn function_with_flags(
    function: &Function,
    modality: u32,
    extra_flags: u32,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<Proto> {
    let mut proto = Proto::default();
    let visibility = if function.private {
        2
    } else if function.internal {
        0
    } else {
        6
    };
    proto.number(9, visibility | modality | extra_flags);
    proto.number(2, strings.id(&function.name)?);
    proto.message(3, &type_proto(&function.result, imports, strings)?);
    let mut desc = String::from("(");
    if let Some(receiver) = &function.receiver {
        proto.message(5, &type_proto(receiver, imports, strings)?);
        desc.push_str(&descriptor(receiver, imports)?);
    }
    for (name, ty) in &function.params {
        proto.message(6, &parameter(name, ty, imports, strings)?);
        desc.push_str(&descriptor(ty, imports)?);
    }
    desc.push(')');
    desc.push_str(&descriptor(&function.result, imports)?);
    proto.message(100, &method_signature(&function.name, &desc, strings)?);
    Ok(proto)
}

pub(crate) fn for_class(
    class: &ClassDecl,
    internal_name: &str,
    imports: Imports<'_>,
) -> Result<Metadata> {
    class_metadata(class, internal_name, imports, false)
}

fn class_metadata(
    class: &ClassDecl,
    internal_name: &str,
    imports: Imports<'_>,
    anonymous: bool,
) -> Result<Metadata> {
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(
        1,
        (if anonymous {
            10
        } else if class.internal {
            0
        } else {
            6
        }) | if class.data { 1024 } else { 0 },
    );
    let class_id = if anonymous {
        internal_name.into()
    } else {
        internal_name.replace('$', ".")
    };
    let name = strings.id(&class_id)?;
    if anonymous {
        strings.local_names.push(name);
    }
    proto.number(3, name);
    if class.interfaces.is_empty() {
        let mut supertype = Proto::default();
        supertype.number(6, strings.id("kotlin/Any")?);
        proto.message(6, &supertype);
    } else {
        for interface in &class.interfaces {
            let mut supertype = Proto::default();
            let name = imported_name(interface, imports)?.replace('$', ".");
            supertype.number(6, strings.id(&name)?);
            proto.message(6, &supertype);
        }
    }
    if !anonymous {
        let mut constructor = Proto::default();
        constructor.number(1, 6);
        let mut constructor_desc = String::from("(");
        for (name, ty) in &class.params {
            constructor.message(2, &parameter(name, ty, imports, &mut strings)?);
            constructor_desc.push_str(&descriptor(ty, imports)?);
        }
        constructor_desc.push_str(")V");
        constructor.message(
            100,
            &method_signature("<init>", &constructor_desc, &mut strings)?,
        );
        proto.message(8, &constructor);
    }
    for method in &class.methods {
        proto.message(
            9,
            &function_proto(&method.function, method.is_override, imports, &mut strings)?,
        );
    }
    for (name, ty) in &class.params {
        let mut property = Proto::default();
        let public = class.public_params.contains(name);
        property.number(11, if public { 518 } else { 514 });
        property.number(2, strings.id(name)?);
        property.message(3, &type_proto(ty, imports, &mut strings)?);
        let mut signature = Proto::default();
        signature.message(
            1,
            &method_signature(name, &descriptor(ty, imports)?, &mut strings)?,
        );
        if public {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(name),
                    &format!("(){}", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
        }
        property.message(100, &signature);
        proto.message(10, &property);
    }
    if class.data {
        for (index, (_, ty)) in class.params.iter().enumerate() {
            let name = format!("component{}", index + 1);
            let mut function = Proto::default();
            function.number(9, 454);
            function.number(2, strings.id(&name)?);
            function.message(3, &type_proto(ty, imports, &mut strings)?);
            function.message(
                100,
                &method_signature(
                    &name,
                    &format!("(){}", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
            proto.message(9, &function);
        }
        let mut copy = Proto::default();
        copy.number(9, 198);
        copy.number(2, strings.id("copy")?);
        let mut result = Proto::default();
        result.number(6, strings.id(&internal_name.replace('$', "."))?);
        copy.message(3, &result);
        let mut desc = String::from("(");
        for (name, ty) in &class.params {
            let mut param = parameter(name, ty, imports, &mut strings)?;
            param.number(1, 2);
            copy.message(6, &param);
            desc.push_str(&descriptor(ty, imports)?);
        }
        desc.push_str(&format!(")L{internal_name};"));
        copy.message(100, &method_signature("copy", &desc, &mut strings)?);
        proto.message(9, &copy);
        for (name, result, flags) in [
            ("equals", Type::Boolean, 66006),
            ("hashCode", Type::Int, 65750),
            ("toString", Type::String, 65750),
        ] {
            let mut function = Proto::default();
            function.number(9, flags);
            function.number(2, strings.id(name)?);
            function.message(3, &type_proto(&result, imports, &mut strings)?);
            let desc = if name == "equals" {
                let mut any = Proto::default();
                any.number(6, strings.id("kotlin/Any")?);
                any.number(3, 1);
                let mut parameter = Proto::default();
                parameter.number(2, strings.id("other")?);
                parameter.message(3, &any);
                function.message(6, &parameter);
                "(Ljava/lang/Object;)Z".into()
            } else {
                format!("(){}", result.descriptor())
            };
            function.message(100, &method_signature(name, &desc, &mut strings)?);
            proto.message(9, &function);
        }
    }
    for property in &class.properties {
        let mut declaration = Proto::default();
        let flags = 6 | if property.is_override { 16 } else { 0 };
        declaration.number(11, flags | 512);
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(&property.ty, imports, &mut strings)?);
        declaration.number(7, flags | 64);
        let mut signature = Proto::default();
        signature.message(
            3,
            &method_signature(
                &crate::check::getter_name(&property.name),
                &format!("(){}", descriptor(&property.ty, imports)?),
                &mut strings,
            )?,
        );
        declaration.message(100, &signature);
        proto.message(10, &declaration);
    }
    strings.finish(1, proto)
}

fn interface_metadata(
    interface: &InterfaceDecl,
    owner: &str,
    header_imports: Imports<'_>,
    imports: Imports<'_>,
) -> Result<Metadata> {
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(1, if interface.internal { 96 } else { 102 });
    proto.number(3, strings.id(&owner.replace('$', "."))?);
    if interface.interfaces.is_empty() {
        let mut supertype = Proto::default();
        supertype.number(6, strings.id("kotlin/Any")?);
        proto.message(6, &supertype);
    } else {
        for name in &interface.interfaces {
            proto.message(
                6,
                &type_proto(&Type::Java(name.clone()), header_imports, &mut strings)?,
            );
        }
    }
    for nested in &interface.nested {
        proto.number(7, strings.id(&nested.name)?);
    }
    if interface.companion.is_some() {
        let name = strings.id("Companion")?;
        proto.number(4, name);
        proto.number(7, name);
    }
    for method in &interface.methods {
        proto.message(
            9,
            &function_with_modality(
                &method.function,
                if method.has_body { 16 } else { 32 },
                imports,
                &mut strings,
            )?,
        );
    }
    for property in &interface.properties {
        let mut declaration = Proto::default();
        let flags = 6 | if property.body.is_some() { 16 } else { 32 };
        declaration.number(
            11,
            flags | 512 | if property.mutable { 256 | 1024 } else { 0 },
        );
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(&property.ty, imports, &mut strings)?);
        if property.body.is_some() {
            declaration.number(7, flags | 64);
        }
        let mut signature = Proto::default();
        signature.message(
            3,
            &method_signature(
                &crate::check::getter_name(&property.name),
                &format!("(){}", descriptor(&property.ty, imports)?),
                &mut strings,
            )?,
        );
        if property.mutable {
            signature.message(
                4,
                &method_signature(
                    &crate::check::setter_name(&property.name),
                    &format!("({})V", descriptor(&property.ty, imports)?),
                    &mut strings,
                )?,
            );
        }
        declaration.message(100, &signature);
        proto.message(10, &declaration);
    }
    proto.number(104, 3);
    strings.finish(1, proto)
}

fn companion_metadata(
    companion: &CompanionDecl,
    owner: &str,
    imports: Imports<'_>,
) -> Result<Metadata> {
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(1, 390);
    proto.number(3, strings.id(&owner.replace('$', "."))?);
    let mut supertype = Proto::default();
    supertype.number(6, strings.id("kotlin/Any")?);
    proto.message(6, &supertype);
    let mut constructor = Proto::default();
    constructor.number(1, 2);
    constructor.message(100, &method_signature("<init>", "()V", &mut strings)?);
    proto.message(8, &constructor);
    for property in &companion.properties {
        let ty = match &property.value {
            CompanionValue::StringArray(_) => Type::StringArray,
            CompanionValue::Object(class) => {
                if property.private || class.interfaces.len() != 1 {
                    return Err(Diagnostic::new(
                        property.offset,
                        "Anonymous companion properties require public visibility and one interface",
                    ));
                }
                Type::Java(class.interfaces[0].clone())
            }
        };
        let mut declaration = Proto::default();
        declaration.number(11, if property.private { 514 } else { 518 });
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(&ty, imports, &mut strings)?);
        let mut signature = Proto::default();
        signature.message(
            1,
            &method_signature(&property.name, &descriptor(&ty, imports)?, &mut strings)?,
        );
        if !property.private {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(&property.name),
                    &format!("(){}", descriptor(&ty, imports)?),
                    &mut strings,
                )?,
            );
        }
        declaration.message(100, &signature);
        proto.message(10, &declaration);
    }
    strings.finish(1, proto)
}

fn interface_tree(
    interface: &InterfaceDecl,
    owner: &str,
    imports: Imports<'_>,
    result: &mut BTreeMap<String, Metadata>,
) -> Result<()> {
    let mut bindings: Vec<_> = interface
        .nested
        .iter()
        .map(|nested| {
            (
                nested.name.clone(),
                format!("{owner}${}", nested.name),
                nested.offset,
            )
        })
        .collect();
    bindings.extend_from_slice(imports);
    result.insert(
        owner.into(),
        interface_metadata(interface, owner, imports, &bindings)?,
    );
    if let Some(companion) = &interface.companion {
        let companion_owner = format!("{owner}$Companion");
        result.insert(
            companion_owner.clone(),
            companion_metadata(companion, &companion_owner, &bindings)?,
        );
        for property in &companion.properties {
            if let CompanionValue::Object(class) = &property.value {
                let owner = format!("{companion_owner}${}$1", property.name);
                result.insert(
                    owner.clone(),
                    class_metadata(class, &owner, &bindings, true)?,
                );
            }
        }
    }
    for nested in &interface.nested {
        interface_tree(
            nested,
            &format!("{owner}${}", nested.name),
            &bindings,
            result,
        )?;
    }
    Ok(())
}

pub(crate) fn for_interfaces(module: &Module, owner: &str) -> Result<BTreeMap<String, Metadata>> {
    let interface = module.interface_decl.as_ref().ok_or_else(|| {
        Diagnostic::new(0, "Interface metadata requires an interface declaration")
    })?;
    let mut result = BTreeMap::new();
    let mut imports = vec![(interface.name.clone(), owner.into(), interface.offset)];
    imports.extend_from_slice(&module.imports);
    interface_tree(interface, owner, &imports, &mut result)?;
    Ok(result)
}

fn platform_type(mut lower: Proto, strings: &mut Strings) -> Result<Proto> {
    let mut upper = lower.clone();
    upper.number(3, 1);
    lower.number(4, strings.id("kotlin.jvm.PlatformType")?);
    lower.message(5, &upper);
    Ok(lower)
}

fn inferred_thread_local(ty: &Type, imports: Imports<'_>, strings: &mut Strings) -> Result<Proto> {
    let Type::ThreadLocal(inner) = ty else {
        return Err(Diagnostic::new(
            0,
            "Object metadata supports inferred ThreadLocal array fields",
        ));
    };
    if !matches!(inner.as_ref(), Type::ByteArray | Type::CharArray) {
        return Err(Diagnostic::new(
            0,
            "Object metadata supports ByteArray and CharArray ThreadLocal arguments",
        ));
    }
    let array = platform_type(type_proto(inner, imports, strings)?, strings)?;
    let mut argument = Proto::default();
    argument.message(2, &array);
    let mut thread_local = Proto::default();
    thread_local.number(6, strings.id("java/lang/ThreadLocal")?);
    thread_local.message(2, &argument);
    platform_type(thread_local, strings)
}

pub(crate) fn for_object(
    module: &Module,
    owner: &str,
    fields: &[(String, Type)],
    interfaces: &[String],
    serializable: bool,
) -> Result<Metadata> {
    let object = module
        .object_decl
        .as_ref()
        .ok_or_else(|| Diagnostic::new(0, "Object metadata requires an object declaration"))?;
    if !object.constants.is_empty() || !object.objects.is_empty() {
        return Err(Diagnostic::new(
            object.offset,
            "Dynamic objects mixed with constant or nested object declarations are unsupported",
        ));
    }
    if fields.len() != object.fields.len() {
        return Err(Diagnostic::new(
            object.offset,
            "Object metadata field types do not match declarations",
        ));
    }
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(
        1,
        (if object.internal { 320 } else { 326 }) | u32::from(serializable),
    );
    proto.number(3, strings.id(&owner.replace('$', "."))?);
    for name in if interfaces.is_empty() {
        vec!["kotlin/Any"]
    } else {
        interfaces.iter().map(String::as_str).collect()
    } {
        let mut supertype = Proto::default();
        supertype.number(6, strings.id(&name.replace('$', "."))?);
        proto.message(6, &supertype);
    }
    let mut constructor = Proto::default();
    constructor.number(1, 2);
    constructor.message(100, &method_signature("<init>", "()V", &mut strings)?);
    proto.message(8, &constructor);
    if serializable {
        let mut function = Proto::default();
        function.number(2, strings.id("serializer")?);
        let mut result = Proto::default();
        result.number(6, strings.id("kotlinx/serialization/KSerializer")?);
        let mut argument = Proto::default();
        let mut ty = Proto::default();
        ty.number(6, strings.id(&owner.replace('$', "."))?);
        argument.message(2, &ty);
        result.message(2, &argument);
        function.message(3, &result);
        proto.message(9, &function);
    }
    for method in &object.methods {
        let flags = u32::from(method.jvm_static) | if method.inline { 1024 } else { 0 };
        proto.message(
            9,
            &function_with_flags(&method.function, 0, flags, &module.imports, &mut strings)?,
        );
    }
    let mut seen = BTreeSet::new();
    for field in &object.fields {
        if !field.private || !seen.insert(&field.name) {
            return Err(Diagnostic::new(
                field.offset,
                "Object metadata requires unique private ThreadLocal fields",
            ));
        }
        let matches: Vec<_> = fields
            .iter()
            .filter(|(name, _)| name == &field.name)
            .collect();
        let [(_, ty)] = matches.as_slice() else {
            return Err(Diagnostic::new(
                field.offset,
                "Object metadata field type is missing or repeated",
            ));
        };
        let mut declaration = Proto::default();
        declaration.number(11, 514);
        declaration.number(2, strings.id(&field.name)?);
        declaration.message(
            3,
            &inferred_thread_local(ty, &module.imports, &mut strings)?,
        );
        let mut signature = Proto::default();
        signature.message(
            1,
            &method_signature(&field.name, &descriptor(ty, &module.imports)?, &mut strings)?,
        );
        declaration.message(100, &signature);
        proto.message(10, &declaration);
    }
    strings.finish(1, proto)
}

pub(crate) fn for_package(module: &Module) -> Result<Metadata> {
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    for function in &module.functions {
        proto.message(
            3,
            &function_proto(function, false, &module.imports, &mut strings)?,
        );
    }
    strings.finish(2, proto)
}

pub(crate) fn module_mapping(facades: &[String]) -> Vec<u8> {
    let mut packages: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for facade in facades {
        let (package, name) = facade.rsplit_once('/').unwrap_or(("", facade));
        packages.entry(package).or_default().insert(name);
    }
    let mut module = Proto::default();
    for (package, names) in packages {
        let mut parts = Proto::default();
        parts.bytes(1, package.replace('/', ".").as_bytes());
        for name in names {
            parts.bytes(2, name.as_bytes());
        }
        module.message(1, &parts);
    }
    let mut bytes = Vec::new();
    for value in [3_u32, 2, 3, 0, 0] {
        bytes.extend(value.to_be_bytes());
    }
    bytes.extend(module.0);
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_encoding_preserves_all_bytes_and_modified_utf8_limits() {
        let bytes: Vec<_> = (0..=255).cycle().take(150_000).collect();
        let chunks = encode_bytes(&bytes);
        assert_eq!(chunks[0][0], 0);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            let encoded_length: usize = chunk
                .iter()
                .map(|unit| if (1..=127).contains(unit) { 1 } else { 2 })
                .sum();
            assert!(encoded_length <= 65535);
        }
        let decoded: Vec<_> = chunks
            .into_iter()
            .flatten()
            .skip(1)
            .map(|unit| unit as u8)
            .collect();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn nested_interface_headers_use_the_enclosing_scope() {
        let source = crate::syntax::parse(
            "interface Root {\ninterface Base\ninterface Child : Base {\ninterface Base\n}\n}",
        )
        .unwrap();
        let metadata = for_interfaces(&source, "Root").unwrap();
        let child = &metadata["Root$Child"];
        assert!(child.data2.iter().any(|name| name == "Root.Base"));
        assert!(!child.data2.iter().any(|name| name == "Root.Child.Base"));
    }

    #[test]
    fn module_mapping_groups_sorts_and_deduplicates_facades() {
        let facades = vec![
            "z/BKt".into(),
            "a/AKt".into(),
            "z/AKt".into(),
            "z/AKt".into(),
        ];
        let mut bytes = Vec::new();
        for value in [3_u32, 2, 3, 0, 0] {
            bytes.extend(value.to_be_bytes());
        }
        bytes.extend_from_slice(
            b"\x0a\x08\x0a\x01a\x12\x03AKt\x0a\x0d\x0a\x01z\x12\x03AKt\x12\x03BKt",
        );
        assert_eq!(module_mapping(&facades), bytes);
    }
}
