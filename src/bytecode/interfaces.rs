use super::*;
use crate::check::{CompanionDecl, CompanionValue, InterfaceDecl};
use crate::metadata::Metadata;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone)]
pub(super) struct Relation {
    owner: String,
    outer: Option<String>,
    name: Option<String>,
    flags: u16,
}

impl Relation {
    fn member(owner: &str, outer: &str, flags: u16) -> Self {
        Self {
            owner: owner.into(),
            outer: Some(outer.into()),
            name: Some(owner.rsplit('$').next().unwrap().into()),
            flags,
        }
    }
    fn anonymous(owner: &str) -> Self {
        Self {
            owner: owner.into(),
            outer: None,
            name: None,
            flags: 0x0019,
        }
    }
}

fn inner_attribute(pool: &mut Pool, relations: &[Relation]) -> Result<Option<Vec<u8>>> {
    if relations.is_empty() {
        return Ok(None);
    }
    let mut relations = relations.to_vec();
    relations.sort_by(|a, b| a.owner.cmp(&b.owner));
    relations.dedup_by(|a, b| a.owner == b.owner);
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("InnerClasses")?);
    u4(&mut attribute, (2 + 8 * relations.len()) as u32);
    u2(
        &mut attribute,
        u16::try_from(relations.len()).map_err(|_| failure("Too many inner classes"))?,
    );
    for relation in relations {
        u2(&mut attribute, pool.class(&relation.owner)?);
        u2(
            &mut attribute,
            match relation.outer {
                Some(outer) => pool.class(&outer)?,
                None => 0,
            },
        );
        u2(
            &mut attribute,
            match relation.name {
                Some(name) => pool.utf8(&name)?,
                None => 0,
            },
        );
        u2(&mut attribute, relation.flags);
    }
    Ok(Some(attribute))
}

fn not_null(pool: &mut Pool) -> Result<Vec<u8>> {
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("RuntimeInvisibleAnnotations")?);
    u4(&mut attribute, 6);
    u2(&mut attribute, 1);
    u2(
        &mut attribute,
        pool.utf8("Lorg/jetbrains/annotations/NotNull;")?,
    );
    u2(&mut attribute, 0);
    Ok(attribute)
}

fn deprecated(pool: &mut Pool, method: &mut Vec<u8>) -> Result<()> {
    let count = u16::from_be_bytes([method[6], method[7]]) + 2;
    method[6..8].copy_from_slice(&count.to_be_bytes());
    u2(method, pool.utf8("Deprecated")?);
    u4(method, 0);
    u2(method, pool.utf8("RuntimeVisibleAnnotations")?);
    u4(method, 6);
    u2(method, 1);
    u2(method, pool.utf8("Ljava/lang/Deprecated;")?);
    u2(method, 0);
    Ok(())
}

pub(super) struct Builder {
    owner: String,
    version: u16,
    flags: u16,
    pub(super) pool: Pool,
    pub(super) class: u16,
    parent: u16,
    interfaces: Vec<u16>,
    pub(super) fields: Vec<Vec<u8>>,
    pub(super) methods: Vec<Vec<u8>>,
    pub(super) attributes: Vec<Vec<u8>>,
    pub(super) annotations: Vec<String>,
}

impl Builder {
    pub(super) fn new(
        owner: &str,
        version: u16,
        flags: u16,
        interfaces: &[String],
    ) -> Result<Self> {
        let mut pool = Pool::default();
        let class = pool.class(owner)?;
        let parent = pool.class("java/lang/Object")?;
        let interfaces = interfaces
            .iter()
            .map(|name| pool.class(name))
            .collect::<Result<_>>()?;
        Ok(Self {
            owner: owner.into(),
            version,
            flags,
            pool,
            class,
            parent,
            interfaces,
            fields: Vec::new(),
            methods: Vec::new(),
            attributes: Vec::new(),
            annotations: Vec::new(),
        })
    }
    pub(super) fn field(
        &mut self,
        name: &str,
        descriptor: &str,
        flags: u16,
        annotated: bool,
    ) -> Result<()> {
        let mut field = Vec::new();
        u2(&mut field, flags);
        u2(&mut field, self.pool.utf8(name)?);
        u2(&mut field, self.pool.utf8(descriptor)?);
        u2(&mut field, u16::from(annotated));
        if annotated {
            field.extend(not_null(&mut self.pool)?);
        }
        self.fields.push(field);
        Ok(())
    }
    pub(super) fn finish(
        mut self,
        file: &str,
        metadata: &Metadata,
        relations: &[Relation],
    ) -> Result<crate::ClassFile> {
        let mut attributes = std::mem::take(&mut self.attributes);
        if let Some(inner) = inner_attribute(&mut self.pool, relations)? {
            attributes.push(inner);
        }
        let mut source = Vec::new();
        u2(&mut source, self.pool.utf8("SourceFile")?);
        u4(&mut source, 2);
        u2(&mut source, self.pool.utf8(file)?);
        attributes.push(source);
        attributes.push(metadata_attribute_with_annotations(
            &mut self.pool,
            metadata,
            &self.annotations,
        )?);
        let mut bytes = Vec::new();
        u4(&mut bytes, 0xcafebabe);
        u2(&mut bytes, 0);
        u2(&mut bytes, self.version);
        defaults::unique_methods(&self.pool, &self.methods)?;
        self.pool.write(&mut bytes);
        u2(&mut bytes, self.flags);
        u2(&mut bytes, self.class);
        u2(&mut bytes, self.parent);
        u2(
            &mut bytes,
            u16::try_from(self.interfaces.len()).map_err(|_| failure("Too many interfaces"))?,
        );
        for interface in self.interfaces {
            u2(&mut bytes, interface);
        }
        u2(
            &mut bytes,
            u16::try_from(self.fields.len()).map_err(|_| failure("Too many JVM fields"))?,
        );
        for field in self.fields {
            bytes.extend(field);
        }
        u2(
            &mut bytes,
            u16::try_from(self.methods.len()).map_err(|_| failure("Too many JVM methods"))?,
        );
        for method in self.methods {
            bytes.extend(method);
        }
        u2(&mut bytes, attributes.len() as u16);
        for attribute in attributes {
            bytes.extend(attribute);
        }
        Ok(crate::ClassFile {
            internal_name: self.owner,
            bytes,
        })
    }
}

fn metadata<'a>(all: &'a BTreeMap<String, Metadata>, owner: &str) -> Result<&'a Metadata> {
    all.get(owner)
        .ok_or_else(|| failure("Missing generated class metadata"))
}

fn static_descriptor(owner: &str, descriptor: &str) -> String {
    format!("(L{owner};{}", &descriptor[1..])
}

fn arguments(code: &mut SimpleCode, params: &[(String, Type)]) {
    code.bytes.push(0x2a);
    for (slot, (_, ty)) in parameter_slots(params, 1).into_iter().zip(params) {
        code.bytes.extend([load_opcode(ty), slot as u8]);
    }
}

fn parameter_checks(
    pool: &mut Pool,
    code: &mut SimpleCode,
    params: &[(String, Type)],
) -> Result<()> {
    for (slot, (name, ty)) in parameter_slots(params, 1).into_iter().zip(params) {
        if nonnull(ty) {
            code.bytes.extend([0x19, slot as u8]);
            code.reference(0x13, pool.string(name)?);
            code.reference(
                0xb8,
                pool.method(
                    "kotlin/jvm/internal/Intrinsics",
                    "checkNotNullParameter",
                    "(Ljava/lang/Object;Ljava/lang/String;)V",
                )?,
            );
        }
    }
    Ok(())
}

fn abstract_method(pool: &mut Pool, function: &crate::check::Function) -> Result<Vec<u8>> {
    let mut method = Vec::new();
    u2(&mut method, 0x0401);
    u2(&mut method, pool.utf8(function.jvm_name())?);
    u2(&mut method, pool.utf8(&function.descriptor())?);
    let signature = generic_function(function);
    u2(
        &mut method,
        u16::from(nonnull(&function.result)) + u16::from(signature.is_some()),
    );
    if let Some(signature) = signature {
        u2(&mut method, pool.utf8("Signature")?);
        u4(&mut method, 2);
        u2(&mut method, pool.utf8(&signature)?);
    }
    if nonnull(&function.result) {
        method.extend(not_null(pool)?);
    }
    nullable_result(pool, &mut method, &function.result)?;
    parameter_annotations(pool, &mut method, &function.params)?;
    Ok(method)
}

fn default_forward(
    pool: &mut Pool,
    interface: &InterfaceDecl,
    function: &crate::check::Function,
) -> Result<Vec<u8>> {
    let mut code = SimpleCode::new(
        (function
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>()
            + 1)
        .max(2) as u16,
        (function
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>()
            + 1) as u16,
    );
    parameter_checks(pool, &mut code, &function.params)?;
    arguments(&mut code, &function.params);
    code.reference(
        0xb8,
        pool.method(
            &format!("{}$DefaultImpls", interface.owner),
            function.jvm_name(),
            &static_descriptor(&interface.owner, &function.descriptor()),
        )?,
    );
    code.bytes.push(return_opcode(&function.result));
    let mut method = simple_method(
        pool,
        0x0001,
        function.jvm_name(),
        &function.descriptor(),
        code,
        generic_function(function).as_deref(),
        nonnull(&function.result),
    )?;
    parameter_annotations(pool, &mut method, &function.params)?;
    nullable_result(pool, &mut method, &function.result)?;
    Ok(method)
}

fn default_access(
    pool: &mut Pool,
    owner: &str,
    name: &str,
    descriptor: &str,
    params: &[(String, Type)],
    result: &Type,
) -> Result<Vec<u8>> {
    let mut code = SimpleCode::new(
        (params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() + 1).max(2) as u16,
        (params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() + 1) as u16,
    );
    arguments(&mut code, params);
    code.reference(0xb7, pool.interface_method(owner, name, descriptor)?);
    code.bytes.push(return_opcode(result));
    simple_method(
        pool,
        0x1009,
        &format!("access${name}$jd"),
        &static_descriptor(owner, descriptor),
        code,
        None,
        false,
    )
}

fn helper_prefix(
    pool: &mut Pool,
    interface: &InterfaceDecl,
    params: &[(String, Type)],
) -> Result<SimpleCode> {
    let mut code = SimpleCode::new(
        2,
        (params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() + 1) as u16,
    );
    if let Some(companion) = &interface.companion {
        code.reference(
            0xb2,
            pool.field(
                &interface.owner,
                "Companion",
                &format!("L{};", companion.owner),
            )?,
        );
        code.bytes.push(0x57);
    }
    code.bytes.push(0x2a);
    code.reference(
        0xb6,
        pool.method("java/lang/Object", "getClass", "()Ljava/lang/Class;")?,
    );
    code.bytes.push(0x57);
    Ok(code)
}

fn helper_body(
    builder: &mut Builder,
    interface: &InterfaceDecl,
    function: &crate::check::Function,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<()> {
    let mut prefix = helper_prefix(&mut builder.pool, interface, &function.params)?;
    parameter_checks(&mut builder.pool, &mut prefix, &function.params)?;
    let mut code = Code {
        pool: &mut builder.pool,
        functions: &[],
        owner: &interface.owner,
        bytes: prefix.bytes,
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
        max_stack: 2,
    };
    code.block(&function.body)?;
    let max_stack =
        u16::try_from(code.max_stack).map_err(|_| failure("JVM operand stack limit exceeded"))?;
    let completed = code.into_simple(function.locals, max_stack)?;
    let code = completed;
    let mut method = simple_method(
        &mut builder.pool,
        0x0009,
        function.jvm_name(),
        &static_descriptor(&interface.owner, &function.descriptor()),
        code,
        generic_function(function)
            .map(|signature| signature.replacen('(', &format!("(L{};", interface.owner), 1))
            .as_deref(),
        function.result.is_reference(),
    )?;
    let mut params = vec![("$this".into(), Type::Java(interface.owner.clone()))];
    params.extend(function.params.clone());
    parameter_annotations(&mut builder.pool, &mut method, &params)?;
    deprecated(&mut builder.pool, &mut method)?;
    builder.methods.push(method);
    Ok(())
}

fn helper_inherited(
    builder: &mut Builder,
    interface: &InterfaceDecl,
    inherited: &crate::classpath::JavaMethod,
) -> Result<()> {
    let (types, result) = crate::check::class_method_types(inherited)?;
    let params: Vec<_> = types.into_iter().map(|ty| (String::new(), ty)).collect();
    let mut code = helper_prefix(&mut builder.pool, interface, &params)?;
    code.max_stack = code
        .max_stack
        .max((params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() + 1) as u16);
    arguments(&mut code, &params);
    code.reference(
        0xb8,
        builder.pool.method(
            &format!("{}$DefaultImpls", inherited.owner),
            &inherited.name,
            &static_descriptor(&inherited.owner, &inherited.descriptor),
        )?,
    );
    code.bytes.push(return_opcode(&result));
    let mut method = simple_method(
        &mut builder.pool,
        0x0009,
        &inherited.name,
        &static_descriptor(&interface.owner, &inherited.descriptor),
        code,
        None,
        result.is_reference(),
    )?;
    let mut annotated = vec![("$this".into(), Type::Java(interface.owner.clone()))];
    annotated.extend(params);
    parameter_annotations(&mut builder.pool, &mut method, &annotated)?;
    deprecated(&mut builder.pool, &mut method)?;
    builder.methods.push(method);
    Ok(())
}

fn collect_value(value: &Value, accesses: &mut HashSet<(String, String)>) {
    match &value.kind {
        ValueKind::Block(body, result) => {
            collect_block(body, accesses);
            collect_value(result, accesses);
        }
        ValueKind::Try {
            body,
            result,
            catches,
            finally,
            ..
        } => {
            collect_block(body, accesses);
            collect_value(result, accesses);
            collect_block(finally, accesses);
            for catch in catches {
                collect_block(&catch.body, accesses);
                collect_value(&catch.result, accesses);
            }
        }
        ValueKind::ArrayInitialize {
            size, initializer, ..
        } => {
            collect_value(size, accesses);
            collect_value(initializer, accesses);
        }
        ValueKind::ListSet(receiver, index, value) => {
            collect_value(receiver, accesses);
            collect_value(index, accesses);
            collect_value(value, accesses);
        }
        ValueKind::Box(value)
        | ValueKind::Unbox(value)
        | ValueKind::CellIncrement { cell: value, .. }
        | ValueKind::CellDecrement { cell: value, .. }
        | ValueKind::TypeTest { value, .. }
        | ValueKind::Cast { value, .. } => collect_value(value, accesses),
        ValueKind::Abrupt(body) => collect_block(body, accesses),
        ValueKind::SuperCall { args, .. }
        | ValueKind::New { args, .. }
        | ValueKind::CompiledInline { args, .. }
        | ValueKind::Lambda { captures: args, .. }
        | ValueKind::DefaultCall(_, args, _)
        | ValueKind::ListFactory { elements: args, .. } => {
            for value in args {
                collect_value(value, accesses);
            }
        }
        ValueKind::JavaField { receiver, .. } => {
            if let Some(value) = receiver {
                collect_value(value, accesses);
            }
        }
        ValueKind::Let { value, body, .. } => {
            collect_value(value, accesses);
            collect_value(body, accesses);
        }
        ValueKind::FunctionInvoke { function, args } => {
            collect_value(function, accesses);
            for arg in args {
                collect_value(arg, accesses);
            }
        }
        ValueKind::NewArray(_, value)
        | ValueKind::Convert(_, value)
        | ValueKind::ThreadLocalGet(value, _)
        | ValueKind::RequireNotNull(value)
        | ValueKind::IsNull(value)
        | ValueKind::ListSize(value) => collect_value(value, accesses),
        ValueKind::ThreadLocalSet(a, b) | ValueKind::CopyOf(a, b) => {
            collect_value(a, accesses);
            collect_value(b, accesses);
        }
        ValueKind::StaticCall { args, .. } | ValueKind::StringFromChars(args) => {
            for value in args {
                collect_value(value, accesses);
            }
        }
        ValueKind::StaticField { .. }
        | ValueKind::ObjectInstance(_)
        | ValueKind::Increment { .. }
        | ValueKind::Decrement { .. } => {}
        ValueKind::CompanionArray { owner, name } => {
            accesses.insert((owner.clone(), name.clone()));
        }
        ValueKind::NewIntArray(inner)
        | ValueKind::NotNull(inner, _)
        | ValueKind::Field(_, inner)
        | ValueKind::Negate(inner)
        | ValueKind::Not(inner)
        | ValueKind::Length(inner)
        | ValueKind::IntToString(inner) => collect_value(inner, accesses),
        ValueKind::StringConcat(a, b)
        | ValueKind::Binary(_, a, b)
        | ValueKind::Index(a, b)
        | ValueKind::Compare(a, b)
        | ValueKind::ListGet(a, b) => {
            collect_value(a, accesses);
            collect_value(b, accesses);
        }
        ValueKind::If(condition, yes, no) => {
            collect_value(condition, accesses);
            collect_value(yes, accesses);
            collect_value(no, accesses);
        }
        ValueKind::JavaCall { args, .. }
        | ValueKind::CopyInto(args)
        | ValueKind::StringCall(_, args)
        | ValueKind::Call(_, args)
        | ValueKind::MemberCall { args, .. } => {
            for value in args {
                collect_value(value, accesses);
            }
        }
        ValueKind::ClassLiteral(_)
        | ValueKind::Unit
        | ValueKind::Float(_)
        | ValueKind::Double(_)
        | ValueKind::Int(_)
        | ValueKind::Long(_)
        | ValueKind::Null
        | ValueKind::Boolean(_)
        | ValueKind::String(_)
        | ValueKind::Local(_) => {}
    }
}
fn collect_block(body: &[Statement], accesses: &mut HashSet<(String, String)>) {
    for statement in body {
        match statement {
            Statement::JavaFieldStore {
                receiver, value, ..
            } => {
                if let Some(receiver) = receiver {
                    collect_value(receiver, accesses);
                }
                collect_value(value, accesses);
            }
            Statement::FieldStore {
                receiver, value, ..
            } => {
                collect_value(receiver, accesses);
                collect_value(value, accesses);
            }
            Statement::ThrowException { message, .. } => {
                if let Some(value) = message {
                    collect_value(value, accesses);
                }
            }
            Statement::IndexStore(array, index, value) => {
                collect_value(array, accesses);
                collect_value(index, accesses);
                collect_value(value, accesses);
            }
            Statement::Continue
            | Statement::Break
            | Statement::ContinueLabel(_)
            | Statement::BreakLabel(_) => {}
            Statement::Labeled { body, .. } => collect_block(body, accesses),
            Statement::ForUntil {
                start, end, body, ..
            } => {
                collect_value(start, accesses);
                collect_value(end, accesses);
                collect_block(body, accesses);
            }
            Statement::ForEach { iterable, body, .. }
            | Statement::ForIterator { iterable, body, .. } => {
                collect_value(iterable, accesses);
                collect_block(body, accesses);
            }
            Statement::Store(_, value)
            | Statement::Expression(value)
            | Statement::Return(value)
            | Statement::NonLocalReturn(value) => collect_value(value, accesses),
            Statement::ThrowValue(value) => collect_value(value, accesses),
            Statement::If(condition, yes, no) => {
                collect_value(condition, accesses);
                collect_block(yes, accesses);
                collect_block(no, accesses);
            }
            Statement::While(condition, body) | Statement::DoWhile(condition, body) => {
                collect_value(condition, accesses);
                collect_block(body, accesses);
            }
        }
    }
}
fn collect_accesses(interface: &InterfaceDecl, accesses: &mut HashSet<(String, String)>) {
    for method in &interface.methods {
        collect_block(&method.function.body, accesses);
    }
    for nested in &interface.nested {
        collect_accesses(nested, accesses);
    }
    if let Some(companion) = &interface.companion {
        for property in &companion.properties {
            if let CompanionValue::Object { class_decl, .. } = &property.value {
                for method in &class_decl.methods {
                    collect_block(&method.body, accesses);
                }
            }
        }
    }
}

struct EmitContext<'a> {
    file: &'a str,
    all: &'a BTreeMap<String, Metadata>,
    lambdas: &'a [crate::check::LambdaDecl],
}

fn companion_classes(
    companion: &CompanionDecl,
    interface: &InterfaceDecl,
    context: &EmitContext<'_>,
    ancestors: &[Relation],
    accesses: &HashSet<(String, String)>,
    output: &mut Vec<crate::ClassFile>,
) -> Result<()> {
    let (file, all, lambdas) = (context.file, context.all, context.lambdas);
    let mut builder = Builder::new(&companion.owner, 52, 0x0031, &[])?;
    let singleton_descriptor = format!("L{};", companion.owner);
    builder.field("$$INSTANCE", &singleton_descriptor, 0x1018, false)?;
    for property in &companion.properties {
        builder.field(&property.name, &property.ty.descriptor(), 0x001a, true)?;
    }
    if let Some(state) = &companion.state {
        for property in &state.properties {
            builder.fields.push(class_field(
                &mut builder.pool,
                &property.name,
                &property.ty,
                0x0008
                    | state
                        .exposed_fields
                        .get(&property.name)
                        .copied()
                        .unwrap_or(0x0002)
                    | if property.mutable { 0 } else { 0x0010 },
            )?);
            if !lambdas.is_empty() {
                builder.methods.extend(field_bridges(
                    &mut builder.pool,
                    &companion.owner,
                    &property.name,
                    &property.ty,
                    true,
                    property.mutable,
                )?);
            }
        }
        for function in &state.methods {
            if !lambdas.is_empty() && function.private {
                builder.methods.push(private_method_bridge(
                    &mut builder.pool,
                    &companion.owner,
                    function,
                    false,
                )?);
            }
            builder.methods.push(checked_method(
                &mut builder.pool,
                &companion.owner,
                &state.methods,
                function,
                lambdas,
                if function.private { 0x0012 } else { 0x0011 },
            )?);
        }
    }
    let mut constructor = SimpleCode::new(1, 1);
    constructor.bytes.push(0x2a);
    constructor.reference(
        0xb7,
        builder.pool.method("java/lang/Object", "<init>", "()V")?,
    );
    constructor.bytes.push(0xb1);
    builder.methods.push(simple_method(
        &mut builder.pool,
        0x0002,
        "<init>",
        "()V",
        constructor,
        None,
        false,
    )?);
    for property in &companion.properties {
        if !property.private {
            let descriptor = format!("(){}", property.ty.descriptor());
            let mut code = SimpleCode::new(1, 1);
            code.reference(
                0xb2,
                builder
                    .pool
                    .field(&companion.owner, &property.name, &property.ty.descriptor())?,
            );
            code.bytes.push(return_opcode(&property.ty));
            builder.methods.push(simple_method(
                &mut builder.pool,
                0x0011,
                &crate::check::getter_name(&property.name),
                &descriptor,
                code,
                None,
                property.ty.is_reference(),
            )?);
        }
    }
    for property in &companion.properties {
        if property.private && accesses.contains(&(companion.owner.clone(), property.name.clone()))
        {
            let mut code = SimpleCode::new(1, 0);
            code.reference(
                0xb2,
                builder
                    .pool
                    .field(&companion.owner, &property.name, &property.ty.descriptor())?,
            );
            code.bytes.push(return_opcode(&property.ty));
            builder.methods.push(simple_method(
                &mut builder.pool,
                0x1019,
                &format!("access$get{}$p", property.name),
                &format!("(){}", property.ty.descriptor()),
                code,
                None,
                false,
            )?);
        }
    }
    let mut initializer = SimpleCode::new(4, 0);
    initializer.reference(0xbb, builder.class);
    initializer.bytes.push(0x59);
    initializer.reference(
        0xb7,
        builder.pool.method(&companion.owner, "<init>", "()V")?,
    );
    initializer.reference(
        0xb3,
        builder
            .pool
            .field(&companion.owner, "$$INSTANCE", &singleton_descriptor)?,
    );
    for property in &companion.properties {
        match &property.value {
            CompanionValue::StringArray(values) => {
                initializer.integer(values.len())?;
                initializer.reference(0xbd, builder.pool.class("java/lang/String")?);
                for (index, value) in values.iter().enumerate() {
                    initializer.bytes.push(0x59);
                    initializer.integer(index)?;
                    initializer.reference(0x13, builder.pool.string_units(value)?);
                    initializer.bytes.push(0x53);
                }
            }
            CompanionValue::Object { owner, .. } => {
                initializer.reference(0xbb, builder.pool.class(owner)?);
                initializer.bytes.push(0x59);
                initializer.reference(0xb7, builder.pool.method(owner, "<init>", "()V")?);
            }
        }
        initializer.reference(
            0xb3,
            builder
                .pool
                .field(&companion.owner, &property.name, &property.ty.descriptor())?,
        );
    }
    let mut code = Code {
        pool: &mut builder.pool,
        functions: &[],
        owner: &companion.owner,
        bytes: initializer.bytes,
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
        max_stack: 4,
    };
    let locals = companion.state.as_ref().map_or(0, |state| state.locals);
    if let Some(state) = &companion.state {
        if state.initializers.is_empty() {
            for property in &state.properties {
                code.value(&property.value)?;
                code.field_reference(
                    0xb3,
                    &companion.owner,
                    &property.name,
                    &property.ty.descriptor(),
                )?;
            }
        } else {
            code.block(&state.initializers)?;
        }
    }
    code.bytes.push(0xb1);
    let stack = code.max_stack as u16;
    let initializer = code.into_simple(locals, stack)?;
    let mut initializer_method = simple_method(
        &mut builder.pool,
        0x0008,
        "<clinit>",
        "()V",
        initializer,
        None,
        false,
    )?;
    frames::attach(
        &mut builder.pool,
        &mut initializer_method,
        &companion.owner,
        &[],
        locals,
        true,
    )?;
    builder.methods.push(initializer_method);
    let mut relations = ancestors.to_vec();
    relations.push(Relation::member(&companion.owner, &interface.owner, 0x0019));
    for property in &companion.properties {
        if let CompanionValue::Object { owner, .. } = &property.value {
            relations.push(Relation::anonymous(owner));
        }
    }
    output.push(builder.finish(file, metadata(all, &companion.owner)?, &relations)?);
    for property in &companion.properties {
        if let CompanionValue::Object { owner, class_decl } = &property.value {
            let mut relations = ancestors.to_vec();
            relations.push(Relation::member(&companion.owner, &interface.owner, 0x0019));
            relations.push(Relation::anonymous(owner));
            let bytes = emit_class_with(
                class_decl,
                owner,
                file,
                metadata(all, owner)?,
                0,
                lambdas,
                None,
                |pool| {
                    let mut attributes = Vec::new();
                    if let Some(inner) = inner_attribute(pool, &relations)? {
                        attributes.push(inner);
                    }
                    let mut enclosing = Vec::new();
                    u2(&mut enclosing, pool.utf8("EnclosingMethod")?);
                    u4(&mut enclosing, 4);
                    u2(&mut enclosing, pool.class(&companion.owner)?);
                    u2(&mut enclosing, 0);
                    attributes.push(enclosing);
                    Ok(attributes)
                },
            )?;
            output.push(crate::ClassFile {
                internal_name: owner.clone(),
                bytes,
            });
        }
    }
    Ok(())
}

fn interface_classes(
    interface: &InterfaceDecl,
    context: &EmitContext<'_>,
    ancestors: &[Relation],
    accesses: &HashSet<(String, String)>,
    output: &mut Vec<crate::ClassFile>,
) -> Result<()> {
    let (file, all, lambdas) = (context.file, context.all, context.lambdas);
    let mut builder = Builder::new(&interface.owner, 52, 0x0601, &interface.interfaces)?;
    if let Some(signature) = classifier_signature(
        &interface.type_params,
        &interface.super_types,
        "java/lang/Object",
        &interface.interfaces,
    ) {
        builder
            .attributes
            .push(signature_attribute(&mut builder.pool, &signature)?);
    }
    if let Some(companion) = &interface.companion {
        builder.field("Companion", &format!("L{};", companion.owner), 0x0019, true)?;
    }
    for method in &interface.methods {
        if method.source_property.as_ref().is_some_and(|property| {
            crate::check::getter_name(property) != method.function.jvm_name()
                && crate::check::setter_name(property) != method.function.jvm_name()
        }) {
            return Err(failure(
                "Interface accessor lowering does not match its source property",
            ));
        }
        let method = if method.has_body {
            default_forward(&mut builder.pool, interface, &method.function)?
        } else {
            abstract_method(&mut builder.pool, &method.function)?
        };
        builder.methods.push(method);
    }
    let has_defaults =
        interface.methods.iter().any(|method| method.has_body) || !interface.defaults.is_empty();
    for method in interface.methods.iter().filter(|method| method.has_body) {
        let function = &method.function;
        builder.methods.push(default_access(
            &mut builder.pool,
            &interface.owner,
            function.jvm_name(),
            &function.descriptor(),
            &function.params,
            &function.result,
        )?);
    }
    for inherited in &interface.defaults {
        let (types, result) = crate::check::class_method_types(inherited)?;
        let params: Vec<_> = types.into_iter().map(|ty| (String::new(), ty)).collect();
        builder.methods.push(default_access(
            &mut builder.pool,
            &interface.owner,
            &inherited.name,
            &inherited.descriptor,
            &params,
            &result,
        )?);
    }
    if let Some(companion) = &interface.companion {
        let mut code = SimpleCode::new(1, 0);
        code.reference(
            0xb2,
            builder.pool.field(
                &companion.owner,
                "$$INSTANCE",
                &format!("L{};", companion.owner),
            )?,
        );
        code.reference(
            0xb3,
            builder.pool.field(
                &interface.owner,
                "Companion",
                &format!("L{};", companion.owner),
            )?,
        );
        code.bytes.push(0xb1);
        builder.methods.push(simple_method(
            &mut builder.pool,
            0x0008,
            "<clinit>",
            "()V",
            code,
            None,
            false,
        )?);
    }
    let mut relations = ancestors.to_vec();
    for nested in &interface.nested {
        relations.push(Relation::member(&nested.owner, &interface.owner, 0x0609));
    }
    if let Some(companion) = &interface.companion {
        relations.push(Relation::member(&companion.owner, &interface.owner, 0x0019));
    }
    let helper_owner = format!("{}$DefaultImpls", interface.owner);
    if has_defaults {
        relations.push(Relation::member(&helper_owner, &interface.owner, 0x0019));
    }
    output.push(builder.finish(file, metadata(all, &interface.owner)?, &relations)?);
    if has_defaults {
        let mut builder = Builder::new(&helper_owner, 49, 0x0031, &[])?;
        for method in interface.methods.iter().filter(|method| method.has_body) {
            helper_body(&mut builder, interface, &method.function, lambdas)?;
        }
        for inherited in &interface.defaults {
            helper_inherited(&mut builder, interface, inherited)?;
        }
        let mut relations = ancestors.to_vec();
        relations.push(Relation::member(&helper_owner, &interface.owner, 0x0019));
        output.push(builder.finish(
            file,
            &Metadata {
                raw_data2: Default::default(),
                kind: 3,
                data1: Vec::new(),
                data2: Vec::new(),
            },
            &relations,
        )?);
    }
    for nested in &interface.nested {
        let mut ancestors = ancestors.to_vec();
        ancestors.push(Relation::member(&nested.owner, &interface.owner, 0x0609));
        interface_classes(nested, context, &ancestors, accesses, output)?;
    }
    if let Some(companion) = &interface.companion {
        companion_classes(companion, interface, context, ancestors, accesses, output)?;
    }
    Ok(())
}

pub fn emit_interfaces(
    interface: &InterfaceDecl,
    file: &str,
    all: &BTreeMap<String, Metadata>,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<Vec<crate::ClassFile>> {
    let mut accesses = HashSet::new();
    collect_accesses(interface, &mut accesses);
    let mut output = Vec::new();
    interface_classes(
        interface,
        &EmitContext { file, all, lambdas },
        &[],
        &accesses,
        &mut output,
    )?;
    Ok(output)
}
