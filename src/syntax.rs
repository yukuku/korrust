use crate::{Diagnostic, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Long,
    Byte,
    Char,
    Boolean,
    Float,
    IntArray,
    ByteArray,
    CharArray,
    StringArray,
    Array(Box<Type>),
    ReceiverFunction {
        receiver: Box<Type>,
        params: Vec<Type>,
        result: Box<Type>,
    },
    String,
    Unit,
    Java(String),
    Parameterized {
        name: String,
        arguments: Vec<TypeArgument>,
    },
    Parameter {
        name: String,
        upper_bound: Box<Type>,
    },
    Function {
        params: Vec<Type>,
        result: Box<Type>,
    },
    ThreadLocal(Box<Type>),
    Platform(Box<Type>),
    Nullable(Box<Type>),
    Null,
    List(Box<Type>),
    MutableList(Box<Type>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variance {
    Invariant,
    In,
    Out,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeArgument {
    pub variance: Variance,
    pub ty: Option<Type>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeParameter {
    pub name: String,
    pub variance: Variance,
    pub bounds: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct LambdaParameter {
    pub name: String,
    pub ty: Option<Type>,
}

impl Type {
    pub fn base(&self) -> &Type {
        match self {
            Self::Platform(t) => t.base(),
            _ => self,
        }
    }
    pub fn is_reference(&self) -> bool {
        if matches!(self.base(), Self::Nullable(_)) {
            return true;
        }
        matches!(
            self.base(),
            Self::IntArray
                | Self::ByteArray
                | Self::CharArray
                | Self::StringArray
                | Self::Array(_)
                | Self::ReceiverFunction { .. }
                | Self::String
                | Self::Java(_)
                | Self::Parameterized { .. }
                | Self::Parameter { .. }
                | Self::Function { .. }
                | Self::ThreadLocal(_)
                | Self::Null
                | Self::List(_)
                | Self::MutableList(_)
        )
    }
    pub fn is_nullable(&self) -> bool {
        matches!(self.base(), Self::Nullable(_) | Self::Null)
    }
    pub fn slots(&self) -> usize {
        if self.base() == &Self::Long {
            2
        } else if self.base() == &Self::Unit {
            0
        } else {
            1
        }
    }
    pub fn descriptor(&self) -> String {
        match self.base() {
            Self::Int => "I".into(),
            Self::Long => "J".into(),
            Self::Byte => "B".into(),
            Self::Char => "C".into(),
            Self::Boolean => "Z".into(),
            Self::Float => "F".into(),
            Self::IntArray => "[I".into(),
            Self::ByteArray => "[B".into(),
            Self::CharArray => "[C".into(),
            Self::StringArray => "[Ljava/lang/String;".into(),
            Self::Array(element) => {
                if **element == Self::Unit {
                    return "[Lkotlin/Unit;".into();
                }
                let boxed = if element.is_reference() {
                    element.as_ref().clone()
                } else {
                    Self::Nullable(element.clone())
                };
                format!("[{}", boxed.descriptor())
            }
            Self::ReceiverFunction { params, .. } => {
                format!("Lkotlin/jvm/functions/Function{};", params.len() + 1)
            }
            Self::String => "Ljava/lang/String;".into(),
            Self::Unit => "V".into(),
            Self::Java(name) | Self::Parameterized { name, .. } => format!("L{name};"),
            Self::Parameter { upper_bound, .. } => {
                if upper_bound.is_reference() {
                    upper_bound.descriptor()
                } else {
                    Self::Nullable(upper_bound.clone()).descriptor()
                }
            }
            Self::Function { params, .. } => {
                format!("Lkotlin/jvm/functions/Function{};", params.len())
            }
            Self::ThreadLocal(_) => "Ljava/lang/ThreadLocal;".into(),
            Self::List(_) | Self::MutableList(_) => "Ljava/util/List;".into(),
            Self::Null => "Ljava/lang/Object;".into(),
            Self::Nullable(inner) => match inner.base() {
                Self::Int => "Ljava/lang/Integer;".into(),
                Self::Long => "Ljava/lang/Long;".into(),
                Self::Byte => "Ljava/lang/Byte;".into(),
                Self::Char => "Ljava/lang/Character;".into(),
                Self::Boolean => "Ljava/lang/Boolean;".into(),
                Self::Float => "Ljava/lang/Float;".into(),
                _ => inner.descriptor(),
            },
            Self::Platform(_) => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub package: String,
    pub imports: Vec<(String, String, usize)>,
    pub functions: Vec<Function>,
    pub constants: Vec<ConstDecl>,
    pub properties: Vec<PropertyDecl>,
    pub enum_decl: Option<EnumDecl>,
    pub object_decl: Option<ObjectDecl>,
    pub class_decl: Option<ClassDecl>,
    pub interface_decl: Option<InterfaceDecl>,
    pub declarations: Vec<Declaration>,
}
#[derive(Clone, Debug)]
pub enum Declaration {
    Class(ClassDecl),
    Object(ObjectDecl),
    Interface(InterfaceDecl),
    Enum(EnumDecl),
}
#[derive(Clone, Debug)]
pub struct SecondaryConstructor {
    pub offset: usize,
    pub params: Vec<(String, Type)>,
    pub defaults: Vec<Option<Expr>>,
    pub delegation: String,
    pub arguments: Vec<Expr>,
    pub body: Vec<Statement>,
}
#[derive(Clone, Debug)]
pub struct CatchClause {
    pub name: String,
    pub ty: Type,
    pub body: Vec<Statement>,
}
#[derive(Clone, Debug)]
pub struct InterfaceDecl {
    pub name: String,
    pub offset: usize,
    pub internal: bool,
    pub interfaces: Vec<String>,
    pub properties: Vec<InterfaceProperty>,
    pub methods: Vec<InterfaceMethod>,
    pub nested: Vec<InterfaceDecl>,
    pub declarations: Vec<Declaration>,
    pub companion: Option<CompanionDecl>,
}
#[derive(Clone, Debug)]
pub struct InterfaceProperty {
    pub name: String,
    pub offset: usize,
    pub ty: Type,
    pub body: Option<Vec<Statement>>,
    pub is_override: bool,
    pub mutable: bool,
}
#[derive(Clone, Debug)]
pub struct InterfaceMethod {
    pub function: Function,
    pub has_body: bool,
    pub is_override: bool,
}
#[derive(Clone, Debug)]
pub struct CompanionDecl {
    pub properties: Vec<CompanionProperty>,
    pub fields: Vec<ClassCompanionProperty>,
    pub methods: Vec<ClassMethod>,
    pub initializers: Vec<(usize, Vec<Statement>)>,
}
#[derive(Clone, Debug)]
pub struct CompanionProperty {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub value: CompanionValue,
}
#[derive(Clone, Debug)]
pub enum CompanionValue {
    StringArray(Vec<Vec<u16>>),
    Object(Box<ClassDecl>),
}
#[derive(Clone, Debug)]
pub struct EnumDecl {
    pub name: String,
    pub offset: usize,
    pub entries: Vec<(String, usize)>,
}
#[derive(Clone, Debug)]
pub struct ObjectDecl {
    pub name: String,
    pub offset: usize,
    pub internal: bool,
    pub annotations: Vec<String>,
    pub interfaces: Vec<String>,
    pub constants: Vec<ConstDecl>,
    pub objects: Vec<ObjectDecl>,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
    pub initializers: Vec<(usize, Vec<Statement>)>,
    pub nested: Vec<Declaration>,
}
#[derive(Clone, Debug)]
pub struct ObjectField {
    pub is_override: bool,
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub initializer: Option<Expr>,
    pub ty: Option<Type>,
    pub mutable: bool,
    pub body: Vec<Statement>,
    pub setter: Option<(String, Vec<Statement>)>,
    pub delegate: Option<Expr>,
    pub lateinit: bool,
}
#[derive(Clone, Debug)]
pub struct ObjectMethod {
    pub is_override: bool,
    pub function: Function,
    pub jvm_static: bool,
    pub throws: Vec<String>,
    pub inline: bool,
    pub suppress: Vec<String>,
}
#[derive(Clone, Debug)]
pub struct ConstDecl {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub internal: bool,
    pub value: ConstValue,
}
#[derive(Clone, Debug)]
pub enum ConstValue {
    Int(i32),
    Long(i64),
    Float(u32),
    Char(u16),
    Boolean(bool),
    String(Vec<u16>),
}
#[derive(Clone, Debug)]
pub struct ClassDecl {
    pub primary_constructor: bool,
    pub name: String,
    pub offset: usize,
    pub internal: bool,
    pub params: Vec<(String, Type)>,
    pub public_params: Vec<String>,
    pub param_properties: Vec<String>,
    pub override_params: Vec<String>,
    pub mutable_params: Vec<String>,
    pub defaults: Vec<Option<Expr>>,
    pub data: bool,
    pub interfaces: Vec<String>,
    pub properties: Vec<PropertyDecl>,
    pub methods: Vec<ClassMethod>,
    pub companion: Option<ClassCompanion>,
    pub initializers: Vec<(usize, Vec<Statement>)>,
    pub nested: Vec<Declaration>,
    pub secondary_constructors: Vec<SecondaryConstructor>,
    pub super_class: Option<(String, Vec<Expr>)>,
    pub open: bool,
    pub abstract_: bool,
}
#[derive(Clone, Debug)]
pub struct ClassCompanion {
    pub properties: Vec<ClassCompanionProperty>,
    pub methods: Vec<ClassMethod>,
    pub initializers: Vec<(usize, Vec<Statement>)>,
}
#[derive(Clone, Debug)]
pub struct ClassCompanionProperty {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub mutable: bool,
    pub ty: Option<Type>,
    pub initializer: Option<Expr>,
    pub body: Vec<Statement>,
    pub setter: Option<(String, Vec<Statement>)>,
    pub delegate: Option<Expr>,
    pub lateinit: bool,
}
#[derive(Clone, Debug)]
pub struct PropertyDecl {
    pub name: String,
    pub private: bool,
    pub protected: bool,
    pub mutable: bool,
    pub inferred: bool,
    pub initializer: Option<Expr>,
    pub offset: usize,
    pub ty: Type,
    pub body: Vec<Statement>,
    pub is_override: bool,
    pub setter: Option<(String, Vec<Statement>)>,
    pub delegate: Option<Expr>,
    pub lateinit: bool,
    pub open: bool,
}
#[derive(Clone, Debug)]
pub struct ClassMethod {
    pub function: Function,
    pub is_override: bool,
    pub open: bool,
    pub has_body: bool,
}
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub protected: bool,
    pub internal: bool,
    pub infix: bool,
    pub operator: bool,
    pub receiver: Option<Type>,
    pub type_params: Vec<TypeParameter>,
    pub params: Vec<(String, Type)>,
    pub defaults: Vec<Option<Expr>>,
    pub result: Type,
    pub result_inferred: bool,
    pub body: Vec<Statement>,
}
#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub offset: usize,
}
#[derive(Clone, Debug)]
pub enum StatementKind {
    Block(Vec<Statement>),
    LocalFunction(Box<Function>),
    Labeled(String, Box<Statement>),
    BreakLabel(String),
    ContinueLabel(String),
    Destructure(Vec<Option<String>>, bool, Expr),
    Let(String, bool, Option<Type>, Expr),
    Declare(String, bool, Type),
    IndexAssign(Expr, Expr, Expr),
    IndicesAssign(Expr, Vec<Expr>, Expr),
    PropertyAssign(Expr, String, Expr),
    Continue,
    Break,
    ForUntil(String, Expr, Expr, Vec<Statement>),
    ForEach(String, Expr, Vec<Statement>),
    Assign(String, Expr),
    AddAssign(String, Expr),
    CompoundAssign(String, String, Expr),
    Return(Option<Expr>),
    Throw(Expr),
    Expression(Expr),
    If(Expr, Vec<Statement>, Vec<Statement>),
    While(Expr, Vec<Statement>),
    DoWhile(Expr, Vec<Statement>),
    WhenBinding(
        String,
        Expr,
        Vec<(Vec<Expr>, Vec<Statement>)>,
        Option<Vec<Statement>>,
    ),
    When(
        Expr,
        Vec<(Vec<Expr>, Vec<Statement>)>,
        Option<Vec<Statement>>,
    ),
}
#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub offset: usize,
}
#[derive(Clone, Debug)]
pub enum ExprKind {
    Block(Vec<Statement>),
    Try {
        body: Vec<Statement>,
        catches: Vec<CatchClause>,
        finally: Vec<Statement>,
    },
    Labeled(String, Box<Expr>),
    LabeledReturn(String, Option<Box<Expr>>),
    WhenBinding(String, Box<Expr>, Vec<(Vec<Expr>, Expr)>, Box<Expr>),
    Indices(Box<Expr>, Vec<Expr>),
    Int(i64),
    Float(u32),
    NamedArgument(String, Box<Expr>),
    Generic(Box<Expr>, Vec<Type>),
    Return(Option<Box<Expr>>),
    Throw(Box<Expr>),
    Long(i128),
    Null,
    Char(u16),
    Boolean(bool),
    String(Vec<u16>),
    Template(Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Name(String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Property(Box<Expr>, String),
    SafeProperty(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Increment {
        value: Box<Expr>,
        prefix: bool,
    },
    Decrement {
        value: Box<Expr>,
        prefix: bool,
    },
    TypeOperation(String, Box<Expr>, Type),
    NonNull(Box<Expr>),
    Lambda(Box<Expr>),
    LambdaWithParams {
        params: Option<Vec<LambdaParameter>>,
        body: Vec<Statement>,
    },
    CallableReference {
        receiver: Option<Box<Expr>>,
        name: String,
    },
    When(Box<Expr>, Vec<(Vec<Expr>, Expr)>, Box<Expr>),
}
fn shift_expression_offsets(expression: &mut Expr, offset: usize) {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        expression.offset += offset;
        match &mut expression.kind {
            ExprKind::Block(body) => shift_statement_offsets(body, offset),
            ExprKind::Try {
                body,
                catches,
                finally,
            } => {
                shift_statement_offsets(body, offset);
                for catch in catches {
                    shift_statement_offsets(&mut catch.body, offset);
                }
                shift_statement_offsets(finally, offset);
            }
            ExprKind::Labeled(_, value) | ExprKind::LabeledReturn(_, Some(value)) => {
                pending.push(value)
            }
            ExprKind::Indices(target, indices) => {
                pending.push(target);
                pending.extend(indices);
            }
            ExprKind::WhenBinding(_, subject, branches, otherwise) => {
                pending.push(subject);
                pending.push(otherwise);
                for (conditions, branch) in branches {
                    pending.extend(conditions);
                    pending.push(branch);
                }
            }
            ExprKind::CallableReference {
                receiver: Some(value),
                ..
            } => pending.push(value),
            ExprKind::LambdaWithParams { body, .. } => shift_statement_offsets(body, offset),
            ExprKind::Template(parts) => pending.extend(parts),
            ExprKind::If(condition, yes, no) => {
                pending.push(condition);
                pending.push(yes);
                pending.push(no);
            }
            ExprKind::When(subject, branches, otherwise) => {
                pending.push(subject);
                pending.push(otherwise);
                for (conditions, branch) in branches {
                    pending.extend(conditions);
                    pending.push(branch);
                }
            }
            ExprKind::Return(Some(value))
            | ExprKind::Throw(value)
            | ExprKind::NamedArgument(_, value)
            | ExprKind::Generic(value, _)
            | ExprKind::Unary(_, value)
            | ExprKind::Property(value, _)
            | ExprKind::SafeProperty(value, _)
            | ExprKind::NonNull(value)
            | ExprKind::Lambda(value)
            | ExprKind::Increment { value, .. }
            | ExprKind::Decrement { value, .. }
            | ExprKind::TypeOperation(_, value, _) => pending.push(value),
            ExprKind::Binary(_, left, right) | ExprKind::Index(left, right) => {
                pending.push(left);
                pending.push(right);
            }
            ExprKind::Call(target, args) => {
                pending.push(target);
                pending.extend(args);
            }
            _ => {}
        }
    }
}
fn shift_statement_offsets(statements: &mut [Statement], offset: usize) {
    for statement in statements {
        statement.offset += offset;
        match &mut statement.kind {
            StatementKind::Block(body) => shift_statement_offsets(body, offset),
            StatementKind::LocalFunction(function) => {
                function.offset += offset;
                for value in function.defaults.iter_mut().flatten() {
                    shift_expression_offsets(value, offset);
                }
                shift_statement_offsets(&mut function.body, offset);
            }
            StatementKind::Labeled(_, statement) => {
                shift_statement_offsets(std::slice::from_mut(statement), offset)
            }
            StatementKind::Destructure(_, _, value) => shift_expression_offsets(value, offset),
            StatementKind::IndicesAssign(target, indices, value) => {
                shift_expression_offsets(target, offset);
                for index in indices {
                    shift_expression_offsets(index, offset);
                }
                shift_expression_offsets(value, offset);
            }
            StatementKind::Let(_, _, _, value)
            | StatementKind::Assign(_, value)
            | StatementKind::AddAssign(_, value)
            | StatementKind::CompoundAssign(_, _, value)
            | StatementKind::Return(Some(value))
            | StatementKind::Throw(value)
            | StatementKind::Expression(value) => shift_expression_offsets(value, offset),
            StatementKind::IndexAssign(target, index, value) => {
                shift_expression_offsets(target, offset);
                shift_expression_offsets(index, offset);
                shift_expression_offsets(value, offset);
            }
            StatementKind::PropertyAssign(target, _, value) => {
                shift_expression_offsets(target, offset);
                shift_expression_offsets(value, offset);
            }
            StatementKind::ForUntil(_, from, to, body) => {
                shift_expression_offsets(from, offset);
                shift_expression_offsets(to, offset);
                shift_statement_offsets(body, offset);
            }
            StatementKind::If(condition, yes, no) => {
                shift_expression_offsets(condition, offset);
                shift_statement_offsets(yes, offset);
                shift_statement_offsets(no, offset);
            }
            StatementKind::While(condition, body)
            | StatementKind::DoWhile(condition, body)
            | StatementKind::ForEach(_, condition, body) => {
                shift_expression_offsets(condition, offset);
                shift_statement_offsets(body, offset);
            }
            StatementKind::When(subject, branches, otherwise)
            | StatementKind::WhenBinding(_, subject, branches, otherwise) => {
                shift_expression_offsets(subject, offset);
                for (conditions, body) in branches {
                    for condition in conditions {
                        shift_expression_offsets(condition, offset);
                    }
                    shift_statement_offsets(body, offset);
                }
                if let Some(body) = otherwise {
                    shift_statement_offsets(body, offset);
                }
            }
            _ => {}
        }
    }
}
#[derive(Debug, Clone)]
struct Token {
    text: String,
    offset: usize,
}

fn quoted_end(source: &str, start: usize, depth: usize) -> Result<usize> {
    if depth > 64 {
        return Err(Diagnostic::new(
            start,
            "String interpolation nesting exceeds the current limit",
        ));
    }
    let bytes = source.as_bytes();
    if bytes[start..].starts_with(b"\"\"\"") {
        return Err(Diagnostic::new(
            start,
            "Raw triple-quoted strings are not supported",
        ));
    }
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok(i + 1),
            b'\\' => i += 2,
            b'\n' | b'\r' => {
                return Err(Diagnostic::new(
                    i,
                    "Newlines are not allowed in ordinary strings",
                ));
            }
            b'$' if bytes.get(i + 1) == Some(&b'{') => {
                i = template_end(source, i + 2, depth + 1)? + 1
            }
            _ => i += 1,
        }
    }
    Err(Diagnostic::new(start, "Unterminated string literal"))
}
fn template_end(source: &str, start: usize, depth: usize) -> Result<usize> {
    if depth > 64 {
        return Err(Diagnostic::new(
            start,
            "String interpolation nesting exceeds the current limit",
        ));
    }
    let bytes = source.as_bytes();
    let mut i = start;
    let mut braces = 1;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => i = quoted_end(source, i, depth + 1)?,
            b'\'' => i = char_literal(source, i)?.0,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let mut comments = 1;
                i += 2;
                while i < bytes.len() && comments > 0 {
                    if bytes[i..].starts_with(b"/*") {
                        comments += 1;
                        i += 2;
                    } else if bytes[i..].starts_with(b"*/") {
                        comments -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if comments > 0 {
                    return Err(Diagnostic::new(start, "Unterminated block comment"));
                }
            }
            b'{' => {
                braces += 1;
                if braces > 128 {
                    return Err(Diagnostic::new(
                        i,
                        "String interpolation nesting exceeds the current limit",
                    ));
                }
                i += 1;
            }
            b'}' => {
                braces -= 1;
                if braces == 0 {
                    return Ok(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    Err(Diagnostic::new(
        start,
        "Unterminated braced string interpolation",
    ))
}

fn string_parts(source: &str, start: usize) -> Result<(usize, Vec<Expr>)> {
    string_parts_depth(source, start, 0)
}

fn string_parts_depth(source: &str, start: usize, depth: usize) -> Result<(usize, Vec<Expr>)> {
    if depth > 64 {
        return Err(Diagnostic::new(
            start,
            "String interpolation nesting exceeds the current limit",
        ));
    }
    let bytes = source.as_bytes();
    if bytes[start..].starts_with(b"\"\"\"") {
        return Err(Diagnostic::new(
            start,
            "Raw triple-quoted strings are not supported",
        ));
    }
    let mut i = start + 1;
    let mut units = Vec::new();
    let mut parts = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                if !units.is_empty() || parts.is_empty() {
                    parts.push(Expr {
                        offset: start,
                        kind: ExprKind::String(units),
                    });
                }
                return Ok((i + 1, parts));
            }
            b'\n' | b'\r' => {
                return Err(Diagnostic::new(
                    i,
                    "Newlines are not allowed in ordinary strings",
                ));
            }
            b'\\' => {
                let escape = i;
                i += 1;
                let b = *bytes
                    .get(i)
                    .ok_or_else(|| Diagnostic::new(start, "Unterminated string literal"))?;
                let unit = match b {
                    b't' => 9,
                    b'b' => 8,
                    b'n' => 10,
                    b'r' => 13,
                    b'\'' => 39,
                    b'"' => 34,
                    b'\\' => 92,
                    b'$' => 36,
                    b'u' => {
                        let digits = bytes.get(i + 1..i + 5).ok_or_else(|| {
                            Diagnostic::new(escape, "Expected four hexadecimal digits after \\u")
                        })?;
                        let mut value = 0u16;
                        for digit in digits {
                            let n = (*digit as char).to_digit(16).ok_or_else(|| {
                                Diagnostic::new(
                                    escape,
                                    "Expected four hexadecimal digits after \\u",
                                )
                            })?;
                            value = value * 16 + n as u16;
                        }
                        i += 4;
                        value
                    }
                    _ => return Err(Diagnostic::new(escape, "Illegal string escape")),
                };
                units.push(unit);
                i += 1;
            }
            b'$' if bytes.get(i + 1) == Some(&b'{') => {
                parts.push(Expr {
                    offset: start,
                    kind: ExprKind::String(std::mem::take(&mut units)),
                });
                let expression_start = i + 2;
                i = template_end(source, expression_start, depth + 1)?;
                let mut tokens = lex(&source[expression_start..i])?;
                for token in &mut tokens {
                    token.offset += expression_start;
                }
                let mut parser = Parser {
                    tokens,
                    pos: 0,
                    depth: depth + 1,
                    type_parameters: Vec::new(),
                };
                parser.lines();
                let expression = parser.expr(0)?;
                parser.lines();
                if !parser.at("<eof>") {
                    return Err(
                        parser.error("Braced string templates require one supported expression")
                    );
                }
                parts.push(expression);
                if parts.len() > 256 {
                    return Err(Diagnostic::new(
                        start,
                        "String interpolation exceeds the current limit",
                    ));
                }
                i += 1;
            }
            b'$' if bytes
                .get(i + 1)
                .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_') =>
            {
                parts.push(Expr {
                    offset: start,
                    kind: ExprKind::String(std::mem::take(&mut units)),
                });
                let name_start = i + 1;
                i = name_start + 1;
                while bytes
                    .get(i)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    i += 1;
                }
                if source[i..]
                    .chars()
                    .next()
                    .is_some_and(|character| !character.is_ascii())
                {
                    return Err(Diagnostic::new(
                        i,
                        "Non-ASCII characters immediately after template names are not supported",
                    ));
                }
                parts.push(Expr {
                    offset: name_start,
                    kind: ExprKind::Name(source[name_start..i].into()),
                });
                if parts.len() > 256 {
                    return Err(Diagnostic::new(
                        start,
                        "String interpolation exceeds the current 128-name limit",
                    ));
                }
            }
            b'$' if source[i + 1..]
                .chars()
                .next()
                .is_some_and(|c| c == '{' || c == '`' || c.is_alphabetic()) =>
            {
                return Err(Diagnostic::new(
                    i,
                    "Only simple ASCII name string interpolation is supported",
                ));
            }
            _ => {
                let c = source[i..].chars().next().unwrap();
                units.extend(c.encode_utf16(&mut [0; 2]).iter().copied());
                i += c.len_utf8();
            }
        }
    }
    Err(Diagnostic::new(start, "Unterminated string literal"))
}

fn string_literal(source: &str, start: usize) -> Result<(usize, Vec<u16>)> {
    let (end, mut parts) = string_parts(source, start)?;
    if parts.len() == 1 {
        if let ExprKind::String(units) = parts.remove(0).kind {
            return Ok((end, units));
        }
    }
    Err(Diagnostic::new(
        start,
        "String interpolation is not supported in literal initializers",
    ))
}

fn char_literal(source: &str, start: usize) -> Result<(usize, u16)> {
    let bytes = source.as_bytes();
    let mut i = start + 1;
    let mut units = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                if units.len() != 1 {
                    return Err(Diagnostic::new(
                        start,
                        "Char literal must contain exactly one UTF-16 code unit",
                    ));
                }
                return Ok((i + 1, units[0]));
            }
            b'\n' | b'\r' => {
                return Err(Diagnostic::new(
                    i,
                    "Newlines are not allowed in Char literals",
                ));
            }
            b'\\' => {
                let escape = i;
                i += 1;
                let b = *bytes
                    .get(i)
                    .ok_or_else(|| Diagnostic::new(start, "Unterminated Char literal"))?;
                let unit = match b {
                    b't' => 9,
                    b'b' => 8,
                    b'n' => 10,
                    b'r' => 13,
                    b'\'' => 39,
                    b'"' => 34,
                    b'\\' => 92,
                    b'$' => 36,
                    b'u' => {
                        let digits = bytes.get(i + 1..i + 5).ok_or_else(|| {
                            Diagnostic::new(escape, "Expected four hexadecimal digits after \\u")
                        })?;
                        let mut value = 0u16;
                        for digit in digits {
                            let n = (*digit as char).to_digit(16).ok_or_else(|| {
                                Diagnostic::new(
                                    escape,
                                    "Expected four hexadecimal digits after \\u",
                                )
                            })?;
                            value = value * 16 + n as u16;
                        }
                        i += 4;
                        value
                    }
                    _ => return Err(Diagnostic::new(escape, "Illegal Char escape")),
                };
                units.push(unit);
                i += 1;
            }
            _ => {
                let c = source[i..].chars().next().unwrap();
                units.extend(c.encode_utf16(&mut [0; 2]).iter().copied());
                i += c.len_utf8();
            }
        }
    }
    Err(Diagnostic::new(start, "Unterminated Char literal"))
}

fn lex(source: &str) -> Result<Vec<Token>> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let b = bytes[i];
        if b == b'\n' {
            tokens.push(Token {
                text: "\n".into(),
                offset: i,
            });
            i += 1;
        } else if b.is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            let mut newline = false;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    newline |= bytes[i] == b'\n';
                    i += 1;
                }
            }
            if depth != 0 {
                return Err(Diagnostic::new(start, "Unterminated block comment"));
            }
            if newline {
                tokens.push(Token {
                    text: "\n".into(),
                    offset: start,
                });
            }
        } else if b == b'"' {
            i = quoted_end(source, start, 0)?;
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b == b'\'' {
            i = char_literal(source, start)?.0;
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b.is_ascii_digit() {
            i += 1;
            let radix = b == b'0' && matches!(bytes.get(i), Some(b'x' | b'X' | b'b' | b'B'));
            while i < bytes.len() {
                let current = bytes[i];
                if current.is_ascii_alphanumeric() || current == b'_' {
                    i += 1;
                    if !radix
                        && matches!(current, b'e' | b'E')
                        && matches!(bytes.get(i), Some(b'+' | b'-'))
                    {
                        i += 1;
                    }
                } else if !radix
                    && current == b'.'
                    && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
                {
                    i += 1;
                } else {
                    break;
                }
            }
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b.is_ascii_alphanumeric() || b == b'_' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b"(){}[]:.,;=+-*/%<>!@?".contains(&b) || b == b'&' || b == b'|' {
            i += 1;
            if i < bytes.len()
                && matches!(
                    &bytes[start..=i],
                    b"=="
                        | b"!="
                        | b"<="
                        | b">="
                        | b"&&"
                        | b"||"
                        | b"+="
                        | b"-="
                        | b"*="
                        | b"/="
                        | b"%="
                        | b"->"
                        | b"++"
                        | b"--"
                        | b"!!"
                        | b".."
                        | b"::"
                        | b"?."
                        | b"?:"
                )
            {
                i += 1;
            }
            if matches!(&bytes[start..i], b"==" | b"!=") && bytes.get(i) == Some(&b'=') {
                i += 1;
            }
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else {
            return Err(Diagnostic::new(
                start,
                "Unsupported token in this Kotlin subset",
            ));
        }
    }
    tokens.push(Token {
        text: "<eof>".into(),
        offset: source.len(),
    });
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    type_parameters: Vec<TypeParameter>,
}
impl Parser {
    fn token(&self) -> &Token {
        &self.tokens[self.pos]
    }
    fn at(&self, s: &str) -> bool {
        self.token().text == s
    }
    fn take(&mut self, s: &str) -> bool {
        if self.at(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.token().offset, message)
    }
    fn expect(&mut self, s: &str) -> Result<()> {
        if self.take(s) {
            Ok(())
        } else {
            Err(self.error(format!("Expected {s}, found {}", self.token().text)))
        }
    }
    fn lines(&mut self) {
        while self.take("\n") {}
    }
    fn separators(&mut self) {
        while self.take("\n") || self.take(";") {}
    }
    fn end(&mut self) -> Result<()> {
        if self.at("}") || self.at("<eof>") {
            return Ok(());
        }
        if !self.at("\n") && !self.at(";") {
            return Err(self.error("Expected a newline or semicolon"));
        }
        self.separators();
        Ok(())
    }
    fn name(&mut self) -> Result<String> {
        let text = self.token().text.clone();
        if text.bytes().all(|b| b == b'_') {
            return Err(self.error("Names consisting only of underscores are reserved"));
        }
        if !identifier(&text) {
            return Err(self.error("Expected an identifier"));
        }
        self.pos += 1;
        Ok(text)
    }
    fn plain_string(&self) -> Result<Vec<u16>> {
        string_literal(&self.token().text, 0)
            .map(|(_, units)| units)
            .map_err(|mut error| {
                error.offset += self.token().offset;
                error
            })
    }
    fn ty(&mut self) -> Result<Type> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.error("Type nesting exceeds the current limit"));
        }
        let ty = self.nonnullable_ty()?;
        let ty = if self.take("?") {
            if ty == Type::Unit {
                return Err(self.error("Nullable Unit is not supported"));
            }
            Type::Nullable(Box::new(ty))
        } else {
            ty
        };
        self.depth -= 1;
        Ok(ty)
    }
    fn nonnullable_ty(&mut self) -> Result<Type> {
        if self.take("(") {
            self.lines();
            let mut params = Vec::new();
            if !self.at(")") {
                loop {
                    if self
                        .tokens
                        .get(self.pos + 1)
                        .is_some_and(|token| token.text == ":")
                    {
                        self.name()?;
                        self.expect(":")?;
                        self.lines();
                    }
                    params.push(self.ty()?);
                    self.lines();
                    if !self.take(",") {
                        break;
                    }
                    self.lines();
                    if self.at(")") {
                        break;
                    }
                }
            }
            self.expect(")")?;
            if self.take("->") {
                self.lines();
                if params.len() > 22 {
                    return Err(
                        self.error("Function types with more than 22 parameters are unsupported")
                    );
                }
                return Ok(Type::Function {
                    params,
                    result: Box::new(self.ty()?),
                });
            }
            return if params.len() == 1 {
                Ok(params.remove(0))
            } else {
                Err(self.error("Parenthesized types require one type or a function arrow"))
            };
        }
        if self.at("suspend") {
            return Err(self.error("Suspend function types are unsupported"));
        }
        let mut name = self.name()?;
        while self.at(".")
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.text != "(")
        {
            self.pos += 1;
            name.push('/');
            name.push_str(&self.name()?);
        }
        let receiver = self.named_ty(name)?;
        let position = self.pos;
        let receiver = if self.take("?") {
            Type::Nullable(Box::new(receiver))
        } else {
            receiver
        };
        if self.take(".") {
            let function = self.nonnullable_ty()?;
            if let Type::Function { params, result } = function {
                if params.len() >= 22 {
                    return Err(
                        self.error("Receiver function types support at most 21 value parameters")
                    );
                }
                return Ok(Type::ReceiverFunction {
                    receiver: Box::new(receiver),
                    params,
                    result,
                });
            }
            return Err(self.error("A receiver type must be followed by a function type"));
        }
        self.pos = position;
        Ok(match receiver {
            Type::Nullable(inner) => *inner,
            other => other,
        })
    }
    fn type_arguments(&mut self) -> Result<Vec<TypeArgument>> {
        self.expect("<")?;
        self.lines();
        let mut arguments = Vec::new();
        loop {
            let variance = if self.take("in") {
                Variance::In
            } else if self.take("out") {
                Variance::Out
            } else {
                Variance::Invariant
            };
            self.lines();
            let ty = if self.take("*") {
                if variance != Variance::Invariant {
                    return Err(self.error("Star projections cannot have variance modifiers"));
                }
                None
            } else {
                Some(self.ty()?)
            };
            arguments.push(TypeArgument { variance, ty });
            self.lines();
            if !self.take(",") {
                break;
            }
            self.lines();
            if self.at(">") {
                break;
            }
        }
        self.expect(">")?;
        Ok(arguments)
    }
    fn named_ty(&mut self, name: String) -> Result<Type> {
        if self.at("<") {
            let arguments = self.type_arguments()?;
            let covariant = matches!(
                name.as_str(),
                "List"
                    | "Set"
                    | "Collection"
                    | "Iterable"
                    | "kotlin/collections/List"
                    | "kotlin/collections/Set"
                    | "kotlin/collections/Collection"
                    | "kotlin/collections/Iterable"
            );
            let conflict = covariant
                && arguments
                    .iter()
                    .any(|argument| argument.variance == Variance::In)
                || matches!(name.as_str(), "Map" | "kotlin/collections/Map")
                    && arguments
                        .get(1)
                        .is_some_and(|argument| argument.variance == Variance::In)
                || matches!(name.as_str(), "Comparable" | "kotlin/Comparable")
                    && arguments.iter().any(|argument| {
                        argument.ty.is_some() && argument.variance == Variance::Out
                    });
            if conflict {
                return Err(self.error("Type projection conflicts with declaration-site variance"));
            }
            if arguments.len() == 1 && arguments[0].variance == Variance::Invariant {
                if let Some(element) = &arguments[0].ty {
                    match name.as_str() {
                        "List" | "MutableList" if *element != Type::Unit => {
                            return Ok(if name == "List" {
                                Type::List(Box::new(element.clone()))
                            } else {
                                Type::MutableList(Box::new(element.clone()))
                            });
                        }
                        "Array" if *element == Type::Unit => {
                            return Err(
                                self.error("Array<Unit> element operations are unsupported")
                            );
                        }
                        "Array" if *element == Type::String => return Ok(Type::StringArray),
                        "Array" if *element != Type::Unit => {
                            return Ok(Type::Array(Box::new(element.clone())));
                        }
                        "ThreadLocal" if matches!(element, Type::ByteArray | Type::CharArray) => {
                            return Ok(Type::ThreadLocal(Box::new(element.clone())));
                        }
                        _ => {}
                    }
                }
            }

            return Ok(Type::Parameterized { name, arguments });
        }
        if let Some(parameter) = self
            .type_parameters
            .iter()
            .find(|parameter| parameter.name == name)
        {
            return Ok(Type::Parameter {
                name,
                upper_bound: Box::new(parameter.bounds.first().cloned().unwrap_or_else(|| {
                    Type::Nullable(Box::new(Type::Java("java/lang/Object".into())))
                })),
            });
        }
        Ok(match name.as_str() {
            "Int" => Type::Int,
            "Long" => Type::Long,
            "Byte" => Type::Byte,
            "Char" => Type::Char,
            "Boolean" => Type::Boolean,
            "Float" => Type::Float,
            "IntArray" => Type::IntArray,
            "ByteArray" => Type::ByteArray,
            "CharArray" => Type::CharArray,
            "String" => Type::String,
            "Unit" => Type::Unit,
            "Any" => Type::Java("java/lang/Object".into()),
            "CharSequence" => Type::Java("java/lang/CharSequence".into()),
            _ => Type::Java(name),
        })
    }
    fn type_parameters(&mut self) -> Result<Vec<TypeParameter>> {
        if !self.take("<") {
            return Ok(Vec::new());
        }
        self.lines();
        let mut parameters = Vec::new();
        loop {
            if matches!(self.token().text.as_str(), "in" | "out" | "reified") {
                return Err(self.error(
                    "Variance and reified modifiers on function type parameters are unsupported",
                ));
            }
            let name = self.name()?;
            if parameters
                .iter()
                .any(|parameter: &TypeParameter| parameter.name == name)
            {
                return Err(self.error("Duplicate type parameter"));
            }
            let mut parameter = TypeParameter {
                name,
                variance: Variance::Invariant,
                bounds: Vec::new(),
            };
            self.type_parameters.push(parameter.clone());
            if self.take(":") {
                self.lines();
                let bound = self.ty()?;
                let primitive_bound = match bound.base() {
                    Type::Nullable(inner) => inner.base(),
                    other => other,
                };
                if matches!(
                    primitive_bound,
                    Type::Int
                        | Type::Long
                        | Type::Byte
                        | Type::Char
                        | Type::Boolean
                        | Type::Float
                        | Type::Unit
                ) || matches!(primitive_bound,Type::Java(name) if matches!(name.as_str(),"java/lang/Integer"|"java/lang/Long"|"java/lang/Byte"|"java/lang/Character"|"java/lang/Boolean"|"java/lang/Float"|"java/lang/Short"|"java/lang/Double"|"kotlin/Int"|"kotlin/Long"|"kotlin/Byte"|"kotlin/Char"|"kotlin/Boolean"|"kotlin/Float"|"kotlin/Short"|"kotlin/Double"|"kotlin/Unit"))
                {
                    return Err(self
                        .error("Primitive and Unit source type parameter bounds are unsupported"));
                }
                parameter.bounds.push(bound);
                *self.type_parameters.last_mut().unwrap() = parameter.clone();
            }
            parameters.push(parameter);
            self.lines();
            if !self.take(",") {
                break;
            }
            self.lines();
            if self.at(">") {
                break;
            }
        }
        self.expect(">")?;
        Ok(parameters)
    }
    fn starts_enum(&self) -> bool {
        self.starts_declaration("enum")
    }
    fn starts_object(&self) -> bool {
        self.at("@") || self.starts_declaration("object")
    }
    fn starts_class(&self) -> bool {
        self.starts_declaration("class")
    }
    fn starts_interface(&self) -> bool {
        self.starts_declaration("interface")
    }
    fn starts_declaration(&self, kind: &str) -> bool {
        self.tokens[self.pos..]
            .iter()
            .find(|token| {
                !matches!(
                    token.text.as_str(),
                    "public"
                        | "internal"
                        | "private"
                        | "protected"
                        | "open"
                        | "final"
                        | "abstract"
                        | "sealed"
                        | "data"
                        | "expect"
                        | "actual"
                        | "external"
                        | "inline"
                        | "infix"
                        | "operator"
                        | "lateinit"
                        | "override"
                        | "value"
                        | "companion"
                        | "\n"
                )
            })
            .is_some_and(|token| token.text == kind)
    }
    fn enum_decl(&mut self) -> Result<EnumDecl> {
        let offset = self.token().offset;
        let mut visibility = None;
        while matches!(self.token().text.as_str(), "public" | "internal") {
            if visibility.is_some() {
                return Err(self.error("Conflicting or repeated enum visibility modifiers"));
            }
            visibility = Some(self.token().text.clone());
            self.pos += 1;
            self.lines();
        }
        if matches!(self.token().text.as_str(), "private" | "protected") {
            return Err(self.error("Unsupported enum visibility; use public or internal"));
        }
        if !self.at("enum") {
            return Err(
                self.error("Unsupported enum modifier; only public and internal are supported")
            );
        }
        self.expect("enum")?;
        self.lines();
        self.expect("class")?;
        self.lines();
        let name = self.class_name()?;
        let after_name = self.pos;
        self.lines();
        if !self.take("{") {
            if matches!(self.token().text.as_str(), "(" | ":" | "<" | "where") {
                return Err(
                    self.error("Enum constructors, supertypes, and generics are not supported")
                );
            }
            self.pos = after_name;
            if self.at("\n") || self.at(";") || self.at("<eof>") {
                return Ok(EnumDecl {
                    name,
                    offset,
                    entries: Vec::new(),
                });
            }
            return Err(self.error("Only bare enum classes with plain entries are supported"));
        }
        self.lines();
        let mut entries = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !self.at("}") && !self.at(";") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated enum body"));
            }
            let entry_offset = self.token().offset;
            let entry = self.declaration_name()?;
            if matches!(entry.as_str(), "name" | "ordinal" | "entries") {
                return Err(Diagnostic::new(
                    entry_offset,
                    "Enum entry conflicts with a generated or inherited member",
                ));
            }
            if !names.insert(entry.clone()) {
                return Err(Diagnostic::new(entry_offset, "Duplicate enum entry"));
            }
            entries.push((entry, entry_offset));
            if entries.len() > 1024 {
                return Err(Diagnostic::new(
                    entry_offset,
                    "Enum exceeds the current 1024-entry limit",
                ));
            }
            self.lines();
            if !self.take(",") {
                if self.at("<eof>") {
                    return Err(self.error("Unterminated enum body"));
                }
                if !matches!(self.token().text.as_str(), "}" | ";" | "(" | "{") {
                    return Err(self
                        .error("Expected a comma, semicolon, or closing brace after enum entry"));
                }
                break;
            }
            self.lines();
        }
        if self.take(";") {
            self.lines();
        }
        if !self.take("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated enum body"));
            }
            return Err(
                self.error("Enum constructors, entry bodies, and members are not supported")
            );
        }
        Ok(EnumDecl {
            name,
            offset,
            entries,
        })
    }
    fn class_name(&mut self) -> Result<String> {
        if matches!(
            self.token().text.as_str(),
            "Int"
                | "Long"
                | "List"
                | "MutableList"
                | "Char"
                | "Byte"
                | "Boolean"
                | "Float"
                | "IntArray"
                | "ByteArray"
                | "ThreadLocal"
                | "CharArray"
                | "Array"
                | "StringArray"
                | "String"
                | "Unit"
        ) {
            return Err(self.error("Declarations shadowing builtin types are not supported"));
        }
        self.declaration_name()
    }
    fn declaration_name(&mut self) -> Result<String> {
        if matches!(self.token().text.as_str(), "catch" | "finally" | "import") {
            return Err(self.error("Expected an identifier"));
        }
        self.name()
    }
    fn object_decl(&mut self, depth: usize, class_count: &mut usize) -> Result<ObjectDecl> {
        let offset = self.token().offset;
        let mut annotations = Vec::new();
        while self.take("@") {
            let mut name = self.declaration_name()?;
            while self.take(".") {
                name.push('/');
                name.push_str(&self.declaration_name()?);
            }
            if self.at("(") || self.at(":") {
                return Err(self
                    .error("Object annotation arguments and use-site targets are not supported"));
            }
            annotations.push(name);
            self.lines();
        }
        if depth > 1 && !annotations.is_empty() {
            return Err(self.error("Annotated nested objects are not supported"));
        }
        if depth > 16 {
            return Err(self.error("Object nesting exceeds the current 16-class depth limit"));
        }
        *class_count += 1;
        if *class_count > 256 {
            return Err(self.error("Objects exceed the current 256-class limit"));
        }
        let mut visibility = None;
        while matches!(self.token().text.as_str(), "public" | "internal") {
            if visibility.is_some() {
                return Err(self.error("Conflicting or repeated object visibility modifiers"));
            }
            visibility = Some(self.token().text.clone());
            self.pos += 1;
            self.lines();
        }
        if matches!(self.token().text.as_str(), "private" | "protected") {
            return Err(self.error("Unsupported object visibility; use public or internal"));
        }
        if !self.take("object") {
            return Err(
                self.error("Unsupported object modifier; only public and internal are supported")
            );
        }
        self.lines();
        let name = self.class_name()?;
        let mut interfaces = Vec::new();
        let after_name = self.pos;
        self.lines();
        if self.take(":") {
            loop {
                self.lines();
                let mut interface = self.declaration_name()?;
                while self.take(".") {
                    interface.push('/');
                    interface.push_str(&self.declaration_name()?);
                }
                interfaces.push(interface);
                if interfaces.len() > 128 {
                    return Err(self.error("Object exceeds the current 128-interface limit"));
                }
                self.lines();
                if !self.take(",") {
                    break;
                }
            }
        }
        let after_header = self.pos;
        if depth > 1 && !interfaces.is_empty() {
            return Err(self.error("Nested object supertypes are not supported"));
        }
        if !self.take("{") {
            if matches!(self.token().text.as_str(), "(" | ":" | "<" | "where") {
                return Err(
                    self.error("Object constructors, supertypes, and generics are not supported")
                );
            }
            self.pos = if interfaces.is_empty() {
                after_name
            } else {
                after_header
            };
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(ObjectDecl {
                    name,
                    offset,
                    internal: visibility.as_deref() == Some("internal"),
                    annotations,
                    interfaces,
                    constants: Vec::new(),
                    objects: Vec::new(),
                    fields: Vec::new(),
                    methods: Vec::new(),
                    initializers: Vec::new(),
                    nested: Vec::new(),
                });
            }
            return Err(self.error("Only plain named singleton objects are supported"));
        }
        self.separators();
        let mut constants = Vec::new();
        let mut objects = Vec::new();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut initializers = Vec::new();
        let mut nested = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated object body"));
            }
            let member_offset = self.token().offset;
            if self.take("init") {
                self.lines();
                initializers.push((member_offset, self.block()?));
                self.end()?;
                continue;
            }
            if self.starts_class() || self.starts_enum() || self.starts_interface() {
                nested.push(self.declaration()?);
                self.end()?;
                continue;
            }
            let member_name = if self.starts_object() && !self.at("@") {
                let object = self.object_decl(depth + 1, class_count)?;
                let name = object.name.clone();
                objects.push(object);
                name
            } else if self.at("@") || self.starts_declaration("fun") {
                methods.push(self.object_method()?);
                if methods.len() > 1024 {
                    return Err(self.error("Objects exceed the current 1024-method limit"));
                }
                self.end()?;
                continue;
            } else if self.starts_declaration("val")
                || self.starts_declaration("var")
                || self.at("lateinit")
            {
                let is_override = self.take("override");
                let private = self.take("private");
                if !private {
                    self.take("public");
                    self.take("internal");
                }
                self.lines();
                let lateinit = self.take("lateinit");
                self.lines();
                let property = self.property(member_offset, private, false, lateinit, false)?;
                let name = property.name.clone();
                fields.push(ObjectField {
                    is_override,
                    name: property.name,
                    offset: property.offset,
                    private: property.private,
                    initializer: property.initializer,
                    ty: if property.inferred {
                        None
                    } else {
                        Some(property.ty)
                    },
                    mutable: property.mutable,
                    body: property.body,
                    setter: property.setter,
                    delegate: property.delegate,
                    lateinit: property.lateinit,
                });
                name
            } else {
                let constant = self.constant()?;
                let name = constant.name.clone();
                constants.push(constant);
                if constants.len() > 1024 {
                    return Err(Diagnostic::new(
                        member_offset,
                        "Object exceeds the current 1024-constant limit",
                    ));
                }
                name
            };
            if member_name == "INSTANCE" {
                return Err(Diagnostic::new(
                    member_offset,
                    "Object member name INSTANCE conflicts with the generated singleton field and is not supported",
                ));
            }
            if !names.insert(member_name) {
                return Err(Diagnostic::new(
                    member_offset,
                    "Duplicate object member name",
                ));
            }
            self.end()?;
        }
        self.expect("}")?;
        Ok(ObjectDecl {
            name,
            offset,
            internal: visibility.as_deref() == Some("internal"),
            annotations,
            interfaces,
            constants,
            objects,
            fields,
            methods,
            initializers,
            nested,
        })
    }
    fn object_method(&mut self) -> Result<ObjectMethod> {
        let offset = self.token().offset;
        let mut jvm_static = false;
        let mut throws = None;
        let mut suppress = None;
        while self.take("@") {
            let annotation = self.declaration_name()?;
            match annotation.as_str() {
                "JvmStatic" => {
                    if jvm_static { return Err(self.error("Repeated @JvmStatic annotation")); }
                    jvm_static = true;
                    if self.take("(") { self.expect(")")?; }
                }
                "Throws" => {
                    if throws.is_some() { return Err(self.error("Repeated @Throws annotation")); }
                    self.expect("(")?;
                    self.lines();
                    let mut exceptions = Vec::new();
                    loop {
                        let exception = self.declaration_name()?;
                        self.expect("::")?;
                        self.expect("class")?;
                        exceptions.push(exception);
                        if exceptions.len() > 32 { return Err(self.error("@Throws exceeds the current 32-exception limit")); }
                        self.lines();
                        if !self.take(",") { break; }
                        self.lines();
                        if self.at(")") { break; }
                    }
                    self.expect(")")?;
                    throws = Some(exceptions);
                }
                "Suppress" => {
                    if suppress.is_some() { return Err(self.error("Repeated @Suppress annotation")); }
                    self.expect("(")?;
                    self.lines();
                    let mut warnings = Vec::new();
                    loop {
                        if !self.token().text.starts_with('"') { return Err(self.error("@Suppress supports only ordinary String literal arguments")); }
                        let warning = String::from_utf16(&self.plain_string()?).map_err(|_| self.error("@Suppress arguments must be valid Unicode strings"))?;
                        warnings.push(warning);
                        self.pos += 1;
                        if warnings.len() > 64 { return Err(self.error("@Suppress exceeds the current 64-warning limit")); }
                        self.lines();
                        if !self.take(",") { break; }
                        self.lines();
                        if self.at(")") { break; }
                    }
                    self.expect(")")?;
                    suppress = Some(warnings);
                }
                _ => return Err(self.error("Only @JvmStatic, @Throws, and @Suppress object method annotations are supported")),
            }
            self.lines();
        }
        let mut visibility = None;
        let mut inline = false;
        let mut infix = false;
        let mut operator = false;
        let mut is_override = false;
        loop {
            if matches!(
                self.token().text.as_str(),
                "public" | "private" | "internal"
            ) {
                if visibility.is_some() {
                    return Err(
                        self.error("Conflicting or repeated object method visibility modifiers")
                    );
                }
                visibility = Some(self.token().text.clone());
                self.pos += 1;
            } else if self.take("override") {
                is_override = true;
            } else if self.take("operator") {
                if operator {
                    return Err(self.error("Repeated operator modifier"));
                }
                operator = true;
            } else if self.take("infix") {
                if infix {
                    return Err(self.error("Repeated infix modifier"));
                }
                infix = true;
            } else if self.take("inline") {
                if inline {
                    return Err(self.error("Repeated inline modifier"));
                }
                inline = true;
            } else {
                break;
            }
            self.lines();
        }
        if visibility.as_deref() == Some("internal") {
            return Err(self.error("Internal object methods are not supported"));
        }
        if inline && visibility.as_deref() != Some("private") {
            return Err(self.error("Only private inline object methods are supported"));
        }
        let mut function = self.function()?;
        function.offset = offset;
        function.private = visibility.as_deref() == Some("private");
        function.infix = infix;
        function.operator = operator;
        Self::validate_infix_function(&function)?;
        if function.receiver.is_some() {
            return Err(Diagnostic::new(
                offset,
                "Object member extension functions are not supported",
            ));
        }
        Ok(ObjectMethod {
            is_override,
            function,
            jvm_static,
            throws: throws.unwrap_or_default(),
            inline,
            suppress: suppress.unwrap_or_default(),
        })
    }
    fn constant(&mut self) -> Result<ConstDecl> {
        let offset = self.token().offset;
        let mut visibility = None;
        let mut is_const = false;
        loop {
            if matches!(
                self.token().text.as_str(),
                "public" | "internal" | "private"
            ) {
                if visibility.is_some() {
                    return Err(self.error("Conflicting or repeated constant visibility modifiers"));
                }
                visibility = Some(self.token().text.clone());
                self.pos += 1;
            } else if self.take("const") {
                if is_const {
                    return Err(self.error("Repeated const modifier"));
                }
                is_const = true;
            } else {
                break;
            }
            self.lines();
        }
        if !is_const || !self.take("val") {
            return Err(self.error("Only const val literal properties are supported"));
        }
        self.lines();
        let name = self.declaration_name()?;
        let declared_type = if self.take(":") {
            self.lines();
            Some(self.ty()?)
        } else {
            None
        };
        self.lines();
        self.expect("=")?;
        self.lines();
        let negative = if self.take("-") {
            Some(true)
        } else if self.take("+") {
            Some(false)
        } else {
            None
        };
        if negative.is_some() {
            self.lines();
        }
        let token = self.token();
        let (value, ty) = if token.text.as_bytes()[0].is_ascii_digit() {
            match numeric_literal(&token.text, token.offset)? {
                ExprKind::Int(value) => {
                    let value = if negative == Some(true) {
                        -value
                    } else {
                        value
                    };
                    if declared_type == Some(Type::Long) || i32::try_from(value).is_err() {
                        (ConstValue::Long(value), Type::Long)
                    } else {
                        (ConstValue::Int(value as i32), Type::Int)
                    }
                }
                ExprKind::Long(value) => {
                    let value = if negative == Some(true) {
                        -value
                    } else {
                        value
                    };
                    let value = i64::try_from(value)
                        .map_err(|_| self.error("Long constant literal is out of range"))?;
                    (ConstValue::Long(value), Type::Long)
                }
                ExprKind::Float(value) => {
                    let value = if negative == Some(true) {
                        (-f32::from_bits(value)).to_bits()
                    } else {
                        value
                    };
                    (ConstValue::Float(value), Type::Float)
                }
                _ => unreachable!(),
            }
        } else if negative.is_some() {
            return Err(self.error("Only numeric constant literals may have a sign"));
        } else if token.text.starts_with('\'') {
            (
                ConstValue::Char(char_literal(&token.text, 0)?.1),
                Type::Char,
            )
        } else if token.text.starts_with('"') {
            (ConstValue::String(self.plain_string()?), Type::String)
        } else if token.text == "true" || token.text == "false" {
            (ConstValue::Boolean(token.text == "true"), Type::Boolean)
        } else {
            return Err(self.error("Const initializers require supported scalar literals"));
        };
        if declared_type.is_some_and(|declared| declared != ty) {
            return Err(self.error(
                "Const literal type mismatch; explicit types must match the supported literal type",
            ));
        }
        self.pos += 1;
        if !matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
            return Err(self
                .error("Const initializers support only literals; expressions are not supported"));
        }
        Ok(ConstDecl {
            name,
            offset,
            private: visibility.as_deref() == Some("private"),
            internal: visibility.as_deref() == Some("internal"),
            value,
        })
    }
    fn class_decl(&mut self) -> Result<ClassDecl> {
        let offset = self.token().offset;
        let mut visibility = None;
        let mut final_modifier = false;
        let mut data = false;
        let mut open = false;
        let mut abstract_ = false;
        loop {
            if matches!(self.token().text.as_str(), "public" | "internal") {
                if visibility.is_some() {
                    return Err(self.error("Conflicting or repeated class visibility modifiers"));
                }
                visibility = Some(self.token().text.clone());
                self.pos += 1;
            } else if self.take("data") {
                if data {
                    return Err(self.error("Repeated data modifier"));
                }
                data = true;
            } else if self.take("abstract") {
                abstract_ = true;
            } else if self.take("open") {
                if open {
                    return Err(self.error("Repeated open modifier"));
                }
                open = true;
            } else if self.take("final") {
                if final_modifier {
                    return Err(self.error("Repeated final class modifier"));
                }
                final_modifier = true;
            } else {
                break;
            }
            self.lines();
        }
        if matches!(self.token().text.as_str(), "private" | "protected") {
            return Err(self.error("Unsupported class visibility; use public or internal"));
        }
        if !self.take("class") {
            return Err(self.error("Only ordinary final classes are supported"));
        }
        self.lines();
        let name = self.class_name()?;
        let after_name = self.pos;
        self.lines();
        let has_constructor = self.take("(");
        let mut params = Vec::new();
        let mut public_params = Vec::new();
        let mut param_properties = Vec::new();
        let mut override_params = Vec::new();
        let mut mutable_params = Vec::new();
        let mut defaults = Vec::new();
        let mut property_names = std::collections::HashSet::new();
        let mut param_names = std::collections::HashSet::new();
        if has_constructor {
            self.lines();
            if !self.at(")") {
                loop {
                    let is_override = self.take("override");
                    let private = self.take("private");
                    let public = !private && self.take("public");
                    self.lines();
                    let mutable = self.take("var");
                    let property = mutable || self.take("val");
                    if (private || public) && !property {
                        return Err(
                            self.error("Visibility modifiers require a constructor property")
                        );
                    }
                    self.lines();
                    let param_offset = self.token().offset;
                    let param = self.declaration_name()?;
                    if !param_names.insert(param.clone()) {
                        return Err(Diagnostic::new(
                            param_offset,
                            "Duplicate constructor parameter name",
                        ));
                    }
                    self.expect(":")?;
                    self.lines();
                    if property {
                        if is_override {
                            override_params.push(param.clone());
                        }
                        property_names.insert(param.clone());
                        param_properties.push(param.clone());
                        if !private {
                            public_params.push(param.clone());
                        }
                        if mutable {
                            mutable_params.push(param.clone());
                        }
                    }
                    params.push((param, self.ty()?));
                    if params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() > 254 {
                        return Err(
                            self.error("Class constructors support at most 254 parameter slots")
                        );
                    }
                    self.lines();
                    defaults.push(if self.take("=") {
                        self.lines();
                        Some(self.expr(0)?)
                    } else {
                        None
                    });
                    self.lines();
                    if !self.take(",") {
                        break;
                    }
                    self.lines();
                    if self.at(")") {
                        break;
                    }
                }
            }
            self.expect(")")?;
        } else {
            self.pos = after_name;
        }
        let after_params = self.pos;
        let mut after_header = after_params;
        self.lines();
        let mut interfaces = Vec::new();
        let mut super_class = None;
        if self.take(":") {
            self.lines();
            loop {
                let mut name = self.declaration_name()?;
                while self.take(".") {
                    name.push('/');
                    name.push_str(&self.declaration_name()?);
                }
                if self.at("(") {
                    if super_class.is_some() {
                        return Err(self.error("Only one superclass constructor is allowed"));
                    }
                    super_class = Some((name, self.arguments()?));
                } else {
                    interfaces.push(name);
                }
                if interfaces.len() > 32 {
                    return Err(self.error("Classes exceed the current 32-interface limit"));
                }
                after_header = self.pos;
                self.lines();
                if !self.take(",") {
                    break;
                }
                self.lines();
            }
        }
        let mut declaration = ClassDecl {
            primary_constructor: true,
            name,
            offset,
            internal: visibility.as_deref() == Some("internal"),
            params,
            public_params,
            param_properties,
            override_params,
            mutable_params,
            defaults,
            data,
            interfaces,
            properties: Vec::new(),
            methods: Vec::new(),
            companion: None,
            initializers: Vec::new(),
            nested: Vec::new(),
            secondary_constructors: Vec::new(),
            super_class,
            open,
            abstract_,
        };
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(declaration);
            }
            return Err(self.error("Expected a class body or declaration separator"));
        }
        self.class_members(&mut declaration, &mut property_names)?;
        declaration.primary_constructor =
            has_constructor || declaration.secondary_constructors.is_empty();
        Ok(declaration)
    }
    fn class_members(
        &mut self,
        declaration: &mut ClassDecl,
        property_names: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        self.separators();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated class body"));
            }
            let offset = self.token().offset;
            if self.take("init") {
                self.lines();
                declaration.initializers.push((offset, self.block()?));
            } else if !self.at("companion")
                && (self.starts_class()
                    || self.starts_enum()
                    || self.starts_object()
                    || self.starts_interface())
            {
                declaration.nested.push(self.declaration()?);
            } else if self.at("constructor") {
                declaration
                    .secondary_constructors
                    .push(self.secondary_constructor()?);
            } else if self.at("companion") {
                if declaration.companion.is_some() {
                    return Err(self.error("Classes support only one companion object"));
                }
                declaration.companion = Some(self.class_companion()?);
            } else {
                let mut is_override = false;
                let mut visibility = None;
                let mut infix = false;
                let mut operator = false;
                let mut open = false;
                let mut final_ = false;
                let mut lateinit = false;
                let mut abstract_ = false;
                loop {
                    if self.take("override") {
                        if is_override {
                            return Err(self.error("Repeated override modifier"));
                        }
                        is_override = true;
                    } else if self.take("infix") {
                        if infix {
                            return Err(self.error("Repeated infix modifier"));
                        }
                        infix = true;
                    } else if self.take("operator") {
                        if operator {
                            return Err(self.error("Repeated operator modifier"));
                        }
                        operator = true;
                    } else if self.take("abstract") {
                        abstract_ = true;
                        open = true;
                    } else if self.take("open") {
                        open = true;
                    } else if self.take("final") {
                        if final_ {
                            return Err(self.error("Repeated final modifier"));
                        }
                        final_ = true;
                    } else if self.take("lateinit") {
                        lateinit = true;
                    } else if matches!(
                        self.token().text.as_str(),
                        "public" | "private" | "internal" | "protected"
                    ) {
                        if visibility.is_some() {
                            return Err(
                                self.error("Conflicting or repeated member visibility modifiers")
                            );
                        }
                        visibility = Some(self.token().text.clone());
                        self.pos += 1;
                    } else {
                        break;
                    }
                    self.lines();
                }
                if final_ && open {
                    return Err(self.error("Conflicting open/abstract and final modifiers"));
                }
                open = !final_ && (open || is_override);
                if self.at("fun") {
                    if lateinit {
                        return Err(self.error("The lateinit modifier applies only to properties"));
                    }
                    let (mut function, has_body) = self.function_body(abstract_)?;
                    function.offset = offset;
                    function.private = visibility.as_deref() == Some("private");
                    function.protected = visibility.as_deref() == Some("protected");
                    function.internal = visibility.as_deref() == Some("internal");
                    function.infix = infix;
                    function.operator = operator;
                    Self::validate_infix_function(&function)?;
                    declaration.methods.push(ClassMethod {
                        function,
                        is_override,
                        open,
                        has_body,
                    });
                } else if self.at("val") || self.at("var") {
                    if infix {
                        return Err(self.error("The infix modifier applies only to functions"));
                    }
                    let mut property = self.property(
                        offset,
                        visibility.as_deref() == Some("private"),
                        is_override,
                        lateinit,
                        open,
                    )?;
                    property.protected = visibility.as_deref() == Some("protected");
                    if !property_names.insert(property.name.clone()) {
                        return Err(Diagnostic::new(offset, "Duplicate class property name"));
                    }
                    declaration.properties.push(property);
                } else {
                    return Err(self.error("Expected a class member declaration"));
                }
            }
            self.end()?;
        }
        self.expect("}")
    }
    fn declaration(&mut self) -> Result<Declaration> {
        self.depth += 1;
        if self.depth > 16 {
            return Err(self.error("Declaration nesting exceeds the current limit"));
        }
        let result = if self.starts_enum() {
            Ok(Declaration::Enum(self.enum_decl()?))
        } else if self.starts_class() {
            Ok(Declaration::Class(self.class_decl()?))
        } else if self.starts_interface() {
            Ok(Declaration::Interface(self.interface_decl(1, &mut 0)?))
        } else {
            Ok(Declaration::Object(self.object_decl(1, &mut 0)?))
        };
        self.depth -= 1;
        result
    }
    fn property(
        &mut self,
        offset: usize,
        private: bool,
        is_override: bool,
        lateinit: bool,
        open: bool,
    ) -> Result<PropertyDecl> {
        let mutable = self.take("var");
        if !mutable {
            self.expect("val")?;
        }
        let name = self.declaration_name()?;
        let ty = if self.take(":") {
            self.lines();
            Some(self.ty()?)
        } else {
            None
        };
        let after_type = self.pos;
        self.lines();
        let mut initializer = None;
        let mut delegate = None;
        if self.take("=") {
            self.lines();
            initializer = Some(self.expr(0)?);
        } else if self.take("by") {
            self.lines();
            delegate = Some(self.expr(0)?);
        } else {
            self.pos = after_type;
        }
        let mut body = Vec::new();
        let mut setter = None;
        let mut has_getter = false;
        loop {
            let save = self.pos;
            self.separators();
            if self.take("get") {
                if has_getter {
                    return Err(self.error("Repeated property getter"));
                }
                has_getter = true;
                body = self.getter_body()?;
            } else if self.take("set") {
                if !mutable {
                    return Err(self.error("Read-only properties cannot have setters"));
                }
                if setter.is_some() {
                    return Err(self.error("Repeated property setter"));
                }
                self.expect("(")?;
                let parameter = self.name()?;
                if self.take(":") {
                    let parameter_type = self.ty()?;
                    if ty.as_ref() != Some(&parameter_type) {
                        return Err(self.error(
                            "Setter parameter types require the matching explicit property type",
                        ));
                    }
                }
                self.expect(")")?;
                self.lines();
                let statements = if self.take("=") {
                    self.lines();
                    vec![Statement {
                        offset: self.token().offset,
                        kind: StatementKind::Expression(self.expression_body()?),
                    }]
                } else {
                    self.block()?
                };
                setter = Some((parameter, statements));
            } else {
                self.pos = save;
                break;
            }
        }
        if lateinit
            && (!mutable
                || ty.is_none()
                || initializer.is_some()
                || delegate.is_some()
                || !body.is_empty()
                || setter.is_some())
        {
            return Err(Diagnostic::new(
                offset,
                "Lateinit requires a typed mutable property without an initializer or custom accessor",
            ));
        }
        if initializer.is_none() && delegate.is_none() && body.is_empty() && !lateinit {
            return Err(Diagnostic::new(
                offset,
                "Properties require an initializer, delegate, getter, or lateinit modifier",
            ));
        }
        Ok(PropertyDecl {
            name,
            private,
            protected: false,
            mutable,
            inferred: ty.is_none(),
            initializer,
            offset,
            ty: ty.unwrap_or(Type::Unit),
            body,
            is_override,
            setter,
            delegate,
            lateinit,
            open,
        })
    }
    fn arguments(&mut self) -> Result<Vec<Expr>> {
        self.expect("(")?;
        self.lines();
        let mut arguments = Vec::new();
        while !self.at(")") {
            let argument = if self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.text == "=")
            {
                let offset = self.token().offset;
                let name = self.name()?;
                self.expect("=")?;
                self.lines();
                Expr {
                    offset,
                    kind: ExprKind::NamedArgument(name, Box::new(self.expr(0)?)),
                }
            } else {
                self.expr(0)?
            };
            arguments.push(argument);
            self.lines();
            if !self.take(",") {
                break;
            }
            self.lines();
        }
        self.expect(")")?;
        Ok(arguments)
    }
    fn secondary_constructor(&mut self) -> Result<SecondaryConstructor> {
        let offset = self.token().offset;
        self.expect("constructor")?;
        self.expect("(")?;
        self.lines();
        let mut params = Vec::new();
        let mut defaults = Vec::new();
        while !self.at(")") {
            let name = self.declaration_name()?;
            self.expect(":")?;
            self.lines();
            params.push((name, self.ty()?));
            self.lines();
            defaults.push(if self.take("=") {
                self.lines();
                Some(self.expr(0)?)
            } else {
                None
            });
            self.lines();
            if !self.take(",") {
                break;
            }
            self.lines();
        }
        self.expect(")")?;
        let after_parameters = self.pos;
        self.lines();
        let (delegation, arguments) = if self.take(":") {
            self.lines();
            let delegation = if self.take("this") {
                "this"
            } else if self.take("super") {
                "super"
            } else {
                return Err(self.error("Constructor delegation must call this or super"));
            }
            .to_string();
            (delegation, self.arguments()?)
        } else {
            self.pos = after_parameters;
            ("super".into(), Vec::new())
        };
        let after_header = self.pos;
        self.lines();
        let body = if self.at("{") {
            self.block()?
        } else {
            self.pos = after_header;
            Vec::new()
        };
        Ok(SecondaryConstructor {
            offset,
            params,
            defaults,
            delegation,
            arguments,
            body,
        })
    }
    fn class_companion(&mut self) -> Result<ClassCompanion> {
        self.expect("companion")?;
        self.lines();
        self.expect("object")?;
        let after_header = self.pos;
        self.lines();
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(ClassCompanion {
                    properties: Vec::new(),
                    methods: Vec::new(),
                    initializers: Vec::new(),
                });
            }
            return Err(self.error("Only unnamed companion objects are supported"));
        }
        let mut declaration = ClassDecl {
            primary_constructor: true,
            name: "Companion".into(),
            offset: self.token().offset,
            internal: false,
            params: Vec::new(),
            public_params: Vec::new(),
            param_properties: Vec::new(),
            override_params: Vec::new(),
            mutable_params: Vec::new(),
            defaults: Vec::new(),
            data: false,
            interfaces: Vec::new(),
            properties: Vec::new(),
            methods: Vec::new(),
            companion: None,
            initializers: Vec::new(),
            nested: Vec::new(),
            secondary_constructors: Vec::new(),
            super_class: None,
            open: false,
            abstract_: false,
        };
        self.class_members(&mut declaration, &mut std::collections::HashSet::new())?;
        let mut properties = Vec::new();
        for property in declaration.properties {
            properties.push(ClassCompanionProperty {
                name: property.name,
                offset: property.offset,
                private: property.private,
                mutable: property.mutable,
                ty: if property.inferred {
                    None
                } else {
                    Some(property.ty)
                },
                initializer: property.initializer,
                body: property.body,
                setter: property.setter,
                delegate: property.delegate,
                lateinit: property.lateinit,
            });
        }
        Ok(ClassCompanion {
            properties,
            methods: declaration.methods,
            initializers: declaration.initializers,
        })
    }
    fn interface_decl(&mut self, depth: usize, count: &mut usize) -> Result<InterfaceDecl> {
        let offset = self.token().offset;
        if depth > 16 {
            return Err(
                self.error("Interface nesting exceeds the current 16-declaration depth limit")
            );
        }
        *count += 1;
        if *count > 256 {
            return Err(self.error("Interfaces exceed the current 256-declaration limit"));
        }
        let mut visibility = None;
        while matches!(self.token().text.as_str(), "public" | "internal") {
            if visibility.is_some() {
                return Err(self.error("Conflicting or repeated interface visibility modifiers"));
            }
            visibility = Some(self.token().text.clone());
            self.pos += 1;
            self.lines();
        }
        if !self.take("interface") {
            return Err(self.error("Only public or internal plain interfaces are supported"));
        }
        self.lines();
        let name = self.class_name()?;
        let mut after_header = self.pos;
        self.lines();
        let mut interfaces = Vec::new();
        if self.take(":") {
            self.lines();
            loop {
                interfaces.push(self.declaration_name()?);
                if interfaces.len() > 32 {
                    return Err(self.error("Interfaces exceed the current 32-superinterface limit"));
                }
                after_header = self.pos;
                self.lines();
                if !self.take(",") {
                    break;
                }
                self.lines();
            }
        }
        let mut declaration = InterfaceDecl {
            name,
            offset,
            internal: visibility.as_deref() == Some("internal"),
            interfaces,
            properties: Vec::new(),
            methods: Vec::new(),
            nested: Vec::new(),
            declarations: Vec::new(),
            companion: None,
        };
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(declaration);
            }
            return Err(self.error(
                "Interface constructors, generics, and qualified supertypes are not supported",
            ));
        }
        self.separators();
        let mut names = std::collections::HashSet::new();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated interface body"));
            }
            if self.starts_interface() {
                let nested = self.interface_decl(depth + 1, count)?;
                if !names.insert(nested.name.clone()) {
                    return Err(Diagnostic::new(
                        nested.offset,
                        "Duplicate interface member name",
                    ));
                }
                declaration.nested.push(nested);
            } else if !self.at("companion")
                && (self.starts_class() || self.starts_enum() || self.starts_object())
            {
                declaration.declarations.push(self.declaration()?);
            } else if self.at("companion") {
                if declaration.companion.is_some() {
                    return Err(self.error("Only one unnamed companion object is supported"));
                }
                declaration.companion = Some(self.companion_decl()?);
            } else {
                let offset = self.token().offset;
                let mut is_override = false;
                let mut public_modifier = false;
                let mut infix = false;
                let mut operator = false;
                loop {
                    if self.take("override") {
                        if is_override {
                            return Err(self.error("Repeated override modifier"));
                        }
                        is_override = true;
                    } else if self.take("operator") {
                        operator = true;
                    } else if self.take("infix") {
                        if infix {
                            return Err(self.error("Repeated infix modifier"));
                        }
                        infix = true;
                    } else if self.take("public") {
                        if public_modifier {
                            return Err(self.error("Repeated public interface member modifier"));
                        }
                        public_modifier = true;
                    } else {
                        break;
                    }
                    self.lines();
                }
                if infix && !self.at("fun") {
                    return Err(self.error("The infix modifier applies only to functions"));
                }
                if self.at("fun") {
                    let (mut function, has_body) = self.function_body(true)?;
                    function.infix = infix;
                    function.operator = operator;
                    Self::validate_infix_function(&function)?;
                    if function.receiver.is_some() {
                        return Err(Diagnostic::new(
                            offset,
                            "Interface extension functions are not supported",
                        ));
                    }
                    declaration.methods.push(InterfaceMethod {
                        function,
                        has_body,
                        is_override,
                    });
                } else if self.at("val") || self.at("var") {
                    let mutable = self.take("var");
                    if !mutable {
                        self.expect("val")?;
                    }
                    let name = self.declaration_name()?;
                    if !names.insert(name.clone()) {
                        return Err(Diagnostic::new(offset, "Duplicate interface member name"));
                    }
                    self.expect(":")?;
                    self.lines();
                    let ty = self.ty()?;
                    let after_type = self.pos;
                    self.lines();
                    let body = if self.take("get") {
                        if mutable {
                            return Err(self.error(
                                "Mutable interface properties require abstract default accessors",
                            ));
                        }
                        Some(self.getter_body()?)
                    } else {
                        self.pos = after_type;
                        None
                    };
                    declaration.properties.push(InterfaceProperty {
                        name,
                        offset,
                        ty,
                        body,
                        is_override,
                        mutable,
                    });
                } else {
                    return Err(self.error("Only public interface properties, functions, nested interfaces, and companion objects are supported"));
                }
            }
            if declaration.properties.len() + declaration.methods.len() + declaration.nested.len()
                > 1024
            {
                return Err(self.error("Interfaces exceed the current 1024-member limit"));
            }
            self.end()?;
        }
        self.expect("}")?;
        Ok(declaration)
    }
    fn getter_body(&mut self) -> Result<Vec<Statement>> {
        self.expect("(")?;
        self.expect(")")?;
        self.lines();
        if self.take("=") {
            self.lines();
            Ok(vec![Statement {
                offset: self.token().offset,
                kind: StatementKind::Return(Some(self.expression_body()?)),
            }])
        } else {
            self.block()
        }
    }
    fn companion_decl(&mut self) -> Result<CompanionDecl> {
        self.expect("companion")?;
        self.lines();
        self.expect("object")?;
        let after_header = self.pos;
        self.lines();
        let mut properties = Vec::new();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut initializers = Vec::new();
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(CompanionDecl {
                    properties,
                    fields,
                    methods,
                    initializers,
                });
            }
            return Err(self.error("Only unnamed companion objects are supported"));
        }
        self.separators();
        let mut names = std::collections::HashSet::new();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated companion object"));
            }
            let offset = self.token().offset;
            if self.take("init") {
                self.lines();
                initializers.push((offset, self.block()?));
                self.end()?;
                continue;
            }
            if self.starts_declaration("fun") {
                methods.push(ClassMethod {
                    function: self.function()?,
                    is_override: false,
                    open: false,
                    has_body: true,
                });
                self.end()?;
                continue;
            }
            let mut cursor = self.pos;
            while self.tokens.get(cursor).is_some_and(|token| {
                !matches!(
                    token.text.as_str(),
                    "=" | "by" | "get" | "\n" | ";" | "}" | "<eof>"
                )
            }) {
                cursor += 1;
            }
            let legacy = self
                .tokens
                .get(cursor)
                .is_some_and(|token| token.text == "=")
                && self
                    .tokens
                    .get(cursor + 1)
                    .is_some_and(|token| matches!(token.text.as_str(), "arrayOf" | "object"));
            if !legacy {
                let private = self.take("private");
                if !private {
                    self.take("public");
                    self.take("internal");
                }
                self.lines();
                let lateinit = self.take("lateinit");
                self.lines();
                let property = self.property(offset, private, false, lateinit, false)?;
                if !names.insert(property.name.clone()) {
                    return Err(Diagnostic::new(offset, "Duplicate companion property name"));
                }
                fields.push(ClassCompanionProperty {
                    name: property.name,
                    offset: property.offset,
                    private: property.private,
                    mutable: property.mutable,
                    ty: if property.inferred {
                        None
                    } else {
                        Some(property.ty)
                    },
                    initializer: property.initializer,
                    body: property.body,
                    setter: property.setter,
                    delegate: property.delegate,
                    lateinit: property.lateinit,
                });
                self.end()?;
                continue;
            }
            let private = self.take("private");
            if !private {
                self.take("public");
            }
            self.lines();
            if !self.take("val") {
                return Err(self.error("Companions support only private literal String arrays and public anonymous interface object properties"));
            }
            let name = self.declaration_name()?;
            if !names.insert(name.clone()) {
                return Err(Diagnostic::new(offset, "Duplicate companion property name"));
            }
            if name == "INSTANCE" {
                return Err(Diagnostic::new(
                    offset,
                    "Companion member name INSTANCE is not supported",
                ));
            }
            let declared_type = if self.take(":") {
                self.lines();
                Some(self.ty()?)
            } else {
                None
            };
            self.lines();
            self.expect("=")?;
            self.lines();
            let value = if self.take("arrayOf") {
                if !private
                    || declared_type
                        .as_ref()
                        .is_some_and(|ty| ty != &Type::StringArray)
                {
                    return Err(self.error("Companion String arrays must be private and have inferred or matching Array<String> type"));
                }
                self.expect("(")?;
                self.lines();
                let mut strings = Vec::new();
                if !self.at(")") {
                    loop {
                        if !self.token().text.starts_with('"') {
                            return Err(self.error(
                                "Companion arrayOf initializers support only String literals",
                            ));
                        }
                        strings.push(self.plain_string()?);
                        self.pos += 1;
                        if strings.len() > 4096 {
                            return Err(self
                                .error("Companion arrays exceed the current 4096-element limit"));
                        }
                        self.lines();
                        if !self.take(",") {
                            break;
                        }
                        self.lines();
                        if self.at(")") {
                            break;
                        }
                    }
                }
                self.expect(")")?;
                if strings.is_empty() && declared_type.is_none() {
                    return Err(self.error(
                        "Empty companion arrayOf initializers require explicit Array<String> type",
                    ));
                }
                CompanionValue::StringArray(strings)
            } else if self.take("object") {
                if private {
                    return Err(self.error("Anonymous companion interface objects must be public"));
                }
                self.expect(":")?;
                self.lines();
                let interface = self.declaration_name()?;
                if declared_type
                    .as_ref()
                    .is_some_and(|ty| ty != &Type::Java(interface.clone()))
                {
                    return Err(self.error(
                        "Anonymous companion object property type must match its interface",
                    ));
                }
                self.lines();
                self.expect("{")?;
                let mut object = ClassDecl {
                    primary_constructor: true,
                    name: String::new(),
                    offset,
                    internal: false,
                    params: Vec::new(),
                    public_params: Vec::new(),
                    param_properties: Vec::new(),
                    override_params: Vec::new(),
                    mutable_params: Vec::new(),
                    defaults: Vec::new(),
                    data: false,
                    interfaces: vec![interface],
                    properties: Vec::new(),
                    methods: Vec::new(),
                    companion: None,
                    initializers: Vec::new(),
                    nested: Vec::new(),
                    secondary_constructors: Vec::new(),
                    super_class: None,
                    open: false,
                    abstract_: false,
                };
                self.class_members(&mut object, &mut std::collections::HashSet::new())?;
                CompanionValue::Object(Box::new(object))
            } else {
                return Err(self.error("Companions support only literal String arrays and anonymous interface object initializers"));
            };
            properties.push(CompanionProperty {
                name,
                offset,
                private,
                value,
            });
            if properties.len() > 256 {
                return Err(self.error("Companions exceed the current 256-property limit"));
            }
            self.end()?;
        }
        self.expect("}")?;
        Ok(CompanionDecl {
            properties,
            fields,
            methods,
            initializers,
        })
    }
    fn validate_infix_function(function: &Function) -> Result<()> {
        if function.infix
            && (function.params.len() != 1 || function.defaults.iter().any(Option::is_some))
        {
            return Err(Diagnostic::new(
                function.offset,
                "Infix functions require exactly one parameter without a default value",
            ));
        }
        Ok(())
    }
    fn function(&mut self) -> Result<Function> {
        self.function_body(false).map(|(function, _)| function)
    }
    fn function_body(&mut self, abstract_allowed: bool) -> Result<(Function, bool)> {
        let offset = self.token().offset;
        let mut visibility = None;
        let mut infix = false;
        let mut operator = false;
        loop {
            if matches!(
                self.token().text.as_str(),
                "private" | "public" | "internal"
            ) {
                if visibility.is_some() {
                    return Err(self.error("Conflicting or repeated visibility modifiers"));
                }
                visibility = Some(self.token().text.clone());
                self.pos += 1;
            } else if self.take("operator") {
                if operator {
                    return Err(self.error("Repeated operator modifier"));
                }
                operator = true;
            } else if self.take("infix") {
                if infix {
                    return Err(self.error("Repeated infix modifier"));
                }
                infix = true;
            } else {
                break;
            }
            self.lines();
        }
        let private = visibility.as_deref() == Some("private");
        let internal = visibility.as_deref() == Some("internal");
        self.expect("fun")?;
        let type_parameter_count = self.type_parameters.len();
        let type_params = self.type_parameters()?;
        let first = self.name()?;
        let (receiver, name) = if self.at("<") || self.at("?") || self.at(".") {
            let receiver = self.named_ty(first)?;
            let receiver = if self.take("?") {
                Type::Nullable(Box::new(receiver))
            } else {
                receiver
            };
            self.expect(".")?;
            (Some(receiver), self.name()?)
        } else {
            (None, first)
        };
        self.expect("(")?;
        self.lines();
        let mut params = Vec::new();
        let mut defaults = Vec::new();
        if !self.at(")") {
            loop {
                let name = self.name()?;
                self.expect(":")?;
                self.lines();
                params.push((name, self.ty()?));
                self.lines();
                defaults.push(if self.take("=") {
                    self.lines();
                    Some(self.expr(0)?)
                } else {
                    None
                });
                self.lines();
                if !self.take(",") {
                    break;
                }
                self.lines();
                if self.at(")") {
                    break;
                }
            }
        }
        self.expect(")")?;
        if infix && (params.len() != 1 || defaults.iter().any(Option::is_some)) {
            return Err(
                self.error("Infix functions require exactly one parameter without a default value")
            );
        }
        let result_declared = self.take(":");
        let result = if result_declared {
            self.lines();
            self.ty()?
        } else {
            Type::Unit
        };
        if self.at("where") {
            return Err(
                self.error("Multiple type parameter bounds in where clauses are unsupported")
            );
        }
        let after_signature = self.pos;
        self.lines();
        let mut has_body = true;
        let mut result_inferred = false;
        let body = if self.take("=") {
            result_inferred = !result_declared;
            self.lines();
            vec![Statement {
                offset: self.token().offset,
                kind: StatementKind::Return(Some(self.expression_body()?)),
            }]
        } else if self.at("{") {
            self.block()?
        } else if abstract_allowed {
            self.pos = after_signature;
            has_body = false;
            Vec::new()
        } else {
            return Err(self.error("Expected a function body"));
        };
        self.type_parameters.truncate(type_parameter_count);
        Ok((
            Function {
                name,
                offset,
                private,
                protected: false,
                internal,
                infix,
                operator,
                receiver,
                type_params,
                params,
                defaults,
                result,
                result_inferred,
                body,
            },
            has_body,
        ))
    }
    fn block(&mut self) -> Result<Vec<Statement>> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.error("Nesting exceeds the current limit"));
        }
        self.expect("{")?;
        self.separators();
        let mut result = Vec::new();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated block"));
            }
            result.push(self.statement()?);
            self.end()?;
        }
        self.expect("}")?;
        self.depth -= 1;
        Ok(result)
    }
    fn statement(&mut self) -> Result<Statement> {
        self.depth += 1;
        if self.depth > 48 {
            return Err(self.error("Nesting exceeds the current limit"));
        }
        let offset = self.token().offset;
        let kind = if self
            .tokens
            .get(self.pos + 1)
            .is_some_and(|token| token.text == "@")
            && identifier(&self.token().text)
        {
            let label = self.name()?;
            self.expect("@")?;
            self.lines();
            StatementKind::Labeled(label, Box::new(self.statement()?))
        } else if self.at("{") {
            StatementKind::Block(self.block()?)
        } else if self.starts_declaration("fun") {
            StatementKind::LocalFunction(Box::new(self.function()?))
        } else if self.at("val") || self.at("var") {
            let mutable = self.take("var");
            if !mutable {
                self.expect("val")?;
            }
            if self.take("(") {
                let mut names = Vec::new();
                loop {
                    let name = if self.take("_") {
                        "_".into()
                    } else {
                        self.name()?
                    };
                    names.push(if name == "_" { None } else { Some(name) });
                    self.lines();
                    if !self.take(",") {
                        break;
                    }
                    self.lines();
                }
                self.expect(")")?;
                self.expect("=")?;
                self.lines();
                let kind = StatementKind::Destructure(names, mutable, self.expr(0)?);
                self.depth -= 1;
                return Ok(Statement { kind, offset });
            }
            let name = self.name()?;
            let ty = if self.take(":") {
                Some(self.ty()?)
            } else {
                None
            };
            if self.take("=") {
                self.lines();
                StatementKind::Let(name, mutable, ty, self.expr(0)?)
            } else {
                let ty = ty.ok_or_else(|| {
                    self.error("Uninitialized locals require an explicit supported type")
                })?;
                StatementKind::Declare(name, mutable, ty)
            }
        } else if self.take("continue") {
            if self.take("@") {
                StatementKind::ContinueLabel(self.name()?)
            } else {
                StatementKind::Continue
            }
        } else if self.take("break") {
            if self.take("@") {
                StatementKind::BreakLabel(self.name()?)
            } else {
                StatementKind::Break
            }
        } else if self.take("for") {
            self.expect("(")?;
            self.lines();
            let pattern = if self.at("(") {
                Some(self.destructuring_names()?)
            } else {
                None
            };
            let name = if pattern.is_some() {
                format!("$destructure{offset}")
            } else {
                self.declaration_name()?
            };
            self.expect("in")?;
            self.lines();
            let iterable = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            self.lines();
            let mut body = self.statement_body()?;
            if let Some(names) = pattern {
                body.insert(
                    0,
                    Statement {
                        offset,
                        kind: StatementKind::Destructure(
                            names,
                            false,
                            Expr {
                                offset,
                                kind: ExprKind::Name(name.clone()),
                            },
                        ),
                    },
                );
            }
            match iterable.kind {
                ExprKind::Binary(op, start, end) if op == "until" => {
                    StatementKind::ForUntil(name, *start, *end, body)
                }
                kind => StatementKind::ForEach(name, Expr { kind, ..iterable }, body),
            }
        } else if self.take("throw") {
            self.lines();
            StatementKind::Throw(self.expr(0)?)
        } else if self.take("return") {
            if self.take("@") {
                let label = self.name()?;
                let value = if self.at("\n") || self.at(";") || self.at("}") {
                    None
                } else {
                    Some(Box::new(self.expr(0)?))
                };
                self.depth -= 1;
                return Ok(Statement {
                    offset,
                    kind: StatementKind::Expression(Expr {
                        offset,
                        kind: ExprKind::LabeledReturn(label, value),
                    }),
                });
            }
            StatementKind::Return(if self.at("\n") || self.at(";") || self.at("}") {
                None
            } else {
                Some(self.expr(0)?)
            })
        } else if self.take("when") {
            let subjectless = !self.at("(");
            let mut binding = None;
            let mut binding_type = None;
            let subject = if self.take("(") {
                self.lines();
                if self.take("val") {
                    let name = self.declaration_name()?;
                    if self.take(":") {
                        binding_type = Some(self.ty()?);
                    }
                    self.expect("=")?;
                    self.lines();
                    binding = Some(name);
                }
                let subject = self.expr(0)?;
                let subject = if let Some(ty) = binding_type {
                    let temporary = format!("$when{offset}");
                    Expr {
                        offset: subject.offset,
                        kind: ExprKind::Block(vec![
                            Statement {
                                offset: subject.offset,
                                kind: StatementKind::Let(
                                    temporary.clone(),
                                    false,
                                    Some(ty),
                                    subject,
                                ),
                            },
                            Statement {
                                offset,
                                kind: StatementKind::Expression(Expr {
                                    offset,
                                    kind: ExprKind::Name(temporary),
                                }),
                            },
                        ]),
                    }
                } else {
                    subject
                };
                self.lines();
                self.expect(")")?;
                subject
            } else {
                Expr {
                    offset,
                    kind: ExprKind::Boolean(true),
                }
            };
            self.lines();
            self.expect("{")?;
            self.separators();
            let mut branches = Vec::new();
            let mut otherwise = None;
            while !self.at("}") {
                if self.at("<eof>") {
                    return Err(self.error("Unterminated when"));
                }
                if otherwise.is_some() {
                    return Err(self.error("The else branch must be unique and last"));
                }
                let is_else = self.take("else");
                let mut conditions = Vec::new();
                if !is_else {
                    loop {
                        conditions.push(self.when_condition(subjectless)?);
                        if conditions.len() > 128 {
                            return Err(self.error("Condition nesting exceeds the current limit"));
                        }
                        self.lines();
                        if !self.take(",") {
                            break;
                        }
                        if subjectless {
                            return Err(
                                self.error("Subjectless when conditions cannot be comma-separated")
                            );
                        }
                        self.lines();
                    }
                }
                self.expect("->")?;
                self.lines();
                let branch_depth = branches.len() + usize::from(!is_else);
                self.depth += branch_depth;
                if self.depth > 128 {
                    return Err(self.error("Branch nesting exceeds the current limit"));
                }
                let branch = if self.at("{") {
                    self.block()?
                } else {
                    vec![self.statement()?]
                };
                self.depth -= branch_depth;
                if is_else {
                    otherwise = Some(branch);
                } else {
                    branches.push((conditions, branch));
                    if branches.len() > 128 {
                        return Err(self.error("Branch nesting exceeds the current limit"));
                    }
                }
                self.end()?;
            }
            self.expect("}")?;
            if let Some(name) = binding {
                StatementKind::WhenBinding(name, subject, branches, otherwise)
            } else {
                StatementKind::When(subject, branches, otherwise)
            }
        } else if self.take("do") {
            self.lines();
            let body = self.statement_body()?;
            self.lines();
            self.expect("while")?;
            self.lines();
            self.expect("(")?;
            self.lines();
            let condition = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            StatementKind::DoWhile(condition, body)
        } else if self.take("while") {
            self.expect("(")?;
            self.lines();
            let cond = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            self.lines();
            StatementKind::While(cond, self.statement_body()?)
        } else if self.take("if") {
            self.expect("(")?;
            self.lines();
            let cond = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            self.lines();
            let yes = self.statement_body()?;
            let save = self.pos;
            self.lines();
            let no = if self.take("else") {
                self.lines();
                if self.at("if") {
                    vec![self.statement()?]
                } else {
                    self.statement_body()?
                }
            } else {
                self.pos = save;
                Vec::new()
            };
            StatementKind::If(cond, yes, no)
        } else if self
            .tokens
            .get(self.pos + 1)
            .is_some_and(|t| t.text == "+=")
        {
            let name = self.name()?;
            self.expect("+=")?;
            self.lines();
            StatementKind::AddAssign(name, self.expr(0)?)
        } else if self
            .tokens
            .get(self.pos + 1)
            .is_some_and(|t| matches!(t.text.as_str(), "-=" | "*=" | "/=" | "%="))
        {
            let name = self.name()?;
            let operator = self.token().text.clone();
            self.pos += 1;
            self.lines();
            StatementKind::CompoundAssign(name, operator, self.expr(0)?)
        } else if self.tokens.get(self.pos + 1).is_some_and(|t| t.text == "=") {
            let name = self.name()?;
            self.expect("=")?;
            self.lines();
            StatementKind::Assign(name, self.expr(0)?)
        } else {
            let target = self.expr(0)?;
            if self.take("=") {
                self.lines();
                let value = self.expr(0)?;
                match target.kind {
                    ExprKind::Indices(array, indices) => {
                        StatementKind::IndicesAssign(*array, indices, value)
                    }
                    ExprKind::Index(array, index) => {
                        StatementKind::IndexAssign(*array, *index, value)
                    }
                    ExprKind::Property(receiver, name) => {
                        StatementKind::PropertyAssign(*receiver, name, value)
                    }
                    _ => {
                        return Err(self.error(
                            "Only locals, array elements, and supported properties can be assigned",
                        ));
                    }
                }
            } else if matches!(self.token().text.as_str(), "+=" | "-=" | "*=" | "/=" | "%=") {
                return Err(
                    self.error("Compound assignments currently require a local variable target")
                );
            } else {
                StatementKind::Expression(target)
            }
        };
        self.depth -= 1;
        Ok(Statement { kind, offset })
    }
    fn statement_body(&mut self) -> Result<Vec<Statement>> {
        if self.at("{") {
            self.block()
        } else {
            Ok(vec![self.statement()?])
        }
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        if self.depth > 48 {
            return Err(self.error("Nesting exceeds the current limit"));
        }
        let mut left = self.prefix()?;
        loop {
            if self.at("\n") {
                let next = self.tokens[self.pos..]
                    .iter()
                    .position(|token| token.text != "\n")
                    .unwrap()
                    + self.pos;
                if matches!(self.tokens[next].text.as_str(), "&&" | "||" | "?:" | "as") {
                    self.pos = next;
                }
            }
            let negated_operator = self.at("!")
                && self.tokens.get(self.pos + 1).is_some_and(|token| {
                    matches!(token.text.as_str(), "in" | "is")
                        && token.offset == self.token().offset + 1
                });
            let mut op = if negated_operator {
                format!("!{}", self.tokens[self.pos + 1].text)
            } else {
                self.token().text.clone()
            };
            let Some(precedence) = precedence(&op) else {
                break;
            };
            if precedence < min {
                break;
            }
            self.pos += if negated_operator { 2 } else { 1 };
            if op == "as"
                && self.at("?")
                && self.token().offset == self.tokens[self.pos - 1].offset + 2
            {
                self.pos += 1;
                op.push('?');
            }
            self.lines();
            if matches!(op.as_str(), "is" | "!is" | "as" | "as?") {
                let ty = self.ty()?;
                let offset = left.offset;
                left = Self::bounded(Expr {
                    offset,
                    kind: ExprKind::TypeOperation(op, Box::new(left), ty),
                })?;
                continue;
            }
            let right = self.expr(if op == "?:" {
                precedence
            } else {
                precedence + 1
            })?;
            let offset = left.offset;
            left = Expr {
                offset,
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
            };
            left = Self::bounded(left)?;
        }
        self.depth -= 1;
        Self::bounded(left)
    }
    fn prefix(&mut self) -> Result<Expr> {
        let offset = self.token().offset;
        if self.at("++") || self.at("--") {
            let increment = self.take("++");
            if !increment {
                self.expect("--")?;
            }
            let value = Box::new(self.expr(12)?);
            return Self::bounded(Expr {
                offset,
                kind: if increment {
                    ExprKind::Increment {
                        value,
                        prefix: true,
                    }
                } else {
                    ExprKind::Decrement {
                        value,
                        prefix: true,
                    }
                },
            });
        }
        if self.at("-") || self.at("+") || self.at("!") {
            let op = self.token().text.clone();
            self.pos += 1;
            return Self::bounded(Expr {
                offset,
                kind: ExprKind::Unary(op, Box::new(self.expr(12)?)),
            });
        }
        let mut expr = if self
            .tokens
            .get(self.pos + 1)
            .is_some_and(|token| token.text == "@")
            && identifier(&self.token().text)
        {
            let label = self.name()?;
            self.expect("@")?;
            self.lines();
            let value = if self.at("{") {
                self.lambda()?
            } else {
                self.prefix()?
            };
            Expr {
                offset,
                kind: ExprKind::Labeled(label, Box::new(value)),
            }
        } else if self.take("try") {
            self.lines();
            let body = self.block()?;
            let mut catches = Vec::new();
            let mut finally = Vec::new();
            let mut has_finally = false;
            loop {
                let save = self.pos;
                self.lines();
                if self.take("catch") {
                    self.expect("(")?;
                    let name = self.declaration_name()?;
                    self.expect(":")?;
                    self.lines();
                    let ty = self.ty()?;
                    self.expect(")")?;
                    self.lines();
                    catches.push(CatchClause {
                        name,
                        ty,
                        body: self.block()?,
                    });
                } else if self.take("finally") {
                    has_finally = true;
                    self.lines();
                    finally = self.block()?;
                    break;
                } else {
                    self.pos = save;
                    break;
                }
            }
            if catches.is_empty() && !has_finally {
                return Err(self.error("Try requires catch or finally"));
            }
            Expr {
                offset,
                kind: ExprKind::Try {
                    body,
                    catches,
                    finally,
                },
            }
        } else if self.take("return") {
            let label = if self.take("@") {
                Some(self.name()?)
            } else {
                None
            };
            let value = if matches!(
                self.token().text.as_str(),
                "\n" | ";" | "}" | ")" | "," | "<eof>"
            ) {
                None
            } else {
                Some(Box::new(self.expr(0)?))
            };
            return Self::bounded(Expr {
                offset,
                kind: if let Some(label) = label {
                    ExprKind::LabeledReturn(label, value)
                } else {
                    ExprKind::Return(value)
                },
            });
        } else if self.take("throw") {
            self.lines();
            return Self::bounded(Expr {
                offset,
                kind: ExprKind::Throw(Box::new(self.expr(0)?)),
            });
        } else if self.take("when") {
            self.when_expression(offset)?
        } else if self.take("if") {
            self.expect("(")?;
            self.lines();
            let condition = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            self.lines();
            let yes = self.expression_branch()?;
            self.lines();
            self.expect("else")?;
            self.lines();
            let no = self.expression_branch()?;
            Expr {
                offset,
                kind: ExprKind::If(Box::new(condition), Box::new(yes), Box::new(no)),
            }
        } else if self.at("{") {
            self.lambda()?
        } else if self.take("::") {
            Expr {
                offset,
                kind: ExprKind::CallableReference {
                    receiver: None,
                    name: self.name()?,
                },
            }
        } else if self.take("(") {
            self.lines();
            let expr = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            expr
        } else if self.token().text.starts_with('"') {
            let mut parts = string_parts(&self.token().text, 0)?.1;
            for part in &mut parts {
                shift_expression_offsets(part, offset);
            }
            self.pos += 1;
            if parts.len() == 1 && matches!(parts[0].kind, ExprKind::String(_)) {
                let mut literal = parts.remove(0);
                literal.offset = offset;
                literal
            } else {
                Expr {
                    offset,
                    kind: ExprKind::Template(parts),
                }
            }
        } else if self.token().text.starts_with('\'') {
            let value = char_literal(&self.token().text, 0)?.1;
            self.pos += 1;
            Expr {
                offset,
                kind: ExprKind::Char(value),
            }
        } else if self.take("null") {
            Expr {
                offset,
                kind: ExprKind::Null,
            }
        } else if self.at("true") || self.at("false") {
            let value = self.take("true");
            if !value {
                self.expect("false")?;
            }
            Expr {
                offset,
                kind: ExprKind::Boolean(value),
            }
        } else if self.token().text.as_bytes()[0].is_ascii_digit() {
            let kind = numeric_literal(&self.token().text, self.token().offset)?;
            self.pos += 1;
            Expr { offset, kind }
        } else {
            Expr {
                offset,
                kind: ExprKind::Name(if self.take("this") {
                    "this".into()
                } else if self.take("super") {
                    "super".into()
                } else {
                    self.name()?
                }),
            }
        };
        loop {
            if self.take("@") {
                let label = self.name()?;
                expr = Expr {
                    offset,
                    kind: ExprKind::Name(format!(
                        "{}@{label}",
                        match expr.kind {
                            ExprKind::Name(name) if name == "this" || name == "super" => name,
                            _ =>
                                return Err(self.error("Only this and super accept receiver labels")),
                        }
                    )),
                };
            } else if self.at("<")
                && matches!(
                    expr.kind,
                    ExprKind::Name(_) | ExprKind::Property(_, _) | ExprKind::SafeProperty(_, _)
                )
                && let Some(arguments) = self.call_type_arguments()
            {
                expr = Expr {
                    offset,
                    kind: ExprKind::Generic(Box::new(expr), arguments),
                };
            } else if self.take("::") {
                expr = Expr {
                    offset,
                    kind: ExprKind::CallableReference {
                        receiver: Some(Box::new(expr)),
                        name: self.name()?,
                    },
                };
            } else if self.take("!!") {
                expr = Expr {
                    offset,
                    kind: ExprKind::NonNull(Box::new(expr)),
                };
            } else if self.take("++") {
                expr = Expr {
                    offset,
                    kind: ExprKind::Increment {
                        value: Box::new(expr),
                        prefix: false,
                    },
                };
            } else if self.take("--") {
                expr = Expr {
                    offset,
                    kind: ExprKind::Decrement {
                        value: Box::new(expr),
                        prefix: false,
                    },
                };
            } else if self.at("{")
                || (identifier(&self.token().text)
                    && self
                        .tokens
                        .get(self.pos + 1)
                        .is_some_and(|token| token.text == "@")
                    && self
                        .tokens
                        .get(self.pos + 2)
                        .is_some_and(|token| token.text == "{"))
            {
                let lambda = if self.at("{") {
                    self.lambda()?
                } else {
                    let label_offset = self.token().offset;
                    let label = self.name()?;
                    self.expect("@")?;
                    Expr {
                        offset: label_offset,
                        kind: ExprKind::Labeled(label, Box::new(self.lambda()?)),
                    }
                };
                expr = match expr.kind {
                    ExprKind::Call(target, mut arguments) => {
                        arguments.push(lambda);
                        Expr {
                            offset,
                            kind: ExprKind::Call(target, arguments),
                        }
                    }
                    kind => Expr {
                        offset,
                        kind: ExprKind::Call(Box::new(Expr { offset, kind }), vec![lambda]),
                    },
                };
            } else if self.take("?.") {
                let name = self.name()?;
                expr = Expr {
                    offset,
                    kind: ExprKind::SafeProperty(Box::new(expr), name),
                };
            } else if self.take(".") {
                let name = self.name()?;
                expr = Expr {
                    offset,
                    kind: ExprKind::Property(Box::new(expr), name),
                };
            } else if self.take("[") {
                self.lines();
                let index = self.expr(0)?;
                self.lines();
                let mut indices = vec![index];
                while self.take(",") {
                    self.lines();
                    indices.push(self.expr(0)?);
                    self.lines();
                }
                self.expect("]")?;
                expr = Expr {
                    offset,
                    kind: if indices.len() == 1 {
                        ExprKind::Index(Box::new(expr), Box::new(indices.remove(0)))
                    } else {
                        ExprKind::Indices(Box::new(expr), indices)
                    },
                };
            } else if self.take("(") {
                self.lines();
                let mut args = Vec::new();
                if !self.at(")") {
                    loop {
                        let argument = if self
                            .tokens
                            .get(self.pos + 1)
                            .is_some_and(|token| token.text == "=")
                        {
                            let offset = self.token().offset;
                            let name = self.name()?;
                            self.expect("=")?;
                            self.lines();
                            Expr {
                                offset,
                                kind: ExprKind::NamedArgument(name, Box::new(self.expr(0)?)),
                            }
                        } else {
                            self.expr(0)?
                        };
                        args.push(argument);
                        self.lines();
                        if !self.take(",") {
                            break;
                        }
                        self.lines();
                        if self.at(")") {
                            break;
                        }
                    }
                }
                self.expect(")")?;
                expr = Expr {
                    offset,
                    kind: ExprKind::Call(Box::new(expr), args),
                };
            } else {
                break;
            }
            expr = Self::bounded(Self::implicit_lambda_labels(expr))?;
        }
        Self::bounded(expr)
    }
    fn implicit_lambda_labels(mut expression: Expr) -> Expr {
        fn call_name(expression: &Expr) -> Option<&str> {
            match &expression.kind {
                ExprKind::Name(name)
                | ExprKind::Property(_, name)
                | ExprKind::SafeProperty(_, name) => Some(name),
                ExprKind::Generic(target, _) => call_name(target),
                _ => None,
            }
        }
        fn needs_label(expression: &Expr, label: &str) -> bool {
            let mut pending = vec![(expression, 0)];
            while let Some((value, depth)) = pending.pop() {
                match &value.kind {
                    ExprKind::LabeledReturn(name, _) if name == label => return true,
                    ExprKind::Labeled(name, _) if name == label => {}
                    ExprKind::Block(body) | ExprKind::LambdaWithParams { body, .. } => {
                        if Parser::statement_expressions(body, &mut pending, depth, true).is_err() {
                            return false;
                        }
                    }
                    ExprKind::Try {
                        body,
                        catches,
                        finally,
                    } => {
                        if Parser::statement_expressions(body, &mut pending, depth, true).is_err() {
                            return false;
                        }
                        for catch in catches {
                            if Parser::statement_expressions(&catch.body, &mut pending, depth, true)
                                .is_err()
                            {
                                return false;
                            }
                        }
                        if Parser::statement_expressions(finally, &mut pending, depth, true)
                            .is_err()
                        {
                            return false;
                        }
                    }
                    ExprKind::Labeled(_, value)
                    | ExprKind::LabeledReturn(_, Some(value))
                    | ExprKind::Return(Some(value))
                    | ExprKind::Throw(value)
                    | ExprKind::NamedArgument(_, value)
                    | ExprKind::Generic(value, _)
                    | ExprKind::Unary(_, value)
                    | ExprKind::Property(value, _)
                    | ExprKind::SafeProperty(value, _)
                    | ExprKind::NonNull(value)
                    | ExprKind::Lambda(value)
                    | ExprKind::Increment { value, .. }
                    | ExprKind::Decrement { value, .. }
                    | ExprKind::TypeOperation(_, value, _) => pending.push((value, depth)),
                    ExprKind::If(condition, yes, no) => {
                        pending.push((condition, depth));
                        pending.push((yes, depth));
                        pending.push((no, depth));
                    }
                    ExprKind::Binary(_, left, right) | ExprKind::Index(left, right) => {
                        pending.push((left, depth));
                        pending.push((right, depth));
                    }
                    ExprKind::Call(target, arguments) | ExprKind::Indices(target, arguments) => {
                        pending.push((target, depth));
                        pending.extend(arguments.iter().map(|argument| (argument, depth)));
                    }
                    ExprKind::Template(parts) => {
                        pending.extend(parts.iter().map(|part| (part, depth)))
                    }
                    ExprKind::When(subject, branches, otherwise)
                    | ExprKind::WhenBinding(_, subject, branches, otherwise) => {
                        pending.push((subject, depth));
                        pending.push((otherwise, depth));
                        for (conditions, value) in branches {
                            pending.extend(conditions.iter().map(|condition| (condition, depth)));
                            pending.push((value, depth));
                        }
                    }
                    ExprKind::CallableReference {
                        receiver: Some(value),
                        ..
                    } => pending.push((value, depth)),
                    _ => {}
                }
            }
            false
        }
        if let ExprKind::Call(target, arguments) = &mut expression.kind {
            if let Some(label) = call_name(target) {
                for argument in arguments {
                    let value = if let ExprKind::NamedArgument(_, value) = &mut argument.kind {
                        value.as_mut()
                    } else {
                        argument
                    };
                    if matches!(
                        value.kind,
                        ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
                    ) && needs_label(value, label)
                    {
                        let offset = value.offset;
                        let lambda = std::mem::replace(
                            value,
                            Expr {
                                offset,
                                kind: ExprKind::Null,
                            },
                        );
                        *value = Expr {
                            offset,
                            kind: ExprKind::Labeled(label.into(), Box::new(lambda)),
                        };
                    }
                }
            }
        }
        expression
    }
    fn when_condition(&mut self, subjectless: bool) -> Result<Expr> {
        let offset = self.token().offset;
        let type_operator = if self.take("is") {
            Some("is")
        } else if self.at("!")
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.text == "is" && token.offset == self.token().offset + 1)
        {
            self.pos += 2;
            Some("!is")
        } else {
            None
        };
        if let Some(operator) = type_operator {
            if subjectless {
                return Err(Diagnostic::new(
                    offset,
                    "Type conditions require a when subject",
                ));
            }
            self.lines();
            return Ok(Expr {
                offset,
                kind: ExprKind::TypeOperation(
                    operator.into(),
                    Box::new(Expr {
                        offset,
                        kind: ExprKind::Name("$whenSubject".into()),
                    }),
                    self.ty()?,
                ),
            });
        }
        let operator = if self.take("in") {
            Some("in")
        } else if self.at("!")
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.text == "in" && token.offset == self.token().offset + 1)
        {
            self.pos += 2;
            Some("!in")
        } else {
            None
        };
        if let Some(operator) = operator {
            if subjectless {
                return Err(Diagnostic::new(
                    offset,
                    "Range conditions require a when subject",
                ));
            }
            self.lines();
            Ok(Expr {
                offset,
                kind: ExprKind::Unary(operator.into(), Box::new(self.expr(0)?)),
            })
        } else {
            self.expr(0)
        }
    }
    fn when_expression(&mut self, offset: usize) -> Result<Expr> {
        let subjectless = !self.at("(");
        let mut binding = None;
        let mut binding_type = None;
        let subject = if self.take("(") {
            self.lines();
            if self.take("val") {
                let name = self.declaration_name()?;
                if self.take(":") {
                    binding_type = Some(self.ty()?);
                }
                self.expect("=")?;
                self.lines();
                binding = Some(name);
            }
            let subject = self.expr(0)?;
            let subject = if let Some(ty) = binding_type {
                let temporary = format!("$when{offset}");
                Expr {
                    offset: subject.offset,
                    kind: ExprKind::Block(vec![
                        Statement {
                            offset: subject.offset,
                            kind: StatementKind::Let(temporary.clone(), false, Some(ty), subject),
                        },
                        Statement {
                            offset,
                            kind: StatementKind::Expression(Expr {
                                offset,
                                kind: ExprKind::Name(temporary),
                            }),
                        },
                    ]),
                }
            } else {
                subject
            };
            self.lines();
            self.expect(")")?;
            subject
        } else {
            Expr {
                offset,
                kind: ExprKind::Boolean(true),
            }
        };
        self.lines();
        self.expect("{")?;
        self.separators();
        let mut branches = Vec::new();
        let mut otherwise = None;
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated when expression"));
            }
            if otherwise.is_some() {
                return Err(self.error("The else branch must be unique and last"));
            }
            let is_else = self.take("else");
            let mut conditions = Vec::new();
            if !is_else {
                loop {
                    conditions.push(self.when_condition(subjectless)?);
                    if conditions.len() > 128 {
                        return Err(self
                            .error("When expression condition nesting exceeds the current limit"));
                    }
                    self.lines();
                    if !self.take(",") {
                        break;
                    }
                    if subjectless {
                        return Err(
                            self.error("Subjectless when conditions cannot be comma-separated")
                        );
                    }
                    self.lines();
                }
            }
            self.expect("->")?;
            self.lines();
            let value = self.expression_branch()?;
            if is_else {
                otherwise = Some(value);
            } else {
                branches.push((conditions, value));
                if branches.len() > 128 {
                    return Err(
                        self.error("When expression branch nesting exceeds the current limit")
                    );
                }
            }
            self.end()?;
        }
        self.expect("}")?;
        let otherwise = otherwise.ok_or_else(|| {
            Diagnostic::new(offset, "When expressions require an explicit else branch")
        })?;
        Ok(Expr {
            offset,
            kind: if let Some(name) = binding {
                ExprKind::WhenBinding(name, Box::new(subject), branches, Box::new(otherwise))
            } else {
                ExprKind::When(Box::new(subject), branches, Box::new(otherwise))
            },
        })
    }
    fn call_type_arguments(&mut self) -> Option<Vec<Type>> {
        let position = self.pos;
        let depth = self.depth;
        let result = self.type_arguments().ok().and_then(|arguments| {
            if !self.at("(") && !self.at("{") {
                return None;
            }
            arguments
                .into_iter()
                .map(|argument| {
                    if argument.variance == Variance::Invariant {
                        argument.ty
                    } else {
                        None
                    }
                })
                .collect()
        });
        if result.is_none() {
            self.pos = position;
            self.depth = depth;
        }
        result
    }
    fn destructuring_names(&mut self) -> Result<Vec<Option<String>>> {
        self.expect("(")?;
        self.lines();
        let mut names = Vec::new();
        loop {
            let name = if self.take("_") {
                None
            } else {
                Some(self.declaration_name()?)
            };
            names.push(name);
            self.lines();
            if !self.take(",") {
                break;
            }
            self.lines();
            if self.at(")") {
                break;
            }
        }
        self.expect(")")?;
        Ok(names)
    }
    fn lambda(&mut self) -> Result<Expr> {
        let offset = self.token().offset;
        self.expect("{")?;
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.error("Lambda nesting exceeds the current limit"));
        }
        self.separators();
        let start = self.pos;
        let depth = self.depth;
        let mut destructures = Vec::new();
        let params = if self.take("->") {
            Some(Vec::new())
        } else {
            let mut parameters = Vec::new();
            let parsed = (|| -> Result<()> {
                loop {
                    let parameter_offset = self.token().offset;
                    let name = if self.at("(") {
                        let names = self.destructuring_names()?;
                        let name = format!("$destructure{parameter_offset}");
                        destructures.push(Statement {
                            offset: parameter_offset,
                            kind: StatementKind::Destructure(
                                names,
                                false,
                                Expr {
                                    offset: parameter_offset,
                                    kind: ExprKind::Name(name.clone()),
                                },
                            ),
                        });
                        name
                    } else if self.take("_") {
                        "_".into()
                    } else {
                        self.name()?
                    };
                    let ty = if self.take(":") {
                        self.lines();
                        Some(self.ty()?)
                    } else {
                        None
                    };
                    parameters.push(LambdaParameter { name, ty });
                    self.lines();
                    if !self.take(",") {
                        break;
                    }
                    self.lines();
                }
                self.expect("->")
            })();
            if parsed.is_ok() {
                Some(parameters)
            } else {
                self.pos = start;
                self.depth = depth;
                destructures.clear();
                None
            }
        };
        self.separators();
        let mut body = destructures;
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated lambda"));
            }
            body.push(self.statement()?);
            self.end()?;
        }
        self.expect("}")?;
        self.depth -= 1;
        if params.is_none()
            && body.len() == 1
            && matches!(body[0].kind, StatementKind::Expression(_))
        {
            let StatementKind::Expression(value) = body.remove(0).kind else {
                unreachable!()
            };
            return Ok(Expr {
                offset,
                kind: ExprKind::Lambda(Box::new(value)),
            });
        }
        Ok(Expr {
            offset,
            kind: ExprKind::LambdaWithParams { params, body },
        })
    }
    fn expression_branch(&mut self) -> Result<Expr> {
        if self.at("{") {
            let offset = self.token().offset;
            let mut statements = self.block()?;
            if statements.len() == 1 && matches!(statements[0].kind, StatementKind::Expression(_)) {
                let StatementKind::Expression(value) = statements.remove(0).kind else {
                    unreachable!()
                };
                Ok(value)
            } else {
                Ok(Expr {
                    offset,
                    kind: ExprKind::Block(statements),
                })
            }
        } else {
            self.expr(0)
        }
    }
    fn expression_body(&mut self) -> Result<Expr> {
        Self::validate_expression(self.expr(0)?, false)
    }
    fn bounded(expr: Expr) -> Result<Expr> {
        Self::validate_expression(expr, true)
    }
    fn statement_expressions<'a>(
        statements: &'a [Statement],
        pending: &mut Vec<(&'a Expr, usize)>,
        depth: usize,
        allow_return: bool,
    ) -> Result<()> {
        if depth > 128 {
            return Err(Diagnostic::new(
                statements.first().map_or(0, |statement| statement.offset),
                "Expression nesting exceeds the current limit",
            ));
        }
        for statement in statements {
            if !allow_return && matches!(statement.kind, StatementKind::Return(_)) {
                return Err(Diagnostic::new(
                    statement.offset,
                    "Returns are not allowed in expression-bodied functions or getters",
                ));
            }
            match &statement.kind {
                StatementKind::Block(body) => {
                    Self::statement_expressions(body, pending, depth + 1, allow_return)?
                }
                StatementKind::Labeled(_, statement) => Self::statement_expressions(
                    std::slice::from_ref(statement),
                    pending,
                    depth + 1,
                    allow_return,
                )?,
                StatementKind::Let(_, _, _, value)
                | StatementKind::Assign(_, value)
                | StatementKind::AddAssign(_, value)
                | StatementKind::CompoundAssign(_, _, value)
                | StatementKind::Return(Some(value))
                | StatementKind::Throw(value)
                | StatementKind::Expression(value)
                | StatementKind::Destructure(_, _, value) => pending.push((value, depth)),
                StatementKind::IndexAssign(target, index, value) => {
                    pending.push((target, depth));
                    pending.push((index, depth));
                    pending.push((value, depth));
                }
                StatementKind::IndicesAssign(target, indices, value) => {
                    pending.push((target, depth));
                    pending.extend(indices.iter().map(|index| (index, depth)));
                    pending.push((value, depth));
                }
                StatementKind::PropertyAssign(target, _, value) => {
                    pending.push((target, depth));
                    pending.push((value, depth));
                }
                StatementKind::ForUntil(_, from, to, body) => {
                    pending.push((from, depth));
                    pending.push((to, depth));
                    Self::statement_expressions(body, pending, depth + 1, allow_return)?;
                }
                StatementKind::ForEach(_, value, body)
                | StatementKind::While(value, body)
                | StatementKind::DoWhile(value, body) => {
                    pending.push((value, depth));
                    Self::statement_expressions(body, pending, depth + 1, allow_return)?;
                }
                StatementKind::If(condition, yes, no) => {
                    pending.push((condition, depth));
                    Self::statement_expressions(yes, pending, depth + 1, allow_return)?;
                    Self::statement_expressions(no, pending, depth + 1, allow_return)?;
                }
                StatementKind::When(subject, branches, otherwise)
                | StatementKind::WhenBinding(_, subject, branches, otherwise) => {
                    pending.push((subject, depth));
                    for (conditions, body) in branches {
                        pending.extend(conditions.iter().map(|condition| (condition, depth)));
                        Self::statement_expressions(body, pending, depth + 1, allow_return)?;
                    }
                    if let Some(body) = otherwise {
                        Self::statement_expressions(body, pending, depth + 1, allow_return)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn validate_expression(expr: Expr, allow_return: bool) -> Result<Expr> {
        let mut pending = vec![(&expr, 1)];
        while let Some((value, depth)) = pending.pop() {
            if !allow_return && matches!(value.kind, ExprKind::Return(_)) {
                return Err(Diagnostic::new(
                    value.offset,
                    "Returns are not allowed in expression-bodied functions or getters",
                ));
            }
            if depth > 128 {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Expression nesting exceeds the current limit",
                ));
            }
            match &value.kind {
                ExprKind::Block(body) => {
                    Self::statement_expressions(body, &mut pending, depth + 1, allow_return)?
                }
                ExprKind::Try {
                    body,
                    catches,
                    finally,
                } => {
                    Self::statement_expressions(body, &mut pending, depth + 1, allow_return)?;
                    for catch in catches {
                        Self::statement_expressions(
                            &catch.body,
                            &mut pending,
                            depth + 1,
                            allow_return,
                        )?;
                    }
                    Self::statement_expressions(finally, &mut pending, depth + 1, allow_return)?;
                }
                ExprKind::Labeled(_, value) | ExprKind::LabeledReturn(_, Some(value)) => {
                    pending.push((value, depth + 1))
                }
                ExprKind::Indices(target, indices) => {
                    pending.push((target, depth + 1));
                    pending.extend(indices.iter().map(|index| (index, depth + 1)));
                }
                ExprKind::WhenBinding(_, subject, branches, otherwise) => {
                    pending.push((subject, depth + 1));
                    pending.push((otherwise, depth + 1));
                    for (conditions, value) in branches {
                        pending.extend(conditions.iter().map(|condition| (condition, depth + 1)));
                        pending.push((value, depth + 1));
                    }
                }
                ExprKind::If(condition, yes, no) => {
                    pending.push((condition, depth + 1));
                    pending.push((yes, depth + 1));
                    pending.push((no, depth + 1));
                }
                ExprKind::CallableReference {
                    receiver: Some(receiver),
                    ..
                } => pending.push((receiver, depth + 1)),
                ExprKind::Template(parts) => {
                    pending.extend(parts.iter().map(|part| (part, depth + 1)))
                }
                ExprKind::When(subject, branches, otherwise) => {
                    pending.push((subject, depth + 1));
                    pending.push((otherwise, depth + 1));
                    for (conditions, value) in branches {
                        pending.extend(conditions.iter().map(|condition| (condition, depth + 1)));
                        pending.push((value, depth + 1));
                    }
                }
                ExprKind::Return(Some(value))
                | ExprKind::Throw(value)
                | ExprKind::NamedArgument(_, value)
                | ExprKind::Generic(value, _)
                | ExprKind::Increment { value, .. }
                | ExprKind::Decrement { value, .. }
                | ExprKind::TypeOperation(_, value, _)
                | ExprKind::NonNull(value)
                | ExprKind::Lambda(value) => pending.push((value, depth + 1)),
                ExprKind::Unary(_, a) | ExprKind::Property(a, _) | ExprKind::SafeProperty(a, _) => {
                    pending.push((a, depth + 1))
                }
                ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                    pending.push((a, depth + 1));
                    pending.push((b, depth + 1));
                }
                ExprKind::Call(target, args) => {
                    pending.push((target, depth + 1));
                    pending.extend(args.iter().map(|v| (v, depth + 1)));
                }
                _ => {}
            }
        }
        Ok(expr)
    }
}
fn numeric_literal(text: &str, offset: usize) -> Result<ExprKind> {
    let radix_literal = text.starts_with("0x")
        || text.starts_with("0X")
        || text.starts_with("0b")
        || text.starts_with("0B");
    if !radix_literal && (text.ends_with('f') || text.ends_with('F')) {
        let digits = &text[..text.len() - 1];
        let bytes = digits.as_bytes();
        if bytes.is_empty()
            || bytes.iter().enumerate().any(|(index, byte)| {
                *byte == b'_'
                    && (index == 0
                        || index + 1 == bytes.len()
                        || !bytes[index - 1].is_ascii_digit()
                        || !bytes[index + 1].is_ascii_digit())
            })
        {
            return Err(Diagnostic::new(
                offset,
                "Float literals require separators between decimal digits",
            ));
        }
        let value: f32 = digits
            .replace('_', "")
            .parse()
            .map_err(|_| Diagnostic::new(offset, "Invalid Float literal"))?;
        if !value.is_finite() {
            return Err(Diagnostic::new(offset, "Float literal is out of range"));
        }
        return Ok(ExprKind::Float(value.to_bits()));
    }
    if let Some(digits) = text.strip_suffix('L') {
        let (digits, radix) = if let Some(hex) = digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
        {
            (hex, 16)
        } else if let Some(binary) = digits
            .strip_prefix("0b")
            .or_else(|| digits.strip_prefix("0B"))
        {
            (binary, 2)
        } else {
            (digits, 10)
        };
        if digits.is_empty()
            || digits.starts_with('_')
            || digits.ends_with('_')
            || !digits.chars().all(|c| c == '_' || c.is_digit(radix))
        {
            return Err(Diagnostic::new(
                offset,
                "Long literals require digits with optional separators between digits",
            ));
        }
        let normalized = digits.replace('_', "");
        if radix == 10 && normalized.len() > 1 && normalized.starts_with('0') {
            return Err(Diagnostic::new(
                offset,
                "Leading zeroes are not allowed in decimal literals",
            ));
        }
        let value = i128::from_str_radix(&normalized, radix)
            .map_err(|_| Diagnostic::new(offset, "Long literal is out of range"))?;
        return Ok(ExprKind::Long(value));
    }
    if !radix_literal
        && (text.contains('.')
            || text.contains('e')
            || text.contains('E')
            || text.ends_with('d')
            || text.ends_with('D'))
    {
        return Err(Diagnostic::new(
            offset,
            "Double literals are not supported; use a Float suffix",
        ));
    }
    Ok(ExprKind::Int(integer_literal(text, offset)?))
}
fn integer_literal(text: &str, offset: usize) -> Result<i64> {
    let (digits, radix) =
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            (hex, 16)
        } else if let Some(binary) = text.strip_prefix("0b").or_else(|| text.strip_prefix("0B")) {
            (binary, 2)
        } else {
            (text, 10)
        };
    if digits.is_empty()
        || digits.starts_with('_')
        || digits.ends_with('_')
        || !digits.chars().all(|c| c == '_' || c.is_digit(radix))
    {
        return Err(Diagnostic::new(
            offset,
            "Integer literals require valid digits with separators between digits",
        ));
    }
    let normalized = digits.replace('_', "");
    if radix == 10 && normalized.len() > 1 && normalized.starts_with('0') {
        return Err(Diagnostic::new(
            offset,
            "Leading zeroes are not allowed in decimal literals",
        ));
    }
    i64::from_str_radix(&normalized, radix)
        .map_err(|_| Diagnostic::new(offset, "Integer literal is out of range"))
}

fn identifier(text: &str) -> bool {
    text.as_bytes()
        .first()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || *byte == b'_')
        && text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !matches!(
            text,
            "fun"
                | "val"
                | "var"
                | "if"
                | "else"
                | "while"
                | "return"
                | "true"
                | "false"
                | "package"
                | "private"
                | "public"
                | "class"
                | "object"
                | "null"
                | "when"
                | "for"
                | "break"
                | "continue"
                | "throw"
                | "try"
                | "is"
                | "in"
                | "as"
                | "this"
                | "super"
                | "interface"
                | "typealias"
                | "typeof"
                | "do"
        )
}

fn precedence(op: &str) -> Option<u8> {
    match op {
        "||" => Some(1),
        "&&" => Some(2),
        "==" | "!=" | "===" | "!==" => Some(3),
        "<" | ">" | "<=" | ">=" => Some(4),
        "in" | "!in" | "is" | "!is" => Some(5),
        "?:" => Some(6),
        ".." => Some(8),
        "+" | "-" => Some(9),
        "*" | "/" | "%" => Some(10),
        "as" => Some(11),
        _ if identifier(op) => Some(7),
        _ => None,
    }
}
pub fn parse(source: &str) -> Result<Module> {
    let mut p = Parser {
        tokens: lex(source)?,
        pos: 0,
        depth: 0,
        type_parameters: Vec::new(),
    };
    p.separators();
    let mut package = String::new();
    if p.take("package") {
        package = p.name()?;
        while p.take(".") {
            package.push('.');
            package.push_str(&p.name()?);
        }
        p.end()?;
    }
    let mut imports = Vec::new();
    while p.take("import") {
        let offset = p.token().offset;
        let mut name = p.name()?;
        let mut alias = name.clone();
        while p.take(".") {
            alias = if p.take("*") { "*".into() } else { p.name()? };
            name.push('/');
            name.push_str(&alias);
            if alias == "*" {
                break;
            }
        }
        if p.take("as") {
            if alias == "*" {
                return Err(p.error("Wildcard imports cannot be aliased"));
            }
            alias = p.name()?;
        }
        p.end()?;
        imports.push((alias, name, offset));
    }
    let mut functions = Vec::new();
    let mut constants = Vec::new();
    let mut properties = Vec::new();
    let mut enum_decl = None;
    let mut object_decl = None;
    let mut class_decl = None;
    let mut interface_decl = None;
    let mut declarations = Vec::new();
    while !p.at("<eof>") {
        if p.starts_declaration("const") {
            constants.push(p.constant()?);
        } else if p.starts_declaration("val") || p.starts_declaration("var") {
            let offset = p.token().offset;
            let private = p.take("private");
            if !private {
                p.take("public");
                p.take("internal");
            }
            p.lines();
            let lateinit = p.take("lateinit");
            p.lines();
            properties.push(p.property(offset, private, false, lateinit, false)?);
        } else if p.starts_enum() {
            let declaration = p.enum_decl()?;
            if enum_decl.is_none() {
                enum_decl = Some(declaration);
            } else {
                declarations.push(Declaration::Enum(declaration));
            }
        } else if p.starts_object() {
            let declaration = p.object_decl(1, &mut 0)?;
            if object_decl.is_none() {
                object_decl = Some(declaration);
            } else {
                declarations.push(Declaration::Object(declaration));
            }
        } else if p.starts_class() {
            let declaration = p.class_decl()?;
            if class_decl.is_none() {
                class_decl = Some(declaration);
            } else {
                declarations.push(Declaration::Class(declaration));
            }
        } else if p.starts_interface() {
            let declaration = p.interface_decl(1, &mut 0)?;
            if interface_decl.is_none() {
                interface_decl = Some(declaration);
            } else {
                declarations.push(Declaration::Interface(declaration));
            }
        } else {
            functions.push(p.function()?);
        }
        p.end()?;
    }
    if functions.is_empty()
        && constants.is_empty()
        && properties.is_empty()
        && enum_decl.is_none()
        && object_decl.is_none()
        && class_decl.is_none()
        && interface_decl.is_none()
    {
        return Err(p.error("Expected at least one function, enum class, object, or class"));
    }
    Ok(Module {
        package,
        imports,
        functions,
        constants,
        properties,
        enum_decl,
        object_decl,
        class_decl,
        interface_decl,
        declarations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_function_types_and_projections_preserve_structure() {
        let module = parse("import demo.Box\nfun <T: Any> identity(value: T, callback: ((T) -> String)?): Box<out T> = make<T>(value)\nfun projections(value: Box<*>, sink: Box<in String>): Unit {}\nfun <T> List<T>.head(): T = get(0)").unwrap();
        let function = &module.functions[0];
        assert_eq!(function.type_params.len(), 1);
        assert_eq!(
            function.type_params[0].bounds,
            vec![Type::Java("java/lang/Object".into())]
        );
        assert!(matches!(&function.params[0].1, Type::Parameter { name, .. } if name == "T"));
        assert!(
            matches!(&function.params[1].1, Type::Nullable(inner) if matches!(inner.as_ref(), Type::Function { params, result } if params.len() == 1 && **result == Type::String))
        );
        assert!(
            matches!(&function.result, Type::Parameterized { name, arguments } if name == "Box" && arguments[0].variance == Variance::Out)
        );
        assert!(
            matches!(&module.functions[1].params[0].1, Type::Parameterized { arguments, .. } if arguments[0].ty.is_none())
        );
        assert!(
            matches!(&module.functions[1].params[1].1, Type::Parameterized { arguments, .. } if arguments[0].variance == Variance::In)
        );
        assert!(
            matches!(&module.functions[2].receiver, Some(Type::List(inner)) if matches!(inner.as_ref(), Type::Parameter { name, .. } if name == "T"))
        );
        assert!(
            matches!(&function.body[0].kind, StatementKind::Return(Some(Expr { kind: ExprKind::Call(target, _), .. })) if matches!(&target.kind, ExprKind::Generic(_, arguments) if matches!(&arguments[0], Type::Parameter { name, .. } if name == "T")))
        );
    }

    #[test]
    fn lambdas_callable_references_and_comparisons_remain_distinct() {
        let module = parse("fun f(x: Int): Unit {\nval implicit = { it + 1 }\nval explicit = { x: Int, _ -> val next = x + 1; next }\nval zero = { -> 1 }\nval statements = { val next = 1; next }\nconsume(::top, String::length, value::method)\nval less = x < 3\nval generic = api.call<String>(\"ok\")\n}").unwrap();
        let body = &module.functions[0].body;
        assert!(matches!(
            &body[0].kind,
            StatementKind::Let(
                _,
                _,
                _,
                Expr {
                    kind: ExprKind::Lambda(_),
                    ..
                }
            )
        ));
        assert!(
            matches!(&body[1].kind, StatementKind::Let(_, _, _, Expr { kind: ExprKind::LambdaWithParams { params: Some(params), body }, .. }) if params.len() == 2 && params[0].ty == Some(Type::Int) && body.len() == 2)
        );
        assert!(
            matches!(&body[2].kind, StatementKind::Let(_, _, _, Expr { kind: ExprKind::LambdaWithParams { params: Some(params), .. }, .. }) if params.is_empty())
        );
        assert!(
            matches!(&body[3].kind, StatementKind::Let(_, _, _, Expr { kind: ExprKind::LambdaWithParams { params: None, body }, .. }) if body.len() == 2)
        );
        assert!(
            matches!(&body[4].kind, StatementKind::Expression(Expr { kind: ExprKind::Call(_, arguments), .. }) if arguments.iter().all(|argument| matches!(argument.kind, ExprKind::CallableReference { .. })))
        );
        assert!(
            matches!(&body[5].kind, StatementKind::Let(_, _, _, Expr { kind: ExprKind::Binary(operator, _, _), .. }) if operator == "<")
        );
        assert!(
            matches!(&body[6].kind, StatementKind::Let(_, _, _, Expr { kind: ExprKind::Call(target, _), .. }) if matches!(target.kind, ExprKind::Generic(_, _)))
        );
    }

    #[test]
    fn unsupported_generic_modifiers_and_function_arities_are_diagnosed() {
        for source in [
            "fun <reified T> f(value: T): T = value",
            "fun <out T> f(value: T): T = value",
            "fun <T: Long> f(value: T): T = value",
            "fun <T: Int?> f(value: T): T = value",
            "fun <T: Unit> f(value: T): T = value",
            "fun <T> f(value: T): T where T: Any = value",
            "fun f(value: suspend () -> Unit): Unit {}",
            "fun f(value: List<in String>): Unit {}",
            "fun f(value: Comparable<out String>): Unit {}",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
        let source = format!(
            "fun f(value: ({}) -> Int): Unit {{}}",
            vec!["Int"; 23].join(",")
        );
        assert!(parse(&source).unwrap_err().message.contains("22"));
        let parameter = Type::Parameter {
            name: "T".into(),
            upper_bound: Box::new(Type::Int),
        };
        assert_eq!(parameter.descriptor(), "Ljava/lang/Integer;");
    }

    #[test]
    fn nullable_primitives_use_reference_descriptors_and_slots() {
        for (ty, descriptor) in [
            (Type::Int, "Ljava/lang/Integer;"),
            (Type::Long, "Ljava/lang/Long;"),
            (Type::Byte, "Ljava/lang/Byte;"),
            (Type::Char, "Ljava/lang/Character;"),
            (Type::Boolean, "Ljava/lang/Boolean;"),
            (Type::Float, "Ljava/lang/Float;"),
        ] {
            let nullable = Type::Nullable(Box::new(ty));
            assert!(nullable.is_reference());
            assert_eq!(nullable.slots(), 1);
            assert_eq!(nullable.descriptor(), descriptor);
        }
    }

    #[test]
    fn constructor_parameters_distinguish_properties_and_initializers() {
        let module = parse(
            "class C(seed: Int, var value: Int = 1, private val hidden: Long = 2L) {\n\
             var doubled = seed * 2\n\
             private val xs: MutableList<Int?> = mutableListOf<Int?>()\n\
             val computed: Int get() = value\n\
             companion object { val IDLE = C(seed = 0) }\n\
             }",
        )
        .unwrap();
        let class = module.class_decl.unwrap();
        assert_eq!(class.param_properties, ["value", "hidden"]);
        assert_eq!(class.mutable_params, ["value"]);
        assert_eq!(class.public_params, ["value"]);
        assert_eq!(class.defaults.len(), class.params.len());
        assert!(class.defaults[0].is_none());
        assert!(class.defaults[1].is_some());
        assert!(class.properties[0].inferred);
        assert!(class.properties[0].initializer.is_some());
        assert!(class.properties[0].body.is_empty());
        assert!(class.properties[1].private);
        assert_eq!(
            class.properties[1].ty,
            Type::MutableList(Box::new(Type::Nullable(Box::new(Type::Int))))
        );
        assert!(class.properties[2].initializer.is_none());
        assert!(!class.properties[2].body.is_empty());
        assert_eq!(class.companion.unwrap().properties[0].name, "IDLE");
        assert!(parse("class Implicit { val x = 1 }").is_ok());
        assert!(parse("class Empty").is_ok());
        assert!(parse("data class D(val x: Int = 1) { val y: Int get() = x }").is_ok());
    }

    #[test]
    fn scalar_constants_and_float_exponents_preserve_values() {
        let module = parse(
            "private const val answer = 42\n\
             internal const val count = -9223372036854775808L\n\
             const val scale = -1.25e+2F\n\
             fun fraction(): Float = 1e-2f\n\
             fun range(x: Int): Boolean = x in 1..3",
        )
        .unwrap();
        assert!(module.constants[0].private);
        assert!(module.constants[1].internal);
        assert!(matches!(
            module.constants[1].value,
            ConstValue::Long(i64::MIN)
        ));
        assert!(
            matches!(module.constants[2].value, ConstValue::Float(bits) if bits == (-125.0f32).to_bits())
        );
        assert!(matches!(
            &module.functions[0].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Float(bits), .. })) if *bits == 0.01f32.to_bits()
        ));
        assert!(parse("fun f(): Float = 1.5").is_err());
        assert!(parse("fun f(): Float = 1e100f").is_err());
        assert!(parse("fun f(): Float = 1_.0f").is_err());
    }

    #[test]
    fn terminal_elvis_named_defaults_and_generic_factory_parse() {
        let module = parse(
            "fun fallback(x: String?): String { return x ?: return \"missing\" }\n\
             fun failure(x: String?): String = x ?: throw IllegalArgumentException(\"missing\")\n\
             fun f(x: Int = 1, y: Int = x + 1): Int = g(y = y, x = x)\n\
             fun list(): List<Int> = emptyList<Int>()",
        )
        .unwrap();
        assert!(matches!(
            &module.functions[0].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Binary(operator, _, right), .. }))
                if operator == "?:" && matches!(right.kind, ExprKind::Return(Some(_)))
        ));
        assert!(matches!(
            &module.functions[1].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Binary(_, _, right), .. }))
                if matches!(right.kind, ExprKind::Throw(_))
        ));
        assert!(module.functions[2].defaults.iter().all(Option::is_some));
        assert!(matches!(
            &module.functions[2].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Call(_, args), .. }))
                if matches!(&args[0].kind, ExprKind::NamedArgument(name, _) if name == "y")
        ));
        assert!(matches!(
            &module.functions[3].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Call(target, _), .. }))
                if matches!(&target.kind, ExprKind::Generic(_, types) if types == &[Type::Int])
        ));
    }

    #[test]
    fn expression_bodies_reject_explicit_returns() {
        for text in [
            "fun f(x: String?): String = x ?: return \"missing\"",
            "fun f(x: String?): String = if (true) x ?: return \"missing\" else \"ok\"",
            "class C { fun f(x: String?): String = x ?: return \"missing\" }",
            "object O { fun f(x: String?): String = x ?: return \"missing\" }",
            "interface I { fun f(x: String?): String = x ?: return \"missing\" }",
            "class C(val x: String?) { val y: String get() = x ?: return \"missing\" }",
        ] {
            assert!(parse(text).is_err(), "unexpected acceptance: {text}");
        }
        assert!(parse("fun f(x: String?): String { return x ?: return \"missing\" }").is_ok());
        assert!(
            parse("fun f(x: String?): String = x ?: throw IllegalArgumentException(\"missing\")")
                .is_ok()
        );
    }

    #[test]
    fn property_assignments_preserve_receiver_expressions() {
        let module = parse("fun f(value: Owner): Unit {\nJavaApi.changing = 1\nvalue.mutable = 2\nvalue.child.mutable = 3\n}").unwrap();
        let body = &module.functions[0].body;
        assert!(
            matches!(&body[0].kind, StatementKind::PropertyAssign(Expr { kind: ExprKind::Name(name), .. }, field, _) if name == "JavaApi" && field == "changing")
        );
        assert!(
            matches!(&body[1].kind, StatementKind::PropertyAssign(Expr { kind: ExprKind::Name(name), .. }, field, _) if name == "value" && field == "mutable")
        );
        assert!(
            matches!(&body[2].kind, StatementKind::PropertyAssign(Expr { kind: ExprKind::Property(_, child), .. }, field, _) if child == "child" && field == "mutable")
        );
    }

    #[test]
    fn new_class_forms_preserve_unsupported_boundaries() {
        for text in [
            "class C { companion object Named {} }",
            "fun f(x: Unit?): Unit {}",
        ] {
            assert!(parse(text).is_err(), "unexpected acceptance: {text}");
        }
        let deep = format!(
            "fun f(x: {}Int{}): Unit {{}}",
            "List<".repeat(129),
            ">".repeat(129)
        );
        assert!(parse(&deep).is_err());
    }
    #[test]
    fn foreach_ranges_and_do_while_preserve_structure() {
        let module = parse("fun f(a: IntArray): Unit { for (i in 0 until 4) {}; for (i in 1..4) {}; for (i in 4 downTo 1 step 2) {}; for (x in a) {}; for (i in a.indices) {}; do { continue } while (false) }").unwrap();
        let body = &module.functions[0].body;
        assert!(matches!(body[0].kind, StatementKind::ForUntil(_, _, _, _)));
        assert!(
            matches!(&body[1].kind, StatementKind::ForEach(_, Expr { kind: ExprKind::Binary(op, _, _), .. }, _) if op == "..")
        );
        let StatementKind::ForEach(
            _,
            Expr {
                kind: ExprKind::Binary(step, progression, _),
                ..
            },
            _,
        ) = &body[2].kind
        else {
            panic!("missing stepped progression")
        };
        assert_eq!(step, "step");
        assert!(matches!(&progression.kind, ExprKind::Binary(op, _, _) if op == "downTo"));
        assert!(
            matches!(&body[3].kind, StatementKind::ForEach(_, Expr { kind: ExprKind::Name(name), .. }, _) if name == "a")
        );
        assert!(
            matches!(&body[4].kind, StatementKind::ForEach(_, Expr { kind: ExprKind::Property(_, name), .. }, _) if name == "indices")
        );
        assert!(
            matches!(&body[5].kind, StatementKind::DoWhile(Expr { kind: ExprKind::Boolean(false), .. }, statements) if matches!(statements[0].kind, StatementKind::Continue))
        );
    }

    #[test]
    fn infix_calls_accept_identifiers_and_use_kotlin_precedence() {
        let module =
            parse("fun downTo(step: Int): Int = step\nfun f(): Int = 1 + 2 custom 3..5 step 2")
                .unwrap();
        assert_eq!(module.functions[0].name, "downTo");
        let StatementKind::Return(Some(expression)) = &module.functions[1].body[0].kind else {
            panic!("missing expression body")
        };
        let ExprKind::Binary(step, left, _) = &expression.kind else {
            panic!("missing outer infix")
        };
        assert_eq!(step, "step");
        let ExprKind::Binary(custom, left, right) = &left.kind else {
            panic!("missing custom infix")
        };
        assert_eq!(custom, "custom");
        assert!(matches!(&left.kind, ExprKind::Binary(op, _, _) if op == "+"));
        assert!(matches!(&right.kind, ExprKind::Binary(op, _, _) if op == ".."));
    }

    #[test]
    fn compound_assignments_and_decrements_keep_operators_and_positions() {
        let module =
            parse("fun f(): Int { var n = 4; n -= 1; n *= 2; n /= 2; n %= 3; --n; return n-- }")
                .unwrap();
        let body = &module.functions[0].body;
        for (statement, expected) in body[1..5].iter().zip(["-=", "*=", "/=", "%="]) {
            assert!(
                matches!(&statement.kind, StatementKind::CompoundAssign(name, op, _) if name == "n" && op == expected)
            );
        }
        assert!(matches!(
            &body[5].kind,
            StatementKind::Expression(Expr {
                kind: ExprKind::Decrement { prefix: true, .. },
                ..
            })
        ));
        assert!(matches!(
            &body[6].kind,
            StatementKind::Return(Some(Expr {
                kind: ExprKind::Decrement { prefix: false, .. },
                ..
            }))
        ));
        for target in ["a[0]", "a.value", "(a)"] {
            let source = format!("fun f(a: IntArray): Unit {{ {target} -= 1 }}");
            assert!(
                parse(&source)
                    .unwrap_err()
                    .message
                    .contains("local variable target")
            );
        }
    }

    #[test]
    fn type_operations_identity_and_when_type_conditions_keep_types() {
        let module = parse("fun f(a: Any?, b: Any?): Boolean = a ?: b is String && a !== null\nfun g(a: Any?): String? = a as? String\nfun h(a: Any?): Int = when (a) { is String -> 1; !is Int -> 2; else -> 3 }").unwrap();
        let StatementKind::Return(Some(Expr {
            kind: ExprKind::Binary(and, test, identity),
            ..
        })) = &module.functions[0].body[0].kind
        else {
            panic!("missing conjunction")
        };
        assert_eq!(and, "&&");
        assert!(
            matches!(&test.kind, ExprKind::TypeOperation(op, value, Type::String) if op == "is" && matches!(&value.kind, ExprKind::Binary(op, _, _) if op == "?:"))
        );
        assert!(matches!(&identity.kind, ExprKind::Binary(op, _, _) if op == "!=="));
        assert!(
            matches!(&module.functions[1].body[0].kind, StatementKind::Return(Some(Expr { kind: ExprKind::TypeOperation(op, _, Type::String), .. })) if op == "as?")
        );
        let StatementKind::Return(Some(Expr {
            kind: ExprKind::When(_, branches, _),
            ..
        })) = &module.functions[2].body[0].kind
        else {
            panic!("missing when")
        };
        for ((conditions, _), expected) in branches.iter().zip(["is", "!is"]) {
            assert!(
                matches!(&conditions[0].kind, ExprKind::TypeOperation(op, subject, _) if op == expected && matches!(&subject.kind, ExprKind::Name(name) if name == "$whenSubject"))
            );
        }
        assert!(parse("fun f(a: Any): Int = -(a as Int) * 2").is_ok());
        assert!(parse("fun f(a: Any): Int = a\nas\nInt").is_ok());
        for source in [
            "fun f(a: Any): Boolean = a ! is Int",
            "fun f(a: Any): Int? = a as ? Int",
        ] {
            assert!(parse(source).is_err(), "unexpected acceptance: {source}");
        }
    }
    #[test]
    fn infix_declarations_preserve_modifiers_and_reject_invalid_signatures() {
        let module = parse("infix public fun Int.combine(value: Int): Int = this + value\nprivate infix fun Int.other(value: Int): Int = this - value\nfun ordinary(value: Int): Int = value").unwrap();
        assert!(module.functions[0].infix);
        assert_eq!(module.functions[0].receiver, Some(Type::Int));
        assert!(module.functions[1].infix && module.functions[1].private);
        assert!(!module.functions[2].infix);
        let class = parse("class C { infix public fun combine(value: Int): Int = value }")
            .unwrap()
            .class_decl
            .unwrap();
        assert!(class.methods[0].function.infix);
        let object = parse("object O { public infix fun combine(value: Int): Int = value }")
            .unwrap()
            .object_decl
            .unwrap();
        assert!(object.methods[0].function.infix);
        let interface = parse("interface I { public infix fun combine(value: Int): Int }")
            .unwrap()
            .interface_decl
            .unwrap();
        assert!(interface.methods[0].function.infix);
        let override_class =
            parse("class C { override infix fun combine(value: Int): Int = value }")
                .unwrap()
                .class_decl
                .unwrap();
        assert!(override_class.methods[0].is_override);
        assert!(override_class.methods[0].function.infix);
        for source in [
            "infix infix fun Int.combine(value: Int): Int = value",
            "infix fun Int.combine(): Int = 1",
            "infix fun Int.combine(a: Int, b: Int): Int = a + b",
            "infix fun Int.combine(value: Int = 1): Int = value",
            "class C { infix fun combine(value: Int = 1): Int = value }",
            "object O { infix fun combine(): Int = 1 }",
            "interface I { infix fun combine(a: Int, b: Int): Int }",
            "class C { infix val value = 1 }",
            "interface I { infix val value: Int }",
        ] {
            assert!(parse(source).is_err(), "unexpected acceptance: {source}");
        }
    }
    #[test]
    fn declaration_fixture_sources_parse_without_rewriting() {
        for source in [
            include_str!("../tests/fixtures/declarations_twenty/Literals.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Mixed.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Bodies.kt"),
            include_str!("../tests/fixtures/declarations_twenty/State.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Initializers.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Nested.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Locals.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Membership.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Arrays.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Receivers.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Labels.kt"),
            include_str!("../tests/fixtures/declarations_twenty/TryFlow.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Operators.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Destructure.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Subject.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Accessors.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Delegates.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Lateinit.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Inheritance.kt"),
            include_str!("../tests/fixtures/declarations_twenty/Secondary.kt"),
        ] {
            parse(source).unwrap_or_else(|error| panic!("{error:?}\n{source}"));
        }
    }

    #[test]
    fn integer_radices_and_separators_preserve_signed_magnitudes() {
        for (source, expected) in [
            ("0xff_ff80ab", 0xffff80ab),
            ("0b1010_0101", 165),
            ("1__000", 1000),
        ] {
            assert!(
                matches!(numeric_literal(source, 0).unwrap(), ExprKind::Int(value) if value == expected)
            );
        }
        assert!(matches!(
            numeric_literal("0x1_0000_0000L", 0).unwrap(),
            ExprKind::Long(4294967296)
        ));
        assert!(matches!(
            numeric_literal("9223372036854775808L", 0).unwrap(),
            ExprKind::Long(9223372036854775808)
        ));
        for source in ["0x_ff", "0b_1", "1_", "1_L", "0b12", "00", "1u", "1UL"] {
            assert!(numeric_literal(source, 0).is_err(), "{source}");
        }
        let module = parse("const val long = 0xff_ff80ab\nconst val explicit: Long = 1\nconst val min = -2147483648").unwrap();
        assert!(matches!(
            module.constants[0].value,
            ConstValue::Long(4294934699)
        ));
        assert!(matches!(module.constants[1].value, ConstValue::Long(1)));
        assert!(matches!(
            module.constants[2].value,
            ConstValue::Int(i32::MIN)
        ));
    }

    #[test]
    fn declaration_order_initializers_and_secondary_constructors_are_preserved() {
        let module = parse("fun first() = 1\nclass A() { val x = 1; init { mark(2) }; val y = 3; init { mark(4) }; constructor(n: Int): this() {} }\nclass B { constructor(n: Int) {} }\nobject O {}\ninterface I {}\nenum class E { X }\nval top = 5").unwrap();
        assert!(module.functions[0].result_inferred);
        assert_eq!(module.properties.len(), 1);
        let class = module.class_decl.unwrap();
        assert!(class.primary_constructor);
        assert_eq!(class.initializers.len(), 2);
        assert!(class.properties[0].offset < class.initializers[0].0);
        assert!(class.initializers[0].0 < class.properties[1].offset);
        assert_eq!(class.secondary_constructors[0].delegation, "this");
        assert!(
            matches!(&module.declarations[0], Declaration::Class(class) if !class.primary_constructor)
        );
        assert!(
            module.object_decl.is_some()
                && module.interface_decl.is_some()
                && module.enum_decl.is_some()
        );
    }

    #[test]
    fn state_and_accessor_shapes_are_uniform_in_classes_objects_and_companions() {
        let module = parse("class C { var x: Int = 1; get() = field; set(v) { field = v }; val y: Int by delegate(); lateinit var z: String; companion object { var n = 1; fun next() = n } }\nobject O { override var n: Int = 1; init {}; override fun next() = n }\ninterface I { companion object { var n = 1; fun next() = n; init {} } }").unwrap();
        let class = module.class_decl.unwrap();
        assert!(class.properties[0].setter.is_some() && !class.properties[0].body.is_empty());
        assert!(class.properties[1].delegate.is_some());
        assert!(class.properties[2].lateinit && class.properties[2].initializer.is_none());
        assert_eq!(class.companion.unwrap().methods.len(), 1);
        let object = module.object_decl.unwrap();
        assert!(object.fields[0].is_override && object.fields[0].mutable);
        assert!(object.methods[0].is_override);
        assert_eq!(object.initializers.len(), 1);
        let companion = module.interface_decl.unwrap().companion.unwrap();
        assert_eq!(companion.fields.len(), 1);
        assert_eq!(companion.methods.len(), 1);
        assert_eq!(companion.initializers.len(), 1);
    }

    #[test]
    fn inherited_and_nested_declarations_keep_virtual_and_visibility_information() {
        let module = parse("abstract class Base(val x: Int) { protected open fun value(): Int = x; abstract fun missing(): Int; class Nested; object O { fun f() = 1 }; interface I }\nclass Child(x: Int): Base(x) { override fun value(): Int = super.value() }").unwrap();
        let base = module.class_decl.unwrap();
        assert!(base.abstract_ && base.methods[0].open && base.methods[0].function.protected);
        assert!(!base.methods[1].has_body);
        assert_eq!(base.nested.len(), 3);
        assert!(
            matches!(&module.declarations[0], Declaration::Class(child) if child.super_class.as_ref().is_some_and(|(name,args)| name == "Base" && args.len() == 1) && child.methods[0].is_override)
        );
    }

    #[test]
    fn arrays_receiver_types_and_operators_retain_callable_contracts() {
        let module = parse("fun f(a: Array<Int>, block: StringBuilder.() -> String): Array<String?> = if (true) { val x = a[0]; arrayOf(x.toString()) } else { arrayOf(null) }\nclass Ops { operator fun contains(x: Int) = true; operator fun get(a: Int, b: Int) = a + b; operator fun set(a: Int, b: Int, v: Int) {} }\nfun use(o: Ops) { o[1, 2] = 3 }").unwrap();
        assert_eq!(
            module.functions[0].params[0].1.descriptor(),
            "[Ljava/lang/Integer;"
        );
        assert_eq!(
            module.functions[0].result.descriptor(),
            "[Ljava/lang/String;"
        );
        assert!(
            matches!(&module.functions[0].params[1].1, Type::ReceiverFunction { params, .. } if params.is_empty())
        );
        assert!(
            module
                .class_decl
                .unwrap()
                .methods
                .iter()
                .all(|method| method.function.operator)
        );
        assert!(
            matches!(&module.functions[1].body[0].kind, StatementKind::IndicesAssign(_, indices, _) if indices.len() == 2)
        );
        assert_eq!(
            Type::Array(Box::new(Type::Array(Box::new(Type::Int)))).descriptor(),
            "[[Ljava/lang/Integer;"
        );
    }

    #[test]
    fn control_bodies_local_functions_destructuring_and_labels_keep_scope() {
        let module = parse("fun f(): Int { fun local(x: Int): Int = x; val (x, _) = value; outer@ while (true) { continue@outer; break@outer }; return invoke done@ { return@done local(x) } }\nfun g(x: Int): Int = when (val subject = x) { 1 -> { val y = subject + 1; y }; else -> try { subject } catch (error: Exception) { 2 } finally {} }").unwrap();
        let body = &module.functions[0].body;
        assert!(
            matches!(&body[0].kind, StatementKind::LocalFunction(function) if function.name == "local")
        );
        assert!(
            matches!(&body[1].kind, StatementKind::Destructure(names, false, _) if names == &vec![Some("x".into()), None])
        );
        assert!(matches!(&body[2].kind, StatementKind::Labeled(label, _) if label == "outer"));
        let StatementKind::Return(Some(value)) = &module.functions[1].body[0].kind else {
            panic!()
        };
        assert!(
            matches!(&value.kind, ExprKind::WhenBinding(name, _, branches, otherwise) if name == "subject" && matches!(branches[0].1.kind, ExprKind::Block(_)) && matches!(otherwise.kind, ExprKind::Try { .. }))
        );
        assert!(!module.functions[1].result_inferred);
        assert!(parse("fun f(): Int = if (true) { return 1 } else 2").is_err());
        assert!(parse("fun f(): Int = try { return 1 } finally {}").is_err());
    }

    #[test]
    fn new_expression_offsets_shift_inside_string_templates() {
        let source = "fun f(): String = \"prefix${when (val x = 1) { 1 -> { val y = x; y }; else -> try { x } finally {} }}\"";
        let module = parse(source).unwrap();
        let StatementKind::Return(Some(value)) = &module.functions[0].body[0].kind else {
            panic!()
        };
        let ExprKind::Template(parts) = &value.kind else {
            panic!()
        };
        let ExprKind::WhenBinding(_, _, branches, otherwise) = &parts[1].kind else {
            panic!()
        };
        let ExprKind::Block(body) = &branches[0].1.kind else {
            panic!()
        };
        assert_eq!(body[0].offset, source.find("val y").unwrap());
        let ExprKind::Try { body, .. } = &otherwise.kind else {
            panic!()
        };
        assert_eq!(body[0].offset, source.find("{ x }").unwrap() + 2);
    }
    #[test]
    fn excessive_statement_nesting_is_rejected_on_the_default_thread_stack() {
        for source in [
            format!("fun f(): Int = {}", vec!["1"; 1000].join(" + ")),
            format!(
                "fun f(): Int {{ {}return 0 }}",
                "if (true) {} else ".repeat(1000)
            ),
            format!("fun f(a: IntArray): Int = a{}", ".size".repeat(1000)),
            format!("fun f(): Int = {}1", "if (true) 1 else ".repeat(1000)),
            format!("{}{}", "class Nested {".repeat(1000), "}".repeat(1000)),
        ] {
            assert!(
                parse(&source)
                    .unwrap_err()
                    .message
                    .to_lowercase()
                    .contains("nesting")
            );
        }
    }

    #[test]
    fn implicit_lambda_labels_follow_the_resolved_callee_name() {
        let module = parse("fun f(): Int = custom { return@custom 3 }\nfun g(): Int = receiver.named(block = { return@named 4 })").unwrap();
        for function in &module.functions {
            let StatementKind::Return(Some(value)) = &function.body[0].kind else {
                panic!()
            };
            let ExprKind::Call(_, arguments) = &value.kind else {
                panic!()
            };
            let lambda = match &arguments[0].kind {
                ExprKind::NamedArgument(_, value) => value.as_ref(),
                _ => &arguments[0],
            };
            assert!(matches!(&lambda.kind, ExprKind::Labeled(_, _)));
        }
    }
    #[test]
    fn typed_when_subjects_and_setters_preserve_type_checks() {
        let module =
            parse("fun f(): String = when (val value: String = input()) { else -> value }")
                .unwrap();
        let StatementKind::Return(Some(value)) = &module.functions[0].body[0].kind else {
            panic!()
        };
        let ExprKind::WhenBinding(_, subject, _, _) = &value.kind else {
            panic!()
        };
        assert!(
            matches!(&subject.kind, ExprKind::Block(statements) if matches!(&statements[0].kind, StatementKind::Let(_, false, Some(Type::String), _)))
        );
        assert!(parse("class C { var value: Int = 1; set(input: Int) { field = input } }").is_ok());
        for source in [
            "class C { var value: Int = 1; set(input: String) {} }",
            "class C { val value: Int get() {} get() = 1 }",
            "class C { operator operator fun get(i: Int): Int = i }",
            "class C { override override fun f(): Int = 1 }",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }
    #[test]
    fn bodyless_class_companions_keep_empty_state() {
        for source in [
            "class E() { companion object }",
            "class E { companion object\n fun f() = 1 }",
            "interface E { companion object }",
        ] {
            let module = parse(source).unwrap();
            if let Some(class) = module.class_decl {
                let companion = class.companion.unwrap();
                assert!(
                    companion.properties.is_empty()
                        && companion.methods.is_empty()
                        && companion.initializers.is_empty()
                );
            }
        }
    }
    #[test]
    fn bodyless_superclass_headers_preserve_following_declarations() {
        for source in [
            "open class Parent; class Child(): Parent()",
            "open class Parent\nclass Child(): Parent()\nfun f() {}",
            "interface I\nclass Child: I\nfun f() {}",
        ] {
            parse(source).unwrap();
        }
    }
    #[test]
    fn override_members_are_open_unless_explicitly_final() {
        let module = parse("class C { override fun f() {}; final override fun g() {}; override val x: Int get() = 1; final override val y: Int get() = 2 }").unwrap();
        let class = module.class_decl.unwrap();
        assert!(class.methods[0].open && !class.methods[1].open);
        assert!(class.properties[0].open && !class.properties[1].open);
        assert!(parse("class C { final open fun f() {} }").is_err());
    }
}
