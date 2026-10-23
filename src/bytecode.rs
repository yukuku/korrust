use crate::{
    Diagnostic, Result,
    check::{Module, Statement, StringMethod, Value, ValueKind},
    syntax::Type,
};
use std::collections::HashMap;

fn failure(message: &str) -> Diagnostic {
    Diagnostic::new(0, message)
}
fn u2(out: &mut Vec<u8>, value: u16) {
    out.extend(value.to_be_bytes());
}
fn u4(out: &mut Vec<u8>, value: u32) {
    out.extend(value.to_be_bytes());
}

#[derive(Clone, Hash, Eq, PartialEq)]
enum Constant {
    Utf8(Vec<u8>),
    Integer(i32),
    Class(u16),
    String(u16),
    NameType(u16, u16),
    Method(u16, u16),
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
    module: &'a Module,
    owner: &'a str,
    bytes: Vec<u8>,
    labels: Vec<Option<usize>>,
    branches: Vec<(usize, usize)>,
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
                        | Type::IntArray
                        | Type::CharArray
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
            ValueKind::Local(slot) => self
                .bytes
                .extend([if value.ty.is_reference() { 0x19 } else { 0x15 }, *slot]),
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
                    Type::CharArray => 0x34,
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
                let function = &self.module.functions[*id];
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
                    self.bytes
                        .extend([if value.ty.is_reference() { 0x3a } else { 0x36 }, *slot]);
                }
                Statement::Return(value) => {
                    self.value(value)?;
                    self.bytes.push(if value.ty.base() == &Type::Unit {
                        0xb1
                    } else if value.ty.is_reference() {
                        0xb0
                    } else {
                        0xac
                    });
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
                    self.block(body)?;
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
pub fn emit(module: &Module, owner: &str, file: &str) -> Result<Vec<u8>> {
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
            module,
            owner,
            bytes: Vec::new(),
            labels: Vec::new(),
            branches: Vec::new(),
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
    u2(&mut out, 1);
    u2(&mut out, source_name);
    u4(&mut out, 2);
    u2(&mut out, source_file);
    Ok(out)
}
