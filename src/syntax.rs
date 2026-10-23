use crate::{Diagnostic, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
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
    ThreadLocal(Box<Type>),
    Platform(Box<Type>),
}

impl Type {
    pub fn base(&self) -> &Type {
        match self {
            Self::Platform(t) => t.base(),
            _ => self,
        }
    }
    pub fn is_reference(&self) -> bool {
        matches!(
            self.base(),
            Self::IntArray
                | Self::ByteArray
                | Self::CharArray
                | Self::StringArray
                | Self::String
                | Self::Java(_)
                | Self::ThreadLocal(_)
        )
    }
    pub fn descriptor(&self) -> String {
        match self.base() {
            Self::Int => "I".into(),
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
            Self::Java(name) => format!("L{name};"),
            Self::ThreadLocal(_) => "Ljava/lang/ThreadLocal;".into(),
            Self::Platform(_) => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub package: String,
    pub imports: Vec<(String, String, usize)>,
    pub functions: Vec<Function>,
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
    Object(ClassDecl),
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
    pub value: ConstValue,
}
#[derive(Debug)]
pub enum ConstValue {
    Int(i32),
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
    pub interfaces: Vec<String>,
    pub properties: Vec<PropertyDecl>,
    pub methods: Vec<ClassMethod>,
}
#[derive(Debug)]
pub struct PropertyDecl {
    pub name: String,
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
    pub params: Vec<(String, Type)>,
    pub result: Type,
    pub body: Vec<Statement>,
}
#[derive(Debug)]
pub struct Statement {
    pub kind: StatementKind,
    pub offset: usize,
}
#[derive(Debug)]
pub enum StatementKind {
    Let(String, bool, Option<Type>, Expr),
    Declare(String, bool, Type),
    IndexAssign(Expr, Expr, Expr),
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
#[derive(Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub offset: usize,
}
#[derive(Debug)]
pub enum ExprKind {
    Int(i64),
    Char(u16),
    Boolean(bool),
    String(Vec<u16>),
    Template(Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Name(String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Property(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Increment { value: Box<Expr>, prefix: bool },
    NonNull(Box<Expr>),
    Lambda(Box<Expr>),
    When(Box<Expr>, Vec<(Vec<Expr>, Expr)>, Box<Expr>),
}
#[derive(Debug, Clone)]
struct Token {
    text: String,
    offset: usize,
}

fn string_parts(source: &str, start: usize) -> Result<(usize, Vec<Expr>)> {
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
            i = string_parts(source, start)?.0;
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
        } else if b.is_ascii_alphabetic() || b == b'_' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b.is_ascii_digit() {
            if b == b'0'
                && bytes
                    .get(i + 1)
                    .is_some_and(|next| *next == b'x' || *next == b'X')
            {
                i += 2;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
            } else {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b"(){}[]:.,;=+-*/%<>!@".contains(&b) || b == b'&' || b == b'|' {
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
        if self.take("ThreadLocal") {
            self.expect("<")?;
            self.lines();
            let element = self.ty()?;
            self.lines();
            self.expect(">")?;
            if !matches!(element, Type::ByteArray | Type::CharArray) {
                return Err(self.error(
                    "Only ThreadLocal<ByteArray> and ThreadLocal<CharArray> are supported",
                ));
            }
            return Ok(Type::ThreadLocal(Box::new(element)));
        }
        if self.take("Array") {
            self.expect("<")?;
            self.lines();
            if !self.take("String") {
                return Err(self.error("Only Array<String> is supported among generic array types"));
            }
            self.lines();
            self.expect(">")?;
            return Ok(Type::StringArray);
        }
        let ty = match self.token().text.as_str() {
            "Int" => Type::Int,
            "Byte" => Type::Byte,
            "Char" => Type::Char,
            "Boolean" => Type::Boolean,
            "Float" => Type::Float,
            "IntArray" => Type::IntArray,
            "ByteArray" => Type::ByteArray,
            "CharArray" => Type::CharArray,
            "String" => Type::String,
            "Unit" => Type::Unit,
            _ => return Ok(Type::Java(self.name()?)),
        };
        self.pos += 1;
        Ok(ty)
    }
    fn starts_enum(&self) -> bool {
        self.starts_declaration("enum")
    }
    fn starts_object(&self) -> bool {
        self.starts_declaration("object")
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
        let after_name = self.pos;
        self.lines();
        if !self.take("{") {
            if matches!(self.token().text.as_str(), "(" | ":" | "<" | "where") {
                return Err(
                    self.error("Object constructors, supertypes, and generics are not supported")
                );
            }
            self.pos = after_name;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(ObjectDecl {
                    name,
                    offset,
                    internal: visibility.as_deref() == Some("internal"),
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
            let member_name = if self.starts_object() {
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
        let mut visibility = false;
        let mut is_const = false;
        loop {
            if matches!(self.token().text.as_str(), "public" | "internal") {
                if visibility {
                    return Err(self.error("Conflicting or repeated constant visibility modifiers"));
                }
                visibility = true;
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
        if matches!(self.token().text.as_str(), "private" | "protected") {
            return Err(self.error("Unsupported constant visibility; use public or internal"));
        }
        if !is_const || !self.take("val") {
            return Err(self.error(
                "Only nested named objects and const val literal properties are supported",
            ));
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
            let magnitude = integer_literal(&token.text, token.offset)?;
            let signed = if negative == Some(true) {
                -magnitude
            } else {
                magnitude
            };
            let value = i32::try_from(signed)
                .map_err(|_| self.error("Int constant literal is out of range"))?;
            (ConstValue::Int(value), Type::Int)
        } else if negative.is_some() {
            return Err(self.error("Const initializers support only signed Int literals or unsigned Char, Boolean, and String literals"));
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
            return Err(self
                .error("Const initializers support only Int, Char, Boolean, and String literals"));
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
        Ok(ConstDecl { name, value })
    }
    fn class_decl(&mut self) -> Result<ClassDecl> {
        let offset = self.token().offset;
        let mut visibility = None;
        let mut final_modifier = false;
        loop {
            if matches!(self.token().text.as_str(), "public" | "internal") {
                if visibility.is_some() {
                    return Err(self.error("Conflicting or repeated class visibility modifiers"));
                }
                visibility = Some(self.token().text.clone());
                self.pos += 1;
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
        self.lines();
        if !self.take("(") {
            return Err(self.error(
                "Classes require an explicit primary constructor with private val parameters",
            ));
        }
        self.lines();
        let mut params = Vec::new();
        let mut property_names = std::collections::HashSet::new();
        if !self.at(")") {
            loop {
                if !self.take("private") {
                    return Err(
                        self.error("Only private val primary constructor parameters are supported")
                    );
                }
                self.lines();
                self.expect("val")?;
                self.lines();
                let param_offset = self.token().offset;
                let param = self.declaration_name()?;
                if !property_names.insert(param.clone()) {
                    return Err(Diagnostic::new(
                        param_offset,
                        "Duplicate class property name",
                    ));
                }
                self.expect(":")?;
                self.lines();
                params.push((param, self.ty()?));
                if params.len() > 254 {
                    return Err(
                        self.error("Class constructors support at most 254 parameter slots")
                    );
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
            interfaces,
            properties: Vec::new(),
            methods: Vec::new(),
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
            } else if self.take("val") {
                if member_visibility.as_deref() == Some("private") {
                    return Err(self.error("Only public class getter properties are supported"));
                }
                let name_offset = self.token().offset;
                let property = self.declaration_name()?;
                if !property_names.insert(property.clone()) {
                    return Err(Diagnostic::new(
                        name_offset,
                        "Duplicate class property name",
                    ));
                }
                self.expect(":")?;
                self.lines();
                let ty = self.ty()?;
                self.lines();
                if !self.take("get") {
                    return Err(self.error(
                        "Class properties require an explicit getter without a backing field",
                    ));
                }
                self.expect("(")?;
                self.expect(")")?;
                self.lines();
                let body = if self.take("=") {
                    self.lines();
                    vec![Statement {
                        offset: self.token().offset,
                        kind: StatementKind::Return(Some(self.expr(0)?)),
                    }]
                } else {
                    self.block()?
                };
                declaration.properties.push(PropertyDecl {
                    name: property,
                    offset: member_offset,
                    ty,
                    body,
                    is_override,
                });
            } else {
                return Err(self.error("Only instance functions and explicit getter properties are supported in classes"));
            }
            if declaration.properties.len() + declaration.methods.len() > 1024 {
                return Err(self.error("Classes exceed the current 1024-member limit"));
            }
            self.end()?;
        }
        self.expect("}")?;
        Ok(())
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
                } else if self.take("val") {
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
                kind: StatementKind::Return(Some(self.expr(0)?)),
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
                    interfaces: vec![interface],
                    properties: Vec::new(),
                    methods: Vec::new(),
                };
                self.class_members(&mut object, &mut std::collections::HashSet::new())?;
                CompanionValue::Object(object)
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
        let first = self.name()?;
        let (receiver, name) = if self.take(".") {
            (
                Some(match first.as_str() {
                    "IntArray" => Type::IntArray,
                    "CharArray" => Type::CharArray,
                    "Char" => Type::Char,
                    "String" => Type::String,
                    "Int" => Type::Int,
                    "Boolean" => Type::Boolean,
                    _ => Type::Java(first),
                }),
                self.name()?,
            )
        } else {
            (None, first)
        };
        self.expect("(")?;
        self.lines();
        let mut params = Vec::new();
        if !self.at(")") {
            loop {
                let name = self.name()?;
                self.expect(":")?;
                self.lines();
                params.push((name, self.ty()?));
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
        let after_signature = self.pos;
        self.lines();
        let mut has_body = true;
        let body = if self.take("=") {
            self.lines();
            vec![Statement {
                offset: self.token().offset,
                kind: StatementKind::Return(Some(self.expr(0)?)),
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
        Ok((
            Function {
                name,
                offset,
                private,
                internal,
                receiver,
                params,
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
            self.expect("(")?;
            self.lines();
            let subject = self.expr(0)?;
            self.lines();
            self.expect(")")?;
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
                let ExprKind::Index(array, index) = target.kind else {
                    return Err(
                        self.error("Only local and array element assignments are supported")
                    );
                };
                self.lines();
                StatementKind::IndexAssign(*array, *index, self.expr(0)?)
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
            let right = self.expr(precedence + 1)?;
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
        let mut expr = if self.take("when") {
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
        } else if self.take("(") {
            self.lines();
            let expr = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            expr
        } else if self.token().text.starts_with('"') {
            let mut parts = string_parts(&self.token().text, 0)?.1;
            for part in &mut parts {
                part.offset += offset;
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
            let value = integer_literal(&self.token().text, self.token().offset)?;
            self.pos += 1;
            Expr {
                offset,
                kind: ExprKind::Int(value),
            }
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
            if self.take("!!") {
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
                let lambda = Expr {
                    offset: self.token().offset,
                    kind: ExprKind::Lambda(Box::new(self.expression_branch()?)),
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
                        args.push(self.expr(0)?);
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
        self.expect("(")?;
        self.lines();
        let subject = self.expr(0)?;
        self.lines();
        self.expect(")")?;
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
    fn bounded(expr: Expr) -> Result<Expr> {
        let mut pending = vec![(&expr, 1)];
        while let Some((value, depth)) = pending.pop() {
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
                ExprKind::Increment { value, .. }
                | ExprKind::NonNull(value)
                | ExprKind::Lambda(value) => pending.push((value, depth + 1)),
                ExprKind::Unary(_, a) | ExprKind::Property(a, _) => pending.push((a, depth + 1)),
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
            alias = p.name()?;
            name.push('/');
            name.push_str(&alias);
        }
        if p.take("as") {
            alias = p.name()?;
        }
        p.end()?;
        imports.push((alias, name, offset));
    }
    let mut functions = Vec::new();
    let mut enum_decl = None;
    let mut object_decl = None;
    let mut class_decl = None;
    let mut interface_decl = None;
    while !p.at("<eof>") {
        if interface_decl.is_some()
            || (p.starts_interface()
                && (!functions.is_empty()
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
                && (!functions.is_empty() || enum_decl.is_some() || object_decl.is_some()))
        {
            return Err(p.error("Only one top-level class without other declarations is supported"));
        }
        if object_decl.is_some()
            || (p.starts_object() && (!functions.is_empty() || enum_decl.is_some()))
        {
            return Err(
                p.error("Only one top-level object without other declarations is supported")
            );
        }
        if enum_decl.is_some() || (p.starts_enum() && !functions.is_empty()) {
            return Err(p.error("Only one enum class without other declarations is supported"));
        }
        if p.starts_enum() {
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
        enum_decl,
        object_decl,
        class_decl,
        interface_decl,
    })
}
