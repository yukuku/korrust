use super::*;

pub(crate) struct CheckedEnum {
    pub class: ClassDecl,
    pub entries: Vec<(String, Value)>,
    pub initializer_locals: u16,
    pub entry_classes: Vec<CheckedEntry>,
    pub lambdas: Vec<LambdaDecl>,
}

pub(crate) struct CheckedEntry {
    pub owner: String,
    pub source: syntax::ClassDecl,
    pub imports: Vec<(String, String, usize)>,
    pub class: ClassDecl,
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
    if source.body.params.len() > 32 && source.body.defaults.iter().any(Option::is_some) {
        return Err(Diagnostic::new(
            source.offset,
            "Enum constructor defaults support at most 32 parameters",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "ENUM_DEFAULT_PARAMETER_LIMIT_UNSUPPORTED",
        ));
    }
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
    let polymorphic = source.entry_bodies.iter().any(Option::is_some);
    body.abstract_ = polymorphic || body.methods.iter().any(|method| !method.has_body);
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
    let declared_header = classpath.load(owner)?;
    class.abstract_ = !class.abstract_methods.is_empty()
        || (polymorphic
            && classpath.methods(owner)?.iter().any(|method| {
                method.access & 0x0400 != 0
                    && !class.methods.iter().any(|implementation| {
                        implementation.name == method.name
                            && implementation.descriptor() == method.descriptor
                    })
                    && !declared_header.methods.iter().any(|implementation| {
                        implementation.access & 0x0400 == 0
                            && implementation.name == method.name
                            && implementation.descriptor == method.descriptor
                    })
                    && !class.generic_bridges.iter().any(|bridge| {
                        bridge.erased.name == method.name
                            && bridge.erased.descriptor == method.descriptor
                    })
            }));
    class.open = polymorphic;
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
    let mut entry_classes = Vec::new();
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
        let mut checker = context.checker(
            false,
            if source.entry_bodies[ordinal].is_some() {
                3
            } else {
                0
            },
            Type::Unit,
        );
        checker.allow_return = false;
        let defaults = source
            .body
            .defaults
            .iter()
            .map(Option::is_some)
            .collect::<Vec<_>>();
        let (arguments, mut lets, mask) =
            checker.arguments(&class.params[2..], &defaults, arguments, true, *offset)?;
        let dependency_value = Checker::wrap_lets(
            Value {
                ty: Type::Java(owner.into()),
                kind: ValueKind::New {
                    owner: owner.into(),
                    descriptor: String::new(),
                    args: arguments.clone(),
                },
            },
            lets.clone(),
        );
        if let Some(dependency) =
            uninitialized_read(&dependency_value, owner, &uninitialized, &lambdas.borrow())
        {
            return Err(Diagnostic::new(
                source.entry_arguments[ordinal]
                    .first()
                    .map_or(*offset, |argument| argument.offset),
                format!("Enum declaration {dependency} is uninitialized in this entry argument"),
            ));
        }
        let hidden = vec![
            Value {
                ty: Type::String,
                kind: ValueKind::String(name.encode_utf16().collect()),
            },
            Value {
                ty: Type::Int,
                kind: ValueKind::Int(ordinal as i32),
            },
        ];
        let mut values = hidden;
        let mut constructor_owner = owner.to_owned();
        let mut constructor_params = class.params.clone();
        if let Some(entry_body) = &source.entry_bodies[ordinal] {
            let entry_owner = format!("{owner}${name}");
            let mut entry_source = entry_body.clone();
            entry_source.params = class.params[..2].to_vec();
            entry_source.defaults = vec![None; entry_source.params.len()];
            entry_source.param_annotations = vec![Vec::new(); entry_source.params.len()];
            entry_source.super_class = Some((
                owner.into(),
                entry_source
                    .params
                    .iter()
                    .map(|(name, _)| Expr {
                        offset: *offset,
                        kind: ExprKind::Name(name.clone()),
                    })
                    .collect(),
            ));
            entry_source.super_types = vec![Type::Java(owner.into())];
            let mut parent = classpath.load(owner)?;
            let original_parent = parent.clone();
            parent.access = (parent.access & !0x0010) | 0x0400;
            parent.methods.retain(|method| method.name != "<init>");
            let mut constructor = original_parent
                .methods
                .iter()
                .find(|method| method.name == "<init>")
                .cloned()
                .ok_or_else(|| Diagnostic::new(*offset, "Missing enum constructor"))?;
            constructor.access = 4;
            constructor.descriptor = format!(
                "({})V",
                entry_source
                    .params
                    .iter()
                    .map(|(_, ty)| ty.descriptor())
                    .collect::<String>()
            );
            constructor.kotlin_params = Some(
                entry_source
                    .params
                    .iter()
                    .map(|(name, ty)| (name.clone(), ty.clone(), false))
                    .collect(),
            );
            parent.methods.push(constructor);
            classpath.register_source(parent);
            let checked_entry = (|| {
                let mut entry_module = module.clone();
                entry_module.enum_decl = None;
                entry_module.class_decl = Some(entry_source.clone());
                classpath.source_header(&entry_module, &entry_owner)?;
                loop {
                    let progress = declarations::infer_source_results(
                        &mut entry_module,
                        classpath,
                        &entry_owner,
                    )?;
                    classpath.source_header(&entry_module, &entry_owner)?;
                    if progress == 0 {
                        break;
                    }
                }
                if let Some(error) =
                    declarations::source_inference_error(&entry_module, classpath, &entry_owner)?
                {
                    return Err(error);
                }
                entry_source = entry_module.class_decl.take().unwrap();
                let mut entry_classes = classes.clone();
                entry_classes.insert(entry_owner.clone(), classpath.load(&entry_owner)?);
                declarations::check_class(
                    &entry_source,
                    &module.package,
                    classpath,
                    &imports,
                    &entry_classes,
                    &HashMap::new(),
                    Some(&entry_owner),
                    &lambdas,
                )
            })();
            classpath.register_source(original_parent);
            let mut checked_entry = checked_entry?;
            let mut super_args = vec![
                Checker::wrap_lets(
                    Value {
                        ty: Type::String,
                        kind: ValueKind::Local(1),
                    },
                    std::mem::take(&mut lets),
                ),
                Value {
                    ty: Type::Int,
                    kind: ValueKind::Local(2),
                },
            ];
            super_args.extend(arguments);
            if mask != 0 {
                super_args.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int(mask as i32),
                });
            }
            super_args.push(Value {
                ty: Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
                kind: ValueKind::Null,
            });
            checked_entry.super_args = super_args;
            checked_entry.constructor_locals = checked_entry.constructor_locals.max(checker.next);
            constructor_params = entry_source.params.clone();
            constructor_owner = entry_owner.clone();
            entry_classes.push(CheckedEntry {
                owner: entry_owner,
                source: entry_source,
                imports: module.imports.clone(),
                class: checked_entry,
            });
        } else {
            if class.abstract_ {
                return Err(
                    Diagnostic::new(*offset, "Enum entry must implement abstract members")
                        .with_kind(
                            crate::DiagnosticKind::InvalidKotlin,
                            "ENUM_ENTRY_ABSTRACT_MEMBER_MISSING",
                        ),
                );
            }
            values.extend(arguments);
            if mask != 0 {
                values.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int(mask as i32),
                });
                values.push(Value {
                    ty: Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
                    kind: ValueKind::Null,
                });
                constructor_params.push(("$mask".into(), Type::Int));
                constructor_params.push((
                    "$marker".into(),
                    Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
                ));
            }
        }
        let descriptor = format!(
            "({})V",
            constructor_params
                .iter()
                .map(|(_, ty)| ty.descriptor())
                .collect::<String>()
        );
        let value = Checker::wrap_lets(
            Value {
                ty: Type::Java(owner.into()),
                kind: ValueKind::New {
                    owner: constructor_owner,
                    descriptor,
                    args: values,
                },
            },
            lets,
        );
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
        entry_classes,
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
    if !checker.classpath.load(owner).is_ok_and(|class| {
        class.access & 0x4000 != 0
            || class.super_name.as_deref().is_some_and(|parent| {
                checker
                    .classpath
                    .load(parent)
                    .is_ok_and(|parent| parent.access & 0x4000 != 0)
            })
    }) {
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
