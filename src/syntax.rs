use crate::{Diagnostic, DiagnosticKind, Result};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Int,
    Long,
    Byte,
    Char,
    Boolean,
    Float,
    Double,
    IntArray,
    ByteArray,
    CharArray,
    LongArray,
    BooleanArray,
    FloatArray,
    DoubleArray,
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
    pub discard: bool,
    pub ty: Option<Type>,
}

impl Type {
    pub fn primitive_array_named(name: &str) -> Option<Type> {
        match name.strip_prefix("kotlin/").unwrap_or(name) {
            "IntArray" => Some(Self::IntArray),
            "ByteArray" => Some(Self::ByteArray),
            "CharArray" => Some(Self::CharArray),
            "LongArray" => Some(Self::LongArray),
            "BooleanArray" => Some(Self::BooleanArray),
            "FloatArray" => Some(Self::FloatArray),
            "DoubleArray" => Some(Self::DoubleArray),
            _ => None,
        }
    }
    pub fn primitive_array_element(&self) -> Option<Type> {
        match self.base() {
            Self::IntArray => Some(Self::Int),
            Self::ByteArray => Some(Self::Byte),
            Self::CharArray => Some(Self::Char),
            Self::LongArray => Some(Self::Long),
            Self::BooleanArray => Some(Self::Boolean),
            Self::FloatArray => Some(Self::Float),
            Self::DoubleArray => Some(Self::Double),
            _ => None,
        }
    }
    pub fn array_element(&self) -> Option<Type> {
        self.primitive_array_element()
            .or_else(|| match self.base() {
                Self::StringArray => Some(Self::String),
                Self::Array(element) => Some(element.as_ref().clone()),
                _ => None,
            })
    }
    pub fn primitive_array_for(element: &Type) -> Option<Type> {
        match element {
            Self::Int => Some(Self::IntArray),
            Self::Byte => Some(Self::ByteArray),
            Self::Char => Some(Self::CharArray),
            Self::Long => Some(Self::LongArray),
            Self::Boolean => Some(Self::BooleanArray),
            Self::Float => Some(Self::FloatArray),
            Self::Double => Some(Self::DoubleArray),
            _ => None,
        }
    }
    pub fn primitive_array_name(&self) -> Option<&'static str> {
        match self.primitive_array_element()? {
            Self::Int => Some("Int"),
            Self::Byte => Some("Byte"),
            Self::Char => Some("Char"),
            Self::Long => Some("Long"),
            Self::Boolean => Some("Boolean"),
            Self::Float => Some("Float"),
            Self::Double => Some("Double"),
            _ => None,
        }
    }
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
                | Self::LongArray
                | Self::BooleanArray
                | Self::FloatArray
                | Self::DoubleArray
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
        if matches!(self.base(), Self::Long | Self::Double) {
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
            Self::Double => "D".into(),
            Self::IntArray => "[I".into(),
            Self::ByteArray => "[B".into(),
            Self::CharArray => "[C".into(),
            Self::LongArray => "[J".into(),
            Self::BooleanArray => "[Z".into(),
            Self::FloatArray => "[F".into(),
            Self::DoubleArray => "[D".into(),

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
                Self::Double => "Ljava/lang/Double;".into(),
                _ => inner.descriptor(),
            },
            Self::Platform(_) => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub type_locations: Vec<(String, usize)>,
    pub type_aliases: Vec<TypeAliasDecl>,
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
pub struct TypeAliasDecl {
    pub type_params: Vec<TypeParameter>,
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub expanded: Type,
}
#[derive(Clone, Debug)]
pub struct AnnotationUse {
    pub name: String,
    pub offset: usize,
    pub use_site: Option<String>,
    pub arguments: Vec<(Option<String>, Expr)>,
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
    pub sealed_: bool,
    pub type_params: Vec<TypeParameter>,
    pub super_types: Vec<Type>,
    pub annotations: Vec<AnnotationUse>,
    pub fun_interface: bool,
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
    pub annotations: Vec<AnnotationUse>,
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
    pub annotations: Vec<AnnotationUse>,
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
    pub body: ClassDecl,
    pub entry_arguments: Vec<Vec<Expr>>,
    pub annotations: Vec<AnnotationUse>,
    pub name: String,
    pub offset: usize,
    pub entries: Vec<(String, usize)>,
}
#[derive(Clone, Debug)]
pub struct ObjectDecl {
    pub annotation_uses: Vec<AnnotationUse>,
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
    pub annotations: Vec<AnnotationUse>,
    pub is_override: bool,
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub internal: bool,
    pub getter_visibility: Option<PropertyVisibility>,
    pub setter_visibility: Option<PropertyVisibility>,
    pub initializer: Option<Expr>,
    pub ty: Option<Type>,
    pub mutable: bool,
    pub body: Vec<Statement>,
    pub setter: Option<(String, Vec<Statement>)>,
    pub delegate: Option<Expr>,
    pub lateinit: bool,
}
impl ObjectField {
    pub fn visibility(&self) -> PropertyVisibility {
        if self.private {
            PropertyVisibility::Private
        } else if false {
            PropertyVisibility::Protected
        } else if self.internal {
            PropertyVisibility::Internal
        } else {
            PropertyVisibility::Public
        }
    }
    pub fn getter_visibility(&self) -> PropertyVisibility {
        self.getter_visibility.unwrap_or_else(|| self.visibility())
    }
    pub fn setter_visibility(&self) -> PropertyVisibility {
        self.setter_visibility.unwrap_or_else(|| self.visibility())
    }
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
    pub annotations: Vec<AnnotationUse>,
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub internal: bool,
    pub value: ConstValue,
}
#[derive(Clone, Debug)]
pub enum ConstValue {
    Expression {
        expression: Box<Expr>,
        declared_type: Option<Type>,
    },
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    Char(u16),
    Boolean(bool),
    String(Vec<u16>),
}
#[derive(Clone, Debug)]
pub struct ClassDecl {
    pub interface_delegates: Vec<InterfaceDelegate>,
    pub sealed_: bool,
    pub type_params: Vec<TypeParameter>,
    pub super_types: Vec<Type>,
    pub param_annotations: Vec<Vec<AnnotationUse>>,
    pub annotations: Vec<AnnotationUse>,
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
pub struct InterfaceDelegate {
    pub ty: Type,
    pub expression: Expr,
    pub offset: usize,
}
#[derive(Clone, Debug)]
pub struct ClassCompanion {
    pub properties: Vec<ClassCompanionProperty>,
    pub methods: Vec<ClassMethod>,
    pub initializers: Vec<(usize, Vec<Statement>)>,
}
#[derive(Clone, Debug)]
pub struct ClassCompanionProperty {
    pub annotations: Vec<AnnotationUse>,
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub internal: bool,
    pub getter_visibility: Option<PropertyVisibility>,
    pub setter_visibility: Option<PropertyVisibility>,
    pub mutable: bool,
    pub ty: Option<Type>,
    pub initializer: Option<Expr>,
    pub body: Vec<Statement>,
    pub setter: Option<(String, Vec<Statement>)>,
    pub delegate: Option<Expr>,
    pub lateinit: bool,
}
impl ClassCompanionProperty {
    pub fn visibility(&self) -> PropertyVisibility {
        if self.private {
            PropertyVisibility::Private
        } else if false {
            PropertyVisibility::Protected
        } else if self.internal {
            PropertyVisibility::Internal
        } else {
            PropertyVisibility::Public
        }
    }
    pub fn getter_visibility(&self) -> PropertyVisibility {
        self.getter_visibility.unwrap_or_else(|| self.visibility())
    }
    pub fn setter_visibility(&self) -> PropertyVisibility {
        self.setter_visibility.unwrap_or_else(|| self.visibility())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyVisibility {
    Public,
    Private,
    Protected,
    Internal,
}
impl PropertyVisibility {
    pub fn access(self) -> u16 {
        match self {
            Self::Private => 2,
            Self::Protected => 4,
            Self::Public | Self::Internal => 1,
        }
    }
    pub fn metadata(self) -> u32 {
        match self {
            Self::Private => 2,
            Self::Protected => 4,
            Self::Public => 6,
            Self::Internal => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PropertyDecl {
    pub annotations: Vec<AnnotationUse>,
    pub name: String,
    pub private: bool,
    pub internal: bool,
    pub getter_visibility: Option<PropertyVisibility>,
    pub setter_visibility: Option<PropertyVisibility>,
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
impl PropertyDecl {
    pub fn visibility(&self) -> PropertyVisibility {
        if self.private {
            PropertyVisibility::Private
        } else if self.protected {
            PropertyVisibility::Protected
        } else if self.internal {
            PropertyVisibility::Internal
        } else {
            PropertyVisibility::Public
        }
    }
    pub fn getter_visibility(&self) -> PropertyVisibility {
        self.getter_visibility.unwrap_or_else(|| self.visibility())
    }
    pub fn setter_visibility(&self) -> PropertyVisibility {
        self.setter_visibility.unwrap_or_else(|| self.visibility())
    }
    pub fn new_visibility(&self) -> bool {
        self.internal || self.getter_visibility.is_some() || self.setter_visibility.is_some()
    }
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
    pub param_annotations: Vec<Vec<AnnotationUse>>,
    pub annotations: Vec<AnnotationUse>,
    pub name: String,
    pub offset: usize,
    pub private: bool,
    pub protected: bool,
    pub internal: bool,
    pub infix: bool,
    pub operator: bool,
    pub vararg: bool,
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestructureEntry {
    pub name: Option<String>,
    pub ty: Option<Type>,
}
#[derive(Clone, Debug)]
pub enum StatementKind {
    Block(Vec<Statement>),
    LocalFunction(Box<Function>),
    Labeled(String, Box<Statement>),
    BreakLabel(String),
    ContinueLabel(String),
    Destructure(Vec<DestructureEntry>, bool, Expr),
    Let(String, bool, Option<Type>, Expr),
    Declare(String, bool, Type),
    IndexAssign(Expr, Expr, Expr),
    IndicesAssign(Expr, Vec<Expr>, Expr),
    PropertyAssign(Expr, String, Expr),
    SafePropertyAssign(Expr, String, Expr),
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
pub(crate) fn normalize_result_tail(body: &mut [Statement]) {
    let Some(last) = body.last_mut() else {
        return;
    };
    let offset = last.offset;
    let block = |mut body: Vec<Statement>| {
        normalize_result_tail(&mut body);
        Expr {
            offset: body.first().map_or(offset, |statement| statement.offset),
            kind: ExprKind::Block(body),
        }
    };
    let expression = match last.kind.clone() {
        StatementKind::If(condition, yes, no) if !no.is_empty() => Some(ExprKind::If(
            Box::new(condition),
            Box::new(block(yes)),
            Box::new(block(no)),
        )),
        StatementKind::When(subject, branches, otherwise) => Some(ExprKind::When(
            Box::new(subject),
            branches
                .into_iter()
                .map(|(conditions, body)| (conditions, block(body)))
                .collect(),
            Box::new(otherwise.map(block).unwrap_or(Expr {
                offset,
                kind: ExprKind::MissingWhenElse,
            })),
        )),
        StatementKind::WhenBinding(name, subject, branches, otherwise) => {
            Some(ExprKind::WhenBinding(
                name,
                Box::new(subject),
                branches
                    .into_iter()
                    .map(|(conditions, body)| (conditions, block(body)))
                    .collect(),
                Box::new(otherwise.map(block).unwrap_or(Expr {
                    offset,
                    kind: ExprKind::MissingWhenElse,
                })),
            ))
        }
        _ => None,
    };
    if let Some(kind) = expression {
        last.kind = StatementKind::Expression(Expr { offset, kind });
    }
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    MissingWhenElse,
    WhenSubjectless,
    VarargArray {
        array: Type,
        elements: Vec<Expr>,
    },
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
    Double(u64),
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
    ClassLiteral {
        receiver: Box<Expr>,
        classifier: Option<Type>,
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
            ExprKind::ClassLiteral {
                receiver: value, ..
            }
            | ExprKind::CallableReference {
                receiver: Some(value),
                ..
            } => pending.push(value),
            ExprKind::LambdaWithParams { body, .. } => shift_statement_offsets(body, offset),
            ExprKind::Template(parts)
            | ExprKind::VarargArray {
                elements: parts, ..
            } => pending.extend(parts),
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
                for annotation in function
                    .annotations
                    .iter_mut()
                    .chain(function.param_annotations.iter_mut().flatten())
                {
                    annotation.offset += offset;
                    for (_, argument) in &mut annotation.arguments {
                        shift_expression_offsets(argument, offset);
                    }
                }
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
            StatementKind::PropertyAssign(target, _, value)
            | StatementKind::SafePropertyAssign(target, _, value) => {
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

fn escaped_identifier_end(source: &str, start: usize) -> Result<usize> {
    let bytes = source.as_bytes();
    let mut end = start + 1;
    while end < bytes.len() && bytes[end] != b'`' {
        if matches!(bytes[end], b'\n' | b'\r') {
            return Err(Diagnostic::new(
                start,
                "Escaped identifiers cannot contain newlines",
            ));
        }
        end += 1;
    }
    if end == bytes.len() {
        return Err(Diagnostic::new(start, "Unterminated escaped identifier"));
    }
    if end == start + 1 {
        return Err(Diagnostic::new(
            start,
            "Escaped identifiers cannot be empty",
        ));
    }
    Ok(end + 1)
}

fn quoted_end(source: &str, start: usize, depth: usize) -> Result<usize> {
    if depth > 64 {
        return Err(Diagnostic::new(
            start,
            "String interpolation nesting exceeds the current limit",
        ));
    }
    let bytes = source.as_bytes();
    let raw = bytes[start..].starts_with(b"\"\"\"");
    let mut i = start + if raw { 3 } else { 1 };
    while i < bytes.len() {
        match bytes[i] {
            b'"' if raw => {
                let end = i + bytes[i..].iter().take_while(|byte| **byte == b'"').count();
                if end - i >= 3 {
                    return Ok(end);
                }
                i = end;
            }
            b'"' => return Ok(i + 1),
            b'\\' if !raw => i += 2,
            b'\n' | b'\r' if !raw => {
                return Err(Diagnostic::new(
                    i,
                    "Newlines are not allowed in ordinary strings",
                ));
            }
            b'$' if bytes.get(i + 1) == Some(&b'{') => {
                i = template_end(source, i + 2, depth + 1)? + 1
            }
            b'$' if bytes.get(i + 1) == Some(&b'`') => {
                i = escaped_identifier_end(source, i + 1)?;
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
            b'`' => i = escaped_identifier_end(source, i)?,
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
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
    string_parts_depth(source, start, 0, &mut Vec::new())
}

fn string_parts_depth(
    source: &str,
    start: usize,
    depth: usize,
    type_locations: &mut Vec<(String, usize)>,
) -> Result<(usize, Vec<Expr>)> {
    if depth > 64 {
        return Err(Diagnostic::new(
            start,
            "String interpolation nesting exceeds the current limit",
        ));
    }
    let bytes = source.as_bytes();
    let raw = bytes[start..].starts_with(b"\"\"\"");
    let mut i = start + if raw { 3 } else { 1 };
    let mut units = Vec::new();
    let mut parts = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' if raw && !bytes[i..].starts_with(b"\"\"\"") => {
                units.push(34);
                i += 1;
            }
            b'"' => {
                let end = if raw {
                    let end = i + bytes[i..].iter().take_while(|byte| **byte == b'"').count();
                    units.extend(std::iter::repeat_n(34, end - i - 3));
                    end
                } else {
                    i + 1
                };
                if !units.is_empty() || parts.is_empty() {
                    parts.push(Expr {
                        offset: start,
                        kind: ExprKind::String(units),
                    });
                }
                return Ok((end, parts));
            }
            b'\r' if raw => {
                units.push(10);
                i += if bytes.get(i + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
            }
            b'\n' | b'\r' if !raw => {
                return Err(Diagnostic::new(
                    i,
                    "Newlines are not allowed in ordinary strings",
                ));
            }
            b'\\' if !raw => {
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
                    in_delegation_expression: false,
                    tokens,
                    pos: 0,
                    depth: depth + 1,
                    type_parameters: Vec::new(),
                    type_locations: Vec::new(),
                };
                parser.lines();
                let expression = parser.expr(0)?;
                parser.lines();
                if !parser.at("<eof>") {
                    return Err(
                        parser.error("Braced string templates require one supported expression")
                    );
                }
                type_locations.extend(parser.type_locations);
                parts.push(expression);
                if parts.len() > 256 {
                    return Err(Diagnostic::new(
                        start,
                        "String interpolation exceeds the current limit",
                    ));
                }
                i += 1;
            }
            b'$' if bytes.get(i + 1) == Some(&b'`') => {
                parts.push(Expr {
                    offset: start,
                    kind: ExprKind::String(std::mem::take(&mut units)),
                });
                let name_start = i + 1;
                i = escaped_identifier_end(source, name_start)?;
                let name = identifier_name(&source[name_start..i], name_start)?;
                parts.push(Expr {
                    offset: name_start,
                    kind: ExprKind::Name(name.into()),
                });
                if parts.len() > 256 {
                    return Err(Diagnostic::new(
                        start,
                        "String interpolation exceeds the current 128-name limit",
                    ));
                }
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
                if source[name_start..i].bytes().all(|byte| byte == b'_') {
                    return Err(Diagnostic::new(
                        name_start,
                        "Names consisting only of underscores are reserved",
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
                .is_some_and(|c| c == '{' || c.is_alphabetic()) =>
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
    let mut delimiters = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        let b = bytes[i];
        if matches!(b, b'\n' | b'\r') {
            if !matches!(delimiters.last(), Some(b'(' | b'[')) {
                tokens.push(Token {
                    text: "\n".into(),
                    offset: i,
                });
            }
            i += if b == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
        } else if b.is_ascii_whitespace() {
            i += 1;
        } else if bytes[i..].starts_with(b"//") {
            while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                i += 1;
            }
        } else if bytes[i..].starts_with(b"/*") {
            i += 2;
            let mut depth = 1;
            while i < bytes.len() && depth > 0 {
                if bytes[i..].starts_with(b"/*") {
                    depth += 1;
                    i += 2;
                } else if bytes[i..].starts_with(b"*/") {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err(Diagnostic::new(start, "Unterminated block comment"));
            }
        } else if b == b'"' {
            i = quoted_end(source, start, 0)?;
            tokens.push(Token {
                text: source[start..i].into(),
                offset: start,
            });
        } else if b == b'$'
            && bytes.get(i + bytes[i..].iter().take_while(|byte| **byte == b'$').count())
                == Some(&b'"')
        {
            return Err(Diagnostic::new(
                start,
                "Multi-dollar string interpolation is not supported",
            )
            .with_kind(
                DiagnosticKind::UnsupportedLanguage,
                "MULTI_DOLLAR_STRING_UNSUPPORTED",
            ));
        } else if b == b'`' {
            i = escaped_identifier_end(source, start)?;
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
        } else if b.is_ascii_digit()
            || b == b'.' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit)
        {
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
            match b {
                b'(' | b'[' | b'{' => delimiters.push(b),
                b')' | b']' | b'}' => {
                    delimiters.pop();
                }
                _ => {}
            }
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

struct ClassHeader {
    declaration: ClassDecl,
    property_names: std::collections::HashSet<String>,
    has_constructor: bool,
    after_header: usize,
}

struct Parser {
    in_delegation_expression: bool,
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    type_parameters: Vec<TypeParameter>,
    type_locations: Vec<(String, usize)>,
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
    fn at_trailing_lambda(&self) -> bool {
        !self.in_delegation_expression
            && (self.at("{")
                || (identifier(&self.token().text)
                    && self
                        .tokens
                        .get(self.pos + 1)
                        .is_some_and(|token| token.text == "@")
                    && self
                        .tokens
                        .get(self.pos + 2)
                        .is_some_and(|token| token.text == "{")))
    }
    fn lines_before_trailing_lambda(&mut self) -> Result<()> {
        let saved = self.pos;
        self.lines();
        if self.at("@") {
            let annotation_start = self.pos;
            let depth = self.depth;
            let type_location_count = self.type_locations.len();
            let trailing = self.annotations().is_ok() && self.at_trailing_lambda();
            self.pos = annotation_start;
            self.depth = depth;
            self.type_locations.truncate(type_location_count);
            if trailing {
                return Err(self.error("Annotated trailing lambdas require unsupported annotation-expression parsing")
                    .with_kind(DiagnosticKind::UnsupportedLanguage, "ANNOTATED_CALL_SUFFIX_UNSUPPORTED"));
            }
        }
        if !self.at_trailing_lambda() {
            self.pos = saved;
        }
        Ok(())
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
        let name = identifier_name(&self.token().text, self.token().offset)?.to_owned();
        self.pos += 1;
        Ok(name)
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
    fn reject_type_annotations(&self) -> Result<()> {
        if self.at("@") {
            return Err(self
                .error("Annotations on type uses require unsupported type-annotation placement semantics")
                .with_kind(DiagnosticKind::UnsupportedLanguage, "ANNOTATION_TYPE_USE_UNSUPPORTED"));
        }
        Ok(())
    }
    fn reject_parameterized_supertype(&self) -> Result<()> {
        let mut index = self.pos;
        loop {
            while self
                .tokens
                .get(index)
                .is_some_and(|token| token.text == "\n")
            {
                index += 1;
            }
            if self
                .tokens
                .get(index)
                .is_some_and(|token| token.text == ".")
            {
                index += 1;
                while self
                    .tokens
                    .get(index)
                    .is_some_and(|token| token.text == "\n")
                {
                    index += 1;
                }
                if self
                    .tokens
                    .get(index)
                    .is_some_and(|token| identifier(&token.text))
                {
                    index += 1;
                    continue;
                }
            }
            break;
        }
        if let Some(token) = self.tokens.get(index).filter(|token| token.text == "<") {
            return Err(Diagnostic::new(
                token.offset,
                "Parameterized supertypes require unsupported source-supertype substitution",
            )
            .with_kind(
                DiagnosticKind::UnsupportedLanguage,
                "PARAMETERIZED_SUPERTYPE_UNSUPPORTED",
            ));
        }
        Ok(())
    }
    fn nonnullable_ty(&mut self) -> Result<Type> {
        self.reject_type_annotations()?;
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
        let offset = self.token().offset;
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
        let receiver = self.named_ty(name, offset)?;
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
    fn record_type(&mut self, name: &str, offset: usize) {
        let name = match name {
            "Any" => "java/lang/Object",
            "CharSequence" => "java/lang/CharSequence",
            other => other,
        };
        self.type_locations.push((name.into(), offset));
    }
    fn named_ty(&mut self, name: String, offset: usize) -> Result<Type> {
        if name.contains('$')
            && !self
                .type_parameters
                .iter()
                .any(|parameter| parameter.name == name)
        {
            return Err(Diagnostic::new(
                offset,
                "Classifier names containing '$' require unsupported JVM nesting disambiguation",
            )
            .with_kind(
                DiagnosticKind::UnsupportedLanguage,
                "CLASSIFIER_DOLLAR_NAME_UNSUPPORTED",
            ));
        }
        self.record_type(&name, offset);
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
        if let Some(array) = Type::primitive_array_named(&name) {
            return Ok(array);
        }
        Ok(match name.as_str() {
            "Int" => Type::Int,
            "Long" => Type::Long,
            "Byte" => Type::Byte,
            "Char" => Type::Char,
            "Boolean" => Type::Boolean,
            "Float" => Type::Float,
            "Double" => Type::Double,
            "Short" | "ShortArray" | "kotlin/Short" | "kotlin/ShortArray" => {
                return Err(self
                    .error("Short and ShortArray semantics are unsupported")
                    .with_kind(DiagnosticKind::UnsupportedLanguage, "SHORT_UNSUPPORTED"));
            }
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
                ).with_kind(DiagnosticKind::UnsupportedLanguage, "TYPE_PARAMETER_MODIFIER_UNSUPPORTED"));
            }
            let name = self.name()?;
            if parameters
                .iter()
                .any(|parameter: &TypeParameter| parameter.name == name)
            {
                return Err(self.error("Duplicate type parameter"));
            }
            if self
                .type_parameters
                .iter()
                .any(|parameter| parameter.name == name)
            {
                return Err(self
                    .error("Shadowing an enclosing type parameter is unsupported")
                    .with_kind(
                        DiagnosticKind::UnsupportedLanguage,
                        "TYPE_PARAMETER_SHADOW_UNSUPPORTED",
                    ));
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
                        | Type::Double
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
        self.starts_declaration("object")
    }
    fn starts_class(&self) -> bool {
        self.starts_declaration("class")
    }
    fn starts_interface(&self) -> bool {
        self.starts_declaration("interface")
    }
    fn starts_declaration(&self, kind: &str) -> bool {
        let mut index = self.pos;
        loop {
            let Some(token) = self.tokens.get(index) else {
                return false;
            };
            if matches!(
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
            ) {
                index += 1;
            } else if token.text == "@" {
                index += 1;
                if !self
                    .tokens
                    .get(index)
                    .is_some_and(|token| identifier(&token.text))
                {
                    return false;
                }
                index += 1;
                if self
                    .tokens
                    .get(index)
                    .is_some_and(|token| token.text == ":")
                {
                    index += 2;
                }
                while self
                    .tokens
                    .get(index)
                    .is_some_and(|token| token.text == ".")
                {
                    index += 2;
                }
                if self
                    .tokens
                    .get(index)
                    .is_some_and(|token| token.text == "(")
                {
                    let mut depth = 1;
                    index += 1;
                    while depth > 0 {
                        let Some(token) = self.tokens.get(index) else {
                            return false;
                        };
                        match token.text.as_str() {
                            "(" => depth += 1,
                            ")" => depth -= 1,
                            "<eof>" => return false,
                            _ => {}
                        }
                        index += 1;
                    }
                }
            } else if token.text == "fun" && kind == "interface" {
                index += 1;
                while self
                    .tokens
                    .get(index)
                    .is_some_and(|token| token.text == "\n")
                {
                    index += 1;
                }
                return self
                    .tokens
                    .get(index)
                    .is_some_and(|token| token.text == "interface");
            } else {
                return token.text == kind;
            }
        }
    }
    fn annotation(&mut self) -> Result<AnnotationUse> {
        let offset = self.token().offset;
        self.expect("@")?;
        let first = self.declaration_name()?;
        let (use_site, mut name) = if self.take(":") {
            (Some(first), self.declaration_name()?)
        } else {
            (None, first)
        };
        while self.take(".") {
            name.push('/');
            name.push_str(&self.declaration_name()?);
        }
        let mut arguments = Vec::new();
        if self.at("(") {
            let mut index = self.pos + 1;
            let mut depth = 1;
            while let Some(token) = self.tokens.get(index) {
                match token.text.as_str() {
                    "(" => depth += 1,
                    ")" => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    "[" if self.tokens[..index]
                        .iter()
                        .rev()
                        .find(|previous| previous.text != "\n")
                        .is_some_and(|previous| {
                            matches!(previous.text.as_str(), "(" | "," | "=")
                        }) =>
                    {
                        return Err(Diagnostic::new(
                            token.offset,
                            "Annotation array literals require unsupported array-valued annotation arguments",
                        ).with_kind(DiagnosticKind::UnsupportedLanguage, "ANNOTATION_ARRAY_LITERAL_UNSUPPORTED"));
                    }
                    "<eof>" => break,
                    _ => {}
                }
                index += 1;
            }
            for argument in self.arguments()? {
                if let ExprKind::NamedArgument(name, value) = argument.kind {
                    arguments.push((Some(name), *value));
                } else {
                    arguments.push((None, argument));
                }
            }
        }
        Ok(AnnotationUse {
            name,
            offset,
            use_site,
            arguments,
        })
    }
    fn annotations(&mut self) -> Result<Vec<AnnotationUse>> {
        let mut annotations = Vec::new();
        while self.at("@") {
            annotations.push(self.annotation()?);
            self.lines();
        }
        Ok(annotations)
    }
    fn type_alias(&mut self) -> Result<TypeAliasDecl> {
        let offset = self.token().offset;
        let private = self.take("private");
        if !private {
            self.take("public");
        }
        self.lines();
        self.expect("typealias")?;
        let name = self.declaration_name()?;
        let scope = self.type_parameters.len();
        let result = (|| {
            let type_params = self.type_parameters().map_err(|error| {
                if error.message.contains("Variance and reified") {
                    error.with_kind(
                        DiagnosticKind::InvalidKotlin,
                        "INVALID_TYPE_ALIAS_PARAMETER",
                    )
                } else {
                    error
                }
            })?;
            if type_params
                .iter()
                .any(|parameter| !parameter.bounds.is_empty())
                || self.at("where")
            {
                return Err(self
                    .error("Type alias parameters cannot declare bounds")
                    .with_kind(
                        DiagnosticKind::InvalidKotlin,
                        "INVALID_TYPE_ALIAS_PARAMETER",
                    ));
            }
            self.expect("=")?;
            self.lines();
            let expanded = self.ty()?;
            let mut target = &expanded;
            while let Type::Nullable(inner) = target {
                target = inner;
            }
            if matches!(target, Type::Parameter { .. }) {
                return Err(Diagnostic::new(
                    offset,
                    "Type aliases must expand to a classifier or function type",
                )
                .with_kind(
                    DiagnosticKind::InvalidKotlin,
                    "TYPE_ALIAS_PARAMETER_TARGET_INVALID",
                ));
            }
            Ok(TypeAliasDecl {
                type_params,
                name,
                offset,
                private,
                expanded,
            })
        })();
        self.type_parameters.truncate(scope);
        result
    }
    fn enum_decl(&mut self) -> Result<EnumDecl> {
        let offset = self.token().offset;
        let mut annotations = self.annotations()?;
        let mut visibility = None;
        while self.at("@") || matches!(self.token().text.as_str(), "public" | "internal") {
            if self.at("@") {
                annotations.extend(self.annotations()?);
                continue;
            }
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
        if !self.at("class") {
            return Err(self.error("Expected class after enum"));
        }
        let ClassHeader {
            declaration: mut body,
            mut property_names,
            after_header,
            ..
        } = self.class_header()?;
        if !body.type_params.is_empty() {
            return Err(self
                .error("Enum classes cannot declare type parameters")
                .with_kind(
                    DiagnosticKind::InvalidKotlin,
                    "ENUM_TYPE_PARAMETERS_INVALID",
                ));
        }
        if !body.interface_delegates.is_empty() {
            return Err(self
                .error("Enum interface delegation is unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "INTERFACE_DELEGATION_ENUM_UNSUPPORTED",
                ));
        }
        body.offset = offset;
        body.annotations = annotations.clone();
        body.internal = visibility.as_deref() == Some("internal");
        body.primary_constructor = true;
        if body.defaults.iter().any(Option::is_some) {
            return Err(self.error("Enum constructor default arguments are unsupported"));
        }
        if body.super_class.is_some() {
            return Err(self.error("Explicit enum superclass constructors are unsupported"));
        }
        let name = body.name.clone();
        let mut entries = Vec::new();
        let mut entry_arguments = Vec::new();
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Ok(EnumDecl {
                    body,
                    entry_arguments,
                    annotations,
                    name,
                    offset,
                    entries,
                });
            }
            return Err(self.error("Expected an enum body or declaration separator"));
        }
        self.lines();
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
            self.lines();
            let arguments = if self.at("(") {
                self.arguments()?
            } else {
                Vec::new()
            };
            self.lines();
            if self.at("{") {
                return Err(self
                    .error("Enum entry-specific class bodies are unsupported")
                    .with_kind(
                        DiagnosticKind::UnsupportedLanguage,
                        "ENUM_ENTRY_BODY_UNSUPPORTED",
                    ));
            }
            entries.push((entry, entry_offset));
            entry_arguments.push(arguments);
            if entries.len() > 1024 {
                return Err(Diagnostic::new(
                    entry_offset,
                    "Enum exceeds the current 1024-entry limit",
                ));
            }
            if !self.take(",") {
                if self.at("<eof>") {
                    return Err(self.error("Unterminated enum body"));
                }
                if !self.at("}") && !self.at(";") {
                    return Err(self
                        .error("Expected a comma, semicolon, or closing brace after enum entry"));
                }
                break;
            }
            self.lines();
        }
        if self.take(";") {
            // Kotlin gives adjacent semicolons a distinct DOUBLE_SEMICOLON token.
            if self.at(";") && self.token().offset == self.tokens[self.pos - 1].offset + 1 {
                return Err(self
                    .error("Repeated enum entry terminators are not valid")
                    .with_kind(
                        DiagnosticKind::InvalidKotlin,
                        "ENUM_ENTRY_TERMINATOR_REPEATED",
                    ));
            }
            self.class_members(&mut body, &mut property_names)?;
            if let Some(property) = body.properties.iter().find(|p| p.new_visibility()) {
                return Err(Diagnostic::new(property.offset, "Independent accessor and internal property visibility in enum members is unsupported").with_kind(DiagnosticKind::UnsupportedLanguage, "PROPERTY_VISIBILITY_CONTEXT_UNSUPPORTED"));
            }
        } else {
            self.expect("}")?;
        }
        if !body.secondary_constructors.is_empty() {
            return Err(self.error("Secondary constructors in enums are unsupported"));
        }
        Ok(EnumDecl {
            body,
            entry_arguments,
            annotations,
            name,
            offset,
            entries,
        })
    }
    fn classifier_name(&mut self) -> Result<String> {
        let text = self.token().text.as_str();
        let name = text
            .strip_prefix('`')
            .and_then(|text| text.strip_suffix('`'))
            .unwrap_or(text);
        if name.contains('$') {
            return Err(self.error("Escaped classifier names containing '$' require unsupported JVM nesting disambiguation")
                .with_kind(DiagnosticKind::UnsupportedLanguage, "CLASSIFIER_DOLLAR_NAME_UNSUPPORTED"));
        }
        if matches!(
            name,
            "Int"
                | "Long"
                | "List"
                | "MutableList"
                | "Char"
                | "Byte"
                | "Boolean"
                | "Float"
                | "Double"
                | "IntArray"
                | "LongArray"
                | "BooleanArray"
                | "FloatArray"
                | "DoubleArray"
                | "ByteArray"
                | "ThreadLocal"
                | "CharArray"
                | "Array"
                | "StringArray"
                | "String"
                | "Unit"
        ) {
            return Err(self
                .error("Declarations shadowing builtin types are not supported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "BUILTIN_CLASSIFIER_SHADOW_UNSUPPORTED",
                ));
        }
        self.declaration_name()
    }
    fn internal_property_error(offset: usize) -> Diagnostic {
        Diagnostic::new(
            offset,
            "Internal property visibility is unsupported in this declaration context",
        )
        .with_kind(
            DiagnosticKind::UnsupportedLanguage,
            "INTERNAL_PROPERTY_VISIBILITY_UNSUPPORTED",
        )
    }
    fn reject_internal_property(&self) -> Result<()> {
        if self.at("internal") {
            return Err(Self::internal_property_error(self.token().offset));
        }
        Ok(())
    }
    fn declaration_name(&mut self) -> Result<String> {
        if matches!(self.token().text.as_str(), "catch" | "finally" | "import") {
            return Err(self.error("Expected an identifier"));
        }
        self.name()
    }
    fn object_decl(&mut self, depth: usize, class_count: &mut usize) -> Result<ObjectDecl> {
        let offset = self.token().offset;
        let mut annotation_uses = self.annotations()?;
        if depth > 16 {
            return Err(self.error("Object nesting exceeds the current 16-class depth limit"));
        }
        *class_count += 1;
        if *class_count > 256 {
            return Err(self.error("Objects exceed the current 256-class limit"));
        }
        let mut visibility = None;
        while self.at("@") || matches!(self.token().text.as_str(), "public" | "internal") {
            if self.at("@") {
                annotation_uses.extend(self.annotations()?);
                continue;
            }
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
        let annotations = annotation_uses
            .iter()
            .filter(|annotation| annotation.use_site.is_none() && annotation.arguments.is_empty())
            .map(|annotation| annotation.name.clone())
            .collect();
        if self.at("data") {
            return Err(self
                .error("Data object equality, hashing, and string contracts are unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "DATA_OBJECT_UNSUPPORTED",
                ));
        }
        if !self.take("object") {
            return Err(
                self.error("Unsupported object modifier; only public and internal are supported")
            );
        }
        self.lines();
        let name = self.classifier_name()?;
        let mut interfaces = Vec::new();
        let after_name = self.pos;
        let mut after_header = after_name;
        self.lines();
        if self.take(":") {
            loop {
                self.lines();
                self.reject_type_annotations()?;
                let type_offset = self.token().offset;
                let mut interface = self.declaration_name()?;
                while self.take(".") {
                    interface.push('/');
                    interface.push_str(&self.declaration_name()?);
                }
                self.record_type(&interface, type_offset);
                self.reject_parameterized_supertype()?;
                interfaces.push(interface);
                after_header = self.pos;
                if interfaces.len() > 128 {
                    return Err(self.error("Object exceeds the current 128-interface limit"));
                }
                self.lines();
                if self.at("by") {
                    return Err(self
                        .error("Delegating object declarations are unsupported")
                        .with_kind(
                            DiagnosticKind::UnsupportedLanguage,
                            "INTERFACE_DELEGATION_OBJECT_UNSUPPORTED",
                        ));
                }
                if !self.take(",") {
                    break;
                }
            }
        }
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
                    annotation_uses,
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
            let member_name = if self.starts_object() {
                let object = self.object_decl(depth + 1, class_count)?;
                let name = object.name.clone();
                objects.push(object);
                name
            } else if self.starts_declaration("fun") {
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
                let annotations = self.annotations()?;
                let is_override = self.take("override");
                let property_visibility = self.property_visibility_modifier()?;
                let private = property_visibility == Some(PropertyVisibility::Private);
                self.lines();
                let lateinit = self.take("lateinit");
                self.lines();
                let mut property =
                    self.property(member_offset, private, is_override, lateinit, false)?;
                property.internal = property_visibility == Some(PropertyVisibility::Internal);
                Self::validate_static_property_visibility(&property)?;
                property.annotations.splice(0..0, annotations);
                let name = property.name.clone();
                fields.push(ObjectField {
                    annotations: property.annotations,
                    is_override,
                    name: property.name,
                    offset: property.offset,
                    private: property.private,
                    internal: property.internal,
                    getter_visibility: property.getter_visibility,
                    setter_visibility: property.setter_visibility,
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
            annotation_uses,
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
        let mut annotations = Vec::new();
        let mut jvm_static = false;
        let mut throws = None;
        let mut suppress = None;
        while self.at("@") {
            let special = self.tokens.get(self.pos + 1).is_some_and(|token| {
                matches!(token.text.as_str(), "JvmStatic" | "Throws" | "Suppress")
            }) && !self
                .tokens
                .get(self.pos + 2)
                .is_some_and(|token| matches!(token.text.as_str(), ":" | "."));
            if !special {
                annotations.push(self.annotation()?);
                self.lines();
                continue;
            }
            self.expect("@")?;
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
                        if self.at("[") {
                            return Err(self.error("Annotation array literals require unsupported array-valued annotation arguments")
                                .with_kind(DiagnosticKind::UnsupportedLanguage, "ANNOTATION_ARRAY_LITERAL_UNSUPPORTED"));
                        }
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
        if inline && visibility.as_deref() != Some("private") {
            return Err(self.error("Only private inline object methods are supported"));
        }
        let mut function = self.function()?;
        function.annotations.splice(0..0, annotations);
        function.offset = offset;
        function.private = visibility.as_deref() == Some("private");
        function.internal = visibility.as_deref() == Some("internal");
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
        let mut annotations = self.annotations()?;
        let mut visibility = None;
        let mut is_const = false;
        loop {
            if self.at("@") {
                annotations.extend(self.annotations()?);
            } else if matches!(
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
        let initializer_start = self.pos;
        let expression = self.expr(0)?;
        let significant_tokens = self.tokens[initializer_start..self.pos]
            .iter()
            .filter(|token| token.text != "\n")
            .count();
        let literal = significant_tokens == 1
            && matches!(
                expression.kind,
                ExprKind::Int(_)
                    | ExprKind::Long(_)
                    | ExprKind::Float(_)
                    | ExprKind::Double(_)
                    | ExprKind::Char(_)
                    | ExprKind::String(_)
                    | ExprKind::MissingWhenElse
                    | ExprKind::WhenSubjectless
                    | ExprKind::Boolean(_)
            )
            || significant_tokens == 2
                && matches!(&expression.kind, ExprKind::Unary(operator, value)
                if matches!(operator.as_str(), "+" | "-")
                    && matches!(value.kind, ExprKind::Int(_) | ExprKind::Long(_) | ExprKind::Float(_) | ExprKind::Double(_)));
        let scalar_type = declared_type.as_ref().is_none_or(|ty| {
            matches!(
                ty,
                Type::Int
                    | Type::Long
                    | Type::Float
                    | Type::Double
                    | Type::Char
                    | Type::Boolean
                    | Type::String
            )
        });
        if !literal || !scalar_type {
            if !matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                return Err(self.error("Const initializers require one expression"));
            }
            return Ok(ConstDecl {
                annotations,
                name,
                offset,
                private: visibility.as_deref() == Some("private"),
                internal: visibility.as_deref() == Some("internal"),
                value: ConstValue::Expression {
                    expression: Box::new(expression),
                    declared_type,
                },
            });
        }
        self.pos = initializer_start;
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
        let (value, ty) = if starts_numeric_literal(&token.text) {
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
                    let value = i64::try_from(value).map_err(|_| {
                        self.error("Long constant literal is out of range")
                            .with_kind(
                                DiagnosticKind::InvalidKotlin,
                                "INTEGER_LITERAL_OUT_OF_RANGE",
                            )
                    })?;
                    let value = if negative == Some(true) {
                        -value
                    } else {
                        value
                    };
                    (ConstValue::Long(value), Type::Long)
                }
                ExprKind::Double(value) => {
                    let value = if negative == Some(true) {
                        (-f64::from_bits(value)).to_bits()
                    } else {
                        value
                    };
                    (ConstValue::Double(value), Type::Double)
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
            annotations,
            name,
            offset,
            private: visibility.as_deref() == Some("private"),
            internal: visibility.as_deref() == Some("internal"),
            value,
        })
    }
    fn class_decl(&mut self) -> Result<ClassDecl> {
        let scope = self.type_parameters.len();
        let ClassHeader {
            mut declaration,
            mut property_names,
            has_constructor,
            after_header,
        } = self.class_header()?;
        if !self.take("{") {
            self.pos = after_header;
            if matches!(self.token().text.as_str(), "\n" | ";" | "}" | "<eof>") {
                self.type_parameters.truncate(scope);
                return Ok(declaration);
            }
            return Err(self.error("Expected a class body or declaration separator"));
        }
        self.class_members(&mut declaration, &mut property_names)?;
        declaration.primary_constructor =
            has_constructor || declaration.secondary_constructors.is_empty();
        self.type_parameters.truncate(scope);
        Ok(declaration)
    }
    fn class_header(&mut self) -> Result<ClassHeader> {
        let offset = self.token().offset;
        let mut annotations = self.annotations()?;
        let mut visibility = None;
        let mut final_modifier = false;
        let mut data = false;
        let mut open = false;
        let mut abstract_ = false;
        let mut sealed_ = false;
        loop {
            if self.at("@") {
                annotations.extend(self.annotations()?);
            } else if self.at("value") {
                return Err(self
                    .error("Value classes require unsupported JVM boxing and unboxing semantics")
                    .with_kind(
                        DiagnosticKind::UnsupportedLanguage,
                        "VALUE_CLASS_JVM_BOXING_UNSUPPORTED",
                    ));
            } else if matches!(self.token().text.as_str(), "public" | "internal") {
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
            } else if self.take("sealed") {
                if sealed_ {
                    return Err(self.error("Repeated sealed modifier"));
                }
                sealed_ = true;
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
        if sealed_ && (abstract_ || open || final_modifier || data) {
            return Err(self
                .error("Conflicting sealed class modifiers")
                .with_kind(DiagnosticKind::InvalidKotlin, "SEALED_MODIFIER_INVALID"));
        }
        abstract_ |= sealed_;
        if matches!(self.token().text.as_str(), "private" | "protected") {
            return Err(self.error("Unsupported class visibility; use public or internal"));
        }
        if !self.take("class") {
            return Err(self.error("Only ordinary final classes are supported"));
        }
        self.lines();
        let name = self.classifier_name()?;
        let type_params = self.type_parameters()?;
        if sealed_ && !type_params.is_empty() {
            return Err(self
                .error("Source generic sealed classifiers are unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "SEALED_GENERIC_CLASSIFIER_UNSUPPORTED",
                ));
        }
        let before_visibility = self.pos;
        self.lines();
        if sealed_
            && matches!(
                self.token().text.as_str(),
                "public" | "internal" | "private" | "protected" | "constructor"
            )
        {
            if matches!(self.token().text.as_str(), "public" | "internal") {
                return Err(self
                    .error("Sealed constructors must be private or protected")
                    .with_kind(
                        DiagnosticKind::InvalidKotlin,
                        "SEALED_CONSTRUCTOR_VISIBILITY_INVALID",
                    ));
            }
            return Err(self
                .error("Explicit sealed constructor visibility is unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "SEALED_CONSTRUCTOR_VISIBILITY_UNSUPPORTED",
                ));
        }
        self.pos = before_visibility;
        let after_name = self.pos;
        self.lines();
        if self.at("@") {
            let annotation_start = self.pos;
            let depth = self.depth;
            let type_location_count = self.type_locations.len();
            let parsed = self.annotations().is_ok();
            while matches!(
                self.token().text.as_str(),
                "public" | "private" | "protected" | "internal"
            ) {
                self.pos += 1;
                self.lines();
            }
            let constructor = parsed && matches!(self.token().text.as_str(), "constructor" | "(");
            self.pos = annotation_start;
            self.depth = depth;
            self.type_locations.truncate(type_location_count);
            if annotation_start == after_name || constructor {
                return Err(self
                    .error("Primary constructor annotations are unsupported")
                    .with_kind(
                        DiagnosticKind::UnsupportedLanguage,
                        "CONSTRUCTOR_ANNOTATION_UNSUPPORTED",
                    ));
            }
        }
        let has_constructor = self.take("(");
        let mut params = Vec::new();
        let mut param_annotations = Vec::new();
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
                    let mut annotations = self.annotations()?;
                    let is_override = self.take("override");
                    self.reject_internal_property()?;
                    let private = self.take("private");
                    let public = !private && self.take("public");
                    self.lines();
                    annotations.extend(self.annotations()?);
                    self.reject_internal_property()?;
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
                    param_annotations.push(annotations);
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
        let mut super_types = Vec::new();
        let mut interface_delegates = Vec::new();
        let mut super_class = None;
        if self.take(":") {
            self.lines();
            loop {
                let ty = self.ty()?;
                let name = match &ty {
                    Type::Java(name) | Type::Parameterized { name, .. } => name.clone(),
                    _ => return Err(self.error("A supertype must name a class or interface")),
                };
                super_types.push(ty.clone());
                let before_by = self.pos;
                self.lines();
                let delegated = self.take("by");
                if !delegated {
                    self.pos = before_by;
                }
                if delegated {
                    self.lines();
                    let offset = self.token().offset;
                    if self.at("object") {
                        return Err(self
                            .error("Delegation to object expressions is unsupported")
                            .with_kind(
                                DiagnosticKind::UnsupportedLanguage,
                                "INTERFACE_DELEGATION_OBJECT_UNSUPPORTED",
                            ));
                    }
                    let old = self.in_delegation_expression;
                    self.in_delegation_expression = true;
                    let expression = self.expr(0);
                    self.in_delegation_expression = old;
                    interface_delegates.push(InterfaceDelegate {
                        ty,
                        expression: expression?,
                        offset,
                    });
                    interfaces.push(name);
                } else if self.at("(") {
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
        if self.at("where") {
            return Err(self
                .error("Multiple source classifier bounds in where clauses are unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "CLASSIFIER_MULTIPLE_BOUNDS_UNSUPPORTED",
                ));
        }
        let declaration = ClassDecl {
            interface_delegates,
            sealed_,
            type_params,
            super_types,
            annotations,
            param_annotations,
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
        Ok(ClassHeader {
            declaration,
            property_names,
            has_constructor,
            after_header,
        })
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
                let mut annotations = self.annotations()?;
                let mut is_override = false;
                let mut visibility = None;
                let mut infix = false;
                let mut operator = false;
                let mut open = false;
                let mut final_ = false;
                let mut lateinit = false;
                let mut abstract_ = false;
                loop {
                    if self.at("@") {
                        annotations.extend(self.annotations()?);
                    } else if self.take("override") {
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
                    function.annotations = annotations;
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
                    property.annotations.splice(0..0, annotations);
                    property.protected = visibility.as_deref() == Some("protected");
                    property.internal = visibility.as_deref() == Some("internal");
                    Self::validate_property_visibility(&property)?;
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
    fn property_visibility_modifier(&mut self) -> Result<Option<PropertyVisibility>> {
        let mut visibility = None;
        while matches!(
            self.token().text.as_str(),
            "public" | "private" | "internal" | "protected"
        ) {
            if visibility.is_some() {
                return Err(self
                    .error("Conflicting or repeated property visibility modifiers")
                    .with_kind(
                        DiagnosticKind::InvalidKotlin,
                        "REPEATED_PROPERTY_VISIBILITY",
                    ));
            }
            visibility = Some(match self.token().text.as_str() {
                "private" => PropertyVisibility::Private,
                "internal" => PropertyVisibility::Internal,
                "protected" => {
                    return Err(self
                        .error("Protected properties require a class declaration")
                        .with_kind(DiagnosticKind::InvalidKotlin, "PROTECTED_PROPERTY_CONTEXT"));
                }
                _ => PropertyVisibility::Public,
            });
            self.pos += 1;
            self.lines();
        }
        Ok(visibility)
    }
    fn validate_static_property_visibility(property: &PropertyDecl) -> Result<()> {
        Self::validate_property_visibility(property)?;
        if property.getter_visibility() == PropertyVisibility::Protected
            || property.setter_visibility() == PropertyVisibility::Protected
        {
            return Err(Diagnostic::new(
                property.offset,
                "Protected accessors require an ordinary class member",
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "PROTECTED_ACCESSOR_CONTEXT"));
        }
        Ok(())
    }
    fn validate_property_visibility(property: &PropertyDecl) -> Result<()> {
        let visibility = property.visibility();
        if property.getter_visibility().ne(&visibility) {
            return Err(Diagnostic::new(
                property.offset,
                "Getter visibility must match property visibility",
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "GETTER_VISIBILITY_MISMATCH"));
        }
        let setter = property.setter_visibility();
        if property.mutable
            && !match visibility {
                PropertyVisibility::Public => true,
                PropertyVisibility::Private => setter == PropertyVisibility::Private,
                PropertyVisibility::Protected => matches!(
                    setter,
                    PropertyVisibility::Private | PropertyVisibility::Protected
                ),
                PropertyVisibility::Internal => matches!(
                    setter,
                    PropertyVisibility::Private | PropertyVisibility::Internal
                ),
            }
        {
            return Err(Diagnostic::new(
                property.offset,
                "Setter visibility cannot be more permissive than property visibility",
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "SETTER_VISIBILITY_MISMATCH"));
        }
        if property.open && setter == PropertyVisibility::Private && property.mutable {
            return Err(Diagnostic::new(
                property.offset,
                "Private setters are prohibited on open properties",
            )
            .with_kind(
                DiagnosticKind::InvalidKotlin,
                "PRIVATE_SETTER_ON_OPEN_PROPERTY",
            ));
        }
        if property.new_visibility()
            && (property.delegate.is_some() || property.lateinit || property.is_override)
        {
            return Err(Diagnostic::new(property.offset, "Independent accessor and internal visibility with delegates, lateinit, or overriding properties is unsupported").with_kind(DiagnosticKind::UnsupportedLanguage, "PROPERTY_VISIBILITY_COMBINATION_UNSUPPORTED"));
        }
        Ok(())
    }
    fn property(
        &mut self,
        offset: usize,
        private: bool,
        is_override: bool,
        lateinit: bool,
        open: bool,
    ) -> Result<PropertyDecl> {
        let annotations = self.annotations()?;
        let mutable = self.take("var");
        if !mutable {
            self.expect("val")?;
        }
        let name = self.declaration_name()?;
        if self.at(".") {
            return Err(self
                .error("Extension property declarations are unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "EXTENSION_PROPERTY_UNSUPPORTED",
                ));
        }
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
        let mut has_setter = false;
        let mut getter_visibility = None;
        let mut setter_visibility = None;
        loop {
            let save = self.pos;
            self.separators();
            let accessor_visibility = if matches!(
                self.token().text.as_str(),
                "public" | "private" | "protected" | "internal"
            ) && self.tokens[self.pos + 1..]
                .iter()
                .find(|token| token.text != "\n")
                .is_some_and(|token| matches!(token.text.as_str(), "get" | "set"))
            {
                let value = match self.token().text.as_str() {
                    "private" => PropertyVisibility::Private,
                    "protected" => PropertyVisibility::Protected,
                    "internal" => PropertyVisibility::Internal,
                    _ => PropertyVisibility::Public,
                };
                self.pos += 1;
                self.lines();
                Some(value)
            } else {
                None
            };
            if self.take("get") {
                if has_getter {
                    return Err(self.error("Repeated property getter"));
                }
                has_getter = true;
                getter_visibility = accessor_visibility;
                if self.at("(") {
                    body = self.getter_body()?;
                }
            } else if self.take("set") {
                if !mutable {
                    return Err(self
                        .error("Read-only properties cannot have setters")
                        .with_kind(DiagnosticKind::InvalidKotlin, "VAL_WITH_SETTER"));
                }
                if has_setter {
                    return Err(self.error("Repeated property setter"));
                }
                has_setter = true;
                setter_visibility = accessor_visibility;
                if self.take("(") {
                    let parameter = self.name()?;
                    if self.take(":") {
                        let parameter_type = self.ty()?;
                        if ty.as_ref() != Some(&parameter_type) {
                            return Err(self.error("Setter parameter types require the matching explicit property type"));
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
                }
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
            annotations,
            name,
            private,
            internal: false,
            getter_visibility,
            setter_visibility,
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
                    kind: ExprKind::NamedArgument(name, Box::new(self.argument_value()?)),
                }
            } else {
                self.argument_value()?
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
    fn argument_value(&mut self) -> Result<Expr> {
        let offset = self.token().offset;
        if self.take("*") {
            self.lines();
            Ok(Expr {
                offset,
                kind: ExprKind::Unary("*".into(), Box::new(self.expr(0)?)),
            })
        } else {
            self.expr(0)
        }
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
            interface_delegates: Vec::new(),
            sealed_: false,
            type_params: Vec::new(),
            super_types: Vec::new(),
            annotations: Vec::new(),
            param_annotations: Vec::new(),
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
            Self::validate_static_property_visibility(&property)?;
            properties.push(ClassCompanionProperty {
                annotations: property.annotations,
                name: property.name,
                offset: property.offset,
                private: property.private,
                internal: property.internal,
                getter_visibility: property.getter_visibility,
                setter_visibility: property.setter_visibility,
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
        let scope = self.type_parameters.len();
        let offset = self.token().offset;
        let mut annotations = self.annotations()?;
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
        let mut sealed_ = false;
        while self.at("@") || matches!(self.token().text.as_str(), "public" | "internal" | "sealed")
        {
            if self.at("@") {
                annotations.extend(self.annotations()?);
                continue;
            }
            if self.take("sealed") {
                if sealed_ {
                    return Err(self.error("Repeated sealed modifier"));
                }
                sealed_ = true;
                self.lines();
                continue;
            }
            if visibility.is_some() {
                return Err(self.error("Conflicting or repeated interface visibility modifiers"));
            }
            visibility = Some(self.token().text.clone());
            self.pos += 1;
            self.lines();
        }
        let fun_interface = self.take("fun");
        if sealed_ && fun_interface {
            return Err(self
                .error("Functional interfaces cannot be sealed")
                .with_kind(
                    DiagnosticKind::InvalidKotlin,
                    "SEALED_FUN_INTERFACE_INVALID",
                ));
        }
        self.lines();
        if !self.take("interface") {
            return Err(self.error("Only public or internal plain interfaces are supported"));
        }
        self.lines();
        let name = self.classifier_name()?;
        let type_params = self.type_parameters()?;
        if sealed_ && !type_params.is_empty() {
            return Err(self
                .error("Source generic sealed classifiers are unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "SEALED_GENERIC_CLASSIFIER_UNSUPPORTED",
                ));
        }
        let mut after_header = self.pos;
        self.lines();
        let mut interfaces = Vec::new();
        let mut super_types = Vec::new();
        if self.take(":") {
            self.lines();
            loop {
                let ty = self.ty()?;
                let name = match &ty {
                    Type::Java(name) | Type::Parameterized { name, .. } => name.clone(),
                    _ => return Err(self.error("An interface supertype must name an interface")),
                };
                super_types.push(ty);
                interfaces.push(name);
                if interfaces.len() > 32 {
                    return Err(self.error("Interfaces exceed the current 32-superinterface limit"));
                }
                after_header = self.pos;
                self.lines();
                if self.at("by") {
                    return Err(self
                        .error("Interfaces cannot delegate their supertypes")
                        .with_kind(
                            DiagnosticKind::InvalidKotlin,
                            "INTERFACE_DELEGATION_IN_INTERFACE_INVALID",
                        ));
                }
                if !self.take(",") {
                    break;
                }
                self.lines();
            }
        }
        if self.at("where") {
            return Err(self
                .error("Multiple source classifier bounds in where clauses are unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "CLASSIFIER_MULTIPLE_BOUNDS_UNSUPPORTED",
                ));
        }
        let mut declaration = InterfaceDecl {
            sealed_,
            type_params,
            super_types,
            annotations,
            fun_interface,
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
                self.type_parameters.truncate(scope);
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
                let mut annotations = self.annotations()?;
                let mut is_override = false;
                let mut public_modifier = false;
                let mut infix = false;
                let mut operator = false;
                loop {
                    if self.at("@") {
                        annotations.extend(self.annotations()?);
                    } else if self.take("override") {
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
                    function.annotations = annotations;
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
                        annotations,
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
        self.type_parameters.truncate(scope);
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
            let annotations = self.annotations()?;
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
                let property_visibility = self.property_visibility_modifier()?;
                let private = property_visibility == Some(PropertyVisibility::Private);
                self.lines();
                let lateinit = self.take("lateinit");
                self.lines();
                let mut property = self.property(offset, private, false, lateinit, false)?;
                property.internal = property_visibility == Some(PropertyVisibility::Internal);
                Self::validate_static_property_visibility(&property)?;
                property.annotations.splice(0..0, annotations);
                if !names.insert(property.name.clone()) {
                    return Err(Diagnostic::new(offset, "Duplicate companion property name"));
                }
                fields.push(ClassCompanionProperty {
                    annotations: property.annotations,
                    name: property.name,
                    offset: property.offset,
                    private: property.private,
                    internal: property.internal,
                    getter_visibility: property.getter_visibility,
                    setter_visibility: property.setter_visibility,
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
            self.reject_internal_property()?;
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
                let type_offset = self.token().offset;
                let interface = self.declaration_name()?;
                self.record_type(&interface, type_offset);
                self.reject_parameterized_supertype()?;
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
                    interface_delegates: Vec::new(),
                    sealed_: false,
                    type_params: Vec::new(),
                    super_types: Vec::new(),
                    annotations: Vec::new(),
                    param_annotations: Vec::new(),
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
                if let Some(property) = object.properties.iter().find(|p| p.new_visibility()) {
                    return Err(Diagnostic::new(property.offset, "Independent accessor and internal property visibility in anonymous object members is unsupported").with_kind(DiagnosticKind::UnsupportedLanguage, "PROPERTY_VISIBILITY_CONTEXT_UNSUPPORTED"));
                }
                CompanionValue::Object(Box::new(object))
            } else {
                return Err(self.error("Companions support only literal String arrays and anonymous interface object initializers"));
            };
            properties.push(CompanionProperty {
                annotations,
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
            && (function.vararg
                || function.params.len() != 1
                || function.defaults.iter().any(Option::is_some))
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
        let mut annotations = self.annotations()?;
        let mut visibility = None;
        let mut infix = false;
        let mut operator = false;
        loop {
            if self.at("@") {
                annotations.extend(self.annotations()?);
            } else if matches!(
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
        let type_offset = self.token().offset;
        let first = self.name()?;
        let (receiver, name) = if self.at("<") || self.at("?") || self.at(".") {
            let receiver = self.named_ty(first, type_offset)?;
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
        let mut param_annotations = Vec::new();
        let mut defaults = Vec::new();
        let mut vararg = false;
        if !self.at(")") {
            loop {
                let mut annotations = self.annotations()?;
                let parameter_vararg = self.take("vararg");
                if vararg {
                    return Err(self.error(if parameter_vararg {
                        "Repeated vararg parameters are unsupported"
                    } else {
                        "A vararg parameter must be the last parameter in this subset"
                    }));
                }
                vararg = parameter_vararg;
                annotations.extend(self.annotations()?);
                let name = self.name()?;
                self.expect(":")?;
                self.lines();
                let ty = self.ty()?;
                let ty = if parameter_vararg {
                    Type::primitive_array_for(&ty).unwrap_or_else(|| Type::Array(Box::new(ty)))
                } else {
                    ty
                };
                params.push((name, ty));
                param_annotations.push(annotations);
                self.lines();
                defaults.push(if self.take("=") {
                    if parameter_vararg {
                        return Err(
                            self.error("Vararg parameters with default values are unsupported")
                        );
                    }
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
        if infix && (vararg || params.len() != 1 || defaults.iter().any(Option::is_some)) {
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
                param_annotations,
                annotations,
                name,
                offset,
                private,
                protected: false,
                internal,
                infix,
                operator,
                vararg,
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
            if self.at("(") {
                let names = self.destructuring_names()?;
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
                    kind: ExprKind::WhenSubjectless,
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
                    ExprKind::SafeProperty(receiver, name) => {
                        StatementKind::SafePropertyAssign(*receiver, name, value)
                    }
                    _ => {
                        return Err(self.error(
                            "Only locals, array elements, and supported properties can be assigned",
                        ));
                    }
                }
            } else if matches!(self.token().text.as_str(), "+=" | "-=" | "*=" | "/=" | "%=") {
                if matches!(target.kind, ExprKind::SafeProperty(_, _)) {
                    return Err(self
                        .error("Compound safe-call assignments are unsupported")
                        .with_kind(
                            DiagnosticKind::UnsupportedLanguage,
                            "SAFE_COMPOUND_ASSIGNMENT_UNSUPPORTED",
                        ));
                }
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
            if op.starts_with('`') {
                let operator_offset = self.token().offset;
                op = self.name()?;
                if escaped_name_conflicts_with_operator(&op) {
                    return Err(Diagnostic::new(
                        operator_offset,
                        "Escaped infix names matching Kotlin operators are unsupported",
                    ));
                }
            } else {
                self.pos += if negated_operator { 2 } else { 1 };
            }
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
    fn class_literal_prefix(&mut self) -> Result<Option<Expr>> {
        if (!identifier(&self.token().text) && !self.at("("))
            || matches!(
                self.token().text.as_str(),
                "true" | "false" | "null" | "this" | "super"
            )
        {
            return Ok(None);
        }
        let position = self.pos;
        let depth = self.depth;
        let locations = self.type_locations.len();
        let mut invalid_nullable = false;
        let result = (|| {
            let classifier = self.ty().ok()?;
            let end = self.pos;
            if self.at("?:")
                && self
                    .tokens
                    .get(self.pos + 1)
                    .is_some_and(|token| token.text == ":")
                && self
                    .tokens
                    .get(self.pos + 2)
                    .is_some_and(|token| token.text == "class")
            {
                invalid_nullable = true;
                return None;
            }
            if !self.take("::") || !self.take("class") {
                return None;
            }
            let mut names = self.tokens[position..end]
                .iter()
                .take_while(|token| !matches!(token.text.as_str(), "<" | "?"))
                .filter(|token| !matches!(token.text.as_str(), "." | "\n" | "(" | ")"));
            let first = names.next()?;
            let mut receiver = Expr {
                offset: first.offset,
                kind: ExprKind::Name(identifier_name(&first.text, first.offset).ok()?.into()),
            };
            for name in names {
                receiver = Expr {
                    offset: receiver.offset,
                    kind: ExprKind::Property(
                        Box::new(receiver),
                        identifier_name(&name.text, name.offset).ok()?.into(),
                    ),
                };
            }
            Some(Expr {
                offset: self.tokens[position].offset,
                kind: ExprKind::ClassLiteral {
                    receiver: Box::new(receiver),
                    classifier: Some(classifier),
                },
            })
        })();
        if result.is_none() {
            self.pos = position;
            self.depth = depth;
            self.type_locations.truncate(locations);
        }
        if invalid_nullable {
            return Err(Diagnostic::new(
                self.tokens[position].offset,
                "Nullable class literal classifiers are invalid",
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "INVALID_CLASS_LITERAL"));
        }
        Ok(result)
    }
    fn prefix(&mut self) -> Result<Expr> {
        let offset = self.token().offset;
        if self.at("++") || self.at("--") {
            let increment = self.take("++");
            if !increment {
                self.expect("--")?;
            }
            self.lines();
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
            self.lines();
            return Self::bounded(Expr {
                offset,
                kind: ExprKind::Unary(op, Box::new(self.expr(12)?)),
            });
        }
        let mut expr = if let Some(literal) = self.class_literal_prefix()? {
            literal
        } else if self
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
                    if self.at("_") {
                        return Err(self
                            .error("Discarded catch bindings are unsupported")
                            .with_kind(
                                DiagnosticKind::UnsupportedLanguage,
                                "DISCARD_CATCH_BINDING_UNSUPPORTED",
                            ));
                    }
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
            let mut type_locations = Vec::new();
            let mut parts = string_parts_depth(&self.token().text, 0, 0, &mut type_locations)
                .map_err(|mut error| {
                    error.offset += offset;
                    error
                })?
                .1;
            self.type_locations.extend(
                type_locations
                    .into_iter()
                    .map(|(name, type_offset)| (name, type_offset + offset)),
            );
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
        } else if starts_numeric_literal(&self.token().text) {
            let kind = numeric_literal(&self.token().text, self.token().offset)?;
            self.pos += 1;
            Expr { offset, kind }
        } else {
            let escaped = self
                .token()
                .text
                .strip_prefix('`')
                .and_then(|text| text.strip_suffix('`'));
            if escaped.is_some_and(|name| {
                name == "this"
                    || name == "super"
                    || name.starts_with("this@")
                    || name.starts_with("super@")
            }) {
                return Err(self.error("Escaped receiver-keyword references are unsupported"));
            }
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
            if self.at("\n")
                && let Some(next) = self.tokens[self.pos..]
                    .iter()
                    .position(|token| token.text != "\n")
                && matches!(self.tokens[self.pos + next].text.as_str(), "." | "?.")
            {
                self.pos += next;
            }
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
                self.lines_before_trailing_lambda()?;
            } else if self.take("::") {
                expr = if self.take("class") {
                    Expr {
                        offset,
                        kind: ExprKind::ClassLiteral {
                            receiver: Box::new(expr),
                            classifier: None,
                        },
                    }
                } else {
                    Expr {
                        offset,
                        kind: ExprKind::CallableReference {
                            receiver: Some(Box::new(expr)),
                            name: self.name()?,
                        },
                    }
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
            } else if self.at_trailing_lambda() {
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
                self.lines_before_trailing_lambda()?;
            } else if self.take("?.") {
                self.lines();
                let name = self.name()?;
                expr = Expr {
                    offset,
                    kind: ExprKind::SafeProperty(Box::new(expr), name),
                };
            } else if self.take(".") {
                self.lines();
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
            } else if self.at("(") {
                let args = self.arguments()?;
                expr = Expr {
                    offset,
                    kind: ExprKind::Call(Box::new(expr), args),
                };
                self.lines_before_trailing_lambda()?;
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
                    ExprKind::Template(parts)
                    | ExprKind::VarargArray {
                        elements: parts, ..
                    } => pending.extend(parts.iter().map(|part| (part, depth))),
                    ExprKind::When(subject, branches, otherwise)
                    | ExprKind::WhenBinding(_, subject, branches, otherwise) => {
                        pending.push((subject, depth));
                        pending.push((otherwise, depth));
                        for (conditions, value) in branches {
                            pending.extend(conditions.iter().map(|condition| (condition, depth)));
                            pending.push((value, depth));
                        }
                    }
                    ExprKind::ClassLiteral {
                        receiver: value, ..
                    }
                    | ExprKind::CallableReference {
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
                kind: ExprKind::Unary(operator.into(), Box::new(self.when_condition_expression()?)),
            })
        } else if subjectless {
            self.expr(0)
        } else {
            self.when_condition_expression()
        }
    }
    fn when_condition_expression(&mut self) -> Result<Expr> {
        let start = self.pos;
        let expression = self.expr(0)?;
        let mut nesting = 0;
        let parenthesized = self.tokens[start].text == "("
            && self.tokens[start..self.pos]
                .iter()
                .enumerate()
                .all(|(index, token)| {
                    match token.text.as_str() {
                        "(" => nesting += 1,
                        ")" => nesting -= 1,
                        _ => {}
                    }
                    nesting > 0 || index + 1 == self.pos - start
                });
        let confusing = match &expression.kind {
            ExprKind::TypeOperation(operator, _, _) => matches!(operator.as_str(), "is" | "!is"),
            ExprKind::Binary(operator, _, _) => matches!(
                operator.as_str(),
                "in" | "!in" | "<" | "<=" | ">" | ">=" | "==" | "!=" | "===" | "!==" | "&&" | "||"
            ),
            _ => false,
        };
        if confusing && !parenthesized {
            return Err(Diagnostic::new(
                expression.offset,
                "Boolean operators in a when branch condition with a subject require parentheses",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "CONFUSING_BRANCH_CONDITION_ERROR",
            ));
        }
        Ok(expression)
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
                kind: ExprKind::WhenSubjectless,
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
        let otherwise = otherwise.unwrap_or(Expr {
            offset,
            kind: ExprKind::MissingWhenElse,
        });
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
        let type_location_count = self.type_locations.len();
        let result = self.type_arguments().ok().and_then(|arguments| {
            if !self.at("(") && !self.at_trailing_lambda() {
                self.lines();
                if !self.at_trailing_lambda() && !self.at("@") {
                    return None;
                }
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
            self.type_locations.truncate(type_location_count);
        }
        result
    }
    fn destructuring_names(&mut self) -> Result<Vec<DestructureEntry>> {
        self.expect("(")?;
        self.lines();
        let mut names = Vec::new();
        loop {
            let name = if self.take("_") {
                None
            } else {
                Some(self.declaration_name()?)
            };
            self.lines();
            let ty = if self.take(":") {
                self.lines();
                Some(self.ty()?)
            } else {
                None
            };
            names.push(DestructureEntry { name, ty });
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
        let type_location_count = self.type_locations.len();
        let mut destructures = Vec::new();
        let params = if self.take("->") {
            Some(Vec::new())
        } else {
            let mut parameters = Vec::new();
            let parsed = (|| -> Result<()> {
                loop {
                    let parameter_offset = self.token().offset;
                    let discard = self.at("_");
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
                    parameters.push(LambdaParameter { name, discard, ty });
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
                self.type_locations.truncate(type_location_count);
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
        normalize_result_tail(&mut body);
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
                StatementKind::PropertyAssign(target, _, value)
                | StatementKind::SafePropertyAssign(target, _, value) => {
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
                ExprKind::ClassLiteral { receiver, .. }
                | ExprKind::CallableReference {
                    receiver: Some(receiver),
                    ..
                } => pending.push((receiver, depth + 1)),
                ExprKind::Template(parts)
                | ExprKind::VarargArray {
                    elements: parts, ..
                } => pending.extend(parts.iter().map(|part| (part, depth + 1))),
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
    if let Some(digits) = ["uL", "UL", "u", "U"]
        .iter()
        .find_map(|suffix| text.strip_suffix(suffix))
    {
        let (digits, radix) = if let Some(value) = digits
            .strip_prefix("0x")
            .or_else(|| digits.strip_prefix("0X"))
        {
            (value, 16)
        } else if let Some(value) = digits
            .strip_prefix("0b")
            .or_else(|| digits.strip_prefix("0B"))
        {
            (value, 2)
        } else {
            (digits, 10)
        };
        if !digits.is_empty()
            && !digits.starts_with('_')
            && !digits.ends_with('_')
            && !(radix == 10 && digits.len() > 1 && digits.starts_with('0'))
            && u64::from_str_radix(&digits.replace('_', ""), radix).is_ok()
        {
            return Err(
                Diagnostic::new(offset, "Unsigned numeric literals are unsupported").with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "UNSIGNED_LITERAL_UNSUPPORTED",
                ),
            );
        }
    }
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
        let bytes = text.as_bytes();
        if bytes.iter().enumerate().any(|(index, byte)| {
            *byte == b'_'
                && (index == 0
                    || index + 1 == bytes.len()
                    || !bytes[index - 1].is_ascii_digit()
                    || !bytes[index + 1].is_ascii_digit())
        }) {
            return Err(Diagnostic::new(
                offset,
                "Double literals require separators between decimal digits",
            ));
        }
        let value: f64 = text
            .replace('_', "")
            .parse()
            .map_err(|_| Diagnostic::new(offset, "Invalid Double literal"))?;
        return Ok(ExprKind::Double(value.to_bits()));
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

fn starts_numeric_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.first().is_some_and(u8::is_ascii_digit)
        || (bytes.first() == Some(&b'.') && bytes.get(1).is_some_and(u8::is_ascii_digit))
}

fn identifier_name(text: &str, offset: usize) -> Result<&str> {
    if text.bytes().all(|byte| byte == b'_') {
        return Err(Diagnostic::new(
            offset,
            "Names consisting only of underscores are reserved",
        ));
    }
    if !identifier(text) {
        return Err(Diagnostic::new(offset, "Expected an identifier"));
    }
    let name = text
        .strip_prefix('`')
        .and_then(|text| text.strip_suffix('`'))
        .unwrap_or(text);
    if name
        .chars()
        .any(|character| ".;[]/<>:\\".contains(character) || character.is_control())
    {
        return Err(Diagnostic::new(
            offset,
            "Identifier contains a character forbidden in JVM names",
        ));
    }
    Ok(name)
}

fn identifier(text: &str) -> bool {
    if let Some(name) = text
        .strip_prefix('`')
        .and_then(|text| text.strip_suffix('`'))
    {
        return !name.is_empty() && !name.contains(['`', '\n', '\r']);
    }
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

fn escaped_name_conflicts_with_operator(name: &str) -> bool {
    precedence(name).is_some_and(|precedence| precedence != 7)
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
        in_delegation_expression: false,
        tokens: lex(source)?,
        pos: 0,
        depth: 0,
        type_parameters: Vec::new(),
        type_locations: Vec::new(),
    };
    p.separators();
    if p.at("@")
        && p.tokens
            .get(p.pos + 1)
            .is_some_and(|token| token.text == "file")
        && p.tokens
            .get(p.pos + 2)
            .is_some_and(|token| token.text == ":")
    {
        return Err(p
            .error("File annotations require unsupported package-header annotation semantics")
            .with_kind(
                DiagnosticKind::UnsupportedLanguage,
                "FILE_ANNOTATION_UNSUPPORTED",
            ));
    }
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
    let mut type_aliases = Vec::new();
    let mut functions = Vec::new();
    let mut constants = Vec::new();
    let mut properties = Vec::new();
    let mut enum_decl = None;
    let mut object_decl = None;
    let mut class_decl = None;
    let mut interface_decl = None;
    let mut declarations = Vec::new();
    while !p.at("<eof>") {
        if p.starts_declaration("typealias") {
            type_aliases.push(p.type_alias()?);
        } else if p.starts_declaration("const") {
            constants.push(p.constant()?);
        } else if p.starts_declaration("val") || p.starts_declaration("var") {
            let offset = p.token().offset;
            let mut annotations = p.annotations()?;
            let property_visibility = p.property_visibility_modifier()?;
            let private = property_visibility == Some(PropertyVisibility::Private);
            p.lines();
            annotations.extend(p.annotations()?);
            let lateinit = p.take("lateinit");
            p.lines();
            let mut property = p.property(offset, private, false, lateinit, false)?;
            property.internal = property_visibility == Some(PropertyVisibility::Internal);
            Parser::validate_static_property_visibility(&property)?;
            property.annotations.splice(0..0, annotations);
            properties.push(property);
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
        && type_aliases.is_empty()
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
        type_locations: p.type_locations,
        type_aliases,
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
    fn adjacent_nullable_class_literals_have_an_invalid_diagnostic() {
        for source in [
            "fun bad() = String?::class",
            "val String: Any = 7\nfun bad() = String?::class",
        ] {
            let error = parse(source).unwrap_err();
            assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
            assert_eq!(error.code, "INVALID_CLASS_LITERAL");
        }
    }

    #[test]
    fn constructor_annotation_guards_preserve_separate_annotated_declarations() {
        assert!(parse("class Empty\n@Ann class Next").is_ok());
        assert!(parse("class Empty\n@Suppress(\"unused\") public class Next").is_ok());
        for source in [
            "class Bad @Ann constructor()",
            "class Bad\n@Ann\nconstructor()",
        ] {
            let error = parse(source).err().unwrap();
            assert_eq!(error.code, "CONSTRUCTOR_ANNOTATION_UNSUPPORTED");
            assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        }
    }

    #[test]
    fn generic_type_alias_parameters_are_scoped_and_reject_invalid_declarations() {
        let module = parse("typealias Values<T> = List<T>\ntypealias Callback<T> = ((T?) -> T)?\nfun later(value: T): T = value").unwrap();
        assert_eq!(module.type_aliases[0].type_params[0].name, "T");
        assert!(
            matches!(&module.type_aliases[0].expanded, Type::List(inner) if matches!(inner.as_ref(), Type::Parameter { name, .. } if name == "T"))
        );
        assert_eq!(module.functions[0].params[0].1, Type::Java("T".into()));
        for source in [
            "typealias Identity<T> = T",
            "typealias Identity<T> = T?",
            "typealias Values<T: Any> = List<T>",
            "typealias Values<in T> = List<T>",
            "typealias Values<out T> = List<T>",
            "typealias Values<reified T> = List<T>",
        ] {
            assert_eq!(
                parse(source).err().unwrap().kind,
                DiagnosticKind::InvalidKotlin,
                "{source}"
            );
        }
    }

    #[test]
    fn raw_literals_preserve_utf16_backslashes_quotes_and_normalize_line_endings() {
        for (source, expected) in [
            (r#""""\n\u0041\""""#, "\\n\\u0041\\"),
            ("\"\"\"a\r\nb\rc\nd\"\"\"", "a\nb\nc\nd"),
            (r#""""😀é""""#, "😀é"),
            (r#""""""""#, ""),
            (r#"""""""""#, "\""),
            (r#"""""""""""#, "\"\"\""),
            (r#""""" a """"""#, "\" a \"\""),
        ] {
            let (end, units) = string_literal(source, 0).unwrap();
            assert_eq!(end, source.len(), "{source:?}");
            assert_eq!(
                units,
                expected.encode_utf16().collect::<Vec<_>>(),
                "{source:?}"
            );
            assert_eq!(quoted_end(source, 0, 0).unwrap(), source.len());
        }
        for source in [r#""""unterminated"#, r#""""${1"#, "\"ordinary\nnewline\""] {
            assert!(quoted_end(source, 0, 0).is_err(), "{source:?}");
        }
    }

    #[test]
    fn trailing_lambda_newlines_follow_call_suffixes_without_crossing_parentheses() {
        for source in [
            "fun f(): Int = invokeInt()\n{ 3 }",
            "fun f(): Int = invokeInt<Int>\n{ 3 }",
            "fun f(): Int = invokeInt()\nlabel@{ 3 }",
            "fun f(): Int = (invokeInt\n{ 3 })",
        ] {
            let module = parse(source).unwrap();
            assert!(
                matches!(&module.functions[0].body[0].kind,
                StatementKind::Return(Some(Expr { kind: ExprKind::Call(_, arguments), .. })) if arguments.len() == 1),
                "{source}"
            );
        }
        for source in [
            "fun f(): Int = invokeInt\n{ 3 }",
            "fun f(): Int = (invokeInt())\n{ 3 }",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }

    #[test]
    fn multi_dollar_strings_remain_an_explicit_unsupported_boundary() {
        for source in [
            r#"fun f() = $$"""$literal $${1 + 2}""""#,
            r#"fun f() = $$"$literal $${1 + 2}""#,
        ] {
            let error = parse(source).err().unwrap();
            assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.code, "MULTI_DOLLAR_STRING_UNSUPPORTED");
            assert_eq!(error.offset, source.find('$').unwrap());
        }
    }

    #[test]
    fn raw_templates_allow_nested_quotes_raw_literals_and_multiline_expressions() {
        for source in [
            r#"fun f(name: String): String = """$name/${name.length}/\${'$'}foo""""#,
            r#"fun f(): String = """${"a${"""b"""}"}""""#,
            "fun f(): String = \"\"\"${\n(1\n+ 2)\n}\"\"\"",
            "fun f(): String = \"${\n\"\"\"raw\ntext\"\"\"\n}\"",
        ] {
            assert!(parse(source).is_ok(), "{source:?}");
        }
    }

    #[test]
    fn newline_expression_modes_preserve_statement_boundaries() {
        for source in [
            "fun f(s: String): Int = s\n.\nlength",
            "fun f(s: String?): Int? = s\n?.\nlength",
            "fun f(): Int = (1\n+\n2)",
            "fun f(): Int = first(1\n+ 2)",
            "fun f(a: IntArray): Int = a[1\n+ 2]",
            "fun f(): Int = -\n1",
            "fun f(): Int { val x = 1\n+ 2\nreturn x }",
            "fun f(): Int = first({ val x = 1\n+ 2\n x })",
            "fun f(): Int = (1 // comment\r + 2)",
            "fun f(): Int = (1 /* comment\n */ + 2)",
            "fun f(): Int = 1 /* comment\n */ + 2",
            "fun f(): Boolean = true\r&& false",
        ] {
            assert!(parse(source).is_ok(), "{source:?}");
        }
        let module = parse("fun f(): Int { val x = 1\n+ 2\nreturn x }").unwrap();
        assert_eq!(module.functions[0].body.len(), 3);
        let module = parse("fun f(): Int = first({ val x = 1\n+ 2\n x })").unwrap();
        let StatementKind::Return(Some(Expr {
            kind: ExprKind::Call(_, arguments),
            ..
        })) = &module.functions[0].body[0].kind
        else {
            panic!()
        };
        let ExprKind::LambdaWithParams { body, .. } = &arguments[0].kind else {
            panic!()
        };
        assert_eq!(body.len(), 3);
        assert!(parse("fun f(): Int = 1\n* 2").is_err());
    }

    #[test]
    fn unsupported_syntax_guards_preserve_supported_neighbors() {
        assert!(!starts_numeric_literal("."));
        assert!(!starts_numeric_literal(".."));
        assert!(starts_numeric_literal(".5"));
        assert!(parse("const val HALF: Double = .5\nfun half(): Double = .5").is_ok());
        assert!(parse("fun length(text: String): Int = text.length").is_ok());
        assert!(parse("fun numericStatements(): Double { 1\n.5\nreturn .5 }").is_ok());
        assert!(parse("class State { val value = 1\nprivate fun read(): Int = value }").is_ok());
        let invalid_navigation = parse("fun bad(value: String) { value\n. }").err().unwrap();
        assert_ne!(
            invalid_navigation.code,
            "NEWLINE_QUALIFIED_CONTINUATION_UNSUPPORTED"
        );
        let invalid_constant = parse("const val INVALID = .").err().unwrap();
        assert_ne!(invalid_constant.message, "Invalid Double literal");
        let generic_class = parse("class Generic<T>").unwrap();
        assert_eq!(generic_class.class_decl.unwrap().type_params[0].name, "T");
    }

    #[test]
    fn class_literal_guard_preserves_callable_references_and_specialized_throws() {
        for source in [
            "fun use(value: Owner) = consume(value::method, Owner::method, ::top)",
            "fun use(value: Owner) = consume(value::`class`, Owner::`class`, ::`class`)",
            "object Holder { @Throws(RuntimeException::class) fun run() {} }",
        ] {
            assert!(parse(source).is_ok(), "{source}");
        }
        let error = parse("fun invalid() = ::class").err().unwrap();
        assert_ne!(error.code, "CLASS_LITERAL_UNSUPPORTED");
    }

    #[test]
    fn dollar_classifier_names_are_explicitly_unsupported_without_restricting_members() {
        for source in [
            "class `Outer$Inner`",
            "object `Outer$Inner` {}",
            "interface `Outer$Inner` {}",
            "enum class `Outer$Inner` { A }",
            "class Outer { class `Inner$Deep` }",
        ] {
            let error = parse(source).err().unwrap();
            assert_eq!(error.code, "CLASSIFIER_DOLLAR_NAME_UNSUPPORTED", "{source}");
            assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.offset, source.find('`').unwrap(), "{source}");
        }

        let module = parse("class Outer { class Inner; class DefaultImpls; fun `name$module`() = 1; val `property$module`: Int = 1 }").unwrap();
        let outer = module.class_decl.unwrap();
        assert_eq!(outer.nested.len(), 2);
        assert_eq!(outer.methods[0].function.name, "name$module");
        assert_eq!(outer.properties[0].name, "property$module");
        assert!(parse("interface Contract { fun value() = 1; interface DefaultImpls }").is_ok());
    }

    #[test]
    fn property_and_accessor_visibility_remain_independent_from_custom_bodies() {
        let module = parse("class State { var first: Int = 0\n private set\ninternal var second: Int = 0\nvar third: Int = 0\n private set(value) { field = value } }").unwrap();
        let class = module.class_decl.unwrap();
        assert_eq!(class.properties[0].visibility(), PropertyVisibility::Public);
        assert_eq!(
            class.properties[0].setter_visibility(),
            PropertyVisibility::Private
        );
        assert!(class.properties[0].setter.is_none());
        assert_eq!(
            class.properties[1].getter_visibility(),
            PropertyVisibility::Internal
        );
        assert_eq!(
            class.properties[1].setter_visibility(),
            PropertyVisibility::Internal
        );
        assert!(class.properties[2].setter.is_some());
    }

    #[test]
    fn safe_call_assignment_has_a_distinct_source_statement() {
        let module = parse("fun change(value: State?) { value?.current = 1 }").unwrap();
        assert!(
            matches!(&module.functions[0].body[0].kind, StatementKind::SafePropertyAssign(Expr { kind: ExprKind::Name(value), .. }, name, _) if value == "value" && name == "current")
        );
        let error = parse("fun change(value: State?) { value?.current += 1 }")
            .err()
            .unwrap();
        assert_eq!(error.code, "SAFE_COMPOUND_ASSIGNMENT_UNSUPPORTED");
    }

    #[test]
    fn internal_property_visibility_is_preserved_and_unsupported_contexts_fail_explicitly() {
        for source in [
            "internal val value = 1",
            "object Holder { internal val value = 1 }",
            "class Holder { internal val value = 1 }",
            "class Holder { companion object { internal var value: Int = 1 } }",
            "interface Contract { companion object { internal val value = 1 } }",
        ] {
            assert!(parse(source).is_ok(), "{source}");
        }
        for source in [
            "interface Contract { companion object { internal val names = arrayOf(\"a\") } }",
            "class Holder(internal val value: Int)",
            "class Holder(@Mark internal var value: Int)",
        ] {
            let error = parse(source).err().unwrap();
            assert_eq!(
                error.code, "INTERNAL_PROPERTY_VISIBILITY_UNSUPPORTED",
                "{source}: {}",
                error.message
            );
            assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.offset, source.find("internal").unwrap(), "{source}");
        }
        let module = parse("internal fun top() = 1\ninternal class Holder { internal fun member() = 2 }\ninternal object Functions { internal fun member() = 3 }").unwrap();
        assert!(module.functions[0].internal);
        assert!(module.class_decl.unwrap().internal);
        let functions = module.object_decl.unwrap();
        assert!(functions.internal);
        assert!(functions.methods[0].function.internal);
    }

    #[test]
    fn named_types_record_their_actual_tokens_including_nested_and_qualified_types() {
        let source = "// Missing in a comment\nval label = \"Missing\"\nfun f(value: Missing): Missing = value";
        let module = parse(source).unwrap();
        let locations = module
            .type_locations
            .iter()
            .filter(|(name, _)| name == "Missing")
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            locations,
            [
                (
                    "Missing".into(),
                    source.find("value: Missing").unwrap() + "value: ".len()
                ),
                (
                    "Missing".into(),
                    source.find("): Missing").unwrap() + "): ".len()
                ),
            ]
        );

        let source =
            "fun f(value: demo.Box<external.Missing?>): (another.Input) -> finalpkg.Result = value";
        let module = parse(source).unwrap();
        for (name, spelling) in [
            ("demo/Box", "demo.Box"),
            ("external/Missing", "external.Missing"),
            ("another/Input", "another.Input"),
            ("finalpkg/Result", "finalpkg.Result"),
        ] {
            assert!(
                module
                    .type_locations
                    .contains(&(name.into(), source.find(spelling).unwrap()))
            );
        }

        let source = "fun f(value: Int): String = \"${make<demo.Missing>()}\"";
        let module = parse(source).unwrap();
        assert!(
            module
                .type_locations
                .contains(&("demo/Missing".into(), source.find("demo.Missing").unwrap()))
        );

        let module = parse("fun f(a: Int, b: Int): Boolean = a < b && b > a").unwrap();
        assert!(
            module
                .type_locations
                .iter()
                .all(|(name, _)| name != "a" && name != "b")
        );
    }

    #[test]
    fn unsupported_constructs_have_stable_codes_and_source_offsets() {
        for (source, marker, code) in [
            (
                "@JvmInline\nvalue class Gid(val raw: Int)",
                "value",
                "VALUE_CLASS_JVM_BOXING_UNSUPPORTED",
            ),
            (
                "object O { @kotlin.jvm.JvmInline value class Gid(val raw: Int) }",
                "value",
                "VALUE_CLASS_JVM_BOXING_UNSUPPORTED",
            ),
            (
                "enum class E { A { fun answer() = 1 } }",
                "{ fun",
                "ENUM_ENTRY_BODY_UNSUPPORTED",
            ),
            (
                "@Mark([1, 2]) class C",
                "[",
                "ANNOTATION_ARRAY_LITERAL_UNSUPPORTED",
            ),
            (
                "@Mark(values =\n [1, 2]) class C",
                "[",
                "ANNOTATION_ARRAY_LITERAL_UNSUPPORTED",
            ),
            (
                "fun f(input: @Mark String) {}",
                "@",
                "ANNOTATION_TYPE_USE_UNSUPPORTED",
            ),
            (
                "fun f(input: List<@Mark String>) {}",
                "@",
                "ANNOTATION_TYPE_USE_UNSUPPORTED",
            ),
            (
                "class C: @Mark Parent()",
                "@",
                "ANNOTATION_TYPE_USE_UNSUPPORTED",
            ),
            (
                "object O: @Mark Parent",
                "@",
                "ANNOTATION_TYPE_USE_UNSUPPORTED",
            ),
            (
                "interface I: @Mark Parent",
                "@",
                "ANNOTATION_TYPE_USE_UNSUPPORTED",
            ),
        ] {
            let error = parse(source).err().unwrap();
            assert_eq!(error.code, code, "{source}: {}", error.message);
            assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.offset, source.find(marker).unwrap(), "{source}");
        }
        let source = include_str!(
            "../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/datatransfer/model/Gid.kt"
        );
        let error = parse(source).err().unwrap();
        assert_eq!(error.code, "VALUE_CLASS_JVM_BOXING_UNSUPPORTED");
        assert_eq!(error.offset, source.find("value class").unwrap());
    }

    #[test]
    fn enum_model_reuses_constructor_members_and_companion_shapes() {
        let module = parse(include_str!(
            "../tests/fixtures/compatibility_enums_numeric/EnumModel.kt"
        ))
        .unwrap();
        let signal = module.enum_decl.unwrap();
        assert_eq!(signal.body.name, "Signal");
        assert_eq!(
            signal.body.params,
            [("code".into(), Type::Int), ("label".into(), Type::String)]
        );
        assert_eq!(signal.body.public_params, ["code"]);
        assert_eq!(signal.body.param_properties, ["code", "label"]);
        assert_eq!(signal.entry_arguments.len(), signal.entries.len());
        assert_eq!(
            signal
                .entry_arguments
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [2, 2]
        );
        assert_eq!(signal.body.properties[0].name, "doubled");
        assert_eq!(signal.body.initializers.len(), 1);
        assert_eq!(signal.body.methods[0].function.name, "caption");
        let companion = signal.body.companion.unwrap();
        assert_eq!(companion.properties[0].name, "initialized");
        assert_eq!(companion.methods[0].function.name, "fromCode");
        assert!(matches!(&module.declarations[0], Declaration::Enum(empty)
            if empty.entries.is_empty() && empty.entry_arguments.is_empty() && empty.body.methods[0].function.name == "count"));
    }

    #[test]
    fn unchanged_goto_tab_preserves_enum_arguments_and_function_alias() {
        let module = parse(include_str!("../benchmarks/sources/androidbible/Alkitab/src/main/java/yuku/alkitab/base/compose/goto/GotoTab.kt")).unwrap();
        let tab = module.enum_decl.unwrap();
        assert_eq!(tab.body.params, [("ordinal1Based".into(), Type::Int)]);
        assert_eq!(
            tab.entries
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["DIALER", "DIRECT", "GRID"]
        );
        assert_eq!(
            tab.entry_arguments
                .iter()
                .map(|arguments| match arguments[0].kind {
                    ExprKind::Int(value) => value,
                    _ => panic!(),
                })
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(module.type_aliases[0].name, "OnGotoFinished");
    }

    #[test]
    fn enum_entries_keep_empty_semicolon_forms_and_explicit_unsupported_boundaries() {
        for source in [
            "enum class E",
            "enum class E {}",
            "enum class E { ; }",
            "enum class E { A, }",
            "enum class E { A; }",
        ] {
            let declaration = parse(source).unwrap().enum_decl.unwrap();
            assert_eq!(declaration.entries.len(), declaration.entry_arguments.len());
            assert!(declaration.body.params.is_empty());
        }
        for (source, message) in [
            ("enum class E { A, A }", "Duplicate enum entry"),
            (
                "enum class E { A { fun answer() = 1 } }",
                "entry-specific class bodies",
            ),
            (
                "enum class E(val value: Int = 1) { A }",
                "default arguments",
            ),
            (
                "enum class E { A; constructor(value: Int) {} }",
                "Secondary constructors",
            ),
        ] {
            assert!(
                parse(source).unwrap_err().message.contains(message),
                "{source}"
            );
        }
        for source in [
            include_str!("../tests/fixtures/compatibility_enums_numeric/InvalidEnumForward.kt"),
            include_str!("../tests/fixtures/compatibility_enums_numeric/InvalidEnumCompanion.kt"),
        ] {
            parse(source).unwrap();
        }
    }

    #[test]
    fn repeated_enum_entry_terminators_are_rejected_at_the_second_semicolon() {
        for source in ["enum class E { A;; }", "enum class E { ;; }"] {
            let error = parse(source).err().unwrap();
            assert_eq!(error.code, "ENUM_ENTRY_TERMINATOR_REPEATED");
            assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
            assert_eq!(error.offset, source.match_indices(';').nth(1).unwrap().0);
        }
        for source in [
            "enum class E { A; ; }",
            "enum class E { A; /* comment */ ; }",
            "enum class E { A;\n; }",
            "enum class E { A; // comment\n; }",
            "enum class E { A; /* comment\n */ ; }",
            "enum class E { A; fun f() {}; val value = 1; }",
            "enum class E { A;\nfun f() {}; val value = 1; }",
            "class Ordinary { fun f() {}; val value = 1; }",
        ] {
            assert!(parse(source).is_ok(), "{source}");
        }
    }

    #[test]
    fn annotation_fixture_preserves_class_constructor_property_and_method_targets() {
        let source = include_str!(
            "../tests/fixtures/compatibility_annotations_visibility/positive/Annotated.kt"
        );
        let module = parse(source).unwrap();
        let class = module.class_decl.unwrap();
        assert_eq!(
            class
                .annotations
                .iter()
                .map(|annotation| annotation.name.as_str())
                .collect::<Vec<_>>(),
            ["RuntimeMark", "ClassMark", "SourceMark"]
        );
        assert_eq!(class.param_annotations.len(), class.params.len());
        assert_eq!(
            class.param_annotations[0]
                .iter()
                .map(|annotation| annotation.use_site.as_deref().unwrap())
                .collect::<Vec<_>>(),
            [
                "param", "field", "get", "set", "setparam", "field", "get", "field"
            ]
        );
        assert_eq!(class.methods[0].function.annotations[0].name, "RuntimeMark");
        assert_eq!(class.methods[0].function.param_annotations.len(), 1);
        assert!(class.methods[0].function.param_annotations[0].is_empty());
        for annotation in class
            .annotations
            .iter()
            .chain(class.param_annotations[0].iter())
        {
            assert_eq!(&source[annotation.offset..annotation.offset + 1], "@");
        }
    }

    #[test]
    fn declaration_annotations_survive_object_interface_enum_and_companion_wrappers() {
        let module = parse("@Mark object Holder { @field:Mark val value = 1; @Mark const val LIMIT = 2; @Mark @JvmStatic @Throws(Exception::class) @Suppress(\"unused\") fun use(@Mark input: Int) {} }\n@Mark interface Contract { @get:Mark val value: Int; companion object { @field:Mark val count: Int = 1; @field:Mark private val names: Array<String> = arrayOf(\"a\") } }\n@Mark enum class Choice { FIRST }\n@field:Mark val top = 1\n@Mark const val LIMIT = 3").unwrap();
        let object = module.object_decl.unwrap();
        assert_eq!(object.annotation_uses[0].name, "Mark");
        assert_eq!(object.annotations, ["Mark"]);
        assert_eq!(
            object.fields[0].annotations[0].use_site.as_deref(),
            Some("field")
        );
        assert_eq!(object.constants[0].annotations[0].name, "Mark");
        let method = &object.methods[0];
        assert!(method.jvm_static);
        assert_eq!(method.throws, ["Exception"]);
        assert_eq!(method.suppress, ["unused"]);
        assert_eq!(method.function.annotations[0].name, "Mark");
        assert_eq!(method.function.param_annotations[0][0].name, "Mark");
        let interface = module.interface_decl.unwrap();
        assert_eq!(interface.annotations[0].name, "Mark");
        assert_eq!(
            interface.properties[0].annotations[0].use_site.as_deref(),
            Some("get")
        );
        let companion = interface.companion.unwrap();
        assert_eq!(
            companion.fields[0].annotations[0].use_site.as_deref(),
            Some("field")
        );
        assert_eq!(
            companion.properties[0].annotations[0].use_site.as_deref(),
            Some("field")
        );
        assert_eq!(module.enum_decl.unwrap().annotations[0].name, "Mark");
        assert_eq!(
            module.properties[0].annotations[0].use_site.as_deref(),
            Some("field")
        );
        assert_eq!(module.constants[0].annotations[0].name, "Mark");
        let class = parse("class C { companion object { @field:Mark val count: Int = 1 } }")
            .unwrap()
            .class_decl
            .unwrap();
        assert_eq!(
            class.companion.unwrap().properties[0].annotations[0]
                .use_site
                .as_deref(),
            Some("field")
        );
    }

    #[test]
    fn parameter_annotation_vectors_preserve_empty_positions_and_unknown_target_names() {
        let module = parse("class C(first: Int, @field:Mark val second: String, third: Int)\nfun use(first: Int, @param:Mark second: Int, @unknown:Mark third: Int) {}").unwrap();
        let class = module.class_decl.unwrap();
        assert_eq!(
            class
                .param_annotations
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [0, 1, 0]
        );
        let function = &module.functions[0];
        assert_eq!(
            function
                .param_annotations
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            [0, 1, 1]
        );
        assert_eq!(
            function.param_annotations[2][0].use_site.as_deref(),
            Some("unknown")
        );
        assert!(parse("class C { @Mark constructor(value: Int) {} }").is_err());
    }

    #[test]
    fn functional_interfaces_aliases_and_annotated_backticks_parse_unchanged() {
        for source in [
            include_str!("../tests/fixtures/compatibility_declarations/FunctionalInterfaces.kt"),
            include_str!("../tests/fixtures/compatibility_declarations/Aliases.kt"),
            include_str!("../tests/fixtures/compatibility_declarations/AliasUses.kt"),
            include_str!("../tests/fixtures/compatibility_declarations/Backticks.kt"),
        ] {
            parse(source).unwrap();
        }
    }

    #[test]
    fn functional_interface_identity_is_separate_from_ordinary_interfaces() {
        let module = parse("fun interface Action { fun run() }\ninterface Ordinary { fun run() }\nclass Container { fun interface Nested { fun call(): Int } }").unwrap();
        assert!(module.interface_decl.unwrap().fun_interface);
        assert!(
            matches!(&module.declarations[0], Declaration::Interface(interface) if !interface.fun_interface)
        );
        let class = module.class_decl.unwrap();
        assert!(
            matches!(&class.nested[0], Declaration::Interface(interface) if interface.fun_interface)
        );
    }

    #[test]
    fn concrete_and_function_aliases_preserve_visibility_offsets_and_chains() {
        let source = "typealias Label = String\nprivate typealias Caption = Label\ntypealias Callback = (Int) -> Unit";
        let module = parse(source).unwrap();
        assert_eq!(module.type_aliases.len(), 3);
        assert_eq!(module.type_aliases[0].expanded, Type::String);
        assert!(!module.type_aliases[0].private);
        assert_eq!(module.type_aliases[1].expanded, Type::Java("Label".into()));
        assert!(module.type_aliases[1].private);
        assert_eq!(
            module.type_aliases[1].offset,
            source.find("private").unwrap()
        );
        assert!(
            matches!(&module.type_aliases[2].expanded, Type::Function { params, result } if params == &[Type::Int] && **result == Type::Unit)
        );
        let generic = parse("typealias Generic<T> = List<T>").unwrap();
        assert_eq!(generic.type_aliases[0].type_params[0].name, "T");
        assert!(
            matches!(&generic.type_aliases[0].expanded, Type::List(inner) if matches!(inner.as_ref(), Type::Parameter { name, .. } if name == "T"))
        );
    }

    #[test]
    fn function_annotations_keep_targets_arguments_and_method_locations() {
        let source = "@example.Marker @target:Other(1, label = \"value\") public fun `top method`() {}\nclass Example { public @example.Test fun `test method`() {} }";
        let module = parse(source).unwrap();
        let function = &module.functions[0];
        assert_eq!(function.annotations.len(), 2);
        assert_eq!(function.annotations[0].name, "example/Marker");
        assert_eq!(function.annotations[0].offset, 0);
        assert_eq!(function.annotations[1].use_site.as_deref(), Some("target"));
        assert_eq!(
            function.annotations[1].arguments[1].0.as_deref(),
            Some("label")
        );
        assert!(matches!(
            &function.annotations[1].arguments[0].1.kind,
            ExprKind::Int(1)
        ));
        assert_eq!(
            module.class_decl.unwrap().methods[0].function.annotations[0].name,
            "example/Test"
        );
        let declarations = parse("@Other class Example { @field:Other val value: Int = 1 }")
            .unwrap()
            .class_decl
            .unwrap();
        assert_eq!(declarations.annotations[0].name, "Other");
        assert_eq!(
            declarations.properties[0].annotations[0]
                .use_site
                .as_deref(),
            Some("field")
        );
    }

    #[test]
    fn typed_destructuring_entries_distinguish_discard_and_named_types() {
        let module = parse("fun use() { val (_: String, value: Int) = parts(); consume { (_: String, value: Int) -> value } }").unwrap();
        let StatementKind::Destructure(entries, _, _) = &module.functions[0].body[0].kind else {
            panic!()
        };
        assert_eq!(
            entries[0],
            DestructureEntry {
                name: None,
                ty: Some(Type::String)
            }
        );
        assert_eq!(
            entries[1],
            DestructureEntry {
                name: Some("value".into()),
                ty: Some(Type::Int)
            }
        );
    }

    #[test]
    fn discard_fixture_parses_without_rewriting() {
        parse(include_str!(
            "../tests/fixtures/compatibility_declarations/Discards.kt"
        ))
        .unwrap();
    }

    #[test]
    fn escaped_method_declarations_and_references_parse_in_classes() {
        let module = parse("class Example { fun `adds two numbers`(): Int = `when`(1); fun `when`(value: Int): Int = value + 1 }").unwrap();
        let class = module.class_decl.unwrap();
        assert_eq!(class.methods[0].function.name, "adds two numbers");
        assert_eq!(class.methods[1].function.name, "when");
    }

    #[test]
    fn escaped_names_in_braced_templates_do_not_close_interpolation() {
        let source = "fun use(`}`: Int): String = \"${`}`}\"";
        let module = parse(source).unwrap();
        let StatementKind::Return(Some(Expr {
            kind: ExprKind::Template(parts),
            ..
        })) = &module.functions[0].body[0].kind
        else {
            panic!()
        };
        assert!(
            parts
                .iter()
                .any(|part| matches!(&part.kind, ExprKind::Name(name) if name == "}"))
        );
        assert!(parse("fun bad(): String = \"$_\"").is_err());
    }

    #[test]
    fn escaped_unbraced_templates_use_identifier_rules_and_actual_token_offsets() {
        for name in ["n", "a value", "!x?", "}", "a\"b", "λ", "name$module", "_"] {
            let source = format!("fun f(`{name}`: Int): String = \"prefix $`{name}` suffix\"");
            let module = parse(&source).unwrap();
            let StatementKind::Return(Some(Expr {
                kind: ExprKind::Template(parts),
                ..
            })) = &module.functions[0].body[0].kind
            else {
                panic!()
            };
            let expression = parts
                .iter()
                .find(|part| matches!(&part.kind, ExprKind::Name(_)))
                .unwrap();
            assert!(matches!(&expression.kind, ExprKind::Name(actual) if actual == name));
            assert_eq!(expression.offset, source.rfind("$`").unwrap() + 1);
        }
        let source = "fun f(n: Int): String = \"$`n`\"";
        let module = parse(source).unwrap();
        assert!(matches!(
            &module.functions[0].body[0].kind,
            StatementKind::Return(Some(Expr {
                kind: ExprKind::Template(_),
                ..
            }))
        ));

        for (source, message) in [
            (
                "fun f(): String = \"$`unfinished\"",
                "Unterminated escaped identifier",
            ),
            (
                "fun f(): String = \"$``\"",
                "Escaped identifiers cannot be empty",
            ),
            (
                "fun f(): String = \"$`bad\nname`\"",
                "Escaped identifiers cannot contain newlines",
            ),
            (
                "fun f(): String = \"$`bad/name`\"",
                "Identifier contains a character forbidden in JVM names",
            ),
        ] {
            let error = parse(source).err().unwrap();
            assert!(
                error.message.contains(message),
                "{source}: {}",
                error.message
            );
            assert_eq!(error.offset, source.find("$`").unwrap() + 1, "{source}");
        }
        assert!(
            parse("fun f(n: Int): String = \"$n二\"")
                .err()
                .unwrap()
                .message
                .contains("Non-ASCII characters")
        );
    }

    #[test]
    fn malformed_escaped_templates_keep_source_offsets_through_public_compilation() {
        for template in ["$`unfinished", "$``", "$`bad/name`"] {
            let source = format!("\n\nfun f(n: Int): String = \"{template}\"");
            let error = crate::compile(&source, "EscapedTemplate.kt").err().unwrap();
            assert_eq!(
                error.offset,
                source.find("$`").unwrap() + 1,
                "{}",
                error.message
            );
            assert!(
                error
                    .render("EscapedTemplate.kt", &source)
                    .starts_with("EscapedTemplate.kt:3:27: error:")
            );
        }
    }

    #[test]
    fn vararg_parameters_record_physical_arrays_and_reject_unsupported_forms() {
        for (element, array) in [
            ("Int", Type::IntArray),
            ("Byte", Type::ByteArray),
            ("Char", Type::CharArray),
            ("Long", Type::LongArray),
            ("Boolean", Type::BooleanArray),
            ("Float", Type::FloatArray),
            ("Double", Type::DoubleArray),
            ("String", Type::Array(Box::new(Type::String))),
            (
                "String?",
                Type::Array(Box::new(Type::Nullable(Box::new(Type::String)))),
            ),
        ] {
            let module = parse(&format!(
                "fun collect(first: Int, vararg values: {element}) {{}}"
            ))
            .unwrap();
            let function = &module.functions[0];
            assert!(function.vararg);
            assert_eq!(function.params[1].1, array);
        }
        assert!(!parse("fun plain(value: Int) {}").unwrap().functions[0].vararg);
        for source in [
            "fun bad(vararg values: Short) {}",
            "fun bad(vararg first: Int, vararg second: Int) {}",
            "fun bad(vararg first: Int, second: Int) {}",
            "fun bad(vararg first: Int = 1) {}",
            "infix fun String.bad(vararg first: Int) {}",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }

    #[test]
    fn spread_arguments_keep_locations_and_named_argument_structure() {
        let source = "fun use(values: IntArray) { collect(1, *values, rest = *values) }";
        let module = parse(source).unwrap();
        let StatementKind::Expression(Expr {
            kind: ExprKind::Call(_, arguments),
            ..
        }) = &module.functions[0].body[0].kind
        else {
            panic!()
        };
        assert_eq!(arguments[1].offset, source.find("*values").unwrap());
        assert!(
            matches!(&arguments[1].kind, ExprKind::Unary(operator, value)
            if operator == "*" && matches!(&value.kind, ExprKind::Name(name) if name == "values"))
        );
        assert!(
            matches!(&arguments[2].kind, ExprKind::NamedArgument(name, value)
            if name == "rest" && matches!(&value.kind, ExprKind::Unary(operator, _) if operator == "*"))
        );
        assert!(parse("fun bad(values: IntArray) = *values").is_err());
    }

    #[test]
    fn escaped_identifiers_preserve_names_and_original_offsets() {
        let source = "fun `when`(`a value`: Int): Int = `a value`\nfun `adds λ`(): Int = `when`(2)\nfun plain(): Int = `plain`(3)";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens[1].text, "`when`");
        assert_eq!(tokens[1].offset, source.find("`when`").unwrap());
        let module = parse(source).unwrap();
        assert_eq!(module.functions[0].name, "when");
        assert_eq!(module.functions[0].params[0].0, "a value");
        assert_eq!(module.functions[1].name, "adds λ");
        let StatementKind::Return(Some(value)) = &module.functions[0].body[0].kind else {
            panic!()
        };
        assert!(matches!(&value.kind, ExprKind::Name(name) if name == "a value"));
        assert_eq!(value.offset, source.rfind("`a value`").unwrap());
        let module = parse("fun use(): Int = 1 `add two` 2").unwrap();
        assert!(matches!(&module.functions[0].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Binary(name, _, _), .. })) if name == "add two"));
    }

    #[test]
    fn escaped_identifiers_reject_invalid_jvm_names_with_source_locations() {
        for name in [
            "bad/name",
            "bad.name",
            "bad;name",
            "bad[name",
            "bad]name",
            "bad<name",
            "bad>name",
            "bad:name",
            "bad\\name",
        ] {
            let source = format!("fun `{name}`() {{}}");
            let error = parse(&source).unwrap_err();
            assert_eq!(error.offset, 4, "{name}");
            assert!(error.message.contains("JVM"), "{name}: {}", error.message);
        }
        for source in [
            "fun ``() {}",
            "fun `unterminated() {}",
            "fun `new\nline`() {}",
        ] {
            let error = parse(source).unwrap_err();
            assert_eq!(error.offset, 4);
        }
    }

    #[test]
    fn escaped_names_preserve_existing_unsupported_boundaries() {
        for source in [
            "fun use(): Int = 1 `is` 2",
            "fun use(): Int = 1 `+` 2",
            "class `Int`",
            "fun use(`this`: Int): Int = `this`",
            "fun use(`super@outer`: Int): Int = `super@outer`",
        ] {
            let error = parse(source).unwrap_err();
            assert!(
                error.message.contains("unsupported") || error.message.contains("not supported"),
                "{source}: {}",
                error.message
            );
        }
    }

    #[test]
    fn raw_discards_and_escaped_underscore_bindings_remain_distinct() {
        let module =
            parse("fun use() {\nval (_, `_`, value) = parts()\nconsume { _, `_`, _ -> `_` }\n}")
                .unwrap();
        assert!(matches!(&module.functions[0].body[0].kind,
            StatementKind::Destructure(names, _, _) if names.iter().map(|entry| entry.name.clone()).collect::<Vec<_>>() == vec![None, Some("_".into()), Some("value".into())]));
        let StatementKind::Expression(Expr {
            kind: ExprKind::Call(_, arguments),
            ..
        }) = &module.functions[0].body[1].kind
        else {
            panic!()
        };
        let ExprKind::LambdaWithParams {
            params: Some(params),
            ..
        } = &arguments[0].kind
        else {
            panic!()
        };
        assert_eq!(
            params
                .iter()
                .map(|parameter| parameter.discard)
                .collect::<Vec<_>>(),
            [true, false, true]
        );
        for source in [
            "val _ = 1",
            "fun bad() = _",
            "fun bad(_) {}",
            "fun bad() { val __ = 1 }",
            "fun bad() { consume { _, _ -> _ } }",
        ] {
            assert!(parse(source).is_err(), "{source}");
        }
    }

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
             internal const val count = -9223372036854775807L\n\
             const val scale = -1.25e+2F\n\
             fun fraction(): Float = 1e-2f\n\
             fun range(x: Int): Boolean = x in 1..3",
        )
        .unwrap();
        assert!(module.constants[0].private);
        assert!(module.constants[1].internal);
        assert!(matches!(
            module.constants[1].value,
            ConstValue::Long(value) if value == -i64::MAX
        ));
        assert!(
            matches!(module.constants[2].value, ConstValue::Float(bits) if bits == (-125.0f32).to_bits())
        );
        assert!(matches!(
            &module.functions[0].body[0].kind,
            StatementKind::Return(Some(Expr { kind: ExprKind::Float(bits), .. })) if *bits == 0.01f32.to_bits()
        ));
        assert!(parse("fun f(): Float = 1.5").is_ok());
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
            matches!(&body[1].kind, StatementKind::Destructure(names, false, _) if names.iter().map(|entry| entry.name.clone()).collect::<Vec<_>>() == vec![Some("x".into()), None])
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
    #[test]
    fn subject_when_conditions_require_parenthesized_boolean_operations() {
        for condition in [
            "a == b",
            "a != b",
            "a === b",
            "a !== b",
            "a < b",
            "a <= b",
            "a > b",
            "a >= b",
            "a && b",
            "a || b",
            "a in b",
            "a !in b",
            "a is String",
            "a !is String",
            "(a) == b",
        ] {
            for body in [
                format!("fun f() = when (true) {{ {condition} -> 1; else -> 0 }}"),
                format!("fun f() {{ when (true) {{ {condition} -> 1; else -> 0 }} }}"),
            ] {
                let error = match parse(&body) {
                    Ok(_) => panic!("invalid subject condition"),
                    Err(error) => error,
                };
                assert_eq!(error.kind, crate::DiagnosticKind::InvalidKotlin, "{body}");
                assert_eq!(error.code, "CONFUSING_BRANCH_CONDITION_ERROR");
            }
            parse(&format!(
                "fun f() = when (true) {{ ({condition}) -> 1; else -> 0 }}"
            ))
            .unwrap();
            parse(&format!("fun f() = when {{ {condition} -> 1; else -> 0 }}")).unwrap();
        }
        parse("fun f(a: Any) = when(a) { is String -> 1; !is Int -> 2; else -> 0 }").unwrap();
        parse("fun f(a: Int) = when(a) { in 1..3 -> 1; else -> 0 }").unwrap();
    }
}
