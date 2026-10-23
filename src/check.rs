use crate::classpath::{Classpath, JavaClass, JavaMethod};
use crate::{
    Diagnostic, Result,
    syntax::{self, Expr, ExprKind, StatementKind, Type},
};
use std::collections::{HashMap, HashSet};

pub struct Module {
    pub functions: Vec<Function>,
}
pub struct Function {
    pub name: String,
    pub private: bool,
    pub params: Vec<(String, Type)>,
    pub result: Type,
    pub locals: u16,
    pub body: Vec<Statement>,
}
impl Function {
    pub fn descriptor(&self) -> String {
        format!(
            "({}){}",
            self.params
                .iter()
                .map(|p| p.1.descriptor())
                .collect::<String>(),
            self.result.descriptor()
        )
    }
}
pub enum Statement {
    Store(u8, Value),
    Expression(Value),
    Return(Value),
    If(Value, Vec<Statement>, Vec<Statement>),
    While(Value, Vec<Statement>),
}
pub struct Value {
    pub ty: Type,
    pub kind: ValueKind,
}
#[derive(Clone, Copy)]
pub enum StringMethod {
    Length,
    IsEmpty,
    RemovePrefix,
    StartsWith,
    EndsWith,
    Substring,
    SubstringRange,
    CompareTo,
}
pub enum ValueKind {
    Unit,
    JavaCall {
        method: JavaMethod,
        args: Vec<Value>,
    },
    NewIntArray(Box<Value>),
    CopyInto(Vec<Value>),
    NotNull(Box<Value>, String),
    Int(i32),
    Boolean(bool),
    String(Vec<u16>),
    StringConcat(Box<Value>, Box<Value>),
    StringCall(StringMethod, Vec<Value>),
    Local(u8),
    Negate(Box<Value>),
    Not(Box<Value>),
    Binary(String, Box<Value>, Box<Value>),
    Length(Box<Value>),
    Index(Box<Value>, Box<Value>),
    Call(usize, Vec<Value>),
    Compare(Box<Value>, Box<Value>),
}
impl Value {
    pub fn stack(&self) -> usize {
        match &self.kind {
            ValueKind::Unit => 0,
            ValueKind::NewIntArray(v) => v.stack().max(1),
            ValueKind::NotNull(v, _) => v.stack().max(3),
            ValueKind::Int(_)
            | ValueKind::Boolean(_)
            | ValueKind::String(_)
            | ValueKind::Local(_) => 1,
            ValueKind::Negate(v) | ValueKind::Not(v) | ValueKind::Length(v) => v.stack().max(2),
            ValueKind::Binary(_, a, b) | ValueKind::Index(a, b) | ValueKind::Compare(a, b) => {
                a.stack().max(1 + b.stack())
            }
            ValueKind::StringConcat(a, b) => 2.max(1 + a.stack()).max(1 + b.stack()),
            ValueKind::Call(_, args)
            | ValueKind::StringCall(_, args)
            | ValueKind::CopyInto(args)
            | ValueKind::JavaCall { args, .. } => args
                .iter()
                .enumerate()
                .map(|(i, v)| i + v.stack())
                .max()
                .unwrap_or(1),
        }
    }
}
#[derive(Clone)]
struct Local {
    slot: u8,
    ty: Type,
    mutable: bool,
}
struct Checker<'a> {
    classpath: &'a Classpath,
    extensions: &'a HashSet<usize>,
    classes: &'a HashMap<String, JavaClass>,
    imports: &'a HashMap<String, String>,
    signatures: &'a Vec<(Vec<(String, Type)>, Type)>,
    receiver: Option<Local>,
    functions: &'a HashMap<String, usize>,
    scopes: Vec<HashMap<String, Local>>,
    next: u16,
    result: Type,
}
impl Checker<'_> {
    fn resolve(&self, ty: &Type, offset: usize) -> Result<Type> {
        resolve_type(ty, self.imports, offset)
    }
    fn coerce(&self, value: Value, expected: Type, offset: usize) -> Result<Value> {
        if value.ty == expected {
            return Ok(value);
        }
        if matches!(value.ty, Type::Platform(_)) && value.ty.base() == &expected {
            let message = match &value.kind {
                ValueKind::JavaCall { method, .. } => {
                    format!("{}(...)", method.name)
                }
                _ => String::new(),
            };
            return Ok(Value {
                ty: expected,
                kind: ValueKind::NotNull(Box::new(value), message),
            });
        }
        if matches!(expected, Type::Platform(_)) && value.ty.base() == expected.base() {
            return Ok(value);
        }
        Err(Diagnostic::new(
            offset,
            format!("Type mismatch: expected {expected:?}, found {:?}", value.ty),
        ))
    }
    fn java_type(&self, descriptor: &str, cursor: &mut usize, offset: usize) -> Result<Type> {
        let bytes = descriptor.as_bytes();
        let start = *cursor;
        *cursor += 1;
        let ty = match bytes.get(start) {
            Some(b'I') => Type::Int,
            Some(b'Z') => Type::Boolean,
            Some(b'V') => Type::Unit,
            Some(b'[') if bytes.get(*cursor) == Some(&b'I') => {
                *cursor += 1;
                Type::IntArray
            }
            Some(b'L') => {
                let end = descriptor[*cursor..]
                    .find(';')
                    .map(|n| n + *cursor)
                    .ok_or_else(|| Diagnostic::new(offset, "Invalid Java descriptor"))?;
                let name = &descriptor[*cursor..end];
                *cursor = end + 1;
                if name == "java/lang/String" {
                    Type::String
                } else if self.classes.contains_key(name) {
                    Type::Java(name.into())
                } else {
                    return Err(Diagnostic::new(
                        offset,
                        "Java reference types require explicit imports",
                    ));
                }
            }
            _ => return Err(Diagnostic::new(offset, "Unsupported Java descriptor type")),
        };
        Ok(ty)
    }
    fn check_ancestors(&self, class: &JavaClass, name: &str, offset: usize) -> Result<()> {
        if matches!(
            name,
            "equals"
                | "hashCode"
                | "toString"
                | "getClass"
                | "wait"
                | "notify"
                | "notifyAll"
                | "clone"
                | "finalize"
        ) {
            return Err(Diagnostic::new(
                offset,
                "Java inherited member resolution is not supported",
            ));
        }
        let mut pending: Vec<_> = class
            .super_name
            .iter()
            .chain(&class.interfaces)
            .cloned()
            .collect();
        let mut visited = HashSet::new();
        while let Some(owner) = pending.pop() {
            if owner == "java/lang/Object" || !visited.insert(owner.clone()) {
                continue;
            }
            if visited.len() > 128 {
                return Err(Diagnostic::new(
                    offset,
                    "Java hierarchy exceeds the current limit",
                ));
            }
            let ancestor = self.classpath.load(&owner)?;
            if ancestor.unsupported || ancestor.methods.iter().any(|method| method.name == name) {
                return Err(Diagnostic::new(
                    offset,
                    "Java inherited member or metadata resolution is not supported",
                ));
            }
            pending.extend(ancestor.super_name.into_iter().chain(ancestor.interfaces));
        }
        Ok(())
    }
    fn java_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let Type::Java(owner) = receiver.ty.base() else {
            unreachable!()
        };
        let class = self
            .classes
            .get(owner)
            .ok_or_else(|| Diagnostic::new(offset, "Java class requires explicit import"))?;
        if class.access & 0x200 != 0 {
            return Err(Diagnostic::new(
                offset,
                "Java interface calls are not supported",
            ));
        }
        self.check_ancestors(class, name, offset)?;
        let methods: Vec<_> = class.methods.iter().filter(|m| m.name == name).collect();
        if methods.len() != 1 {
            return Err(Diagnostic::new(
                offset,
                format!("Java method missing or overloaded: {name}"),
            ));
        }
        let method = methods[0].clone();
        if method.access & 1 == 0 || method.access & 8 != 0 || method.unsupported {
            return Err(Diagnostic::new(
                offset,
                "Java method is inaccessible, static, generic, annotated, or unsupported",
            ));
        }
        if !method.descriptor.starts_with('(') {
            return Err(Diagnostic::new(offset, "Invalid Java method descriptor"));
        }
        let mut cursor = 1;
        let mut expected = Vec::new();
        while method.descriptor.as_bytes().get(cursor) != Some(&b')') {
            expected.push(self.java_type(&method.descriptor, &mut cursor, offset)?);
        }
        cursor += 1;
        let result = self.java_type(&method.descriptor, &mut cursor, offset)?;
        if cursor != method.descriptor.len() || expected.contains(&Type::Unit) {
            return Err(Diagnostic::new(offset, "Invalid Java method descriptor"));
        }
        if args.len() != expected.len() {
            return Err(Diagnostic::new(offset, "Wrong Java argument count"));
        }
        let mut values = vec![receiver];
        for (arg, ty) in args.iter().zip(expected) {
            let value = self.value(arg)?;
            if value.ty.base() != &ty || ty == Type::Unit {
                return Err(Diagnostic::new(arg.offset, "Java argument type mismatch"));
            }
            values.push(value);
        }
        let ty = if result.is_reference() {
            Type::Platform(Box::new(result))
        } else {
            result
        };
        Ok(Value {
            ty,
            kind: ValueKind::JavaCall {
                method,
                args: values,
            },
        })
    }
    fn local(&self, name: &str) -> Option<Local> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }
    fn declare(&mut self, name: &str, ty: Type, mutable: bool, offset: usize) -> Result<u8> {
        if self.scopes.last().unwrap().contains_key(name) {
            return Err(Diagnostic::new(
                offset,
                format!("Conflicting local declaration: {name}"),
            ));
        }
        let slot = self.allocate_slot(offset)?;
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.into(), Local { slot, ty, mutable });
        Ok(slot)
    }
    fn allocate_slot(&mut self, offset: usize) -> Result<u8> {
        if self.next >= 256 {
            return Err(Diagnostic::new(
                offset,
                "More than 256 local slots are not supported",
            ));
        }
        let slot = self.next as u8;
        self.next += 1;
        Ok(slot)
    }
    fn require(&self, value: &Value, expected: Type, offset: usize) -> Result<()> {
        if value.ty != expected {
            Err(Diagnostic::new(
                offset,
                format!("Type mismatch: expected {expected:?}, found {:?}", value.ty),
            ))
        } else {
            Ok(())
        }
    }
    fn block(&mut self, source: &[syntax::Statement]) -> Result<(Vec<Statement>, bool)> {
        self.scopes.push(HashMap::new());
        let mut body = Vec::new();
        let mut returns = false;
        for statement in source {
            let offset = statement.offset;
            let typed = match &statement.kind {
                StatementKind::Let(name, mutable, annotation, expr) => {
                    let mut value = self.value(expr)?;
                    if let Some(ty) = annotation {
                        value = self.coerce(value, self.resolve(ty, offset)?, offset)?;
                    }
                    if value.ty == Type::Unit {
                        return Err(Diagnostic::new(offset, "Unit locals are not supported"));
                    }
                    let slot = self.declare(name, value.ty.clone(), *mutable, offset)?;
                    Statement::Store(slot, value)
                }
                StatementKind::Assign(name, expr) => {
                    let local = self.local(name).ok_or_else(|| {
                        Diagnostic::new(offset, format!("Unresolved local: {name}"))
                    })?;
                    if !local.mutable {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Cannot reassign val or parameter: {name}"),
                        ));
                    }
                    let value = self.value(expr)?;
                    let value = self.coerce(value, local.ty, offset)?;
                    Statement::Store(local.slot, value)
                }
                StatementKind::AddAssign(name, expr) => {
                    let local = self.local(name).ok_or_else(|| {
                        Diagnostic::new(offset, format!("Unresolved local: {name}"))
                    })?;
                    if !local.mutable {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Cannot reassign val or parameter: {name}"),
                        ));
                    }
                    if local.ty != Type::Int {
                        return Err(Diagnostic::new(
                            offset,
                            "+= supports only mutable Int locals",
                        ));
                    }
                    let right = self.value(expr)?;
                    self.require(&right, Type::Int, expr.offset)?;
                    let left = Value {
                        ty: Type::Int,
                        kind: ValueKind::Local(local.slot),
                    };
                    Statement::Store(
                        local.slot,
                        Value {
                            ty: Type::Int,
                            kind: ValueKind::Binary("+".into(), Box::new(left), Box::new(right)),
                        },
                    )
                }
                StatementKind::When(subject, branches, otherwise) => {
                    let subject = self.value(subject)?;
                    if !matches!(subject.ty, Type::Int | Type::Char) {
                        return Err(Diagnostic::new(
                            offset,
                            "when subjects support only Int and Char",
                        ));
                    }
                    let subject_type = subject.ty.clone();
                    let slot = self.allocate_slot(offset)?;
                    body.push(Statement::Store(slot, subject));
                    let mut checked = Vec::new();
                    let mut all_return = otherwise.is_some();
                    for (conditions, branch) in branches {
                        let mut combined = None;
                        for condition in conditions {
                            let right = self.value(condition)?;
                            self.require(&right, subject_type.clone(), condition.offset)?;
                            let left = Value {
                                ty: subject_type.clone(),
                                kind: ValueKind::Local(slot),
                            };
                            let equal = Value {
                                ty: Type::Boolean,
                                kind: ValueKind::Binary(
                                    "==".into(),
                                    Box::new(left),
                                    Box::new(right),
                                ),
                            };
                            combined = Some(match combined {
                                None => equal,
                                Some(previous) => Value {
                                    ty: Type::Boolean,
                                    kind: ValueKind::Binary(
                                        "||".into(),
                                        Box::new(previous),
                                        Box::new(equal),
                                    ),
                                },
                            });
                        }
                        let (branch, branch_returns) = self.block(branch)?;
                        all_return &= branch_returns;
                        checked.push((combined.unwrap(), branch));
                    }
                    let mut tail = if let Some(otherwise) = otherwise {
                        let (branch, branch_returns) = self.block(otherwise)?;
                        all_return &= branch_returns;
                        branch
                    } else {
                        Vec::new()
                    };
                    for (condition, branch) in checked.into_iter().rev() {
                        tail = vec![Statement::If(condition, branch, tail)];
                    }
                    body.extend(tail);
                    returns |= all_return;
                    continue;
                }
                StatementKind::Return(expr) => {
                    let value = match expr {
                        Some(expr) => self.value(expr)?,
                        None => Value {
                            ty: Type::Unit,
                            kind: ValueKind::Unit,
                        },
                    };
                    let value = self.coerce(value, self.result.clone(), offset)?;
                    returns = true;
                    Statement::Return(value)
                }
                StatementKind::Expression(expr) => Statement::Expression(self.value(expr)?),
                StatementKind::If(cond, yes, no) => {
                    let cond = self.value(cond)?;
                    self.require(&cond, Type::Boolean, offset)?;
                    let (yes, y) = self.block(yes)?;
                    let (no, n) = self.block(no)?;
                    returns |= y && n;
                    Statement::If(cond, yes, no)
                }
                StatementKind::While(cond, body) => {
                    let cond = self.value(cond)?;
                    self.require(&cond, Type::Boolean, offset)?;
                    let (body, _) = self.block(body)?;
                    Statement::While(cond, body)
                }
            };
            body.push(typed);
        }
        self.scopes.pop();
        Ok((body, returns))
    }
    fn value(&mut self, expr: &Expr) -> Result<Value> {
        let offset = expr.offset;
        let (ty, kind) = match &expr.kind {
            ExprKind::Int(value) => (
                Type::Int,
                ValueKind::Int(
                    i32::try_from(*value)
                        .map_err(|_| Diagnostic::new(offset, "Int literal is out of range"))?,
                ),
            ),
            ExprKind::Char(value) => (Type::Char, ValueKind::Int(i32::from(*value))),
            ExprKind::Boolean(value) => (Type::Boolean, ValueKind::Boolean(*value)),
            ExprKind::String(value) => (Type::String, ValueKind::String(value.clone())),
            ExprKind::Name(name) => {
                let local = if name == "this" {
                    self.receiver.clone()
                } else {
                    self.local(name)
                }
                .ok_or_else(|| Diagnostic::new(offset, format!("Unresolved local: {name}")))?;
                (local.ty, ValueKind::Local(local.slot))
            }
            ExprKind::Unary(op, inner) => {
                if op == "-" && matches!(inner.kind, ExprKind::Int(2147483648)) {
                    return Ok(Value {
                        ty: Type::Int,
                        kind: ValueKind::Int(i32::MIN),
                    });
                }
                let inner = self.value(inner)?;
                if op == "!" {
                    self.require(&inner, Type::Boolean, offset)?;
                    (Type::Boolean, ValueKind::Not(Box::new(inner)))
                } else {
                    self.require(&inner, Type::Int, offset)?;
                    if op == "+" {
                        return Ok(inner);
                    }
                    (Type::Int, ValueKind::Negate(Box::new(inner)))
                }
            }
            ExprKind::Binary(op, a, b) => {
                let a = self.value(a)?;
                let b = self.value(b)?;
                if a.ty == Type::Unit
                    || b.ty == Type::Unit
                    || matches!(a.ty.base(), Type::Java(_))
                    || matches!(b.ty.base(), Type::Java(_))
                    || matches!(a.ty, Type::Platform(_)) && a.ty.base() == &Type::String
                    || matches!(b.ty, Type::Platform(_)) && b.ty.base() == &Type::String
                {
                    return Err(Diagnostic::new(offset, "Unsupported value operand type"));
                }
                if op == "+" && a.ty == Type::String {
                    if !matches!(b.ty, Type::String | Type::Int | Type::Boolean) {
                        return Err(Diagnostic::new(
                            offset,
                            "String concatenation supports only String, Int, and Boolean operands",
                        ));
                    }
                    return Ok(Value {
                        ty: Type::String,
                        kind: ValueKind::StringConcat(Box::new(a), Box::new(b)),
                    });
                }
                let ty = if op == "&&" || op == "||" {
                    self.require(&a, Type::Boolean, offset)?;
                    self.require(&b, Type::Boolean, offset)?;
                    Type::Boolean
                } else if op == "==" || op == "!=" {
                    if matches!(a.ty.base(), Type::IntArray | Type::CharArray)
                        || matches!(b.ty.base(), Type::IntArray | Type::CharArray)
                    {
                        return Err(Diagnostic::new(offset, "Array equality is not supported"));
                    }
                    self.require(&b, a.ty.clone(), offset)?;
                    Type::Boolean
                } else if matches!(op.as_str(), "<" | ">" | "<=" | ">=") {
                    if !matches!(a.ty, Type::Int | Type::Char) {
                        return Err(Diagnostic::new(
                            offset,
                            "Ordered comparisons require Int or Char",
                        ));
                    }
                    self.require(&b, a.ty.clone(), offset)?;
                    Type::Boolean
                } else {
                    self.require(&a, Type::Int, offset)?;
                    self.require(&b, Type::Int, offset)?;
                    Type::Int
                };
                (ty, ValueKind::Binary(op.clone(), Box::new(a), Box::new(b)))
            }
            ExprKind::Property(receiver, name) => {
                if matches!(&receiver.kind, ExprKind::Name(n) if n == "Int")
                    && self.local("Int").is_none()
                {
                    let value = match name.as_str() {
                        "MIN_VALUE" => i32::MIN,
                        "MAX_VALUE" => i32::MAX,
                        _ => {
                            return Err(Diagnostic::new(
                                offset,
                                format!("Unsupported Int member: {name}"),
                            ));
                        }
                    };
                    (Type::Int, ValueKind::Int(value))
                } else {
                    let receiver = self.value(receiver)?;
                    if receiver.ty == Type::String && name == "length" {
                        return Ok(Value {
                            ty: Type::Int,
                            kind: ValueKind::StringCall(StringMethod::Length, vec![receiver]),
                        });
                    }
                    if name != "size" {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Unsupported property: {name}"),
                        ));
                    }
                    if !matches!(receiver.ty.base(), Type::IntArray | Type::CharArray) {
                        return Err(Diagnostic::new(offset, "Expected IntArray or CharArray"));
                    }
                    (Type::Int, ValueKind::Length(Box::new(receiver)))
                }
            }
            ExprKind::Index(array, index) => {
                let array = self.value(array)?;
                let element = match array.ty.base() {
                    Type::IntArray => Type::Int,
                    Type::CharArray => Type::Char,
                    _ => return Err(Diagnostic::new(offset, "Expected IntArray or CharArray")),
                };
                let index = self.value(index)?;
                self.require(&index, Type::Int, offset)?;
                (element, ValueKind::Index(Box::new(array), Box::new(index)))
            }
            ExprKind::Call(target, args) => match &target.kind {
                ExprKind::Name(name) => {
                    if self.local(name).is_some() {
                        return Err(Diagnostic::new(
                            offset,
                            "Invoking local values is not supported",
                        ));
                    }
                    if let Some(receiver) = self.receiver.clone() {
                        if let Type::Java(owner) = receiver.ty.base() {
                            if let Some(class) = self.classes.get(owner) {
                                self.check_ancestors(class, name, offset)?;
                            }
                            if self
                                .classes
                                .get(owner)
                                .is_some_and(|c| c.methods.iter().any(|m| m.name == *name))
                            {
                                if self.functions.contains_key(name) {
                                    return Err(Diagnostic::new(
                                        offset,
                                        "Ambiguous implicit receiver call",
                                    ));
                                }
                                return self.java_call(
                                    Value {
                                        ty: receiver.ty,
                                        kind: ValueKind::Local(receiver.slot),
                                    },
                                    name,
                                    args,
                                    offset,
                                );
                            }
                        }
                    }
                    if name == "IntArray" && !self.functions.contains_key(name) {
                        if args.len() != 1 {
                            return Err(Diagnostic::new(
                                offset,
                                "IntArray requires one size argument",
                            ));
                        }
                        let size = self.value(&args[0])?;
                        self.require(&size, Type::Int, offset)?;
                        return Ok(Value {
                            ty: Type::IntArray,
                            kind: ValueKind::NewIntArray(Box::new(size)),
                        });
                    }
                    let id = *self.functions.get(name).ok_or_else(|| {
                        Diagnostic::new(offset, format!("Unresolved function: {name}"))
                    })?;
                    if self.extensions.contains(&id) {
                        return Err(Diagnostic::new(
                            offset,
                            "Calling declared Kotlin extensions is not supported",
                        ));
                    }
                    let (params, result) = self.signatures[id].clone();
                    if args.len() != params.len() {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Wrong argument count for {name}"),
                        ));
                    }
                    let mut values = Vec::new();
                    for (arg, (_, expected)) in args.iter().zip(&params) {
                        let value = self.value(arg)?;
                        values.push(self.coerce(value, expected.clone(), arg.offset)?);
                    }
                    (result, ValueKind::Call(id, values))
                }
                ExprKind::Property(receiver, name) => {
                    if self
                        .functions
                        .get(name)
                        .is_some_and(|id| self.extensions.contains(id))
                    {
                        return Err(Diagnostic::new(
                            offset,
                            "Calling declared Kotlin extensions is not supported",
                        ));
                    }
                    let receiver = self.value(receiver)?;
                    if matches!(receiver.ty.base(), Type::Java(_)) {
                        return self.java_call(receiver, name, args, offset);
                    }
                    if receiver.ty.base() == &Type::IntArray && name == "copyInto" {
                        if args.len() != 4 {
                            return Err(Diagnostic::new(
                                offset,
                                "copyInto requires four explicit arguments",
                            ));
                        }
                        let receiver = self.coerce(receiver, Type::IntArray, offset)?;
                        let mut values = vec![receiver];
                        for (i, arg) in args.iter().enumerate() {
                            let value = self.value(arg)?;
                            let expected = if i == 0 { Type::IntArray } else { Type::Int };
                            if value.ty.base() != &expected {
                                return Err(Diagnostic::new(
                                    arg.offset,
                                    "copyInto argument type mismatch",
                                ));
                            }
                            let value = if i == 0 {
                                self.coerce(value, Type::IntArray, arg.offset)?
                            } else {
                                value
                            };
                            values.push(value);
                        }
                        return Ok(Value {
                            ty: Type::IntArray,
                            kind: ValueKind::CopyInto(values),
                        });
                    }
                    if matches!(receiver.ty, Type::Platform(_)) {
                        return Err(Diagnostic::new(
                            offset,
                            "Platform String operations are not supported",
                        ));
                    }
                    if receiver.ty == Type::Int && name == "compareTo" && args.len() == 1 {
                        let arg = self.value(&args[0])?;
                        self.require(&arg, Type::Int, args[0].offset)?;
                        (
                            Type::Int,
                            ValueKind::Compare(Box::new(receiver), Box::new(arg)),
                        )
                    } else {
                        self.require(&receiver, Type::String, offset)?;
                        let (method, expected, result): (StringMethod, &[Type], Type) = match name
                            .as_str()
                        {
                            "isEmpty" => (StringMethod::IsEmpty, &[], Type::Boolean),
                            "removePrefix" => {
                                (StringMethod::RemovePrefix, &[Type::String], Type::String)
                            }
                            "startsWith" => {
                                (StringMethod::StartsWith, &[Type::String], Type::Boolean)
                            }
                            "endsWith" => (StringMethod::EndsWith, &[Type::String], Type::Boolean),
                            "substring" if args.len() == 2 => (
                                StringMethod::SubstringRange,
                                &[Type::Int, Type::Int],
                                Type::String,
                            ),
                            "substring" => (StringMethod::Substring, &[Type::Int], Type::String),
                            "compareTo" => (StringMethod::CompareTo, &[Type::String], Type::Int),
                            _ => {
                                return Err(Diagnostic::new(
                                    offset,
                                    format!("Unsupported String method: {name}"),
                                ));
                            }
                        };
                        if args.len() != expected.len() {
                            return Err(Diagnostic::new(
                                offset,
                                format!("Wrong argument count for {name}"),
                            ));
                        }
                        let mut values = vec![receiver];
                        for (arg, ty) in args.iter().zip(expected) {
                            let value = self.value(arg)?;
                            self.require(&value, ty.clone(), arg.offset)?;
                            values.push(value);
                        }
                        (result, ValueKind::StringCall(method, values))
                    }
                }
                _ => return Err(Diagnostic::new(offset, "Unsupported call target")),
            },
        };
        Ok(Value { ty, kind })
    }
}
fn resolve_type(ty: &Type, imports: &HashMap<String, String>, offset: usize) -> Result<Type> {
    match ty {
        Type::Java(name) => imports
            .get(name)
            .map(|n| Type::Java(n.clone()))
            .ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    format!("Type requires an explicit class import: {name}"),
                )
            }),
        _ => Ok(ty.clone()),
    }
}
pub fn check(source: &syntax::Module, classpath: &Classpath) -> Result<Module> {
    let mut imports = HashMap::new();
    let mut classes = HashMap::new();
    for (alias, name, offset) in &source.imports {
        if matches!(
            alias.as_str(),
            "Int" | "Char" | "Boolean" | "String" | "IntArray" | "CharArray" | "Unit"
        ) {
            return Err(Diagnostic::new(
                *offset,
                "Imports shadowing builtin types are not supported",
            ));
        }
        let class = classpath.load(name)?;
        if class.access & 1 == 0 || class.unsupported {
            return Err(Diagnostic::new(
                *offset,
                "Imported class is inaccessible or unsupported",
            ));
        }
        if imports.insert(alias.clone(), name.clone()).is_some() {
            return Err(Diagnostic::new(*offset, "Conflicting class import"));
        }
        classes.insert(class.name.clone(), class);
    }
    let mut functions = HashMap::new();
    let mut signatures = Vec::new();
    let mut extensions = HashSet::new();
    for (i, function) in source.functions.iter().enumerate() {
        if functions.insert(function.name.clone(), i).is_some() {
            return Err(Diagnostic::new(
                function.offset,
                "Function overloads are not supported",
            ));
        }
        let mut params = Vec::new();
        if let Some(receiver) = &function.receiver {
            extensions.insert(i);
            params.push((
                format!("$this${}", function.name),
                resolve_type(receiver, &imports, function.offset)?,
            ));
        }
        for (name, ty) in &function.params {
            params.push((name.clone(), resolve_type(ty, &imports, function.offset)?));
        }
        if params.iter().any(|p| p.1 == Type::Unit) {
            return Err(Diagnostic::new(
                function.offset,
                "Unit parameters are not supported",
            ));
        }
        if params.len() > 255 {
            return Err(Diagnostic::new(
                function.offset,
                "JVM methods support at most 255 parameter slots",
            ));
        }
        signatures.push((
            params,
            resolve_type(&function.result, &imports, function.offset)?,
        ));
    }
    if functions.len() > 65535 {
        return Err(Diagnostic::new(0, "Too many functions for a JVM class"));
    }
    let mut result = Vec::new();
    for (id, function) in source.functions.iter().enumerate() {
        let (params, return_type) = signatures[id].clone();
        let mut checker = Checker {
            classpath,
            extensions: &extensions,
            classes: &classes,
            imports: &imports,
            signatures: &signatures,
            receiver: None,
            functions: &functions,
            scopes: vec![HashMap::new()],
            next: 0,
            result: return_type.clone(),
        };
        for (name, ty) in &params {
            checker.declare(name, ty.clone(), false, function.offset)?;
        }
        if function.receiver.is_some() {
            checker.receiver = checker.local(&params[0].0);
        }
        let (mut body, returns) = checker.block(&function.body)?;
        if !returns {
            if return_type != Type::Unit {
                return Err(Diagnostic::new(
                    function.offset,
                    "Function must return on all paths; unconditional-loop analysis is not supported",
                ));
            }
            body.push(Statement::Return(Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            }));
        }
        let mut params = params;
        if function.receiver.is_some() {
            params[0].0 = "<this>".into();
        }
        result.push(Function {
            name: function.name.clone(),
            private: function.private,
            params,
            result: return_type,
            locals: checker.next,
            body,
        });
    }
    Ok(Module { functions: result })
}
