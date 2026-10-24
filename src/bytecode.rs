use crate::{
    Diagnostic, Result,
    check::{Module, Statement, StringMethod, Value, ValueKind},
    syntax::{ConstValue, EnumDecl, ObjectDecl, Type},
};
use std::collections::HashMap;

mod class_companion;
mod data;
mod defaults;
mod frames;
mod inline;
mod interfaces;
mod lambdas;
mod objects;
pub use interfaces::emit_interfaces;
pub use lambdas::emit_lambdas;
pub use objects::emit_dynamic_object;

fn failure(message: &str) -> Diagnostic {
    Diagnostic::new(0, message)
}
fn u2(out: &mut Vec<u8>, value: u16) {
    out.extend(value.to_be_bytes());
}
fn u4(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_be_bytes());
}

fn load_opcode(ty: &Type) -> u8 {
    if ty.is_reference() {
        0x19
    } else if ty == &Type::Long {
        0x16
    } else if ty == &Type::Float {
        0x17
    } else {
        0x15
    }
}
fn store_opcode(ty: &Type) -> u8 {
    if ty.is_reference() {
        0x3a
    } else if ty == &Type::Long {
        0x37
    } else if ty == &Type::Float {
        0x38
    } else {
        0x36
    }
}
fn return_opcode(ty: &Type) -> u8 {
    if ty.base() == &Type::Unit {
        0xb1
    } else if ty.is_reference() {
        0xb0
    } else if ty == &Type::Long {
        0xad
    } else if ty == &Type::Float {
        0xae
    } else {
        0xac
    }
}

fn parameter_slots(params: &[(String, Type)], start: usize) -> Vec<usize> {
    let mut slot = start;
    params
        .iter()
        .map(|(_, ty)| {
            let current = slot;
            slot += ty.slots();
            current
        })
        .collect()
}

fn nonnull(ty: &Type) -> bool {
    match ty {
        Type::Parameter { upper_bound, .. } => nonnull(upper_bound),
        _ => ty.is_reference() && !ty.is_nullable() && !matches!(ty, Type::Platform(_)),
    }
}

fn nullability_annotation(ty: &Type) -> Option<&'static str> {
    if !ty.is_reference() {
        None
    } else if ty.is_nullable() {
        Some("Lorg/jetbrains/annotations/Nullable;")
    } else if nonnull(ty) {
        Some("Lorg/jetbrains/annotations/NotNull;")
    } else {
        None
    }
}

fn generic_type(ty: &Type) -> String {
    match ty {
        Type::Nullable(inner) if !inner.is_reference() => boxed_descriptor(inner).into(),
        Type::Nullable(inner) | Type::Platform(inner) => generic_type(inner),
        Type::List(inner) | Type::MutableList(inner) => {
            format!("Ljava/util/List<{}>;", generic_argument(inner))
        }
        Type::Function { params, result } => format!(
            "Lkotlin/jvm/functions/Function{}<{}{}>;",
            params.len(),
            params.iter().map(generic_argument).collect::<String>(),
            if result.base() == &Type::Unit {
                "Lkotlin/Unit;".into()
            } else {
                generic_argument(result)
            }
        ),
        Type::Parameterized { name, arguments } => format!(
            "L{name}<{}>;",
            arguments
                .iter()
                .map(|arg| match &arg.ty {
                    None => "*".into(),
                    Some(ty) => format!(
                        "{}{}",
                        match arg.variance {
                            crate::syntax::Variance::Invariant => "",
                            crate::syntax::Variance::In => "-",
                            crate::syntax::Variance::Out if name == "java/util/List" =>
                                covariant_wildcard(ty),
                            crate::syntax::Variance::Out => "+",
                        },
                        generic_argument(ty)
                    ),
                })
                .collect::<String>()
        ),
        _ => ty.descriptor(),
    }
}

fn boxed_owner(ty: &Type) -> &'static str {
    match ty.base() {
        Type::Int => "java/lang/Integer",
        Type::Long => "java/lang/Long",
        Type::Float => "java/lang/Float",
        Type::Byte => "java/lang/Byte",
        Type::Char => "java/lang/Character",
        Type::Boolean => "java/lang/Boolean",
        _ => "java/lang/Object",
    }
}
fn boxed_descriptor(ty: &Type) -> &'static str {
    match ty.base() {
        Type::Int => "Ljava/lang/Integer;",
        Type::Long => "Ljava/lang/Long;",
        Type::Float => "Ljava/lang/Float;",
        Type::Byte => "Ljava/lang/Byte;",
        Type::Char => "Ljava/lang/Character;",
        Type::Boolean => "Ljava/lang/Boolean;",
        _ => "Ljava/lang/Object;",
    }
}
fn generic_argument(ty: &Type) -> String {
    if ty.base() == &Type::Unit {
        "Lkotlin/Unit;".into()
    } else if ty.is_reference() {
        generic_type(ty)
    } else {
        boxed_descriptor(ty).into()
    }
}

fn covariant_wildcard(ty: &Type) -> &'static str {
    match ty.base() {
        Type::Nullable(inner) => covariant_wildcard(inner),
        Type::Int
        | Type::Long
        | Type::Float
        | Type::Byte
        | Type::Char
        | Type::Boolean
        | Type::String
        | Type::Unit
        | Type::IntArray
        | Type::ByteArray
        | Type::CharArray
        | Type::StringArray => "",
        _ => "+",
    }
}

fn generic_parameter(ty: &Type) -> String {
    match ty {
        Type::Nullable(inner) | Type::Platform(inner) if inner.is_reference() => {
            generic_parameter(inner)
        }
        Type::Function { params, result } => format!(
            "Lkotlin/jvm/functions/Function{}<{}{}{}>;",
            params.len(),
            params
                .iter()
                .map(|t| format!("-{}", generic_argument(t)))
                .collect::<String>(),
            covariant_wildcard(result),
            if result.base() == &Type::Unit {
                "Lkotlin/Unit;".into()
            } else {
                generic_argument(result)
            }
        ),
        _ => generic_type(ty),
    }
}

fn generic_method(params: &[(String, Type)], result: &Type) -> Option<String> {
    let signature = format!(
        "({}){}",
        params
            .iter()
            .map(|(_, ty)| generic_parameter(ty))
            .collect::<String>(),
        generic_type(result)
    );
    let descriptor = format!(
        "({}){}",
        params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>(),
        result.descriptor()
    );
    (signature != descriptor).then_some(signature)
}

fn generic_function(function: &crate::check::Function) -> Option<String> {
    if function.type_params.is_empty() {
        return generic_method(&function.params, &function.result);
    }
    fn scoped(ty: &Type, names: &[&str], argument: bool) -> String {
        match ty {
            Type::Parameter { name, .. } if names.contains(&name.as_str()) => format!("T{name};"),
            Type::Nullable(inner) if !inner.is_reference() => boxed_descriptor(inner).into(),
            Type::Nullable(inner) | Type::Platform(inner) => scoped(inner, names, argument),
            Type::List(inner) | Type::MutableList(inner) => {
                format!("Ljava/util/List<{}>;", scoped(inner, names, true))
            }
            Type::Function { params, result } => format!(
                "Lkotlin/jvm/functions/Function{}<{}{}>;",
                params.len(),
                params
                    .iter()
                    .map(|t| scoped(t, names, true))
                    .collect::<String>(),
                scoped(result, names, true)
            ),
            Type::Parameterized { name, arguments } => format!(
                "L{name}<{}>;",
                arguments
                    .iter()
                    .map(|a| match &a.ty {
                        None => "*".into(),
                        Some(t) => format!(
                            "{}{}",
                            match a.variance {
                                crate::syntax::Variance::Invariant => "",
                                crate::syntax::Variance::In => "-",
                                crate::syntax::Variance::Out if name == "java/util/List" =>
                                    covariant_wildcard(t),
                                crate::syntax::Variance::Out => "+",
                            },
                            scoped(t, names, true)
                        ),
                    })
                    .collect::<String>()
            ),
            Type::Unit if argument => "Lkotlin/Unit;".into(),
            _ if argument => generic_argument(ty),
            _ => generic_type(ty),
        }
    }
    let names: Vec<_> = function
        .type_params
        .iter()
        .map(|t| t.name.as_str())
        .collect();
    let declarations = function
        .type_params
        .iter()
        .map(|p| {
            let bounds = if p.bounds.is_empty() {
                "Ljava/lang/Object;".into()
            } else {
                p.bounds
                    .iter()
                    .map(|b| scoped(b, &names, true))
                    .collect::<Vec<_>>()
                    .join(":")
            };
            format!("{}:{bounds}", p.name)
        })
        .collect::<String>();
    fn parameter(ty: &Type, names: &[&str]) -> String {
        match ty {
            Type::Nullable(inner) | Type::Platform(inner) if inner.is_reference() => {
                parameter(inner, names)
            }
            Type::Function { params, result } => format!(
                "Lkotlin/jvm/functions/Function{}<{}{}{}>;",
                params.len(),
                params
                    .iter()
                    .map(|t| format!("-{}", scoped(t, names, true)))
                    .collect::<String>(),
                covariant_wildcard(result),
                scoped(result, names, true)
            ),
            _ => scoped(ty, names, false),
        }
    }
    Some(format!(
        "<{declarations}>({}){}",
        function
            .params
            .iter()
            .map(|(_, t)| parameter(t, &names))
            .collect::<String>(),
        scoped(&function.result, &names, false)
    ))
}

fn add_method_attribute(method: &mut Vec<u8>, attribute: Vec<u8>) {
    let count = u16::from_be_bytes([method[6], method[7]]) + 1;
    method[6..8].copy_from_slice(&count.to_be_bytes());
    method.extend(attribute);
}

fn nullable_result(pool: &mut Pool, method: &mut Vec<u8>, ty: &Type) -> Result<()> {
    if ty.is_reference() && ty.is_nullable() {
        let mut attribute = Vec::new();
        u2(&mut attribute, pool.utf8("RuntimeInvisibleAnnotations")?);
        u4(&mut attribute, 6);
        u2(&mut attribute, 1);
        u2(
            &mut attribute,
            pool.utf8("Lorg/jetbrains/annotations/Nullable;")?,
        );
        u2(&mut attribute, 0);
        add_method_attribute(method, attribute);
    }
    Ok(())
}

fn reference_class(ty: &Type) -> Result<String> {
    let descriptor = ty.descriptor();
    if descriptor.starts_with('L') && descriptor.ends_with(';') {
        Ok(descriptor[1..descriptor.len() - 1].into())
    } else if descriptor.starts_with('[') {
        Ok(descriptor)
    } else {
        Err(failure("Expected a reference type"))
    }
}

fn invocation_types(descriptor: &str) -> Result<(Vec<Type>, Type)> {
    fn ty(bytes: &[u8], cursor: &mut usize) -> Result<Type> {
        let start = *cursor;
        let tag = *bytes
            .get(*cursor)
            .ok_or_else(|| failure("Invalid JVM invocation descriptor"))?;
        *cursor += 1;
        Ok(match tag {
            b'I' => Type::Int,
            b'J' => Type::Long,
            b'F' => Type::Float,
            b'B' => Type::Byte,
            b'C' => Type::Char,
            b'Z' => Type::Boolean,
            b'V' => Type::Unit,
            b'L' => {
                let end = bytes[*cursor..]
                    .iter()
                    .position(|b| *b == b';')
                    .ok_or_else(|| failure("Invalid JVM invocation reference"))?
                    + *cursor;
                let name = String::from_utf8(bytes[*cursor..end].to_vec())
                    .map_err(|_| failure("Invalid JVM invocation reference"))?;
                *cursor = end + 1;
                if name == "java/lang/String" {
                    Type::String
                } else {
                    Type::Java(name)
                }
            }
            b'[' => {
                ty(bytes, cursor)?;
                match &bytes[start..*cursor] {
                    b"[I" => Type::IntArray,
                    b"[B" => Type::ByteArray,
                    b"[C" => Type::CharArray,
                    b"[Ljava/lang/String;" => Type::StringArray,
                    _ => return Err(failure("Unsupported JVM invocation array type")),
                }
            }
            _ => return Err(failure("Unsupported JVM invocation descriptor")),
        })
    }
    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return Err(failure("Invalid JVM invocation descriptor"));
    }
    let mut cursor = 1;
    let mut params = Vec::new();
    while bytes.get(cursor) != Some(&b')') {
        let param = ty(bytes, &mut cursor)?;
        if param == Type::Unit {
            return Err(failure("Invalid void JVM parameter"));
        }
        params.push(param);
    }
    cursor += 1;
    let result = ty(bytes, &mut cursor)?;
    if cursor != bytes.len() {
        return Err(failure("Invalid JVM invocation descriptor"));
    }
    Ok((params, result))
}

fn primitive_array_type(ty: &Type) -> Result<u8> {
    match ty.base() {
        Type::ByteArray => Ok(8),
        Type::CharArray => Ok(5),
        Type::IntArray => Ok(10),
        _ => Err(failure("Unsupported primitive array type")),
    }
}

#[derive(Clone, Hash, Eq, PartialEq)]
enum Constant {
    Utf8(Vec<u8>),
    Integer(i32),
    Float(u32),
    Long(i64),
    Reserved,
    Class(u16),
    String(u16),
    NameType(u16, u16),
    Method(u16, u16),
    InterfaceMethod(u16, u16),
    Field(u16, u16),
}
#[derive(Default)]
struct Pool {
    entries: Vec<Constant>,
    indices: HashMap<Constant, u16>,
}
impl Pool {
    fn add(&mut self, entry: Constant) -> Result<u16> {
        if let Some(index) = self.indices.get(&entry) {
            return Ok(*index);
        }
        let width = if matches!(entry, Constant::Long(_)) {
            2
        } else {
            1
        };
        if self.entries.len() + width > 65534 {
            return Err(failure("JVM constant pool limit exceeded"));
        }
        let index = (self.entries.len() + 1) as u16;
        self.entries.push(entry.clone());
        if width == 2 {
            self.entries.push(Constant::Reserved);
        }
        self.indices.insert(entry, index);
        Ok(index)
    }
    fn utf8(&mut self, value: &str) -> Result<u16> {
        self.utf16(&value.encode_utf16().collect::<Vec<_>>())
    }
    fn utf16(&mut self, value: &[u16]) -> Result<u16> {
        let mut bytes = Vec::new();
        for &unit in value {
            match unit {
                1..=0x7f => bytes.push(unit as u8),
                0..=0x7ff => bytes.extend([0xc0 | (unit >> 6) as u8, 0x80 | (unit & 0x3f) as u8]),
                _ => bytes.extend([
                    0xe0 | (unit >> 12) as u8,
                    0x80 | ((unit >> 6) & 0x3f) as u8,
                    0x80 | (unit & 0x3f) as u8,
                ]),
            }
            if bytes.len() > 65535 {
                return Err(failure("JVM constant string exceeds 65535 encoded bytes"));
            }
        }
        self.add(Constant::Utf8(bytes))
    }
    fn string_units(&mut self, value: &[u16]) -> Result<u16> {
        let value = self.utf16(value)?;
        self.add(Constant::String(value))
    }
    fn class(&mut self, name: &str) -> Result<u16> {
        let name = self.utf8(name)?;
        self.add(Constant::Class(name))
    }
    fn string(&mut self, value: &str) -> Result<u16> {
        let value = self.utf8(value)?;
        self.add(Constant::String(value))
    }
    fn method(&mut self, owner: &str, name: &str, descriptor: &str) -> Result<u16> {
        let class = self.class(owner)?;
        let name = self.utf8(name)?;
        let descriptor = self.utf8(descriptor)?;
        let pair = self.add(Constant::NameType(name, descriptor))?;
        self.add(Constant::Method(class, pair))
    }
    fn interface_method(&mut self, owner: &str, name: &str, descriptor: &str) -> Result<u16> {
        let class = self.class(owner)?;
        let name = self.utf8(name)?;
        let descriptor = self.utf8(descriptor)?;
        let pair = self.add(Constant::NameType(name, descriptor))?;
        self.add(Constant::InterfaceMethod(class, pair))
    }
    fn field(&mut self, owner: &str, name: &str, descriptor: &str) -> Result<u16> {
        let class = self.class(owner)?;
        let name = self.utf8(name)?;
        let descriptor = self.utf8(descriptor)?;
        let pair = self.add(Constant::NameType(name, descriptor))?;
        self.add(Constant::Field(class, pair))
    }
    fn write(&self, out: &mut Vec<u8>) {
        u2(out, (self.entries.len() + 1) as u16);
        for entry in &self.entries {
            match entry {
                Constant::Utf8(value) => {
                    out.push(1);
                    u2(out, value.len() as u16);
                    out.extend(value);
                }
                Constant::Integer(value) => {
                    out.push(3);
                    out.extend(value.to_be_bytes());
                }
                Constant::Float(value) => {
                    out.push(4);
                    out.extend(value.to_be_bytes());
                }
                Constant::Long(value) => {
                    out.push(5);
                    out.extend(value.to_be_bytes());
                }
                Constant::Reserved => {}
                Constant::Class(index) => {
                    out.push(7);
                    u2(out, *index);
                }
                Constant::String(index) => {
                    out.push(8);
                    u2(out, *index);
                }
                Constant::Field(a, b) => {
                    out.push(9);
                    u2(out, *a);
                    u2(out, *b);
                }
                Constant::InterfaceMethod(a, b) => {
                    out.push(11);
                    u2(out, *a);
                    u2(out, *b);
                }
                Constant::Method(a, b) => {
                    out.push(10);
                    u2(out, *a);
                    u2(out, *b);
                }
                Constant::NameType(a, b) => {
                    out.push(12);
                    u2(out, *a);
                    u2(out, *b);
                }
            }
        }
    }
}
struct Code<'a> {
    pool: &'a mut Pool,
    functions: &'a [crate::check::Function],
    owner: &'a str,
    bytes: Vec<u8>,
    labels: Vec<Option<usize>>,
    branches: Vec<(usize, usize)>,
    loops: Vec<(usize, usize)>,
    max_stack: usize,
    lambdas: &'a [crate::check::LambdaDecl],
    inline_local_maps: Vec<Vec<u16>>,
    inline_returns: Vec<usize>,
}
fn terminal_block(body: &[Statement]) -> bool {
    match body.last() {
        Some(
            Statement::Return(_)
            | Statement::NonLocalReturn(_)
            | Statement::Throw(_)
            | Statement::ThrowException { .. }
            | Statement::Break
            | Statement::Continue,
        ) => true,
        Some(Statement::Expression(value)) => terminal_value(value),
        Some(Statement::If(_, yes, no)) => terminal_block(yes) && terminal_block(no),
        _ => false,
    }
}
fn terminal_value(value: &Value) -> bool {
    match &value.kind {
        ValueKind::Abrupt(body) => terminal_block(body),
        ValueKind::Let { body, .. } => terminal_value(body),
        ValueKind::If(_, yes, no) => terminal_value(yes) && terminal_value(no),
        _ => false,
    }
}
impl Code<'_> {
    fn adapt_value(&mut self, from: &Type, to: &Type) -> Result<()> {
        if from.base() == &Type::Unit {
            if to.base() != &Type::Unit {
                if !to.is_reference() {
                    return Err(failure("Cannot adapt Unit to a primitive JVM value"));
                }
                self.field_reference(0xb2, "kotlin/Unit", "INSTANCE", "Lkotlin/Unit;")?;
            }
        } else if to.base() == &Type::Unit {
            self.bytes.push(if from.slots() == 2 { 0x58 } else { 0x57 });
        } else if from.is_reference() && !to.is_reference() {
            self.generic_result(to)?;
        } else if !from.is_reference() && to.is_reference() {
            self.box_value(from)?;
            let target = reference_class(to)?;
            if target != "java/lang/Object" && target != boxed_owner(from) {
                self.class_reference(0xc0, &target)?;
            }
        } else if from.is_reference() && to.is_reference() && from.descriptor() != to.descriptor() {
            let target = reference_class(to)?;
            if target != "java/lang/Object" {
                self.class_reference(0xc0, &target)?;
            }
        } else if from.descriptor() != to.descriptor() {
            return Err(failure("Unsupported JVM primitive adaptation"));
        }
        Ok(())
    }
    fn call_arguments(&mut self, args: &[Value], params: &[Type]) -> Result<()> {
        if args.len() != params.len() {
            return Err(failure("JVM invocation argument count mismatch"));
        }
        let mut slots = 0;
        for (arg, param) in args.iter().zip(params) {
            self.max_stack = self.max_stack.max(slots + arg.stack() + 2);
            self.value(arg)?;
            self.adapt_value(&arg.ty, param)?;
            slots += param.slots();
        }
        self.max_stack = self.max_stack.max(slots);
        Ok(())
    }
    fn box_value(&mut self, ty: &Type) -> Result<()> {
        self.method_reference(
            0xb8,
            boxed_owner(ty),
            "valueOf",
            &format!("({}){}", ty.descriptor(), boxed_descriptor(ty)),
        )
    }
    fn unbox_value(&mut self, ty: &Type) -> Result<()> {
        let name = match ty {
            Type::Int => "intValue",
            Type::Long => "longValue",
            Type::Float => "floatValue",
            Type::Byte => "byteValue",
            Type::Char => "charValue",
            Type::Boolean => "booleanValue",
            _ => return Err(failure("Unsupported primitive unboxing")),
        };
        self.method_reference(
            0xb6,
            boxed_owner(ty),
            name,
            &format!("(){}", ty.descriptor()),
        )
    }
    fn generic_result(&mut self, ty: &Type) -> Result<()> {
        if ty.is_reference() {
            return self.class_reference(0xc0, &reference_class(ty)?);
        }
        let numeric = matches!(ty, Type::Int | Type::Long | Type::Float | Type::Byte);
        let owner = if numeric {
            "java/lang/Number"
        } else {
            boxed_owner(ty)
        };
        self.class_reference(0xc0, owner)?;
        if numeric {
            let name = match ty {
                Type::Int => "intValue",
                Type::Long => "longValue",
                Type::Float => "floatValue",
                Type::Byte => "byteValue",
                _ => unreachable!(),
            };
            self.method_reference(0xb6, owner, name, &format!("(){}", ty.descriptor()))
        } else {
            self.unbox_value(ty)
        }
    }
    fn boxed_value(&mut self, value: &Value) -> Result<()> {
        match &value.kind {
            ValueKind::If(condition, yes, no) => {
                let otherwise = self.label();
                let end = self.label();
                self.value(condition)?;
                self.branch(0x99, otherwise);
                self.boxed_value(yes)?;
                if !terminal_value(yes) {
                    self.branch(0xa7, end);
                }
                self.mark(otherwise);
                self.boxed_value(no)?;
                self.mark(end);
                return Ok(());
            }
            ValueKind::Let {
                slot,
                value: initializer,
                body,
            } => {
                self.value(initializer)?;
                self.bytes
                    .extend([store_opcode(&initializer.ty), self.local_slot(*slot)?]);
                return self.boxed_value(body);
            }
            ValueKind::Abrupt(body) => return self.block(body),
            ValueKind::Unbox(asserted) => {
                if let ValueKind::RequireNotNull(operand) = &asserted.kind {
                    if self.erased_list_value(operand)? {
                        self.bytes.push(0x59);
                        self.method_reference(
                            0xb8,
                            "kotlin/jvm/internal/Intrinsics",
                            "checkNotNull",
                            "(Ljava/lang/Object;)V",
                        )?;
                        return self.class_reference(0xc0, boxed_owner(&value.ty));
                    }
                }
            }
            _ => {}
        }
        if self.erased_list_value(value)? {
            return self.class_reference(0xc0, boxed_owner(&value.ty));
        }
        self.value(value)?;
        self.box_value(&value.ty)
    }
    fn erased_list_value(&mut self, value: &Value) -> Result<bool> {
        match &value.kind {
            ValueKind::ListGet(receiver, index) => {
                self.value(receiver)?;
                self.value(index)?;
                let method =
                    self.pool
                        .interface_method("java/util/List", "get", "(I)Ljava/lang/Object;")?;
                self.reference(0xb9, method);
                self.bytes.extend([2, 0]);
            }
            ValueKind::ListSet(receiver, index, element) => {
                self.value(receiver)?;
                self.value(index)?;
                self.value(element)?;
                if !element.ty.is_reference() {
                    self.box_value(&element.ty)?;
                }
                let method = self.pool.interface_method(
                    "java/util/List",
                    "set",
                    "(ILjava/lang/Object;)Ljava/lang/Object;",
                )?;
                self.reference(0xb9, method);
                self.bytes.extend([3, 0]);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn label(&mut self) -> usize {
        let id = self.labels.len();
        self.labels.push(None);
        id
    }
    fn mark(&mut self, label: usize) {
        self.labels[label] = Some(self.bytes.len());
    }
    fn branch(&mut self, opcode: u8, label: usize) {
        self.branches.push((self.bytes.len(), label));
        self.bytes.extend([opcode, 0, 0]);
    }
    fn reference(&mut self, opcode: u8, index: u16) {
        self.bytes.push(opcode);
        u2(&mut self.bytes, index);
    }
    fn method_reference(
        &mut self,
        opcode: u8,
        owner: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<()> {
        let index = self.pool.method(owner, name, descriptor)?;
        self.reference(opcode, index);
        Ok(())
    }
    fn field_reference(
        &mut self,
        opcode: u8,
        owner: &str,
        name: &str,
        descriptor: &str,
    ) -> Result<()> {
        let index = self.pool.field(owner, name, descriptor)?;
        self.reference(opcode, index);
        Ok(())
    }
    fn class_reference(&mut self, opcode: u8, name: &str) -> Result<()> {
        let index = self.pool.class(name)?;
        self.reference(opcode, index);
        Ok(())
    }
    fn integer(&mut self, value: i32) -> Result<()> {
        if (-1..=5).contains(&value) {
            self.bytes.push((value + 3) as u8);
        } else if let Ok(value) = i8::try_from(value) {
            self.bytes.extend([0x10, value as u8]);
        } else if let Ok(value) = i16::try_from(value) {
            self.bytes.push(0x11);
            self.bytes.extend(value.to_be_bytes());
        } else {
            let index = self.pool.add(Constant::Integer(value))?;
            self.reference(0x13, index);
        }
        Ok(())
    }
    fn comparison(&mut self, opcode: u8) {
        let yes = self.label();
        let end = self.label();
        self.branch(opcode, yes);
        self.bytes.push(0x03);
        self.branch(0xa7, end);
        self.mark(yes);
        self.bytes.push(0x04);
        self.mark(end);
    }
    fn value(&mut self, value: &Value) -> Result<()> {
        self.max_stack = self.max_stack.max(value.stack());
        match &value.kind {
            ValueKind::CompiledInline {
                method,
                args,
                local_base,
                lambda_bases,
            } => {
                self.compiled_inline(method, args, *local_base, lambda_bases, &value.ty)?;
            }
            ValueKind::Lambda { owner, captures } => {
                self.class_reference(0xbb, owner)?;
                self.bytes.push(0x59);
                for capture in captures {
                    self.value(capture)?;
                }
                self.max_stack = self
                    .max_stack
                    .max(2 + captures.iter().map(|v| v.ty.slots()).sum::<usize>());
                self.method_reference(
                    0xb7,
                    owner,
                    "<init>",
                    &format!(
                        "({})V",
                        captures
                            .iter()
                            .map(|v| v.ty.descriptor())
                            .collect::<String>()
                    ),
                )?;
            }
            ValueKind::FunctionInvoke { function, args } => {
                let Type::Function { params, .. } = function.ty.base() else {
                    return Err(failure("Expected a function type for invoke"));
                };
                if params.len() != args.len() || params.len() > 22 {
                    return Err(failure("Unsupported function invocation arity"));
                }
                self.value(function)?;
                for arg in args {
                    self.value(arg)?;
                    self.adapt_value(&arg.ty, &Type::Java("java/lang/Object".into()))?;
                }
                self.max_stack = self.max_stack.max(3 + args.len() * 2);
                let owner = format!("kotlin/jvm/functions/Function{}", params.len());
                let descriptor = format!(
                    "({})Ljava/lang/Object;",
                    "Ljava/lang/Object;".repeat(params.len())
                );
                let method = self.pool.interface_method(&owner, "invoke", &descriptor)?;
                self.reference(0xb9, method);
                self.bytes.extend([(args.len() + 1) as u8, 0]);
                self.adapt_value(&Type::Java("java/lang/Object".into()), &value.ty)?;
            }
            ValueKind::Float(bits) => {
                let constant = self.pool.add(Constant::Float(*bits))?;
                self.reference(0x13, constant);
            }
            ValueKind::Box(inner) => self.boxed_value(inner)?,
            ValueKind::Unbox(inner) => {
                if let ValueKind::RequireNotNull(operand) = &inner.kind {
                    if self.erased_list_value(operand)? {
                        self.bytes.push(0x59);
                        self.method_reference(
                            0xb8,
                            "kotlin/jvm/internal/Intrinsics",
                            "checkNotNull",
                            "(Ljava/lang/Object;)V",
                        )?;
                        self.generic_result(&value.ty)?;
                        return Ok(());
                    }
                }
                self.value(inner)?;
                self.unbox_value(&value.ty)?;
            }
            ValueKind::New {
                owner,
                descriptor,
                args,
            } => {
                self.class_reference(0xbb, owner)?;
                self.bytes.push(0x59);
                for arg in args {
                    self.value(arg)?;
                }
                self.method_reference(0xb7, owner, "<init>", descriptor)?;
            }
            ValueKind::JavaField {
                owner,
                name,
                receiver,
            } => {
                if let Some(receiver) = receiver {
                    self.value(receiver)?;
                }
                let object_cell = owner == "kotlin/jvm/internal/Ref$ObjectRef" && name == "element";
                self.field_reference(
                    if receiver.is_some() { 0xb4 } else { 0xb2 },
                    owner,
                    name,
                    &if object_cell {
                        "Ljava/lang/Object;".into()
                    } else {
                        value.ty.descriptor()
                    },
                )?;
                if object_cell {
                    self.adapt_value(&Type::Java("java/lang/Object".into()), &value.ty)?;
                }
            }
            ValueKind::ListFactory { mutable, elements } => {
                if elements.is_empty() && !mutable {
                    self.method_reference(
                        0xb8,
                        "kotlin/collections/CollectionsKt",
                        "emptyList",
                        "()Ljava/util/List;",
                    )?;
                } else if elements.len() == 1 && !mutable {
                    self.value(&elements[0])?;
                    if !elements[0].ty.is_reference() {
                        self.box_value(&elements[0].ty)?;
                    }
                    self.method_reference(
                        0xb8,
                        "kotlin/collections/CollectionsKt",
                        "listOf",
                        "(Ljava/lang/Object;)Ljava/util/List;",
                    )?;
                } else {
                    self.integer(elements.len() as i32)?;
                    let element_class = match value.ty.base() {
                        Type::List(element) | Type::MutableList(element) => {
                            if element.is_reference() {
                                reference_class(element)?
                            } else {
                                boxed_owner(element).into()
                            }
                        }
                        _ => return Err(failure("Expected a list factory type")),
                    };
                    self.class_reference(0xbd, &element_class)?;
                    for (index, element) in elements.iter().enumerate() {
                        self.bytes.push(0x59);
                        self.integer(index as i32)?;
                        self.value(element)?;
                        if !element.ty.is_reference() {
                            self.box_value(&element.ty)?;
                        }
                        self.bytes.push(0x53);
                    }
                    self.method_reference(
                        0xb8,
                        "kotlin/collections/CollectionsKt",
                        if *mutable { "mutableListOf" } else { "listOf" },
                        "([Ljava/lang/Object;)Ljava/util/List;",
                    )?;
                }
            }
            ValueKind::Abrupt(body) => self.block(body)?,
            ValueKind::DefaultCall(id, args, mask) => {
                self.max_stack = self
                    .max_stack
                    .max(2 + args.iter().map(|value| value.ty.slots()).sum::<usize>());
                let function = &self.functions[*id];
                let (params, result) = invocation_types(&function.descriptor())?;
                self.call_arguments(args, &params)?;
                self.integer(*mask as i32)?;
                self.bytes.push(0x01);
                let descriptor = function
                    .descriptor()
                    .replacen(')', "ILjava/lang/Object;)", 1);
                self.method_reference(
                    0xb8,
                    self.owner,
                    &format!("{}$default", function.name),
                    &descriptor,
                )?;
                self.adapt_value(&result, &value.ty)?;
            }
            ValueKind::Unit => {}
            ValueKind::String(units) => {
                let constant = self.pool.string_units(units)?;
                self.reference(0x13, constant);
            }
            ValueKind::StringConcat(a, b) => {
                let class = self.pool.class("java/lang/StringBuilder")?;
                self.reference(0xbb, class);
                self.bytes.push(0x59);
                let constructor = self
                    .pool
                    .method("java/lang/StringBuilder", "<init>", "()V")?;
                self.reference(0xb7, constructor);
                for part in [a, b] {
                    self.value(part)?;
                    let descriptor = if part.ty.is_reference() {
                        if part.ty.base() == &Type::String {
                            "(Ljava/lang/String;)Ljava/lang/StringBuilder;"
                        } else {
                            "(Ljava/lang/Object;)Ljava/lang/StringBuilder;"
                        }
                    } else {
                        match part.ty.base() {
                            Type::Long => "(J)Ljava/lang/StringBuilder;",
                            Type::Float => "(F)Ljava/lang/StringBuilder;",
                            Type::Char => "(C)Ljava/lang/StringBuilder;",
                            Type::Boolean => "(Z)Ljava/lang/StringBuilder;",
                            Type::Int | Type::Byte => "(I)Ljava/lang/StringBuilder;",
                            _ => return Err(failure("Unsupported string concatenation operand")),
                        }
                    };
                    let append =
                        self.pool
                            .method("java/lang/StringBuilder", "append", descriptor)?;
                    self.reference(0xb6, append);
                }
                let result = self.pool.method(
                    "java/lang/StringBuilder",
                    "toString",
                    "()Ljava/lang/String;",
                )?;
                self.reference(0xb6, result);
            }
            ValueKind::StringCall(method, args) => {
                for arg in args {
                    self.value(arg)?;
                }
                let (owner, name, descriptor, opcode) = match method {
                    StringMethod::Length => ("java/lang/String", "length", "()I", 0xb6),
                    StringMethod::IsEmpty => ("java/lang/String", "isEmpty", "()Z", 0xb6),
                    StringMethod::RemovePrefix => (
                        "kotlin/text/StringsKt",
                        "removePrefix",
                        "(Ljava/lang/String;Ljava/lang/CharSequence;)Ljava/lang/String;",
                        0xb8,
                    ),
                    StringMethod::StartsWith => (
                        "java/lang/String",
                        "startsWith",
                        "(Ljava/lang/String;)Z",
                        0xb6,
                    ),
                    StringMethod::EndsWith => (
                        "java/lang/String",
                        "endsWith",
                        "(Ljava/lang/String;)Z",
                        0xb6,
                    ),
                    StringMethod::Substring => (
                        "java/lang/String",
                        "substring",
                        "(I)Ljava/lang/String;",
                        0xb6,
                    ),
                    StringMethod::SubstringRange => (
                        "java/lang/String",
                        "substring",
                        "(II)Ljava/lang/String;",
                        0xb6,
                    ),
                    StringMethod::CompareTo => (
                        "java/lang/String",
                        "compareTo",
                        "(Ljava/lang/String;)I",
                        0xb6,
                    ),
                };
                let method = self.pool.method(owner, name, descriptor)?;
                self.reference(opcode, method);
            }
            ValueKind::JavaCall { method, args } => {
                let (mut params, result) = invocation_types(&method.descriptor)?;
                if method.access & 8 == 0 {
                    params.insert(0, Type::Java(method.owner.clone()));
                }
                self.call_arguments(args, &params)?;
                let opcode = if method.access & 8 != 0 { 0xb8 } else { 0xb6 };
                let constant = self
                    .pool
                    .method(&method.owner, &method.name, &method.descriptor)?;
                self.reference(opcode, constant);
                self.adapt_value(&result, &value.ty)?;
            }
            ValueKind::Let { slot, value, body } => {
                self.value(value)?;
                self.bytes
                    .extend([store_opcode(&value.ty), self.local_slot(*slot)?]);
                self.value(body)?;
            }
            ValueKind::NewArray(array_type, size) => {
                self.value(size)?;
                self.bytes.extend([0xbc, primitive_array_type(array_type)?]);
            }
            ValueKind::Convert(target, value) => {
                self.value(value)?;
                match target {
                    Type::Long => self.bytes.push(0x85),
                    Type::Byte => self.bytes.push(0x91),
                    Type::Char => self.bytes.push(0x92),
                    Type::Int => {}
                    _ => return Err(failure("Unsupported primitive conversion")),
                }
            }
            ValueKind::StaticField { owner, name } => {
                self.field_reference(0xb2, owner, name, &value.ty.descriptor())?;
            }
            ValueKind::ObjectInstance(owner) => {
                self.field_reference(0xb2, owner, "INSTANCE", &format!("L{owner};"))?;
            }
            ValueKind::ThreadLocalGet(receiver, array_type) => {
                self.value(receiver)?;
                self.method_reference(
                    0xb6,
                    "java/lang/ThreadLocal",
                    "get",
                    "()Ljava/lang/Object;",
                )?;
                self.class_reference(0xc0, &array_type.descriptor())?;
            }
            ValueKind::ThreadLocalSet(receiver, value) => {
                self.value(receiver)?;
                self.value(value)?;
                self.method_reference(
                    0xb6,
                    "java/lang/ThreadLocal",
                    "set",
                    "(Ljava/lang/Object;)V",
                )?;
            }
            ValueKind::CopyOf(array, length) => {
                self.value(array)?;
                self.value(length)?;
                let array_type = array.ty.descriptor();
                self.method_reference(
                    0xb8,
                    "java/util/Arrays",
                    "copyOf",
                    &format!("({array_type}I){array_type}"),
                )?;
            }
            ValueKind::StringFromChars(args) => {
                self.max_stack = self.max_stack.max(
                    2 + args
                        .iter()
                        .enumerate()
                        .map(|(index, value)| index + value.stack())
                        .max()
                        .unwrap_or(0),
                );
                self.class_reference(0xbb, "java/lang/String")?;
                self.bytes.push(0x59);
                for arg in args {
                    self.value(arg)?;
                }
                self.method_reference(0xb7, "java/lang/String", "<init>", "([CII)V")?;
            }
            ValueKind::StaticCall {
                owner,
                name,
                descriptor,
                args,
            } => {
                let (params, result) = invocation_types(descriptor)?;
                self.call_arguments(args, &params)?;
                self.method_reference(0xb8, owner, name, descriptor)?;
                self.adapt_value(&result, &value.ty)?;
            }
            ValueKind::Increment { slot, prefix } => {
                let slot = self.local_slot(*slot)?;
                if *prefix {
                    self.bytes.extend([0x84, slot, 1]);
                }
                self.bytes.extend([0x15, slot]);
                if !*prefix {
                    self.bytes.extend([0x84, slot, 1]);
                }
            }
            ValueKind::CellIncrement { cell, prefix } => {
                self.value(cell)?;
                self.bytes.push(0x59);
                self.field_reference(0xb4, "kotlin/jvm/internal/Ref$IntRef", "element", "I")?;
                if *prefix {
                    self.bytes.extend([0x04, 0x60, 0x5a]);
                } else {
                    self.bytes.extend([0x5a, 0x04, 0x60]);
                }
                self.field_reference(0xb5, "kotlin/jvm/internal/Ref$IntRef", "element", "I")?;
            }
            ValueKind::RequireNotNull(inner) => {
                self.value(inner)?;
                self.bytes.push(0x59);
                self.method_reference(
                    0xb8,
                    "kotlin/jvm/internal/Intrinsics",
                    "checkNotNull",
                    "(Ljava/lang/Object;)V",
                )?;
                self.class_reference(0xc0, &reference_class(&value.ty)?)?;
            }
            ValueKind::If(condition, yes, no) => {
                let otherwise = self.label();
                let end = self.label();
                self.value(condition)?;
                self.branch(0x99, otherwise);
                self.value(yes)?;
                if !terminal_value(yes) {
                    self.branch(0xa7, end);
                }
                self.mark(otherwise);
                self.value(no)?;
                self.mark(end);
            }
            ValueKind::IntToString(value) => {
                self.value(value)?;
                let method = self.pool.method(
                    "java/lang/String",
                    "valueOf",
                    match value.ty {
                        Type::Long => "(J)Ljava/lang/String;",
                        Type::Float => "(F)Ljava/lang/String;",
                        Type::Char => "(C)Ljava/lang/String;",
                        Type::Boolean => "(Z)Ljava/lang/String;",
                        _ => "(I)Ljava/lang/String;",
                    },
                )?;
                self.reference(0xb8, method);
            }
            ValueKind::CompanionArray { owner, name } => {
                let method = self.pool.method(
                    owner,
                    &format!("access$get{name}$p"),
                    "()[Ljava/lang/String;",
                )?;
                self.reference(0xb8, method);
            }
            ValueKind::MemberCall {
                method,
                interface,
                args,
            } => {
                let (mut params, result) = invocation_types(&method.descriptor)?;
                params.insert(0, Type::Java(method.owner.clone()));
                self.call_arguments(args, &params)?;
                if *interface {
                    let constant = self.pool.interface_method(
                        &method.owner,
                        &method.name,
                        &method.descriptor,
                    )?;
                    self.reference(0xb9, constant);
                    self.bytes.extend([
                        u8::try_from(params.iter().map(Type::slots).sum::<usize>())
                            .map_err(|_| failure("Too many interface call arguments"))?,
                        0,
                    ]);
                } else {
                    let constant =
                        self.pool
                            .method(&method.owner, &method.name, &method.descriptor)?;
                    self.reference(if method.access & 2 != 0 { 0xb7 } else { 0xb6 }, constant);
                }
                self.adapt_value(&result, &value.ty)?;
            }
            ValueKind::NewIntArray(size) => {
                self.value(size)?;
                self.bytes.extend([0xbc, 10]);
            }
            ValueKind::CopyInto(args) => {
                for arg in args {
                    self.value(arg)?;
                }
                let method =
                    self.pool
                        .method("kotlin/collections/ArraysKt", "copyInto", "([I[IIII)[I")?;
                self.reference(0xb8, method);
            }
            ValueKind::NotNull(inner, message) => {
                self.value(inner)?;
                self.bytes.push(0x59);
                let check = if message.is_empty() {
                    self.pool.method(
                        "kotlin/jvm/internal/Intrinsics",
                        "checkNotNull",
                        "(Ljava/lang/Object;)V",
                    )?
                } else {
                    let message = self.pool.string(message)?;
                    self.reference(0x13, message);
                    self.pool.method(
                        "kotlin/jvm/internal/Intrinsics",
                        "checkNotNullExpressionValue",
                        "(Ljava/lang/Object;Ljava/lang/String;)V",
                    )?
                };
                self.reference(0xb8, check);
            }
            ValueKind::Int(n) => self.integer(*n)?,
            ValueKind::Long(n) => {
                if matches!(*n, 0 | 1) {
                    self.bytes.push(0x09 + *n as u8);
                } else {
                    let constant = self.pool.add(Constant::Long(*n))?;
                    self.reference(0x14, constant);
                }
            }
            ValueKind::Null => self.bytes.push(0x01),
            ValueKind::IsNull(inner) => {
                self.value(inner)?;
                self.comparison(0xc6);
            }
            ValueKind::ListSize(inner) => {
                self.value(inner)?;
                let method = self
                    .pool
                    .interface_method("java/util/List", "size", "()I")?;
                self.reference(0xb9, method);
                self.bytes.extend([1, 0]);
            }
            ValueKind::ListGet(..) | ValueKind::ListSet(..) => {
                self.erased_list_value(value)?;
                self.generic_result(&value.ty)?;
            }
            ValueKind::Boolean(b) => self.bytes.push(if *b { 0x04 } else { 0x03 }),
            ValueKind::Field(name, receiver) => {
                self.value(receiver)?;
                let owner = match receiver.ty.base() {
                    Type::Java(owner) => owner.as_str(),
                    _ => self.owner,
                };
                let field = self.pool.field(owner, name, &value.ty.descriptor())?;
                self.reference(0xb4, field);
            }
            ValueKind::Local(slot) => self
                .bytes
                .extend([load_opcode(&value.ty), self.local_slot(*slot)?]),
            ValueKind::Negate(inner) => {
                self.value(inner)?;
                self.bytes.push(if inner.ty == Type::Long {
                    0x75
                } else if inner.ty == Type::Float {
                    0x76
                } else {
                    0x74
                });
            }
            ValueKind::Not(inner) => {
                self.value(inner)?;
                self.bytes.extend([0x04, 0x82]);
            }
            ValueKind::Length(inner) => {
                self.value(inner)?;
                self.bytes.push(0xbe);
            }
            ValueKind::Index(array, index) => {
                self.value(array)?;
                self.value(index)?;
                self.bytes.push(match array.ty.base() {
                    Type::IntArray => 0x2e,
                    Type::ByteArray => 0x33,
                    Type::CharArray => 0x34,
                    Type::StringArray => 0x32,
                    _ => return Err(failure("Unsupported array indexing operand")),
                });
            }
            ValueKind::Compare(a, b) => {
                self.value(a)?;
                self.value(b)?;
                let method = self.pool.method("java/lang/Integer", "compare", "(II)I")?;
                self.reference(0xb8, method);
            }
            ValueKind::Call(id, args) => {
                let function = &self.functions[*id];
                let (params, result) = invocation_types(&function.descriptor())?;
                self.call_arguments(args, &params)?;
                let name = if function.private && !self.lambdas.is_empty() {
                    format!("access${}", function.name)
                } else {
                    function.name.clone()
                };
                let method = self
                    .pool
                    .method(self.owner, &name, &function.descriptor())?;
                self.reference(0xb8, method);
                self.adapt_value(&result, &value.ty)?;
            }
            ValueKind::Binary(op, a, b) if op == "&&" || op == "||" => {
                let short = self.label();
                let end = self.label();
                self.value(a)?;
                self.branch(if op == "&&" { 0x99 } else { 0x9a }, short);
                self.value(b)?;
                self.branch(0xa7, end);
                self.mark(short);
                self.bytes.push(if op == "&&" { 0x03 } else { 0x04 });
                self.mark(end);
            }
            ValueKind::Binary(op, a, b) if a.ty.base() == &Type::String => {
                self.value(a)?;
                self.value(b)?;
                let equal = self.pool.method(
                    "kotlin/jvm/internal/Intrinsics",
                    "areEqual",
                    "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                )?;
                self.reference(0xb8, equal);
                if op == "!=" {
                    self.bytes.extend([0x04, 0x82]);
                } else if op != "==" {
                    return Err(failure("Unsupported string binary operation"));
                }
            }
            ValueKind::Binary(op, a, b) => {
                self.value(a)?;
                self.value(b)?;
                if a.ty == Type::Long {
                    match op.as_str() {
                        "+" => self.bytes.push(0x61),
                        "-" => self.bytes.push(0x65),
                        "*" => self.bytes.push(0x69),
                        "/" => self.bytes.push(0x6d),
                        "%" => self.bytes.push(0x71),
                        "shl" => self.bytes.push(0x79),
                        "shr" => self.bytes.push(0x7b),
                        "ushr" => self.bytes.push(0x7d),
                        "and" => self.bytes.push(0x7f),
                        "or" => self.bytes.push(0x81),
                        "xor" => self.bytes.push(0x83),
                        "==" | "!=" | "<" | ">=" | ">" | "<=" => {
                            self.bytes.push(0x94);
                            self.comparison(match op.as_str() {
                                "==" => 0x99,
                                "!=" => 0x9a,
                                "<" => 0x9b,
                                ">=" => 0x9c,
                                ">" => 0x9d,
                                _ => 0x9e,
                            });
                        }
                        _ => return Err(failure("Unsupported Long operation")),
                    }
                    return Ok(());
                }
                if a.ty.is_reference() || b.ty.is_reference() {
                    if !matches!(op.as_str(), "==" | "!=") {
                        return Err(failure("Unsupported reference operation"));
                    }
                    self.method_reference(
                        0xb8,
                        "kotlin/jvm/internal/Intrinsics",
                        "areEqual",
                        "(Ljava/lang/Object;Ljava/lang/Object;)Z",
                    )?;
                    if op == "!=" {
                        self.bytes.extend([0x04, 0x82]);
                    }
                    return Ok(());
                }
                match op.as_str() {
                    "+" => self.bytes.push(0x60),
                    "-" => self.bytes.push(0x64),
                    "*" => self.bytes.push(0x68),
                    "/" => self.bytes.push(0x6c),
                    "%" => self.bytes.push(0x70),
                    "shl" => self.bytes.push(0x78),
                    "shr" => self.bytes.push(0x7a),
                    "ushr" => self.bytes.push(0x7c),
                    "and" => self.bytes.push(0x7e),
                    "or" => self.bytes.push(0x80),
                    "xor" => self.bytes.push(0x82),
                    "==" => self.comparison(0x9f),
                    "!=" => self.comparison(0xa0),
                    "<" => self.comparison(0xa1),
                    ">=" => self.comparison(0xa2),
                    ">" => self.comparison(0xa3),
                    "<=" => self.comparison(0xa4),
                    _ => return Err(failure("Unsupported binary operation")),
                }
            }
        }
        Ok(())
    }
    fn block(&mut self, body: &[Statement]) -> Result<()> {
        for statement in body {
            match statement {
                Statement::JavaFieldStore {
                    owner,
                    name,
                    ty,
                    receiver,
                    value,
                } => {
                    let prefix = usize::from(receiver.is_some());
                    self.max_stack = self.max_stack.max(
                        receiver
                            .as_ref()
                            .map_or(0, Value::stack)
                            .max(prefix + value.stack()),
                    );
                    if let Some(receiver) = receiver {
                        self.value(receiver)?;
                    }
                    self.value(value)?;
                    self.field_reference(
                        if receiver.is_some() { 0xb5 } else { 0xb3 },
                        owner,
                        name,
                        &ty.descriptor(),
                    )?;
                }
                Statement::ListSet(receiver, index, value) => {
                    self.max_stack = self.max_stack.max(
                        receiver
                            .stack()
                            .max(1 + index.stack())
                            .max(2 + value.stack()),
                    );
                    self.value(receiver)?;
                    self.value(index)?;
                    self.value(value)?;
                    if !value.ty.is_reference() {
                        self.box_value(&value.ty)?;
                    }
                    let method = self.pool.interface_method(
                        "java/util/List",
                        "set",
                        "(ILjava/lang/Object;)Ljava/lang/Object;",
                    )?;
                    self.reference(0xb9, method);
                    self.bytes.extend([3, 0, 0x57]);
                }
                Statement::FieldStore {
                    name,
                    receiver,
                    value,
                } => {
                    self.max_stack = self.max_stack.max(receiver.stack().max(1 + value.stack()));
                    self.value(receiver)?;
                    self.value(value)?;
                    let owner = match receiver.ty.base() {
                        Type::Java(owner) => owner.as_str(),
                        _ => self.owner,
                    };
                    self.field_reference(0xb5, owner, name, &value.ty.descriptor())?;
                }
                Statement::Store(slot, value) => {
                    self.value(value)?;
                    self.bytes
                        .extend([store_opcode(&value.ty), self.local_slot(*slot)?]);
                }
                Statement::Return(value) => {
                    if self.inline_return(value)? {
                        continue;
                    }
                    self.value(value)?;
                    if !terminal_value(value) {
                        self.bytes.push(return_opcode(&value.ty));
                    }
                }
                Statement::NonLocalReturn(value) => {
                    self.value(value)?;
                    if !terminal_value(value) {
                        self.bytes.push(return_opcode(&value.ty));
                    }
                }
                Statement::Throw(message) => {
                    self.max_stack = self.max_stack.max(2 + message.stack());
                    let class = self.pool.class("java/lang/IllegalArgumentException")?;
                    self.reference(0xbb, class);
                    self.bytes.push(0x59);
                    self.value(message)?;
                    let constructor = self.pool.method(
                        "java/lang/IllegalArgumentException",
                        "<init>",
                        "(Ljava/lang/String;)V",
                    )?;
                    self.reference(0xb7, constructor);
                    self.bytes.push(0xbf);
                }
                Statement::ThrowException { owner, message } => {
                    self.max_stack = self
                        .max_stack
                        .max(2 + message.as_ref().map_or(0, Value::stack));
                    self.class_reference(0xbb, owner)?;
                    self.bytes.push(0x59);
                    if let Some(message) = message {
                        self.value(message)?;
                    }
                    self.method_reference(
                        0xb7,
                        owner,
                        "<init>",
                        if message.is_some() {
                            "(Ljava/lang/String;)V"
                        } else {
                            "()V"
                        },
                    )?;
                    self.bytes.push(0xbf);
                }
                Statement::IndexStore(array, index, value) => {
                    self.max_stack = self
                        .max_stack
                        .max(array.stack().max(1 + index.stack()).max(2 + value.stack()));
                    self.value(array)?;
                    self.value(index)?;
                    self.value(value)?;
                    self.bytes.push(match array.ty.base() {
                        Type::IntArray => 0x4f,
                        Type::ByteArray => 0x54,
                        Type::CharArray => 0x55,
                        Type::StringArray => 0x53,
                        _ => return Err(failure("Unsupported array store operand")),
                    });
                }
                Statement::Break => {
                    let target = self
                        .loops
                        .last()
                        .ok_or_else(|| failure("Break requires a loop"))?
                        .1;
                    self.branch(0xa7, target);
                }
                Statement::Continue => {
                    let target = self
                        .loops
                        .last()
                        .ok_or_else(|| failure("Continue requires a loop"))?
                        .0;
                    self.branch(0xa7, target);
                }
                Statement::ForUntil {
                    slot,
                    start,
                    end_slot,
                    end,
                    body,
                } => {
                    let slot = self.local_slot(*slot)?;
                    let end_slot = self.local_slot(*end_slot)?;
                    self.value(start)?;
                    self.bytes.extend([0x36, slot]);
                    self.value(end)?;
                    self.bytes.extend([0x36, end_slot]);
                    let head = self.label();
                    let step = self.label();
                    let done = self.label();
                    self.mark(head);
                    self.bytes.extend([0x15, slot, 0x15, end_slot]);
                    self.max_stack = self.max_stack.max(2);
                    self.branch(0xa2, done);
                    self.loops.push((step, done));
                    self.block(body)?;
                    self.loops.pop();
                    self.mark(step);
                    self.bytes.extend([0x84, slot, 1]);
                    self.branch(0xa7, head);
                    self.mark(done);
                }
                Statement::Expression(value) => {
                    self.value(value)?;
                    if value.ty.base() != &Type::Unit && !terminal_value(value) {
                        self.bytes
                            .push(if value.ty.slots() == 2 { 0x58 } else { 0x57 });
                    }
                }
                Statement::If(cond, yes, no) => {
                    let otherwise = self.label();
                    let end = self.label();
                    self.value(cond)?;
                    self.branch(0x99, otherwise);
                    self.block(yes)?;
                    if !terminal_block(yes) {
                        self.branch(0xa7, end);
                    }
                    self.mark(otherwise);
                    self.block(no)?;
                    self.mark(end);
                }
                Statement::While(cond, body) => {
                    let start = self.label();
                    let end = self.label();
                    self.mark(start);
                    self.value(cond)?;
                    self.branch(0x99, end);
                    self.loops.push((start, end));
                    self.block(body)?;
                    self.loops.pop();
                    self.branch(0xa7, start);
                    self.mark(end);
                }
            }
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Vec<u8>> {
        self.bytes.extend([0x01, 0xbf]);
        if self.bytes.len() > 65535 {
            return Err(failure("JVM method code size limit exceeded"));
        }
        for (position, label) in self.branches {
            let target = self.labels[label].ok_or_else(|| failure("Unresolved bytecode label"))?;
            let offset = i16::try_from(target as isize - position as isize)
                .map_err(|_| failure("Branch exceeds the currently supported 16-bit range"))?;
            self.bytes[position + 1..position + 3].copy_from_slice(&offset.to_be_bytes());
        }
        Ok(self.bytes)
    }
}
pub fn emit(
    module: &Module,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
) -> Result<Vec<u8>> {
    let mut pool = Pool::default();
    let class = pool.class(owner)?;
    let parent = pool.class("java/lang/Object")?;
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let mut methods = Vec::new();
    for function in &module.functions {
        let mut code = Code {
            pool: &mut pool,
            functions: &module.functions,
            owner,
            bytes: Vec::new(),
            labels: Vec::new(),
            branches: Vec::new(),
            loops: Vec::new(),
            lambdas: &module.lambdas,
            inline_local_maps: Vec::new(),
            inline_returns: Vec::new(),
            max_stack: 2,
        };
        if !function.private {
            for (slot, (name, ty)) in parameter_slots(&function.params, 0)
                .into_iter()
                .zip(&function.params)
            {
                if nonnull(ty) {
                    code.bytes.extend([0x19, slot as u8]);
                    let name = code.pool.string(if name.starts_with("$this$") {
                        "<this>"
                    } else {
                        name
                    })?;
                    code.reference(0x13, name);
                    let check = code.pool.method(
                        "kotlin/jvm/internal/Intrinsics",
                        "checkNotNullParameter",
                        "(Ljava/lang/Object;Ljava/lang/String;)V",
                    )?;
                    code.reference(0xb8, check);
                }
            }
        }
        code.block(&function.body)?;
        let max_stack = u16::try_from(code.max_stack)
            .map_err(|_| failure("JVM operand stack limit exceeded"))?;
        let bytes = code.finish()?;
        let mut method = simple_method(
            &mut pool,
            if function.private { 0x001a } else { 0x0019 },
            &function.name,
            &function.descriptor(),
            SimpleCode {
                bytes,
                max_stack,
                locals: function.locals,
            },
            generic_function(function).as_deref(),
            nonnull(&function.result),
        )?;
        parameter_annotations(&mut pool, &mut method, &function.params)?;
        nullable_result(&mut pool, &mut method, &function.result)?;
        methods.push(method);
        if function.private && !module.lambdas.is_empty() {
            let slots = function
                .params
                .iter()
                .map(|(_, ty)| ty.slots())
                .sum::<usize>();
            let mut code = SimpleCode::new(slots.max(function.result.slots()) as u16, slots as u16);
            for (slot, (_, ty)) in parameter_slots(&function.params, 0)
                .into_iter()
                .zip(&function.params)
            {
                code.bytes.extend([load_opcode(ty), slot as u8]);
            }
            code.reference(
                0xb8,
                pool.method(owner, &function.name, &function.descriptor())?,
            );
            code.bytes.push(return_opcode(&function.result));
            methods.push(simple_method(
                &mut pool,
                0x1009,
                &format!("access${}", function.name),
                &function.descriptor(),
                code,
                None,
                false,
            )?);
        }
        if let Some(default) = defaults::function(
            &mut pool,
            &module.functions,
            function,
            owner,
            true,
            &module.lambdas,
        )? {
            methods.push(default);
        }
    }
    let metadata = metadata_attribute(&mut pool, metadata)?;
    let mut fields = Vec::new();
    for constant in &module.constants {
        fields.extend(constant_field(&mut pool, constant)?);
    }
    let mut out = Vec::new();
    u4(&mut out, 0xcafebabe);
    u2(&mut out, 0);
    u2(&mut out, 49);
    pool.write(&mut out);
    u2(&mut out, 0x0031);
    u2(&mut out, class);
    u2(&mut out, parent);
    u2(&mut out, 0);
    u2(&mut out, module.constants.len() as u16);
    out.extend(fields);
    u2(&mut out, methods.len() as u16);
    for method in methods {
        out.extend(method);
    }
    u2(&mut out, 2);
    u2(&mut out, source_name);
    u4(&mut out, 2);
    u2(&mut out, source_file);
    out.extend(metadata);
    Ok(out)
}

struct SimpleCode {
    bytes: Vec<u8>,
    max_stack: u16,
    locals: u16,
}

impl SimpleCode {
    fn new(max_stack: u16, locals: u16) -> Self {
        Self {
            bytes: Vec::new(),
            max_stack,
            locals,
        }
    }

    fn reference(&mut self, opcode: u8, index: u16) {
        self.bytes.push(opcode);
        u2(&mut self.bytes, index);
    }

    fn integer(&mut self, value: usize) -> Result<()> {
        if value <= 5 {
            self.bytes.push(value as u8 + 3);
        } else if let Ok(value) = i8::try_from(value) {
            self.bytes.extend([0x10, value as u8]);
        } else {
            let value = i16::try_from(value)
                .map_err(|_| failure("Enum ordinal exceeds the supported range"))?;
            self.bytes.push(0x11);
            self.bytes.extend(value.to_be_bytes());
        }
        Ok(())
    }
}

fn simple_method(
    pool: &mut Pool,
    flags: u16,
    name: &str,
    descriptor: &str,
    code: SimpleCode,
    signature: Option<&str>,
    not_null: bool,
) -> Result<Vec<u8>> {
    if code.bytes.len() > 65535 {
        return Err(failure("JVM method code size limit exceeded"));
    }
    let mut method = Vec::new();
    u2(&mut method, flags);
    u2(&mut method, pool.utf8(name)?);
    u2(&mut method, pool.utf8(descriptor)?);
    u2(
        &mut method,
        1 + u16::from(signature.is_some()) + u16::from(not_null),
    );
    u2(&mut method, pool.utf8("Code")?);
    u4(&mut method, (12 + code.bytes.len()) as u32);
    u2(&mut method, code.max_stack);
    u2(&mut method, code.locals);
    u4(&mut method, code.bytes.len() as u32);
    method.extend(code.bytes);
    u2(&mut method, 0);
    u2(&mut method, 0);
    if let Some(signature) = signature {
        u2(&mut method, pool.utf8("Signature")?);
        u4(&mut method, 2);
        u2(&mut method, pool.utf8(signature)?);
    }
    if not_null {
        u2(&mut method, pool.utf8("RuntimeInvisibleAnnotations")?);
        u4(&mut method, 6);
        u2(&mut method, 1);
        u2(
            &mut method,
            pool.utf8("Lorg/jetbrains/annotations/NotNull;")?,
        );
        u2(&mut method, 0);
    }
    Ok(method)
}

pub fn emit_enum(decl: &EnumDecl, owner: &str, file: &str) -> Result<Vec<u8>> {
    if decl.entries.len() > 1024 {
        return Err(Diagnostic::new(
            decl.offset,
            "Enum exceeds the current 1024-entry limit",
        ));
    }
    let mut pool = Pool::default();
    let class = pool.class(owner)?;
    let parent = pool.class("java/lang/Enum")?;
    let descriptor = format!("L{owner};");
    let array_descriptor = format!("[{descriptor}");
    let values_descriptor = format!("(){array_descriptor}");
    let entries_descriptor = "Lkotlin/enums/EnumEntries;";
    let constructor_descriptor = "(Ljava/lang/String;I)V";
    let array_class = pool.class(&array_descriptor)?;
    let enum_array_class = pool.class("[Ljava/lang/Enum;")?;
    let constructor = pool.method(owner, "<init>", constructor_descriptor)?;
    let parent_constructor = pool.method("java/lang/Enum", "<init>", constructor_descriptor)?;
    let values_field = pool.field(owner, "$VALUES", &array_descriptor)?;
    let entries_field = pool.field(owner, "$ENTRIES", entries_descriptor)?;
    let values_method = pool.method(owner, "$values", &values_descriptor)?;
    let clone = pool.method("java/lang/Object", "clone", "()Ljava/lang/Object;")?;
    let value_of = pool.method(
        "java/lang/Enum",
        "valueOf",
        "(Ljava/lang/Class;Ljava/lang/String;)Ljava/lang/Enum;",
    )?;
    let enum_entries = pool.method(
        "kotlin/enums/EnumEntriesKt",
        "enumEntries",
        "([Ljava/lang/Enum;)Lkotlin/enums/EnumEntries;",
    )?;
    let mut fields = Vec::new();
    let mut entry_fields = Vec::new();
    for (entry, _) in &decl.entries {
        entry_fields.push(pool.field(owner, entry, &descriptor)?);
        u2(&mut fields, 0x4019);
        u2(&mut fields, pool.utf8(entry)?);
        u2(&mut fields, pool.utf8(&descriptor)?);
        u2(&mut fields, 0);
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
    let mut methods = Vec::new();
    let mut code = SimpleCode::new(3, 3);
    code.bytes.extend([0x2a, 0x2b, 0x1c]);
    code.reference(0xb7, parent_constructor);
    code.bytes.push(0xb1);
    methods.push(simple_method(
        &mut pool,
        0x0002,
        "<init>",
        constructor_descriptor,
        code,
        Some("()V"),
        false,
    )?);
    let mut code = SimpleCode::new(1, 0);
    code.reference(0xb2, values_field);
    code.reference(0xb6, clone);
    code.reference(0xc0, array_class);
    code.bytes.push(0xb0);
    methods.push(simple_method(
        &mut pool,
        0x0009,
        "values",
        &values_descriptor,
        code,
        None,
        false,
    )?);
    let mut code = SimpleCode::new(2, 1);
    code.reference(0x13, class);
    code.bytes.push(0x2a);
    code.reference(0xb8, value_of);
    code.reference(0xc0, class);
    code.bytes.push(0xb0);
    methods.push(simple_method(
        &mut pool,
        0x0009,
        "valueOf",
        &format!("(Ljava/lang/String;){descriptor}"),
        code,
        None,
        false,
    )?);
    let mut code = SimpleCode::new(1, 0);
    code.reference(0xb2, entries_field);
    code.bytes.push(0xb0);
    methods.push(simple_method(
        &mut pool,
        0x0009,
        "getEntries",
        &format!("(){entries_descriptor}"),
        code,
        Some(&format!("()Lkotlin/enums/EnumEntries<{descriptor}>;")),
        true,
    )?);
    let mut code = if decl.entries.is_empty() {
        SimpleCode::new(1, 0)
    } else {
        SimpleCode::new(3, 1)
    };
    code.integer(decl.entries.len())?;
    code.reference(0xbd, class);
    if !decl.entries.is_empty() {
        code.bytes.push(0x4b);
        for (ordinal, field) in entry_fields.iter().enumerate() {
            code.bytes.push(0x2a);
            code.integer(ordinal)?;
            code.reference(0xb2, *field);
            code.bytes.push(0x53);
        }
        code.bytes.push(0x2a);
    }
    code.bytes.push(0xb0);
    methods.push(simple_method(
        &mut pool,
        0x101a,
        "$values",
        &values_descriptor,
        code,
        None,
        false,
    )?);
    let mut code = SimpleCode::new(if decl.entries.is_empty() { 1 } else { 4 }, 0);
    for (ordinal, ((entry, _), field)) in decl.entries.iter().zip(&entry_fields).enumerate() {
        code.reference(0xbb, class);
        code.bytes.push(0x59);
        code.reference(0x13, pool.string(entry)?);
        code.integer(ordinal)?;
        code.reference(0xb7, constructor);
        code.reference(0xb3, *field);
    }
    code.reference(0xb8, values_method);
    code.reference(0xb3, values_field);
    code.reference(0xb2, values_field);
    code.reference(0xc0, enum_array_class);
    code.reference(0xb8, enum_entries);
    code.reference(0xb3, entries_field);
    code.bytes.push(0xb1);
    methods.push(simple_method(
        &mut pool, 0x0008, "<clinit>", "()V", code, None, false,
    )?);
    let signature_name = pool.utf8("Signature")?;
    let signature = pool.utf8(&format!("Ljava/lang/Enum<{descriptor}>;"))?;
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let mut out = Vec::new();
    u4(&mut out, 0xcafebabe);
    u2(&mut out, 0);
    u2(&mut out, 49);
    pool.write(&mut out);
    u2(&mut out, 0x4031);
    u2(&mut out, class);
    u2(&mut out, parent);
    u2(&mut out, 0);
    u2(&mut out, (decl.entries.len() + 2) as u16);
    out.extend(fields);
    u2(&mut out, methods.len() as u16);
    for method in methods {
        out.extend(method);
    }
    u2(&mut out, 2);
    u2(&mut out, signature_name);
    u4(&mut out, 2);
    u2(&mut out, signature);
    u2(&mut out, source_name);
    u4(&mut out, 2);
    u2(&mut out, source_file);
    Ok(out)
}

struct ObjectClass<'a> {
    decl: &'a ObjectDecl,
    lineage: Vec<(String, String)>,
}

fn collect_objects<'a>(
    decl: &'a ObjectDecl,
    owner: String,
    lineage: &mut Vec<(String, String)>,
    classes: &mut Vec<ObjectClass<'a>>,
) -> Result<()> {
    if lineage.len() >= 16 {
        return Err(Diagnostic::new(
            decl.offset,
            "Object nesting exceeds the current depth limit of 16",
        ));
    }
    if classes.len() >= 256 {
        return Err(Diagnostic::new(
            decl.offset,
            "Object tree exceeds the current 256-class limit",
        ));
    }
    if decl.constants.len() > 1024 {
        return Err(Diagnostic::new(
            decl.offset,
            "Object exceeds the current 1024-constant limit",
        ));
    }
    if decl
        .constants
        .iter()
        .any(|constant| constant.name == "INSTANCE")
        || decl.objects.iter().any(|object| object.name == "INSTANCE")
    {
        return Err(Diagnostic::new(
            decl.offset,
            "Object members named INSTANCE are not supported",
        ));
    }
    lineage.push((owner.clone(), decl.name.clone()));
    classes.push(ObjectClass {
        decl,
        lineage: lineage.clone(),
    });
    for child in &decl.objects {
        collect_objects(child, format!("{owner}${}", child.name), lineage, classes)?;
    }
    lineage.pop();
    Ok(())
}

fn object_field(
    pool: &mut Pool,
    name: &str,
    descriptor: &str,
    constant: Option<u16>,
    not_null: bool,
) -> Result<Vec<u8>> {
    let mut field = Vec::new();
    u2(&mut field, 0x0019);
    u2(&mut field, pool.utf8(name)?);
    u2(&mut field, pool.utf8(descriptor)?);
    u2(
        &mut field,
        u16::from(constant.is_some()) + u16::from(not_null),
    );
    if let Some(constant) = constant {
        u2(&mut field, pool.utf8("ConstantValue")?);
        u4(&mut field, 2);
        u2(&mut field, constant);
    }
    if not_null {
        u2(&mut field, pool.utf8("RuntimeInvisibleAnnotations")?);
        u4(&mut field, 6);
        u2(&mut field, 1);
        u2(
            &mut field,
            pool.utf8("Lorg/jetbrains/annotations/NotNull;")?,
        );
        u2(&mut field, 0);
    }
    Ok(field)
}

fn constant_field(pool: &mut Pool, constant: &crate::syntax::ConstDecl) -> Result<Vec<u8>> {
    let (descriptor, value, not_null) = match &constant.value {
        ConstValue::Int(value) => ("I", pool.add(Constant::Integer(*value))?, false),
        ConstValue::Long(value) => ("J", pool.add(Constant::Long(*value))?, false),
        ConstValue::Float(value) => ("F", pool.add(Constant::Float(*value))?, false),
        ConstValue::Char(value) => ("C", pool.add(Constant::Integer(i32::from(*value)))?, false),
        ConstValue::Boolean(value) => ("Z", pool.add(Constant::Integer(i32::from(*value)))?, false),
        ConstValue::String(value) => ("Ljava/lang/String;", pool.string_units(value)?, true),
    };
    let mut field = object_field(pool, &constant.name, descriptor, Some(value), not_null)?;
    if constant.private {
        field[0..2].copy_from_slice(&0x001au16.to_be_bytes());
    }
    Ok(field)
}

fn emit_object(class: &ObjectClass<'_>, file: &str) -> Result<crate::ClassFile> {
    let owner = &class.lineage.last().unwrap().0;
    let mut pool = Pool::default();
    let this = pool.class(owner)?;
    let parent = pool.class("java/lang/Object")?;
    let descriptor = format!("L{owner};");
    let instance = pool.field(owner, "INSTANCE", &descriptor)?;
    let parent_constructor = pool.method("java/lang/Object", "<init>", "()V")?;
    let mut fields = object_field(&mut pool, "INSTANCE", &descriptor, None, true)?;
    for constant in &class.decl.constants {
        fields.extend(constant_field(&mut pool, constant)?);
    }
    let mut constructor_code = SimpleCode::new(1, 1);
    constructor_code.bytes.push(0x2a);
    constructor_code.reference(0xb7, parent_constructor);
    constructor_code.bytes.push(0xb1);
    let constructor = simple_method(
        &mut pool,
        0x0002,
        "<init>",
        "()V",
        constructor_code,
        None,
        false,
    )?;
    let mut initializer_code = SimpleCode::new(2, 0);
    initializer_code.reference(0xbb, this);
    initializer_code.bytes.push(0x59);
    initializer_code.reference(0xb7, pool.method(owner, "<init>", "()V")?);
    initializer_code.reference(0xb3, instance);
    initializer_code.bytes.push(0xb1);
    let initializer = simple_method(
        &mut pool,
        0x0008,
        "<clinit>",
        "()V",
        initializer_code,
        None,
        false,
    )?;
    let mut inner_classes = Vec::new();
    for pair in class.lineage.windows(2) {
        inner_classes.push((pair[1].0.clone(), pair[0].0.clone(), pair[1].1.clone()));
    }
    for child in &class.decl.objects {
        inner_classes.push((
            format!("{owner}${}", child.name),
            owner.clone(),
            child.name.clone(),
        ));
    }
    inner_classes.sort_by(|a, b| a.0.cmp(&b.0));
    let mut attributes = Vec::new();
    if !inner_classes.is_empty() {
        u2(&mut attributes, pool.utf8("InnerClasses")?);
        u4(&mut attributes, (2 + 8 * inner_classes.len()) as u32);
        u2(&mut attributes, inner_classes.len() as u16);
        for (inner, outer, name) in &inner_classes {
            u2(&mut attributes, pool.class(inner)?);
            u2(&mut attributes, pool.class(outer)?);
            u2(&mut attributes, pool.utf8(name)?);
            u2(&mut attributes, 0x0019);
        }
    }
    u2(&mut attributes, pool.utf8("SourceFile")?);
    u4(&mut attributes, 2);
    u2(&mut attributes, pool.utf8(file)?);
    let mut bytes = Vec::new();
    u4(&mut bytes, 0xcafebabe);
    u2(&mut bytes, 0);
    u2(&mut bytes, 49);
    pool.write(&mut bytes);
    u2(&mut bytes, 0x0031);
    u2(&mut bytes, this);
    u2(&mut bytes, parent);
    u2(&mut bytes, 0);
    u2(&mut bytes, (class.decl.constants.len() + 1) as u16);
    bytes.extend(fields);
    u2(&mut bytes, 2);
    bytes.extend(constructor);
    bytes.extend(initializer);
    u2(&mut bytes, 1 + u16::from(!inner_classes.is_empty()));
    bytes.extend(attributes);
    Ok(crate::ClassFile {
        internal_name: owner.clone(),
        bytes,
    })
}

pub fn emit_objects(decl: &ObjectDecl, owner: &str, file: &str) -> Result<Vec<crate::ClassFile>> {
    let mut classes = Vec::new();
    collect_objects(decl, owner.into(), &mut Vec::new(), &mut classes)?;
    classes
        .iter()
        .map(|class| emit_object(class, file))
        .collect()
}

fn metadata_attribute(pool: &mut Pool, metadata: &crate::metadata::Metadata) -> Result<Vec<u8>> {
    metadata_attribute_with_annotations(pool, metadata, &[])
}

fn metadata_attribute_with_annotations(
    pool: &mut Pool,
    metadata: &crate::metadata::Metadata,
    annotations: &[String],
) -> Result<Vec<u8>> {
    fn integer(pool: &mut Pool, out: &mut Vec<u8>, value: i32) -> Result<()> {
        out.push(b'I');
        u2(out, pool.add(Constant::Integer(value))?);
        Ok(())
    }
    let mut annotation = Vec::new();
    u2(&mut annotation, (annotations.len() + 1) as u16);
    for name in annotations {
        u2(&mut annotation, pool.utf8(name)?);
        u2(&mut annotation, 0);
    }
    u2(&mut annotation, pool.utf8("Lkotlin/Metadata;")?);
    u2(&mut annotation, if metadata.kind == 3 { 3 } else { 5 });
    u2(&mut annotation, pool.utf8("mv")?);
    annotation.push(b'[');
    u2(&mut annotation, 3);
    for value in [2, 3, 0] {
        integer(pool, &mut annotation, value)?;
    }
    u2(&mut annotation, pool.utf8("k")?);
    integer(pool, &mut annotation, metadata.kind)?;
    u2(&mut annotation, pool.utf8("xi")?);
    integer(pool, &mut annotation, 48)?;
    if metadata.kind != 3 {
        u2(&mut annotation, pool.utf8("d1")?);
        annotation.push(b'[');
        u2(
            &mut annotation,
            u16::try_from(metadata.data1.len())
                .map_err(|_| failure("Metadata array exceeds the JVM limit"))?,
        );
        for value in &metadata.data1 {
            annotation.push(b's');
            u2(&mut annotation, pool.utf16(value)?);
        }
        u2(&mut annotation, pool.utf8("d2")?);
        annotation.push(b'[');
        u2(
            &mut annotation,
            u16::try_from(metadata.data2.len())
                .map_err(|_| failure("Metadata array exceeds the JVM limit"))?,
        );
        for value in &metadata.data2 {
            annotation.push(b's');
            u2(&mut annotation, pool.utf8(value)?);
        }
    }
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("RuntimeVisibleAnnotations")?);
    u4(&mut attribute, annotation.len() as u32);
    attribute.extend(annotation);
    Ok(attribute)
}

fn parameter_annotations(
    pool: &mut Pool,
    method: &mut Vec<u8>,
    params: &[(String, Type)],
) -> Result<()> {
    if !params
        .iter()
        .any(|(_, ty)| nullability_annotation(ty).is_some())
    {
        return Ok(());
    }
    let mut annotation = Vec::new();
    annotation
        .push(u8::try_from(params.len()).map_err(|_| failure("Too many annotated parameters"))?);
    for (_, ty) in params {
        let nullable = nullability_annotation(ty);
        u2(&mut annotation, u16::from(nullable.is_some()));
        if let Some(nullable) = nullable {
            u2(&mut annotation, pool.utf8(nullable)?);
            u2(&mut annotation, 0);
        }
    }
    let count = u16::from_be_bytes([method[6], method[7]]) + 1;
    method[6..8].copy_from_slice(&count.to_be_bytes());
    u2(method, pool.utf8("RuntimeInvisibleParameterAnnotations")?);
    u4(method, annotation.len() as u32);
    method.extend(annotation);
    Ok(())
}

pub fn emit_class_files(
    class: &crate::check::ClassDecl,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
    companion_metadata: Option<&crate::metadata::Metadata>,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<Vec<crate::ClassFile>> {
    let mut files = vec![crate::ClassFile {
        internal_name: owner.into(),
        bytes: emit_class_with(class, owner, file, metadata, 0x0001, lambdas, |_| {
            Ok(Vec::new())
        })?,
    }];
    if let Some(companion) = &class.companion {
        files.push(class_companion::emit(
            companion,
            owner,
            file,
            companion_metadata.ok_or_else(|| failure("Missing class companion metadata"))?,
        )?);
    }
    Ok(files)
}

fn class_field(pool: &mut Pool, name: &str, ty: &Type, flags: u16) -> Result<Vec<u8>> {
    let mut field = Vec::new();
    u2(&mut field, flags);
    u2(&mut field, pool.utf8(name)?);
    u2(&mut field, pool.utf8(&ty.descriptor())?);
    let generic = generic_type(ty);
    let has_signature = generic != ty.descriptor();
    let annotation = nullability_annotation(ty);
    u2(
        &mut field,
        u16::from(annotation.is_some()) + u16::from(has_signature),
    );
    if has_signature {
        u2(&mut field, pool.utf8("Signature")?);
        u4(&mut field, 2);
        u2(&mut field, pool.utf8(&generic)?);
    }
    if let Some(annotation) = annotation {
        u2(&mut field, pool.utf8("RuntimeInvisibleAnnotations")?);
        u4(&mut field, 6);
        u2(&mut field, 1);
        u2(&mut field, pool.utf8(annotation)?);
        u2(&mut field, 0);
    }
    Ok(field)
}

fn emit_class_with(
    class: &crate::check::ClassDecl,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
    constructor_flags: u16,
    lambdas: &[crate::check::LambdaDecl],
    additional_attributes: impl FnOnce(&mut Pool) -> Result<Vec<Vec<u8>>>,
) -> Result<Vec<u8>> {
    let mut pool = Pool::default();
    let this = pool.class(owner)?;
    let parent = pool.class("java/lang/Object")?;
    let mut interfaces = Vec::new();
    for interface in &class.interfaces {
        interfaces.push(pool.class(interface)?);
    }
    let mut fields = Vec::new();
    for field in &class.fields {
        fields.extend(class_field(
            &mut pool,
            &field.name,
            &field.ty,
            if field.mutable { 0x0002 } else { 0x0012 },
        )?);
    }
    let constructor_descriptor = format!(
        "({})V",
        class
            .params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let constructor_slots = 1 + class.params.iter().map(|(_, ty)| ty.slots()).sum::<usize>();
    let constructor_locals = class.constructor_locals.max(constructor_slots as u16);
    let mut code = Code {
        pool: &mut pool,
        functions: &class.methods,
        owner,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        lambdas,
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        max_stack: 3,
    };
    for (slot, (name, ty)) in parameter_slots(&class.params, 1)
        .into_iter()
        .zip(&class.params)
    {
        if nonnull(ty) {
            code.bytes.extend([0x19, slot as u8]);
            let name = code.pool.string(name)?;
            code.reference(0x13, name);
            code.method_reference(
                0xb8,
                "kotlin/jvm/internal/Intrinsics",
                "checkNotNullParameter",
                "(Ljava/lang/Object;Ljava/lang/String;)V",
            )?;
        }
    }
    code.bytes.push(0x2a);
    code.method_reference(0xb7, "java/lang/Object", "<init>", "()V")?;
    let slots = parameter_slots(&class.params, 1);
    for field in &class.fields {
        if field.param.is_none() && field.initializer.is_none() {
            continue;
        }
        code.bytes.push(0x2a);
        if let Some(index) = field.param {
            code.bytes
                .extend([load_opcode(&field.ty), slots[index] as u8]);
            code.max_stack = code.max_stack.max(1 + field.ty.slots());
        } else if let Some(value) = &field.initializer {
            code.max_stack = code.max_stack.max(1 + value.stack());
            code.value(value)?;
        }
        code.field_reference(0xb5, owner, &field.name, &field.ty.descriptor())?;
    }
    code.bytes.push(0xb1);
    let max_stack = code.max_stack as u16;
    let code = SimpleCode {
        bytes: code.finish()?,
        locals: constructor_locals,
        max_stack,
    };
    let mut constructor = simple_method(
        &mut pool,
        constructor_flags,
        "<init>",
        &constructor_descriptor,
        code,
        generic_method(&class.params, &Type::Unit).as_deref(),
        false,
    )?;
    parameter_annotations(&mut pool, &mut constructor, &class.params)?;
    frames::attach_constructor(
        &mut pool,
        &mut constructor,
        owner,
        &class.params,
        constructor_locals,
    )?;
    let mut methods = vec![constructor];
    methods.extend(defaults::constructors(&mut pool, class, owner, lambdas)?);
    if class.data {
        methods.extend(data::methods(&mut pool, class, owner)?);
    }
    for function in &class.methods {
        if function.body.iter().enumerate().any(|(index, statement)| {
            matches!(statement, Statement::Return(_) | Statement::Throw(_))
                && index + 1 != function.body.len()
        }) {
            return Err(failure(
                "Unreachable code in class members is not supported",
            ));
        }
        let mut code = Code {
            pool: &mut pool,
            functions: &class.methods,
            owner,
            bytes: Vec::new(),
            labels: Vec::new(),
            branches: Vec::new(),
            loops: Vec::new(),
            lambdas,
            inline_local_maps: Vec::new(),
            inline_returns: Vec::new(),
            max_stack: 2,
        };
        if !function.private {
            for (slot, (name, ty)) in parameter_slots(&function.params, 1)
                .into_iter()
                .zip(&function.params)
            {
                if nonnull(ty) {
                    code.bytes.extend([0x19, slot as u8]);
                    let name = code.pool.string(name)?;
                    code.reference(0x13, name);
                    let check = code.pool.method(
                        "kotlin/jvm/internal/Intrinsics",
                        "checkNotNullParameter",
                        "(Ljava/lang/Object;Ljava/lang/String;)V",
                    )?;
                    code.reference(0xb8, check);
                }
            }
        }
        code.block(&function.body)?;
        let max_stack = u16::try_from(code.max_stack)
            .map_err(|_| failure("JVM operand stack limit exceeded"))?;
        let code = SimpleCode {
            max_stack,
            locals: function.locals,
            bytes: code.finish()?,
        };
        let flags = if function.private {
            0x0012
        } else if function.is_override {
            0x0001
        } else {
            0x0011
        };
        let mut method = simple_method(
            &mut pool,
            flags,
            &function.name,
            &function.descriptor(),
            code,
            generic_function(function).as_deref(),
            nonnull(&function.result),
        )?;
        parameter_annotations(&mut pool, &mut method, &function.params)?;
        nullable_result(&mut pool, &mut method, &function.result)?;
        frames::attach(
            &mut pool,
            &mut method,
            owner,
            &function.params,
            function.locals,
            false,
        )?;
        methods.push(method);
        if let Some(default) =
            defaults::function(&mut pool, &class.methods, function, owner, false, lambdas)?
        {
            methods.push(default);
        }
    }
    for bridge in &class.bridges {
        let (params, result) = crate::check::class_method_types(bridge)?;
        let count = 1 + params.iter().map(Type::slots).sum::<usize>();
        let mut code = SimpleCode::new(count.max(result.slots()) as u16, count as u16);
        code.bytes.push(0x2a);
        let mut slot = 1;
        for ty in &params {
            code.bytes.extend([load_opcode(ty), slot as u8]);
            slot += ty.slots();
        }
        code.reference(
            0xb7,
            pool.interface_method(&bridge.owner, &bridge.name, &bridge.descriptor)?,
        );
        code.bytes.push(return_opcode(&result));
        let mut method = simple_method(
            &mut pool,
            0x0041,
            &bridge.name,
            &bridge.descriptor,
            code,
            generic_method(
                &params
                    .iter()
                    .cloned()
                    .map(|ty| (String::new(), ty))
                    .collect::<Vec<_>>(),
                &result,
            )
            .as_deref(),
            nonnull(&result),
        )?;
        let params: Vec<_> = params.into_iter().map(|ty| (String::new(), ty)).collect();
        parameter_annotations(&mut pool, &mut method, &params)?;
        nullable_result(&mut pool, &mut method, &result)?;
        methods.push(method);
    }
    let mut additional_attributes = additional_attributes(&mut pool)?;
    if let Some(companion) = &class.companion {
        class_companion::outer(
            &mut pool,
            companion,
            owner,
            &mut fields,
            &mut methods,
            lambdas,
        )?;
        additional_attributes.push(class_companion::inner_attribute(
            &mut pool,
            owner,
            &companion.owner,
        )?);
    }
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let metadata = metadata_attribute(&mut pool, metadata)?;
    let mut out = Vec::new();
    u4(&mut out, 0xcafebabe);
    u2(&mut out, 0);
    u2(&mut out, 52);
    pool.write(&mut out);
    u2(&mut out, 0x0031);
    u2(&mut out, this);
    u2(&mut out, parent);
    u2(&mut out, interfaces.len() as u16);
    for interface in interfaces {
        u2(&mut out, interface);
    }
    u2(
        &mut out,
        (class.fields.len()
            + class
                .companion
                .as_ref()
                .map_or(0, |companion| companion.properties.len() + 1)) as u16,
    );
    out.extend(fields);
    u2(&mut out, methods.len() as u16);
    for method in methods {
        out.extend(method);
    }
    u2(&mut out, (2 + additional_attributes.len()) as u16);
    for attribute in additional_attributes {
        out.extend(attribute);
    }
    u2(&mut out, source_name);
    u4(&mut out, 2);
    u2(&mut out, source_file);
    out.extend(metadata);
    Ok(out)
}
