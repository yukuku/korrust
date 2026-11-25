use super::*;
use std::collections::BTreeMap;

struct Point {
    value: Value,
    spills: Vec<(u8, Type)>,
}

struct Plan {
    body: Vec<Statement>,
    points: Vec<Point>,
    fields: BTreeMap<u8, Type>,
    locals: u16,
}

pub(super) struct State {
    pub(super) frame: u8,
    pub(super) pending_depth: usize,
    result: u8,
    marker: u8,
    temporary: u8,
    pub(super) frame_owner: String,
    fields: BTreeMap<u8, Type>,
    states: Vec<usize>,
}

fn unsupported(message: &str) -> Diagnostic {
    Diagnostic::new(0, message).with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "SUSPEND_STATE_MACHINE_SHAPE_UNSUPPORTED",
    )
}

pub(super) fn suspension(value: &Value, functions: &[crate::check::Function]) -> bool {
    match &value.kind {
        ValueKind::Call(index, _) => functions[*index].suspend_,
        ValueKind::JavaCall { method, .. }
        | ValueKind::MemberCall { method, .. }
        | ValueKind::SuperCall { method, .. } => method
            .library
            .as_ref()
            .is_some_and(|method| method.suspend_),
        ValueKind::FunctionInvoke { function, .. } => {
            matches!(function.ty.base(), Type::SuspendFunction { .. })
        }
        _ => false,
    }
}

pub(super) fn contains_suspension(
    body: &[Statement],
    functions: &[crate::check::Function],
) -> bool {
    let mut found = false;
    crate::check::ir_walk::statements(&mut body.to_vec(), "", &mut |value: &mut Value, _: &str| {
        found |= suspension(value, functions);
    });
    found
}

fn boxed_return(value: &Value) -> Value {
    if value.ty == Type::Unit {
        Value {
            ty: Type::Java("kotlin/Unit".into()),
            kind: ValueKind::Block(
                vec![Statement::Expression(value.clone())],
                Box::new(Value {
                    ty: Type::Java("kotlin/Unit".into()),
                    kind: ValueKind::StaticField {
                        owner: "kotlin/Unit".into(),
                        name: "INSTANCE".into(),
                    },
                }),
            ),
        }
    } else {
        Value {
            ty: Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
            kind: ValueKind::Box(Box::new(value.clone())),
        }
    }
}

fn plan(function: &crate::check::Function, functions: &[crate::check::Function]) -> Result<Plan> {
    if !function.suspend_ && contains_suspension(&function.body, functions) {
        return Err(
            Diagnostic::new(0, "Suspend calls require a suspend function context").with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "SUSPEND_CALL_CONTEXT_INVALID",
            ),
        );
    }
    struct Planner<'a> {
        functions: &'a [crate::check::Function],
        fields: BTreeMap<u8, Type>,
        points: Vec<Point>,
        error: Option<Diagnostic>,
        suspend_: bool,
        counts: Vec<usize>,
        locals: u16,
    }
    impl crate::check::ir_walk::Visitor for Planner<'_> {
        fn enter_value(&mut self, _: &Value) {
            self.counts.push(0);
        }
        fn value(&mut self, value: &mut Value, _: &str) {
            let direct = suspension(value, self.functions);
            let count = self.counts.pop().unwrap() + usize::from(direct);
            if let Some(parent) = self.counts.last_mut() {
                *parent += count;
            }
            if count != 0 && !direct {
                let slots = match &value.kind {
                    ValueKind::Binary(operator, left, right)
                        if operator != "&&" && operator != "||" =>
                    {
                        Some(left.ty.slots() + right.ty.slots())
                    }
                    _ => None,
                };
                if let Some(slots) = slots {
                    if self.locals as usize + slots > 255 {
                        self.error = Some(unsupported(
                            "Suspend expression temporaries exceed JVM local limit",
                        ));
                    } else {
                        let ValueKind::Binary(operator, left, right) =
                            std::mem::replace(&mut value.kind, ValueKind::Unit)
                        else {
                            unreachable!()
                        };
                        let left_slot = self.locals as u8;
                        self.locals += left.ty.slots() as u16;
                        let right_slot = self.locals as u8;
                        self.locals += right.ty.slots() as u16;
                        self.fields.insert(left_slot, left.ty.clone());
                        self.fields.insert(right_slot, right.ty.clone());
                        let result = Value {
                            ty: value.ty.clone(),
                            kind: ValueKind::Binary(
                                operator,
                                Box::new(Value {
                                    ty: left.ty.clone(),
                                    kind: ValueKind::Local(left_slot),
                                }),
                                Box::new(Value {
                                    ty: right.ty.clone(),
                                    kind: ValueKind::Local(right_slot),
                                }),
                            ),
                        };
                        value.kind = ValueKind::Let {
                            slot: left_slot,
                            value: left,
                            body: Box::new(Value {
                                ty: value.ty.clone(),
                                kind: ValueKind::Let {
                                    slot: right_slot,
                                    value: right,
                                    body: Box::new(result),
                                },
                            }),
                        };
                    }
                }
            }
            if direct {
                if count != 1 {
                    self.error = Some(unsupported(
                        "Nested suspension arguments and expressions are unsupported",
                    ));
                }
                if matches!(&value.kind, ValueKind::JavaCall { method, .. } if method.name.ends_with("$default"))
                {
                    self.error = Some(unsupported(
                        "Imported suspend default argument calls are unsupported",
                    ));
                }
            } else if count != 0
                && !matches!(&value.kind, ValueKind::Binary(operator, _, _) if operator == "&&" || operator == "||")
                && !matches!(
                    &value.kind,
                    ValueKind::If(..)
                        | ValueKind::Block(..)
                        | ValueKind::Let { .. }
                        | ValueKind::Try { .. }
                        | ValueKind::Box(_)
                        | ValueKind::Unbox(_)
                        | ValueKind::Convert(..)
                        | ValueKind::Not(_)
                        | ValueKind::Negate(_)
                        | ValueKind::RequireNotNull(_)
                        | ValueKind::NotNull(..)
                        | ValueKind::Abrupt(_)
                )
            {
                self.error = Some(unsupported(
                    "Suspension inside expression operands is unsupported",
                ));
            }
            if let ValueKind::Let { slot, value, .. } = &value.kind {
                self.fields.insert(*slot, value.ty.clone());
            }
            super::suspend_regions::gather(value, &mut self.fields);
            if suspension(value, self.functions) {
                self.points.push(Point {
                    value: value.clone(),
                    spills: Vec::new(),
                });
            }
        }
        fn statement(&mut self, statement: &mut Statement, _: &str) {
            match statement {
                Statement::Store(slot, value) => {
                    if value.ty == Type::Unit {
                        self.error = Some(unsupported(
                            "Saved Unit locals in suspend functions are unsupported",
                        ));
                    }
                    self.fields.insert(*slot, value.ty.clone());
                }
                Statement::ForUntil { slot, end_slot, .. } => {
                    self.fields.insert(*slot, Type::Int);
                    self.fields.insert(*end_slot, Type::Int);
                }
                Statement::ForIterator {
                    slot,
                    iterator_slot,
                    element,
                    ..
                } => {
                    self.fields.insert(*slot, element.clone());
                    self.fields
                        .insert(*iterator_slot, Type::Java("java/util/Iterator".into()));
                }
                Statement::ForEach {
                    slot,
                    iterable,
                    iterable_slot,
                    index_slot,
                    ..
                } => {
                    let element = match iterable.ty.base() {
                        Type::String => Type::Char,
                        Type::List(element) | Type::MutableList(element) => *element.clone(),
                        array => array.array_element().unwrap(),
                    };
                    self.fields.insert(*slot, element);
                    self.fields.insert(
                        *iterable_slot,
                        if matches!(iterable.ty.base(), Type::List(_) | Type::MutableList(_)) {
                            Type::Java("java/util/Iterator".into())
                        } else {
                            iterable.ty.clone()
                        },
                    );
                    self.fields.insert(*index_slot, Type::Int);
                }
                Statement::Return(value) if self.suspend_ => {
                    *value = boxed_return(value);
                }
                Statement::FieldStore { .. }
                | Statement::JavaFieldStore { .. }
                | Statement::IndexStore(..)
                    if contains_suspension(std::slice::from_ref(statement), self.functions) =>
                {
                    self.error = Some(unsupported(
                        "Suspension in assignment operands is unsupported",
                    ));
                }
                _ => {}
            }
        }
    }
    let fields = parameter_slots(&function.params, 0)
        .into_iter()
        .zip(&function.params)
        .map(|(slot, (_, ty))| (slot as u8, ty.clone()))
        .collect();
    let mut planner = Planner {
        functions,
        fields,
        points: Vec::new(),
        error: None,
        suspend_: function.suspend_,
        counts: Vec::new(),
        locals: function.locals,
    };
    let mut body = function.body.clone();
    crate::check::ir_walk::statements(&mut body, "", &mut planner);
    if let Some(error) = planner.error {
        return Err(error);
    }
    Ok(Plan {
        body,
        points: planner.points,
        fields: planner.fields,
        locals: planner.locals,
    })
}

fn unsupported_unit_type(ty: &Type) -> bool {
    match ty {
        Type::Nullable(inner) => inner.as_ref() == &Type::Unit || unsupported_unit_type(inner),
        Type::Platform(inner)
        | Type::Array(inner)
        | Type::OutArray(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner)
        | Type::Parameter {
            upper_bound: inner, ..
        } => unsupported_unit_type(inner),
        Type::Parameterized { arguments, .. } => arguments
            .iter()
            .filter_map(|argument| argument.ty.as_ref())
            .any(unsupported_unit_type),
        Type::SuspendFunction { params, result } => {
            params
                .iter()
                .any(|ty| ty == &Type::Unit || unsupported_unit_type(ty))
                || unsupported_unit_type(result)
        }
        Type::Function { params, result } => {
            params.iter().any(unsupported_unit_type) || unsupported_unit_type(result)
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            unsupported_unit_type(receiver)
                || params.iter().any(unsupported_unit_type)
                || unsupported_unit_type(result)
        }
        _ => false,
    }
}

fn forbidden_type(ty: &Type, owners: &std::collections::BTreeSet<String>) -> bool {
    match ty {
        Type::Java(owner) => owners.contains(owner),
        Type::Parameterized { name, arguments } => {
            owners.contains(name)
                || arguments
                    .iter()
                    .filter_map(|argument| argument.ty.as_ref())
                    .any(|ty| forbidden_type(ty, owners))
        }
        Type::Nullable(inner)
        | Type::Platform(inner)
        | Type::Array(inner)
        | Type::OutArray(inner)
        | Type::List(inner)
        | Type::MutableList(inner)
        | Type::ThreadLocal(inner)
        | Type::Parameter {
            upper_bound: inner, ..
        } => forbidden_type(inner, owners),
        Type::Function { params, result } | Type::SuspendFunction { params, result } => {
            params.iter().any(|ty| forbidden_type(ty, owners)) || forbidden_type(result, owners)
        }
        Type::ReceiverFunction {
            receiver,
            params,
            result,
        } => {
            forbidden_type(receiver, owners)
                || params.iter().any(|ty| forbidden_type(ty, owners))
                || forbidden_type(result, owners)
        }
        _ => false,
    }
}

fn forbidden_value(value: &Value, owners: &std::collections::BTreeSet<String>) -> bool {
    forbidden_type(&value.ty, owners)
        || match &value.kind {
            ValueKind::JavaCall { method, .. }
            | ValueKind::MemberCall { method, .. }
            | ValueKind::SuperCall { method, .. }
            | ValueKind::CompiledInline { method, .. } => {
                owners.contains(&method.owner)
                    || method.kotlin_params.as_ref().is_some_and(|params| {
                        params.iter().any(|(_, ty, _)| forbidden_type(ty, owners))
                    })
                    || method
                        .kotlin_result
                        .as_ref()
                        .is_some_and(|ty| forbidden_type(ty, owners))
                    || method.library.as_ref().is_some_and(|library| {
                        library
                            .reified_arguments
                            .iter()
                            .any(|(_, ty)| forbidden_type(ty, owners))
                    })
            }
            ValueKind::ClassLiteral(ty)
            | ValueKind::TypeTest { target: ty, .. }
            | ValueKind::Cast { target: ty, .. } => forbidden_type(ty, owners),
            ValueKind::StaticCall { owner, .. }
            | ValueKind::CompanionArray { owner, .. }
            | ValueKind::New { owner, .. }
            | ValueKind::ObjectInstance(owner)
            | ValueKind::StaticField { owner, .. }
            | ValueKind::JavaField { owner, .. } => owners.contains(owner),
            _ => false,
        }
}

fn validate_parameter_annotations(function: &crate::syntax::Function) -> Result<()> {
    if function.suspend_
        && function
            .param_annotations
            .iter()
            .any(|annotations| !annotations.is_empty())
    {
        return Err(Diagnostic::new(
            function.offset,
            "Custom parameter annotations on suspend declarations are unsupported",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "SUSPEND_PARAMETER_ANNOTATION_UNSUPPORTED",
        ));
    }
    Ok(())
}

fn validate_member_shape(
    function: &crate::syntax::Function,
    generic_owner: bool,
    inheritable_body: bool,
) -> Result<()> {
    validate_parameter_annotations(function)?;
    if function.suspend_
        && (function.private
            || generic_owner
            || inheritable_body
            || function.receiver.is_some()
            || !function.type_params.is_empty()
            || function.inline
            || function.vararg
            || function.defaults.iter().any(Option::is_some))
    {
        return Err(Diagnostic::new(function.offset, "Private, generic, value-class, extension, inline, vararg, defaulted, and inheritable concrete suspend members are unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "SUSPEND_MEMBER_SHAPE_UNSUPPORTED"));
    }
    Ok(())
}

fn validate_function(
    function: &crate::check::Function,
    functions: &[crate::check::Function],
    forbidden_owners: &std::collections::BTreeSet<String>,
) -> Result<()> {
    if function.suspend_ {
        if function
            .params
            .iter()
            .any(|(_, ty)| unsupported_unit_type(ty))
            || unsupported_unit_type(&function.result)
        {
            return Err(unsupported(
                "Nullable Unit and Unit-valued suspend callback parameters are unsupported",
            ));
        }
        if function
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>()
            > 253
        {
            return Err(unsupported(
                "Suspend member parameters exceed the JVM limit including receiver and continuation",
            ));
        }
        let mut forbidden = function
            .params
            .iter()
            .any(|(_, ty)| forbidden_type(ty, forbidden_owners))
            || forbidden_type(&function.result, forbidden_owners);
        crate::check::ir_walk::statements(
            &mut function.body.clone(),
            "",
            &mut |value: &mut Value, _: &str| {
                forbidden |= forbidden_value(value, forbidden_owners);
            },
        );
        if forbidden {
            return Err(
                Diagnostic::new(0, "Value-class types in suspend members are unsupported")
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "SUSPEND_VALUE_CLASS_INTERACTION_UNSUPPORTED",
                    ),
            );
        }
    }
    if function.suspend_ || contains_suspension(&function.body, functions) {
        plan(function, functions)?;
    }
    Ok(())
}

pub(crate) fn validate(
    module: &Module,
    source: &crate::syntax::Module,
    forbidden_owners: &std::collections::BTreeSet<String>,
) -> Result<()> {
    for function in &source.functions {
        validate_parameter_annotations(function)?;
        if function.suspend_
            && (function.receiver.is_some()
                || !function.type_params.is_empty()
                || function.inline
                || function.vararg
                || function.defaults.iter().any(Option::is_some))
        {
            return Err(Diagnostic::new(function.offset, "Generic, extension, inline, vararg, and defaulted suspend declarations are unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "SUSPEND_DECLARATION_SHAPE_UNSUPPORTED"));
        }
    }
    if let Some(class) = &source.class_decl {
        for method in &class.methods {
            validate_member_shape(
                &method.function,
                !class.type_params.is_empty() || class.value_,
                method.has_body && (class.open || class.abstract_),
            )?;
        }
    }
    fn interface_shapes(interface: &crate::syntax::InterfaceDecl) -> Result<()> {
        for method in &interface.methods {
            validate_member_shape(&method.function, !interface.type_params.is_empty(), false)?;
        }
        for nested in &interface.nested {
            interface_shapes(nested)?;
        }
        Ok(())
    }
    if let Some(interface) = &source.interface_decl {
        interface_shapes(interface)?;
    }
    let mut members = Vec::new();
    if let Some(class) = &module.class_decl {
        members.extend(class.methods.iter());
    }
    fn interface_functions<'a>(
        interface: &'a crate::check::InterfaceDecl,
        members: &mut Vec<&'a crate::check::Function>,
    ) {
        members.extend(interface.methods.iter().map(|method| &method.function));
        for nested in &interface.nested {
            interface_functions(nested, members);
        }
    }
    if let Some(interface) = &module.interface_decl {
        interface_functions(interface, &mut members);
    }
    for function in members {
        validate_function(function, &module.functions, forbidden_owners)?;
    }
    let mut signatures = std::collections::BTreeSet::new();
    for function in &module.functions {
        if !signatures.insert((function.jvm_name(), function.descriptor())) {
            return Err(
                Diagnostic::new(0, "Conflicting physical JVM function signatures")
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "JVM_SIGNATURE_CLASH"),
            );
        }
        if function.suspend_ {
            let mut unsupported_unit = function
                .params
                .iter()
                .any(|(_, ty)| unsupported_unit_type(ty))
                || unsupported_unit_type(&function.result);
            crate::check::ir_walk::statements(
                &mut function.body.clone(),
                "",
                &mut |value: &mut Value, _: &str| {
                    unsupported_unit |= unsupported_unit_type(&value.ty);
                },
            );
            if unsupported_unit {
                return Err(unsupported(
                    "Nullable Unit and Unit-valued suspend callback parameters are unsupported",
                ));
            }
            if function
                .params
                .iter()
                .map(|(_, ty)| ty.slots())
                .sum::<usize>()
                >= 255
            {
                return Err(unsupported(
                    "Suspend function parameters exceed the JVM limit including the continuation",
                ));
            }
            let mut forbidden = function
                .params
                .iter()
                .any(|(_, ty)| forbidden_type(ty, forbidden_owners))
                || forbidden_type(&function.result, forbidden_owners);
            crate::check::ir_walk::statements(
                &mut function.body.clone(),
                "",
                &mut |value: &mut Value, _: &str| {
                    forbidden |= forbidden_value(value, forbidden_owners);
                },
            );
            if forbidden {
                return Err(Diagnostic::new(source.functions.iter().find(|source| source.name == function.name).map_or(0, |source| source.offset), "Value-class types in suspend declarations, callbacks, or saved values are unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "SUSPEND_VALUE_CLASS_INTERACTION_UNSUPPORTED"));
            }
        }
        if !function.suspend_ && !contains_suspension(&function.body, &module.functions) {
            continue;
        }
        plan(function, &module.functions).map_err(|mut error| {
            error.offset = source
                .functions
                .iter()
                .find(|source| source.name == function.name)
                .map_or(0, |source| source.offset);
            error
        })?;
    }
    for lambda in &module.lambdas {
        if lambda.function.suspend_ {
            validate_function(&lambda.function, &module.functions, forbidden_owners)?;
        } else if contains_suspension(&lambda.function.body, &module.functions) {
            return Err(
                Diagnostic::new(0, "Suspend calls require a suspend function context").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "SUSPEND_CALL_CONTEXT_INVALID",
                ),
            );
        }
    }
    Ok(())
}

fn frame_owner(owner: &str, index: usize) -> String {
    format!("{owner}$suspend${index}")
}

fn code<'a>(
    pool: &'a mut Pool,
    functions: &'a [crate::check::Function],
    lambdas: &'a [crate::check::LambdaDecl],
    owner: &'a str,
) -> Code<'a> {
    Code {
        coroutine: true,
        suspension: None,
        inline_markers: false,
        pool,
        functions,
        owner,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        loop_finally_depths: Vec::new(),
        loop_names: Vec::new(),
        pending_loop_name: None,
        finalizers: Vec::new(),
        exceptions: Vec::new(),
        temporary_count: 0,
        temporary_patches: Vec::new(),
        lambdas,
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        in_lambda: false,
        max_stack: 32,
    }
}

fn spill_name(slot: u8) -> String {
    format!("spill${slot}")
}

fn throw_on_failure(code: &mut Code<'_>, result: u8) -> Result<()> {
    code.bytes.extend([0x19, result]);
    code.method_reference(
        0xb8,
        "kotlin/ResultKt",
        "throwOnFailure",
        "(Ljava/lang/Object;)V",
    )
}

fn call(
    code: &mut Code<'_>,
    point: &Point,
    frame: u8,
    frame_owner: &str,
    next_state: usize,
    temporary: u8,
) -> Result<()> {
    let (receiver, args) = match &point.value.kind {
        ValueKind::Call(_, args) => (None, args.as_slice()),
        ValueKind::JavaCall { method, args } if method.access & 8 != 0 => (None, args.as_slice()),
        ValueKind::JavaCall { args, .. } | ValueKind::MemberCall { args, .. } => {
            (Some(&args[0]), &args[1..])
        }
        ValueKind::FunctionInvoke { function, args } => (Some(function.as_ref()), args.as_slice()),
        _ => return Err(unsupported("Unsupported suspension invocation")),
    };
    let mut next = temporary as u16;
    let receiver_slot = if let Some(receiver) = receiver {
        code.value(receiver)?;
        let slot = next as u8;
        next += 1;
        code.bytes.extend([0x3a, slot]);
        Some(slot)
    } else {
        None
    };
    let mut slots = Vec::new();
    for argument in args {
        if next + argument.ty.slots() as u16 > 255 {
            return Err(unsupported(
                "Suspend invocation temporaries exceed JVM local limit",
            ));
        }
        let slot = next as u8;
        next += argument.ty.slots() as u16;
        code.value(argument)?;
        code.bytes.extend([store_opcode(&argument.ty), slot]);
        slots.push(slot);
    }
    for (slot, ty) in &point.spills {
        code.bytes.extend([0x19, frame, load_opcode(ty), *slot]);
        code.field_reference(0xb5, frame_owner, &spill_name(*slot), &ty.descriptor())?;
    }
    code.bytes.extend([0x19, frame]);
    code.integer(next_state as i32)?;
    code.field_reference(0xb5, frame_owner, "label", "I")?;
    if let Some(slot) = receiver_slot {
        code.bytes.extend([0x19, slot]);
    }
    let physical = match &point.value.kind {
        ValueKind::Call(index, _) => invocation_types(&code.functions[*index].descriptor())?.0,
        ValueKind::JavaCall { method, .. } | ValueKind::MemberCall { method, .. } => {
            invocation_types(&method.descriptor)?.0
        }
        ValueKind::FunctionInvoke { .. } => {
            vec![Type::Java("java/lang/Object".into()); args.len() + 1]
        }
        _ => unreachable!(),
    };
    if physical.len() != args.len() + 1 {
        return Err(unsupported(
            "Suspension invocation has an unsupported physical parameter layout",
        ));
    }
    for ((argument, slot), expected) in args.iter().zip(slots).zip(&physical) {
        code.bytes.extend([load_opcode(&argument.ty), slot]);
        code.adapt_value(&argument.ty, expected)?;
    }
    code.bytes.extend([0x19, frame]);
    match &point.value.kind {
        ValueKind::Call(index, _) => {
            let function = &code.functions[*index];
            code.method_reference(
                0xb8,
                code.owner,
                function.jvm_name(),
                &function.descriptor(),
            )?;
        }
        ValueKind::JavaCall { method, .. } => {
            code.method_reference(
                if method.access & 8 != 0 {
                    0xb8
                } else if method.access & 2 != 0 {
                    0xb7
                } else {
                    0xb6
                },
                &method.owner,
                &method.name,
                &method.descriptor,
            )?;
        }
        ValueKind::MemberCall {
            method, interface, ..
        } => {
            if *interface {
                let reference =
                    code.pool
                        .interface_method(&method.owner, &method.name, &method.descriptor)?;
                code.reference(0xb9, reference);
                code.bytes.extend([
                    (physical.iter().map(Type::slots).sum::<usize>() + 1) as u8,
                    0,
                ]);
            } else {
                code.method_reference(0xb6, &method.owner, &method.name, &method.descriptor)?;
            }
        }
        ValueKind::FunctionInvoke { args, .. } => {
            let arity = args.len() + 1;
            let method = code.pool.interface_method(
                &format!("kotlin/jvm/functions/Function{arity}"),
                "invoke",
                &format!("({})Ljava/lang/Object;", "Ljava/lang/Object;".repeat(arity)),
            )?;
            code.reference(0xb9, method);
            code.bytes.extend([(arity + 1) as u8, 0]);
        }
        _ => unreachable!(),
    }
    code.max_stack = code.max_stack.max(
        2 + args
            .iter()
            .map(|argument| argument.ty.slots())
            .sum::<usize>(),
    );
    Ok(())
}

pub(super) fn emit_value(code: &mut Code<'_>, value: &Value) -> Result<()> {
    let state = code.suspension.as_ref().unwrap();
    let (frame, result, marker, temporary) =
        (state.frame, state.result, state.marker, state.temporary);
    let owner = state.frame_owner.clone();
    let point = Point {
        value: value.clone(),
        spills: state
            .fields
            .iter()
            .map(|(slot, ty)| (*slot, ty.clone()))
            .collect(),
    };
    let next_state = state.states.len();
    let resume = code.label();
    let after = code.label();
    let immediate = code.label();
    code.suspension.as_mut().unwrap().states.push(resume);
    call(code, &point, frame, &owner, next_state, temporary)?;
    code.bytes
        .extend([0x3a, result, 0x19, result, 0x19, marker]);
    code.branch(0xa6, immediate);
    code.bytes.extend([0x19, marker, 0xb0]);
    code.mark(immediate);
    code.branch(0xa7, after);
    code.mark(resume);
    for (slot, ty) in &point.spills {
        code.bytes.extend([0x19, frame]);
        code.field_reference(0xb4, &owner, &spill_name(*slot), &ty.descriptor())?;
        code.bytes.extend([store_opcode(ty), *slot]);
    }
    throw_on_failure(code, result)?;
    code.mark(after);
    if value.ty != Type::Unit {
        code.bytes.extend([0x19, result]);
        code.adapt_value(&Type::Java("java/lang/Object".into()), &value.ty)?;
    }
    Ok(())
}

pub(super) fn method(
    pool: &mut Pool,
    module: &Module,
    owner: &str,
    index: usize,
) -> Result<Vec<u8>> {
    emit_method(
        pool,
        &module.functions,
        &module.lambdas,
        owner,
        &module.functions[index],
        index,
        None,
    )
}

fn with_receiver(function: &crate::check::Function, owner: &str) -> crate::check::Function {
    let mut planned = function.clone();
    planned
        .params
        .insert(0, ("$this".into(), Type::Java(owner.into())));
    planned
}

pub(super) fn emit_method(
    pool: &mut Pool,
    functions: &[crate::check::Function],
    lambdas: &[crate::check::LambdaDecl],
    owner: &str,
    function: &crate::check::Function,
    index: usize,
    member_flags: Option<u16>,
) -> Result<Vec<u8>> {
    emit_method_in(
        pool,
        functions,
        lambdas,
        owner,
        owner,
        function,
        index,
        member_flags,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_method_in(
    pool: &mut Pool,
    functions: &[crate::check::Function],
    lambdas: &[crate::check::LambdaDecl],
    owner: &str,
    facade: &str,
    function: &crate::check::Function,
    index: usize,
    member_flags: Option<u16>,
) -> Result<Vec<u8>> {
    let planned = member_flags.map(|_| with_receiver(function, owner));
    let planned = planned.as_ref().unwrap_or(function);
    let plan = plan(planned, functions)?;
    let mut code = code(pool, functions, lambdas, facade);
    code.in_lambda = owner != facade;
    let mut locals = plan.locals;
    if plan.points.is_empty() {
        code.block(&plan.body)?;
    } else {
        let incoming = planned
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>() as u8;
        let temporaries = plan
            .points
            .iter()
            .map(|point| match &point.value.kind {
                ValueKind::Call(_, args) | ValueKind::JavaCall { args, .. } => args
                    .iter()
                    .map(|argument| argument.ty.slots())
                    .sum::<usize>(),
                ValueKind::MemberCall { args, .. } => args
                    .iter()
                    .map(|argument| argument.ty.slots())
                    .sum::<usize>(),
                ValueKind::FunctionInvoke { args, .. } => {
                    1 + args
                        .iter()
                        .map(|argument| argument.ty.slots())
                        .sum::<usize>()
                }
                _ => 0,
            })
            .max()
            .unwrap_or(0);
        if plan.locals as usize + 3 + temporaries > 255 {
            return Err(unsupported("Suspend frame exceeds JVM local limit"));
        }
        let frame = plan.locals as u8;
        let result = frame + 1;
        let marker = frame + 2;
        let temporary = frame + 3;
        locals = plan.locals + 3 + temporaries as u16;
        let frame_owner = frame_owner(owner, index);
        let parameters = planned
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>();
        for (slot, ty) in &plan.fields {
            if (*slot as usize) < parameters {
                continue;
            }
            code.bytes.push(if ty.is_reference() {
                0x01
            } else if ty == &Type::Long {
                0x09
            } else if ty == &Type::Double {
                0x0e
            } else if ty == &Type::Float {
                0x0b
            } else {
                0x03
            });
            code.bytes.extend([store_opcode(ty), *slot]);
        }
        let create = code.label();
        let ready = code.label();
        code.bytes.extend([0x19, incoming]);
        code.class_reference(0xc1, &frame_owner)?;
        code.branch(0x99, create);
        code.bytes.extend([0x19, incoming]);
        code.class_reference(0xc0, &frame_owner)?;
        code.bytes.extend([0x3a, frame]);
        code.bytes.extend([0x19, frame]);
        code.field_reference(0xb4, &frame_owner, "label", "I")?;
        code.integer(i32::MIN)?;
        code.bytes.push(0x7e);
        code.branch(0x99, create);
        code.bytes.extend([0x19, frame, 0x59]);
        code.field_reference(0xb4, &frame_owner, "label", "I")?;
        code.integer(i32::MIN)?;
        code.bytes.push(0x64);
        code.field_reference(0xb5, &frame_owner, "label", "I")?;
        code.branch(0xa7, ready);
        code.mark(create);
        code.class_reference(0xbb, &frame_owner)?;
        code.bytes.push(0x59);
        code.bytes.extend([0x19, incoming]);
        code.method_reference(
            0xb7,
            &frame_owner,
            "<init>",
            "(Lkotlin/coroutines/Continuation;)V",
        )?;
        code.bytes.extend([0x3a, frame]);
        code.mark(ready);
        code.bytes.extend([0x19, frame]);
        code.field_reference(0xb4, &frame_owner, "result", "Ljava/lang/Object;")?;
        code.bytes.extend([0x3a, result]);
        code.method_reference(
            0xb8,
            "kotlin/coroutines/intrinsics/IntrinsicsKt",
            "getCOROUTINE_SUSPENDED",
            "()Ljava/lang/Object;",
        )?;
        code.bytes.extend([0x3a, marker]);
        let dispatch = code.label();
        let entry = code.label();
        code.branch(0xa7, dispatch);
        code.mark(entry);
        throw_on_failure(&mut code, result)?;
        code.suspension = Some(State {
            pending_depth: 0,
            frame,
            result,
            marker,
            temporary,
            frame_owner: frame_owner.clone(),
            fields: plan.fields.clone(),
            states: vec![entry],
        });
        code.block(&plan.body)?;
        code.mark(dispatch);
        let states = code.suspension.as_ref().unwrap().states.clone();
        for (index, state) in states.iter().enumerate() {
            code.bytes.extend([0x19, frame]);
            code.field_reference(0xb4, &frame_owner, "label", "I")?;
            code.integer(index as i32)?;
            code.branch(0x9f, *state);
        }
        code.class_reference(0xbb, "java/lang/IllegalStateException")?;
        code.bytes.push(0x59);
        code.value(&Value {
            ty: Type::String,
            kind: ValueKind::String(
                "call to 'resume' before 'invoke' with coroutine"
                    .encode_utf16()
                    .collect(),
            ),
        })?;
        code.method_reference(
            0xb7,
            "java/lang/IllegalStateException",
            "<init>",
            "(Ljava/lang/String;)V",
        )?;
        code.bytes.push(0xbf);
    }
    let stack = code.max_stack as u16;
    let code = code.into_simple(locals, stack)?;
    let mut method = simple_method(
        pool,
        member_flags.unwrap_or(if function.private { 0x001a } else { 0x0019 }),
        function.jvm_name(),
        &function.descriptor(),
        code,
        generic_function(function).as_deref(),
        false,
    )?;
    let mut params = function.params.clone();
    params.push((
        "$completion".into(),
        Type::continuation(&function.result, crate::syntax::Variance::In),
    ));
    parameter_annotations(pool, &mut method, &params)?;
    annotations::append_annotations(pool, &mut method, &function.annotations)?;
    nullable_result(pool, &mut method, function.physical_result())?;
    if member_flags.is_some() {
        frames::attach(pool, &mut method, owner, &params, function.locals, false)?;
    }
    Ok(method)
}

pub(super) fn access_bridge(
    pool: &mut Pool,
    function: &crate::check::Function,
    owner: &str,
    index: usize,
) -> Result<Vec<u8>> {
    let mut params = function.params.clone();
    params.push((
        "$completion".into(),
        Type::Java("kotlin/coroutines/Continuation".into()),
    ));
    let slots = params.iter().map(|(_, ty)| ty.slots()).sum::<usize>();
    let mut code = SimpleCode::new(slots as u16, slots as u16);
    for (slot, (_, ty)) in parameter_slots(&params, 0).into_iter().zip(&params) {
        code.bytes.extend([load_opcode(ty), slot as u8]);
    }
    code.reference(
        0xb8,
        pool.method(owner, function.jvm_name(), &function.descriptor())?,
    );
    code.bytes.push(0xb0);
    simple_method(
        pool,
        0x1009,
        &format!("access$suspend${index}"),
        &function.descriptor(),
        code,
        None,
        false,
    )
}

pub(crate) fn frames(module: &Module, owner: &str, file: &str) -> Result<Vec<crate::ClassFile>> {
    emit_frames(&module.functions, owner, file, false)
}

pub(super) fn emit_frames(
    functions: &[crate::check::Function],
    owner: &str,
    file: &str,
    member: bool,
) -> Result<Vec<crate::ClassFile>> {
    emit_frames_in(functions, functions, owner, file, member)
}

pub(super) fn emit_frames_in(
    functions: &[crate::check::Function],
    context: &[crate::check::Function],
    owner: &str,
    file: &str,
    member: bool,
) -> Result<Vec<crate::ClassFile>> {
    let mut files = Vec::new();
    for (index, function) in functions
        .iter()
        .enumerate()
        .filter(|(_, function)| function.suspend_)
    {
        let planned = member.then(|| with_receiver(function, owner));
        let plan = plan(planned.as_ref().unwrap_or(function), context)?;
        if plan.points.is_empty() {
            continue;
        }
        let frame_owner = frame_owner(owner, index);
        let mut pool = Pool::default();
        let this = pool.class(&frame_owner)?;
        let parent = pool.class("kotlin/coroutines/jvm/internal/ContinuationImpl")?;
        let mut fields = Vec::new();
        for (slot, ty) in &plan.fields {
            fields.extend(class_field_scoped(&mut pool, &spill_name(*slot), ty, 0)?);
        }
        for depth in 1..=super::suspend_regions::field_count(&function.body) {
            fields.extend(class_field(
                &mut pool,
                &format!("pending${depth}"),
                &Type::Java("java/lang/Object".into()),
                0,
            )?);
        }
        fields.extend(class_field(
            &mut pool,
            "result",
            &Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
            0,
        )?);
        fields.extend(class_field(&mut pool, "label", &Type::Int, 0)?);
        let mut constructor = SimpleCode::new(2, 2);
        constructor.bytes.extend([0x2a, 0x2b]);
        constructor.reference(
            0xb7,
            pool.method(
                "kotlin/coroutines/jvm/internal/ContinuationImpl",
                "<init>",
                "(Lkotlin/coroutines/Continuation;)V",
            )?,
        );
        constructor.bytes.push(0xb1);
        let constructor = simple_method(
            &mut pool,
            0,
            "<init>",
            "(Lkotlin/coroutines/Continuation;)V",
            constructor,
            None,
            false,
        )?;
        let mut invoke = SimpleCode::new(
            (2 + function
                .params
                .iter()
                .map(|(_, ty)| ty.slots())
                .sum::<usize>())
            .max(3) as u16,
            2,
        );
        invoke.bytes.extend([0x2a, 0x2b]);
        invoke.reference(
            0xb5,
            pool.field(&frame_owner, "result", "Ljava/lang/Object;")?,
        );
        invoke.bytes.extend([0x2a, 0x59]);
        invoke.reference(0xb4, pool.field(&frame_owner, "label", "I")?);
        invoke.reference(0x13, pool.add(Constant::Integer(i32::MIN))?);
        invoke.bytes.push(0x80);
        invoke.reference(0xb5, pool.field(&frame_owner, "label", "I")?);
        if member {
            invoke.bytes.push(0x2a);
            invoke.reference(
                0xb4,
                pool.field(&frame_owner, "spill$0", &format!("L{owner};"))?,
            );
        }
        for (_, ty) in &function.params {
            invoke.bytes.push(if ty.is_reference() {
                0x01
            } else if ty == &Type::Long {
                0x09
            } else if ty == &Type::Double {
                0x0e
            } else if ty == &Type::Float {
                0x0b
            } else {
                0x03
            });
        }
        invoke.bytes.push(0x2a);
        let callable = if function.private {
            format!("access$suspend${index}")
        } else {
            function.jvm_name().to_owned()
        };
        invoke.reference(
            if member { 0xb6 } else { 0xb8 },
            pool.method(owner, &callable, &function.descriptor())?,
        );
        invoke.bytes.push(0xb0);
        let invoke = simple_method(
            &mut pool,
            0x0011,
            "invokeSuspend",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            invoke,
            None,
            false,
        )?;
        let source_name = pool.utf8("SourceFile")?;
        let source_file = pool.utf8(file)?;
        let metadata = metadata_attribute(
            &mut pool,
            &crate::metadata::Metadata {
                kind: 3,
                value_classes: Default::default(),
                data1: Vec::new(),
                data2: Vec::new(),
                raw_data2: Default::default(),
            },
        )?;
        let mut bytes = Vec::new();
        u4(&mut bytes, 0xcafebabe);
        u2(&mut bytes, 0);
        u2(&mut bytes, 49);
        pool.write(&mut bytes);
        u2(&mut bytes, 0x0030);
        u2(&mut bytes, this);
        u2(&mut bytes, parent);
        u2(&mut bytes, 0);
        u2(
            &mut bytes,
            (plan.fields.len() + 2 + super::suspend_regions::field_count(&function.body)) as u16,
        );
        bytes.extend(fields);
        u2(&mut bytes, 2);
        bytes.extend(constructor);
        bytes.extend(invoke);
        u2(&mut bytes, 2);
        u2(&mut bytes, source_name);
        u4(&mut bytes, 2);
        u2(&mut bytes, source_file);
        bytes.extend(metadata);
        files.push(crate::ClassFile {
            internal_name: frame_owner,
            bytes,
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_carriers_reject_parameters_callbacks_and_saved_locals() {
        let classpath = crate::classpath::Classpath::new(&[]).unwrap();
        classpath
            .source_header(&crate::syntax::parse("class Token").unwrap(), "Token")
            .unwrap();
        for source in [
            "suspend fun blocked(value: Token): Token = value",
            "suspend fun blocked(step: suspend () -> Token): String { val result = step(); return \"x\" }",
            "suspend fun blocked(step: suspend () -> String): String { val kept = Token(); val result = step(); return result }",
        ] {
            let mut syntax = crate::syntax::parse(source).unwrap();
            syntax.imports.push(("Token".into(), "Token".into(), 0));
            let checked = crate::check::check_with_owner(&syntax, &classpath, "BlockedKt").unwrap();
            let error = validate(
                &checked,
                &syntax,
                &std::collections::BTreeSet::from(["Token".into()]),
            )
            .unwrap_err();
            assert_eq!(error.kind, crate::DiagnosticKind::UnsupportedLanguage);
            assert_eq!(error.code, "SUSPEND_VALUE_CLASS_INTERACTION_UNSUPPORTED");
        }
    }
}
