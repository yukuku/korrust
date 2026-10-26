use super::*;

pub(crate) struct CheckedEnum {
    pub class: ClassDecl,
    pub entries: Vec<(String, Value)>,
    pub initializer_locals: u16,
    pub lambdas: Vec<LambdaDecl>,
}

/// Check enum user declarations with the ordinary class/expression machinery.
/// The two JVM-only constructor parameters are inserted before checking so all
/// constructor slots, default expressions, and initialized properties agree.
pub(crate) fn check_enum(
    module: &syntax::Module,
    classpath: &Classpath,
    owner: &str,
) -> Result<CheckedEnum> {
    let source = module
        .enum_decl
        .as_ref()
        .ok_or_else(|| Diagnostic::new(0, "Missing enum declaration"))?;
    if source.entries.len() > 1024 {
        return Err(Diagnostic::new(
            source.offset,
            "Enum exceeds the current 1024-entry limit",
        ));
    }
    if source.entries.len() != source.entry_arguments.len() {
        return Err(Diagnostic::new(
            source.offset,
            "Enum entry arguments are inconsistent",
        ));
    }
    let mut imports = module
        .imports
        .iter()
        .filter(|(alias, _, _)| alias != "*")
        .map(|(alias, path, _)| (alias.clone(), path.clone()))
        .collect::<HashMap<_, _>>();
    let mut classes = HashMap::new();
    for (alias, path, _) in &module.imports {
        if alias == "*" {
            for class in classpath.package_classes(path.trim_end_matches("/*"))? {
                imports
                    .entry(class.name.rsplit('/').next().unwrap().into())
                    .or_insert_with(|| class.name.clone());
                classes.insert(class.name.clone(), class);
            }
        } else if let Ok(class) = classpath.load(path) {
            classes.insert(path.clone(), class);
        } else if let Some((package, name)) = path.rsplit_once('/') {
            for class in classpath.package_classes(package)? {
                if class.methods.iter().any(|method| {
                    method
                        .library
                        .as_ref()
                        .is_some_and(|library| library.source_name == name)
                        || method.source_property.as_deref() == Some(name)
                }) {
                    imports.insert(alias.clone(), format!("{}::{name}", class.name));
                    classes.insert(class.name.clone(), class);
                }
            }
        }
    }
    imports.insert(source.name.clone(), owner.into());
    classes.insert(owner.into(), classpath.load(owner)?);
    for (entry, _) in &source.entries {
        imports.insert(entry.clone(), format!("{owner}::{entry}"));
    }
    imports.insert("entries".into(), format!("{owner}::entries"));
    let mut body = source.body.clone();
    body.params.splice(
        0..0,
        [
            ("<enum-name>".into(), Type::String),
            ("<enum-ordinal>".into(), Type::Int),
        ],
    );
    body.defaults.splice(0..0, [None, None]);
    body.param_annotations
        .splice(0..0, [Vec::new(), Vec::new()]);
    body.super_class = Some((
        "java/lang/Enum".into(),
        vec![
            Expr {
                offset: source.offset,
                kind: ExprKind::Name("<enum-name>".into()),
            },
            Expr {
                offset: source.offset,
                kind: ExprKind::Name("<enum-ordinal>".into()),
            },
        ],
    ));
    let lambdas = Rc::new(RefCell::new(Vec::new()));
    let mut class = declarations::check_class(
        &body,
        &module.package,
        classpath,
        &imports,
        &classes,
        &HashMap::new(),
        Some(owner),
        &lambdas,
    )?;
    let enum_base = classpath.load("java/lang/Enum")?;
    // Kotlin Enum exposes these final functions; other JVM Enum/Object
    // implementation methods are not additional Kotlin declaration symbols.
    for inherited in classpath
        .methods("java/lang/Enum")?
        .into_iter()
        .filter(|method| {
            method.access & 0x18 == 0x10
                && matches!(method.name.as_str(), "compareTo" | "equals" | "hashCode")
        })
    {
        let inherited = relations::specialize_receiver(
            inherited,
            &Type::Java(owner.into()),
            &enum_base,
            classpath,
        );
        let (mut parameters, _) = class_method_types(&inherited)?;
        if inherited.name == "equals" {
            parameters = vec![Type::Nullable(Box::new(Type::Java(
                "java/lang/Object".into(),
            )))];
        }
        for method in &class.methods {
            if method.name == inherited.name
                && method.params.len() == parameters.len()
                && method
                    .params
                    .iter()
                    .zip(&parameters)
                    .all(|((_, actual), expected)| actual == expected.base())
            {
                return Err(Diagnostic::new(
                    source
                        .body
                        .methods
                        .iter()
                        .find(|declaration| declaration.function.name == method.name)
                        .map_or(source.offset, |declaration| declaration.function.offset),
                    format!(
                        "Enum member cannot override final inherited method: {}",
                        method.name
                    ),
                ));
            }
        }
    }
    for field in &class.fields {
        if matches!(field.name.as_str(), "name" | "ordinal") {
            return Err(Diagnostic::new(
                source.offset,
                format!(
                    "Enum property cannot hide final inherited property: {}",
                    field.name
                ),
            ));
        }
    }
    let generated_methods = [
        ("values", format!("()[L{owner};")),
        ("valueOf", format!("(Ljava/lang/String;)L{owner};")),
        ("getEntries", "()Lkotlin/enums/EnumEntries;".into()),
        ("$values", format!("()[L{owner};")),
    ];
    for function in &class.methods {
        if generated_methods.iter().any(|(name, descriptor)| {
            function.name == *name && function.descriptor() == *descriptor
        }) {
            return Err(Diagnostic::new(
                source.offset,
                format!(
                    "Enum member conflicts with generated JVM method: {}",
                    function.name
                ),
            ));
        }
    }
    for field in &class.fields {
        if source.entries.iter().any(|(name, _)| *name == field.name)
            || matches!(field.name.as_str(), "$VALUES" | "$ENTRIES")
        {
            return Err(Diagnostic::new(
                source.offset,
                format!(
                    "Enum property conflicts with generated field: {}",
                    field.name
                ),
            ));
        }
    }
    class.super_class = "java/lang/Enum".into();
    class.super_args = vec![
        Value {
            ty: Type::String,
            kind: ValueKind::Local(1),
        },
        Value {
            ty: Type::Int,
            kind: ValueKind::Local(2),
        },
    ];
    let companion_names = source
        .body
        .companion
        .as_ref()
        .map(|companion| {
            companion
                .methods
                .iter()
                .map(|method| method.function.name.clone())
                .chain(
                    companion
                        .properties
                        .iter()
                        .map(|property| property.name.clone()),
                )
                .chain(std::iter::once("Companion".into()))
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let context =
        declarations::DeclarationContext::new(owner, classpath, &imports, &classes, &lambdas);
    let mut entries = Vec::new();
    let mut initializer_locals = 0;
    for (ordinal, ((name, offset), arguments)) in source
        .entries
        .iter()
        .zip(&source.entry_arguments)
        .enumerate()
    {
        let uninitialized = source.entries[ordinal..]
            .iter()
            .map(|(name, _)| name.clone())
            .chain(companion_names.iter().cloned())
            .collect::<HashSet<_>>();
        let mut arguments = arguments.clone();
        arguments.splice(
            0..0,
            [
                Expr {
                    offset: *offset,
                    kind: ExprKind::String(name.encode_utf16().collect()),
                },
                Expr {
                    offset: *offset,
                    kind: ExprKind::Int(ordinal as i64),
                },
            ],
        );
        let mut checker = context.checker(false, 0, Type::Unit);
        checker.allow_return = false;
        let value = checker.construct_value(
            owner,
            &class.params,
            &vec![false; class.params.len()],
            &arguments,
            true,
            *offset,
        )?;
        if let Some(dependency) =
            uninitialized_read(&value, owner, &uninitialized, &lambdas.borrow())
        {
            return Err(Diagnostic::new(
                source.entry_arguments[ordinal]
                    .first()
                    .map_or(*offset, |argument| argument.offset),
                format!("Enum declaration {dependency} is uninitialized in this entry argument"),
            ));
        }
        initializer_locals = initializer_locals.max(checker.next);
        entries.push((name.clone(), value));
    }
    drop(context);
    Ok(CheckedEnum {
        class,
        entries,
        initializer_locals,
        lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
    })
}

/// Kotlin's predeclared Enum properties map to Enum.name()/ordinal() on JVM.
/// Apply this language mapping only to actual enum declarations; ordinary Java
/// methods with these names retain ordinary function-call semantics.
pub(super) fn property(
    checker: &mut Checker<'_>,
    receiver: &Value,
    name: &str,
    offset: usize,
) -> Result<Option<Value>> {
    let (descriptor, result) = match name {
        "name" => ("()Ljava/lang/String;", Type::String),
        "ordinal" => ("()I", Type::Int),
        _ => return Ok(None),
    };
    let Some(owner) = library::type_owner(&receiver.ty) else {
        return Ok(None);
    };
    if !checker
        .classpath
        .load(owner)
        .is_ok_and(|class| class.access & 0x4000 != 0)
    {
        return Ok(None);
    }
    let mut method = checker
        .classpath
        .methods("java/lang/Enum")?
        .into_iter()
        .find(|method| method.name == name && method.descriptor == descriptor)
        .ok_or_else(|| Diagnostic::new(offset, "Missing JVM Enum property method"))?;
    method.kotlin_params = Some(Vec::new());
    method.kotlin_result = Some(result);
    checker
        .member_call(method, receiver.clone(), &[], offset)
        .map(Some)
}

fn uninitialized_read<'a>(
    value: &'a Value,
    owner: &str,
    unavailable: &HashSet<String>,
    lambdas: &'a [LambdaDecl],
) -> Option<&'a str> {
    use ValueKind as V;
    let child = |value| uninitialized_read(value, owner, unavailable, lambdas);
    let children = |values: &'a [Value]| values.iter().find_map(child);
    let body =
        |statements: &'a [Statement]| uninitialized_body(statements, owner, unavailable, lambdas);
    match &value.kind {
        V::JavaField {
            owner: field_owner,
            name,
            receiver,
        } => {
            if field_owner == owner && (unavailable.contains(name) || name == "Companion") {
                Some(name)
            } else {
                receiver.as_deref().and_then(child)
            }
        }
        V::StaticField {
            owner: field_owner,
            name,
        }
        | V::CompanionArray {
            owner: field_owner,
            name,
        } => (field_owner == owner && (unavailable.contains(name) || name == "Companion"))
            .then_some(name),
        V::ObjectInstance(class) if class == &format!("{owner}$Companion") => Some("Companion"),
        V::MemberCall { method, args, .. } | V::JavaCall { method, args } => {
            if method.owner == format!("{owner}$Companion") {
                Some("Companion")
            } else if method.owner == owner
                && unavailable.contains(
                    method
                        .library
                        .as_ref()
                        .map_or(method.name.as_str(), |library| library.source_name.as_str()),
                )
            {
                Some(&method.name)
            } else {
                children(args)
            }
        }
        V::CompiledInline { args, .. } => children(args).or_else(|| {
            // Supported inline callback parameters execute in the enum-entry
            // initialization context even when the body invokes them zero times.
            args.iter()
                .find_map(|value| invoked_lambda(value, owner, unavailable, lambdas))
        }),
        V::FunctionInvoke { function, args } => child(function)
            .or_else(|| children(args))
            .or_else(|| invoked_lambda(function, owner, unavailable, lambdas)),
        V::Lambda { captures, .. } => children(captures),
        V::Block(statements, result) => body(statements).or_else(|| child(result)),
        V::Try {
            body: statements,
            result,
            catches,
            finally,
            ..
        } => body(statements)
            .or_else(|| child(result))
            .or_else(|| {
                catches
                    .iter()
                    .find_map(|catch| body(&catch.body).or_else(|| child(&catch.result)))
            })
            .or_else(|| body(finally)),
        V::Abrupt(statements) => body(statements),
        V::ArrayInitialize {
            size, initializer, ..
        } => child(size)
            .or_else(|| child(initializer))
            .or_else(|| invoked_lambda(initializer, owner, unavailable, lambdas)),
        V::Box(value)
        | V::Unbox(value)
        | V::IsNull(value)
        | V::ListSize(value)
        | V::RequireNotNull(value)
        | V::IntToString(value)
        | V::NewIntArray(value)
        | V::Negate(value)
        | V::Not(value)
        | V::Length(value)
        | V::Field(_, value)
        | V::NotNull(value, _)
        | V::NewArray(_, value)
        | V::Convert(_, value)
        | V::ThreadLocalGet(value, _)
        | V::TypeTest { value, .. }
        | V::Cast { value, .. }
        | V::CellIncrement { cell: value, .. }
        | V::CellDecrement { cell: value, .. } => child(value),
        V::ListGet(left, right)
        | V::ThreadLocalSet(left, right)
        | V::CopyOf(left, right)
        | V::StringConcat(left, right)
        | V::Binary(_, left, right)
        | V::Index(left, right)
        | V::Compare(left, right) => child(left).or_else(|| child(right)),
        V::ListSet(first, second, third) | V::If(first, second, third) => child(first)
            .or_else(|| child(second))
            .or_else(|| child(third)),
        V::Let { value, body, .. } => child(value).or_else(|| child(body)),
        V::New { args, .. }
        | V::SuperCall { args, .. }
        | V::StaticCall { args, .. }
        | V::StringFromChars(args)
        | V::CopyInto(args)
        | V::StringCall(_, args)
        | V::Call(_, args)
        | V::DefaultCall(_, args, _)
        | V::ListFactory { elements: args, .. } => children(args),
        _ => None,
    }
}

fn invoked_lambda<'a>(
    value: &'a Value,
    owner: &str,
    unavailable: &HashSet<String>,
    lambdas: &'a [LambdaDecl],
) -> Option<&'a str> {
    match &value.kind {
        ValueKind::Lambda {
            owner: lambda_owner,
            ..
        } => lambdas
            .iter()
            .find(|lambda| &lambda.owner == lambda_owner)
            .and_then(|lambda| {
                uninitialized_body(&lambda.function.body, owner, unavailable, lambdas)
            }),
        ValueKind::Cast { value, .. }
        | ValueKind::Box(value)
        | ValueKind::Unbox(value)
        | ValueKind::NotNull(value, _) => invoked_lambda(value, owner, unavailable, lambdas),
        _ => None,
    }
}

fn uninitialized_body<'a>(
    statements: &'a [Statement],
    owner: &str,
    unavailable: &HashSet<String>,
    lambdas: &'a [LambdaDecl],
) -> Option<&'a str> {
    let value = |value| uninitialized_read(value, owner, unavailable, lambdas);
    let body = |body| uninitialized_body(body, owner, unavailable, lambdas);
    statements.iter().find_map(|statement| match statement {
        Statement::ThrowValue(first)
        | Statement::Store(_, first)
        | Statement::Expression(first)
        | Statement::Return(first)
        | Statement::NonLocalReturn(first) => value(first),
        Statement::Labeled {
            body: statements, ..
        } => body(statements),
        Statement::JavaFieldStore {
            receiver,
            value: stored,
            ..
        } => receiver.as_ref().and_then(value).or_else(|| value(stored)),
        Statement::FieldStore {
            receiver,
            value: stored,
            ..
        } => value(receiver).or_else(|| value(stored)),
        Statement::IndexStore(first, second, third) => value(first)
            .or_else(|| value(second))
            .or_else(|| value(third)),
        Statement::If(condition, yes, no) => {
            value(condition).or_else(|| body(yes)).or_else(|| body(no))
        }
        Statement::While(condition, statements) | Statement::DoWhile(condition, statements) => {
            value(condition).or_else(|| body(statements))
        }
        Statement::ForUntil {
            start,
            end,
            body: statements,
            ..
        } => value(start)
            .or_else(|| value(end))
            .or_else(|| body(statements)),
        Statement::ForIterator {
            iterable,
            body: statements,
            ..
        }
        | Statement::ForEach {
            iterable,
            body: statements,
            ..
        } => value(iterable).or_else(|| body(statements)),
        Statement::ThrowException { message, .. } => message.as_ref().and_then(value),
        _ => None,
    })
}
