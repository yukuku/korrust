use crate::{
    Diagnostic, Result,
    check::{Module, Statement, StringMethod, Value, ValueKind},
    syntax::{ConstValue, EnumDecl, ObjectDecl, Type},
};
use std::collections::HashMap;

mod interfaces;
mod objects;
pub use interfaces::emit_interfaces;
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
    } else if ty == &Type::Float {
        0x17
    } else {
        0x15
    }
}
fn store_opcode(ty: &Type) -> u8 {
    if ty.is_reference() {
        0x3a
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
    } else if ty == &Type::Float {
        0xae
    } else {
        0xac
    }
}

fn reference_class(ty: &Type) -> Result<String> {
    match ty.base() {
        Type::String => Ok("java/lang/String".into()),
        Type::Java(owner) => Ok(owner.clone()),
        Type::ThreadLocal(_) => Ok("java/lang/ThreadLocal".into()),
        Type::IntArray | Type::ByteArray | Type::CharArray | Type::StringArray => {
            Ok(ty.descriptor())
        }
        _ => Err(failure("Expected a reference type")),
    }
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
        if self.entries.len() >= 65534 {
            return Err(failure("JVM constant pool limit exceeded"));
        }
        let index = (self.entries.len() + 1) as u16;
        self.entries.push(entry.clone());
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
}
impl Code<'_> {
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
                    let descriptor = match part.ty.base() {
                        Type::String => "(Ljava/lang/String;)Ljava/lang/StringBuilder;",
                        Type::Int => "(I)Ljava/lang/StringBuilder;",
                        Type::Boolean => "(Z)Ljava/lang/StringBuilder;",
                        Type::Char
                        | Type::Byte
                        | Type::Float
                        | Type::IntArray
                        | Type::ByteArray
                        | Type::ThreadLocal(_)
                        | Type::CharArray
                        | Type::StringArray
                        | Type::Java(_)
                        | Type::Unit
                        | Type::Platform(_) => {
                            return Err(failure("Unsupported string concatenation operand"));
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
                for arg in args {
                    self.value(arg)?;
                }
                let method = self
                    .pool
                    .method(&method.owner, &method.name, &method.descriptor)?;
                self.reference(0xb6, method);
            }
            ValueKind::Let { slot, value, body } => {
                self.value(value)?;
                self.bytes.extend([store_opcode(&value.ty), *slot]);
                self.value(body)?;
            }
            ValueKind::NewArray(array_type, size) => {
                self.value(size)?;
                self.bytes.extend([0xbc, primitive_array_type(array_type)?]);
            }
            ValueKind::Convert(target, value) => {
                self.value(value)?;
                match target {
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
                for arg in args {
                    self.value(arg)?;
                }
                self.method_reference(0xb8, owner, name, descriptor)?;
            }
            ValueKind::Increment { slot, prefix } => {
                if *prefix {
                    self.bytes.extend([0x84, *slot, 1]);
                }
                self.bytes.extend([0x15, *slot]);
                if !*prefix {
                    self.bytes.extend([0x84, *slot, 1]);
                }
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
                self.branch(0xa7, end);
                self.mark(otherwise);
                self.value(no)?;
                self.mark(end);
            }
            ValueKind::IntToString(value) => {
                self.value(value)?;
                let method =
                    self.pool
                        .method("java/lang/String", "valueOf", "(I)Ljava/lang/String;")?;
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
                for arg in args {
                    self.value(arg)?;
                }
                if *interface {
                    let constant = self.pool.interface_method(
                        &method.owner,
                        &method.name,
                        &method.descriptor,
                    )?;
                    self.reference(0xb9, constant);
                    self.bytes.extend([
                        u8::try_from(args.len())
                            .map_err(|_| failure("Too many interface call arguments"))?,
                        0,
                    ]);
                } else {
                    let constant =
                        self.pool
                            .method(&method.owner, &method.name, &method.descriptor)?;
                    self.reference(if method.access & 2 != 0 { 0xb7 } else { 0xb6 }, constant);
                }
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
            ValueKind::Boolean(b) => self.bytes.push(if *b { 0x04 } else { 0x03 }),
            ValueKind::Field(name, receiver) => {
                self.value(receiver)?;
                let field = self.pool.field(self.owner, name, &value.ty.descriptor())?;
                self.reference(0xb4, field);
            }
            ValueKind::Local(slot) => self.bytes.extend([load_opcode(&value.ty), *slot]),
            ValueKind::Negate(inner) => {
                self.value(inner)?;
                self.bytes.push(0x74);
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
                for arg in args {
                    self.value(arg)?;
                }
                let function = &self.functions[*id];
                let method =
                    self.pool
                        .method(self.owner, &function.name, &function.descriptor())?;
                self.reference(0xb8, method);
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
                Statement::Store(slot, value) => {
                    self.value(value)?;
                    self.bytes.extend([store_opcode(&value.ty), *slot]);
                }
                Statement::Return(value) => {
                    self.value(value)?;
                    self.bytes.push(return_opcode(&value.ty));
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
                    self.value(start)?;
                    self.bytes.extend([0x36, *slot]);
                    self.value(end)?;
                    self.bytes.extend([0x36, *end_slot]);
                    let head = self.label();
                    let step = self.label();
                    let done = self.label();
                    self.mark(head);
                    self.bytes.extend([0x15, *slot, 0x15, *end_slot]);
                    self.max_stack = self.max_stack.max(2);
                    self.branch(0xa2, done);
                    self.loops.push((step, done));
                    self.block(body)?;
                    self.loops.pop();
                    self.mark(step);
                    self.bytes.extend([0x84, *slot, 1]);
                    self.branch(0xa7, head);
                    self.mark(done);
                }
                Statement::Expression(value) => {
                    self.value(value)?;
                    if value.ty.base() != &Type::Unit {
                        self.bytes.push(0x57);
                    }
                }
                Statement::If(cond, yes, no) => {
                    let otherwise = self.label();
                    let end = self.label();
                    self.value(cond)?;
                    self.branch(0x99, otherwise);
                    self.block(yes)?;
                    self.branch(0xa7, end);
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
    let code_name = pool.utf8("Code")?;
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let mut methods = Vec::new();
    for function in &module.functions {
        let name = pool.utf8(&function.name)?;
        let descriptor = pool.utf8(&function.descriptor())?;
        let mut code = Code {
            pool: &mut pool,
            functions: &module.functions,
            owner,
            bytes: Vec::new(),
            labels: Vec::new(),
            branches: Vec::new(),
            loops: Vec::new(),
            max_stack: 2,
        };
        if !function.private {
            for (i, (name, ty)) in function.params.iter().enumerate() {
                if ty.is_reference() && !matches!(ty, Type::Platform(_)) {
                    code.bytes.extend([0x19, i as u8]);
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
        let mut method = Vec::new();
        u2(&mut method, if function.private { 0x001a } else { 0x0019 });
        u2(&mut method, name);
        u2(&mut method, descriptor);
        u2(&mut method, 1);
        u2(&mut method, code_name);
        u4(&mut method, (12 + bytes.len()) as u32);
        u2(&mut method, max_stack);
        u2(&mut method, function.locals);
        u4(&mut method, bytes.len() as u32);
        method.extend(bytes);
        u2(&mut method, 0);
        u2(&mut method, 0);
        methods.push(method);
    }
    let metadata = metadata_attribute(&mut pool, metadata)?;
    let mut out = Vec::new();
    u4(&mut out, 0xcafebabe);
    u2(&mut out, 0);
    u2(&mut out, 49);
    pool.write(&mut out);
    u2(&mut out, 0x0031);
    u2(&mut out, class);
    u2(&mut out, parent);
    u2(&mut out, 0);
    u2(&mut out, 0);
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
        let (descriptor, value, not_null) = match &constant.value {
            ConstValue::Int(value) => ("I", pool.add(Constant::Integer(*value))?, false),
            ConstValue::Char(value) => {
                ("C", pool.add(Constant::Integer(i32::from(*value)))?, false)
            }
            ConstValue::Boolean(value) => {
                ("Z", pool.add(Constant::Integer(i32::from(*value)))?, false)
            }
            ConstValue::String(value) => ("Ljava/lang/String;", pool.string_units(value)?, true),
        };
        fields.extend(object_field(
            &mut pool,
            &constant.name,
            descriptor,
            Some(value),
            not_null,
        )?);
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
    fn integer(pool: &mut Pool, out: &mut Vec<u8>, value: i32) -> Result<()> {
        out.push(b'I');
        u2(out, pool.add(Constant::Integer(value))?);
        Ok(())
    }
    let mut annotation = Vec::new();
    u2(&mut annotation, 1);
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
    if !params.iter().any(|(_, ty)| ty.is_reference()) {
        return Ok(());
    }
    let mut annotation = Vec::new();
    annotation
        .push(u8::try_from(params.len()).map_err(|_| failure("Too many annotated parameters"))?);
    for (_, ty) in params {
        u2(&mut annotation, u16::from(ty.is_reference()));
        if ty.is_reference() {
            u2(
                &mut annotation,
                pool.utf8("Lorg/jetbrains/annotations/NotNull;")?,
            );
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

pub fn emit_class(
    class: &crate::check::ClassDecl,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
) -> Result<Vec<u8>> {
    emit_class_with(class, owner, file, metadata, 0x0001, |_| Ok(Vec::new()))
}

fn emit_class_with(
    class: &crate::check::ClassDecl,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
    constructor_flags: u16,
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
    for (name, ty) in &class.params {
        u2(&mut fields, 0x0012);
        u2(&mut fields, pool.utf8(name)?);
        u2(&mut fields, pool.utf8(&ty.descriptor())?);
        u2(&mut fields, u16::from(ty.is_reference()));
        if ty.is_reference() {
            u2(&mut fields, pool.utf8("RuntimeInvisibleAnnotations")?);
            u4(&mut fields, 6);
            u2(&mut fields, 1);
            u2(
                &mut fields,
                pool.utf8("Lorg/jetbrains/annotations/NotNull;")?,
            );
            u2(&mut fields, 0);
        }
    }
    let constructor_descriptor = format!(
        "({})V",
        class
            .params
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let mut code = SimpleCode::new(2, (class.params.len() + 1) as u16);
    let null_check = pool.method(
        "kotlin/jvm/internal/Intrinsics",
        "checkNotNullParameter",
        "(Ljava/lang/Object;Ljava/lang/String;)V",
    )?;
    for (index, (name, ty)) in class.params.iter().enumerate() {
        if ty.is_reference() {
            code.bytes.extend([0x19, (index + 1) as u8]);
            code.reference(0x13, pool.string(name)?);
            code.reference(0xb8, null_check);
        }
    }
    code.bytes.push(0x2a);
    code.reference(0xb7, pool.method("java/lang/Object", "<init>", "()V")?);
    for (index, (name, ty)) in class.params.iter().enumerate() {
        code.bytes.push(0x2a);
        code.bytes.extend([load_opcode(ty), (index + 1) as u8]);
        code.reference(0xb5, pool.field(owner, name, &ty.descriptor())?);
    }
    code.bytes.push(0xb1);
    let mut constructor = simple_method(
        &mut pool,
        constructor_flags,
        "<init>",
        &constructor_descriptor,
        code,
        None,
        false,
    )?;
    parameter_annotations(&mut pool, &mut constructor, &class.params)?;
    let mut methods = vec![constructor];
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
            max_stack: 2,
        };
        if !function.private {
            for (index, (name, ty)) in function.params.iter().enumerate() {
                if ty.is_reference() {
                    code.bytes.extend([0x19, (index + 1) as u8]);
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
        if !code.branches.is_empty() {
            return Err(failure("Control flow in class members is not supported"));
        }
        let code = SimpleCode {
            max_stack: u16::try_from(code.max_stack)
                .map_err(|_| failure("JVM operand stack limit exceeded"))?,
            locals: function.locals,
            bytes: code.bytes,
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
            None,
            function.result.is_reference(),
        )?;
        parameter_annotations(&mut pool, &mut method, &function.params)?;
        methods.push(method);
    }
    for bridge in &class.bridges {
        let (params, result) = crate::check::class_method_types(bridge)?;
        let mut code = SimpleCode::new((params.len() + 1) as u16, (params.len() + 1) as u16);
        code.bytes.push(0x2a);
        for (index, ty) in params.iter().enumerate() {
            code.bytes.extend([load_opcode(ty), (index + 1) as u8]);
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
            None,
            result.is_reference(),
        )?;
        let params: Vec<_> = params.into_iter().map(|ty| (String::new(), ty)).collect();
        parameter_annotations(&mut pool, &mut method, &params)?;
        methods.push(method);
    }
    let additional_attributes = additional_attributes(&mut pool)?;
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
    u2(&mut out, class.params.len() as u16);
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
