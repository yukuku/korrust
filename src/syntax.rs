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

#[derive(Debug)]
pub struct Module {
    pub package: String,
    pub imports: Vec<(String, String, usize)>,
    pub functions: Vec<Function>,
    pub constants: Vec<ConstDecl>,
    pub enum_decl: Option<EnumDecl>,
    pub object_decl: Option<ObjectDecl>,
    pub class_decl: Option<ClassDecl>,
    pub interface_decl: Option<InterfaceDecl>,
}
#[derive(Debug)]
pub struct InterfaceDecl {
    pub name: String,
    pub offset: usize,
    pub internal: bool,
    pub interfaces: Vec<String>,
    pub properties: Vec<InterfaceProperty>,
    pub methods: Vec<InterfaceMethod>,
    pub nested: Vec<InterfaceDecl>,
    pub companion: Option<CompanionDecl>,
}
#[derive(Debug)]
pub struct InterfaceProperty {
    pub name: String,
    pub offset: usize,
    pub ty: Type,
    pub body: Option<Vec<Statement>>,
    pub is_override: bool,
    pub mutable: bool,
}
#[derive(Debug)]
pub struct InterfaceMethod {
    pub function: Function,
    pub has_body: bool,
    pub is_override: bool,
}
#[derive(Debug)]
pub struct CompanionDecl {
    pub properties: Vec<CompanionProperty>,
}
#[derive(Debug)]
pub struct CompanionProperty {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub value: CompanionValue,
}
#[derive(Debug)]
pub enum CompanionValue {
    StringArray(Vec<Vec<u16>>),
    Object(Box<ClassDecl>),
}
#[derive(Debug)]
pub struct EnumDecl {
    pub name: String,
    pub offset: usize,
    pub entries: Vec<(String, usize)>,
}
#[derive(Debug)]
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
}
#[derive(Debug)]
pub struct ObjectField {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub initializer: Expr,
}
#[derive(Debug)]
pub struct ObjectMethod {
    pub function: Function,
    pub jvm_static: bool,
    pub throws: Vec<String>,
    pub inline: bool,
    pub suppress: Vec<String>,
}
#[derive(Debug)]
pub struct ConstDecl {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub internal: bool,
    pub value: ConstValue,
}
#[derive(Debug)]
pub enum ConstValue {
    Int(i32),
    Long(i64),
    Float(u32),
    Char(u16),
    Boolean(bool),
    String(Vec<u16>),
}
#[derive(Debug)]
pub struct ClassDecl {
    pub name: String,
    pub offset: usize,
    pub internal: bool,
    pub params: Vec<(String, Type)>,
    pub public_params: Vec<String>,
    pub param_properties: Vec<String>,
    pub mutable_params: Vec<String>,
    pub defaults: Vec<Option<Expr>>,
    pub data: bool,
    pub interfaces: Vec<String>,
    pub properties: Vec<PropertyDecl>,
    pub methods: Vec<ClassMethod>,
    pub companion: Option<ClassCompanion>,
}
#[derive(Debug)]
pub struct ClassCompanion {
    pub properties: Vec<ClassCompanionProperty>,
}
#[derive(Debug)]
pub struct ClassCompanionProperty {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub mutable: bool,
    pub ty: Option<Type>,
    pub initializer: Expr,
}
#[derive(Debug)]
pub struct PropertyDecl {
    pub name: String,
    pub private: bool,
    pub mutable: bool,
    pub inferred: bool,
    pub initializer: Option<Expr>,
    pub offset: usize,
    pub ty: Type,
    pub body: Vec<Statement>,
    pub is_override: bool,
}
#[derive(Debug)]
pub struct ClassMethod {
    pub function: Function,
    pub is_override: bool,
}
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub internal: bool,
    pub receiver: Option<Type>,
    pub type_params: Vec<TypeParameter>,
    pub params: Vec<(String, Type)>,
    pub defaults: Vec<Option<Expr>>,
    pub result: Type,
    pub body: Vec<Statement>,
}
#[derive(Clone, Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub offset: usize,
}
#[derive(Clone, Debug)]
pub enum StatementKind {
    Let(String, bool, Option<Type>, Expr),
    Declare(String, bool, Type),
    IndexAssign(Expr, Expr, Expr),
    PropertyAssign(Expr, String, Expr),
    Continue,
    Break,
    ForUntil(String, Expr, Expr, Vec<Statement>),
    Assign(String, Expr),
    AddAssign(String, Expr),
    Return(Option<Expr>),
    Throw(Expr),
    Expression(Expr),
    If(Expr, Vec<Statement>, Vec<Statement>),
    While(Expr, Vec<Statement>),
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
            | ExprKind::Increment { value, .. } => pending.push(value),
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
            StatementKind::Let(_, _, _, value)
            | StatementKind::Assign(_, value)
            | StatementKind::AddAssign(_, value)
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
            StatementKind::While(condition, body) => {
                shift_expression_offsets(condition, offset);
                shift_statement_offsets(body, offset);
            }
            StatementKind::When(subject, branches, otherwise) => {
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
        let first = text.as_bytes()[0];
        if text.bytes().all(|b| b == b'_') {
            return Err(self.error("Names consisting only of underscores are reserved"));
        }
        if !(first.is_ascii_alphabetic() || first == b'_')
            || matches!(
                text.as_str(),
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
        {
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
        while self.take(".") {
            name.push('/');
            name.push_str(&self.name()?);
        }
        self.named_ty(name)
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
                        "Array" if *element == Type::String => return Ok(Type::StringArray),
                        "ThreadLocal" if matches!(element, Type::ByteArray | Type::CharArray) => {
                            return Ok(Type::ThreadLocal(Box::new(element.clone())));
                        }
                        _ => {}
                    }
                }
            }
            if name == "Array" {
                return Err(self.error("Only Array<String> is supported among generic array types"));
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
                });
            }
            return Err(self.error("Only plain named singleton objects are supported"));
        }
        self.separators();
        let mut constants = Vec::new();
        let mut objects = Vec::new();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated object body"));
            }
            let member_offset = self.token().offset;
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
            } else if self.at("private")
                && self.tokens[self.pos + 1..]
                    .iter()
                    .find(|token| token.text != "\n")
                    .is_some_and(|token| token.text == "val")
            {
                self.expect("private")?;
                self.lines();
                self.expect("val")?;
                let name = self.declaration_name()?;
                if self.at(":") {
                    return Err(self.error(
                        "Object private field types must be inferred from supported initializers",
                    ));
                }
                self.expect("=")?;
                self.lines();
                let initializer = self.expr(0)?;
                fields.push(ObjectField {
                    name: name.clone(),
                    offset: member_offset,
                    private: true,
                    initializer,
                });
                if fields.len() > 256 {
                    return Err(self.error("Objects exceed the current 256-private-field limit"));
                }
                name
            } else {
                let constant = self.constant()?;
                if constant.private {
                    return Err(Diagnostic::new(
                        member_offset,
                        "Private object constants are not supported",
                    ));
                }
                if matches!(constant.value, ConstValue::Long(_) | ConstValue::Float(_)) {
                    return Err(Diagnostic::new(
                        member_offset,
                        "Object constants support only Int, Char, Boolean, and String literals",
                    ));
                }
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
        if depth > 1 && (!fields.is_empty() || !methods.is_empty()) {
            return Err(Diagnostic::new(
                offset,
                "Nested objects with initialized fields or methods are not supported",
            ));
        }
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
        if function.receiver.is_some() {
            return Err(Diagnostic::new(
                offset,
                "Object member extension functions are not supported",
            ));
        }
        Ok(ObjectMethod {
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
                    let value = i32::try_from(value)
                        .map_err(|_| self.error("Int constant literal is out of range"))?;
                    (ConstValue::Int(value), Type::Int)
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
        let mut mutable_params = Vec::new();
        let mut defaults = Vec::new();
        let mut property_names = std::collections::HashSet::new();
        let mut param_names = std::collections::HashSet::new();
        if has_constructor {
            self.lines();
            if !self.at(")") {
                loop {
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
        self.lines();
        let mut interfaces = Vec::new();
        if self.take(":") {
            self.lines();
            loop {
                interfaces.push(self.declaration_name()?);
                if interfaces.len() > 32 {
                    return Err(self.error("Classes exceed the current 32-interface limit"));
                }
                self.lines();
                if !self.take(",") {
                    break;
                }
                self.lines();
            }
        }
        let mut declaration = ClassDecl {
            name,
            offset,
            internal: visibility.as_deref() == Some("internal"),
            params,
            public_params,
            param_properties,
            mutable_params,
            defaults,
            data,
            interfaces,
            properties: Vec::new(),
            methods: Vec::new(),
            companion: None,
        };
        if !self.take("{") {
            if declaration.interfaces.is_empty() {
                self.pos = after_params;
            }
            if matches!(self.token().text.as_str(), "\n" | ";" | "<eof>") {
                return Ok(declaration);
            }
            return Err(self.error(
                "Only imported interface supertypes without constructor calls are supported",
            ));
        }
        self.class_members(&mut declaration, &mut property_names)?;
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
            let member_offset = self.token().offset;
            let mut is_override = false;
            let mut member_visibility = None;
            loop {
                if self.take("override") {
                    if is_override {
                        return Err(self.error("Repeated override modifier"));
                    }
                    is_override = true;
                } else if matches!(
                    self.token().text.as_str(),
                    "public" | "private" | "internal" | "protected"
                ) {
                    if member_visibility.is_some() {
                        return Err(
                            self.error("Conflicting or repeated member visibility modifiers")
                        );
                    }
                    member_visibility = Some(self.token().text.clone());
                    self.pos += 1;
                } else {
                    break;
                }
                self.lines();
            }
            if matches!(member_visibility.as_deref(), Some("internal" | "protected")) {
                return Err(self.error("Internal and protected class members are not supported"));
            }
            if self.at("fun") {
                if is_override && member_visibility.as_deref() == Some("private") {
                    return Err(self.error("Overrides cannot be private"));
                }
                let mut function = self.function()?;
                function.offset = member_offset;
                function.private = member_visibility.as_deref() == Some("private");
                if function.receiver.is_some() {
                    return Err(Diagnostic::new(
                        member_offset,
                        "Member extension functions are not supported",
                    ));
                }
                declaration.methods.push(ClassMethod {
                    function,
                    is_override,
                });
            } else if self.at("val") || self.at("var") {
                let mutable = self.take("var");
                if !mutable {
                    self.expect("val")?;
                }
                let name_offset = self.token().offset;
                let property = self.declaration_name()?;
                if !property_names.insert(property.clone()) {
                    return Err(Diagnostic::new(
                        name_offset,
                        "Duplicate class property name",
                    ));
                }
                let declared_type = if self.take(":") {
                    self.lines();
                    Some(self.ty()?)
                } else {
                    None
                };
                self.lines();
                let (initializer, body) = if self.take("=") {
                    self.lines();
                    (Some(self.expr(0)?), Vec::new())
                } else {
                    if mutable {
                        return Err(self.error("Mutable computed properties are not supported"));
                    }
                    if declared_type.is_none() {
                        return Err(self.error("Computed properties require an explicit type"));
                    }
                    self.expect("get")?;
                    self.expect("(")?;
                    self.expect(")")?;
                    self.lines();
                    let body = if self.take("=") {
                        self.lines();
                        vec![Statement {
                            offset: self.token().offset,
                            kind: StatementKind::Return(Some(self.expression_body()?)),
                        }]
                    } else {
                        self.block()?
                    };
                    (None, body)
                };
                declaration.properties.push(PropertyDecl {
                    name: property,
                    private: member_visibility.as_deref() == Some("private"),
                    mutable,
                    inferred: declared_type.is_none(),
                    initializer,
                    offset: member_offset,
                    ty: declared_type.unwrap_or(Type::Unit),
                    body,
                    is_override,
                });
            } else if self.at("companion") {
                if is_override || member_visibility.is_some() {
                    return Err(self.error("Class companion modifiers are not supported"));
                }
                if declaration.companion.is_some() {
                    return Err(self.error("Classes support only one companion object"));
                }
                declaration.companion = Some(self.class_companion()?);
            } else {
                return Err(self.error("Only instance functions, properties, and companion properties are supported in classes"));
            }
            if declaration.properties.len() + declaration.methods.len() > 1024 {
                return Err(self.error("Classes exceed the current 1024-member limit"));
            }
            self.end()?;
        }
        self.expect("}")?;
        Ok(())
    }
    fn class_companion(&mut self) -> Result<ClassCompanion> {
        self.expect("companion")?;
        self.lines();
        self.expect("object")?;
        self.lines();
        self.expect("{")?;
        self.separators();
        let mut properties = Vec::new();
        let mut names = std::collections::HashSet::new();
        while !self.at("}") {
            if self.at("<eof>") {
                return Err(self.error("Unterminated class companion"));
            }
            let offset = self.token().offset;
            let private = self.take("private");
            if !private {
                self.take("public");
            }
            self.lines();
            let mutable = self.take("var");
            if !mutable {
                self.expect("val")?;
            }
            let name = self.declaration_name()?;
            if !names.insert(name.clone()) {
                return Err(Diagnostic::new(offset, "Duplicate companion property name"));
            }
            let ty = if self.take(":") {
                self.lines();
                Some(self.ty()?)
            } else {
                None
            };
            self.lines();
            self.expect("=")?;
            self.lines();
            let initializer = self.expr(0)?;
            properties.push(ClassCompanionProperty {
                name,
                offset,
                private,
                mutable,
                ty,
                initializer,
            });
            if properties.len() > 256 {
                return Err(self.error("Companions exceed the current 256-property limit"));
            }
            self.end()?;
        }
        self.expect("}")?;
        Ok(ClassCompanion { properties })
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
            } else if self.at("companion") {
                if declaration.companion.is_some() {
                    return Err(self.error("Only one unnamed companion object is supported"));
                }
                declaration.companion = Some(self.companion_decl()?);
            } else {
                let offset = self.token().offset;
                let mut is_override = false;
                let mut public_modifier = false;
                loop {
                    if self.take("override") {
                        if is_override {
                            return Err(self.error("Repeated override modifier"));
                        }
                        is_override = true;
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
                if self.at("fun") {
                    let (function, has_body) = self.function_body(true)?;
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
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(CompanionDecl { properties });
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
                    name: String::new(),
                    offset,
                    internal: false,
                    params: Vec::new(),
                    public_params: Vec::new(),
                    param_properties: Vec::new(),
                    mutable_params: Vec::new(),
                    defaults: Vec::new(),
                    data: false,
                    interfaces: vec![interface],
                    properties: Vec::new(),
                    methods: Vec::new(),
                    companion: None,
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
        Ok(CompanionDecl { properties })
    }
    fn function(&mut self) -> Result<Function> {
        self.function_body(false).map(|(function, _)| function)
    }
    fn function_body(&mut self, abstract_allowed: bool) -> Result<(Function, bool)> {
        let offset = self.token().offset;
        let mut visibility = None;
        while matches!(
            self.token().text.as_str(),
            "private" | "public" | "internal"
        ) {
            if visibility.is_some() {
                return Err(self.error("Conflicting or repeated visibility modifiers"));
            }
            visibility = Some(self.token().text.clone());
            self.pos += 1;
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
        let result = if self.take(":") {
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
        let body = if self.take("=") {
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
                internal,
                receiver,
                type_params,
                params,
                defaults,
                result,
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
        if self.depth > 128 {
            return Err(self.error("Nesting exceeds the current limit"));
        }
        let offset = self.token().offset;
        let kind = if self.at("val") || self.at("var") {
            let mutable = self.take("var");
            if !mutable {
                self.expect("val")?;
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
            StatementKind::Continue
        } else if self.take("break") {
            StatementKind::Break
        } else if self.take("for") {
            self.expect("(")?;
            self.lines();
            let name = self.declaration_name()?;
            self.expect("in")?;
            self.lines();
            let start = self.expr(0)?;
            self.expect("until")?;
            self.lines();
            let end = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            self.lines();
            StatementKind::ForUntil(name, start, end, self.statement_body()?)
        } else if self.take("throw") {
            self.lines();
            StatementKind::Throw(self.expr(0)?)
        } else if self.take("return") {
            StatementKind::Return(if self.at("\n") || self.at(";") || self.at("}") {
                None
            } else {
                Some(self.expr(0)?)
            })
        } else if self.take("when") {
            let subjectless = !self.at("(");
            let subject = if self.take("(") {
                self.lines();
                let subject = self.expr(0)?;
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
                        conditions.push(self.when_condition()?);
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
            StatementKind::When(subject, branches, otherwise)
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
        if self.depth > 128 {
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
                if matches!(self.tokens[next].text.as_str(), "&&" | "||") {
                    self.pos = next;
                }
            }
            let negated_membership = self.at("!")
                && self
                    .tokens
                    .get(self.pos + 1)
                    .is_some_and(|token| token.text == "in");
            let op = if negated_membership {
                "!in".to_string()
            } else {
                self.token().text.clone()
            };
            let Some(precedence) = precedence(&op) else {
                break;
            };
            if precedence < min {
                break;
            }
            self.pos += if negated_membership { 2 } else { 1 };
            self.lines();
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
        if self.take("++") {
            return Self::bounded(Expr {
                offset,
                kind: ExprKind::Increment {
                    value: Box::new(self.expr(9)?),
                    prefix: true,
                },
            });
        }
        if self.at("-") || self.at("+") || self.at("!") {
            let op = self.token().text.clone();
            self.pos += 1;
            return Self::bounded(Expr {
                offset,
                kind: ExprKind::Unary(op, Box::new(self.expr(9)?)),
            });
        }
        let mut expr = if self.take("return") {
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
                kind: ExprKind::Return(value),
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
                } else {
                    self.name()?
                }),
            }
        };
        loop {
            if self.at("<")
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
            } else if self.at("{") {
                let lambda = self.lambda()?;
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
                self.expect("]")?;
                expr = Expr {
                    offset,
                    kind: ExprKind::Index(Box::new(expr), Box::new(index)),
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
            expr = Self::bounded(expr)?;
        }
        Self::bounded(expr)
    }
    fn when_condition(&mut self) -> Result<Expr> {
        let offset = self.token().offset;
        let operator = if self.take("in") {
            Some("in")
        } else if self.at("!")
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| token.text == "in")
        {
            self.pos += 2;
            Some("!in")
        } else {
            None
        };
        if let Some(operator) = operator {
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
        let subject = if self.take("(") {
            self.lines();
            let subject = self.expr(0)?;
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
                    conditions.push(self.when_condition()?);
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
            kind: ExprKind::When(Box::new(subject), branches, Box::new(otherwise)),
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
        let params = if self.take("->") {
            Some(Vec::new())
        } else {
            let mut parameters = Vec::new();
            let parsed = (|| -> Result<()> {
                loop {
                    let name = if self.take("_") {
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
                None
            }
        };
        self.separators();
        let mut body = Vec::new();
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
        if self.take("{") {
            self.lines();
            let value = self.expr(0)?;
            self.lines();
            self.expect("}")?;
            Ok(value)
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
    if let Some(digits) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Diagnostic::new(
                offset,
                "Hexadecimal literals require plain hexadecimal digits without suffixes or separators",
            ));
        }
        i64::from_str_radix(digits, 16)
            .map_err(|_| Diagnostic::new(offset, "Integer literal is out of range"))
    } else {
        if text.len() > 1 && text.starts_with('0') {
            return Err(Diagnostic::new(
                offset,
                "Leading zeroes are not allowed in decimal literals",
            ));
        }
        text.parse()
            .map_err(|_| Diagnostic::new(offset, "Integer literal is out of range"))
    }
}

fn precedence(op: &str) -> Option<u8> {
    match op {
        "||" => Some(1),
        "&&" => Some(2),
        "==" | "!=" => Some(3),
        "?:" => Some(4),
        "<" | ">" | "<=" | ">=" | "in" | "!in" => Some(4),
        "shl" | "shr" | "ushr" | "and" | "or" | "xor" => Some(5),
        ".." => Some(6),
        "+" | "-" => Some(7),
        "*" | "/" | "%" => Some(8),
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
    let mut enum_decl = None;
    let mut object_decl = None;
    let mut class_decl = None;
    let mut interface_decl = None;
    while !p.at("<eof>") {
        if interface_decl.is_some()
            || (p.starts_interface()
                && (!constants.is_empty()
                    || !functions.is_empty()
                    || enum_decl.is_some()
                    || object_decl.is_some()
                    || class_decl.is_some()))
        {
            return Err(
                p.error("Only one top-level interface without other declarations is supported")
            );
        }
        if class_decl.is_some()
            || (p.starts_class()
                && (!constants.is_empty()
                    || !functions.is_empty()
                    || enum_decl.is_some()
                    || object_decl.is_some()))
        {
            return Err(p.error("Only one top-level class without other declarations is supported"));
        }
        if object_decl.is_some()
            || (p.starts_object()
                && (!constants.is_empty() || !functions.is_empty() || enum_decl.is_some()))
        {
            return Err(
                p.error("Only one top-level object without other declarations is supported")
            );
        }
        if enum_decl.is_some()
            || (p.starts_enum() && (!functions.is_empty() || !constants.is_empty()))
        {
            return Err(p.error("Only one enum class without other declarations is supported"));
        }
        if p.starts_declaration("const") {
            constants.push(p.constant()?);
        } else if p.starts_enum() {
            enum_decl = Some(p.enum_decl()?);
        } else if p.starts_object() {
            object_decl = Some(p.object_decl(1, &mut 0)?);
        } else if p.starts_class() {
            class_decl = Some(p.class_decl()?);
        } else if p.starts_interface() {
            interface_decl = Some(p.interface_decl(1, &mut 0)?);
        } else {
            functions.push(p.function()?);
        }
        p.end()?;
    }
    if functions.is_empty()
        && constants.is_empty()
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
        enum_decl,
        object_decl,
        class_decl,
        interface_decl,
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
            "class C { init {} }",
            "object E { private const val N = 1 }",
            "object E { const val N = 1L }",
            "object E { const val N = 1.0f }",
            "class C: Base()",
            "class C { companion object Named {} }",
            "class C { companion object { fun f(): Int = 1 } }",
            "fun f(x: Unit?): Unit {}",
            "const val x = 1\nenum class E",
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
}
