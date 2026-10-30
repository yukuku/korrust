#[path = "check_annotation_placements.rs"]
mod annotation_placements;
#[path = "check_class_delegation.rs"]
mod class_delegation;
#[path = "check_sealed.rs"]
mod sealed;
use crate::classpath::{Classpath, JavaClass, JavaMethod};
use crate::{
    Diagnostic, Result,
    syntax::{self, Expr, ExprKind, StatementKind, Type},
};
pub(crate) use annotation_placements::annotation_placements;
use std::collections::{HashMap, HashSet};
use std::{cell::RefCell, rc::Rc};

#[path = "check_declarations.rs"]
mod declarations;
#[path = "check_delegates.rs"]
mod delegates;
#[path = "check_enums.rs"]
pub(crate) mod enums;
use declarations::check_class;
pub use declarations::{
    infer_source_results, source_inference_error, unresolved_source_dependencies,
};
#[path = "check_captures.rs"]
mod captures;
#[path = "check_classes.rs"]
mod classes;
#[path = "check_control.rs"]
mod control;
#[path = "check_functions.rs"]
mod function_values;
#[path = "check_ir_walk.rs"]
pub(crate) mod ir_walk;
#[path = "check_java_properties.rs"]
mod java_properties;
#[path = "check_library.rs"]
mod library;
#[path = "check_operations.rs"]
mod operations;
#[path = "check_overloads.rs"]
mod overloads;
#[path = "check_references.rs"]
mod references;
#[path = "check_relations.rs"]
mod relations;

#[path = "check_varargs.rs"]
mod varargs;

#[path = "check_annotations.rs"]
mod annotations;
pub(crate) use annotations::unsupported_lowering as unsupported_annotation_lowering;

#[path = "check_interfaces.rs"]
mod interfaces;
#[path = "check_objects.rs"]
mod objects;
#[path = "check_singletons.rs"]
mod singletons;

pub(crate) use annotations::unsupported_lowering as annotation_unsupported_lowering;

#[derive(Clone, Debug)]
pub enum DeclarationAnnotationValue {
    Long(i64),
    Float(u32),
    Double(u64),
    Char(u16),
    Enum { descriptor: String, entry: String },
    Class { descriptor: String, logical: Type },
    Array(Vec<DeclarationAnnotationValue>),
    Nested(Box<DeclarationAnnotation>),
    Int(i32),
    Boolean(bool),
    String(Vec<u16>),
}

#[derive(Clone, Debug)]
pub struct DeclarationAnnotation {
    pub descriptor: String,
    pub runtime: bool,
    pub values: Vec<(String, DeclarationAnnotationValue)>,
}

pub struct LambdaDecl {
    pub defaults: Vec<JavaMethod>,
    pub owner: String,
    pub captures: Vec<(String, Type)>,
    pub function: Function,
    pub interface: Option<JavaMethod>,
}
pub struct Module {
    pub exposed_fields: HashMap<String, u16>,
    pub properties: Vec<ClassCompanionProperty>,
    pub initializers: Vec<Statement>,
    pub locals: u16,
    pub lambdas: Vec<LambdaDecl>,
    pub constants: Vec<syntax::ConstDecl>,
    pub functions: Vec<Function>,
    pub class_decl: Option<ClassDecl>,
    pub interface_decl: Option<InterfaceDecl>,
    pub object_decl: Option<DynamicObject>,
}
pub struct DynamicObject {
    pub bridges: Vec<JavaMethod>,
    pub exposed_fields: HashMap<String, u16>,
    pub constants: Vec<syntax::ConstDecl>,
    pub initializers: Vec<Statement>,
    pub locals: u16,
    pub owner: String,
    pub interfaces: Vec<String>,
    pub serializable: bool,
    pub fields: Vec<ObjectField>,
    pub methods: Vec<ObjectMethod>,
}
pub struct ObjectField {
    pub mutable: bool,
    pub name: String,
    pub ty: Type,
    pub initializer: ObjectInitializer,
}
pub enum ObjectInitializer {
    Value {
        value: Value,
        locals: u16,
    },
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
    pub sealed_: bool,
    pub sealed_subclasses: Vec<String>,
    pub type_params: Vec<syntax::TypeParameter>,
    pub super_types: Vec<Type>,
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
    pub state: Option<ClassCompanion>,
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
        class_decl: Box<ClassDecl>,
    },
}
pub struct ClassField {
    pub name: String,
    pub ty: Type,
    pub private: bool,
    pub mutable: bool,
    pub param: Option<usize>,
    pub initializer: Option<Value>,
}
pub struct ClassCompanionProperty {
    pub name: String,
    pub ty: Type,
    pub private: bool,
    pub setter_visibility: syntax::PropertyVisibility,
    pub mutable: bool,
    pub value: Value,
}
pub struct ClassCompanion {
    pub exposed_fields: HashMap<String, u16>,
    pub methods: Vec<Function>,
    pub initializers: Vec<Statement>,
    pub owner: String,
    pub properties: Vec<ClassCompanionProperty>,
    pub locals: u16,
}
pub struct SecondaryConstructor {
    pub initializers: Vec<Statement>,
    pub initializer_local_base: u16,
    pub function: Function,
    pub delegation_owner: String,
    pub delegation_descriptor: String,
    pub delegation_args: Vec<Value>,
    pub delegates_to_this: bool,
}
pub struct GenericBridge {
    pub erased: JavaMethod,
    pub target_params: Vec<Type>,
    pub target_result: Type,
}
pub struct InterfaceDelegateStorage {
    pub slot: u8,
    pub name: String,
    pub ty: Type,
    pub initializer: Value,
}
pub struct InterfaceDelegatedMember {
    pub declaration: JavaMethod,
    pub jvm_descriptor: String,
}
pub struct ClassDecl {
    pub interface_delegate_storage: Vec<InterfaceDelegateStorage>,
    pub interface_delegated_members: Vec<InterfaceDelegatedMember>,
    pub sealed_: bool,
    pub sealed_subclasses: Vec<String>,
    pub type_params: Vec<syntax::TypeParameter>,
    pub super_types: Vec<Type>,
    pub generic_bridges: Vec<GenericBridge>,
    pub exposed_fields: HashMap<String, u16>,
    pub delegated_properties: Vec<(String, Value)>,
    pub primary_constructor: bool,
    pub abstract_: bool,
    pub open_methods: HashSet<String>,
    pub protected_methods: HashSet<String>,
    pub abstract_methods: HashSet<String>,
    pub constructor_body: Vec<Statement>,
    pub super_class: String,
    pub super_args: Vec<Value>,
    pub open: bool,
    pub secondary_constructors: Vec<SecondaryConstructor>,
    pub fields: Vec<ClassField>,
    pub defaults: Vec<Option<Value>>,
    pub constructor_locals: u16,
    pub companion: Option<ClassCompanion>,
    pub params: Vec<(String, Type)>,
    pub data: bool,
    pub interfaces: Vec<String>,
    pub methods: Vec<Function>,
    pub bridges: Vec<JavaMethod>,
}
pub struct Function {
    pub class_type_params: Vec<String>,
    pub jvm_result: Option<Type>,
    pub jvm_name: Option<String>,
    pub annotations: Vec<DeclarationAnnotation>,
    pub vararg: bool,
    pub type_params: Vec<syntax::TypeParameter>,
    pub defaults: Vec<Option<Value>>,
    pub name: String,
    pub private: bool,
    pub is_override: bool,
    pub params: Vec<(String, Type)>,
    pub result: Type,
    pub locals: u16,
    pub body: Vec<Statement>,
}
impl Function {
    pub fn jvm_name(&self) -> &str {
        self.jvm_name.as_deref().unwrap_or(&self.name)
    }
    pub fn physical_result(&self) -> &Type {
        self.jvm_result.as_ref().unwrap_or(&self.result)
    }
    pub fn descriptor(&self) -> String {
        format!(
            "({}){}",
            self.params
                .iter()
                .map(|p| p.1.descriptor())
                .collect::<String>(),
            self.physical_result().descriptor()
        )
    }
}
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Statement {
    ThrowValue(Value),
    Labeled {
        label: String,
        body: Vec<Statement>,
    },
    BreakLabel(String),
    ContinueLabel(String),
    JavaFieldStore {
        owner: String,
        name: String,
        ty: Type,
        receiver: Option<Value>,
        value: Value,
    },
    FieldStore {
        name: String,
        receiver: Value,
        value: Value,
    },
    Store(u8, Value),
    Expression(Value),
    Return(Value),
    NonLocalReturn(Value),
    If(Value, Vec<Statement>, Vec<Statement>),
    While(Value, Vec<Statement>),
    DoWhile(Value, Vec<Statement>),
    ForIterator {
        element: Type,
        iterable: Value,
        iterator_slot: u8,
        slot: u8,
        body: Vec<Statement>,
    },
    ForEach {
        slot: u8,
        iterable: Value,
        iterable_slot: u8,
        index_slot: u8,
        body: Vec<Statement>,
    },
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
#[derive(Clone)]
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
#[derive(Clone)]
pub struct Catch {
    pub owner: String,
    pub slot: u8,
    pub body: Vec<Statement>,
    pub result: Value,
}
#[derive(Clone)]
pub enum ValueKind {
    ClassLiteral(Type),
    SuperCall {
        method: JavaMethod,
        args: Vec<Value>,
    },
    ArrayInitialize {
        array: Type,
        size: Box<Value>,
        initializer: Box<Value>,
        array_slot: u8,
        index_slot: u8,
        initializer_slot: u8,
    },
    Block(Vec<Statement>, Box<Value>),
    Try {
        body: Vec<Statement>,
        result: Box<Value>,
        catches: Vec<Catch>,
        finally: Vec<Statement>,
        exception_slot: u8,
        result_slot: Option<u8>,
    },
    CompiledInline {
        method: JavaMethod,
        args: Vec<Value>,
        local_base: u16,
        lambda_bases: Vec<Option<(String, u16)>>,
        reified_temp: Option<u16>,
    },
    Lambda {
        owner: String,
        captures: Vec<Value>,
    },
    FunctionInvoke {
        function: Box<Value>,
        args: Vec<Value>,
    },
    CellIncrement {
        cell: Box<Value>,
        prefix: bool,
    },
    Decrement {
        slot: u8,
        prefix: bool,
    },
    CellDecrement {
        cell: Box<Value>,
        prefix: bool,
    },
    TypeTest {
        value: Box<Value>,
        target: Type,
        negated: bool,
    },
    Cast {
        value: Box<Value>,
        target: Type,
        safe: bool,
    },
    Float(u32),
    Double(u64),
    Box(Box<Value>),
    Unbox(Box<Value>),
    New {
        owner: String,
        descriptor: String,
        args: Vec<Value>,
    },
    JavaField {
        owner: String,
        name: String,
        receiver: Option<Box<Value>>,
    },
    ListFactory {
        mutable: bool,
        elements: Vec<Value>,
    },
    Abrupt(Vec<Statement>),
    Unit,
    Long(i64),
    Null,
    IsNull(Box<Value>),
    ListSize(Box<Value>),
    ListGet(Box<Value>, Box<Value>),
    ListSet(Box<Value>, Box<Value>, Box<Value>),
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
    DefaultCall(usize, Vec<Value>, u32),
    Compare(Box<Value>, Box<Value>),
}
impl Value {
    pub fn stack(&self) -> usize {
        match &self.kind {
            ValueKind::ClassLiteral(_) => 1,
            ValueKind::ArrayInitialize { .. }
            | ValueKind::Block(..)
            | ValueKind::Try { .. }
            | ValueKind::SuperCall { .. } => 256,
            ValueKind::CellIncrement { cell, .. } | ValueKind::CellDecrement { cell, .. } => {
                cell.stack().max(5)
            }
            ValueKind::Decrement { .. } => 2,
            ValueKind::TypeTest { value, .. } | ValueKind::Cast { value, .. } => {
                value.stack().max(4)
            }
            ValueKind::CompiledInline { args, .. } => {
                args.iter().map(Value::stack).max().unwrap_or(1).max(256)
            }
            ValueKind::Lambda { captures, .. } => captures
                .iter()
                .scan(2, |n, v| {
                    let m = *n + v.stack();
                    *n += v.ty.slots();
                    Some(m)
                })
                .max()
                .unwrap_or(2),
            ValueKind::FunctionInvoke { function, args } => function.stack().max(
                args.iter()
                    .enumerate()
                    .map(|(i, v)| i + 1 + v.stack().max(2))
                    .max()
                    .unwrap_or(1),
            ),
            ValueKind::Float(_) => 1,
            ValueKind::Double(_) => 2,
            ValueKind::Box(value) | ValueKind::Unbox(value) => value.stack().max(2),
            ValueKind::JavaField { receiver, .. } => receiver
                .as_ref()
                .map_or(1, |v| v.stack())
                .max(self.ty.slots()),
            ValueKind::Abrupt(_) => 256,
            ValueKind::ListFactory { elements, .. } => {
                elements.iter().map(|v| 4 + v.stack()).max().unwrap_or(2)
            }
            ValueKind::New { args, .. } => args
                .iter()
                .scan(2, |slots, v| {
                    let n = *slots + v.stack();
                    *slots += v.ty.slots();
                    Some(n)
                })
                .max()
                .unwrap_or(2),
            ValueKind::Unit => 0,
            ValueKind::Long(_) => 2,
            ValueKind::Null => 1,
            ValueKind::IsNull(value) | ValueKind::ListSize(value) => value.stack().max(1),
            ValueKind::ListGet(a, b) => a.stack().max(1 + b.stack()),
            ValueKind::ListSet(a, b, c) => a.stack().max(1 + b.stack()).max(2 + c.stack()),
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
            ValueKind::Field(_, receiver) => receiver.stack().max(self.ty.slots()),
            ValueKind::NotNull(v, _) => v.stack().max(3),
            ValueKind::Int(_)
            | ValueKind::Boolean(_)
            | ValueKind::String(_)
            | ValueKind::Local(_) => self.ty.slots(),
            ValueKind::Negate(v) | ValueKind::Not(v) | ValueKind::Length(v) => v.stack().max(2),
            ValueKind::Binary(_, a, b) | ValueKind::Index(a, b) | ValueKind::Compare(a, b) => {
                a.stack().max(a.ty.slots() + b.stack())
            }
            ValueKind::StringConcat(a, b) => 2.max(1 + a.stack()).max(1 + b.stack()),
            ValueKind::DefaultCall(_, args, _) => args
                .iter()
                .scan(0, |slots, v| {
                    let n = *slots + v.stack();
                    *slots += v.ty.slots();
                    Some(n)
                })
                .max()
                .unwrap_or(0)
                .max(args.iter().map(|v| v.ty.slots()).sum::<usize>() + 2),
            ValueKind::Call(_, args)
            | ValueKind::StaticCall { args, .. }
            | ValueKind::StringCall(_, args)
            | ValueKind::CopyInto(args)
            | ValueKind::JavaCall { args, .. }
            | ValueKind::MemberCall { args, .. } => args
                .iter()
                .scan(0, |slots, v| {
                    let stack = *slots + v.stack();
                    *slots += v.ty.slots();
                    Some(stack)
                })
                .max()
                .unwrap_or(1)
                .max(self.ty.slots()),
        }
    }
}
#[derive(Clone)]
struct Local {
    slot: u8,
    ty: Type,
    mutable: bool,
}
type ConstructorSignature<'a> = (&'a str, &'a Vec<(String, Type)>, &'a Vec<Option<Expr>>);
type CheckedArguments = (Vec<Value>, Vec<(u8, Value)>, u64);
type CallableParameters = (Vec<(String, Type)>, Vec<bool>);
struct Checker<'a> {
    inline_argument_modes: Vec<syntax::InlineParameterMode>,
    inline_analysis: bool,
    public_inline_body: bool,
    reified_parameters: Vec<syntax::TypeParameter>,
    exhaustiveness_flow: bool,
    lambdas: Rc<RefCell<Vec<LambdaDecl>>>,
    captured: HashMap<String, (String, Type)>,
    captured_cells: HashMap<String, Type>,
    cells: HashMap<u8, String>,
    cell_names: HashSet<String>,
    lambda_owner: String,
    inline_result: Option<Type>,
    nonlocal_result: Option<Type>,
    classpath: &'a Classpath,
    constants: &'a HashMap<String, (Type, Expr)>,
    constructor: Option<ConstructorSignature<'a>>,
    defaults: &'a Vec<Vec<Option<Expr>>>,
    function_names: &'a [String],
    mutable_fields: &'a HashSet<String>,
    extensions: &'a HashSet<usize>,
    classes: &'a HashMap<String, JavaClass>,
    imports: &'a HashMap<String, String>,
    signatures: &'a Vec<(Vec<(String, Type)>, Type)>,
    source_type_params: &'a [Vec<syntax::TypeParameter>],
    source_infix: &'a HashSet<usize>,
    source_operator: &'a HashSet<usize>,
    receiver: Option<Local>,
    fields: &'a HashMap<String, Type>,
    members: &'a HashMap<String, JavaMethod>,
    companion_fields: &'a HashMap<String, (String, Type)>,
    functions: &'a HashMap<String, usize>,
    object_methods: &'a HashMap<String, Vec<JavaMethod>>,
    object_fields: &'a HashMap<String, (String, Type)>,
    object_owner: Option<&'a str>,
    allow_return: bool,
    refinements: HashMap<u8, Type>,
    initialized: HashSet<u8>,
    loop_depth: usize,
    scopes: Vec<HashMap<String, Local>>,
    next: u16,
    result: Type,
}
impl Checker<'_> {
    fn list_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        if unresolved_list(&receiver.ty) {
            return Err(Diagnostic::new(
                offset,
                "Cannot infer list element type; empty or null-only factories require an explicit or expected element type",
            ));
        }
        if matches!(name, "get" | "isEmpty" | "set" | "add")
            && args.len()
                != match name {
                    "get" | "add" => 1,
                    "set" => 2,
                    _ => 0,
                }
        {
            return Err(
                Diagnostic::new(offset, "List member argument count mismatch").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "LIST_MEMBER_ARGUMENT_COUNT",
                ),
            );
        }
        let (element, mutable) = match receiver.ty.base() {
            Type::List(e) => (e.as_ref().clone(), false),
            Type::MutableList(e) => (e.as_ref().clone(), true),
            _ => unreachable!(),
        };
        if name == "get" && args.len() == 1 {
            let index = self.value(&args[0])?;
            self.require(&index, Type::Int, offset)?;
            return Ok(Value {
                ty: element,
                kind: ValueKind::ListGet(Box::new(receiver), Box::new(index)),
            });
        }
        if name == "isEmpty" && args.is_empty() {
            return Ok(Value {
                ty: Type::Boolean,
                kind: ValueKind::Binary(
                    "==".into(),
                    Box::new(Value {
                        ty: Type::Int,
                        kind: ValueKind::ListSize(Box::new(receiver)),
                    }),
                    Box::new(Value {
                        ty: Type::Int,
                        kind: ValueKind::Int(0),
                    }),
                ),
            });
        }
        if !mutable {
            return self.extension_call(receiver, name, args, &[], offset);
        }
        if name == "set" && args.len() == 2 {
            let index = self.value(&args[0])?;
            self.require(&index, Type::Int, offset)?;
            let value = self.value(&args[1])?;
            let value = self.coerce(value, element.clone(), offset)?;
            return Ok(Value {
                ty: element,
                kind: ValueKind::ListSet(Box::new(receiver), Box::new(index), Box::new(value)),
            });
        }
        let (descriptor, result) = match (name, args.len()) {
            ("add", 1) => ("(Ljava/lang/Object;)Z", Type::Boolean),
            ("set", 2) => (
                "(ILjava/lang/Object;)Ljava/lang/Object;",
                Type::Java("java/lang/Object".into()),
            ),
            _ => {
                let owner = library::type_owner(&receiver.ty).unwrap();
                let mapped =
                    library::ordinary_member_methods(self.classpath, owner, name, args.len())?
                        .iter()
                        .any(|method| {
                            method.source_name() == name && method.source_name() != method.name
                        });
                return if mapped {
                    self.java_call(receiver, name, args, offset)
                } else {
                    self.extension_call(receiver, name, args, &[], offset)
                };
            }
        };
        let mut values = vec![receiver];
        if name == "set" {
            let index = self.value(&args[0])?;
            self.require(&index, Type::Int, offset)?;
            values.push(index);
        }
        let arg = args.last().unwrap();
        let value = self.value(arg)?;
        let value = self.coerce(value, element.clone(), arg.offset)?;
        values.push(if value.ty.is_reference() {
            value
        } else {
            Value {
                ty: Type::Nullable(Box::new(value.ty.clone())),
                kind: ValueKind::Box(Box::new(value)),
            }
        });
        let method = JavaMethod {
            library: None,
            owner: "java/util/List".into(),
            name: name.into(),
            descriptor: descriptor.into(),
            access: 0x0401,
            unsupported: false,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        };
        let call = Value {
            ty: result,
            kind: ValueKind::MemberCall {
                method,
                interface: true,
                args: values,
            },
        };
        if name == "set" {
            return Err(Diagnostic::new(
                offset,
                "Use indexed assignment for mutable list set",
            ));
        }
        Ok(call)
    }
    fn checked_defaults(
        &mut self,
        defaults: &[Option<Expr>],
        params: &[(String, Type)],
        offset: usize,
    ) -> Result<Vec<Option<Value>>> {
        if defaults.iter().any(Option::is_some) && params.len() > 32 {
            return Err(Diagnostic::new(
                offset,
                "Default arguments support at most 32 parameters",
            ));
        }
        let allow_return = self.allow_return;
        self.allow_return = false;
        let all = self.initialized.clone();
        let refinements = self.refinements.clone();
        let mut earlier = HashSet::new();
        let mut values = Vec::new();
        for ((name, ty), expr) in params.iter().zip(defaults) {
            self.refinements = refinements.clone();
            self.initialized = earlier.clone();
            let value = expr
                .as_ref()
                .map(|expr| {
                    let value = self.value(expr)?;
                    self.coerce(value, ty.clone(), expr.offset)
                })
                .transpose()?;
            values.push(value);
            if let Some(local) = self.local(name) {
                earlier.insert(local.slot);
            }
        }
        self.initialized = all;
        self.refinements = refinements;
        self.allow_return = allow_return;
        Ok(values)
    }
    fn refine(&mut self, condition: &Expr, truth: bool) {
        if let ExprKind::TypeOperation(op, value, target) = &condition.kind {
            if (op == "is" && truth) || (op == "!is" && !truth) {
                if let ExprKind::Name(name) = &value.kind {
                    if let Some(local) = self.local(name) {
                        if !local.mutable {
                            if let Ok(target) = self.resolve(target, condition.offset) {
                                self.refinements.insert(local.slot, target);
                            }
                        }
                    }
                }
            }
        }
        if let ExprKind::Unary(op, inner) = &condition.kind {
            if op == "!" {
                self.refine(inner, !truth);
            }
        }
        if let ExprKind::Binary(op, left, right) = &condition.kind {
            if op == "&&" && truth {
                self.refine(left, true);
                self.refine(right, true);
            }
            if op == "||" && !truth {
                self.refine(left, false);
                self.refine(right, false);
            }
            let name = match (&left.kind, &right.kind) {
                (ExprKind::Name(name), ExprKind::Null) | (ExprKind::Null, ExprKind::Name(name)) => {
                    Some(name)
                }
                _ => None,
            };
            if let Some(name) = name {
                if (matches!(op.as_str(), "!=" | "!==") && truth)
                    || (matches!(op.as_str(), "==" | "===") && !truth)
                {
                    if let Some(local) = self.local(name) {
                        if !local.mutable {
                            match local.ty {
                                Type::Nullable(inner) => {
                                    self.refinements.insert(local.slot, *inner);
                                }
                                ty @ Type::Parameter { .. } => {
                                    self.refinements.insert(local.slot, ty);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }
    fn applicability_result(
        &mut self,
        params: &[(String, Type)],
        defaults: &[bool],
        args: &[Expr],
        named: bool,
        offset: usize,
    ) -> Result<usize> {
        let mapping = Self::argument_mapping(params, defaults, args, named, offset)?;
        let before = self.next;
        let refinements = self.refinements.clone();
        let initialized = self.initialized.clone();
        let lambda_count = self.lambdas.borrow().len();
        let mut score = 0;
        let result = (|| {
            for (arg, i) in args.iter().zip(mapping) {
                let arg = if let ExprKind::NamedArgument(_, value) = &arg.kind {
                    value.as_ref()
                } else {
                    arg
                };
                let value = self.inline_argument_value(arg, &params[i].1, i)?;
                if value.ty != params[i].1 {
                    score += if value.ty == Type::Null { 4 } else { 1 };
                }
                self.coerce(value, params[i].1.clone(), arg.offset)?;
            }
            Ok(score * 64 + params.len().saturating_sub(args.len()))
        })();
        self.lambdas.borrow_mut().truncate(lambda_count);
        self.next = before;
        self.refinements = refinements;
        self.initialized = initialized;
        result
    }
    fn select_function(
        &mut self,
        candidates: &[usize],
        args: &[Expr],
        offset: usize,
    ) -> Result<usize> {
        self.select_function_types(candidates, args, &[], offset)
    }
    fn select_function_types(
        &mut self,
        candidates: &[usize],
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
    ) -> Result<usize> {
        let mut applicable = Vec::new();
        let mut errors = Vec::new();
        for id in candidates {
            let next = self.next;
            let count = self.lambdas.borrow().len();
            let specialized = self.specialize_source(*id, args, explicit, offset);
            self.next = next;
            self.lambdas.borrow_mut().truncate(count);
            let (params, _) = match specialized {
                Ok(signature) => signature,
                Err(error) => {
                    errors.push(error);
                    continue;
                }
            };
            let defaults = self
                .defaults
                .get(*id)
                .map(|v| v.iter().map(Option::is_some).collect::<Vec<_>>())
                .unwrap_or_else(|| vec![false; params.len()]);
            match self.applicability_result(&params, &defaults, args, true, offset) {
                Ok(_) => applicable.push((*id, defaults)),
                Err(error) => errors.push(error),
            }
        }
        if applicable.is_empty() {
            if errors.len() == 1 {
                return Err(errors.pop().unwrap());
            }
            return Err(Diagnostic::new(
                offset,
                "No applicable function overload or wrong argument count",
            ));
        }
        let parameters = applicable
            .iter()
            .map(|(id, _)| self.signatures[*id].0.clone())
            .collect::<Vec<_>>();
        let defaults = applicable
            .iter()
            .map(|(_, defaults)| defaults.clone())
            .collect::<Vec<_>>();
        let generic = applicable
            .iter()
            .map(|(id, _)| {
                self.source_type_params
                    .get(*id)
                    .is_some_and(|p| !p.is_empty())
            })
            .collect::<Vec<_>>();
        let selected = self.most_specific(
            &parameters,
            &defaults,
            &vec![true; applicable.len()],
            args,
            &generic,
            offset,
        )?;
        Ok(applicable[selected].0)
    }
    fn select_method(
        &mut self,
        methods: &[JavaMethod],
        args: &[Expr],
        offset: usize,
    ) -> Result<JavaMethod> {
        self.select_method_types(methods, args, &[], offset)
    }
    fn select_method_types(
        &mut self,
        methods: &[JavaMethod],
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
    ) -> Result<JavaMethod> {
        self.select_method_expected(methods, args, explicit, None, offset)
    }
    fn select_method_expected(
        &mut self,
        methods: &[JavaMethod],
        args: &[Expr],
        explicit: &[Type],
        expected: Option<&Type>,
        offset: usize,
    ) -> Result<JavaMethod> {
        if args
            .iter()
            .any(|a| matches!(a.kind, ExprKind::NamedArgument(..)))
            && !methods.is_empty()
            && methods.iter().all(|m| !self.method_named(m))
        {
            return Err(Diagnostic::new(
                offset,
                "Named arguments are not allowed for Java declarations",
            ));
        }
        let mut applicable = Vec::new();
        let mut failures = Vec::new();
        for original in methods {
            if original.unsupported
                || (original.kotlin_internal && !self.classpath.internal_visible(&original.owner))
            {
                continue;
            }
            let normalized = match self.vararg_arguments(original, args, offset) {
                Ok(args) => args,
                Err(error) => {
                    failures.push(error);
                    continue;
                }
            };
            let normalized = normalized.as_slice();
            let next = self.next;
            let count = self.lambdas.borrow().len();
            let specialized =
                self.specialize_method_expected(original, normalized, explicit, expected, offset);
            self.next = next;
            self.lambdas.borrow_mut().truncate(count);
            let method = match specialized {
                Ok(method) => method,
                Err(error) => {
                    failures.push(error);
                    continue;
                }
            };
            let (params, defaults) = match Self::method_parameters(&method) {
                Ok(value) => value,
                Err(error) => {
                    if !self.unsupported_parameters_inapplicable(&method, args, offset)? {
                        return Err(Diagnostic::new(
                            offset,
                            "A potentially applicable overload has unsupported parameter types",
                        ));
                    }
                    failures.push(error);
                    continue;
                }
            };
            let previous = self.inline_result.clone();
            self.inline_result = if method.library.as_ref().is_some_and(|l| l.inline)
                && (self.allow_return || self.nonlocal_result.is_some())
            {
                Some(
                    self.nonlocal_result
                        .clone()
                        .unwrap_or_else(|| self.result.clone()),
                )
            } else {
                None
            };
            let previous_modes = std::mem::replace(
                &mut self.inline_argument_modes,
                method
                    .library
                    .as_ref()
                    .map(|library| library.parameter_modes.clone())
                    .unwrap_or_default(),
            );
            let score = self.applicability_result(
                &params,
                &defaults,
                normalized,
                self.method_named(&method),
                offset,
            );
            self.inline_result = previous;
            self.inline_argument_modes = previous_modes;
            match score {
                Ok(_) => applicable.push((method, original.clone())),
                Err(error) => failures.push(error),
            }
        }
        if applicable.is_empty() {
            if failures.len() == 1 {
                return Err(failures.pop().unwrap());
            }
            if let Some(error) = failures
                .iter()
                .find(|error| error.kind == crate::DiagnosticKind::UnsupportedLanguage)
            {
                return Err(error.clone());
            }
            if !methods.is_empty() && methods.iter().all(|m| m.kotlin_internal) {
                return Err(Diagnostic::new(
                    offset,
                    "Cannot access internal library declaration",
                ));
            }
            if methods.iter().any(|m| m.unsupported) && failures.is_empty() {
                return Err(
                    Diagnostic::new(offset, "Unsupported library declaration").with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "LIBRARY_DECLARATION_UNSUPPORTED",
                    ),
                );
            }
            return Err(Diagnostic::new(offset, "No applicable member overload"));
        }
        if applicable
            .iter()
            .any(|(_, original)| original.access & 0x80 == 0)
        {
            applicable.retain(|(_, original)| original.access & 0x80 == 0);
        }
        if applicable.len() > 1
            && applicable.iter().any(|(_, m)| {
                m.library
                    .as_ref()
                    .is_some_and(|l| l.type_parameters.iter().any(|p| p.bounds.len() > 1))
            })
        {
            return Err(Diagnostic::new(
                offset,
                "Overload specificity with intersection bounds is unsupported",
            ));
        }
        let declarations = applicable
            .iter()
            .map(|(_, m)| Self::method_parameters(m))
            .collect::<Result<Vec<_>>>()?;
        let parameters = declarations
            .iter()
            .map(|(p, _)| p.clone())
            .collect::<Vec<_>>();
        let defaults = declarations.into_iter().map(|(_, d)| d).collect::<Vec<_>>();
        let named = applicable
            .iter()
            .map(|(_, m)| self.method_named(m))
            .collect::<Vec<_>>();
        let generic = applicable
            .iter()
            .map(|(_, m)| {
                m.library
                    .as_ref()
                    .is_some_and(|l| !l.type_parameters.is_empty())
            })
            .collect::<Vec<_>>();
        let normalized_arguments = self.vararg_arguments(&applicable[0].0, args, offset)?;
        let selected = self.most_specific(
            &parameters,
            &defaults,
            &named,
            &normalized_arguments,
            &generic,
            offset,
        )?;
        Ok(applicable.swap_remove(selected).0)
    }
    fn unsupported_parameters_inapplicable(
        &mut self,
        method: &JavaMethod,
        args: &[Expr],
        offset: usize,
    ) -> Result<bool> {
        let bytes = method.descriptor.as_bytes();
        let mut cursor = 1;
        let mut parameters = Vec::new();
        while bytes.get(cursor) != Some(&b')') {
            let start = cursor;
            while bytes.get(cursor) == Some(&b'[') {
                cursor += 1;
            }
            if bytes.get(cursor) == Some(&b'L') {
                let Some(end) = method.descriptor[cursor..].find(';') else {
                    return Ok(false);
                };
                cursor += end + 1;
            } else if bytes.get(cursor).is_some() {
                cursor += 1;
            } else {
                return Ok(false);
            }
            parameters.push(&method.descriptor[start..cursor]);
        }
        if parameters.len() != args.len() {
            return Ok(true);
        }
        for (parameter, arg) in parameters.into_iter().zip(args) {
            if !matches!(parameter, "D" | "S" | "[D" | "[S") {
                continue;
            }
            if matches!(
                arg.kind,
                ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. } | ExprKind::Labeled(..)
            ) {
                return Ok(true);
            }
            let value = self.value(arg)?;
            let potential = match parameter {
                "D" => value.ty.base() == &Type::Double,
                "S" => matches!(value.ty.base(), Type::Int | Type::Java(_)),
                _ => value.ty.descriptor() == parameter,
            };
            if !potential {
                return Ok(true);
            }
        }
        let _ = offset;
        Ok(false)
    }
    fn method_parameters(method: &JavaMethod) -> Result<CallableParameters> {
        if let Some(params) = &method.kotlin_params {
            return Ok((
                params.iter().map(|p| (p.0.clone(), p.1.clone())).collect(),
                params.iter().map(|p| p.2).collect(),
            ));
        }
        let mut parameter_signature = method.clone();
        let end = method
            .descriptor
            .find(')')
            .ok_or_else(|| Diagnostic::new(0, "Invalid method descriptor"))?;
        parameter_signature.descriptor = format!("{}V", &method.descriptor[..=end]);
        parameter_signature.kotlin_result = None;
        let (types, _) = class_method_types(&parameter_signature)?;
        let count = types.len();
        Ok((
            types
                .into_iter()
                .enumerate()
                .map(|(i, t)| (format!("p{i}"), java_parameter(t)))
                .collect(),
            vec![false; count],
        ))
    }
    fn method_named(&self, method: &JavaMethod) -> bool {
        (method.library.is_none() && method.kotlin_params.is_some())
            || (method.owner == self.lambda_owner
                && self
                    .members
                    .values()
                    .any(|m| m.name == method.name && m.descriptor == method.descriptor))
            || self
                .classes
                .get(&method.owner)
                .is_some_and(|c| c.kotlin_class)
            || self
                .classpath
                .load(&method.owner)
                .is_ok_and(|c| c.kotlin_class)
    }
    fn factory(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        if name == "emptyList" && !args.is_empty() || type_args.len() > 1 {
            return Err(Diagnostic::new(
                offset,
                "Unsupported list factory arguments",
            ));
        }
        let mut elements = Vec::new();
        let mut element = type_args
            .first()
            .map(|ty| self.resolve(ty, offset))
            .transpose()?;
        for arg in args {
            let value = self.value(arg)?;
            if element.is_none() {
                element = Some(value.ty.clone());
            }
            elements.push(self.coerce(value, element.clone().unwrap(), arg.offset)?);
        }
        let element = element.unwrap_or(Type::Null);
        if matches!(element, Type::Unit | Type::ThreadLocal(_)) {
            return Err(Diagnostic::new(offset, "Unsupported list element type"));
        }
        let mutable = name == "mutableListOf";
        Ok(Value {
            ty: if mutable {
                Type::MutableList(Box::new(element))
            } else {
                Type::List(Box::new(element))
            },
            kind: ValueKind::ListFactory { mutable, elements },
        })
    }
    fn argument_mapping(
        params: &[(String, Type)],
        defaults: &[bool],
        args: &[Expr],
        named: bool,
        offset: usize,
    ) -> Result<Vec<usize>> {
        let mut used = HashSet::new();
        let mut mapping = Vec::new();
        let mut seen_named = false;
        for (position, arg) in args.iter().enumerate() {
            let index = if let ExprKind::NamedArgument(name, _) = &arg.kind {
                if !named {
                    return Err(Diagnostic::new(
                        arg.offset,
                        "Named arguments require a Kotlin declaration",
                    ));
                }
                seen_named = true;
                params.iter().position(|p| p.0 == *name).ok_or_else(|| {
                    Diagnostic::new(arg.offset, "Unknown named argument").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ARGUMENT_MAPPING_MISMATCH",
                    )
                })?
            } else {
                if seen_named {
                    return Err(Diagnostic::new(
                        arg.offset,
                        "Positional arguments after named arguments are not supported",
                    ));
                }
                position
            };
            if index >= params.len() || !used.insert(index) {
                return Err(
                    Diagnostic::new(arg.offset, "Wrong or duplicate arguments").with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ARGUMENT_MAPPING_MISMATCH",
                    ),
                );
            }
            mapping.push(index);
        }
        if (0..params.len())
            .any(|i| !used.contains(&i) && !defaults.get(i).copied().unwrap_or(false))
        {
            return Err(
                Diagnostic::new(offset, "Wrong argument count: missing required argument")
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "ARGUMENT_MAPPING_MISMATCH",
                    ),
            );
        }
        Ok(mapping)
    }
    fn inline_argument_value(
        &mut self,
        expression: &Expr,
        ty: &Type,
        index: usize,
    ) -> Result<Value> {
        let old = self.inline_result.clone();
        if self
            .inline_argument_modes
            .get(index)
            .is_some_and(|mode| *mode != syntax::InlineParameterMode::Ordinary)
        {
            self.inline_result = None;
        }
        let result = self.argument_value(expression, ty);
        self.inline_result = old;
        result
    }
    fn arguments(
        &mut self,
        params: &[(String, Type)],
        defaults: &[bool],
        args: &[Expr],
        named: bool,
        offset: usize,
    ) -> Result<CheckedArguments> {
        if params.len() > 64 && defaults.iter().any(|d| *d) {
            return Err(Diagnostic::new(
                offset,
                "Compiled default arguments support at most 64 parameters",
            ));
        }
        let mapping = Self::argument_mapping(params, defaults, args, named, offset)?;
        let reordered = mapping.iter().enumerate().any(|(i, p)| i != *p);
        let mut ordered: Vec<Option<Value>> = (0..params.len()).map(|_| None).collect();
        let mut lets = Vec::new();
        for (arg, index) in args.iter().zip(mapping) {
            let expr = if let ExprKind::NamedArgument(_, expr) = &arg.kind {
                expr.as_ref()
            } else {
                arg
            };
            let value = self.inline_argument_value(expr, &params[index].1, index)?;
            ordered[index] = Some(if reordered {
                let slot = self.allocate_typed_slot(&value.ty, offset)?;
                let ty = value.ty.clone();
                lets.push((slot, value));
                Value {
                    ty,
                    kind: ValueKind::Local(slot),
                }
            } else {
                value
            });
        }
        let mut mask = 0;
        let mut values = Vec::new();
        for (i, value) in ordered.into_iter().enumerate() {
            values.push(if let Some(v) = value {
                v
            } else {
                mask |= 1u64 << i;
                zero_value(&params[i].1)
            });
        }
        Ok((values, lets, mask))
    }
    fn wrap_lets(mut value: Value, lets: Vec<(u8, Value)>) -> Value {
        for (slot, initializer) in lets.into_iter().rev() {
            value = Value {
                ty: value.ty.clone(),
                kind: ValueKind::Let {
                    slot,
                    value: Box::new(initializer),
                    body: Box::new(value),
                },
            };
        }
        value
    }
    fn constructor_call(
        &mut self,
        owner: &str,
        params: &[(String, Type)],
        defaults: &[bool],
        args: &[Expr],
        named: bool,
        offset: usize,
    ) -> Result<Value> {
        if self
            .classpath
            .load(owner)
            .is_ok_and(|class| class.access & 0x4000 != 0)
        {
            return Err(Diagnostic::new(
                offset,
                "Enum constructors can only be called by enum entry initialization",
            ));
        }
        self.construct_value(owner, params, defaults, args, named, offset)
    }
    fn construct_value(
        &mut self,
        owner: &str,
        params: &[(String, Type)],
        defaults: &[bool],
        args: &[Expr],
        named: bool,
        offset: usize,
    ) -> Result<Value> {
        let (mut values, lets, mask) = self.arguments(params, defaults, args, named, offset)?;
        let mut descriptor = format!(
            "({}",
            params.iter().map(|p| p.1.descriptor()).collect::<String>()
        );
        if mask != 0 {
            descriptor.push_str(&"I".repeat(params.len().div_ceil(32)));
            descriptor.push_str("Lkotlin/jvm/internal/DefaultConstructorMarker;");
            for index in 0..params.len().div_ceil(32) {
                values.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int((mask >> (index * 32)) as i32),
                });
            }
            values.push(Value {
                ty: Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
                kind: ValueKind::Null,
            });
        }
        descriptor.push_str(")V");
        Ok(Self::wrap_lets(
            Value {
                ty: Type::Java(owner.into()),
                kind: ValueKind::New {
                    owner: owner.into(),
                    descriptor,
                    args: values,
                },
            },
            lets,
        ))
    }
    fn safe_access(
        &mut self,
        receiver: &Expr,
        name: &str,
        args: Option<&Vec<Expr>>,
        offset: usize,
    ) -> Result<Value> {
        let receiver = self.value(receiver)?;
        let inner = match receiver.ty.base() {
            Type::Nullable(inner) => inner.as_ref().clone(),
            Type::Null => {
                return Err(Diagnostic::new(
                    offset,
                    "Safe calls on literal null have no supported receiver type",
                ));
            }
            ty if ty.is_reference() => ty.clone(),
            _ => {
                return Err(Diagnostic::new(
                    offset,
                    "Safe calls require reference receivers",
                ));
            }
        };
        let slot = self.allocate_slot(offset)?;
        let mut scope = HashMap::new();
        scope.insert(
            "<safe>".into(),
            Local {
                slot,
                ty: receiver.ty.clone(),
                mutable: false,
            },
        );
        self.scopes.push(scope);
        self.refinements.insert(slot, inner);
        self.initialized.insert(slot);
        let property = Expr {
            offset,
            kind: ExprKind::Property(
                Box::new(Expr {
                    offset,
                    kind: ExprKind::Name("<safe>".into()),
                }),
                name.into(),
            ),
        };
        let expression = match args {
            Some(args) => Expr {
                offset,
                kind: ExprKind::Call(Box::new(property), args.clone()),
            },
            None => property,
        };
        let result = self.value(&expression);
        self.scopes.pop();
        self.initialized.remove(&slot);
        self.refinements.remove(&slot);
        let result = result?;
        if result.ty == Type::Unit {
            return Err(Diagnostic::new(
                offset,
                "Safe calls returning Unit are not supported",
            ));
        }
        let result = if result.ty.is_reference() {
            result
        } else {
            Value {
                ty: Type::Nullable(Box::new(result.ty.clone())),
                kind: ValueKind::Box(Box::new(result)),
            }
        };
        let ty = match result.ty.base() {
            Type::Nullable(_) => result.ty.clone(),
            ty => Type::Nullable(Box::new(ty.clone())),
        };
        let condition = Value {
            ty: Type::Boolean,
            kind: ValueKind::IsNull(Box::new(Value {
                ty: receiver.ty.clone(),
                kind: ValueKind::Local(slot),
            })),
        };
        Ok(Value {
            ty: ty.clone(),
            kind: ValueKind::Let {
                slot,
                value: Box::new(receiver),
                body: Box::new(Value {
                    ty: ty.clone(),
                    kind: ValueKind::If(
                        Box::new(condition),
                        Box::new(Value {
                            ty: ty.clone(),
                            kind: ValueKind::Null,
                        }),
                        Box::new(Value {
                            ty,
                            kind: result.kind,
                        }),
                    ),
                }),
            },
        })
    }
    fn elvis(&mut self, left: &Expr, right: &Expr, offset: usize) -> Result<Value> {
        let source_left = left;
        let left = self.value(left)?;
        if left.ty == Type::Null {
            let body = self.value(right)?;
            let slot = self.allocate_slot(offset)?;
            return Ok(Value {
                ty: body.ty.clone(),
                kind: ValueKind::Let {
                    slot,
                    value: Box::new(left),
                    body: Box::new(body),
                },
            });
        }
        let inner = match left.ty.base() {
            Type::Nullable(inner) | Type::Platform(inner) => inner.as_ref().clone(),
            ty if ty.is_reference() => ty.clone(),
            _ => {
                return Err(Diagnostic::new(
                    offset,
                    "Elvis requires a reference left operand",
                ));
            }
        };
        let right = self.value(right)?;
        let abrupt = matches!(right.kind, ValueKind::Abrupt(_));
        if abrupt {
            if let ExprKind::Name(name) = &source_left.kind {
                if let Some(local) = self.local(name) {
                    if !local.mutable {
                        self.refinements.insert(local.slot, inner.clone());
                    }
                }
            }
        }
        let ty = if right.ty == inner || abrupt {
            inner.clone()
        } else if right.ty == Type::Null || right.ty == Type::Nullable(Box::new(inner.clone())) {
            Type::Nullable(Box::new(inner.clone()))
        } else {
            return Err(Diagnostic::new(
                offset,
                "Elvis operands require the same supported reference type",
            ));
        };
        let slot = self.allocate_slot(offset)?;
        let condition = Value {
            ty: Type::Boolean,
            kind: ValueKind::IsNull(Box::new(Value {
                ty: left.ty.clone(),
                kind: ValueKind::Local(slot),
            })),
        };
        let left_ty = left.ty.clone();
        Ok(Value {
            ty: ty.clone(),
            kind: ValueKind::Let {
                slot,
                value: Box::new(left),
                body: Box::new(Value {
                    ty: ty.clone(),
                    kind: ValueKind::If(
                        Box::new(condition),
                        Box::new(if abrupt {
                            Value {
                                ty: ty.clone(),
                                kind: right.kind,
                            }
                        } else {
                            right
                        }),
                        Box::new(if !ty.is_reference() {
                            Value {
                                ty: ty.clone(),
                                kind: ValueKind::Unbox(Box::new(Value {
                                    ty: left_ty.clone(),
                                    kind: ValueKind::Local(slot),
                                })),
                            }
                        } else {
                            Value {
                                ty,
                                kind: ValueKind::Local(slot),
                            }
                        }),
                    ),
                }),
            },
        })
    }
    fn receiver_value(&self, offset: usize) -> Result<Value> {
        if let Some(receiver) = &self.receiver {
            return Ok(Value {
                ty: receiver.ty.clone(),
                kind: ValueKind::Local(receiver.slot),
            });
        }
        if let Some(owner) = self.object_owner {
            let kind = if let Some(outer) = owner.strip_suffix("$Companion") {
                ValueKind::StaticField {
                    owner: outer.into(),
                    name: "Companion".into(),
                }
            } else {
                ValueKind::ObjectInstance(owner.into())
            };
            return Ok(Value {
                ty: Type::Java(owner.into()),
                kind,
            });
        }
        Err(Diagnostic::new(offset, "Member access requires a receiver"))
    }
    fn source_backing_accessor(
        &self,
        method: &JavaMethod,
        mut args: Vec<Value>,
        offset: usize,
    ) -> Result<Option<Value>> {
        let Some(field) = self.classpath.default_source_accessor(method) else {
            return Ok(None);
        };
        let setter = method.source_name() == setter_name(&field.name);
        let (parameters, result) = class_method_types(method)?;
        let logical = if setter {
            parameters.last().unwrap().clone()
        } else {
            result
        };
        if logical.descriptor() != field.ty.descriptor() {
            return Err(Diagnostic::new(
                offset,
                "Specialized private default accessors with different JVM field representations are unsupported",
            ).with_kind(crate::DiagnosticKind::UnsupportedLanguage, "SPECIALIZED_PRIVATE_ACCESSOR_UNSUPPORTED"));
        }
        let receiver = if method.access & 8 == 0 {
            Some(args.remove(0))
        } else {
            None
        };
        let mut body = Vec::new();
        let receiver = if field.is_static {
            if let Some(receiver) = receiver {
                body.push(Statement::Expression(receiver));
            }
            None
        } else {
            receiver
        };
        let result = if field.companion_bridge {
            Value {
                ty: if setter { Type::Unit } else { logical.clone() },
                kind: ValueKind::StaticCall {
                    owner: field.owner,
                    name: format!(
                        "access${}$cp",
                        if setter {
                            setter_name(&field.name)
                        } else {
                            getter_name(&field.name)
                        }
                    ),
                    descriptor: if setter {
                        format!("({})V", field.ty.descriptor())
                    } else {
                        format!("(){}", field.ty.descriptor())
                    },
                    args,
                },
            }
        } else if setter {
            body.push(Statement::JavaFieldStore {
                owner: field.owner,
                name: field.name,
                ty: field.ty,
                receiver,
                value: args.remove(0),
            });
            Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            }
        } else {
            Value {
                ty: logical,
                kind: ValueKind::JavaField {
                    owner: field.owner,
                    name: field.name,
                    receiver: receiver.map(Box::new),
                },
            }
        };
        Ok(Some(if body.is_empty() {
            result
        } else {
            Value {
                ty: result.ty.clone(),
                kind: ValueKind::Block(body, Box::new(result)),
            }
        }))
    }
    fn member_call(
        &mut self,
        method: JavaMethod,
        receiver: Value,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let method = if let Some(class) = library::type_owner(&receiver.ty).and_then(|owner| {
            self.classes
                .get(owner)
                .cloned()
                .or_else(|| self.classpath.load(owner).ok())
        }) {
            if relations::projected_input(&method, &receiver.ty, &class, self.classpath) {
                return Err(Diagnostic::new(
                    offset,
                    "Cannot write through an out or star type projection",
                ));
            }
            library::specialize_receiver(method, &receiver.ty, &class, self.classpath)
        } else {
            method
        };
        if method.access & 1 == 0 {
            let private_allowed = self
                .classpath
                .file_private_visible(&self.lambda_owner, &method)
                || method.access & 2 != 0
                    && (self.lambda_owner == method.owner
                        || self.lambda_owner.starts_with(&format!("{}$", method.owner)));
            let protected_allowed = method.access & 4 != 0
                && self.receiver.as_ref().is_some_and(|current| {
                    self.subtype(&current.ty, &Type::Java(method.owner.clone()))
                        && self.subtype(&receiver.ty, &current.ty)
                });
            if !private_allowed && !protected_allowed {
                return Err(Diagnostic::new(
                    offset,
                    "Member is inaccessible from this receiver",
                ));
            }
        }
        if method.kotlin_internal && !self.classpath.internal_visible(&method.owner) {
            return Err(Diagnostic::new(
                offset,
                "Cannot access internal library member",
            ));
        }
        if method.unsupported {
            return Err(Diagnostic::new(offset, "Unsupported library member"));
        }
        let (types, mut result) = class_method_types(&method)?;
        if method.kotlin_result.is_none() && result.is_reference() {
            result = Type::Platform(Box::new(result));
        }
        let params = method
            .kotlin_params
            .as_ref()
            .map(|v| {
                v.iter()
                    .map(|p| (p.0.clone(), p.1.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                types
                    .into_iter()
                    .enumerate()
                    .map(|(i, t)| (format!("p{i}"), java_parameter(t)))
                    .collect()
            });
        let defaults = method
            .kotlin_params
            .as_ref()
            .map(|v| v.iter().map(|p| p.2).collect::<Vec<_>>())
            .unwrap_or_else(|| vec![false; params.len()]);
        let normalized = self.vararg_arguments(&method, args, offset)?;
        let args = normalized.as_slice();
        let (mut values, mut lets, mask) =
            self.arguments(&params, &defaults, args, self.method_named(&method), offset)?;
        let receiver = if !lets.is_empty() {
            let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
            let ty = receiver.ty.clone();
            lets.insert(0, (slot, receiver));
            Value {
                ty,
                kind: ValueKind::Local(slot),
            }
        } else {
            receiver
        };
        values.insert(0, receiver);
        if mask == 0
            && !self.public_inline_body
            && self.classpath.default_source_accessor(&method).is_some()
        {
            return Ok(Self::wrap_lets(
                self.source_backing_accessor(&method, values, offset)?
                    .unwrap(),
                lets,
            ));
        }
        let mut method = method;
        if mask == 0
            && self
                .classpath
                .load(&method.owner)
                .is_ok_and(|declaring| !declaring.kotlin_class && declaring.access & 1 == 0)
        {
            if let Some(qualifier) = library::type_owner(&values[0].ty) {
                if self
                    .classpath
                    .load(qualifier)
                    .is_ok_and(|class| class.access & 1 != 0)
                {
                    method.owner = qualifier.into();
                }
            }
        }
        let interface = self
            .classes
            .get(&method.owner)
            .is_some_and(|c| c.access & 0x0200 != 0)
            || self
                .classpath
                .load(&method.owner)
                .is_ok_and(|c| c.access & 0x0200 != 0);
        let annotation_mapping = if mask == 0
            && self
                .classpath
                .load(&method.owner)
                .is_ok_and(|class| class.access & 0x2000 != 0)
        {
            match (method.descriptor.as_str(), result.base()) {
                ("()Ljava/lang/Class;", Type::Parameterized { name, .. })
                    if name == "kotlin/reflect/KClass" =>
                {
                    Some((
                        "getOrCreateKotlinClass",
                        "(Ljava/lang/Class;)Lkotlin/reflect/KClass;",
                        Type::Java("java/lang/Class".into()),
                    ))
                }
                ("()[Ljava/lang/Class;", Type::Array(inner)) if matches!(inner.base(), Type::Parameterized { name, .. } if name == "kotlin/reflect/KClass") => {
                    Some((
                        "getOrCreateKotlinClasses",
                        "([Ljava/lang/Class;)[Lkotlin/reflect/KClass;",
                        Type::Array(Box::new(Type::Java("java/lang/Class".into()))),
                    ))
                }
                _ => None,
            }
        } else {
            None
        };
        let kind = if mask == 0 {
            if interface {
                ValueKind::MemberCall {
                    method,
                    interface,
                    args: values,
                }
            } else {
                ValueKind::JavaCall {
                    method,
                    args: values,
                }
            }
        } else {
            let descriptor = format!(
                "(L{};{}{}Ljava/lang/Object;){}",
                method.owner,
                method.descriptor[1..method.descriptor.find(')').unwrap()].to_owned(),
                "I".repeat(params.len().div_ceil(32)),
                &method.descriptor[method.descriptor.find(')').unwrap() + 1..]
            );
            for index in 0..params.len().div_ceil(32) {
                values.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int((mask >> (index * 32)) as i32),
                });
            }
            values.push(Value {
                ty: Type::Java("java/lang/Object".into()),
                kind: ValueKind::Null,
            });
            ValueKind::StaticCall {
                owner: method.owner,
                name: format!("{}$default", method.name),
                descriptor,
                args: values,
            }
        };
        let value = if let Some((name, descriptor, physical)) = annotation_mapping {
            let method = self
                .classpath
                .methods("kotlin/jvm/internal/Reflection")?
                .into_iter()
                .find(|method| {
                    method.name == name
                        && method.descriptor == descriptor
                        && method.access & 9 == 9
                        && !method.unsupported
                })
                .ok_or_else(|| {
                    Diagnostic::new(
                        offset,
                        "Annotation class conversion runtime declaration is unavailable",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "ANNOTATION_KCLASS_RUNTIME_UNSUPPORTED",
                    )
                })?;
            Value {
                ty: result,
                kind: ValueKind::StaticCall {
                    owner: method.owner,
                    name: method.name,
                    descriptor: method.descriptor,
                    args: vec![Value { ty: physical, kind }],
                },
            }
        } else {
            Value { ty: result, kind }
        };
        Ok(Self::wrap_lets(value, lets))
    }
    fn resolve(&self, ty: &Type, offset: usize) -> Result<Type> {
        let expanded =
            self.classpath
                .expand_type_alias(ty, self.imports, library::DEFAULT_PACKAGES)?;
        let imports = library::default_type_imports(&expanded, self.imports, self.classpath);
        let ty = resolve_type(&expanded, &imports, offset)?;
        self.validate_type_arguments(&ty, offset)?;
        Ok(ty)
    }
    #[allow(clippy::only_used_in_recursion)]
    fn coerce(&self, value: Value, expected: Type, offset: usize) -> Result<Value> {
        if matches!(value.kind, ValueKind::Abrupt(_)) {
            return Ok(Value {
                ty: expected,
                kind: value.kind,
            });
        }
        if value.ty == expected {
            return Ok(value);
        }
        if self.subtype(&value.ty, &expected)
            && !(matches!(value.ty, Type::Platform(_))
                && !expected.is_nullable()
                && !matches!(expected, Type::Platform(_)))
        {
            if value.ty.is_reference() {
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
            if expected.is_reference() {
                return Ok(Value {
                    ty: expected,
                    kind: ValueKind::Box(Box::new(value)),
                });
            }
        }
        if let Type::Platform(inner) = &expected {
            if inner.is_reference()
                && (value.ty == Type::Null
                    || matches!(&value.ty,Type::Nullable(t) if t.as_ref()==inner.as_ref()))
            {
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
        }
        if value.ty.is_reference()
            && expected.is_reference()
            && (expected.is_nullable() || matches!(expected, Type::Platform(_)))
            && value.ty.descriptor() == expected.descriptor()
            && self.subtype(&value.ty, &expected)
        {
            return Ok(Value {
                ty: expected,
                kind: value.kind,
            });
        }
        if !value.ty.is_reference()
            && expected.is_reference()
            && Type::Nullable(Box::new(value.ty.clone())).descriptor() == expected.descriptor()
        {
            return Ok(Value {
                ty: expected,
                kind: ValueKind::Box(Box::new(value)),
            });
        }

        if let Type::Nullable(inner) = &expected {
            if value.ty == Type::Null {
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
            if value.ty.base() == inner.as_ref() && value.ty.is_reference() {
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
            if !value.ty.is_nullable() {
                let value = self.coerce(value, inner.as_ref().clone(), offset)?;
                let boxed = !value.ty.is_reference();
                return Ok(Value {
                    ty: expected,
                    kind: if boxed {
                        ValueKind::Box(Box::new(value))
                    } else {
                        value.kind
                    },
                });
            }
        }
        if let (Type::List(expected_element), Type::List(element) | Type::MutableList(element)) =
            (&expected, &value.ty)
        {
            if matches!(expected_element.as_ref(),Type::Nullable(inner) if inner.as_ref()==element.as_ref())
            {
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
        }
        if let (Type::List(expected_element), Type::MutableList(element)) = (&expected, &value.ty) {
            if expected_element == element {
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
        }
        if let (
            Type::List(element) | Type::MutableList(element),
            ValueKind::ListFactory { elements, .. },
        ) = (&expected, &value.kind)
        {
            let compatible = matches!(
                (&expected, &value.ty),
                (Type::List(_), Type::List(_) | Type::MutableList(_))
                    | (Type::MutableList(_), Type::MutableList(_))
            );
            if elements.is_empty() && unresolved_list(&value.ty) && compatible {
                let _ = element;
                return Ok(Value {
                    ty: expected,
                    kind: value.kind,
                });
            }
        }
        if expected == Type::Long
            && value.ty == Type::Int
            && matches!(value.kind, ValueKind::Int(_))
        {
            if let ValueKind::Int(n) = value.kind {
                return Ok(Value {
                    ty: Type::Long,
                    kind: ValueKind::Long(i64::from(n)),
                });
            }
        }
        if expected == Type::Short
            && value.ty == Type::Int
            && matches!(value.kind, ValueKind::Int(n) if (-32768..=32767).contains(&n))
        {
            return Ok(Value {
                ty: Type::Short,
                kind: value.kind,
            });
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
        if matches!(value.ty, Type::Platform(_))
            && (value.ty.base() == &expected
                || value.ty.is_reference() && self.subtype(value.ty.base(), &expected))
        {
            fn expression_name(kind: &ValueKind) -> String {
                match kind {
                    ValueKind::Cast { value, .. }
                    | ValueKind::Unbox(value)
                    | ValueKind::Box(value) => expression_name(&value.kind),
                    ValueKind::JavaCall { method, .. } | ValueKind::MemberCall { method, .. } => {
                        format!("{}(...)", method.name)
                    }
                    ValueKind::StaticCall { name, .. } => format!("{name}(...)"),
                    ValueKind::JavaField { name, .. } => name.clone(),
                    ValueKind::ThreadLocalGet(_, _) => "get(...)".into(),
                    ValueKind::Let { body, .. } => expression_name(&body.kind),
                    _ => String::new(),
                }
            }
            let message = expression_name(&value.kind);
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
        )
        .with_kind(crate::DiagnosticKind::InvalidKotlin, "TYPE_MISMATCH"))
    }
    fn java_type(&self, descriptor: &str, cursor: &mut usize, offset: usize) -> Result<Type> {
        let bytes = descriptor.as_bytes();
        let start = *cursor;
        *cursor += 1;
        let ty = match bytes.get(start) {
            Some(b'I') => Type::Int,
            Some(b'J') => Type::Long,
            Some(b'F') => Type::Float,
            Some(b'D') => Type::Double,
            Some(b'B') => Type::Byte,
            Some(b'S') => Type::Short,
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
            Some(b'[') if bytes.get(*cursor) == Some(&b'J') => {
                *cursor += 1;
                Type::LongArray
            }
            Some(b'[') if bytes.get(*cursor) == Some(&b'Z') => {
                *cursor += 1;
                Type::BooleanArray
            }
            Some(b'[') if bytes.get(*cursor) == Some(&b'F') => {
                *cursor += 1;
                Type::FloatArray
            }
            Some(b'[') if bytes.get(*cursor) == Some(&b'D') => {
                *cursor += 1;
                Type::DoubleArray
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
    fn java_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let owner = library::type_owner(&receiver.ty)
            .ok_or_else(|| Diagnostic::new(offset, "Unsupported member receiver"))?;
        let class = self
            .classes
            .get(owner)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.classpath.load(owner))?;
        let inherited =
            match library::ordinary_member_methods(self.classpath, owner, name, args.len()) {
                Ok(methods) => methods,
                Err(_) if self.classes.contains_key(owner) => class.methods.clone(),
                Err(error) => return Err(error),
            };
        let singleton = class
            .fields
            .iter()
            .any(|f| f.name == "INSTANCE" && f.descriptor == format!("L{};", class.name));
        let methods = inherited
            .into_iter()
            .filter(|m| {
                m.source_property.is_none()
                    && m.source_name() == name
                    && (m.access & 8 == 0 || singleton)
                    && (m.access & 1 != 0
                        || m.access & 4 != 0
                            && self.receiver.as_ref().is_some_and(|current| {
                                self.subtype(&current.ty, &Type::Java(m.owner.clone()))
                                    && self.subtype(&receiver.ty, &current.ty)
                            })
                        || m.access & 2 != 0
                            && (self.lambda_owner == m.owner
                                || self.lambda_owner.starts_with(&format!("{}$", m.owner))))
            })
            .collect::<Vec<_>>();
        if methods.is_empty() {
            return self.member_extension_fallback(
                receiver,
                name,
                args,
                Diagnostic::new(
                    offset,
                    format!("Unresolved member or imported extension: {name}"),
                ),
            );
        }
        let projected = methods
            .iter()
            .any(|m| relations::projected_input(m, &receiver.ty, &class, self.classpath));
        let methods = methods
            .into_iter()
            .filter(|m| !relations::projected_input(m, &receiver.ty, &class, self.classpath))
            .map(|m| library::specialize_receiver(m, &receiver.ty, &class, self.classpath))
            .collect::<Vec<_>>();
        let methods = library::remove_overridden(methods, self.classpath);
        if projected && methods.is_empty() {
            return Err(Diagnostic::new(
                offset,
                "Cannot write through an out or star type projection",
            ));
        }
        let method = match self.select_method(&methods, args, offset) {
            Ok(method) => method,
            Err(_) if projected => {
                return Err(Diagnostic::new(
                    offset,
                    "Cannot write through an out or star type projection",
                ));
            }
            Err(error) if error.message.contains("Ambiguous") => return Err(error),
            Err(error) => return self.member_extension_fallback(receiver, name, args, error),
        };
        if method.access & 8 != 0 {
            let call = self.library_static_call(method, args, offset)?;
            let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
            return Ok(Value {
                ty: call.ty.clone(),
                kind: ValueKind::Let {
                    slot,
                    value: Box::new(receiver),
                    body: Box::new(call),
                },
            });
        }
        self.member_call(method, receiver, args, offset)
    }
    fn qualified_java_class(&self, expression: &Expr) -> Option<String> {
        fn parts<'a>(expression: &'a Expr, names: &mut Vec<&'a str>) -> bool {
            match &expression.kind {
                ExprKind::Name(name) => {
                    names.push(name);
                    true
                }
                ExprKind::Property(receiver, name) if parts(receiver, names) => {
                    names.push(name);
                    true
                }
                _ => false,
            }
        }
        let mut names = Vec::new();
        if !parts(expression, &mut names) || names.len() < 2 || !self.unshadowed(names[0]) {
            return None;
        }
        let owner = names.join("/");
        self.classpath
            .load(&owner)
            .ok()
            .filter(|class| class.access & 1 != 0 && !class.kotlin_class)
            .map(|_| owner)
    }
    fn java_static(
        &mut self,
        owner: &str,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        let methods = self
            .classpath
            .methods(owner)?
            .into_iter()
            .filter(|m| {
                m.source_name() == name
                    && m.access & 9 == 9
                    && (m.owner == owner
                        || self
                            .classpath
                            .load(&m.owner)
                            .is_ok_and(|declaring| declaring.access & 0x0200 == 0))
            })
            .collect::<Vec<_>>();
        let mut method = self.select_method(&methods, args, offset)?;
        if self
            .classpath
            .load(&method.owner)
            .is_ok_and(|declaring| !declaring.kotlin_class && declaring.access & 1 == 0)
        {
            method.owner = owner.into();
        }
        self.library_static_call(method, args, offset)
    }
    fn field_assignment(
        &mut self,
        receiver: &Expr,
        name: &str,
        expr: &Expr,
        offset: usize,
    ) -> Result<Statement> {
        if matches!(&receiver.kind,ExprKind::Name(n) if n=="this") {
            if let Some(method) = self.members.get(&format!("#set:{name}")) {
                return Ok(Statement::Expression(self.member_call(
                    method.clone(),
                    self.receiver_value(offset)?,
                    std::slice::from_ref(expr),
                    offset,
                )?));
            }
        }
        if matches!(&receiver.kind,ExprKind::Name(n) if n=="this") && self.fields.contains_key(name)
        {
            if !self.mutable_fields.contains(name) {
                return Err(Diagnostic::new(offset, "Cannot reassign val property"));
            }
            let value = self.value(expr)?;
            let value = self.coerce(value, self.fields[name].clone(), offset)?;
            return Ok(Statement::FieldStore {
                name: name.into(),
                receiver: self.receiver_value(offset)?,
                value,
            });
        }
        let (owner, receiver) = if let ExprKind::Name(alias) = &receiver.kind {
            if self.local(alias).is_none()
                && !self.fields.contains_key(alias)
                && !self.object_fields.contains_key(alias)
                && !self.constants.contains_key(alias)
            {
                if let Some(owner) = self.imports.get(alias) {
                    (owner.clone(), None)
                } else {
                    let value = self.value(receiver)?;
                    let Some(owner) = library::type_owner(&value.ty) else {
                        return Err(Diagnostic::new(
                            offset,
                            "Java field requires a class receiver",
                        ));
                    };
                    (owner.to_owned(), Some(value))
                }
            } else {
                let value = self.value(receiver)?;
                let Some(owner) = library::type_owner(&value.ty) else {
                    return Err(Diagnostic::new(
                        offset,
                        "Java field requires a class receiver",
                    ));
                };
                (owner.to_owned(), Some(value))
            }
        } else {
            let value = self.value(receiver)?;
            let Some(owner) = library::type_owner(&value.ty) else {
                return Err(Diagnostic::new(
                    offset,
                    "Java field requires a class receiver",
                ));
            };
            (owner.to_owned(), Some(value))
        };
        if receiver.as_ref().is_some_and(|v| v.ty.is_nullable()) {
            return Err(Diagnostic::new(
                offset,
                "Nullable field receivers require a not-null assertion",
            ));
        }
        self.assign_java_member(owner, receiver, name, expr, offset)
    }
    fn assign_java_member(
        &mut self,
        owner: String,
        receiver: Option<Value>,
        name: &str,
        expr: &Expr,
        offset: usize,
    ) -> Result<Statement> {
        let class = self
            .classes
            .get(&owner)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| self.classpath.load(&owner))?;
        let methods = self
            .classpath
            .methods(&owner)
            .unwrap_or_else(|_| class.methods.clone());
        if let Some(method) = methods
            .iter()
            .find(|m| {
                m.source_property.as_deref() == Some(name)
                    && m.source_name() == setter_name(name)
                    && (receiver.is_some()
                        || m.access & 8 != 0
                        || class.fields.iter().any(|field| field.name == "INSTANCE"))
            })
            .cloned()
        {
            let call = if method.access & 8 != 0 {
                self.library_static_call(method, std::slice::from_ref(expr), offset)?
            } else {
                let receiver = receiver.clone().unwrap_or_else(|| Value {
                    ty: Type::Java(owner.clone()),
                    kind: ValueKind::ObjectInstance(owner.clone()),
                });
                self.member_call(method, receiver, std::slice::from_ref(expr), offset)?
            };
            return Ok(Statement::Expression(call));
        }
        if receiver.is_none() && class.kotlin_class {
            if let Some(companion) = class
                .fields
                .iter()
                .find(|field| field.name == "Companion" && field.access & 9 == 9)
                .and_then(|field| {
                    field
                        .descriptor
                        .strip_prefix('L')
                        .and_then(|name| name.strip_suffix(';'))
                })
            {
                if let Some(method) =
                    self.classpath
                        .methods(companion)?
                        .into_iter()
                        .find(|method| {
                            method.source_property.as_deref() == Some(name)
                                && method.source_name() == setter_name(name)
                        })
                {
                    if method.access & 2 != 0
                        && self.lambda_owner != method.owner
                        && (self.lambda_owner == owner
                            || self.lambda_owner.starts_with(&format!("{owner}$")))
                    {
                        return Err(Diagnostic::new(offset, "Qualified private companion accessors from enclosing declarations require unsupported lexical bridge planning").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "PRIVATE_COMPANION_ACCESSOR_SCOPE_UNSUPPORTED"));
                    }
                    let receiver = Value {
                        ty: Type::Java(companion.into()),
                        kind: ValueKind::StaticField {
                            owner: owner.clone(),
                            name: "Companion".into(),
                        },
                    };
                    return Ok(Statement::Expression(self.member_call(
                        method,
                        receiver,
                        std::slice::from_ref(expr),
                        offset,
                    )?));
                }
            }
        }
        let candidates = self.classpath.field_candidates(&class.name, name)?;
        if !candidates.iter().any(|field| field.access & 1 != 0) {
            if let Some(value) = &receiver {
                if let Some(method) = self.java_property_setter(&value.ty, name, offset)? {
                    let call = self.member_call(
                        method,
                        value.clone(),
                        std::slice::from_ref(expr),
                        offset,
                    )?;
                    return Ok(Statement::Expression(call));
                }
            }
        }
        let field = match candidates.as_slice() {
            [field] => field,
            [] => return Err(Diagnostic::new(offset, "Unresolved Java field")),
            _ => {
                return Err(Diagnostic::new(offset, "Ambiguous inherited Java field")
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "AMBIGUOUS_JAVA_FIELD"));
            }
        };
        if field.access & 0x10 != 0 {
            return Err(Diagnostic::new(offset, "Cannot assign a final Java field"));
        }
        if field.access & 1 == 0
            || field.unsupported
            || (field.access & 8 != 0) != receiver.is_none()
        {
            return Err(Diagnostic::new(
                offset,
                "Java field is inaccessible or unsupported",
            ));
        }
        let mut cursor = 0;
        let ty = java_parameter(self.java_type(&field.descriptor, &mut cursor, offset)?);
        let value = self.value(expr)?;
        let value = self.coerce(value, ty.clone(), offset)?;
        Ok(Statement::JavaFieldStore {
            owner,
            name: name.into(),
            ty,
            receiver,
            value,
        })
    }
    fn java_field(
        &mut self,
        owner: &str,
        name: &str,
        receiver: Option<Value>,
        offset: usize,
    ) -> Result<Value> {
        let loaded = self.classpath.load(owner).ok();
        let class = self
            .classes
            .get(owner)
            .or(loaded.as_ref())
            .ok_or_else(|| Diagnostic::new(offset, "Unresolved Java class"))?;
        let candidates = self.classpath.field_candidates(&class.name, name)?;
        let field = match candidates.as_slice() {
            [field] => field,
            [] => return Err(Diagnostic::new(offset, "Unresolved Java field")),
            _ => {
                return Err(Diagnostic::new(offset, "Ambiguous inherited Java field")
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "AMBIGUOUS_JAVA_FIELD"));
            }
        };
        if !self
            .classpath
            .kotlin_field_visible(&self.lambda_owner, field)
        {
            return Err(Diagnostic::new(offset, "Field is inaccessible")
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "INVISIBLE_REFERENCE"));
        }
        if field.unsupported
            || field.access & 1 == 0 && !field.kotlin.as_ref().is_some_and(|field| field.constant)
            || (field.access & 8 != 0) != receiver.is_none()
        {
            return Err(Diagnostic::new(
                offset,
                "Java field is inaccessible or unsupported",
            ));
        }
        let mut cursor = 0;
        let ty = if let Some(kotlin) = &field.kotlin {
            kotlin.ty.clone()
        } else {
            self.java_type(&field.descriptor, &mut cursor, offset)?
        };
        if let Some(constant) = field.constant.as_ref().filter(|_| {
            field.access & 0x18 == 0x18 && field.kotlin.as_ref().is_none_or(|field| field.constant)
        }) {
            use crate::classpath::JavaConstant as C;
            let kind = match constant {
                C::Int(n) if ty == Type::Boolean => ValueKind::Boolean(*n != 0),
                C::Int(n) => ValueKind::Int(*n),
                C::Long(n) => ValueKind::Long(*n),
                C::Float(bits) => ValueKind::Float(*bits),
                C::Double(bits) => ValueKind::Double(*bits),
                C::String(v) => ValueKind::String(v.clone()),
            };
            let value = Value { ty, kind };
            return Ok(if let Some(receiver) = receiver {
                let slot = self.allocate_typed_slot(&receiver.ty, offset)?;
                Self::wrap_lets(value, vec![(slot, receiver)])
            } else {
                value
            });
        }
        let ty = if ty.is_reference() && field.kotlin.is_none() {
            Type::Platform(Box::new(ty))
        } else {
            ty
        };
        Ok(Value {
            ty,
            kind: ValueKind::JavaField {
                owner: owner.into(),
                name: name.into(),
                receiver: receiver.map(Box::new),
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
        let slot = self.allocate_typed_slot(&ty, offset)?;
        self.initialized.insert(slot);
        self.scopes
            .last_mut()
            .unwrap()
            .insert(name.into(), Local { slot, ty, mutable });
        Ok(slot)
    }
    fn allocate_typed_slot(&mut self, ty: &Type, offset: usize) -> Result<u8> {
        let width = ty.slots() as u16;
        if self.next + width > 256 {
            return Err(Diagnostic::new(
                offset,
                "More than 256 local slots are not supported",
            ));
        }
        let slot = self.next as u8;
        self.next += width;
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
        self.block_scoped(source, true)
    }
    fn block_scoped(
        &mut self,
        source: &[syntax::Statement],
        pop_scope: bool,
    ) -> Result<(Vec<Statement>, bool)> {
        self.cell_names
            .extend(captures::lambda_capture_names(source));
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
                StatementKind::Block(_)
                | StatementKind::LocalFunction(_)
                | StatementKind::Labeled(..)
                | StatementKind::BreakLabel(_)
                | StatementKind::ContinueLabel(_)
                | StatementKind::Destructure(..)
                | StatementKind::WhenBinding(..) => {
                    let (statements, terminal) = self.control_statement(statement)?;
                    body.extend(statements);
                    returns |= terminal;
                    continue;
                }
                StatementKind::ForEach(name, iterable, source_body) => {
                    self.check_for_iteration(name, iterable, source_body, offset)?
                }
                StatementKind::DoWhile(cond, source_body) => {
                    self.check_do_while(cond, source_body, offset)?
                }
                StatementKind::CompoundAssign(name, op, expr) => {
                    self.check_compound_assignment(name, op, expr, offset)?
                }
                StatementKind::SafePropertyAssign(receiver, name, expr) => {
                    let temporary = format!("<safe-assignment-{}>", self.next);
                    let named = || Expr {
                        offset,
                        kind: ExprKind::Name(temporary.clone()),
                    };
                    let statements = vec![
                        syntax::Statement {
                            offset,
                            kind: StatementKind::Let(
                                temporary.clone(),
                                false,
                                None,
                                receiver.clone(),
                            ),
                        },
                        syntax::Statement {
                            offset,
                            kind: StatementKind::If(
                                Expr {
                                    offset,
                                    kind: ExprKind::Binary(
                                        "!=".into(),
                                        Box::new(named()),
                                        Box::new(Expr {
                                            offset,
                                            kind: ExprKind::Null,
                                        }),
                                    ),
                                },
                                vec![syntax::Statement {
                                    offset,
                                    kind: StatementKind::PropertyAssign(
                                        Expr {
                                            offset,
                                            kind: ExprKind::NonNull(Box::new(named())),
                                        },
                                        name.clone(),
                                        expr.clone(),
                                    ),
                                }],
                                Vec::new(),
                            ),
                        },
                    ];
                    let (statements, _) = self.block(&statements)?;
                    body.extend(statements);
                    continue;
                }
                StatementKind::PropertyAssign(receiver, name, expr) => {
                    self.field_assignment(receiver, name, expr, offset)?
                }
                StatementKind::Declare(name, mutable, annotation) => {
                    let ty = self.resolve(annotation, offset)?;
                    if matches!(ty, Type::Unit | Type::ThreadLocal(_)) {
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
                    let slot = self.declare(name, ty.clone(), *mutable, offset)?;
                    if self.cell_names.contains(name) {
                        let owner = Self::cell_owner(&ty);
                        self.cells.insert(slot, owner.clone());
                        body.push(Statement::Store(
                            slot,
                            Value {
                                ty: Type::Java(owner.clone()),
                                kind: ValueKind::New {
                                    owner,
                                    descriptor: "()V".into(),
                                    args: Vec::new(),
                                },
                            },
                        ));
                    }
                    self.initialized.remove(&slot);
                    continue;
                }
                StatementKind::IndexAssign(array, index, value) => self.check_indices_assignment(
                    array,
                    std::slice::from_ref(index),
                    value,
                    offset,
                )?,
                StatementKind::IndicesAssign(array, indices, value) => {
                    self.check_indices_assignment(array, indices, value, offset)?
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
                    let refinements = self.refinements.clone();
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
                    self.refinements = refinements;
                    Statement::ForUntil {
                        slot,
                        start,
                        end_slot,
                        end,
                        body,
                    }
                }
                StatementKind::Let(name, mutable, annotation, expr) => {
                    let mut value = if let Some(ty) = annotation {
                        self.expected_value(expr, &self.resolve(ty, offset)?)?
                    } else {
                        self.value(expr)?
                    };
                    if let Some(ty) = annotation {
                        if matches!(ty, Type::ThreadLocal(_)) {
                            return Err(Diagnostic::new(
                                offset,
                                "Explicit ThreadLocal local types are not supported",
                            ));
                        }
                        value = self.coerce(value, self.resolve(ty, offset)?, offset)?;
                    }
                    if unresolved_list(&value.ty) {
                        return Err(Diagnostic::new(
                            offset,
                            "Cannot infer list element type; empty or null-only factories require an explicit or expected element type",
                        ));
                    }
                    if value.ty == Type::Unit {
                        return Err(Diagnostic::new(offset, "Unit locals are not supported"));
                    }
                    let slot = self.declare(name, value.ty.clone(), *mutable, offset)?;
                    if *mutable && self.cell_names.contains(name) {
                        let owner = Self::cell_owner(&value.ty);
                        self.cells.insert(slot, owner.clone());
                        body.push(Statement::Store(
                            slot,
                            Value {
                                ty: Type::Java(owner.clone()),
                                kind: ValueKind::New {
                                    owner: owner.clone(),
                                    descriptor: "()V".into(),
                                    args: Vec::new(),
                                },
                            },
                        ));
                        self.cell_store(
                            Value {
                                ty: Type::Java(owner),
                                kind: ValueKind::Local(slot),
                            },
                            value.ty.clone(),
                            value,
                            offset,
                        )?
                    } else {
                        Statement::Store(slot, value)
                    }
                }
                StatementKind::Assign(name, expr) => {
                    if name == "field" {
                        if let Some(backing) = self.members.get("#backing") {
                            let name = backing.name.clone();
                            if !self.mutable_fields.contains(&name) {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Cannot reassign val backing field",
                                ));
                            }
                            if let Some((owner, ty)) = self.object_fields.get(&name) {
                                let value = self.expected_value(expr, ty)?;
                                if self.lambda_owner.starts_with(&format!("{owner}$Companion")) {
                                    body.push(Statement::Expression(Value {
                                        ty: Type::Unit,
                                        kind: ValueKind::StaticCall {
                                            owner: owner.clone(),
                                            name: format!("access${}$cp", setter_name(&name)),
                                            descriptor: format!("({})V", ty.descriptor()),
                                            args: vec![value],
                                        },
                                    }));
                                } else {
                                    body.push(Statement::JavaFieldStore {
                                        owner: owner.clone(),
                                        name,
                                        ty: ty.clone(),
                                        receiver: None,
                                        value,
                                    });
                                }
                                continue;
                            }
                            let ty = self.fields[&name].clone();
                            let value = self.expected_value(expr, &ty)?;
                            body.push(Statement::FieldStore {
                                name,
                                receiver: self.receiver_value(offset)?,
                                value,
                            });
                            continue;
                        }
                    }
                    if self.local(name).is_none() {
                        if let Some(method) = self.members.get(&format!("#set:{name}")) {
                            let value = if method.access & 8 != 0 {
                                self.library_static_call(
                                    method.clone(),
                                    std::slice::from_ref(expr),
                                    offset,
                                )?
                            } else {
                                self.member_call(
                                    method.clone(),
                                    self.receiver_value(offset)?,
                                    std::slice::from_ref(expr),
                                    offset,
                                )?
                            };
                            body.push(Statement::Expression(value));
                            continue;
                        }
                        if !self.fields.contains_key(name)
                            && !self.captured.contains_key(name)
                            && !self.constants.contains_key(name)
                        {
                            if let Some((owner, member)) = self
                                .imports
                                .get(name)
                                .and_then(|binding| binding.rsplit_once("::"))
                                .map(|(owner, member)| (owner.to_owned(), member.to_owned()))
                            {
                                if self
                                    .classpath
                                    .field_candidates(&owner, &member)?
                                    .iter()
                                    .any(|field| field.access & 8 != 0)
                                {
                                    body.push(
                                        self.assign_java_member(
                                            owner, None, &member, expr, offset,
                                        )?,
                                    );
                                    continue;
                                }
                            }
                        }
                        if let Some((owner, ty)) = self.object_fields.get(name) {
                            if !self.mutable_fields.contains(name) {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Cannot reassign val property",
                                ));
                            }
                            let value = self.expected_value(expr, ty)?;
                            if self.lambda_owner.starts_with(&format!("{owner}$Companion")) {
                                body.push(Statement::Expression(Value {
                                    ty: Type::Unit,
                                    kind: ValueKind::StaticCall {
                                        owner: owner.clone(),
                                        name: format!("access${}$cp", setter_name(name)),
                                        descriptor: format!("({})V", ty.descriptor()),
                                        args: vec![value],
                                    },
                                }));
                            } else {
                                body.push(Statement::JavaFieldStore {
                                    owner: owner.clone(),
                                    name: name.clone(),
                                    ty: ty.clone(),
                                    receiver: None,
                                    value,
                                });
                            }
                            continue;
                        }
                    }
                    if self.local(name).is_none()
                        && !self.captured.contains_key(name)
                        && !self.fields.contains_key(name)
                        && !self.object_fields.contains_key(name)
                        && !self.constants.contains_key(name)
                    {
                        let setters = self
                            .library_candidates(name)
                            .into_iter()
                            .filter(|m| {
                                m.source_property.is_some()
                                    && m.kotlin_params.as_ref().is_some_and(|p| p.len() == 1)
                            })
                            .collect::<Vec<_>>();
                        if !setters.is_empty() {
                            let method =
                                self.select_method(&setters, std::slice::from_ref(expr), offset)?;
                            body.push(Statement::Expression(self.library_static_call(
                                method,
                                std::slice::from_ref(expr),
                                offset,
                            )?));
                            continue;
                        }
                        if self
                            .library_candidates(name)
                            .iter()
                            .any(|m| m.source_property.is_some())
                        {
                            return Err(Diagnostic::new(
                                offset,
                                "Cannot reassign read-only library property",
                            ));
                        }
                    }
                    if self.cell_is_readonly(name) {
                        return Err(Diagnostic::new(
                            offset,
                            "Cannot reassign val or function declaration",
                        ));
                    }
                    if let Some((cell, ty)) = self.cell_binding(name) {
                        let value = self.expected_value(expr, &ty)?;
                        if let Some(local) = self.local(name) {
                            self.initialized.insert(local.slot);
                        }
                        body.push(self.cell_store(cell, ty, value, offset)?);
                        continue;
                    }
                    if self.local(name).is_none() && self.fields.contains_key(name) {
                        if !self.mutable_fields.contains(name) {
                            return Err(Diagnostic::new(
                                offset,
                                format!("Cannot reassign val or parameter: {name}"),
                            ));
                        }
                        let value = self.value(expr)?;
                        let value = self.coerce(value, self.fields[name].clone(), offset)?;
                        let receiver = self.receiver_value(offset)?;
                        body.push(Statement::FieldStore {
                            name: name.clone(),
                            receiver,
                            value,
                        });
                        continue;
                    }
                    let local = self.local(name).ok_or_else(|| {
                        if self
                            .members
                            .values()
                            .any(|method| method.source_property.as_deref() == Some(name))
                        {
                            Diagnostic::new(
                                offset,
                                "Property assignments in method bodies are not supported",
                            )
                        } else if self.fields.contains_key(name) {
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
                    self.check_compound_assignment(name, "+=", expr, offset)?
                }
                StatementKind::When(subject, branches, otherwise) => {
                    let source_subject = subject;
                    let before_refinements = self.refinements.clone();
                    let subject = self.value(subject)?;
                    if subject.ty == Type::Unit {
                        return Err(Diagnostic::new(offset, "Unsupported when subject type"));
                    }
                    let exhaustive = otherwise.is_none()
                        && !matches!(source_subject.kind, ExprKind::WhenSubjectless)
                        && self.closed_when_type(&subject.ty);
                    if exhaustive {
                        self.require_when_exhaustive(
                            &subject.ty,
                            &branches
                                .iter()
                                .flat_map(|(conditions, _)| conditions.iter())
                                .collect::<Vec<_>>(),
                            offset,
                        )?;
                    }
                    let subject_type = subject.ty.clone();
                    let slot = self.allocate_typed_slot(&subject.ty, offset)?;
                    body.push(Statement::Store(slot, subject));
                    let before = self.initialized.clone();
                    let mut continuing = Vec::new();
                    let mut checked = Vec::new();
                    let mut all_return = otherwise.is_some() || exhaustive;
                    for (conditions, branch) in branches {
                        self.initialized = before.clone();
                        self.refinements = before_refinements.clone();
                        let mut combined = None;
                        for condition in conditions {
                            let left = Value {
                                ty: subject_type.clone(),
                                kind: ValueKind::Local(slot),
                            };
                            let equal = self.check_when_condition(left, condition)?;
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
                        self.refine_when_branch(source_subject, conditions);
                        let (branch, branch_returns) = self.block(branch)?;
                        if !branch_returns {
                            continuing.push(self.initialized.clone());
                        }
                        all_return &= branch_returns;
                        checked.push((combined.unwrap(), branch));
                    }
                    self.initialized = before.clone();
                    self.refinements = before_refinements.clone();
                    let mut tail = if let Some(otherwise) = otherwise {
                        let (branch, branch_returns) = self.block(otherwise)?;
                        if !branch_returns {
                            continuing.push(self.initialized.clone());
                        }
                        all_return &= branch_returns;
                        branch
                    } else if exhaustive {
                        let value = self.unmatched_when(Type::Unit, offset)?;
                        let ValueKind::Abrupt(body) = value.kind else {
                            unreachable!()
                        };
                        body
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
                    self.refinements = before_refinements;
                    returns |= all_return;
                    continue;
                }
                StatementKind::Return(expr) => {
                    if !self.allow_return && self.nonlocal_result.is_none() {
                        return Err(Diagnostic::new(
                            offset,
                            "Non-local return requires an inline lambda",
                        )
                        .with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "NON_LOCAL_RETURN_NOT_ALLOWED",
                        ));
                    }
                    let expected = self
                        .nonlocal_result
                        .clone()
                        .unwrap_or_else(|| self.result.clone());
                    let value = match expr {
                        Some(expr) => self.expected_value(expr, &expected)?,
                        None => Value {
                            ty: Type::Unit,
                            kind: ValueKind::Unit,
                        },
                    };
                    let value = self.coerce(value, expected, offset)?;
                    returns = true;
                    if self.nonlocal_result.is_some() {
                        Statement::NonLocalReturn(value)
                    } else {
                        Statement::Return(value)
                    }
                }
                StatementKind::Throw(expr) => {
                    let statement = self.checked_throw(expr, offset)?;
                    returns = true;
                    statement
                }
                StatementKind::Expression(expr) => {
                    returns |= control::expression_terminates(expr);
                    Statement::Expression(self.value(expr)?)
                }
                StatementKind::If(cond, yes, no) => {
                    let source_condition = cond;
                    let before_refinements = self.refinements.clone();
                    let cond = self.value(cond)?;
                    self.require(&cond, Type::Boolean, offset)?;
                    let before = self.initialized.clone();
                    self.refine(source_condition, true);
                    let (yes, y) = self.block(yes)?;
                    let yes_refinements = self.refinements.clone();
                    self.refinements = before_refinements.clone();
                    self.refine(source_condition, false);
                    let yes_initialized = self.initialized.clone();
                    self.initialized = before;
                    let (no, n) = self.block(no)?;
                    let no_initialized = self.initialized.clone();
                    let no_refinements = self.refinements.clone();
                    self.refinements = match (y, n) {
                        (true, false) => no_refinements,
                        (false, true) => yes_refinements,
                        _ => before_refinements,
                    };
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
                    self.exhaustiveness_flow |= y != n;
                    returns |= y && n;
                    Statement::If(cond, yes, no)
                }
                StatementKind::While(cond, body) => {
                    let source_condition = cond;
                    let refinements = self.refinements.clone();
                    let cond = self.value(cond)?;
                    self.require(&cond, Type::Boolean, offset)?;
                    let before = self.initialized.clone();
                    self.refine(source_condition, true);
                    self.loop_depth += 1;
                    let (body, _) = self.block(body)?;
                    self.loop_depth -= 1;
                    self.initialized = before;
                    self.refinements = refinements;
                    Statement::While(cond, body)
                }
            };
            body.push(typed);
        }
        if pop_scope {
            self.scopes.pop();
        }
        Ok((body, returns))
    }
    fn value(&mut self, expr: &Expr) -> Result<Value> {
        let offset = expr.offset;
        if let ExprKind::Call(target, args) = &expr.kind {
            if let Some(value) = self.qualified_constructor(target, args, offset)? {
                return Ok(value);
            }
        }

        let (ty, kind) = match &expr.kind {
            ExprKind::AnnotationConstant { .. } | ExprKind::AnnotationArray(_) => {
                return Err(Diagnostic::new(
                    expr.offset,
                    "Annotation array literals cannot be evaluated as ordinary expressions",
                )
                .with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "ANNOTATION_ARRAY_EXPRESSION_INVALID",
                ));
            }
            ExprKind::WhenSubjectless => (Type::Boolean, ValueKind::Boolean(true)),
            ExprKind::MissingWhenElse => {
                return Err(Diagnostic::new(
                    offset,
                    "A missing when branch requires exhaustive coverage",
                )
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_EXHAUSTIVE_WHEN"));
            }
            ExprKind::ClassLiteral {
                receiver,
                classifier,
            } => return self.class_literal(receiver, classifier.as_ref(), offset),
            ExprKind::VarargArray { array, elements } => {
                return self.vararg_array(array, elements, offset);
            }
            ExprKind::Block(_)
            | ExprKind::Try { .. }
            | ExprKind::Labeled(..)
            | ExprKind::LabeledReturn(..)
            | ExprKind::WhenBinding(..) => return self.control_value(expr),
            ExprKind::Decrement { value, prefix } => {
                return self.check_decrement(value, *prefix, offset);
            }
            ExprKind::TypeOperation(op, value, target) => {
                return self.check_type_operation(op, value, target, offset);
            }
            ExprKind::Int(value) => {
                if let Ok(value) = i32::try_from(*value) {
                    (Type::Int, ValueKind::Int(value))
                } else {
                    (Type::Long, ValueKind::Long(*value))
                }
            }
            ExprKind::Long(value) => (
                Type::Long,
                ValueKind::Long(
                    i64::try_from(*value)
                        .map_err(|_| Diagnostic::new(offset, "Long literal is out of range"))?,
                ),
            ),
            ExprKind::Float(bits) => (Type::Float, ValueKind::Float(*bits)),
            ExprKind::Double(bits) => (Type::Double, ValueKind::Double(*bits)),
            ExprKind::NamedArgument(_, _) | ExprKind::Generic(_, _) => {
                return Err(Diagnostic::new(
                    offset,
                    "Named arguments and generic types require a supported call",
                ));
            }
            ExprKind::Return(expr) => {
                if !self.allow_return && self.nonlocal_result.is_none() {
                    return Err(Diagnostic::new(
                        offset,
                        "Return expressions require an enclosing function or inline lambda",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "NON_LOCAL_RETURN_NOT_ALLOWED",
                    ));
                }
                let value = match expr {
                    Some(value) => self.value(value)?,
                    None => Value {
                        ty: Type::Unit,
                        kind: ValueKind::Unit,
                    },
                };
                let expected = self
                    .nonlocal_result
                    .clone()
                    .unwrap_or_else(|| self.result.clone());
                let value = self.coerce(value, expected, offset)?;
                let statement = if self.nonlocal_result.is_some() {
                    Statement::NonLocalReturn(value)
                } else {
                    Statement::Return(value)
                };
                (self.result.clone(), ValueKind::Abrupt(vec![statement]))
            }
            ExprKind::Throw(expr) => (
                Type::Unit,
                ValueKind::Abrupt(vec![self.checked_throw(expr, offset)?]),
            ),
            ExprKind::Null => (Type::Null, ValueKind::Null),
            ExprKind::SafeProperty(receiver, name) => {
                return self.safe_access(receiver, name, None, offset);
            }
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
                if let Some((cell, ty)) = self.cell_binding(name) {
                    if ty != Type::Int {
                        return Err(Diagnostic::new(offset, "Captured increment requires Int"));
                    }
                    return Ok(Value {
                        ty: Type::Int,
                        kind: ValueKind::CellIncrement {
                            cell: Box::new(cell),
                            prefix: *prefix,
                        },
                    });
                }
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
                let inner = match value.ty.base() {
                    Type::Nullable(inner) => inner.as_ref().clone(),
                    Type::Null => {
                        return Err(Diagnostic::new(
                            offset,
                            "A null literal has no supported asserted type",
                        ));
                    }
                    ty => ty.clone(),
                };
                if !inner.is_reference() {
                    let nullable = value.ty.clone();
                    return Ok(Value {
                        ty: inner,
                        kind: ValueKind::Unbox(Box::new(Value {
                            ty: nullable,
                            kind: ValueKind::RequireNotNull(Box::new(value)),
                        })),
                    });
                }
                (inner, ValueKind::RequireNotNull(Box::new(value)))
            }
            ExprKind::Lambda(_)
            | ExprKind::LambdaWithParams { .. }
            | ExprKind::CallableReference { .. } => {
                return Err(Diagnostic::new(
                    offset,
                    "Lambdas are supported only in private ThreadLocal array initializers",
                ));
            }
            ExprKind::When(subject, branches, otherwise) => {
                let source_subject = subject;
                let before_refinements = self.refinements.clone();
                let subject = self.value(subject)?;
                if subject.ty == Type::Unit {
                    return Err(Diagnostic::new(
                        offset,
                        "Unsupported when expression subject type",
                    ));
                }
                let missing = matches!(otherwise.kind, ExprKind::MissingWhenElse);
                if missing {
                    if matches!(source_subject.kind, ExprKind::WhenSubjectless) {
                        return Err(Diagnostic::new(
                            offset,
                            "Subjectless when expressions require an else branch",
                        )
                        .with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_EXHAUSTIVE_WHEN"));
                    }
                    self.require_when_exhaustive(
                        &subject.ty,
                        &branches
                            .iter()
                            .flat_map(|(conditions, _)| conditions.iter())
                            .collect::<Vec<_>>(),
                        offset,
                    )?;
                }
                let slot = self.allocate_typed_slot(&subject.ty, offset)?;
                let mut checked = Vec::new();
                for (conditions, branch) in branches {
                    self.refinements = before_refinements.clone();
                    let mut combined = None;
                    for condition in conditions {
                        let left = Value {
                            ty: subject.ty.clone(),
                            kind: ValueKind::Local(slot),
                        };
                        let condition = self.check_when_condition(left, condition)?;
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
                    self.refine_when_branch(source_subject, conditions);
                    checked.push((
                        combined.ok_or_else(|| Diagnostic::new(offset, "Empty when condition"))?,
                        self.value(branch)?,
                    ));
                }
                self.refinements = before_refinements.clone();
                let mut body = if missing {
                    let mut result: Option<Type> = None;
                    for (_, value) in &checked {
                        if matches!(value.kind, ValueKind::Abrupt(_)) {
                            continue;
                        }
                        result = Some(match result {
                            None => value.ty.clone(),
                            Some(previous)
                                if previous == value.ty || self.subtype(&value.ty, &previous) =>
                            {
                                previous
                            }
                            Some(previous) if self.subtype(&previous, &value.ty) => {
                                value.ty.clone()
                            }
                            Some(Type::Null) => {
                                if value.ty.is_nullable() {
                                    value.ty.clone()
                                } else {
                                    Type::Nullable(Box::new(value.ty.clone()))
                                }
                            }
                            Some(previous) if value.ty == Type::Null => {
                                if previous.is_nullable() {
                                    previous
                                } else {
                                    Type::Nullable(Box::new(previous))
                                }
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    offset,
                                    "The exhaustive when branch common type is unsupported",
                                )
                                .with_kind(
                                    crate::DiagnosticKind::UnsupportedLanguage,
                                    "WHEN_RESULT_JOIN_UNSUPPORTED",
                                ));
                            }
                        });
                    }
                    let result = result.ok_or_else(|| Diagnostic::new(offset, "Exhaustive when expressions with only abrupt branches require unsupported Nothing inference").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "WHEN_ABRUPT_RESULT_UNSUPPORTED"))?;
                    self.unmatched_when(result, offset)?
                } else {
                    self.value(otherwise)?
                };
                self.refinements = before_refinements;
                if body.ty == Type::Unit {
                    return Err(Diagnostic::new(
                        offset,
                        "Unit when expressions are not supported",
                    ));
                }
                for (condition, branch) in checked.into_iter().rev() {
                    let branch = self.coerce(branch, body.ty.clone(), offset)?;
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
                let source_condition = condition;
                let before_refinements = self.refinements.clone();
                let condition = self.value(condition)?;
                self.require(&condition, Type::Boolean, offset)?;
                self.refine(source_condition, true);
                let mut yes = self.value(yes)?;
                self.refinements = before_refinements.clone();
                self.refine(source_condition, false);
                let mut no = self.value(no)?;
                self.refinements = before_refinements;
                if yes.ty == Type::Unit {
                    return Err(Diagnostic::new(
                        offset,
                        "Unit if expressions are not supported",
                    ));
                }
                if yes.ty != no.ty {
                    let common = if yes.ty == Type::Null {
                        Some(if no.ty.is_nullable() {
                            no.ty.clone()
                        } else {
                            Type::Nullable(Box::new(no.ty.clone()))
                        })
                    } else if no.ty == Type::Null {
                        Some(if yes.ty.is_nullable() {
                            yes.ty.clone()
                        } else {
                            Type::Nullable(Box::new(yes.ty.clone()))
                        })
                    } else if yes.ty.is_nullable() && self.subtype(&no.ty, &yes.ty) {
                        Some(yes.ty.clone())
                    } else if no.ty.is_nullable() && self.subtype(&yes.ty, &no.ty) {
                        Some(no.ty.clone())
                    } else if (matches!(yes.ty, Type::Platform(_))
                        || matches!(no.ty, Type::Platform(_)))
                        && yes.ty.base() == no.ty.base()
                    {
                        Some(Type::Platform(Box::new(yes.ty.base().clone())))
                    } else if self.subtype(&yes.ty, &no.ty) {
                        Some(no.ty.clone())
                    } else if self.subtype(&no.ty, &yes.ty) {
                        Some(yes.ty.clone())
                    } else {
                        None
                    };
                    if let Some(common) = common {
                        yes = self.coerce(yes, common.clone(), offset)?;
                        no = self.coerce(no, common, offset)?;
                    } else {
                        return Err(Diagnostic::new(
                            offset,
                            "Type mismatch between if expression branches; common type inference is not supported",
                        ));
                    }
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
                    if !matches!(
                        value.ty.base(),
                        Type::String
                            | Type::Int
                            | Type::Long
                            | Type::Byte
                            | Type::Short
                            | Type::Float
                            | Type::Double
                            | Type::Boolean
                            | Type::Char
                            | Type::Null
                    ) && !matches!(value.ty.base(), Type::Nullable(inner) if matches!(inner.as_ref(), Type::String | Type::Int | Type::Long | Type::Byte | Type::Short | Type::Float | Type::Double | Type::Boolean | Type::Char))
                    {
                        return Err(Diagnostic::new(
                            part.offset,
                            "String templates support only String, supported primitives, their nullable forms, and null",
                        ).with_kind(crate::DiagnosticKind::UnsupportedLanguage, "STRING_TEMPLATE_VALUE_UNSUPPORTED"));
                    }
                    result = Value {
                        ty: Type::String,
                        kind: ValueKind::StringConcat(Box::new(result), Box::new(value)),
                    };
                }
                return Ok(result);
            }
            ExprKind::Name(name) => {
                if name == "field" {
                    if let Some(backing) = self.members.get("#backing") {
                        if let Some((owner, ty)) = self.object_fields.get(&backing.name) {
                            return Ok(Value {
                                ty: ty.clone(),
                                kind: if self
                                    .lambda_owner
                                    .starts_with(&format!("{owner}$Companion"))
                                {
                                    ValueKind::StaticCall {
                                        owner: owner.clone(),
                                        name: format!("access${}$cp", getter_name(&backing.name)),
                                        descriptor: format!("(){}", ty.descriptor()),
                                        args: Vec::new(),
                                    }
                                } else {
                                    ValueKind::StaticField {
                                        owner: owner.clone(),
                                        name: backing.name.clone(),
                                    }
                                },
                            });
                        }
                        let ty = self.fields.get(&backing.name).cloned().ok_or_else(|| {
                            Diagnostic::new(offset, "Accessor has no backing field")
                        })?;
                        return Ok(Value {
                            ty,
                            kind: ValueKind::Field(
                                backing.name.clone(),
                                Box::new(self.receiver_value(offset)?),
                            ),
                        });
                    }
                }
                if self.local(name).is_none() && !self.captured.contains_key(name) {
                    if let Some(method) = self.members.get(&format!("#get:{name}")) {
                        return if method.access & 8 != 0 {
                            self.library_static_call(method.clone(), &[], offset)
                        } else {
                            self.member_call(
                                method.clone(),
                                self.receiver_value(offset)?,
                                &[],
                                offset,
                            )
                        };
                    }
                }
                if let Some(value) = self.imported_value(name, offset)? {
                    return Ok(value);
                }
                let getters = self
                    .library_candidates(name)
                    .into_iter()
                    .filter(|m| {
                        m.source_property.is_some()
                            && m.kotlin_params.as_ref().is_some_and(|p| p.is_empty())
                    })
                    .collect::<Vec<_>>();
                if !getters.is_empty()
                    && self.local(name).is_none()
                    && !self.captured.contains_key(name)
                    && !self.fields.contains_key(name)
                    && !self.constants.contains_key(name)
                    && !self.object_fields.contains_key(name)
                    && !self.companion_fields.contains_key(name)
                {
                    let method = self.select_method(&getters, &[], offset)?;
                    return self.library_static_call(method, &[], offset);
                }
                if let Some((cell, ty)) = self.cell_binding(name) {
                    if let Some(local) = self.local(name) {
                        if !self.initialized.contains(&local.slot) {
                            return Err(Diagnostic::new(
                                offset,
                                "Captured variable must be initialized before use",
                            ));
                        }
                    }
                    return Ok(self.cell_read(cell, ty));
                }
                if let Some((owner, ty)) = self.captured.get(name) {
                    return Ok(Value {
                        ty: ty.clone(),
                        kind: ValueKind::JavaField {
                            owner: owner.clone(),
                            name: name.clone(),
                            receiver: Some(Box::new(Value {
                                ty: Type::Java(owner.clone()),
                                kind: ValueKind::Local(0),
                            })),
                        },
                    });
                }
                if let Some((ty, expr)) = self
                    .constants
                    .get(name)
                    .filter(|_| self.local(name).is_none() && !self.fields.contains_key(name))
                {
                    let value = self.value(expr)?;
                    return self.coerce(value, ty.clone(), offset);
                }
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
                    if let Some(refined) = self.refinements.get(&local.slot) {
                        if !local.mutable {
                            let raw = Value {
                                ty: local.ty.clone(),
                                kind: ValueKind::Local(local.slot),
                            };
                            return Ok(Value {
                                ty: refined.clone(),
                                kind: ValueKind::Cast {
                                    value: Box::new(raw),
                                    target: refined.clone(),
                                    safe: false,
                                },
                            });
                        }
                    }
                    (local.ty, ValueKind::Local(local.slot))
                } else if let Some((owner, ty)) = self.object_fields.get(name) {
                    (
                        ty.clone(),
                        if self.lambda_owner.starts_with(&format!("{owner}$Companion")) {
                            ValueKind::StaticCall {
                                owner: owner.clone(),
                                name: format!("access${}$cp", getter_name(name)),
                                descriptor: format!("(){}", ty.descriptor()),
                                args: Vec::new(),
                            }
                        } else {
                            ValueKind::StaticField {
                                owner: owner.clone(),
                                name: name.clone(),
                            }
                        },
                    )
                } else if name == "this" && self.object_owner.is_some() {
                    let value = self.receiver_value(offset)?;
                    (value.ty, value.kind)
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
                } else if let Some(method) = self.members.values().find(|method| {
                    method.source_property.as_deref() == Some(name)
                        && method.source_name() == getter_name(name)
                }) {
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
                    if let Some(value) = self.imported_value(name, offset)? {
                        return Ok(value);
                    }
                    if self.receiver.is_some() {
                        return self.receiver_property(name, offset);
                    }
                    return Err(Diagnostic::new(offset, format!("Unresolved local: {name}")));
                }
            }
            ExprKind::Unary(op, inner) => {
                if let ExprKind::Int(n) = inner.kind {
                    if op == "-" && n <= i64::from(i32::MAX) {
                        return Ok(Value {
                            ty: Type::Int,
                            kind: ValueKind::Int(-(n as i32)),
                        });
                    }
                }
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
                    if !matches!(
                        inner.ty,
                        Type::Int | Type::Long | Type::Float | Type::Double
                    ) {
                        return Err(Diagnostic::new(
                            offset,
                            "Unary arithmetic requires Int or Long",
                        ));
                    }
                    if op == "+" {
                        return Ok(inner);
                    }
                    (inner.ty.clone(), ValueKind::Negate(Box::new(inner)))
                }
            }
            ExprKind::Binary(op, a, b) => {
                if op == "?:" {
                    return self.elvis(a, b, offset);
                }
                if op == "in" || op == "!in" {
                    let value = self.value(a)?;
                    let value = self.check_contains(value, b, offset)?;
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
                    let start = self.value(a)?;
                    let end = self.value(b)?;
                    self.require(&end, start.ty.clone(), offset)?;
                    let (owner, descriptor) = match start.ty {
                        Type::Int => ("kotlin/ranges/IntRange", "(II)V"),
                        Type::Char => ("kotlin/ranges/CharRange", "(CC)V"),
                        Type::Long => ("kotlin/ranges/LongRange", "(JJ)V"),
                        _ => {
                            return Err(Diagnostic::new(
                                offset,
                                "Ranges require Int, Long, or Char operands",
                            ));
                        }
                    };
                    return Ok(Value {
                        ty: Type::Java(owner.into()),
                        kind: ValueKind::New {
                            owner: owner.into(),
                            descriptor: descriptor.into(),
                            args: vec![start, end],
                        },
                    });
                }
                if op.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && !matches!(op.as_str(), "and" | "or" | "xor" | "shl" | "shr" | "ushr")
                {
                    return self.check_infix_call(a, op, b, offset);
                }
                let source_left = a;
                let a = self.value(a)?;
                let refinements = self.refinements.clone();
                if op == "&&" || op == "||" {
                    self.refine(source_left, op == "&&");
                }
                let b = self.value(b)?;
                if op == "&&" || op == "||" {
                    self.refinements = refinements;
                }
                if matches!(op.as_str(), "==" | "!=" | "===" | "!==") {
                    return self.check_equality(op, a, b, offset);
                }
                if op == "+" && a.ty.base() == &Type::String {
                    if b.ty.base() == &Type::Unit {
                        return Err(Diagnostic::new(
                            offset,
                            "String concatenation with Unit operands is unsupported",
                        )
                        .with_kind(
                            crate::DiagnosticKind::UnsupportedLanguage,
                            "STRING_UNIT_CONCAT_UNSUPPORTED",
                        ));
                    }
                    return Ok(Value {
                        ty: Type::String,
                        kind: ValueKind::StringConcat(Box::new(a), Box::new(b)),
                    });
                }
                if a.ty == Type::Float
                    || b.ty == Type::Float
                    || a.ty == Type::Double
                    || b.ty == Type::Double
                {
                    return self.check_float_binary(op, a, b, offset);
                }
                if a.ty.is_nullable() || b.ty.is_nullable() {
                    return Err(Diagnostic::new(
                        offset,
                        "Nullable operands support only equality with null and Elvis",
                    ));
                }
                if matches!(a.ty, Type::Unit | Type::Float)
                    || matches!(b.ty, Type::Unit | Type::Float)
                    || matches!(
                        a.ty.base(),
                        Type::Java(_) | Type::ThreadLocal(_) | Type::List(_)
                    )
                    || matches!(
                        b.ty.base(),
                        Type::Java(_) | Type::ThreadLocal(_) | Type::List(_)
                    )
                    || matches!(a.ty, Type::Platform(_)) && a.ty.base() == &Type::String
                    || matches!(b.ty, Type::Platform(_)) && b.ty.base() == &Type::String
                {
                    return Err(Diagnostic::new(offset, "Unsupported value operand type"));
                }
                let ty = if op == "&&" || op == "||" {
                    self.require(&a, Type::Boolean, offset)?;
                    self.require(&b, Type::Boolean, offset)?;
                    Type::Boolean
                } else if matches!(op.as_str(), "<" | ">" | "<=" | ">=") {
                    if !matches!(a.ty, Type::Int | Type::Char | Type::Long) {
                        return Err(Diagnostic::new(
                            offset,
                            "Ordered comparisons require Int or Char",
                        ));
                    }
                    self.require(&b, a.ty.clone(), offset)?;
                    Type::Boolean
                } else {
                    if !matches!(a.ty, Type::Int | Type::Long) {
                        return Err(Diagnostic::new(offset, "Arithmetic requires Int or Long"));
                    }
                    let expected = if matches!(op.as_str(), "shl" | "shr" | "ushr") {
                        Type::Int
                    } else {
                        a.ty.clone()
                    };
                    self.require(&b, expected, offset)?;
                    a.ty.clone()
                };
                (ty, ValueKind::Binary(op.clone(), Box::new(a), Box::new(b)))
            }
            ExprKind::Property(receiver, name) => {
                if let Some(class) = self.qualified_classifier(receiver) {
                    if class.access & 0x4000 != 0
                        && class
                            .fields
                            .iter()
                            .any(|field| field.name == *name && field.access & 0x4009 == 0x4009)
                    {
                        return self.java_field(&class.name, name, None, offset);
                    }
                }
                if let Some(class) = self.qualified_classifier(expr) {
                    if class.fields.iter().any(|f| {
                        f.name == "INSTANCE" && f.descriptor == format!("L{};", class.name)
                    }) {
                        return Ok(Value {
                            ty: Type::Java(class.name.clone()),
                            kind: ValueKind::ObjectInstance(class.name),
                        });
                    }
                }
                if let Some(owner) = self.qualified_java_class(receiver) {
                    return self.java_field(&owner, name, None, offset);
                }
                if let ExprKind::Name(primitive) = &receiver.kind {
                    if matches!(primitive.as_str(), "Float" | "Double")
                        && self.unshadowed(primitive)
                    {
                        return self.java_field(
                            &format!("kotlin/jvm/internal/{primitive}CompanionObject"),
                            name,
                            None,
                            offset,
                        );
                    }
                }
                if matches!(&receiver.kind, ExprKind::Name(n) if n == "Long")
                    && self.unshadowed("Long")
                {
                    let value = match name.as_str() {
                        "MIN_VALUE" => i64::MIN,
                        "MAX_VALUE" => i64::MAX,
                        _ => return Err(Diagnostic::new(offset, "Unsupported Long member")),
                    };
                    (Type::Long, ValueKind::Long(value))
                } else if matches!(&receiver.kind, ExprKind::Name(n) if n == "Int")
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
                    if let ExprKind::Name(alias) = &receiver.kind {
                        if self.local(alias).is_none()
                            && !self.fields.contains_key(alias)
                            && !self.object_fields.contains_key(alias)
                            && !self.constants.contains_key(alias)
                        {
                            if let Some(class) = self.value_classifier(receiver)? {
                                let owner = &class.name;
                                {
                                    if class.fields.iter().any(|f| {
                                        f.name == "INSTANCE" && f.descriptor == format!("L{owner};")
                                    }) {
                                        if let Some(method) = class
                                            .methods
                                            .iter()
                                            .find(|m| {
                                                m.source_property.as_deref() == Some(name)
                                                    && m.kotlin_params
                                                        .as_ref()
                                                        .is_some_and(Vec::is_empty)
                                            })
                                            .cloned()
                                        {
                                            return self.member_call(
                                                method,
                                                Value {
                                                    ty: Type::Java(owner.clone()),
                                                    kind: ValueKind::ObjectInstance(owner.clone()),
                                                },
                                                &[],
                                                offset,
                                            );
                                        }
                                    }
                                    if let Some(method) = class
                                        .methods
                                        .iter()
                                        .find(|method| {
                                            method.access & 8 != 0
                                                && method.source_property.as_deref() == Some(name)
                                                && method
                                                    .kotlin_params
                                                    .as_ref()
                                                    .is_some_and(Vec::is_empty)
                                        })
                                        .cloned()
                                    {
                                        return self.library_static_call(method, &[], offset);
                                    }
                                    if let Some(field) =
                                        class.fields.iter().find(|f| f.name == "Companion")
                                    {
                                        let companion = field
                                            .descriptor
                                            .trim_start_matches('L')
                                            .trim_end_matches(';')
                                            .to_owned();
                                        let class = self.classpath.load(&companion)?;
                                        if let Some(method) = class
                                            .methods
                                            .iter()
                                            .find(|m| {
                                                m.source_property.as_deref() == Some(name)
                                                    && m.kotlin_params
                                                        .as_ref()
                                                        .is_some_and(Vec::is_empty)
                                            })
                                            .cloned()
                                        {
                                            return self.member_call(
                                                method,
                                                Value {
                                                    ty: Type::Java(companion),
                                                    kind: ValueKind::StaticField {
                                                        owner: owner.clone(),
                                                        name: "Companion".into(),
                                                    },
                                                },
                                                &[],
                                                offset,
                                            );
                                        }
                                    }
                                    return self.java_field(owner, name, None, offset);
                                }
                            }
                        }
                    }
                    let receiver = self.value(receiver)?;
                    if receiver.ty.is_nullable() {
                        return Err(Diagnostic::new(
                            offset,
                            "Nullable receivers require a safe call or not-null assertion",
                        ));
                    }
                    if matches!(receiver.ty.base(), Type::List(_) | Type::MutableList(_)) {
                        if unresolved_list(&receiver.ty) {
                            return Err(Diagnostic::new(
                                offset,
                                "Cannot infer list element type; empty or null-only factories require an explicit or expected element type",
                            ));
                        }
                        if name == "size" {
                            return Ok(Value {
                                ty: Type::Int,
                                kind: ValueKind::ListSize(Box::new(receiver)),
                            });
                        }
                    }
                    if self
                        .receiver
                        .as_ref()
                        .is_some_and(|this| this.ty == receiver.ty)
                    {
                        if let Some(method) = self.members.get(&format!("#get:{name}")) {
                            return self.member_call(method.clone(), receiver, &[], offset);
                        }
                    }
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
                        if let Some(method) = self.members.values().find(|method| {
                            method.source_property.as_deref() == Some(name)
                                && method.source_name() == getter_name(name)
                        }) {
                            return self.member_call(method.clone(), receiver, &[], offset);
                        }
                    }
                    if let Some(owner) = library::type_owner(&receiver.ty).map(str::to_owned) {
                        let loaded = self.classpath.load(&owner).ok();
                        if let Some(class) = self.classes.get(&owner).or(loaded.as_ref()) {
                            if let Some(method) = self
                                .classpath
                                .methods(&owner)
                                .unwrap_or_else(|_| class.methods.clone())
                                .into_iter()
                                .find(|m| {
                                    m.source_property.as_deref() == Some(name)
                                        && m.kotlin_params.as_ref().is_some_and(Vec::is_empty)
                                })
                            {
                                return self.member_call(method, receiver, &[], offset);
                            }
                            if self
                                .classpath
                                .field_candidates(&owner, name)?
                                .iter()
                                .any(|field| field.access & 1 != 0)
                            {
                                return self.java_field(
                                    &owner.clone(),
                                    name,
                                    Some(receiver),
                                    offset,
                                );
                            }
                        }
                    }
                    if let Some(method) = self.java_property_getter(&receiver.ty, name, offset)? {
                        return self.member_call(method, receiver, &[], offset);
                    }
                    if let Some(value) = enums::property(self, &receiver, name, offset)? {
                        return Ok(value);
                    }
                    if receiver.ty == Type::Char && name == "code" {
                        return Ok(Value {
                            ty: Type::Int,
                            kind: ValueKind::Convert(Type::Int, Box::new(receiver)),
                        });
                    }
                    if receiver.ty.base() == &Type::String && name == "length" {
                        return Ok(Value {
                            ty: Type::Int,
                            kind: ValueKind::StringCall(StringMethod::Length, vec![receiver]),
                        });
                    }
                    if name == "size"
                        && matches!(
                            receiver.ty.base(),
                            Type::IntArray
                                | Type::ByteArray
                                | Type::CharArray
                                | Type::LongArray
                                | Type::BooleanArray
                                | Type::FloatArray
                                | Type::DoubleArray
                                | Type::StringArray
                                | Type::Array(_)
                        )
                    {
                        return Ok(Value {
                            ty: Type::Int,
                            kind: ValueKind::Length(Box::new(receiver)),
                        });
                    }
                    if !self.extension_candidates(name, true)?.is_empty() {
                        return self.resolved_extension_call(
                            receiver,
                            name,
                            &[],
                            &[],
                            operations::ExtensionKind::Property,
                            offset,
                        );
                    }
                    if self.callable_name_was_renamed(name) {
                        return Err(Diagnostic::new(
                            offset,
                            format!("Unresolved property: {name}"),
                        ));
                    }
                    if let Some(owner) = library::type_owner(&receiver.ty) {
                        if self.classpath.load(owner).is_ok_and(|class| {
                            class.methods.iter().any(|method| {
                                method.name == "<init>"
                                    && method.kotlin_params.as_ref().is_some_and(|params| {
                                        params.iter().any(|(parameter, _, _)| parameter == name)
                                    })
                            })
                        }) {
                            return Err(Diagnostic::new(offset, format!("Constructor parameter {name} is not a property and is unavailable outside initialization")).with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_PROPERTY_CONSTRUCTOR_PARAMETER"));
                        }
                    }
                    return Err(Diagnostic::new(
                        offset,
                        format!("Unsupported property: {name}"),
                    ));
                }
            }
            ExprKind::Index(array, index) => {
                return self.check_indices(array, std::slice::from_ref(index), offset);
            }
            ExprKind::Indices(array, indices) => return self.check_indices(array, indices, offset),
            ExprKind::Call(target, args) => match &target.kind {
                ExprKind::Generic(target, types) => {
                    if let ExprKind::Name(name) = &target.kind {
                        if let Some(value) = self.lexical_call(name, args, types, None, offset)? {
                            return Ok(value);
                        }
                    }
                    if let ExprKind::Name(name) = &target.kind {
                        if name == "Array" && self.unshadowed(name) {
                            return self.array_constructor(types, args, offset);
                        }
                        if matches!(name.as_str(), "emptyList" | "listOf" | "mutableListOf")
                            && self.unshadowed(name)
                        {
                            return self.factory(name, types, args, offset);
                        }
                    }
                    if let ExprKind::Name(name) = &target.kind {
                        if self.function_names.iter().any(|n| n == name)
                            || self.classpath.load(&self.lambda_owner).is_ok_and(|class| {
                                class
                                    .methods
                                    .iter()
                                    .any(|method| method.source_name() == name)
                            })
                        {
                            return self.source_call(name, args, types, offset);
                        }
                    }
                    if let ExprKind::Name(name) = &target.kind {
                        if let Some(value) = self.generic_constructor(name, types, args, offset)? {
                            return Ok(value);
                        }
                    }
                    let methods = match &target.kind {
                        ExprKind::Name(name) => self.library_call_candidates(name),
                        ExprKind::Property(receiver, name) => {
                            if let ExprKind::Name(alias) = &receiver.kind {
                                if let Some(owner) = self.imports.get(alias) {
                                    if self.classes.contains_key(owner) {
                                        let methods = self
                                            .classpath
                                            .methods(owner)?
                                            .into_iter()
                                            .filter(|m| {
                                                m.source_name() == name && m.access & 9 == 9
                                            })
                                            .collect::<Vec<_>>();
                                        let method = self
                                            .select_method_types(&methods, args, types, offset)?;
                                        return self.library_static_call(method, args, offset);
                                    }
                                }
                            }
                            let value = self.value(receiver)?;
                            return self.extension_call(value, name, args, types, offset);
                        }
                        _ => Vec::new(),
                    };
                    let method = self.select_method_types(&methods, args, types, offset)?;
                    return self.library_static_call(method, args, offset);
                }
                ExprKind::SafeProperty(receiver, name) => {
                    return self.safe_access(receiver, name, Some(args), offset);
                }
                ExprKind::Name(name) => {
                    if name == "Array" && self.unshadowed(name) {
                        return self.array_constructor(&[], args, offset);
                    }
                    if self.local(name).is_some() || self.captured.contains_key(name) {
                        let function = self.value(target)?;
                        return self.function_invoke(function, args, offset);
                    }
                    if let Some(value) = self.lexical_call(name, args, &[], None, offset)? {
                        return Ok(value);
                    }
                    let imported = self
                        .library_call_candidates(name)
                        .into_iter()
                        .filter(|m| {
                            m.source_property.is_none()
                                && m.library.as_ref().is_none_or(|l| l.receiver.is_none())
                        })
                        .collect::<Vec<_>>();
                    if !imported.is_empty() {
                        let has_arity = imported.iter().any(|method| {
                            method.kotlin_params.as_ref().is_none_or(|params| {
                                params.iter().filter(|(_, _, default)| !default).count()
                                    <= args.len()
                                    && (method.access & 0x80 != 0 || args.len() <= params.len())
                            })
                        });
                        if !has_arity {
                            if let Some(value) = self.default_constructor(name, args, offset)? {
                                return Ok(value);
                            }
                        }
                        let method = self.select_method(&imported, args, offset)?;
                        return self.library_static_call(method, args, offset);
                    }
                    if let Some((owner, params, defaults)) = self.constructor {
                        if self.imports.get(name).is_some_and(|n| n == owner) {
                            return self.constructor_call(
                                owner,
                                params,
                                &defaults.iter().map(Option::is_some).collect::<Vec<_>>(),
                                args,
                                true,
                                offset,
                            );
                        }
                    }
                    if let Some(owner) = self.imports.get(name) {
                        if let Some(class) = self.classes.get(owner) {
                            if class.access & 0x2000 != 0 {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Runtime annotation instance construction is unsupported",
                                )
                                .with_kind(
                                    crate::DiagnosticKind::UnsupportedLanguage,
                                    "ANNOTATION_INSTANCE_CONSTRUCTION_UNSUPPORTED",
                                ));
                            }
                            if class.access & (0x200 | 0x400) != 0 && !class.kotlin_fun_interface {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Abstract classes and interfaces cannot be constructed",
                                )
                                .with_kind(
                                    crate::DiagnosticKind::InvalidKotlin,
                                    "ABSTRACT_CONSTRUCTION_INVALID",
                                ));
                            }
                            if class.access & 0x4000 != 0 {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Enum constructors can only be called by enum entry initialization",
                                ));
                            }
                            if class.kotlin_fun_interface {
                                if args.len() != 1 {
                                    return Err(Diagnostic::new(
                                        offset,
                                        "Functional interface construction requires one function argument",
                                    ));
                                }
                                return self.expected_value(&args[0], &Type::Java(owner.clone()));
                            }
                            if class
                                .library
                                .as_ref()
                                .is_some_and(|l| !l.type_parameters.is_empty())
                            {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Source generic construction requires explicit type arguments",
                                )
                                .with_kind(
                                    crate::DiagnosticKind::UnsupportedLanguage,
                                    "GENERIC_CONSTRUCTOR_INFERENCE_UNSUPPORTED",
                                ));
                            }
                            let constructors: Vec<_> = class
                                .methods
                                .iter()
                                .filter(|m| {
                                    m.name == "<init>"
                                        && m.access & 1 != 0
                                        && !m.unsupported
                                        && (!class.kotlin_class || m.kotlin_params.is_some())
                                })
                                .cloned()
                                .collect();
                            let method = self.select_method(&constructors, args, offset)?;
                            let normalized = self.vararg_arguments(&method, args, offset)?;
                            let args = normalized.as_slice();
                            let (types, _) = class_method_types(&method)?;
                            let params = method.kotlin_params.clone().unwrap_or_else(|| {
                                types
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, t)| (format!("p{i}"), java_parameter(t), false))
                                    .collect()
                            });
                            return self.constructor_call(
                                owner,
                                &params
                                    .iter()
                                    .map(|p| (p.0.clone(), p.1.clone()))
                                    .collect::<Vec<_>>(),
                                &params.iter().map(|p| p.2).collect::<Vec<_>>(),
                                args,
                                method.kotlin_params.is_some(),
                                offset,
                            );
                        }
                    }
                    if matches!(name.as_str(), "emptyList" | "listOf" | "mutableListOf")
                        && self.unshadowed(name)
                    {
                        return self.factory(name, &[], args, offset);
                    }
                    if self.local(name).is_some() {
                        return Err(Diagnostic::new(
                            offset,
                            "Invoking local values is not supported",
                        ));
                    }
                    if self.object_methods.contains_key(name) {
                        return self.object_call(name, args, None, offset);
                    }
                    if self.members.values().any(|m| m.source_name() == name) {
                        let candidates = self
                            .members
                            .values()
                            .filter(|m| m.source_name() == name)
                            .cloned()
                            .collect::<Vec<_>>();
                        let method = self.select_method(&candidates, args, offset)?;
                        if method.source_property.is_some() {
                            return Err(Diagnostic::new(
                                offset,
                                "Kotlin properties cannot be invoked as functions",
                            ));
                        }
                        let receiver = self.receiver_value(offset)?;
                        return self.member_call(method, receiver, args, offset);
                    }
                    if self.receiver.is_some() && !self.functions.contains_key(name) {
                        if let Ok(call) = self.implicit_receiver_call(name, args, offset) {
                            return Ok(call);
                        }
                    }
                    if name == "ShortArray" && self.unshadowed(name) {
                        return Err(Diagnostic::new(
                            offset,
                            "ShortArray semantics are unsupported",
                        )
                        .with_kind(
                            crate::DiagnosticKind::UnsupportedLanguage,
                            "SHORT_UNSUPPORTED",
                        ));
                    }
                    if let Some(array) =
                        Type::primitive_array_named(name).filter(|_| self.unshadowed(name))
                    {
                        if args.len() == 2 {
                            return self.primitive_array_constructor(array, args, offset);
                        }
                        if args.len() != 1 {
                            return Err(Diagnostic::new(
                                offset,
                                "Array allocation requires size and optional initializer",
                            ));
                        }
                        let size = self.expected_value(&args[0], &Type::Int)?;
                        return Ok(Value {
                            ty: array.clone(),
                            kind: if array == Type::IntArray {
                                ValueKind::NewIntArray(Box::new(size))
                            } else {
                                ValueKind::NewArray(array, Box::new(size))
                            },
                        });
                    }
                    if let Some(value) = self.default_constructor(name, args, offset)? {
                        return Ok(value);
                    }
                    if !self.functions.contains_key(name)
                        && (self.fields.contains_key(name)
                            || self.object_fields.contains_key(name)
                            || self.companion_fields.contains_key(name)
                            || self
                                .library_candidates(name)
                                .iter()
                                .any(|method| method.source_property.as_deref() == Some(name)))
                    {
                        let function = self.value(target)?;
                        return self.function_invoke(function, args, offset);
                    }
                    if self.receiver.is_some() && !self.functions.contains_key(name) {
                        if let Ok(call) = self.implicit_receiver_call(name, args, offset) {
                            return Ok(call);
                        }
                    }
                    return self.source_call(name, args, &[], offset);
                }
                ExprKind::Property(receiver, name) => {
                    if let Some(owner) = self.qualified_java_class(receiver) {
                        return self.java_static(&owner, name, args, offset);
                    }
                    if self
                        .local(name)
                        .is_some_and(|l| matches!(l.ty, Type::ReceiverFunction { .. }))
                        || self
                            .captured
                            .get(name)
                            .is_some_and(|(_, ty)| matches!(ty, Type::ReceiverFunction { .. }))
                    {
                        return self.receiver_function_call(receiver, name, args, offset);
                    }
                    if matches!(&receiver.kind,ExprKind::Name(n) if n=="super") {
                        let this = self.receiver_value(offset)?;
                        let owner = library::type_owner(&this.ty).ok_or_else(|| {
                            Diagnostic::new(offset, "super requires a class receiver")
                        })?;
                        let class = self.classpath.load(owner)?;
                        let parent = class
                            .super_name
                            .ok_or_else(|| Diagnostic::new(offset, "Class has no superclass"))?;
                        let methods = self
                            .classpath
                            .methods(&parent)?
                            .into_iter()
                            .filter(|m| {
                                m.source_name() == name && m.access & 8 == 0 && m.access & 5 != 0
                            })
                            .collect::<Vec<_>>();
                        let method = self.select_method(&methods, args, offset)?;
                        let call = self.member_call(method.clone(), this, args, offset)?;
                        return match call.kind {
                            ValueKind::JavaCall { args, .. }
                            | ValueKind::MemberCall { args, .. } => Ok(Value {
                                ty: call.ty,
                                kind: ValueKind::SuperCall { method, args },
                            }),
                            _ => Err(Diagnostic::new(
                                offset,
                                "Default super calls are unsupported",
                            )),
                        };
                    }
                    if let ExprKind::Name(alias) = &receiver.kind {
                        if self.unshadowed(alias) {
                            let owner = format!("java/lang/{alias}");
                            if self.classpath.load(&owner).is_ok_and(|c| c.access & 1 != 0) {
                                return self.java_static(&owner, name, args, offset);
                            }
                        }
                    }
                    if let ExprKind::Name(alias) = &receiver.kind {
                        if self.local(alias).is_none()
                            && !self.fields.contains_key(alias)
                            && !self.object_fields.contains_key(alias)
                            && !self.constants.contains_key(alias)
                        {
                            if let Some(owner) = self.imports.get(alias) {
                                if let Some(class) = self.classes.get(owner) {
                                    if class.fields.iter().any(|f| {
                                        f.name == "INSTANCE" && f.descriptor == format!("L{owner};")
                                    }) {
                                        let methods = self
                                            .classpath
                                            .methods(owner)?
                                            .into_iter()
                                            .filter(|m| {
                                                m.source_name() == name && m.access & 1 != 0
                                            })
                                            .collect::<Vec<_>>();
                                        let method = self.select_method(&methods, args, offset)?;
                                        return if method.access & 8 != 0 {
                                            self.library_static_call(method, args, offset)
                                        } else {
                                            self.member_call(
                                                method,
                                                Value {
                                                    ty: Type::Java(owner.clone()),
                                                    kind: ValueKind::ObjectInstance(owner.clone()),
                                                },
                                                args,
                                                offset,
                                            )
                                        };
                                    }
                                    if let Some(field) =
                                        class.fields.iter().find(|f| f.name == "Companion")
                                    {
                                        let companion = field
                                            .descriptor
                                            .trim_start_matches('L')
                                            .trim_end_matches(';')
                                            .to_owned();
                                        return self.java_call(
                                            Value {
                                                ty: Type::Java(companion),
                                                kind: ValueKind::StaticField {
                                                    owner: owner.clone(),
                                                    name: "Companion".into(),
                                                },
                                            },
                                            name,
                                            args,
                                            offset,
                                        );
                                    }
                                    return self.java_static(owner, name, args, offset);
                                }
                            }
                        }
                    }
                    let receiver = self.value(receiver)?;
                    if name == "invoke"
                        && matches!(
                            receiver.ty,
                            Type::Function { .. } | Type::ReceiverFunction { .. }
                        )
                    {
                        return self.function_invoke(receiver, args, offset);
                    }
                    if receiver.ty.is_nullable() {
                        return Err(Diagnostic::new(
                            offset,
                            "Nullable receivers require a safe call or not-null assertion",
                        ));
                    }
                    if self
                        .object_owner
                        .is_some_and(|owner| receiver.ty == Type::Java(owner.into()))
                        && self.object_methods.contains_key(name)
                    {
                        return self.object_call(name, args, Some(receiver), offset);
                    }
                    if matches!(receiver.ty.base(), Type::List(_) | Type::MutableList(_)) {
                        return self.list_call(receiver, name, args, offset);
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
                    if matches!(
                        receiver.ty,
                        Type::Int
                            | Type::Byte
                            | Type::Short
                            | Type::Char
                            | Type::Long
                            | Type::Float
                            | Type::Double
                    ) && args.is_empty()
                        && matches!(
                            name.as_str(),
                            "toInt"
                                | "toByte"
                                | "toShort"
                                | "toChar"
                                | "toLong"
                                | "toFloat"
                                | "toDouble"
                        )
                    {
                        if matches!(receiver.ty, Type::Float | Type::Double) && name == "toShort" {
                            return Err(Diagnostic::new(offset, "Direct floating-point conversions to Short are deprecated; convert to Int first").with_kind(crate::DiagnosticKind::InvalidKotlin, "DEPRECATED_FLOATING_SHORT_CONVERSION"));
                        }
                        if matches!(receiver.ty, Type::Float | Type::Double | Type::Long)
                            && name == "toChar"
                        {
                            return Err(Diagnostic::new(
                                offset,
                                "Direct Long, Float, and Double conversions to Char are deprecated; convert to Int first",
                            ));
                        }
                        let target = match name.as_str() {
                            "toByte" => Type::Byte,
                            "toShort" => Type::Short,
                            "toChar" => Type::Char,
                            "toLong" => Type::Long,
                            "toFloat" => Type::Float,
                            "toDouble" => Type::Double,
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
                        && self.members.values().any(|m| m.source_name() == name)
                    {
                        let candidates = self
                            .members
                            .values()
                            .filter(|m| m.source_name() == name)
                            .cloned()
                            .collect::<Vec<_>>();
                        let selected = self.select_method(&candidates, args, offset);
                        if let Err(error) = &selected {
                            if error.message.contains("Ambiguous") {
                                return Err(error.clone());
                            }
                        }
                        if let Ok(method) = selected {
                            if method.source_property.is_some() {
                                return Err(Diagnostic::new(
                                    offset,
                                    "Kotlin properties cannot be invoked as functions",
                                ));
                            }
                            return self.member_call(method, receiver, args, offset);
                        }
                    }
                    if matches!(
                        receiver.ty,
                        Type::Int | Type::Long | Type::Float | Type::Double
                    ) && name == "toString"
                        && args.is_empty()
                    {
                        return Ok(Value {
                            ty: Type::String,
                            kind: ValueKind::IntToString(Box::new(receiver)),
                        });
                    }
                    if library::type_owner(&receiver.ty).is_some()
                        && receiver.ty.base() != &Type::String
                    {
                        return self.java_call(receiver, name, args, offset);
                    }
                    if self
                        .function_names
                        .iter()
                        .enumerate()
                        .any(|(id, candidate)| candidate == name && self.extensions.contains(&id))
                    {
                        return self.source_extension_call(
                            receiver,
                            name,
                            args,
                            &[],
                            false,
                            offset,
                        );
                    }
                    if self
                        .extension_candidates(name, false)?
                        .iter()
                        .any(|m| m.library.as_ref().is_some_and(|l| l.receiver.is_some()))
                    {
                        return self.extension_call(receiver, name, args, &[], offset);
                    }
                    if library::type_owner(&receiver.ty).is_some()
                        && receiver.ty.base() != &Type::String
                    {
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
                        if receiver.ty != Type::String
                            && library::type_owner(&receiver.ty).is_none()
                        {
                            return self.resolved_extension_call(
                                receiver,
                                name,
                                args,
                                &[],
                                operations::ExtensionKind::Ordinary,
                                offset,
                            );
                        }
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
        Type::Parameter { name, upper_bound } => Ok(Type::Parameter {
            name: name.clone(),
            upper_bound: Box::new(resolve_type(upper_bound, imports, offset)?),
        }),
        Type::Array(element) => Ok(Type::Array(Box::new(resolve_type(
            element, imports, offset,
        )?))),
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => Ok(Type::ReceiverFunction {
            receiver: Box::new(resolve_type(receiver, imports, offset)?),
            params: params
                .iter()
                .map(|p| resolve_type(p, imports, offset))
                .collect::<Result<Vec<_>>>()?,
            result: Box::new(resolve_type(result, imports, offset)?),
        }),
        Type::Function { params, result } => Ok(Type::Function {
            params: params
                .iter()
                .map(|p| resolve_type(p, imports, offset))
                .collect::<Result<Vec<_>>>()?,
            result: Box::new(resolve_type(result, imports, offset)?),
        }),
        Type::Parameterized { name, arguments } => {
            if name == "Array" {
                return Err(Diagnostic::new(offset, "Only Array<String> is supported"));
            }
            let resolved = resolve_type(&Type::Java(name.clone()), imports, offset)?;
            let Type::Java(name) = resolved else {
                unreachable!()
            };
            Ok(Type::Parameterized {
                name,
                arguments: arguments
                    .iter()
                    .map(|a| {
                        Ok(syntax::TypeArgument {
                            variance: a.variance,
                            ty: a
                                .ty
                                .as_ref()
                                .map(|t| resolve_type(t, imports, offset))
                                .transpose()?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            })
        }
        Type::Nullable(inner) => {
            let inner = resolve_type(inner, imports, offset)?;
            if inner == Type::Unit || inner.is_nullable() {
                return Err(Diagnostic::new(offset, "Unsupported nullable type"));
            }
            Ok(Type::Nullable(Box::new(inner)))
        }
        Type::MutableList(element) => Ok(Type::MutableList(Box::new(resolve_type(
            element, imports, offset,
        )?))),
        Type::List(element) => Ok(Type::List(Box::new(resolve_type(
            element, imports, offset,
        )?))),
        Type::ThreadLocal(element) => Ok(Type::Parameterized {
            name: "java/lang/ThreadLocal".into(),
            arguments: vec![syntax::TypeArgument {
                variance: syntax::Variance::Invariant,
                ty: Some(resolve_type(element, imports, offset)?),
            }],
        }),

        Type::Java(name)
            if matches!(
                name.as_str(),
                "List"
                    | "MutableList"
                    | "Collection"
                    | "MutableCollection"
                    | "Iterable"
                    | "Set"
                    | "MutableSet"
                    | "Map"
                    | "MutableMap"
            ) =>
        {
            Ok(Type::Java(
                match name.as_str() {
                    "List" | "MutableList" => "java/util/List",
                    "Collection" | "MutableCollection" => "java/util/Collection",
                    "Iterable" => "java/lang/Iterable",
                    "Set" | "MutableSet" => "java/util/Set",
                    _ => "java/util/Map",
                }
                .into(),
            ))
        }
        Type::Java(name) if imports.contains_key(name) => Ok(Type::Java(imports[name].clone())),
        Type::Java(name)
            if name.split_once('/').is_some_and(|(root, _)| {
                imports.get(root).is_some_and(|owner| !owner.contains("::"))
            }) =>
        {
            let (root, nested) = name.split_once('/').unwrap();
            Ok(Type::Java(format!(
                "{}${}",
                imports[root],
                nested.replace('/', "$")
            )))
        }
        Type::Java(name) if name.contains('/') || imports.values().any(|n| n == name) => {
            Ok(ty.clone())
        }
        Type::Java(name) => imports
            .get(name)
            .map(|n| Type::Java(n.clone()))
            .ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    format!("Type requires an explicit class import: {name}"),
                )
                .with_kind(crate::DiagnosticKind::MissingDependency, "CLASS_NOT_FOUND")
                .for_type(name)
            }),
        _ => Ok(ty.clone()),
    }
}
fn resolve_type_checked(
    ty: &Type,
    imports: &HashMap<String, String>,
    offset: usize,
    classpath: &Classpath,
    classes: &HashMap<String, JavaClass>,
) -> Result<Type> {
    let expanded = classpath.expand_type_alias(ty, imports, library::DEFAULT_PACKAGES)?;
    let enriched_imports = library::default_type_imports(&expanded, imports, classpath);
    let ty = resolve_type(&expanded, &enriched_imports, offset)?;
    relations::validate_type_arguments_with_classes(&ty, classpath, classes, offset)?;
    Ok(ty)
}
fn resolve_parameters(
    parameters: &[syntax::TypeParameter],
    imports: &HashMap<String, String>,
    classpath: &Classpath,
    offset: usize,
    classes: &HashMap<String, JavaClass>,
) -> Result<Vec<syntax::TypeParameter>> {
    parameters
        .iter()
        .map(|parameter| {
            Ok(syntax::TypeParameter {
                only_input_types: parameter.only_input_types,
                reified: parameter.reified,
                name: parameter.name.clone(),
                variance: parameter.variance,
                bounds: parameter
                    .bounds
                    .iter()
                    .map(|bound| resolve_type_checked(bound, imports, offset, classpath, classes))
                    .collect::<Result<Vec<_>>>()?,
            })
        })
        .collect()
}
fn loses_member_type(_ty: &Type) -> bool {
    false
}
fn call_names(expr: &Expr, names: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::AnnotationConstant { value, .. } => call_names(value, names),
        ExprKind::AnnotationArray(elements) | ExprKind::VarargArray { elements, .. } => {
            for element in elements {
                call_names(element, names);
            }
        }
        ExprKind::Block(body) => body_call_names(body, names),
        ExprKind::Try {
            body,
            catches,
            finally,
        } => {
            body_call_names(body, names);
            for catch in catches {
                body_call_names(&catch.body, names);
            }
            body_call_names(finally, names);
        }
        ExprKind::Labeled(_, value) => call_names(value, names),
        ExprKind::LabeledReturn(_, value) => {
            if let Some(value) = value {
                call_names(value, names);
            }
        }
        ExprKind::WhenBinding(_, value, branches, otherwise) => {
            call_names(value, names);
            for (conditions, body) in branches {
                for condition in conditions {
                    call_names(condition, names);
                }
                call_names(body, names);
            }
            call_names(otherwise, names);
        }
        ExprKind::Indices(target, indices) => {
            call_names(target, names);
            for index in indices {
                call_names(index, names);
            }
        }
        ExprKind::Name(_)
        | ExprKind::Int(_)
        | ExprKind::Long(_)
        | ExprKind::Float(_)
        | ExprKind::Double(_)
        | ExprKind::MissingWhenElse
        | ExprKind::WhenSubjectless
        | ExprKind::Boolean(_)
        | ExprKind::String(_)
        | ExprKind::Char(_)
        | ExprKind::Null => {}
        ExprKind::Call(target, args) => {
            let target = if let ExprKind::Generic(target, _) = &target.kind {
                target.as_ref()
            } else {
                target.as_ref()
            };
            if let ExprKind::Name(name) = &target.kind {
                names.insert(name.clone());
            }
            call_names(target, names);
            for arg in args {
                call_names(arg, names);
            }
        }
        ExprKind::Property(a, _) | ExprKind::SafeProperty(a, _) => {
            if let ExprKind::Name(name) = &a.kind {
                names.insert(name.clone());
            }
            call_names(a, names);
        }
        ExprKind::ClassLiteral { receiver: a, .. }
        | ExprKind::Generic(a, _)
        | ExprKind::NamedArgument(_, a)
        | ExprKind::Unary(_, a)
        | ExprKind::NonNull(a)
        | ExprKind::Lambda(a)
        | ExprKind::Throw(a)
        | ExprKind::Increment { value: a, .. }
        | ExprKind::Decrement { value: a, .. }
        | ExprKind::TypeOperation(_, a, _) => call_names(a, names),
        ExprKind::LambdaWithParams { body, .. } => body_call_names(body, names),
        ExprKind::CallableReference { receiver, .. } => {
            if let Some(receiver) = receiver {
                call_names(receiver, names);
            }
        }
        ExprKind::Return(a) => {
            if let Some(a) = a {
                call_names(a, names);
            }
        }
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            call_names(a, names);
            call_names(b, names);
        }
        ExprKind::If(a, b, c) => {
            call_names(a, names);
            call_names(b, names);
            call_names(c, names);
        }
        ExprKind::Template(parts) => {
            for part in parts {
                call_names(part, names);
            }
        }
        ExprKind::When(subject, branches, tail) => {
            call_names(subject, names);
            for (conditions, value) in branches {
                for condition in conditions {
                    call_names(condition, names);
                }
                call_names(value, names);
            }
            call_names(tail, names);
        }
    }
}
pub(crate) fn body_free_names(
    body: &[syntax::Statement],
    parameters: &[String],
) -> HashSet<String> {
    captures::free_names(body, parameters)
}
pub(crate) fn body_call_names(body: &[syntax::Statement], names: &mut HashSet<String>) {
    for statement in body {
        match &statement.kind {
            StatementKind::Block(body) => body_call_names(body, names),
            StatementKind::LocalFunction(function) => body_call_names(&function.body, names),
            StatementKind::Labeled(_, body) => {
                body_call_names(std::slice::from_ref(body.as_ref()), names)
            }
            StatementKind::BreakLabel(_) | StatementKind::ContinueLabel(_) => {}
            StatementKind::Destructure(_, _, value) => call_names(value, names),
            StatementKind::IndicesAssign(target, indices, value) => {
                call_names(target, names);
                for index in indices {
                    call_names(index, names);
                }
                call_names(value, names);
            }
            StatementKind::WhenBinding(_, value, branches, otherwise) => {
                call_names(value, names);
                for (conditions, body) in branches {
                    for condition in conditions {
                        call_names(condition, names);
                    }
                    body_call_names(body, names);
                }
                if let Some(body) = otherwise {
                    body_call_names(body, names);
                }
            }
            StatementKind::Let(_, _, _, e)
            | StatementKind::Assign(_, e)
            | StatementKind::AddAssign(_, e)
            | StatementKind::CompoundAssign(_, _, e)
            | StatementKind::Throw(e)
            | StatementKind::Expression(e) => call_names(e, names),
            StatementKind::Return(e) => {
                if let Some(e) = e {
                    call_names(e, names);
                }
            }
            StatementKind::PropertyAssign(a, _, b) | StatementKind::SafePropertyAssign(a, _, b) => {
                call_names(a, names);
                call_names(b, names);
            }
            StatementKind::IndexAssign(a, b, c) => {
                call_names(a, names);
                call_names(b, names);
                call_names(c, names);
            }
            StatementKind::If(c, a, b) => {
                call_names(c, names);
                body_call_names(a, names);
                body_call_names(b, names);
            }
            StatementKind::While(c, b)
            | StatementKind::DoWhile(c, b)
            | StatementKind::ForEach(_, c, b) => {
                call_names(c, names);
                body_call_names(b, names);
            }
            StatementKind::ForUntil(_, a, b, body) => {
                call_names(a, names);
                call_names(b, names);
                body_call_names(body, names);
            }
            StatementKind::When(e, branches, tail) => {
                call_names(e, names);
                for (conditions, body) in branches {
                    for c in conditions {
                        call_names(c, names);
                    }
                    body_call_names(body, names);
                }
                if let Some(tail) = tail {
                    body_call_names(tail, names);
                }
            }
            StatementKind::Declare(_, _, _) | StatementKind::Continue | StatementKind::Break => {}
        }
    }
}
fn has_platform(ty: &Type) -> bool {
    match ty {
        Type::Platform(_) => true,
        Type::Nullable(t) | Type::List(t) | Type::MutableList(t) => has_platform(t),
        _ => false,
    }
}
fn unresolved_list(ty: &Type) -> bool {
    match ty {
        Type::List(t) | Type::MutableList(t) => t.as_ref() == &Type::Null || unresolved_list(t),
        Type::Nullable(t) => unresolved_list(t),
        _ => false,
    }
}
#[cfg(test)]
pub fn check(source: &syntax::Module, classpath: &Classpath) -> Result<Module> {
    let name = source
        .class_decl
        .as_ref()
        .map(|d| &d.name)
        .or_else(|| source.object_decl.as_ref().map(|d| &d.name))
        .or_else(|| source.interface_decl.as_ref().map(|d| &d.name));
    let owner = name
        .map(|name| {
            if source.package.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", source.package.replace('.', "/"), name)
            }
        })
        .unwrap_or_else(|| "Korrust".into());
    if source.class_decl.is_some() {
        classpath.source_header(source, &owner)?;
    }
    check_with_owner(source, classpath, &owner)
}
pub fn check_with_owner(
    source: &syntax::Module,
    classpath: &Classpath,
    owner: &str,
) -> Result<Module> {
    classpath.register_value_scope(owner, source);
    annotation_placements::preflight_property_visibility(source, classpath)?;
    if source.object_decl.as_ref().is_some_and(|object| {
        !object.annotations.is_empty()
            || !object.interfaces.is_empty()
                && object.methods.is_empty()
                && object.fields.is_empty()
    }) {
        return singletons::check_with_owner(source, classpath, owner);
    }
    let renamed_imports = source
        .imports
        .iter()
        .filter_map(|(alias, path, _)| {
            let original = path.rsplit('/').next()?;
            (alias != "*"
                && alias != original
                && alias != original.rsplit('$').next().unwrap_or(original)
                && alias != &original.replace('$', "/")
                && !source.imports.iter().any(|(other_alias, other_path, _)| {
                    other_path == path && other_alias == original
                }))
            .then(|| path.clone())
        })
        .collect::<HashSet<_>>();
    let mut imports = HashMap::new();
    let mut classes = HashMap::new();
    for (alias, name, offset) in &source.imports {
        if let Some((package, member)) = name.rsplit_once('/') {
            if classpath.type_alias(package, member)?.is_some() {
                imports.insert(alias.clone(), name.clone());
                continue;
            }
        }
        if alias == "*" {
            let package = name.trim_end_matches("/*");
            let package_classes = classpath.package_classes(package)?;
            if package_classes.is_empty() {
                return Err(Diagnostic::new(
                    *offset,
                    "Unresolved wildcard import package",
                ));
            }
            for class in package_classes {
                if class.library.as_ref().is_some_and(|l| l.package.is_some()) {
                    for method in &class.methods {
                        if method.access & 9 == 9 {
                            let source_name = method
                                .source_property
                                .as_deref()
                                .or_else(|| method.library.as_ref().map(|l| l.source_name.as_str()))
                                .unwrap_or(&method.name);
                            if renamed_imports.contains(&format!("{package}/{source_name}")) {
                                continue;
                            }
                            imports
                                .entry(source_name.into())
                                .or_insert_with(|| format!("{}::{source_name}", class.name));
                        }
                    }
                } else {
                    if class.access & 1 == 0 || class.unsupported {
                        continue;
                    }
                    let simple = class.name.rsplit('/').next().unwrap().to_owned();
                    if renamed_imports.contains(&class.name) {
                        classes.insert(class.name.clone(), class);
                        continue;
                    }
                    imports.entry(simple).or_insert_with(|| class.name.clone());
                }
                classes.insert(class.name.clone(), class);
            }
            continue;
        }
        if classpath.load(name).is_err() {
            if !name.contains('/')
                && let Some(symbol) = classpath.value_symbol(owner, alias, *offset)?
            {
                let field = match symbol {
                    crate::classpath::ValueSymbol::Field(field) => Some((field.owner, field.name)),
                    crate::classpath::ValueSymbol::Property(methods) => {
                        methods.first().map(|method| {
                            (
                                method.owner.clone(),
                                method.source_property.clone().unwrap(),
                            )
                        })
                    }
                    crate::classpath::ValueSymbol::Classifier(_) => None,
                };
                if let Some((declaring, member)) = field {
                    imports.insert(alias.clone(), format!("{declaring}::{member}"));
                    classes.insert(declaring.clone(), classpath.load(&declaring)?);
                    continue;
                }
            }
            if let Some((parent, member)) = name.rsplit_once('/') {
                if let Some(class) =
                    library::static_import_owner(classpath, parent, member, *offset)?
                {
                    imports.insert(alias.clone(), format!("{parent}::{member}"));
                    classes.insert(parent.into(), class);
                    continue;
                }
            }
            if let Some((package, member)) = name.rsplit_once('/') {
                let mut found = false;
                for class in classpath.package_classes(package)? {
                    if class.methods.iter().any(|m| {
                        m.source_property.as_deref() == Some(member)
                            || m.library.as_ref().is_some_and(|l| l.source_name == member)
                    }) || class
                        .fields
                        .iter()
                        .any(|field| field.name == member && field.access & 8 != 0)
                    {
                        imports.insert(alias.clone(), format!("{}::{member}", class.name));
                        classes.insert(class.name.clone(), class);
                        found = true;
                    }
                }
                if found {
                    continue;
                }
            }
        }
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
                | "Long"
                | "List"
                | "Byte"
                | "ByteArray"
                | "ThreadLocal"
                | "Char"
                | "Float"
                | "Double"
                | "Boolean"
                | "String"
                | "IntArray"
                | "CharArray"
                | "StringArray"
                | "Array"
                | "Unit"
        ) && !(alias == "ThreadLocal" && name == "java/lang/ThreadLocal")
        {
            return Err(Diagnostic::new(
                *offset,
                "Imports shadowing builtin types are not supported",
            ));
        }
        if let Some(code) = annotations::unsupported_lowering(name) {
            return Err(Diagnostic::new(
                *offset,
                format!("Annotation {name} requires unsupported compiler JVM semantics"),
            )
            .with_kind(crate::DiagnosticKind::UnsupportedLanguage, code));
        }
        let class = classpath.load(name)?;
        let supported_jvm_marker = matches!(
            name.as_str(),
            "kotlin/jvm/JvmOverloads" | "kotlin/jvm/JvmStatic"
        );
        if class.access & 0x2000 != 0
            && class.annotation_definition.is_none()
            && !supported_jvm_marker
        {
            return Err(Diagnostic::new(
                *offset,
                format!("Unsupported annotation definition or required compiler plugin: {name}"),
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "ANNOTATION_DEFINITION_UNSUPPORTED",
            ));
        }
        if class.access & 1 == 0 || (class.unsupported && !supported_jvm_marker) {
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
    let mut referenced = HashSet::new();
    fn collect_type(ty: &Type, names: &mut HashSet<String>) {
        match ty {
            Type::Java(name) => {
                names.insert(name.clone());
            }
            Type::Nullable(inner)
            | Type::List(inner)
            | Type::MutableList(inner)
            | Type::Array(inner)
            | Type::ThreadLocal(inner) => collect_type(inner, names),
            Type::Parameterized { name, arguments } => {
                names.insert(name.clone());
                for argument in arguments {
                    if let Some(ty) = &argument.ty {
                        collect_type(ty, names);
                    }
                }
            }
            Type::Function { params, result } => {
                for ty in params {
                    collect_type(ty, names);
                }
                collect_type(result, names);
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => {
                collect_type(receiver, names);
                for ty in params {
                    collect_type(ty, names);
                }
                collect_type(result, names);
            }
            _ => {}
        }
    }
    for function in &source.functions {
        for (_, ty) in &function.params {
            collect_type(ty, &mut referenced);
        }
        collect_type(&function.result, &mut referenced);
    }
    for property in &source.properties {
        if !property.inferred {
            collect_type(&property.ty, &mut referenced);
        }
    }
    if let Some(class) = &source.class_decl {
        let class_owner = owner.to_owned();
        imports.insert(class.name.clone(), class_owner.clone());
        classes.insert(
            class_owner.clone(),
            classpath.load(&class_owner).unwrap_or_else(|_| JavaClass {
                sealed_: false,
                sealed_subclasses: Vec::new(),
                annotation_definition: None,
                kotlin_fun_interface: false,
                library: None,
                name: class_owner,
                access: 1,
                super_name: Some("java/lang/Object".into()),
                interfaces: Vec::new(),
                methods: Vec::new(),
                fields: Vec::new(),
                kotlin_class: true,
                unsupported: false,
                kotlin_interface: false,
            }),
        );
        if let Some((name, _)) = &class.super_class {
            referenced.insert(name.clone());
        }
        for interface in &class.interfaces {
            referenced.insert(interface.clone());
        }
        for (_, ty) in &class.params {
            collect_type(ty, &mut referenced);
        }
        for property in &class.properties {
            collect_type(&property.ty, &mut referenced);
        }
        for method in &class.methods {
            for (_, ty) in &method.function.params {
                collect_type(ty, &mut referenced);
            }
            collect_type(&method.function.result, &mut referenced);
        }
    }
    if let Some(interface) = &source.interface_decl {
        fn collect_interface(i: &syntax::InterfaceDecl, names: &mut HashSet<String>) {
            for name in &i.interfaces {
                names.insert(name.clone());
            }
            for p in &i.properties {
                collect_type(&p.ty, names);
            }
            for m in &i.methods {
                for (_, ty) in &m.function.params {
                    collect_type(ty, names);
                }
                collect_type(&m.function.result, names);
            }
            for nested in &i.nested {
                collect_interface(nested, names);
            }
        }
        collect_interface(interface, &mut referenced);
        let mut own = vec![interface];
        while let Some(i) = own.pop() {
            referenced.remove(&i.name);
            own.extend(i.nested.iter());
        }
    }
    let super_types = source
        .class_decl
        .as_ref()
        .map(|d| d.super_types.as_slice())
        .or_else(|| {
            source
                .interface_decl
                .as_ref()
                .map(|d| d.super_types.as_slice())
        })
        .unwrap_or(&[]);
    for ty in super_types {
        if !matches!(ty, Type::Parameterized { .. }) {
            continue;
        }
        let resolved = resolve_type_checked(ty, &imports, 0, classpath, &classes)?;
        let raw = match ty {
            Type::Java(n) | Type::Parameterized { name: n, .. } => n,
            _ => continue,
        };
        if let Some(owner) = library::type_owner(&resolved) {
            imports.insert(raw.clone(), owner.into());
            classes.insert(owner.into(), classpath.load(owner)?);
        }
    }
    let mut referenced = referenced.into_iter().collect::<Vec<_>>();
    referenced.sort_by_key(|name| {
        (
            source
                .type_locations
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map_or(usize::MAX, |(_, offset)| *offset),
            name.clone(),
        )
    });
    for name in referenced {
        let type_offset = source
            .type_locations
            .iter()
            .find(|(candidate, _)| candidate == &name)
            .map_or(0, |(_, offset)| *offset);
        let resolved_owner = resolve_type(&Type::Java(name.clone()), &imports, 0)
            .ok()
            .and_then(|ty| match ty {
                Type::Java(owner) if owner.contains('/') => Some(owner),
                _ => None,
            });
        if let std::collections::hash_map::Entry::Vacant(entry) = imports.entry(name.clone()) {
            let owner = resolved_owner.unwrap_or_else(|| {
                if source.package.is_empty() || name.contains('/') {
                    name.clone()
                } else {
                    format!("{}/{}", source.package.replace('.', "/"), name)
                }
            });
            if !name.contains('/') && renamed_imports.contains(&owner) {
                return Err(Diagnostic::new(
                    type_offset,
                    format!("Unresolved aliased type: {name}"),
                ));
            }
            let class = classpath.load(&owner).map_err(|mut error| {
                error.offset = type_offset;
                error
            })?;
            if class.access & 1 == 0 || class.unsupported {
                return Err(Diagnostic::new(
                    type_offset,
                    "Same-package class is inaccessible or unsupported",
                ));
            }
            entry.insert(owner.clone());
            classes.insert(owner, class);
        }
    }
    let mut called = HashSet::new();
    for function in &source.functions {
        body_call_names(&function.body, &mut called);
        for expr in function.defaults.iter().flatten() {
            call_names(expr, &mut called);
        }
    }
    if let Some(class) = &source.class_decl {
        for method in &class.methods {
            body_call_names(&method.function.body, &mut called);
            for expr in method.function.defaults.iter().flatten() {
                call_names(expr, &mut called);
            }
        }
        for property in &class.properties {
            body_call_names(&property.body, &mut called);
            if let Some(e) = &property.initializer {
                call_names(e, &mut called);
            }
        }
        for expr in class.defaults.iter().flatten() {
            call_names(expr, &mut called);
        }
        if let Some(companion) = &class.companion {
            for property in &companion.properties {
                if let Some(expr) = &property.initializer {
                    call_names(expr, &mut called);
                }
            }
        }
    }
    for alias in called {
        if imports.contains_key(&alias)
            || source.functions.iter().any(|f| f.name == alias)
            || matches!(
                alias.as_str(),
                "String"
                    | "IntArray"
                    | "ByteArray"
                    | "CharArray"
                    | "Character"
                    | "emptyList"
                    | "listOf"
                    | "mutableListOf"
                    | "IllegalArgumentException"
                    | "IllegalStateException"
                    | "IndexOutOfBoundsException"
                    | "UnsupportedOperationException"
            )
        {
            continue;
        }
        let owner = if source.package.is_empty() {
            alias.clone()
        } else {
            format!("{}/{}", source.package.replace('.', "/"), alias)
        };
        if renamed_imports.contains(&owner) {
            continue;
        }
        if let Ok(class) = classpath.load(&owner) {
            if class.access & 1 == 0 || class.unsupported {
                return Err(Diagnostic::new(
                    0,
                    "Same-package class is inaccessible or unsupported",
                ));
            }
            imports.insert(alias, owner.clone());
            classes.insert(owner, class);
        }
    }
    let mut constants = HashMap::new();
    for constant in &source.constants {
        let (ty, kind) = constant_expr(&constant.value)?;
        if constants
            .insert(
                constant.name.clone(),
                (
                    ty,
                    Expr {
                        offset: constant.offset,
                        kind,
                    },
                ),
            )
            .is_some()
        {
            return Err(Diagnostic::new(
                constant.offset,
                "Conflicting constant declaration",
            ));
        }
    }
    if let Some(object) = &source.object_decl {
        if !object.methods.is_empty() || !object.fields.is_empty() {
            return objects::check(
                object,
                &source.package,
                classpath,
                &imports,
                &classes,
                Some(owner),
            );
        }
    }
    if let Some(interface) = &source.interface_decl {
        return interfaces::check(
            interface,
            &source.package,
            classpath,
            imports,
            classes,
            Some(owner),
        );
    }
    let mut functions = HashMap::new();
    let mut signatures = Vec::new();
    let mut extensions = HashSet::new();
    for (i, function) in source.functions.iter().enumerate() {
        declarations::validate_operator(function, false)?;
        for parameter in &function.type_params {
            for bound in &parameter.bounds {
                resolve_type_checked(bound, &imports, function.offset, classpath, &classes)?;
            }
        }
        if function.infix && function.receiver.is_none() {
            return Err(Diagnostic::new(
                function.offset,
                "Infix functions require a receiver",
            ));
        }
        if function.receiver.is_some() && function.defaults.iter().any(Option::is_some) {
            return Err(Diagnostic::new(
                function.offset,
                "Default arguments on extension functions are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "SOURCE_EXTENSION_DEFAULTS_UNSUPPORTED",
            ));
        }
        functions.entry(function.name.clone()).or_insert(i);
        let mut params = Vec::new();
        if let Some(receiver) = &function.receiver {
            extensions.insert(i);
            params.push((
                format!("$this${}", function.name),
                resolve_type_checked(receiver, &imports, function.offset, classpath, &classes)?,
            ));
        }
        for (name, ty) in &function.params {
            params.push((
                name.clone(),
                resolve_type_checked(ty, &imports, function.offset, classpath, &classes)?,
            ));
        }
        if params.iter().any(|p| p.1 == Type::Unit) {
            return Err(Diagnostic::new(
                function.offset,
                "Unit parameters are not supported",
            ));
        }
        if params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() > 255 {
            return Err(Diagnostic::new(
                function.offset,
                "JVM methods support at most 255 parameter slots",
            ));
        }
        signatures.push((
            params,
            resolve_type_checked(
                &function.result,
                &imports,
                function.offset,
                classpath,
                &classes,
            )?,
        ));
    }
    if signatures.len() > 65535 {
        return Err(Diagnostic::new(0, "Too many functions for a JVM class"));
    }
    let lambdas = Rc::new(RefCell::new(Vec::new()));
    if let Some(decl) = &source.class_decl {
        let class_decl = declarations::check_class_with_constants(
            decl,
            &source.package,
            classpath,
            &imports,
            &classes,
            &HashMap::new(),
            Some(owner),
            &lambdas,
            &constants,
        )?;
        return Ok(Module {
            exposed_fields: HashMap::new(),
            properties: Vec::new(),
            initializers: Vec::new(),
            locals: 0,
            lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
            constants: Vec::new(),
            functions: Vec::new(),
            interface_decl: None,
            object_decl: None,
            class_decl: Some(class_decl),
        });
    }
    let static_properties = source
        .properties
        .iter()
        .map(|p| syntax::ClassCompanionProperty {
            annotations: p.annotations.clone(),
            name: p.name.clone(),
            offset: p.offset,
            private: p.private,
            internal: p.internal,
            getter_visibility: p.getter_visibility,
            setter_visibility: p.setter_visibility,
            mutable: p.mutable,
            ty: (!p.inferred).then(|| p.ty.clone()),
            initializer: p.initializer.clone(),
            body: p.body.clone(),
            setter: p.setter.clone(),
            delegate: p.delegate.clone(),
            lateinit: p.lateinit,
        })
        .collect::<Vec<_>>();
    let mut static_state = declarations::check_static_state(
        owner,
        owner,
        &static_properties,
        &[],
        &[],
        false,
        classpath,
        &imports,
        &classes,
        &lambdas,
    )?;
    let mut static_fields = HashMap::new();
    let mut static_mutable = HashSet::new();
    for property in &static_state.properties {
        static_fields.insert(
            property.name.clone(),
            (owner.to_owned(), property.ty.clone()),
        );
        if property.mutable {
            static_mutable.insert(property.name.clone());
        }
    }
    let mut jvm_signatures = HashSet::new();
    for (function, (params, _)) in source.functions.iter().zip(&signatures) {
        let key = format!(
            "{}({})",
            function.name,
            params.iter().map(|p| p.1.descriptor()).collect::<String>()
        );
        if !jvm_signatures.insert(key) {
            return Err(
                Diagnostic::new(function.offset, "Conflicting JVM function signature")
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "JVM_SIGNATURE_CLASH"),
            );
        }
    }
    let function_names = source
        .functions
        .iter()
        .map(|f| f.name.clone())
        .collect::<Vec<_>>();
    let source_operator = source
        .functions
        .iter()
        .enumerate()
        .filter_map(|(i, f)| f.operator.then_some(i))
        .collect::<HashSet<_>>();
    let source_infix = source
        .functions
        .iter()
        .enumerate()
        .filter_map(|(id, f)| f.infix.then_some(id))
        .collect::<HashSet<_>>();
    let source_type_params = source
        .functions
        .iter()
        .map(|f| f.type_params.clone())
        .collect::<Vec<_>>();
    let source_defaults = source
        .functions
        .iter()
        .map(|f| f.defaults.clone())
        .collect::<Vec<_>>();
    let fields = HashMap::new();
    let mut members = HashMap::new();
    for property in &source.properties {
        for (prefix, name, active) in [
            (
                "#get:",
                getter_name(&property.name),
                !property.body.is_empty() || property.lateinit,
            ),
            (
                "#set:",
                setter_name(&property.name),
                property.setter.is_some(),
            ),
        ] {
            if active {
                if let Some(function) = static_state.methods.iter().find(|f| f.name == name) {
                    members.insert(
                        format!("{prefix}{}", property.name),
                        declarations::function_method(function, owner, 8, Some(&property.name)),
                    );
                }
            }
        }
    }
    let companion_fields = HashMap::new();
    let mut result = Vec::new();
    for (id, function) in source.functions.iter().enumerate() {
        let (params, return_type) = signatures[id].clone();
        let mut checker = Checker {
            inline_argument_modes: Vec::new(),
            inline_analysis: false,
            public_inline_body: function.inline && !function.private && !function.internal,
            reified_parameters: function.type_params.clone(),
            exhaustiveness_flow: false,
            lambdas: lambdas.clone(),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: owner.into(),
            inline_result: None,
            nonlocal_result: None,
            constants: &constants,
            constructor: None,
            defaults: &source_defaults,
            function_names: &function_names,
            mutable_fields: &static_mutable,
            classpath,
            extensions: &extensions,
            classes: &classes,
            imports: &imports,
            signatures: &signatures,
            source_type_params: &source_type_params,
            source_infix: &source_infix,
            source_operator: &source_operator,
            receiver: None,
            fields: &fields,
            members: &members,
            companion_fields: &companion_fields,
            functions: &functions,
            object_methods: &HashMap::new(),
            object_fields: &static_fields,
            object_owner: None,
            allow_return: true,
            refinements: HashMap::new(),
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
        if function.defaults.iter().any(Option::is_some) {
            if params.len() > 32 {
                return Err(Diagnostic::new(
                    function.offset,
                    "Default arguments support at most 32 parameters",
                ));
            }
            checker.next += 2;
        }
        let defaults = checker.checked_defaults(&function.defaults, &params, function.offset)?;
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
            class_type_params: Vec::new(),
            jvm_result: None,
            jvm_name: None,
            annotations: annotations::declaration_annotations(
                &function.annotations,
                classpath,
                &imports,
                "METHOD",
                None,
                Some(owner),
            )?,
            vararg: function.vararg,
            type_params: resolve_parameters(
                &function.type_params,
                &imports,
                classpath,
                function.offset,
                &classes,
            )?,
            defaults,
            name: function.name.clone(),
            private: function.private,
            is_override: false,
            params,
            result: return_type,
            locals: checker.next,
            body,
        });
    }
    result.append(&mut static_state.methods);
    Ok(Module {
        exposed_fields: static_state.exposed_fields,
        properties: static_state.properties,
        initializers: static_state.initializers,
        locals: static_state.locals,
        lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
        constants: source.constants.iter().map(clone_constant).collect(),
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

pub(crate) fn setter_name(name: &str) -> String {
    let suffix = if getter_name(name) == name {
        &name[2..]
    } else {
        name
    };
    format!("set{}{}", suffix[..1].to_ascii_uppercase(), &suffix[1..])
}

pub(crate) fn source_delegated_members(
    classpath: &Classpath,
    target: &Type,
) -> Result<Vec<JavaMethod>> {
    class_delegation::headers(classpath, target)
}

pub(crate) fn source_jvm_result(
    classpath: &Classpath,
    supertypes: &[Type],
    method: &JavaMethod,
) -> Result<Option<Type>> {
    let (parameters, result) = class_method_types(method)?;
    if result.is_reference() || result == Type::Unit {
        return Ok(None);
    }
    for parent in supertypes {
        let Some(owner) = library::type_owner(parent) else {
            continue;
        };
        let class = classpath.load(owner)?;
        for inherited in classpath.methods(owner)? {
            if inherited.name != method.name || inherited.access & 8 != 0 {
                continue;
            }
            let specialized =
                library::specialize_receiver(inherited.clone(), parent, &class, classpath);
            let (args, returns) = class_method_types(&specialized)?;
            if args
                .iter()
                .map(Type::descriptor)
                .eq(parameters.iter().map(Type::descriptor))
                && returns.base() == result.base()
            {
                let mut erased = inherited;
                erased.kotlin_params = None;
                erased.kotlin_result = None;
                if class_method_types(&erased)?.1.is_reference() {
                    let descriptor = Type::Nullable(Box::new(result.clone())).descriptor();
                    return Ok(Some(Type::Java(
                        descriptor
                            .trim_start_matches('L')
                            .trim_end_matches(';')
                            .into(),
                    )));
                }
            }
        }
    }
    Ok(None)
}

pub(crate) fn class_method_types(method: &JavaMethod) -> Result<(Vec<Type>, Type)> {
    if let (Some(params), Some(result)) = (&method.kotlin_params, &method.kotlin_result) {
        return Ok((params.iter().map(|p| p.1.clone()).collect(), result.clone()));
    }
    let (parameters, result) = crate::jvm_descriptor::method_types(&method.descriptor)?;
    if parameters.iter().map(Type::slots).sum::<usize>() > 254 {
        return Err(Diagnostic::new(
            0,
            "Unsupported interface method parameters",
        ));
    }
    Ok((parameters, result))
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

fn zero_value(ty: &Type) -> Value {
    Value {
        ty: ty.clone(),
        kind: match ty {
            Type::Long => ValueKind::Long(0),
            Type::Float => ValueKind::Float(0),
            Type::Double => ValueKind::Double(0),
            Type::Boolean => ValueKind::Boolean(false),
            t if t.is_reference() => ValueKind::Null,
            _ => ValueKind::Int(0),
        },
    }
}

fn clone_constant(c: &syntax::ConstDecl) -> syntax::ConstDecl {
    c.clone()
}
fn constant_expr(value: &syntax::ConstValue) -> Result<(Type, ExprKind)> {
    use syntax::ConstValue as C;
    Ok(match value {
        C::Int(v) => (Type::Int, ExprKind::Int(i64::from(*v))),
        C::Long(v) => (Type::Long, ExprKind::Long(i128::from(*v))),
        C::Float(v) => (Type::Float, ExprKind::Float(*v)),
        C::Double(v) => (Type::Double, ExprKind::Double(*v)),
        C::Char(v) => (Type::Char, ExprKind::Char(*v)),
        C::Boolean(v) => (Type::Boolean, ExprKind::Boolean(*v)),
        C::String(v) => (Type::String, ExprKind::String(v.clone())),
        C::Expression { expression, .. } => {
            return Err(
                Diagnostic::new(expression.offset, "Unresolved constant initializer").with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "CONSTANT_EXPRESSION_UNRESOLVED",
                ),
            );
        }
    })
}

fn java_parameter(ty: Type) -> Type {
    if ty.is_reference() {
        Type::Platform(Box::new(ty))
    } else {
        ty
    }
}
