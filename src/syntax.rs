use crate::{Diagnostic, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Char,
    Boolean,
    IntArray,
    CharArray,
    String,
    Unit,
    Java(String),
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
            Self::IntArray | Self::CharArray | Self::String | Self::Java(_)
        )
    }
    pub fn descriptor(&self) -> String {
        match self.base() {
            Self::Int => "I".into(),
            Self::Char => "C".into(),
            Self::Boolean => "Z".into(),
            Self::IntArray => "[I".into(),
            Self::CharArray => "[C".into(),
            Self::String => "Ljava/lang/String;".into(),
            Self::Unit => "V".into(),
            Self::Java(name) => format!("L{name};"),
            Self::Platform(_) => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct Module {
    pub package: String,
    pub imports: Vec<(String, String, usize)>,
    pub functions: Vec<Function>,
}
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub offset: usize,
    pub private: bool,
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
    Assign(String, Expr),
    AddAssign(String, Expr),
    Return(Option<Expr>),
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
    Name(String),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Property(Box<Expr>, String),
    Call(Box<Expr>, Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
}
#[derive(Debug, Clone)]
struct Token {
    text: String,
    offset: usize,
}

fn string_literal(source: &str, start: usize) -> Result<(usize, Vec<u16>)> {
    let bytes = source.as_bytes();
    if bytes[start..].starts_with(b"\"\"\"") {
        return Err(Diagnostic::new(
            start,
            "Raw triple-quoted strings are not supported",
        ));
    }
    let mut i = start + 1;
    let mut units = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok((i + 1, units)),
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
            b'$' if source[i + 1..]
                .chars()
                .next()
                .is_some_and(|c| c == '{' || c == '`' || c == '_' || c.is_alphabetic()) =>
            {
                return Err(Diagnostic::new(i, "String interpolation is not supported"));
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
            i = string_literal(source, start)?.0;
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
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b"(){}[]:.,;=+-*/%<>!".contains(&b) || b == b'&' || b == b'|' {
            i += 1;
            if i < bytes.len()
                && matches!(
                    &bytes[start..=i],
                    b"==" | b"!=" | b"<=" | b">=" | b"&&" | b"||" | b"+=" | b"->" | b"++" | b"--"
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
    fn ty(&mut self) -> Result<Type> {
        let ty = match self.token().text.as_str() {
            "Int" => Type::Int,
            "Char" => Type::Char,
            "Boolean" => Type::Boolean,
            "IntArray" => Type::IntArray,
            "CharArray" => Type::CharArray,
            "String" => Type::String,
            "Unit" => Type::Unit,
            _ => return Ok(Type::Java(self.name()?)),
        };
        self.pos += 1;
        Ok(ty)
    }
    fn function(&mut self) -> Result<Function> {
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
        Ok(Function {
            name,
            offset,
            private,
            receiver,
            params,
            result,
            body,
        })
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
            self.expect("=")?;
            self.lines();
            StatementKind::Let(name, mutable, ty, self.expr(0)?)
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
                        conditions.push(self.expr(0)?);
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
            StatementKind::While(cond, self.block()?)
        } else if self.take("if") {
            self.expect("(")?;
            self.lines();
            let cond = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            self.lines();
            let yes = self.block()?;
            let save = self.pos;
            self.lines();
            let no = if self.take("else") {
                self.lines();
                if self.at("if") {
                    vec![self.statement()?]
                } else {
                    self.block()?
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
            StatementKind::Expression(self.expr(0)?)
        };
        self.depth -= 1;
        Ok(Statement { kind, offset })
    }
    fn expr(&mut self, min: u8) -> Result<Expr> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(self.error("Nesting exceeds the current limit"));
        }
        let mut left = self.prefix()?;
        while let Some(precedence) = precedence(&self.token().text) {
            if precedence < min {
                break;
            }
            let op = self.token().text.clone();
            self.pos += 1;
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
        if self.at("-") || self.at("+") || self.at("!") {
            let op = self.token().text.clone();
            self.pos += 1;
            return Self::bounded(Expr {
                offset,
                kind: ExprKind::Unary(op, Box::new(self.expr(8)?)),
            });
        }
        let mut expr = if self.take("(") {
            self.lines();
            let expr = self.expr(0)?;
            self.lines();
            self.expect(")")?;
            expr
        } else if self.token().text.starts_with('"') {
            let value = string_literal(&self.token().text, 0)?.1;
            self.pos += 1;
            Expr {
                offset,
                kind: ExprKind::String(value),
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
            let text = &self.token().text;
            if text.len() > 1 && text.starts_with('0') {
                return Err(self.error("Leading zeroes are not allowed in decimal literals"));
            }
            let value = text
                .parse()
                .map_err(|_| self.error("Integer literal is out of range"))?;
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
            if self.take(".") {
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
fn precedence(op: &str) -> Option<u8> {
    match op {
        "||" => Some(1),
        "&&" => Some(2),
        "==" | "!=" => Some(3),
        "<" | ">" | "<=" | ">=" => Some(4),
        "shl" | "shr" | "ushr" | "and" | "or" | "xor" => Some(5),
        "+" | "-" => Some(6),
        "*" | "/" | "%" => Some(7),
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
    while !p.at("<eof>") {
        functions.push(p.function()?);
        p.end()?;
    }
    if functions.is_empty() {
        return Err(p.error("Expected at least one function"));
    }
    Ok(Module {
        package,
        imports,
        functions,
    })
}
