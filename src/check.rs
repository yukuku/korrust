use crate::classpath::{Classpath, JavaClass, JavaMethod};
use crate::{
    Diagnostic, Result,
    syntax::{self, Expr, ExprKind, StatementKind, Type},
};
use std::collections::{HashMap, HashSet};

#[path = "check_interfaces.rs"]
mod interfaces;
#[path = "check_objects.rs"]
mod objects;

pub struct Module {
    pub functions: Vec<Function>,
    pub class_decl: Option<ClassDecl>,
    pub interface_decl: Option<InterfaceDecl>,
    pub object_decl: Option<DynamicObject>,
}
pub struct DynamicObject {
    pub owner: String,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
}
pub struct ObjectField {
    pub name: String,
    pub ty: Type,
    pub initializer: ObjectInitializer,
}
pub enum ObjectInitializer {
    ThreadLocalArray {
        array_type: Type,
        size: Value,
        locals: u16,
    },
}
pub struct ObjectMethod {
    pub function: Function,
    pub jvm_static: bool,
    pub throws: Vec<String>,
    pub inline: bool,
}
pub struct InterfaceDecl {
    pub owner: String,
    pub interfaces: Vec<String>,
    pub methods: Vec<InterfaceMethod>,
    pub nested: Vec<InterfaceDecl>,
    pub companion: Option<CompanionDecl>,
    pub defaults: Vec<JavaMethod>,
}
pub struct InterfaceMethod {
    pub function: Function,
    pub has_body: bool,
    pub source_property: Option<String>,
}
pub struct CompanionDecl {
    pub owner: String,
    pub properties: Vec<CompanionProperty>,
}
pub struct CompanionProperty {
    pub name: String,
    pub private: bool,
    pub ty: Type,
    pub value: CompanionValue,
}
pub enum CompanionValue {
    StringArray(Vec<Vec<u16>>),
    Object {
        owner: String,
        class_decl: ClassDecl,
    },
}
pub struct ClassDecl {
    pub params: Vec<(String, Type)>,
    pub interfaces: Vec<String>,
    pub methods: Vec<Function>,
    pub bridges: Vec<JavaMethod>,
}
pub struct Function {
    pub name: String,
    pub private: bool,
    pub is_override: bool,
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
    Throw(Value),
    ThrowException {
        owner: String,
        message: Option<Value>,
    },
    IndexStore(Value, Value, Value),
    Continue,
    Break,
    ForUntil {
        slot: u8,
        start: Value,
        end_slot: u8,
        end: Value,
        body: Vec<Statement>,
    },
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
    Let {
        slot: u8,
        value: Box<Value>,
        body: Box<Value>,
    },
    NewArray(Type, Box<Value>),
    Convert(Type, Box<Value>),
    StaticField {
        owner: String,
        name: String,
    },
    ObjectInstance(String),
    ThreadLocalGet(Box<Value>, Type),
    ThreadLocalSet(Box<Value>, Box<Value>),
    CopyOf(Box<Value>, Box<Value>),
    StringFromChars(Vec<Value>),
    StaticCall {
        owner: String,
        name: String,
        descriptor: String,
        args: Vec<Value>,
    },
    Increment {
        slot: u8,
        prefix: bool,
    },
    RequireNotNull(Box<Value>),
    If(Box<Value>, Box<Value>, Box<Value>),
    IntToString(Box<Value>),
    CompanionArray {
        owner: String,
        name: String,
    },
    MemberCall {
        method: JavaMethod,
        interface: bool,
        args: Vec<Value>,
    },
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
    Field(String, Box<Value>),
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
            ValueKind::Let { value, body, .. } => value.stack().max(body.stack()),
            ValueKind::NewArray(_, value)
            | ValueKind::Convert(_, value)
            | ValueKind::ThreadLocalGet(value, _)
            | ValueKind::RequireNotNull(value) => value.stack().max(2),
            ValueKind::StaticField { .. }
            | ValueKind::ObjectInstance(_)
            | ValueKind::Increment { .. } => 2,
            ValueKind::ThreadLocalSet(a, b) | ValueKind::CopyOf(a, b) => {
                a.stack().max(1 + b.stack())
            }
            ValueKind::If(condition, yes, no) => condition.stack().max(yes.stack()).max(no.stack()),
            ValueKind::IntToString(value) => value.stack(),
            ValueKind::CompanionArray { .. } => 1,
            ValueKind::NewIntArray(v) => v.stack().max(1),
            ValueKind::Field(_, receiver) => receiver.stack(),
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
            ValueKind::StringFromChars(args) => args
                .iter()
                .enumerate()
                .map(|(index, value)| 2 + index + value.stack())
                .max()
                .unwrap_or(2),
            ValueKind::Call(_, args)
            | ValueKind::StaticCall { args, .. }
            | ValueKind::StringCall(_, args)
            | ValueKind::CopyInto(args)
            | ValueKind::JavaCall { args, .. }
            | ValueKind::MemberCall { args, .. } => args
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
    fields: &'a HashMap<String, Type>,
    members: &'a HashMap<String, JavaMethod>,
    companion_fields: &'a HashMap<String, (String, Type)>,
    functions: &'a HashMap<String, usize>,
    object_methods: &'a HashMap<String, Vec<JavaMethod>>,
    object_fields: &'a HashMap<String, (String, Type)>,
    object_owner: Option<&'a str>,
    initialized: HashSet<u8>,
    loop_depth: usize,
    scopes: Vec<HashMap<String, Local>>,
    next: u16,
    result: Type,
}
impl Checker<'_> {
    fn receiver_value(&self, offset: usize) -> Result<Value> {
        let receiver = self
            .receiver
            .as_ref()
            .ok_or_else(|| Diagnostic::new(offset, "Member access requires a receiver"))?;
        Ok(Value {
            ty: receiver.ty.clone(),
            kind: ValueKind::Local(receiver.slot),
        })
    }
    fn member_call(
        &mut self,
        method: JavaMethod,
        receiver: Value,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let (params, result) = class_method_types(&method)?;
        if params.len() != args.len() || result == Type::Float || params.contains(&Type::Float) {
            return Err(Diagnostic::new(
                offset,
                "Unsupported member call arguments or Float result",
            ));
        }
        let interface = self
            .classes
            .get(&method.owner)
            .is_some_and(|class| class.access & 0x0200 != 0);
        let mut values = vec![receiver];
        for (arg, ty) in args.iter().zip(params) {
            let value = self.value(arg)?;
            values.push(self.coerce(value, ty, arg.offset)?);
        }
        Ok(Value {
            ty: result,
            kind: ValueKind::MemberCall {
                method,
                interface,
                args: values,
            },
        })
    }
    fn resolve(&self, ty: &Type, offset: usize) -> Result<Type> {
        resolve_type(ty, self.imports, offset)
    }
    fn coerce(&self, value: Value, expected: Type, offset: usize) -> Result<Value> {
        if value.ty == expected {
            return Ok(value);
        }
        if expected == Type::Byte
            && value.ty == Type::Int
            && matches!(value.kind, ValueKind::Int(n) if (-128..=127).contains(&n))
        {
            return Ok(Value {
                ty: Type::Byte,
                kind: value.kind,
            });
        }
        if matches!(value.ty, Type::Platform(_)) && value.ty.base() == &expected {
            let message = match &value.kind {
                ValueKind::JavaCall { method, .. } => {
                    format!("{}(...)", method.name)
                }
                ValueKind::ThreadLocalGet(_, _) => "get(...)".into(),
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
            Some(b'B') => Type::Byte,
            Some(b'C') => Type::Char,
            Some(b'Z') => Type::Boolean,
            Some(b'V') => Type::Unit,
            Some(b'[') if bytes.get(*cursor) == Some(&b'B') => {
                *cursor += 1;
                Type::ByteArray
            }
            Some(b'[') if bytes.get(*cursor) == Some(&b'C') => {
                *cursor += 1;
                Type::CharArray
            }
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
        if inherited_object_method(name) {
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
        let named: Vec<_> = class
            .methods
            .iter()
            .filter(|method| method.name == name)
            .collect();
        let methods: Vec<_> = if named.len() == 1 {
            named
        } else {
            named
                .into_iter()
                .filter(|method| objects::arity(&method.descriptor) == Some(args.len()))
                .collect()
        };
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
        self.initialized.insert(slot);
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
            if returns {
                return Err(Diagnostic::new(
                    offset,
                    "Unreachable statements are not supported",
                ));
            }
            let typed = match &statement.kind {
                StatementKind::Declare(name, mutable, annotation) => {
                    let ty = self.resolve(annotation, offset)?;
                    if matches!(ty, Type::Unit | Type::Float | Type::ThreadLocal(_)) {
                        return Err(Diagnostic::new(
                            offset,
                            "Unsupported uninitialized local type",
                        ));
                    }
                    if !mutable {
                        return Err(Diagnostic::new(
                            offset,
                            "Uninitialized val declarations are not supported",
                        ));
                    }
                    let slot = self.declare(name, ty, *mutable, offset)?;
                    self.initialized.remove(&slot);
                    continue;
                }
                StatementKind::IndexAssign(array, index, value) => {
                    let array = self.value(array)?;
                    let element = match array.ty.base() {
                        Type::IntArray => Type::Int,
                        Type::ByteArray => Type::Byte,
                        Type::CharArray => Type::Char,
                        _ => {
                            return Err(Diagnostic::new(
                                offset,
                                "Array assignment supports IntArray, ByteArray, and CharArray",
                            ));
                        }
                    };
                    let index = self.value(index)?;
                    self.require(&index, Type::Int, offset)?;
                    let value = self.value(value)?;
                    let value = self.coerce(value, element, offset)?;
                    Statement::IndexStore(array, index, value)
                }
                StatementKind::Continue | StatementKind::Break => {
                    if self.loop_depth == 0 {
                        return Err(Diagnostic::new(
                            offset,
                            "Loop control requires an enclosing loop",
                        ));
                    }
                    returns = true;
                    if matches!(statement.kind, StatementKind::Continue) {
                        Statement::Continue
                    } else {
                        Statement::Break
                    }
                }
                StatementKind::ForUntil(name, start, end, source_body) => {
                    let start = self.value(start)?;
                    let end = self.value(end)?;
                    self.require(&start, Type::Int, offset)?;
                    self.require(&end, Type::Int, offset)?;
                    let before = self.initialized.clone();
                    self.scopes.push(HashMap::new());
                    let slot = self.declare(name, Type::Int, false, offset)?;
                    let end_slot = self.allocate_slot(offset)?;
                    self.loop_depth += 1;
                    let (body, _) = self.block(source_body)?;
                    self.loop_depth -= 1;
                    self.scopes.pop();
                    self.initialized = before;
                    Statement::ForUntil {
                        slot,
                        start,
                        end_slot,
                        end,
                        body,
                    }
                }
                StatementKind::Let(name, mutable, annotation, expr) => {
                    let mut value = self.value(expr)?;
                    if let Some(ty) = annotation {
                        if matches!(ty, Type::ThreadLocal(_)) {
                            return Err(Diagnostic::new(
                                offset,
                                "Explicit ThreadLocal local types are not supported",
                            ));
                        }
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
                        if self.fields.contains_key(name) {
                            Diagnostic::new(
                                offset,
                                format!("Cannot reassign val or parameter: {name}"),
                            )
                        } else {
                            Diagnostic::new(offset, format!("Unresolved local: {name}"))
                        }
                    })?;
                    if !local.mutable {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Cannot reassign val or parameter: {name}"),
                        ));
                    }
                    let value = self.value(expr)?;
                    let value = self.coerce(value, local.ty, offset)?;
                    self.initialized.insert(local.slot);
                    Statement::Store(local.slot, value)
                }
                StatementKind::AddAssign(name, expr) => {
                    let local = self.local(name).ok_or_else(|| {
                        if self.fields.contains_key(name) {
                            Diagnostic::new(
                                offset,
                                format!("Cannot reassign val or parameter: {name}"),
                            )
                        } else {
                            Diagnostic::new(offset, format!("Unresolved local: {name}"))
                        }
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
                    if !self.initialized.contains(&local.slot) {
                        return Err(Diagnostic::new(
                            offset,
                            "Variable must be initialized before use",
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
                    let before = self.initialized.clone();
                    let mut continuing = Vec::new();
                    let mut checked = Vec::new();
                    let mut all_return = otherwise.is_some();
                    for (conditions, branch) in branches {
                        self.initialized = before.clone();
                        let mut combined = None;
                        for condition in conditions {
                            let left = Value {
                                ty: subject_type.clone(),
                                kind: ValueKind::Local(slot),
                            };
                            let equal = if let ExprKind::Unary(operator, range) = &condition.kind {
                                if operator != "in" {
                                    return Err(Diagnostic::new(
                                        condition.offset,
                                        "Unsupported when range condition",
                                    ));
                                }
                                self.range_contains(left, range, condition.offset)?
                            } else {
                                let right = self.value(condition)?;
                                self.require(&right, subject_type.clone(), condition.offset)?;
                                Value {
                                    ty: Type::Boolean,
                                    kind: ValueKind::Binary(
                                        "==".into(),
                                        Box::new(left),
                                        Box::new(right),
                                    ),
                                }
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
                        if !branch_returns {
                            continuing.push(self.initialized.clone());
                        }
                        all_return &= branch_returns;
                        checked.push((combined.unwrap(), branch));
                    }
                    self.initialized = before.clone();
                    let mut tail = if let Some(otherwise) = otherwise {
                        let (branch, branch_returns) = self.block(otherwise)?;
                        if !branch_returns {
                            continuing.push(self.initialized.clone());
                        }
                        all_return &= branch_returns;
                        branch
                    } else {
                        continuing.push(before.clone());
                        Vec::new()
                    };
                    self.initialized = continuing
                        .into_iter()
                        .reduce(|previous, current| {
                            previous.intersection(&current).copied().collect()
                        })
                        .unwrap_or(before);
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
                StatementKind::Throw(expr) => {
                    let statement = self.throw_exception(expr, offset)?;
                    returns = true;
                    statement
                }
                StatementKind::Expression(expr) => Statement::Expression(self.value(expr)?),
                StatementKind::If(cond, yes, no) => {
                    let cond = self.value(cond)?;
                    self.require(&cond, Type::Boolean, offset)?;
                    let before = self.initialized.clone();
                    let (yes, y) = self.block(yes)?;
                    let yes_initialized = self.initialized.clone();
                    self.initialized = before;
                    let (no, n) = self.block(no)?;
                    let no_initialized = self.initialized.clone();
                    self.initialized = match (y, n) {
                        (false, false) => yes_initialized
                            .intersection(&no_initialized)
                            .copied()
                            .collect(),
                        (false, true) => yes_initialized,
                        (true, false) => no_initialized,
                        (true, true) => yes_initialized
                            .intersection(&no_initialized)
                            .copied()
                            .collect(),
                    };
                    returns |= y && n;
                    Statement::If(cond, yes, no)
                }
                StatementKind::While(cond, body) => {
                    let cond = self.value(cond)?;
                    self.require(&cond, Type::Boolean, offset)?;
                    let before = self.initialized.clone();
                    self.loop_depth += 1;
                    let (body, _) = self.block(body)?;
                    self.loop_depth -= 1;
                    self.initialized = before;
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
            ExprKind::Increment { value, prefix } => {
                let ExprKind::Name(name) = &value.kind else {
                    return Err(Diagnostic::new(
                        offset,
                        "Increment supports mutable Int locals only",
                    ));
                };
                let local = self
                    .local(name)
                    .ok_or_else(|| Diagnostic::new(offset, "Increment requires a local"))?;
                if !local.mutable
                    || local.ty != Type::Int
                    || !self.initialized.contains(&local.slot)
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Increment supports initialized mutable Int locals only",
                    ));
                }
                (
                    Type::Int,
                    ValueKind::Increment {
                        slot: local.slot,
                        prefix: *prefix,
                    },
                )
            }
            ExprKind::NonNull(value) => {
                let value = self.value(value)?;
                if !value.ty.is_reference() {
                    return Err(Diagnostic::new(
                        offset,
                        "Not-null assertions require reference values",
                    ));
                }
                (
                    value.ty.base().clone(),
                    ValueKind::RequireNotNull(Box::new(value)),
                )
            }
            ExprKind::Lambda(_) => {
                return Err(Diagnostic::new(
                    offset,
                    "Lambdas are supported only in private ThreadLocal array initializers",
                ));
            }
            ExprKind::When(subject, branches, otherwise) => {
                let subject = self.value(subject)?;
                if !matches!(subject.ty, Type::Int | Type::Char | Type::Byte) {
                    return Err(Diagnostic::new(
                        offset,
                        "when expression subjects support Int, Byte, and Char",
                    ));
                }
                let slot = self.allocate_slot(offset)?;
                let mut checked = Vec::new();
                for (conditions, branch) in branches {
                    let mut combined = None;
                    for condition in conditions {
                        let left = Value {
                            ty: subject.ty.clone(),
                            kind: ValueKind::Local(slot),
                        };
                        let condition = if let ExprKind::Unary(operator, range) = &condition.kind {
                            if operator != "in" {
                                return Err(Diagnostic::new(
                                    condition.offset,
                                    "Unsupported when range condition",
                                ));
                            }
                            self.range_contains(left, range, condition.offset)?
                        } else {
                            let right = self.value(condition)?;
                            self.require(&right, subject.ty.clone(), condition.offset)?;
                            Value {
                                ty: Type::Boolean,
                                kind: ValueKind::Binary(
                                    "==".into(),
                                    Box::new(left),
                                    Box::new(right),
                                ),
                            }
                        };
                        combined = Some(match combined {
                            None => condition,
                            Some(previous) => Value {
                                ty: Type::Boolean,
                                kind: ValueKind::Binary(
                                    "||".into(),
                                    Box::new(previous),
                                    Box::new(condition),
                                ),
                            },
                        });
                    }
                    checked.push((
                        combined.ok_or_else(|| Diagnostic::new(offset, "Empty when condition"))?,
                        self.value(branch)?,
                    ));
                }
                let mut body = self.value(otherwise)?;
                if body.ty == Type::Unit {
                    return Err(Diagnostic::new(
                        offset,
                        "Unit when expressions are not supported",
                    ));
                }
                for (condition, branch) in checked.into_iter().rev() {
                    self.require(&branch, body.ty.clone(), offset)?;
                    body = Value {
                        ty: body.ty.clone(),
                        kind: ValueKind::If(Box::new(condition), Box::new(branch), Box::new(body)),
                    };
                }
                (
                    body.ty.clone(),
                    ValueKind::Let {
                        slot,
                        value: Box::new(subject),
                        body: Box::new(body),
                    },
                )
            }
            ExprKind::If(condition, yes, no) => {
                let condition = self.value(condition)?;
                self.require(&condition, Type::Boolean, offset)?;
                let yes = self.value(yes)?;
                let no = self.value(no)?;
                if yes.ty == Type::Unit {
                    return Err(Diagnostic::new(
                        offset,
                        "Unit if expressions are not supported",
                    ));
                }
                if yes.ty != no.ty {
                    return Err(Diagnostic::new(
                        offset,
                        "Type mismatch between if expression branches; common type inference is not supported",
                    ));
                }
                (
                    yes.ty.clone(),
                    ValueKind::If(Box::new(condition), Box::new(yes), Box::new(no)),
                )
            }
            ExprKind::Template(parts) => {
                let mut result = Value {
                    ty: Type::String,
                    kind: ValueKind::String(Vec::new()),
                };
                for part in parts {
                    let value = self.value(part)?;
                    if !matches!(value.ty, Type::String | Type::Int | Type::Boolean) {
                        return Err(Diagnostic::new(
                            part.offset,
                            "String templates support only String, Int, and Boolean values",
                        ));
                    }
                    result = Value {
                        ty: Type::String,
                        kind: ValueKind::StringConcat(Box::new(result), Box::new(value)),
                    };
                }
                return Ok(result);
            }
            ExprKind::Name(name) => {
                if let Some(local) = if name == "this" {
                    self.receiver.clone()
                } else {
                    self.local(name)
                } {
                    if name != "this" && !self.initialized.contains(&local.slot) {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Variable must be initialized before use: {name}"),
                        ));
                    }
                    (local.ty, ValueKind::Local(local.slot))
                } else if let Some((owner, ty)) = self.object_fields.get(name) {
                    (
                        ty.clone(),
                        ValueKind::StaticField {
                            owner: owner.clone(),
                            name: name.clone(),
                        },
                    )
                } else if name == "this" && self.object_owner.is_some() {
                    let owner = self.object_owner.unwrap();
                    (
                        Type::Java(owner.into()),
                        ValueKind::ObjectInstance(owner.into()),
                    )
                } else if let Some(ty) = self.fields.get(name) {
                    let receiver = self.receiver.as_ref().ok_or_else(|| {
                        Diagnostic::new(offset, "Instance field requires a receiver")
                    })?;
                    (
                        ty.clone(),
                        ValueKind::Field(
                            name.clone(),
                            Box::new(Value {
                                ty: receiver.ty.clone(),
                                kind: ValueKind::Local(receiver.slot),
                            }),
                        ),
                    )
                } else if let Some(method) = self
                    .members
                    .values()
                    .find(|method| method.source_property.as_deref() == Some(name))
                {
                    let receiver = self.receiver_value(offset)?;
                    return self.member_call(method.clone(), receiver, &[], offset);
                } else if let Some((owner, ty)) = self.companion_fields.get(name) {
                    (
                        ty.clone(),
                        ValueKind::CompanionArray {
                            owner: owner.clone(),
                            name: name.clone(),
                        },
                    )
                } else {
                    return Err(Diagnostic::new(offset, format!("Unresolved local: {name}")));
                }
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
                if op == "in" || op == "!in" {
                    let value = self.value(a)?;
                    let value = self.range_contains(value, b, offset)?;
                    return Ok(if op == "!in" {
                        Value {
                            ty: Type::Boolean,
                            kind: ValueKind::Not(Box::new(value)),
                        }
                    } else {
                        value
                    });
                }
                if op == ".." {
                    return Err(Diagnostic::new(
                        offset,
                        "Range values are supported only by membership and when conditions",
                    ));
                }
                let a = self.value(a)?;
                let b = self.value(b)?;
                if matches!(a.ty, Type::Unit | Type::Float)
                    || matches!(b.ty, Type::Unit | Type::Float)
                    || matches!(a.ty.base(), Type::Java(_) | Type::ThreadLocal(_))
                    || matches!(b.ty.base(), Type::Java(_) | Type::ThreadLocal(_))
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
                    if matches!(
                        a.ty.base(),
                        Type::IntArray | Type::ByteArray | Type::CharArray | Type::StringArray
                    ) || matches!(
                        b.ty.base(),
                        Type::IntArray | Type::ByteArray | Type::CharArray | Type::StringArray
                    ) {
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
                    && self.unshadowed("Int")
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
                    if self
                        .receiver
                        .as_ref()
                        .is_some_and(|this| this.ty == receiver.ty)
                    {
                        if let Some(ty) = self.fields.get(name) {
                            return Ok(Value {
                                ty: ty.clone(),
                                kind: ValueKind::Field(name.clone(), Box::new(receiver)),
                            });
                        }
                    }
                    if self
                        .receiver
                        .as_ref()
                        .is_some_and(|this| this.ty == receiver.ty)
                    {
                        if let Some(method) = self
                            .members
                            .values()
                            .find(|method| method.source_property.as_deref() == Some(name))
                        {
                            return self.member_call(method.clone(), receiver, &[], offset);
                        }
                    }
                    if receiver.ty == Type::Char && name == "code" {
                        return Ok(Value {
                            ty: Type::Int,
                            kind: ValueKind::Convert(Type::Int, Box::new(receiver)),
                        });
                    }
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
                    if !matches!(
                        receiver.ty.base(),
                        Type::IntArray | Type::ByteArray | Type::CharArray | Type::StringArray
                    ) {
                        return Err(Diagnostic::new(offset, "Expected IntArray or CharArray"));
                    }
                    (Type::Int, ValueKind::Length(Box::new(receiver)))
                }
            }
            ExprKind::Index(array, index) => {
                let array = self.value(array)?;
                let element = match array.ty.base() {
                    Type::IntArray => Type::Int,
                    Type::ByteArray => Type::Byte,
                    Type::CharArray => Type::Char,
                    Type::StringArray => Type::String,
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
                    if self.object_methods.contains_key(name) {
                        return self.object_call(name, args, None, offset);
                    }
                    if let Some(method) = self.members.get(name).cloned() {
                        if method.source_property.is_some() {
                            return Err(Diagnostic::new(
                                offset,
                                "Kotlin properties cannot be invoked as functions",
                            ));
                        }
                        let receiver = self.receiver_value(offset)?;
                        return self.member_call(method, receiver, args, offset);
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
                    if matches!(name.as_str(), "IntArray" | "ByteArray" | "CharArray")
                        && self.unshadowed(name)
                    {
                        if args.len() != 1 {
                            return Err(Diagnostic::new(
                                offset,
                                "Array allocation requires one size argument",
                            ));
                        }
                        let size = self.value(&args[0])?;
                        self.require(&size, Type::Int, args[0].offset)?;
                        let array = match name.as_str() {
                            "IntArray" => Type::IntArray,
                            "ByteArray" => Type::ByteArray,
                            _ => Type::CharArray,
                        };
                        return Ok(Value {
                            ty: array.clone(),
                            kind: if array == Type::IntArray {
                                ValueKind::NewIntArray(Box::new(size))
                            } else {
                                ValueKind::NewArray(array, Box::new(size))
                            },
                        });
                    }
                    if name == "String" && self.unshadowed(name) {
                        if args.len() != 3 {
                            return Err(Diagnostic::new(
                                offset,
                                "String array constructor requires CharArray, offset, and count",
                            ));
                        }
                        let mut values = Vec::new();
                        for (index, arg) in args.iter().enumerate() {
                            let value = self.value(arg)?;
                            let expected = if index == 0 {
                                Type::CharArray
                            } else {
                                Type::Int
                            };
                            values.push(self.coerce(value, expected, arg.offset)?);
                        }
                        return Ok(Value {
                            ty: Type::String,
                            kind: ValueKind::StringFromChars(values),
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
                    if matches!(&receiver.kind, ExprKind::Name(name) if name == "Character")
                        && self.unshadowed("Character")
                    {
                        if args.len() != 1 {
                            return Err(Diagnostic::new(
                                offset,
                                "Character conversion requires one Int argument",
                            ));
                        }
                        let (descriptor, result) = match name.as_str() {
                            "toLowerCase" => ("(I)I", Type::Int),
                            "highSurrogate" | "lowSurrogate" => ("(I)C", Type::Char),
                            _ => {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Unsupported Character static method",
                                ));
                            }
                        };
                        let value = self.value(&args[0])?;
                        self.require(&value, Type::Int, args[0].offset)?;
                        return Ok(Value {
                            ty: result,
                            kind: ValueKind::StaticCall {
                                owner: "java/lang/Character".into(),
                                name: name.clone(),
                                descriptor: descriptor.into(),
                                args: vec![value],
                            },
                        });
                    }
                    let receiver = self.value(receiver)?;
                    if self
                        .object_owner
                        .is_some_and(|owner| receiver.ty == Type::Java(owner.into()))
                        && self.object_methods.contains_key(name)
                    {
                        return self.object_call(name, args, Some(receiver), offset);
                    }
                    if let Type::ThreadLocal(element) = receiver.ty.base() {
                        let element = element.as_ref().clone();
                        match name.as_str() {
                            "get" if args.is_empty() => {
                                return Ok(Value {
                                    ty: Type::Platform(Box::new(element.clone())),
                                    kind: ValueKind::ThreadLocalGet(Box::new(receiver), element),
                                });
                            }
                            "set" if args.len() == 1 => {
                                let value = self.value(&args[0])?;
                                if value.ty.base() != &element {
                                    return Err(Diagnostic::new(
                                        offset,
                                        "ThreadLocal.set value type mismatch",
                                    ));
                                }
                                return Ok(Value {
                                    ty: Type::Unit,
                                    kind: ValueKind::ThreadLocalSet(
                                        Box::new(receiver),
                                        Box::new(value),
                                    ),
                                });
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Unsupported ThreadLocal call",
                                ));
                            }
                        }
                    }
                    if matches!(receiver.ty, Type::Int | Type::Byte | Type::Char)
                        && args.is_empty()
                        && matches!(name.as_str(), "toInt" | "toByte" | "toChar")
                    {
                        let target = match name.as_str() {
                            "toByte" => Type::Byte,
                            "toChar" => Type::Char,
                            _ => Type::Int,
                        };
                        return Ok(Value {
                            ty: target.clone(),
                            kind: ValueKind::Convert(target, Box::new(receiver)),
                        });
                    }
                    if matches!(
                        receiver.ty,
                        Type::CharArray | Type::ByteArray | Type::IntArray
                    ) && name == "copyOf"
                        && args.len() == 1
                    {
                        let size = self.value(&args[0])?;
                        self.require(&size, Type::Int, args[0].offset)?;
                        return Ok(Value {
                            ty: receiver.ty.clone(),
                            kind: ValueKind::CopyOf(Box::new(receiver), Box::new(size)),
                        });
                    }

                    if self
                        .receiver
                        .as_ref()
                        .is_some_and(|this| this.ty == receiver.ty)
                    {
                        if let Some(method) = self.members.get(name).cloned() {
                            if method.source_property.is_some() {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Kotlin properties cannot be invoked as functions",
                                ));
                            }
                            return self.member_call(method, receiver, args, offset);
                        }
                    }
                    if receiver.ty == Type::Int && name == "toString" && args.is_empty() {
                        return Ok(Value {
                            ty: Type::String,
                            kind: ValueKind::IntToString(Box::new(receiver)),
                        });
                    }
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
        Type::ThreadLocal(_) => Err(Diagnostic::new(
            offset,
            "Explicit ThreadLocal types are not supported; only inferred private array buffers are supported",
        )),
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
        if source
            .class_decl
            .as_ref()
            .is_some_and(|decl| decl.name == *alias)
        {
            return Err(Diagnostic::new(
                *offset,
                "Class import conflicts with class declaration",
            ));
        }
        if source
            .object_decl
            .as_ref()
            .is_some_and(|declaration| declaration.name == *alias)
        {
            return Err(Diagnostic::new(
                *offset,
                "Class import conflicts with object declaration",
            ));
        }
        if source
            .enum_decl
            .as_ref()
            .is_some_and(|declaration| declaration.name == *alias)
        {
            return Err(Diagnostic::new(
                *offset,
                "Class import conflicts with enum declaration",
            ));
        }
        if matches!(
            alias.as_str(),
            "Int"
                | "Byte"
                | "ByteArray"
                | "ThreadLocal"
                | "Char"
                | "Float"
                | "Boolean"
                | "String"
                | "IntArray"
                | "CharArray"
                | "StringArray"
                | "Array"
                | "Unit"
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
    if let Some(object) = &source.object_decl {
        if !object.methods.is_empty() || !object.fields.is_empty() {
            return objects::check(object, &source.package, classpath, &imports, &classes);
        }
    }
    if let Some(interface) = &source.interface_decl {
        return interfaces::check(interface, &source.package, classpath, imports, classes);
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
        if params
            .iter()
            .any(|p| matches!(p.1, Type::Unit | Type::Float))
            || function.result == Type::Float
        {
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
    if let Some(decl) = &source.class_decl {
        return Ok(Module {
            functions: Vec::new(),
            interface_decl: None,
            object_decl: None,
            class_decl: Some(check_class(
                decl,
                &source.package,
                classpath,
                &imports,
                &classes,
                &HashMap::new(),
                None,
            )?),
        });
    }
    let fields = HashMap::new();
    let members = HashMap::new();
    let companion_fields = HashMap::new();
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
            fields: &fields,
            members: &members,
            companion_fields: &companion_fields,
            functions: &functions,
            object_methods: &HashMap::new(),
            object_fields: &HashMap::new(),
            object_owner: None,
            initialized: HashSet::new(),
            loop_depth: 0,
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
            is_override: false,
            params,
            result: return_type,
            locals: checker.next,
            body,
        });
    }
    Ok(Module {
        functions: result,
        class_decl: None,
        interface_decl: None,
        object_decl: None,
    })
}

pub(crate) fn getter_name(name: &str) -> String {
    if name.starts_with("is")
        && name
            .as_bytes()
            .get(2)
            .is_some_and(|c| !c.is_ascii_lowercase())
    {
        name.into()
    } else {
        format!("get{}{}", name[..1].to_ascii_uppercase(), &name[1..])
    }
}

#[allow(clippy::too_many_arguments)]
fn check_class(
    source: &syntax::ClassDecl,
    package: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    companion_fields: &HashMap<String, (String, Type)>,
    owner_override: Option<&str>,
) -> Result<ClassDecl> {
    let owner = if let Some(owner) = owner_override {
        owner.into()
    } else if package.is_empty() {
        source.name.clone()
    } else {
        format!("{}/{}", package.replace('.', "/"), source.name)
    };
    let mut params = Vec::new();
    let mut fields = HashMap::new();
    for (name, ty) in &source.params {
        let ty = resolve_type(ty, imports, source.offset)?;
        if matches!(ty, Type::Unit | Type::Float) {
            return Err(Diagnostic::new(
                source.offset,
                "Unit parameters are not supported",
            ));
        }
        if fields.insert(name.clone(), ty.clone()).is_some() {
            return Err(Diagnostic::new(
                source.offset,
                "Conflicting constructor field",
            ));
        }
        params.push((name.clone(), ty));
    }
    if params.len() > 254 {
        return Err(Diagnostic::new(
            source.offset,
            "Instance methods support at most 254 parameter slots",
        ));
    }
    let mut interfaces = Vec::new();
    let mut required = HashMap::new();
    for alias in &source.interfaces {
        let name = imports
            .get(alias)
            .ok_or_else(|| Diagnostic::new(source.offset, "Interfaces require explicit imports"))?;
        let interface = &classes[name];
        if interfaces.contains(name) {
            return Err(Diagnostic::new(source.offset, "Duplicate interface"));
        }
        interfaces.push(name.clone());
        for (method_name, method) in
            interfaces::interface_methods(interface, classes, source.offset)?
        {
            if required.insert(method_name, method).is_some() {
                return Err(Diagnostic::new(
                    source.offset,
                    "Overloaded or conflicting interface methods are not supported",
                ));
            }
        }
    }

    let mut declarations = Vec::new();
    for property in &source.properties {
        if property.ty == Type::Unit {
            return Err(Diagnostic::new(
                property.offset,
                "Unit properties are not supported",
            ));
        }
        if fields.contains_key(&property.name) {
            return Err(Diagnostic::new(
                property.offset,
                "Conflicting property declaration",
            ));
        }
        declarations.push((
            getter_name(&property.name),
            false,
            property.is_override,
            Some(property.name.as_str()),
            Vec::new(),
            &property.ty,
            &property.body,
            property.offset,
        ));
    }
    for method in &source.methods {
        let function = &method.function;
        if function.receiver.is_some() {
            return Err(Diagnostic::new(
                function.offset,
                "Member extension functions are not supported",
            ));
        }
        declarations.push((
            function.name.clone(),
            function.private,
            method.is_override,
            None,
            function.params.clone(),
            &function.result,
            &function.body,
            function.offset,
        ));
    }
    let mut members = required.clone();
    for (name, private, _, property, source_params, result, _, offset) in &declarations {
        let params = source_params
            .iter()
            .map(|(_, ty)| resolve_type(ty, imports, *offset).map(|ty| ty.descriptor()))
            .collect::<Result<Vec<_>>>()?
            .concat();
        let result = resolve_type(result, imports, *offset)?;
        members.insert(
            name.clone(),
            JavaMethod {
                owner: owner.clone(),
                name: name.clone(),
                descriptor: format!("({params}){}", result.descriptor()),
                access: if *private { 2 } else { 1 },
                unsupported: false,
                source_property: property.map(str::to_owned),
            },
        );
    }
    let functions = HashMap::new();
    let signatures = Vec::new();
    let extensions = HashSet::new();
    let mut methods = Vec::new();
    let mut names = HashSet::new();
    for (name, private, is_override, source_property, source_params, result, body, offset) in
        declarations
    {
        if inherited_object_method(&name) {
            return Err(Diagnostic::new(
                offset,
                "Any and Object inherited members are not supported",
            ));
        }
        if !names.insert(name.clone()) {
            return Err(Diagnostic::new(
                offset,
                "Member overloads and JVM getter collisions are not supported",
            ));
        }
        let result = resolve_type(result, imports, offset)?;
        if result == Type::Float {
            return Err(Diagnostic::new(
                offset,
                "Float method bodies are not supported",
            ));
        }
        let mut checker = Checker {
            classpath,
            extensions: &extensions,
            classes,
            imports,
            signatures: &signatures,
            receiver: Some(Local {
                slot: 0,
                ty: Type::Java(owner.clone()),
                mutable: false,
            }),
            fields: &fields,
            members: &members,
            companion_fields,
            functions: &functions,
            object_methods: &HashMap::new(),
            object_fields: &HashMap::new(),
            object_owner: None,
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: 1,
            result: result.clone(),
        };
        let mut method_params = Vec::new();
        if source_params.len() > 254 {
            return Err(Diagnostic::new(
                offset,
                "Instance methods support at most 254 parameter slots",
            ));
        }
        for (param, ty) in source_params {
            let ty = resolve_type(&ty, imports, offset)?;
            if matches!(ty, Type::Unit | Type::Float) {
                return Err(Diagnostic::new(
                    offset,
                    "Unit and Float parameters are not supported",
                ));
            }
            checker.declare(&param, ty.clone(), false, offset)?;
            method_params.push((param, ty));
        }
        let (mut body, returns) = checker.block(body)?;
        if !returns {
            if result != Type::Unit {
                return Err(Diagnostic::new(
                    offset,
                    "Function must return on all paths; unconditional-loop analysis is not supported",
                ));
            }
            body.push(Statement::Return(Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            }));
        }
        let method = Function {
            name,
            private,
            is_override,
            params: method_params,
            result,
            locals: checker.next,
            body,
        };
        match required.get(&method.name) {
            Some(required) => {
                if !is_override
                    || private
                    || method.descriptor() != required.descriptor
                    || required.source_property.as_deref() != source_property
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Interface implementation requires override and an exact supported JVM signature",
                    ));
                }
            }
            None if is_override => {
                return Err(Diagnostic::new(
                    offset,
                    "Member overrides no supported interface declaration",
                ));
            }
            None => {}
        }
        methods.push(method);
    }
    let mut bridges = Vec::new();
    let mut required: Vec<_> = required.into_values().collect();
    required.sort_by(|a, b| a.name.cmp(&b.name));
    for method in required {
        if names.contains(&method.name) {
            continue;
        }
        if method.access & 0x0400 != 0 {
            return Err(Diagnostic::new(
                source.offset,
                format!("Missing interface implementation: {}", method.name),
            ));
        }
        if classes[&method.owner].kotlin_interface {
            bridges.push(method);
        }
    }
    if methods.len() + bridges.len() + 1 > 65535 {
        return Err(Diagnostic::new(
            source.offset,
            "Too many methods for a JVM class",
        ));
    }
    Ok(ClassDecl {
        params,
        interfaces,
        methods,
        bridges,
    })
}

pub(crate) fn class_method_types(method: &JavaMethod) -> Result<(Vec<Type>, Type)> {
    fn ty(descriptor: &str, cursor: &mut usize) -> Result<Type> {
        let rest = descriptor
            .get(*cursor..)
            .ok_or_else(|| Diagnostic::new(0, "Invalid interface method descriptor"))?;
        for (prefix, ty) in [
            ("[Ljava/lang/String;", Type::StringArray),
            ("Ljava/lang/String;", Type::String),
            ("[I", Type::IntArray),
            ("[B", Type::ByteArray),
            ("[C", Type::CharArray),
            ("I", Type::Int),
            ("B", Type::Byte),
            ("C", Type::Char),
            ("Z", Type::Boolean),
            ("F", Type::Float),
            ("V", Type::Unit),
        ] {
            if rest.starts_with(prefix) {
                *cursor += prefix.len();
                return Ok(ty);
            }
        }
        Err(Diagnostic::new(
            0,
            "Unsupported interface method descriptor",
        ))
    }
    if !method.descriptor.starts_with('(') {
        return Err(Diagnostic::new(0, "Invalid interface method descriptor"));
    }
    let mut cursor = 1;
    let mut params = Vec::new();
    while method.descriptor.as_bytes().get(cursor) != Some(&b')') {
        let param = ty(&method.descriptor, &mut cursor)?;
        if param == Type::Unit || params.len() >= 254 {
            return Err(Diagnostic::new(
                0,
                "Unsupported interface method parameters",
            ));
        }
        params.push(param);
    }
    cursor += 1;
    let result = ty(&method.descriptor, &mut cursor)?;
    if cursor != method.descriptor.len() {
        return Err(Diagnostic::new(0, "Invalid interface method descriptor"));
    }
    Ok((params, result))
}

fn inherited_object_method(name: &str) -> bool {
    matches!(
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
    )
}
