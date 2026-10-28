mod enums;
pub(crate) use enums::for_enum;
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    Diagnostic, Result,
    syntax::{
        ClassDecl, CompanionDecl, CompanionValue, ConstDecl, ConstValue, Function, InterfaceDecl,
        Module, Type, TypeAliasDecl, Variance,
    },
};

pub(crate) struct Metadata {
    pub kind: i32,
    pub data1: Vec<Vec<u16>>,
    pub data2: Vec<String>,
}

impl Metadata {
    pub(crate) fn in_module(mut self, module_name: &str) -> Result<Self> {
        if module_name == "main" {
            return Ok(self);
        }
        let bytes = self
            .data1
            .iter()
            .flatten()
            .skip(1)
            .map(|unit| {
                u8::try_from(*unit)
                    .map_err(|_| Diagnostic::new(0, "Invalid generated Kotlin metadata encoding"))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut offset = 0;
        let mut table_length = 0usize;
        let mut shift = 0;
        loop {
            let byte = *bytes
                .get(offset)
                .ok_or_else(|| Diagnostic::new(0, "Missing generated Kotlin string table"))?;
            offset += 1;
            table_length |= usize::from(byte & 127) << shift;
            if byte & 128 == 0 {
                break;
            }
            shift += 7;
            if shift >= usize::BITS {
                return Err(Diagnostic::new(
                    0,
                    "Invalid generated Kotlin string table length",
                ));
            }
        }
        let table_end = offset
            .checked_add(table_length)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| Diagnostic::new(0, "Invalid generated Kotlin string table length"))?;
        let mut table = Proto(bytes[offset..table_end].to_vec());
        let mut record = Proto::default();
        record.number(1, 1);
        table.message(1, &record);
        let mut payload = Proto(bytes[table_end..].to_vec());
        payload.number(101, self.data2.len() as u32);
        self.data2.push(module_name.into());
        let mut encoded = Proto::default();
        encoded.varint(table.0.len() as u64);
        encoded.0.extend(table.0);
        encoded.0.extend(payload.0);
        self.data1 = encode_bytes(&encoded.0);
        Ok(self)
    }
}

pub(crate) fn annotation_source(
    module: &Module,
    classpath: &crate::classpath::Classpath,
) -> Result<Module> {
    use crate::syntax::{AnnotationUse, Declaration, ObjectDecl};

    struct Filter<'a> {
        classpath: &'a crate::classpath::Classpath,
        imports: Imports<'a>,
        package: String,
    }
    impl Filter<'_> {
        fn definition(
            &self,
            annotation: &AnnotationUse,
        ) -> Result<crate::classpath::AnnotationDefinition> {
            let imported = self
                .imports
                .iter()
                .find(|(alias, _, _)| alias == &annotation.name);
            let owner = imported.map_or(annotation.name.as_str(), |(_, owner, _)| owner.as_str());
            let class = self.classpath.load(owner).or_else(|error| {
                for prefix in [
                    self.package.as_str(),
                    "kotlin",
                    "kotlin/annotation",
                    "kotlin/jvm",
                ] {
                    if let Ok(class) = self.classpath.load(&format!("{prefix}/{owner}")) {
                        return Ok(class);
                    }
                }
                Err(error)
            })?;
            class.annotation_definition.ok_or_else(|| {
                Diagnostic::new(annotation.offset, "Unsupported annotation definition")
            })
        }
        fn uses(&self, annotations: &mut Vec<AnnotationUse>, constructor: bool) -> Result<()> {
            let mut retained = Vec::new();
            for mut annotation in std::mem::take(annotations) {
                let Ok(definition) = self.definition(&annotation) else {
                    retained.push(annotation);
                    continue;
                };
                if definition.retention == crate::classpath::AnnotationRetention::Source {
                    continue;
                }
                if constructor && annotation.use_site.is_none() {
                    annotation.use_site = Some(
                        if definition
                            .targets
                            .iter()
                            .any(|target| target == "PARAMETER")
                        {
                            "param"
                        } else {
                            "field"
                        }
                        .into(),
                    );
                }
                retained.push(annotation);
            }
            *annotations = retained;
            Ok(())
        }
        fn function(&self, function: &mut Function) -> Result<()> {
            self.uses(&mut function.annotations, false)?;
            for annotations in &mut function.param_annotations {
                self.uses(annotations, false)?;
            }
            Ok(())
        }
        fn class_companion(&self, companion: &mut crate::syntax::ClassCompanion) -> Result<()> {
            for property in &mut companion.properties {
                self.uses(&mut property.annotations, false)?;
            }
            for method in &mut companion.methods {
                self.function(&mut method.function)?;
            }
            Ok(())
        }
        fn class(&self, class: &mut ClassDecl) -> Result<()> {
            self.uses(&mut class.annotations, false)?;
            for annotations in &mut class.param_annotations {
                self.uses(annotations, true)?;
            }
            for property in &mut class.properties {
                self.uses(&mut property.annotations, false)?;
            }
            for method in &mut class.methods {
                self.function(&mut method.function)?;
            }
            if let Some(companion) = &mut class.companion {
                self.class_companion(companion)?;
            }
            self.declarations(&mut class.nested)
        }
        fn object(&self, object: &mut ObjectDecl) -> Result<()> {
            self.uses(&mut object.annotation_uses, false)?;
            for constant in &mut object.constants {
                self.uses(&mut constant.annotations, false)?;
            }
            for property in &mut object.fields {
                self.uses(&mut property.annotations, false)?;
            }
            for method in &mut object.methods {
                self.function(&mut method.function)?;
            }
            for nested in &mut object.objects {
                self.object(nested)?;
            }
            self.declarations(&mut object.nested)
        }
        fn interface(&self, interface: &mut InterfaceDecl) -> Result<()> {
            self.uses(&mut interface.annotations, false)?;
            for property in &mut interface.properties {
                self.uses(&mut property.annotations, false)?;
            }
            for method in &mut interface.methods {
                self.function(&mut method.function)?;
            }
            for nested in &mut interface.nested {
                self.interface(nested)?;
            }
            if let Some(companion) = &mut interface.companion {
                for property in &mut companion.properties {
                    self.uses(&mut property.annotations, false)?;
                    if let CompanionValue::Object(class) = &mut property.value {
                        self.class(class)?;
                    }
                }
                for field in &mut companion.fields {
                    self.uses(&mut field.annotations, false)?;
                }
                for method in &mut companion.methods {
                    self.function(&mut method.function)?;
                }
            }
            self.declarations(&mut interface.declarations)
        }
        fn enumeration(&self, enumeration: &mut crate::syntax::EnumDecl) -> Result<()> {
            self.uses(&mut enumeration.annotations, false)?;
            self.class(&mut enumeration.body)
        }
        fn declarations(&self, declarations: &mut [Declaration]) -> Result<()> {
            for declaration in declarations {
                match declaration {
                    Declaration::Class(class) => self.class(class)?,
                    Declaration::Object(object) => self.object(object)?,
                    Declaration::Interface(interface) => self.interface(interface)?,
                    Declaration::Enum(enumeration) => self.enumeration(enumeration)?,
                }
            }
            Ok(())
        }
    }
    let mut source = module.clone();
    let filter = Filter {
        classpath,
        imports: &module.imports,
        package: module.package.replace('.', "/"),
    };
    for function in &mut source.functions {
        filter.function(function)?;
    }
    for constant in &mut source.constants {
        filter.uses(&mut constant.annotations, false)?;
    }
    for property in &mut source.properties {
        filter.uses(&mut property.annotations, false)?;
    }
    if let Some(class) = &mut source.class_decl {
        filter.class(class)?;
    }
    if let Some(object) = &mut source.object_decl {
        filter.object(object)?;
    }
    if let Some(interface) = &mut source.interface_decl {
        filter.interface(interface)?;
    }
    if let Some(enumeration) = &mut source.enum_decl {
        filter.enumeration(enumeration)?;
    }
    filter.declarations(&mut source.declarations)?;
    Ok(source)
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

fn imported_name<'a>(name: &'a str, imports: Imports<'a>) -> Result<&'a str> {
    if name.contains('/') {
        return Ok(name);
    }
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
        Type::Nullable(inner) if inner.is_reference() => descriptor(inner, imports),
        Type::Nullable(_) => Ok(ty.descriptor()),
        Type::Array(inner) => {
            let component = if inner.is_reference() {
                descriptor(inner, imports)?
            } else {
                Type::Nullable(inner.clone()).descriptor()
            };
            Ok(format!("[{component}"))
        }
        Type::Java(name) => Ok(format!("L{};", imported_name(name, imports)?)),
        Type::Parameterized { name, .. } => Ok(format!("L{};", jvm_classifier(name, imports)?)),
        Type::Parameter { upper_bound, .. } => {
            if upper_bound.is_reference() {
                descriptor(upper_bound, imports)
            } else {
                descriptor(&Type::Nullable(upper_bound.clone()), imports)
            }
        }
        Type::Platform(inner) => descriptor(inner, imports),
        _ => Ok(ty.descriptor()),
    }
}

fn jvm_classifier<'a>(name: &'a str, imports: Imports<'a>) -> Result<&'a str> {
    match name {
        "List" | "MutableList" | "kotlin/collections/List" | "kotlin/collections/MutableList" => {
            Ok("java/util/List")
        }
        "Set" | "MutableSet" | "kotlin/collections/Set" | "kotlin/collections/MutableSet" => {
            Ok("java/util/Set")
        }
        "Map" | "MutableMap" | "kotlin/collections/Map" | "kotlin/collections/MutableMap" => {
            Ok("java/util/Map")
        }
        "Collection"
        | "MutableCollection"
        | "kotlin/collections/Collection"
        | "kotlin/collections/MutableCollection" => Ok("java/util/Collection"),
        "Iterable"
        | "MutableIterable"
        | "kotlin/collections/Iterable"
        | "kotlin/collections/MutableIterable" => Ok("java/lang/Iterable"),
        "ThreadLocal" => Ok("java/lang/ThreadLocal"),
        _ => imported_name(name, imports),
    }
}

fn metadata_classifier<'a>(name: &'a str, imports: Imports<'a>) -> Result<&'a str> {
    match name {
        "List" => Ok("kotlin/collections/List"),
        "MutableList" => Ok("kotlin/collections/MutableList"),
        "Set" => Ok("kotlin/collections/Set"),
        "MutableSet" => Ok("kotlin/collections/MutableSet"),
        "Map" => Ok("kotlin/collections/Map"),
        "MutableMap" => Ok("kotlin/collections/MutableMap"),
        "Collection" => Ok("kotlin/collections/Collection"),
        "MutableCollection" => Ok("kotlin/collections/MutableCollection"),
        "Iterable" => Ok("kotlin/collections/Iterable"),
        "MutableIterable" => Ok("kotlin/collections/MutableIterable"),
        "Array" => Ok("kotlin/Array"),
        "ThreadLocal" => Ok("java/lang/ThreadLocal"),
        "java/lang/Object" => Ok("kotlin/Any"),
        "java/lang/CharSequence" => Ok("kotlin/CharSequence"),
        _ => imported_name(name, imports),
    }
}

fn type_proto(ty: &Type, imports: Imports<'_>, strings: &mut Strings) -> Result<Proto> {
    if let Type::Nullable(inner) = ty {
        if inner.is_nullable() {
            return type_proto(inner, imports, strings);
        }
        if let Type::Platform(inner) = inner.as_ref() {
            return type_proto(&Type::Nullable(inner.clone()), imports, strings);
        }
        let mut proto = type_proto(inner, imports, strings)?;
        proto.number(3, 1);
        return Ok(proto);
    }
    let mut proto = Proto::default();
    let name = match ty {
        Type::Int => "kotlin/Int",
        Type::Long => "kotlin/Long",
        Type::Null => {
            proto.number(3, 1);
            "kotlin/Nothing"
        }
        Type::Nullable(_) => unreachable!(),
        Type::List(inner) | Type::MutableList(inner) => {
            let mut argument = Proto::default();
            argument.message(2, &type_proto(inner, imports, strings)?);
            proto.message(2, &argument);
            if matches!(ty, Type::MutableList(_)) {
                "kotlin/collections/MutableList"
            } else {
                "kotlin/collections/List"
            }
        }
        Type::Byte => "kotlin/Byte",
        Type::Char => "kotlin/Char",
        Type::Boolean => "kotlin/Boolean",
        Type::Float => "kotlin/Float",
        Type::Double => "kotlin/Double",
        Type::IntArray => "kotlin/IntArray",
        Type::ByteArray => "kotlin/ByteArray",
        Type::CharArray => "kotlin/CharArray",
        Type::String => "kotlin/String",
        Type::Unit => "kotlin/Unit",
        Type::Array(inner) => {
            let mut argument = Proto::default();
            argument.message(2, &type_proto(inner, imports, strings)?);
            proto.message(2, &argument);
            "kotlin/Array"
        }
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
        Type::Parameterized { name, arguments } => {
            for argument in arguments {
                let mut value = Proto::default();
                value.number(
                    1,
                    match (&argument.ty, argument.variance) {
                        (None, _) => 3,
                        (_, Variance::In) => 0,
                        (_, Variance::Out) => 1,
                        (_, Variance::Invariant) => 2,
                    },
                );
                if let Some(ty) = &argument.ty {
                    value.message(2, &type_proto(ty, imports, strings)?);
                }
                proto.message(2, &value);
            }
            let name = metadata_classifier(name, imports)?.replace('$', ".");
            proto.number(6, strings.id(&name)?);
            return Ok(proto);
        }
        Type::Parameter { name, .. } => {
            proto.number(9, strings.id(name)?);
            return Ok(proto);
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            let mut annotation = Proto::default();
            annotation.number(1, strings.id("kotlin/ExtensionFunctionType")?);
            proto.message(100, &annotation);
            for ty in std::iter::once(receiver.as_ref())
                .chain(params.iter())
                .chain(std::iter::once(result.as_ref()))
            {
                let mut argument = Proto::default();
                argument.message(2, &type_proto(ty, imports, strings)?);
                proto.message(2, &argument);
            }
            proto.number(
                6,
                strings.id(&format!("kotlin/Function{}", params.len() + 1))?,
            );
            return Ok(proto);
        }
        Type::Function { params, result } => {
            for ty in params.iter().chain(std::iter::once(result.as_ref())) {
                let mut argument = Proto::default();
                argument.message(2, &type_proto(ty, imports, strings)?);
                proto.message(2, &argument);
            }
            proto.number(6, strings.id(&format!("kotlin/Function{}", params.len()))?);
            return Ok(proto);
        }
        Type::Java(name) => {
            let name = metadata_classifier(name, imports)?.replace('$', ".");
            proto.number(6, strings.id(&name)?);
            return Ok(proto);
        }
        Type::Platform(inner) => {
            if inner.is_nullable() {
                return type_proto(inner, imports, strings);
            }
            let lower = type_proto(inner, imports, strings)?;
            return platform_type(lower, strings);
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
    function_with_jvm_name(
        function,
        modality,
        extra_flags,
        &function.name,
        imports,
        strings,
    )
}

fn function_with_jvm_name(
    function: &Function,
    modality: u32,
    extra_flags: u32,
    jvm_name: &str,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<Proto> {
    let mut proto = Proto::default();
    let visibility = if function.protected {
        4
    } else if function.private {
        2
    } else if function.internal {
        0
    } else {
        6
    };
    proto.number(
        9,
        visibility
            | modality
            | extra_flags
            | if function.annotations.is_empty() {
                0
            } else {
                1
            }
            | if function.infix { 512 } else { 0 }
            | if function.operator { 256 } else { 0 },
    );
    proto.number(2, strings.id(&function.name)?);
    for (index, parameter) in function.type_params.iter().enumerate() {
        let mut declaration = Proto::default();
        declaration.number(1, index as u32);
        declaration.number(2, strings.id(&parameter.name)?);
        declaration.number(
            4,
            match parameter.variance {
                Variance::In => 0,
                Variance::Out => 1,
                Variance::Invariant => 2,
            },
        );
        for bound in &parameter.bounds {
            declaration.message(5, &type_proto(bound, imports, strings)?);
        }
        proto.message(4, &declaration);
    }
    proto.message(3, &type_proto(&function.result, imports, strings)?);
    let mut desc = String::from("(");
    if let Some(receiver) = &function.receiver {
        proto.message(5, &type_proto(receiver, imports, strings)?);
        desc.push_str(&descriptor(receiver, imports)?);
    }
    for (index, (name, ty)) in function.params.iter().enumerate() {
        let mut param = parameter(name, ty, imports, strings)?;
        if function.vararg && index + 1 == function.params.len() {
            let element = match ty {
                Type::IntArray => Type::Int,
                Type::ByteArray => Type::Byte,
                Type::CharArray => Type::Char,
                Type::StringArray => Type::String,
                Type::Array(element) => element.as_ref().clone(),
                _ => {
                    return Err(Diagnostic::new(
                        function.offset,
                        "Invalid source vararg array type",
                    ));
                }
            };
            param.message(4, &type_proto(&element, imports, strings)?);
        }
        let flags = (u32::from(function.defaults.get(index).is_some_and(Option::is_some)) * 2)
            | u32::from(
                function
                    .param_annotations
                    .get(index)
                    .is_some_and(|annotations| !annotations.is_empty()),
            );
        if flags != 0 {
            param.number(1, flags);
        }
        proto.message(6, &param);
        desc.push_str(&descriptor(ty, imports)?);
    }
    desc.push(')');
    desc.push_str(&descriptor(&function.result, imports)?);
    proto.message(100, &method_signature(jvm_name, &desc, strings)?);
    Ok(proto)
}

fn class_metadata(
    class: &ClassDecl,
    internal_name: &str,
    imports: Imports<'_>,
    anonymous: bool,
    checked: Option<&crate::check::ClassDecl>,
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
        }) | u32::from(!class.annotations.is_empty())
            | if class.data { 1024 } else { 0 }
            | if class.abstract_ {
                32
            } else if class.open {
                16
            } else {
                0
            },
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
    for nested in &class.nested {
        proto.number(7, strings.id(crate::declaration_name(nested))?);
    }
    if let Some((name, _)) = &class.super_class {
        let mut supertype = Proto::default();
        supertype.number(
            6,
            strings.id(&imported_name(name, imports)?.replace('$', "."))?,
        );
        proto.message(6, &supertype);
    }
    if class.interfaces.is_empty() && class.super_class.is_none() {
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
    if class.companion.is_some() {
        let name = strings.id("Companion")?;
        proto.number(4, name);
        proto.number(7, name);
    }
    if !anonymous && class.primary_constructor {
        let mut constructor = Proto::default();
        constructor.number(1, 6);
        let mut constructor_desc = String::from("(");
        for (index, (name, ty)) in class.params.iter().enumerate() {
            let mut param = parameter(name, ty, imports, &mut strings)?;
            let flags = (u32::from(class.defaults.get(index).is_some_and(Option::is_some)) * 2)
                | u32::from(
                    class
                        .param_annotations
                        .get(index)
                        .is_some_and(|annotations| {
                            annotations.iter().any(|annotation| {
                                annotation
                                    .use_site
                                    .as_deref()
                                    .is_none_or(|site| site == "param")
                            })
                        }),
                );
            if flags != 0 {
                param.number(1, flags);
            }
            constructor.message(2, &param);
            constructor_desc.push_str(&descriptor(ty, imports)?);
        }
        constructor_desc.push_str(")V");
        constructor.message(
            100,
            &method_signature("<init>", &constructor_desc, &mut strings)?,
        );
        proto.message(8, &constructor);
    }
    for source in &class.secondary_constructors {
        let mut constructor = Proto::default();
        constructor.number(1, 22);
        let mut desc = String::from("(");
        for (index, (name, ty)) in source.params.iter().enumerate() {
            let mut parameter = parameter(name, ty, imports, &mut strings)?;
            if source.defaults.get(index).is_some_and(Option::is_some) {
                parameter.number(1, 2);
            }
            constructor.message(2, &parameter);
            desc.push_str(&descriptor(ty, imports)?);
        }
        desc.push_str(")V");
        constructor.message(100, &method_signature("<init>", &desc, &mut strings)?);
        proto.message(8, &constructor);
    }
    for method in &class.methods {
        proto.message(
            9,
            &function_with_jvm_name(
                &method.function,
                if !method.has_body {
                    32
                } else if method.open {
                    16
                } else {
                    0
                },
                0,
                checked_method_name(
                    &method.function,
                    checked.into_iter().flat_map(|class| class.methods.iter()),
                    imports,
                ),
                imports,
                &mut strings,
            )?,
        );
    }
    for (index, (name, ty)) in class.params.iter().enumerate() {
        if !class.param_properties.contains(name) {
            continue;
        }
        let mut property = Proto::default();
        let public = class.public_params.contains(name);
        property.number(
            11,
            (if public { 518 } else { 514 })
                | u32::from(property_has_annotations(
                    class
                        .param_annotations
                        .get(index)
                        .map_or(&[], Vec::as_slice),
                ))
                | if class.override_params.contains(name) {
                    16
                } else {
                    0
                }
                | if class.mutable_params.contains(name) {
                    256 | 1024
                } else {
                    0
                },
        );
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
        if public && class.mutable_params.contains(name) {
            signature.message(
                4,
                &method_signature(
                    &crate::check::setter_name(name),
                    &format!("({})V", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
        }
        accessor_annotation_flags(
            &mut property,
            class
                .param_annotations
                .get(index)
                .map_or(&[], Vec::as_slice),
            ty,
            if public { 6 } else { 2 },
            false,
            false,
            class.mutable_params.contains(name),
            "<set-?>",
            imports,
            &mut strings,
        )?;
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
        let ty = checked
            .and_then(|class| {
                class
                    .fields
                    .iter()
                    .find(|field| field.name == property.name)
            })
            .map(|field| &field.ty)
            .or_else(|| {
                checked
                    .and_then(|class| {
                        class
                            .methods
                            .iter()
                            .find(|method| method.name == crate::check::getter_name(&property.name))
                    })
                    .map(|method| &method.result)
            })
            .unwrap_or(&property.ty);
        let flags = (if property.private {
            2
        } else if property.protected {
            4
        } else {
            6
        }) | if property.open { 16 } else { 0 };
        declaration.number(
            11,
            flags
                | 512
                | u32::from(property_has_annotations(&property.annotations))
                | if property.mutable { 256 | 1024 } else { 0 }
                | if property.lateinit { 4096 } else { 0 }
                | if property.delegate.is_some() {
                    32768
                } else {
                    0
                },
        );
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(ty, imports, &mut strings)?);
        if !property.body.is_empty() || property.delegate.is_some() {
            declaration.number(7, flags | 64);
        }
        if property.mutable && (property.setter.is_some() || property.delegate.is_some()) {
            declaration.number(8, flags | 64);
            let name = property
                .setter
                .as_ref()
                .map_or("<set-?>", |(name, _)| name.as_str());
            declaration.message(6, &parameter(name, ty, imports, &mut strings)?);
        }
        let mut signature = Proto::default();
        let backing = checked.and_then(|class| {
            class.fields.iter().find(|field| {
                field.name == property.name
                    || property.delegate.is_some()
                        && field.name == format!("{}$delegate", property.name)
            })
        });
        if let Some(field) = backing {
            signature.message(
                1,
                &method_signature(&field.name, &descriptor(&field.ty, imports)?, &mut strings)?,
            );
        } else if checked.is_none() && property.initializer.is_some() {
            signature.message(
                1,
                &method_signature(&property.name, &descriptor(ty, imports)?, &mut strings)?,
            );
        }
        if !property.private {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(&property.name),
                    &format!("(){}", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
        }
        if !property.private && property.mutable {
            signature.message(
                4,
                &method_signature(
                    &crate::check::setter_name(&property.name),
                    &format!("({})V", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
        }
        accessor_annotation_flags(
            &mut declaration,
            &property.annotations,
            ty,
            flags,
            !property.body.is_empty() || property.delegate.is_some(),
            property.setter.is_some() || property.delegate.is_some(),
            property.mutable,
            property.setter.as_ref().map_or("<set-?>", |(name, _)| name),
            imports,
            &mut strings,
        )?;
        declaration.message(100, &signature);
        proto.message(10, &declaration);
    }
    strings.finish(1, proto)
}

fn property_has_annotations(annotations: &[crate::syntax::AnnotationUse]) -> bool {
    annotations.iter().any(|annotation| {
        annotation
            .use_site
            .as_deref()
            .is_none_or(|site| matches!(site, "property" | "field"))
    })
}

#[allow(clippy::too_many_arguments)]
fn accessor_annotation_flags(
    property: &mut Proto,
    annotations: &[crate::syntax::AnnotationUse],
    ty: &Type,
    visibility: u32,
    custom_getter: bool,
    custom_setter: bool,
    mutable: bool,
    setter_name: &str,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<()> {
    let has_site = |site| {
        annotations
            .iter()
            .any(|annotation| annotation.use_site.as_deref() == Some(site))
    };
    let property_annotations = property_has_annotations(annotations);
    let getter_annotations = has_site("get");
    let setter_annotations = has_site("set");
    let setter_parameter_annotations = has_site("setparam");
    if getter_annotations != property_annotations || custom_getter && getter_annotations {
        property.number(
            7,
            visibility | (u32::from(custom_getter) * 64) | u32::from(getter_annotations),
        );
    }
    if mutable
        && (setter_annotations != property_annotations
            || custom_setter && setter_annotations
            || setter_parameter_annotations)
    {
        property.number(
            8,
            visibility
                | (u32::from(custom_setter || setter_parameter_annotations) * 64)
                | u32::from(setter_annotations),
        );
    }
    if mutable && setter_parameter_annotations {
        let mut value = parameter(setter_name, ty, imports, strings)?;
        value.number(1, 1);
        property.message(6, &value);
    }
    Ok(())
}

fn same_metadata_type(source: &Type, checked: &Type, imports: Imports<'_>) -> bool {
    match (source, checked) {
        (Type::Java(alias), Type::Java(owner)) => imports
            .iter()
            .find(|(name, _, _)| name == alias)
            .map(|(_, imported, _)| imported == owner)
            .unwrap_or_else(|| owner.rsplit('/').next() == Some(alias.as_str())),
        (
            Type::Parameterized {
                name: source,
                arguments: a,
            },
            Type::Parameterized {
                name: checked,
                arguments: b,
            },
        ) => {
            same_metadata_type(
                &Type::Java(source.clone()),
                &Type::Java(checked.clone()),
                imports,
            ) && a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.variance == b.variance
                        && match (&a.ty, &b.ty) {
                            (Some(a), Some(b)) => same_metadata_type(a, b, imports),
                            (None, None) => true,
                            _ => false,
                        }
                })
        }
        (
            Type::Function {
                params: a,
                result: ar,
            },
            Type::Function {
                params: b,
                result: br,
            },
        ) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| same_metadata_type(a, b, imports))
                && same_metadata_type(ar, br, imports)
        }
        (Type::Nullable(a), Type::Nullable(b))
        | (Type::List(a), Type::List(b))
        | (Type::MutableList(a), Type::MutableList(b)) => same_metadata_type(a, b, imports),
        _ => source == checked,
    }
}

fn checked_method_name<'a>(
    source: &'a Function,
    checked: impl Iterator<Item = &'a crate::check::Function>,
    imports: Imports<'_>,
) -> &'a str {
    checked
        .filter(|actual| actual.name == source.name)
        .find(|actual| {
            source.params.len() + usize::from(source.receiver.is_some()) == actual.params.len()
                && source
                    .receiver
                    .iter()
                    .chain(source.params.iter().map(|(_, ty)| ty))
                    .zip(&actual.params)
                    .all(|(source, (_, actual))| {
                        same_metadata_type(source, actual, imports)
                            || descriptor(source, imports)
                                .is_ok_and(|source| source == actual.descriptor())
                    })
        })
        .map_or(source.name.as_str(), crate::check::Function::jvm_name)
}

pub(crate) fn for_checked_class(
    source: &ClassDecl,
    checked: &crate::check::ClassDecl,
    owner: &str,
    imports: Imports<'_>,
) -> Result<Metadata> {
    let mut bindings = imports.to_vec();
    bindings.push((source.name.clone(), owner.into(), source.offset));
    fn bind(source: &Type, checked: &Type, bindings: &mut Vec<(String, String, usize)>) {
        match (source, checked) {
            (Type::Java(alias), Type::Java(owner)) => {
                bindings.push((alias.clone(), owner.clone(), 0))
            }
            (
                Type::Parameterized {
                    name: alias,
                    arguments: source,
                },
                Type::Parameterized {
                    name: owner,
                    arguments: checked,
                },
            ) => {
                bindings.push((alias.clone(), owner.clone(), 0));
                for (source, checked) in source.iter().zip(checked) {
                    if let (Some(source), Some(checked)) = (&source.ty, &checked.ty) {
                        bind(source, checked, bindings);
                    }
                }
            }
            (
                Type::Parameter {
                    upper_bound: source,
                    ..
                },
                Type::Parameter {
                    upper_bound: checked,
                    ..
                },
            ) => bind(source, checked, bindings),
            (
                Type::Function {
                    params: source,
                    result: source_result,
                },
                Type::Function {
                    params: checked,
                    result: checked_result,
                },
            ) => {
                for (source, checked) in source.iter().zip(checked) {
                    bind(source, checked, bindings);
                }
                bind(source_result, checked_result, bindings);
            }
            (Type::Nullable(a), Type::Nullable(b))
            | (Type::List(a), Type::List(b))
            | (Type::MutableList(a), Type::MutableList(b))
            | (Type::Array(a), Type::Array(b)) => bind(a, b, bindings),
            _ => {}
        }
    }
    for ((_, source), (_, checked)) in source.params.iter().zip(&checked.params) {
        bind(source, checked, &mut bindings);
    }
    for property in &source.properties {
        if let Some(field) = checked
            .fields
            .iter()
            .find(|field| field.name == property.name)
        {
            bind(&property.ty, &field.ty, &mut bindings);
        }
    }

    for method in &source.methods {
        if let Some(actual) = checked.methods.iter().find(|actual| {
            actual.name == method.function.name
                && actual.params.len() == method.function.params.len()
                && method.function.params.iter().zip(&actual.params).all(
                    |((_, source), (_, checked))| same_metadata_type(source, checked, &bindings),
                )
        }) {
            bind(&method.function.result, &actual.result, &mut bindings);
            for ((_, source), (_, checked)) in method.function.params.iter().zip(&actual.params) {
                bind(source, checked, &mut bindings);
            }
        }
    }
    class_metadata(source, owner, &bindings, false, Some(checked))
}

pub(crate) fn for_class_companion(
    source: &crate::syntax::ClassCompanion,
    companion: &crate::check::ClassCompanion,
    imports: Imports<'_>,
) -> Result<Metadata> {
    let owner = companion
        .owner
        .strip_suffix("$Companion")
        .ok_or_else(|| Diagnostic::new(0, "Invalid class companion owner"))?;
    let mut bindings = imports.to_vec();
    bindings.push((
        owner.rsplit('/').next().unwrap_or(owner).into(),
        owner.into(),
        0,
    ));
    let imports = bindings.as_slice();
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(1, 390);
    proto.number(3, strings.id(&companion.owner.replace('$', "."))?);
    proto.message(
        6,
        &type_proto(&Type::Java("kotlin/Any".into()), imports, &mut strings)?,
    );
    let mut constructor = Proto::default();
    constructor.number(1, 2);
    constructor.message(100, &method_signature("<init>", "()V", &mut strings)?);
    proto.message(8, &constructor);
    for method in &source.methods {
        proto.message(
            9,
            &function_with_jvm_name(
                &method.function,
                0,
                0,
                checked_method_name(&method.function, companion.methods.iter(), imports),
                imports,
                &mut strings,
            )?,
        );
    }
    for property in &source.properties {
        let ty = companion
            .methods
            .iter()
            .find(|method| method.name == crate::check::getter_name(&property.name))
            .map(|method| &method.result)
            .or_else(|| {
                companion
                    .properties
                    .iter()
                    .find(|field| field.name == property.name)
                    .map(|field| &field.ty)
            })
            .or(property.ty.as_ref())
            .ok_or_else(|| {
                Diagnostic::new(
                    property.offset,
                    "Companion property metadata type is missing",
                )
            })?;
        let mut declaration = Proto::default();
        declaration.number(
            11,
            (if property.private { 514 } else { 518 })
                | u32::from(property_has_annotations(&property.annotations))
                | if property.mutable { 256 | 1024 } else { 0 }
                | if property.lateinit { 4096 } else { 0 }
                | if property.delegate.is_some() {
                    32768
                } else {
                    0
                },
        );
        let accessor_flags = if property.private { 2 } else { 6 };
        if !property.body.is_empty() || property.delegate.is_some() {
            declaration.number(7, accessor_flags | 64);
        }
        if property.mutable && (property.setter.is_some() || property.delegate.is_some()) {
            declaration.number(8, accessor_flags | 64);
            let name = property
                .setter
                .as_ref()
                .map_or("<set-?>", |(name, _)| name.as_str());
            declaration.message(6, &parameter(name, ty, imports, &mut strings)?);
        }
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(ty, imports, &mut strings)?);
        let mut signature = Proto::default();
        if let Some(field) = companion.properties.iter().find(|field| {
            field.name == property.name
                || property.delegate.is_some()
                    && field.name == format!("{}$delegate", property.name)
        }) {
            signature.message(
                1,
                &method_signature(&field.name, &descriptor(&field.ty, imports)?, &mut strings)?,
            );
        }
        if !property.private {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(&property.name),
                    &format!("(){}", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
            if property.mutable {
                signature.message(
                    4,
                    &method_signature(
                        &crate::check::setter_name(&property.name),
                        &format!("({})V", descriptor(ty, imports)?),
                        &mut strings,
                    )?,
                );
            }
        }
        accessor_annotation_flags(
            &mut declaration,
            &property.annotations,
            ty,
            if property.private { 2 } else { 6 },
            !property.body.is_empty() || property.delegate.is_some(),
            property.setter.is_some() || property.delegate.is_some(),
            property.mutable,
            property.setter.as_ref().map_or("<set-?>", |(name, _)| name),
            imports,
            &mut strings,
        )?;
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
    checked: Option<&crate::check::InterfaceDecl>,
) -> Result<Metadata> {
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(
        1,
        (if interface.internal { 96 } else { 102 })
            | u32::from(!interface.annotations.is_empty())
            | if interface.fun_interface { 16384 } else { 0 },
    );
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
    for nested in &interface.declarations {
        proto.number(7, strings.id(crate::declaration_name(nested))?);
    }
    if interface.companion.is_some() {
        let name = strings.id("Companion")?;
        proto.number(4, name);
        proto.number(7, name);
    }
    for method in &interface.methods {
        proto.message(
            9,
            &function_with_jvm_name(
                &method.function,
                if method.has_body { 16 } else { 32 },
                0,
                checked_method_name(
                    &method.function,
                    checked.into_iter().flat_map(|interface| {
                        interface.methods.iter().map(|method| &method.function)
                    }),
                    imports,
                ),
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
    checked: Option<&crate::check::ClassCompanion>,
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
    for method in &companion.methods {
        proto.message(
            9,
            &function_with_jvm_name(
                &method.function,
                0,
                0,
                checked_method_name(
                    &method.function,
                    checked
                        .into_iter()
                        .flat_map(|companion| companion.methods.iter()),
                    imports,
                ),
                imports,
                &mut strings,
            )?,
        );
    }
    for property in &companion.fields {
        let ty = property.ty.as_ref().ok_or_else(|| {
            Diagnostic::new(property.offset, "Companion property type was not inferred")
        })?;
        let mut declaration = Proto::default();
        declaration.number(
            11,
            (if property.private { 514 } else { 518 })
                | u32::from(property_has_annotations(&property.annotations))
                | if property.mutable { 256 | 1024 } else { 0 }
                | if property.lateinit { 4096 } else { 0 }
                | if property.delegate.is_some() {
                    32768
                } else {
                    0
                },
        );
        let accessor_flags = if property.private { 2 } else { 6 };
        if !property.body.is_empty() || property.delegate.is_some() {
            declaration.number(7, accessor_flags | 64);
        }
        if property.mutable && (property.setter.is_some() || property.delegate.is_some()) {
            declaration.number(8, accessor_flags | 64);
            let name = property
                .setter
                .as_ref()
                .map_or("<set-?>", |(name, _)| name.as_str());
            declaration.message(6, &parameter(name, ty, imports, &mut strings)?);
        }
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(ty, imports, &mut strings)?);
        let mut signature = Proto::default();
        if let Some(field) = checked.and_then(|state| {
            state.properties.iter().find(|field| {
                field.name == property.name
                    || property.delegate.is_some()
                        && field.name == format!("{}$delegate", property.name)
            })
        }) {
            signature.message(
                1,
                &method_signature(&field.name, &descriptor(&field.ty, imports)?, &mut strings)?,
            );
        } else if checked.is_none() && property.initializer.is_some() {
            signature.message(
                1,
                &method_signature(&property.name, &descriptor(ty, imports)?, &mut strings)?,
            );
        }
        if !property.private {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(&property.name),
                    &format!("(){}", descriptor(ty, imports)?),
                    &mut strings,
                )?,
            );
            if property.mutable {
                signature.message(
                    4,
                    &method_signature(
                        &crate::check::setter_name(&property.name),
                        &format!("({})V", descriptor(ty, imports)?),
                        &mut strings,
                    )?,
                );
            }
        }
        accessor_annotation_flags(
            &mut declaration,
            &property.annotations,
            ty,
            if property.private { 2 } else { 6 },
            !property.body.is_empty() || property.delegate.is_some(),
            property.setter.is_some() || property.delegate.is_some(),
            property.mutable,
            property.setter.as_ref().map_or("<set-?>", |(name, _)| name),
            imports,
            &mut strings,
        )?;
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
    checked: Option<&crate::check::InterfaceDecl>,
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
        interface_metadata(interface, owner, imports, &bindings, checked)?,
    );
    if let Some(companion) = &interface.companion {
        let companion_owner = format!("{owner}$Companion");
        result.insert(
            companion_owner.clone(),
            companion_metadata(
                companion,
                &companion_owner,
                &bindings,
                checked
                    .and_then(|interface| interface.companion.as_ref())
                    .and_then(|companion| companion.state.as_ref()),
            )?,
        );
        for property in &companion.properties {
            if let CompanionValue::Object(class) = &property.value {
                let owner = format!("{companion_owner}${}$1", property.name);
                result.insert(
                    owner.clone(),
                    class_metadata(class, &owner, &bindings, true, None)?,
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
            checked.and_then(|interface| {
                interface
                    .nested
                    .iter()
                    .find(|child| child.owner == format!("{owner}${}", nested.name))
            }),
        )?;
    }
    Ok(())
}

pub(crate) fn for_interfaces(
    module: &Module,
    owner: &str,
    checked: Option<&crate::check::InterfaceDecl>,
) -> Result<BTreeMap<String, Metadata>> {
    let interface = module.interface_decl.as_ref().ok_or_else(|| {
        Diagnostic::new(0, "Interface metadata requires an interface declaration")
    })?;
    let mut result = BTreeMap::new();
    let mut imports = vec![(interface.name.clone(), owner.into(), interface.offset)];
    imports.extend_from_slice(&module.imports);
    interface_tree(interface, owner, &imports, &mut result, checked)?;
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

#[cfg(test)]
pub(crate) fn for_object(
    module: &Module,
    owner: &str,
    fields: &[(String, Type)],
    interfaces: &[String],
    serializable: bool,
) -> Result<Metadata> {
    object_metadata(module, owner, fields, interfaces, serializable, None)
}

pub(crate) fn for_checked_object(
    module: &Module,
    owner: &str,
    fields: &[(String, Type)],
    interfaces: &[String],
    serializable: bool,
    checked: &crate::check::DynamicObject,
) -> Result<Metadata> {
    object_metadata(
        module,
        owner,
        fields,
        interfaces,
        serializable,
        Some(checked),
    )
}

fn object_metadata(
    module: &Module,
    owner: &str,
    fields: &[(String, Type)],
    interfaces: &[String],
    serializable: bool,
    checked: Option<&crate::check::DynamicObject>,
) -> Result<Metadata> {
    let object = module
        .object_decl
        .as_ref()
        .ok_or_else(|| Diagnostic::new(0, "Object metadata requires an object declaration"))?;
    let mut context = module.clone();
    context
        .imports
        .insert(0, (object.name.clone(), owner.into(), object.offset));
    let module = &context;
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    proto.number(
        1,
        (if object.internal { 320 } else { 326 })
            | u32::from(serializable || !object.annotation_uses.is_empty()),
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
    for nested in &object.nested {
        proto.number(7, strings.id(crate::declaration_name(nested))?);
    }
    for nested in &object.objects {
        proto.number(7, strings.id(&nested.name)?);
    }
    for constant in &object.constants {
        proto.message(
            10,
            &constant_proto(constant, &module.imports, &mut strings)?,
        );
    }
    for method in &object.methods {
        let flags = u32::from(method.jvm_static) | if method.inline { 1024 } else { 0 };
        proto.message(
            9,
            &function_with_jvm_name(
                &method.function,
                0,
                flags,
                checked_method_name(
                    &method.function,
                    checked
                        .into_iter()
                        .flat_map(|object| object.methods.iter().map(|method| &method.function)),
                    &module.imports,
                ),
                &module.imports,
                &mut strings,
            )?,
        );
    }
    let mut seen = BTreeSet::new();
    for field in &object.fields {
        if !seen.insert(&field.name) {
            return Err(Diagnostic::new(
                field.offset,
                "Object metadata requires unique field names",
            ));
        }
        let ty = fields
            .iter()
            .find(|(name, _)| name == &field.name)
            .map(|(_, ty)| ty)
            .or(field.ty.as_ref())
            .ok_or_else(|| {
                Diagnostic::new(field.offset, "Object metadata property type is missing")
            })?;
        let mut declaration = Proto::default();
        declaration.number(
            11,
            (if field.private { 514 } else { 518 })
                | u32::from(property_has_annotations(&field.annotations))
                | if field.mutable { 256 | 1024 } else { 0 }
                | if field.lateinit { 4096 } else { 0 }
                | if field.delegate.is_some() { 32768 } else { 0 },
        );
        let accessor_flags = if field.private { 2 } else { 6 };
        if !field.body.is_empty() || field.delegate.is_some() {
            declaration.number(7, accessor_flags | 64);
        }
        if field.mutable && (field.setter.is_some() || field.delegate.is_some()) {
            declaration.number(8, accessor_flags | 64);
            let name = field
                .setter
                .as_ref()
                .map_or("<set-?>", |(name, _)| name.as_str());
            declaration.message(6, &parameter(name, ty, &module.imports, &mut strings)?);
        }
        declaration.number(2, strings.id(&field.name)?);
        let metadata_type = if matches!(ty, Type::ThreadLocal(_)) {
            inferred_thread_local(ty, &module.imports, &mut strings)?
        } else {
            type_proto(ty, &module.imports, &mut strings)?
        };
        declaration.message(3, &metadata_type);
        let mut signature = Proto::default();
        if let Some((name, backing_ty)) = fields.iter().find(|(name, _)| {
            name == &field.name
                || field.delegate.is_some() && *name == format!("{}$delegate", field.name)
        }) {
            signature.message(
                1,
                &method_signature(
                    name,
                    &descriptor(backing_ty, &module.imports)?,
                    &mut strings,
                )?,
            );
        }
        if !field.private {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(&field.name),
                    &format!("(){}", descriptor(ty, &module.imports)?),
                    &mut strings,
                )?,
            );
            if field.mutable {
                signature.message(
                    4,
                    &method_signature(
                        &crate::check::setter_name(&field.name),
                        &format!("({})V", descriptor(ty, &module.imports)?),
                        &mut strings,
                    )?,
                );
            }
        }
        accessor_annotation_flags(
            &mut declaration,
            &field.annotations,
            ty,
            if field.private { 2 } else { 6 },
            !field.body.is_empty() || field.delegate.is_some(),
            field.setter.is_some() || field.delegate.is_some(),
            field.mutable,
            field.setter.as_ref().map_or("<set-?>", |(name, _)| name),
            &module.imports,
            &mut strings,
        )?;
        declaration.message(100, &signature);
        proto.message(10, &declaration);
    }
    strings.finish(1, proto)
}

fn constant_proto(
    constant: &ConstDecl,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<Proto> {
    let ty = match constant.value {
        ConstValue::Int(_) => Type::Int,
        ConstValue::Long(_) => Type::Long,
        ConstValue::Float(_) => Type::Float,
        ConstValue::Double(_) => Type::Double,
        ConstValue::Char(_) => Type::Char,
        ConstValue::Boolean(_) => Type::Boolean,
        ConstValue::String(_) => Type::String,
    };
    let mut proto = Proto::default();
    proto.number(
        11,
        (if constant.private {
            2
        } else if constant.internal {
            0
        } else {
            6
        }) | 0x2a00
            | u32::from(property_has_annotations(&constant.annotations)),
    );
    proto.number(2, strings.id(&constant.name)?);
    proto.message(3, &type_proto(&ty, imports, strings)?);
    let mut signature = Proto::default();
    signature.message(
        1,
        &method_signature(&constant.name, &descriptor(&ty, imports)?, strings)?,
    );
    proto.message(100, &signature);
    Ok(proto)
}

fn type_alias_proto(
    alias: &TypeAliasDecl,
    imports: Imports<'_>,
    strings: &mut Strings,
) -> Result<Proto> {
    let mut proto = Proto::default();
    proto.number(1, if alias.private { 2 } else { 6 });
    proto.number(2, strings.id(&alias.name)?);
    for (index, parameter) in alias.type_params.iter().enumerate() {
        let mut declaration = Proto::default();
        declaration.number(1, index as u32);
        declaration.number(2, strings.id(&parameter.name)?);
        declaration.number(4, 2);
        for bound in &parameter.bounds {
            declaration.message(5, &type_proto(bound, imports, strings)?);
        }
        proto.message(3, &declaration);
    }
    let expanded = type_proto(&alias.expanded, imports, strings)?;
    proto.message(4, &expanded);
    proto.message(6, &expanded);
    Ok(proto)
}

pub(crate) fn for_checked_package(
    module: &Module,
    checked: &crate::check::Module,
) -> Result<Metadata> {
    let mut bindings = module.imports.clone();
    fn bind(source: &Type, checked: &Type, bindings: &mut Vec<(String, String, usize)>) {
        match (source, checked) {
            (Type::Java(alias), Type::Java(owner)) => {
                bindings.push((alias.clone(), owner.clone(), 0))
            }
            (
                Type::Parameterized {
                    name: alias,
                    arguments: source,
                },
                Type::Parameterized {
                    name: owner,
                    arguments: checked,
                },
            ) => {
                bindings.push((alias.clone(), owner.clone(), 0));
                for (source, checked) in source.iter().zip(checked) {
                    if let (Some(source), Some(checked)) = (&source.ty, &checked.ty) {
                        bind(source, checked, bindings);
                    }
                }
            }
            (
                Type::Parameter {
                    upper_bound: source,
                    ..
                },
                Type::Parameter {
                    upper_bound: checked,
                    ..
                },
            ) => bind(source, checked, bindings),
            (
                Type::Function {
                    params: source,
                    result: source_result,
                },
                Type::Function {
                    params: checked,
                    result: checked_result,
                },
            ) => {
                for (source, checked) in source.iter().zip(checked) {
                    bind(source, checked, bindings);
                }
                bind(source_result, checked_result, bindings);
            }
            (Type::Nullable(a), Type::Nullable(b))
            | (Type::List(a), Type::List(b))
            | (Type::MutableList(a), Type::MutableList(b))
            | (Type::ThreadLocal(a), Type::ThreadLocal(b))
            | (Type::Array(a), Type::Array(b)) => bind(a, b, bindings),
            _ => {}
        }
    }
    for source in &module.functions {
        let checked = checked
            .functions
            .iter()
            .find(|actual| {
                source.name == actual.name
                    && source.params.len() + usize::from(source.receiver.is_some())
                        == actual.params.len()
                    && source
                        .receiver
                        .iter()
                        .chain(source.params.iter().map(|(_, ty)| ty))
                        .zip(&actual.params)
                        .all(|(source, (_, actual))| {
                            descriptor(source, &bindings)
                                .is_ok_and(|source| source == actual.descriptor())
                                || same_metadata_type(source, actual, &bindings)
                        })
            })
            .ok_or_else(|| {
                Diagnostic::new(
                    source.offset,
                    "Checked functions do not match package metadata declarations",
                )
            })?;
        let receiver_count = usize::from(source.receiver.is_some());
        if source.name != checked.name
            || source.params.len() + receiver_count != checked.params.len()
        {
            return Err(Diagnostic::new(
                source.offset,
                "Checked function does not match metadata declaration",
            ));
        }
        bind(&source.result, &checked.result, &mut bindings);
        if let Some(receiver) = &source.receiver {
            bind(receiver, &checked.params[0].1, &mut bindings);
        }
        for ((_, source), (_, checked)) in
            source.params.iter().zip(&checked.params[receiver_count..])
        {
            bind(source, checked, &mut bindings);
        }
    }
    for property in &module.properties {
        if let Some(actual) = checked
            .properties
            .iter()
            .find(|actual| actual.name == property.name)
        {
            bind(&property.ty, &actual.ty, &mut bindings);
        }
    }
    let mut strings = Strings::default();
    let mut proto = Proto::default();
    for alias in &module.type_aliases {
        proto.message(5, &type_alias_proto(alias, &bindings, &mut strings)?);
    }
    for function in &module.functions {
        proto.message(
            3,
            &function_proto(function, false, &bindings, &mut strings)?,
        );
    }
    for constant in &module.constants {
        proto.message(4, &constant_proto(constant, &bindings, &mut strings)?);
    }
    for property in &module.properties {
        let mut declaration = Proto::default();
        declaration.number(
            11,
            (if property.private { 514 } else { 518 })
                | u32::from(property_has_annotations(&property.annotations))
                | if property.mutable { 256 | 1024 } else { 0 }
                | if property.lateinit { 4096 } else { 0 }
                | if property.delegate.is_some() {
                    32768
                } else {
                    0
                },
        );
        let accessor_flags = if property.private { 2 } else { 6 };
        if !property.body.is_empty() || property.delegate.is_some() {
            declaration.number(7, accessor_flags | 64);
        }
        if property.mutable && (property.setter.is_some() || property.delegate.is_some()) {
            declaration.number(8, accessor_flags | 64);
            let name = property
                .setter
                .as_ref()
                .map_or("<set-?>", |(name, _)| name.as_str());
            declaration.message(6, &parameter(name, &property.ty, &bindings, &mut strings)?);
        }
        declaration.number(2, strings.id(&property.name)?);
        declaration.message(3, &type_proto(&property.ty, &bindings, &mut strings)?);
        let mut signature = Proto::default();
        if let Some(field) = checked.properties.iter().find(|field| {
            field.name == property.name
                || property.delegate.is_some()
                    && field.name == format!("{}$delegate", property.name)
        }) {
            signature.message(
                1,
                &method_signature(
                    &field.name,
                    &descriptor(&field.ty, &bindings)?,
                    &mut strings,
                )?,
            );
        }
        if !property.private
            || checked
                .functions
                .iter()
                .any(|function| function.name == crate::check::getter_name(&property.name))
        {
            signature.message(
                3,
                &method_signature(
                    &crate::check::getter_name(&property.name),
                    &format!("(){}", descriptor(&property.ty, &bindings)?),
                    &mut strings,
                )?,
            );
        }
        if property.mutable
            && (!property.private
                || checked
                    .functions
                    .iter()
                    .any(|function| function.name == crate::check::setter_name(&property.name)))
        {
            signature.message(
                4,
                &method_signature(
                    &crate::check::setter_name(&property.name),
                    &format!("({})V", descriptor(&property.ty, &bindings)?),
                    &mut strings,
                )?,
            );
        }
        accessor_annotation_flags(
            &mut declaration,
            &property.annotations,
            &property.ty,
            accessor_flags,
            !property.body.is_empty() || property.delegate.is_some(),
            property.setter.is_some() || property.delegate.is_some(),
            property.mutable,
            property.setter.as_ref().map_or("<set-?>", |(name, _)| name),
            &bindings,
            &mut strings,
        )?;
        declaration.message(100, &signature);
        proto.message(4, &declaration);
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

    fn metadata_fields(metadata: &Metadata) -> Vec<(u32, Option<u64>, Vec<u8>)> {
        let bytes = metadata
            .data1
            .iter()
            .flatten()
            .skip(1)
            .map(|value| u8::try_from(*value).unwrap())
            .collect::<Vec<_>>();
        let mut cursor = 0;
        let mut length = 0;
        for shift in (0..64).step_by(7) {
            let byte = bytes[cursor];
            cursor += 1;
            length |= usize::from(byte & 127) << shift;
            if byte < 128 {
                break;
            }
        }
        fields(&bytes[cursor + length..])
    }

    #[test]
    fn functional_interface_metadata_uses_the_pinned_fun_flag() {
        for (source, expected) in [
            ("fun interface Action { fun run() }", 102 | 16384),
            ("interface Action { fun run() }", 102),
        ] {
            let module = crate::syntax::parse(source).unwrap();
            let metadata = for_interfaces(&module, "Action", None)
                .unwrap()
                .remove("Action")
                .unwrap();
            let flags = metadata_fields(&metadata)
                .into_iter()
                .find(|(field, _, _)| *field == 1)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(flags, expected);
        }
    }

    #[test]
    fn type_alias_metadata_preserves_name_visibility_and_expanded_function_types() {
        let module = crate::syntax::parse(
            "typealias Label = String\nprivate typealias Callback = (Int) -> Unit",
        )
        .unwrap();
        let mut strings = Strings::default();
        let mut package = Proto::default();
        for alias in &module.type_aliases {
            package.message(5, &type_alias_proto(alias, &[], &mut strings).unwrap());
        }
        let metadata = strings.finish(2, package).unwrap();
        let aliases = metadata_fields(&metadata)
            .into_iter()
            .filter(|(field, _, _)| *field == 5)
            .map(|(_, _, bytes)| fields(&bytes))
            .collect::<Vec<_>>();
        assert_eq!(aliases.len(), 2);
        for (index, expected_flags) in [6, 2].into_iter().enumerate() {
            let alias = &aliases[index];
            assert_eq!(
                alias.iter().find(|(field, _, _)| *field == 1).unwrap().1,
                Some(expected_flags)
            );
            let name = alias
                .iter()
                .find(|(field, _, _)| *field == 2)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(
                metadata.data2[name as usize],
                module.type_aliases[index].name
            );
            let underlying = &alias.iter().find(|(field, _, _)| *field == 4).unwrap().2;
            let expanded = &alias.iter().find(|(field, _, _)| *field == 6).unwrap().2;
            assert_eq!(underlying, expanded);
            let class_name = fields(expanded)
                .into_iter()
                .find(|(field, _, _)| *field == 6)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(
                metadata.data2[class_name as usize],
                ["kotlin/String", "kotlin/Function1"][index]
            );
        }
    }

    #[test]
    fn custom_setters_preserve_named_typed_parameters_in_every_declaration_scope() {
        let property = "var value: Int=0\nset(input) {field=input}";
        for scope in 0..5 {
            let source = match scope {
                0 => format!("class Holder {{{property}}}"),
                1 => property.into(),
                2 => format!("object Holder {{{property}}}"),
                3 => format!("class Holder {{companion object {{{property}}}}}"),
                _ => format!("interface Holder {{companion object {{{property}}}}}"),
            };
            let module = crate::syntax::parse(&source).unwrap();
            let classpath = crate::classpath::Classpath::new(&[]).unwrap();
            let owner = if scope == 1 { "FileKt" } else { "Holder" };
            classpath.source_header(&module, owner).unwrap();
            let checked = crate::check::check_with_owner(&module, &classpath, owner).unwrap();
            let metadata = match scope {
                0 => for_checked_class(
                    module.class_decl.as_ref().unwrap(),
                    checked.class_decl.as_ref().unwrap(),
                    owner,
                    &module.imports,
                )
                .unwrap(),
                1 => for_checked_package(&module, &checked).unwrap(),
                2 => {
                    for_object(&module, owner, &[("value".into(), Type::Int)], &[], false).unwrap()
                }
                3 => for_class_companion(
                    module
                        .class_decl
                        .as_ref()
                        .unwrap()
                        .companion
                        .as_ref()
                        .unwrap(),
                    checked
                        .class_decl
                        .as_ref()
                        .unwrap()
                        .companion
                        .as_ref()
                        .unwrap(),
                    &module.imports,
                )
                .unwrap(),
                _ => for_interfaces(&module, owner, checked.interface_decl.as_ref())
                    .unwrap()
                    .remove("Holder$Companion")
                    .unwrap(),
            };
            let property = metadata_fields(&metadata)
                .into_iter()
                .find(|(field, _, _)| *field == if scope == 1 { 4 } else { 10 })
                .unwrap()
                .2;
            let declaration = fields(&property);
            assert!(
                declaration
                    .iter()
                    .any(|(field, value, _)| *field == 8 && *value == Some(70)),
                "scope {scope}"
            );
            let setter = fields(
                &declaration
                    .iter()
                    .find(|(field, _, _)| *field == 6)
                    .unwrap()
                    .2,
            );
            let name = setter
                .iter()
                .find(|(field, _, _)| *field == 2)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(metadata.data2[name as usize], "input", "scope {scope}");
            let ty = fields(&setter.iter().find(|(field, _, _)| *field == 3).unwrap().2);
            let classifier = ty
                .iter()
                .find(|(field, _, _)| *field == 6)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(
                metadata.data2[classifier as usize], "kotlin/Int",
                "scope {scope}"
            );
        }
    }

    #[test]
    fn delegated_setters_have_typed_implicit_parameters_only_for_mutable_properties() {
        let source = crate::syntax::parse(
            "class Holder {var score: Int by delegate;val readonly: Int by delegate}",
        )
        .unwrap();
        let metadata = class_metadata(
            source.class_decl.as_ref().unwrap(),
            "Holder",
            &[],
            false,
            None,
        )
        .unwrap();
        for (_, _, bytes) in metadata_fields(&metadata)
            .into_iter()
            .filter(|(field, _, _)| *field == 10)
        {
            let property = fields(&bytes);
            let name = property
                .iter()
                .find(|(field, _, _)| *field == 2)
                .unwrap()
                .1
                .unwrap();
            if metadata.data2[name as usize] == "readonly" {
                assert!(!property.iter().any(|(field, _, _)| matches!(field, 6 | 8)));
                continue;
            }
            let parameter = fields(&property.iter().find(|(field, _, _)| *field == 6).unwrap().2);
            let name = parameter
                .iter()
                .find(|(field, _, _)| *field == 2)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(metadata.data2[name as usize], "<set-?>");
            let ty = fields(
                &parameter
                    .iter()
                    .find(|(field, _, _)| *field == 3)
                    .unwrap()
                    .2,
            );
            let name = ty
                .iter()
                .find(|(field, _, _)| *field == 6)
                .unwrap()
                .1
                .unwrap();
            assert_eq!(metadata.data2[name as usize], "kotlin/Int");
        }
    }

    #[test]
    fn property_metadata_preserves_lateinit_and_delegated_flags() {
        let source = crate::syntax::parse(
            "class Holder { lateinit var text: String; var score: Int by delegate }",
        )
        .unwrap();
        let metadata = class_metadata(
            source.class_decl.as_ref().unwrap(),
            "Holder",
            &[],
            false,
            None,
        )
        .unwrap();
        let bytes = metadata
            .data1
            .iter()
            .flatten()
            .skip(1)
            .map(|value| u8::try_from(*value).unwrap())
            .collect::<Vec<_>>();
        let mut cursor = 0;
        let mut length = 0;
        for shift in (0..64).step_by(7) {
            let byte = bytes[cursor];
            cursor += 1;
            length |= usize::from(byte & 127) << shift;
            if byte < 128 {
                break;
            }
        }
        let properties = fields(&bytes[cursor + length..])
            .into_iter()
            .filter(|(field, _, _)| *field == 10)
            .map(|(_, _, bytes)| {
                let property = fields(&bytes);
                let name = property
                    .iter()
                    .find(|(field, _, _)| *field == 2)
                    .unwrap()
                    .1
                    .unwrap();
                let flags = property
                    .iter()
                    .find(|(field, _, _)| *field == 11)
                    .unwrap()
                    .1
                    .unwrap();
                (metadata.data2[name as usize].clone(), flags)
            })
            .collect::<BTreeMap<_, _>>();
        assert_ne!(properties["text"] & 4096, 0);
        assert_eq!(properties["text"] & 32768, 0);
        assert_ne!(properties["score"] & 32768, 0);
        assert_eq!(properties["score"] & 4096, 0);
    }

    fn fields(bytes: &[u8]) -> Vec<(u32, Option<u64>, Vec<u8>)> {
        fn varint(bytes: &mut &[u8]) -> u64 {
            let mut value = 0;
            for shift in (0..64).step_by(7) {
                let byte = bytes[0];
                *bytes = &bytes[1..];
                value |= u64::from(byte & 127) << shift;
                if byte < 128 {
                    return value;
                }
            }
            panic!("invalid protobuf varint")
        }
        let mut bytes = bytes;
        let mut result = Vec::new();
        while !bytes.is_empty() {
            let tag = varint(&mut bytes);
            match tag & 7 {
                0 => result.push(((tag >> 3) as u32, Some(varint(&mut bytes)), Vec::new())),
                2 => {
                    let length = varint(&mut bytes) as usize;
                    result.push(((tag >> 3) as u32, None, bytes[..length].to_vec()));
                    bytes = &bytes[length..];
                }
                _ => panic!("unexpected protobuf wire type"),
            }
        }
        result
    }

    #[test]
    fn metadata_keeps_projections_type_parameters_and_nullable_functions() {
        let module = crate::syntax::parse("import demo.Box\nfun <T: CharSequence> transform(value: Box<out T>, ignored: Box<*>, callback: ((T) -> String)?): T = value.get()").unwrap();
        let function = &module.functions[0];
        let mut strings = Strings::default();
        let proto = function_proto(function, false, &module.imports, &mut strings).unwrap();
        let function_fields = fields(&proto.0);
        let declaration = fields(
            &function_fields
                .iter()
                .find(|(number, _, _)| *number == 4)
                .unwrap()
                .2,
        );
        let name = declaration
            .iter()
            .find(|(number, _, _)| *number == 2)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(strings.values[name as usize], "T");
        let bound = fields(
            &declaration
                .iter()
                .find(|(number, _, _)| *number == 5)
                .unwrap()
                .2,
        );
        let classifier = bound
            .iter()
            .find(|(number, _, _)| *number == 6)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(strings.values[classifier as usize], "kotlin/CharSequence");
        let result = fields(
            &function_fields
                .iter()
                .find(|(number, _, _)| *number == 3)
                .unwrap()
                .2,
        );
        assert!(result.iter().any(|(number, _, _)| *number == 9));
        assert!(!result.iter().any(|(number, _, _)| *number == 6));
        let projected = type_proto(&function.params[0].1, &module.imports, &mut strings).unwrap();
        let projected_fields = fields(&projected.0);
        let argument = fields(
            &projected_fields
                .iter()
                .find(|(number, _, _)| *number == 2)
                .unwrap()
                .2,
        );
        assert!(
            argument
                .iter()
                .any(|(number, value, _)| *number == 1 && *value == Some(1))
        );
        let star = type_proto(&function.params[1].1, &module.imports, &mut strings).unwrap();
        let star_fields = fields(&star.0);
        let argument = fields(
            &star_fields
                .iter()
                .find(|(number, _, _)| *number == 2)
                .unwrap()
                .2,
        );
        assert_eq!(argument, vec![(1, Some(3), Vec::new())]);
        let callback = type_proto(&function.params[2].1, &module.imports, &mut strings).unwrap();
        let callback_fields = fields(&callback.0);
        assert!(
            callback_fields
                .iter()
                .any(|(number, value, _)| *number == 3 && *value == Some(1))
        );
        let classifier = callback_fields
            .iter()
            .find(|(number, _, _)| *number == 6)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(strings.values[classifier as usize], "kotlin/Function1");
        assert_eq!(
            callback_fields
                .iter()
                .filter(|(number, _, _)| *number == 2)
                .count(),
            2
        );
        assert_eq!(
            descriptor(&function.params[0].1, &module.imports).unwrap(),
            "Ldemo/Box;"
        );
        assert_eq!(
            descriptor(&function.params[2].1, &module.imports).unwrap(),
            "Lkotlin/jvm/functions/Function1;"
        );
        assert_eq!(
            descriptor(&function.params[0].1, &[]).unwrap_err().message,
            "Metadata type requires an explicit import: Box"
        );
    }

    #[test]
    fn inferred_platform_metadata_preserves_flexible_lower_and_nullable_upper_bounds() {
        let mut strings = Strings::default();
        let value = type_proto(&Type::Platform(Box::new(Type::String)), &[], &mut strings).unwrap();
        let lower = fields(&value.0);
        let capability = lower
            .iter()
            .find(|(field, _, _)| *field == 4)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(
            strings.values[capability as usize],
            "kotlin.jvm.PlatformType"
        );
        assert!(!lower.iter().any(|(field, _, _)| *field == 3));
        let upper = fields(&lower.iter().find(|(field, _, _)| *field == 5).unwrap().2);
        assert!(
            upper
                .iter()
                .any(|(field, value, _)| *field == 3 && *value == Some(1))
        );
        assert_eq!(
            descriptor(&Type::Platform(Box::new(Type::String)), &[]).unwrap(),
            "Ljava/lang/String;"
        );
        let null = type_proto(&Type::Null, &[], &mut strings).unwrap();
        assert!(
            fields(&null.0)
                .iter()
                .any(|(field, value, _)| *field == 3 && *value == Some(1))
        );
    }

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
        let metadata = for_interfaces(&source, "Root", None).unwrap();
        let child = &metadata["Root$Child"];
        assert!(child.data2.iter().any(|name| name == "Root.Base"));
        assert!(!child.data2.iter().any(|name| name == "Root.Child.Base"));
    }

    #[test]
    fn package_metadata_uses_checked_class_names_for_unimported_source_types() {
        let source =
            crate::syntax::parse("package source\nfun echo(value: External): External = value")
                .unwrap();
        let ty = Type::Java("resolved/External".into());
        let checked = crate::check::Module {
            exposed_fields: std::collections::HashMap::new(),
            properties: Vec::new(),
            initializers: Vec::new(),
            locals: 0,
            lambdas: Vec::new(),
            constants: Vec::new(),
            functions: vec![crate::check::Function {
                annotations: Vec::new(),
                vararg: false,
                type_params: Vec::new(),
                defaults: Vec::new(),
                name: "echo".into(),
                jvm_name: None,
                private: false,
                is_override: false,
                params: vec![("value".into(), ty.clone())],
                result: ty,
                locals: 1,
                body: Vec::new(),
            }],
            class_decl: None,
            interface_decl: None,
            object_decl: None,
        };
        let metadata = for_checked_package(&source, &checked).unwrap();
        assert!(
            metadata
                .data2
                .iter()
                .any(|value| value == "resolved/External")
        );
        assert!(
            metadata
                .data2
                .iter()
                .any(|value| value == "(Lresolved/External;)Lresolved/External;")
        );
        assert!(
            !metadata
                .data2
                .iter()
                .any(|value| value.contains("source/External"))
        );
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
    #[test]
    fn annotated_internal_function_metadata_keeps_source_and_physical_names() {
        let module =
            crate::syntax::parse("@Mark internal fun sourceName(value: Int): Int = value").unwrap();
        let function = &module.functions[0];
        let mut strings = Strings::default();
        let proto = function_with_jvm_name(
            function,
            0,
            0,
            "sourceName$feature_module",
            &[],
            &mut strings,
        )
        .unwrap();
        let declaration = fields(&proto.0);
        let flags = declaration
            .iter()
            .find(|(field, _, _)| *field == 9)
            .unwrap()
            .1
            .unwrap();
        let source_name = declaration
            .iter()
            .find(|(field, _, _)| *field == 2)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(flags, 1);
        assert_eq!(strings.values[source_name as usize], "sourceName");
        let signature = fields(
            &declaration
                .iter()
                .find(|(field, _, _)| *field == 100)
                .unwrap()
                .2,
        );
        let physical_name = signature
            .iter()
            .find(|(field, _, _)| *field == 1)
            .unwrap()
            .1
            .unwrap();
        assert_eq!(
            strings.values[physical_name as usize],
            "sourceName$feature_module"
        );
    }

    #[test]
    fn property_annotation_sites_match_oracle_accessor_overrides() {
        for (site, mutability, property_flags, getter, setter, setter_parameter) in [
            ("field", "var", 1799, Some(6), Some(6), false),
            ("get", "var", 1798, Some(7), None, false),
            ("set", "var", 1798, None, Some(7), false),
            ("setparam", "var", 1798, None, Some(70), true),
            ("field", "val", 519, Some(6), None, false),
        ] {
            let module = crate::syntax::parse(&format!(
                "class Sample(@{site}:Mark {mutability} value: Int)"
            ))
            .unwrap();
            let metadata = class_metadata(
                module.class_decl.as_ref().unwrap(),
                "Sample",
                &[],
                false,
                None,
            )
            .unwrap();
            let declarations = metadata_fields(&metadata);
            let property = fields(
                &declarations
                    .iter()
                    .find(|(field, _, _)| *field == 10)
                    .unwrap()
                    .2,
            );
            let number = |id| {
                property
                    .iter()
                    .rev()
                    .find(|(field, _, _)| *field == id)
                    .and_then(|(_, number, _)| *number)
            };
            assert_eq!(number(11), Some(property_flags), "{site}");
            assert_eq!(number(7), getter, "{site}");
            assert_eq!(number(8), setter, "{site}");
            assert_eq!(
                property.iter().any(|(field, _, _)| *field == 6),
                setter_parameter,
                "{site}"
            );
        }
    }

    #[test]
    fn ordinary_annotation_flags_preserve_class_parameter_and_accessor_discovery() {
        let module = crate::syntax::parse(include_str!(
            "../tests/fixtures/compatibility_annotations_visibility/positive/Annotated.kt"
        ))
        .unwrap();
        let class = module.class_decl.as_ref().unwrap();
        let metadata = class_metadata(
            class,
            "compatibility/annotations/Annotated",
            &[],
            false,
            None,
        )
        .unwrap();
        let declarations = metadata_fields(&metadata);
        let number = |fields: &[(u32, Option<u64>, Vec<u8>)], id| {
            fields
                .iter()
                .rev()
                .find(|(field, _, _)| *field == id)
                .and_then(|(_, number, _)| *number)
                .unwrap_or(0)
        };
        assert_eq!(number(&declarations, 1) & 1, 1);
        let constructor = fields(
            &declarations
                .iter()
                .find(|(field, _, _)| *field == 8)
                .unwrap()
                .2,
        );
        let parameter = fields(
            &constructor
                .iter()
                .find(|(field, _, _)| *field == 2)
                .unwrap()
                .2,
        );
        assert_eq!(number(&parameter, 1) & 1, 1);
        let property = fields(
            &declarations
                .iter()
                .find(|(field, _, _)| *field == 10)
                .unwrap()
                .2,
        );
        assert_eq!(number(&property, 11), 1799);
        assert!(!property.iter().any(|(field, _, _)| *field == 7));
        assert_eq!(number(&property, 8), 71);
        let setter_parameter =
            fields(&property.iter().find(|(field, _, _)| *field == 6).unwrap().2);
        assert_eq!(number(&setter_parameter, 1) & 1, 1);
        let method = fields(
            &declarations
                .iter()
                .find(|(field, _, _)| *field == 9)
                .unwrap()
                .2,
        );
        assert_eq!(number(&method, 9) & 1, 1);
    }
}
