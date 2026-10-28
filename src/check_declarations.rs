use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn check_class(
    source: &syntax::ClassDecl,
    package: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    companion_fields: &HashMap<String, (String, Type)>,
    owner_override: Option<&str>,
    lambdas: &Rc<RefCell<Vec<LambdaDecl>>>,
) -> Result<ClassDecl> {
    check_class_with_constants(
        source,
        package,
        classpath,
        imports,
        classes,
        companion_fields,
        owner_override,
        lambdas,
        &HashMap::new(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn check_class_with_constants(
    source: &syntax::ClassDecl,
    package: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    companion_fields: &HashMap<String, (String, Type)>,
    owner_override: Option<&str>,
    lambdas: &Rc<RefCell<Vec<LambdaDecl>>>,
    constants: &HashMap<String, (Type, Expr)>,
) -> Result<ClassDecl> {
    validate_constructor_assignments(source)?;
    for method in &source.methods {
        validate_operator(&method.function, true)?;
        validate_overriding_defaults(&method.function, method.is_override)?;
    }
    if source.data
        && (source.params.is_empty()
            || source.params.len() > 32
            || source.param_properties.len() != source.params.len()
            || !source.interfaces.is_empty()
            || !source.mutable_params.is_empty()
            || source.public_params.len() != source.params.len())
    {
        return Err(Diagnostic::new(
            source.offset,
            "Data classes require 1 to 32 public val constructor properties without interfaces",
        ));
    }
    if source.data
        && source.methods.iter().any(|m| {
            m.function.name == "copy"
                || (1..=source.params.len()).any(|i| m.function.name == format!("component{i}"))
        })
    {
        return Err(Diagnostic::new(
            source.offset,
            "Data class member conflicts with generated copy or component methods",
        ));
    }
    let owner = if let Some(owner) = owner_override {
        owner.into()
    } else if package.is_empty() {
        source.name.clone()
    } else {
        format!("{}/{}", package.replace('.', "/"), source.name)
    };
    fn data_type(ty: &Type) -> bool {
        match ty {
            Type::Int
            | Type::Long
            | Type::Byte
            | Type::Char
            | Type::Boolean
            | Type::Float
            | Type::Double
            | Type::String
            | Type::Java(_) => true,
            Type::Nullable(inner) | Type::List(inner) | Type::MutableList(inner) => {
                data_type(inner)
            }
            _ => false,
        }
    }
    if source.data && source.params.iter().any(|(_, ty)| !data_type(ty)) {
        return Err(Diagnostic::new(
            source.offset,
            "Data classes require supported scalar, nullable, List, or reference constructor properties; arrays are unsupported",
        ));
    }
    let mut params = Vec::new();
    let mut fields = HashMap::new();
    for (name, ty) in &source.params {
        let ty = resolve_type_checked(ty, imports, source.offset, classpath, classes)?;
        if ty == Type::Unit {
            return Err(Diagnostic::new(
                source.offset,
                "Unit parameters are not supported",
            ));
        }
        if params.iter().any(|(n, _)| n == name) {
            return Err(Diagnostic::new(
                source.offset,
                "Conflicting constructor field",
            ));
        }
        if source.param_properties.contains(name) {
            fields.insert(name.clone(), ty.clone());
        }
        params.push((name.clone(), ty));
    }
    if params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() > 254 {
        return Err(Diagnostic::new(
            source.offset,
            "Instance methods support at most 254 parameter slots",
        ));
    }
    let super_class = source
        .super_class
        .as_ref()
        .map(|(name, _)| imports.get(name).cloned().unwrap_or_else(|| name.clone()))
        .unwrap_or_else(|| "java/lang/Object".into());
    if super_class == "java/lang/Enum"
        && !classpath
            .load(&owner)
            .is_ok_and(|class| class.access & 0x4000 != 0)
    {
        return Err(Diagnostic::new(
            source.offset,
            "Only enum declarations may inherit java.lang.Enum",
        )
        .with_kind(
            crate::DiagnosticKind::InvalidKotlin,
            "ENUM_DIRECT_INHERITANCE",
        ));
    }
    let mut interfaces = Vec::new();
    let mut required = HashMap::new();
    if let Ok(object) = classpath.load("java/lang/Object") {
        for method in object.methods {
            if source
                .methods
                .iter()
                .any(|m| m.function.name == method.name)
                && method.access & 8 == 0
            {
                required.insert(member_key(&method.name, &method.descriptor), method);
            }
        }
    }
    if super_class != "java/lang/Object" {
        let parent = classpath.load(&super_class)?;
        if parent.access & (0x0010 | 0x0200) != 0 || parent.access & 1 == 0 {
            return Err(Diagnostic::new(
                source.offset,
                "Superclass must be an accessible non-final class",
            ));
        }
        for method in classpath.methods(&super_class)? {
            if !method.name.starts_with('<')
                && method.access & 8 == 0
                && method.access & 5 != 0
                && method.owner != "java/lang/Object"
            {
                required.insert(member_key(&method.name, &method.descriptor), method);
            }
        }
    }
    for alias in &source.interfaces {
        let name = imports
            .get(alias)
            .ok_or_else(|| Diagnostic::new(source.offset, "Interfaces require explicit imports"))?;
        let interface = &classes[name];
        if interfaces.contains(name) {
            return Err(Diagnostic::new(source.offset, "Duplicate interface"));
        }
        interfaces.push(name.clone());
        for method in
            interfaces::interface_methods(interface, classes, source.offset)?.into_values()
        {
            if required
                .insert(member_key(&method.name, &method.descriptor), method)
                .is_some()
            {
                return Err(Diagnostic::new(
                    source.offset,
                    "Overloaded or conflicting interface methods are not supported",
                ));
            }
        }
    }

    let extensions = HashSet::new();
    let signatures = Vec::new();
    let functions = HashMap::new();
    let empty_members = HashMap::new();
    let mut class_fields = Vec::new();
    for (i, (name, ty)) in params.iter().enumerate() {
        if source.param_properties.contains(name) {
            class_fields.push(ClassField {
                name: name.clone(),
                ty: ty.clone(),
                private: !source.public_params.contains(name),
                mutable: source.mutable_params.contains(name),
                param: Some(i),
                initializer: None,
            });
        }
    }
    let mut mutable_fields: HashSet<_> = source.mutable_params.iter().cloned().collect();
    let mut initializer_checker = Checker {
        source_type_params: &[],
        source_infix: &HashSet::new(),
        source_operator: &HashSet::new(),
        lambdas: lambdas.clone(),
        captured: HashMap::new(),
        captured_cells: HashMap::new(),
        cells: HashMap::new(),
        cell_names: HashSet::new(),
        lambda_owner: owner.clone(),
        inline_result: None,
        nonlocal_result: None,
        classpath,
        constants,
        constructor: None,
        defaults: &Vec::new(),
        function_names: &[],
        mutable_fields: &mutable_fields,
        extensions: &extensions,
        classes,
        imports,
        signatures: &signatures,
        receiver: Some(Local {
            slot: 0,
            ty: Type::Java(owner.clone()),
            mutable: false,
        }),
        fields: &fields,
        members: &empty_members,
        companion_fields,
        functions: &functions,
        object_methods: &HashMap::new(),
        object_fields: &HashMap::new(),
        object_owner: None,
        allow_return: true,
        refinements: HashMap::new(),
        initialized: HashSet::new(),
        loop_depth: 0,
        scopes: vec![HashMap::new()],
        next: 1,
        result: Type::Unit,
    };
    for (name, ty) in &params {
        initializer_checker.declare(name, ty.clone(), false, source.offset)?;
    }
    if source.defaults.iter().any(Option::is_some) {
        if params.len() > 32 {
            return Err(Diagnostic::new(
                source.offset,
                "Default arguments support at most 32 parameters",
            ));
        }
        initializer_checker.next += 2;
    }
    initializer_checker.receiver = None;
    let defaults =
        initializer_checker.checked_defaults(&source.defaults, &params, source.offset)?;
    let mut super_args = Vec::new();
    if let Some((_, arguments)) = &source.super_class {
        initializer_checker.receiver = None;
        let parent = classpath.load(&super_class)?;
        let constructors = parent
            .methods
            .iter()
            .filter(|m| m.name == "<init>" && m.access & 5 != 0)
            .cloned()
            .collect::<Vec<_>>();
        let method = initializer_checker.select_method(&constructors, arguments, source.offset)?;
        let (types, _) = class_method_types(&method)?;
        let call_params = method
            .kotlin_params
            .as_ref()
            .map(|p| {
                p.iter()
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
        let (args, lets, mask) = initializer_checker.arguments(
            &call_params,
            &vec![false; call_params.len()],
            arguments,
            method.kotlin_params.is_some(),
            source.offset,
        )?;
        if mask != 0 || !lets.is_empty() {
            return Err(Diagnostic::new(
                source.offset,
                "Reordered and default superclass constructor arguments are unsupported",
            ));
        }
        super_args = args;
    }
    let mut constructor_locals = initializer_checker.next;
    drop(initializer_checker);
    let mut property_types = HashMap::new();
    let mut delegated_properties = Vec::new();
    let mut delegated_methods = Vec::new();
    for property in &source.properties {
        if property.mutable
            && property.initializer.is_none()
            && property.delegate.is_none()
            && !property.lateinit
            && !property.body.is_empty()
            && property.setter.is_none()
        {
            return Err(Diagnostic::new(
                property.offset,
                "A mutable property with a default setter requires an initialized backing field",
            ));
        }
        if fields.contains_key(&property.name) {
            return Err(Diagnostic::new(
                property.offset,
                "Conflicting property declaration",
            ));
        }
        if property.delegate.is_some() {
            let mut context = DeclarationContext::new(&owner, classpath, imports, classes, lambdas);
            context.constants = constants.clone();
            context.fields = fields.clone();
            context.mutable_fields = mutable_fields.clone();
            let checked = delegates::check_property(
                property,
                &context,
                &owner,
                &owner,
                false,
                true,
                &params,
                constructor_locals,
            )?;
            constructor_locals = checked.locals;
            property_types.insert(property.name.clone(), checked.property_type);
            fields.insert(checked.field.name.clone(), checked.field.ty.clone());
            class_fields.push(checked.field);
            delegated_properties.push((checked.metadata_name, checked.metadata));
            delegated_methods.extend(checked.methods);
            continue;
        }
        if let Some(expr) = &property.initializer {
            let mut checker = Checker {
                lambdas: lambdas.clone(),
                captured: HashMap::new(),
                captured_cells: HashMap::new(),
                cells: HashMap::new(),
                cell_names: HashSet::new(),
                lambda_owner: owner.clone(),
                inline_result: None,
                nonlocal_result: None,
                classpath,
                constants,
                constructor: None,
                defaults: &Vec::new(),
                function_names: &[],
                mutable_fields: &mutable_fields,
                extensions: &extensions,
                classes,
                imports,
                signatures: &signatures,
                source_type_params: &[],
                source_infix: &HashSet::new(),
                source_operator: &HashSet::new(),
                receiver: Some(Local {
                    slot: 0,
                    ty: Type::Java(owner.clone()),
                    mutable: false,
                }),
                fields: &fields,
                members: &empty_members,
                companion_fields,
                functions: &functions,
                object_methods: &HashMap::new(),
                object_fields: &HashMap::new(),
                object_owner: None,
                allow_return: true,
                refinements: HashMap::new(),
                initialized: HashSet::new(),
                loop_depth: 0,
                scopes: vec![HashMap::new()],
                next: 1,
                result: Type::Unit,
            };
            for (name, ty) in &params {
                checker.declare(name, ty.clone(), false, source.offset)?;
            }
            checker.next = constructor_locals;
            checker.allow_return = false;
            let value = if property.inferred {
                checker.value(expr)?
            } else {
                let expected = resolve_type_checked(
                    &property.ty,
                    imports,
                    property.offset,
                    classpath,
                    classes,
                )?;
                checker.expected_value(expr, &expected)?
            };
            if property.inferred && has_platform(&value.ty) {
                return Err(Diagnostic::new(
                    property.offset,
                    "Inferred platform property types require an explicit supported type annotation",
                ));
            }
            let value = if property.inferred {
                value
            } else {
                checker.coerce(
                    value,
                    resolve_type_checked(
                        &property.ty,
                        imports,
                        property.offset,
                        classpath,
                        classes,
                    )?,
                    property.offset,
                )?
            };
            if value.ty == Type::Unit || value.ty == Type::Null {
                return Err(Diagnostic::new(
                    property.offset,
                    "Initialized properties require a supported explicit or inferred type",
                ));
            }
            if unresolved_list(&value.ty) {
                return Err(Diagnostic::new(
                    property.offset,
                    "Cannot infer list element type; empty or null-only factories require an explicit or expected element type",
                ));
            }
            let ty = value.ty.clone();
            constructor_locals = checker.next;
            drop(checker);
            fields.insert(property.name.clone(), ty.clone());
            property_types.insert(property.name.clone(), ty.clone());
            if property.mutable {
                mutable_fields.insert(property.name.clone());
            }
            class_fields.push(ClassField {
                name: property.name.clone(),
                ty,
                private: property.private,
                mutable: property.mutable,
                param: None,
                initializer: Some(value),
            });
        } else {
            let ty =
                resolve_type_checked(&property.ty, imports, property.offset, classpath, classes)?;
            property_types.insert(property.name.clone(), ty.clone());
            if property.lateinit {
                if !property.mutable
                    || !ty.is_reference()
                    || ty.is_nullable()
                    || property.delegate.is_some()
                    || !property.body.is_empty()
                    || property.setter.is_some()
                {
                    return Err(Diagnostic::new(
                        property.offset,
                        "lateinit requires a mutable non-null reference property with default accessors",
                    ));
                }
                fields.insert(property.name.clone(), ty.clone());
                mutable_fields.insert(property.name.clone());
                class_fields.push(ClassField {
                    name: property.name.clone(),
                    ty,
                    private: property.private,
                    mutable: true,
                    param: None,
                    initializer: None,
                });
            } else if property.body.is_empty() && property.delegate.is_none() {
                if source.secondary_constructors.is_empty() && source.initializers.is_empty() {
                    return Err(Diagnostic::new(
                        property.offset,
                        "Property requires an initializer or getter",
                    ));
                }
                fields.insert(property.name.clone(), ty.clone());
                if property.mutable {
                    mutable_fields.insert(property.name.clone());
                }
                class_fields.push(ClassField {
                    name: property.name.clone(),
                    ty,
                    private: property.private,
                    mutable: property.mutable,
                    param: None,
                    initializer: None,
                });
            }
        }
    }
    if source.companion.is_some() && fields.contains_key("Companion") {
        return Err(Diagnostic::new(
            source.offset,
            "Instance field conflicts with generated Companion field",
        ));
    }
    let mut companion_static_fields = HashMap::new();
    let mut checked_companion = None;
    if let Some(companion) = &source.companion {
        let checked = check_static_state(
            &format!("{owner}$Companion"),
            &owner,
            &companion.properties,
            &companion.methods,
            &companion.initializers,
            true,
            classpath,
            imports,
            classes,
            lambdas,
        )?;
        for property in &checked.properties {
            companion_static_fields
                .insert(property.name.clone(), (owner.clone(), property.ty.clone()));
        }
        checked_companion = Some(checked);
    }
    let mut declarations = Vec::new();
    for property in &source.properties {
        if property_types[&property.name] == Type::Unit {
            return Err(Diagnostic::new(
                property.offset,
                "Unit properties are not supported",
            ));
        }
        if property.body.is_empty() {
            continue;
        }
        declarations.push((
            getter_name(&property.name),
            property.private,
            property.is_override,
            Some(property.name.as_str()),
            Vec::new(),
            &property_types[&property.name],
            &property.body,
            property.offset,
        ));
    }
    for property in &source.properties {
        if let Some((parameter, body)) = &property.setter {
            if !property.mutable {
                return Err(Diagnostic::new(
                    property.offset,
                    "A val cannot declare a setter",
                ));
            }
            declarations.push((
                setter_name(&property.name),
                property.private,
                property.is_override,
                Some(property.name.as_str()),
                vec![(parameter.clone(), property_types[&property.name].clone())],
                &Type::Unit,
                body,
                property.offset,
            ));
        }
    }
    for method in &source.methods {
        let function = &method.function;
        if function.receiver.is_some() {
            return Err(Diagnostic::new(
                function.offset,
                "Member extension functions are not supported",
            ));
        }
        declarations.push((
            function.name.clone(),
            function.private,
            method.is_override,
            None,
            function.params.clone(),
            &function.result,
            &function.body,
            function.offset,
        ));
    }
    let mut members = required.clone();
    for (name, private, _, property, source_params, result, _, offset) in &declarations {
        let params = source_params
            .iter()
            .map(|(_, ty)| {
                resolve_type_checked(ty, imports, *offset, classpath, classes)
                    .map(|ty| ty.descriptor())
            })
            .collect::<Result<Vec<_>>>()?
            .concat();
        let result = resolve_type_checked(result, imports, *offset, classpath, classes)?;
        let descriptor = format!("({params}){}", result.descriptor());
        members.retain(|_, m| m.name != *name || m.descriptor != descriptor);
        members.insert(
            format!("{name}({params})"),
            JavaMethod {
                library: source
                    .methods
                    .iter()
                    .find(|m| m.function.name == *name && m.function.params == *source_params)
                    .map(|m| crate::classpath::LibraryMethod {
                        abstract_: false,
                        builtin_array: false,
                        source_name: m.function.name.clone(),
                        operator: m.function.operator,
                        kotlin_public: !m.function.private
                            && !m.function.protected
                            && !m.function.internal,
                        type_parameters: Vec::new(),
                        receiver: None,
                        infix: m.function.infix,
                        inline: false,
                        code: None,
                    }),
                kotlin_params: Some(
                    source_params
                        .iter()
                        .enumerate()
                        .map(|(i, (n, t))| {
                            Ok((
                                n.clone(),
                                resolve_type_checked(t, imports, *offset, classpath, classes)?,
                                source
                                    .methods
                                    .iter()
                                    .find(|m| {
                                        m.function.name == *name
                                            && m.function.params == *source_params
                                    })
                                    .is_some_and(|m| {
                                        m.function.defaults.get(i).is_some_and(Option::is_some)
                                    }),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
                kotlin_result: Some(result.clone()),
                kotlin_internal: false,
                owner: owner.clone(),
                name: name.clone(),
                descriptor: format!("({params}){}", result.descriptor()),
                access: if *private {
                    2
                } else if source
                    .methods
                    .iter()
                    .any(|m| m.function.offset == *offset && m.function.protected)
                    || source
                        .properties
                        .iter()
                        .any(|p| p.offset == *offset && p.protected)
                {
                    4
                } else {
                    1
                },
                unsupported: false,
                source_property: property.map(str::to_owned),
            },
        );
    }
    for function in &delegated_methods {
        let property = source
            .properties
            .iter()
            .find(|p| {
                getter_name(&p.name) == function.name || setter_name(&p.name) == function.name
            })
            .unwrap();
        let method = function_method(function, &owner, 0, Some(&property.name));
        members.insert(
            format!("{}{}", function.name, function.descriptor()),
            method.clone(),
        );
        members.insert(
            format!(
                "{}{}",
                if function.params.is_empty() {
                    "#get:"
                } else {
                    "#set:"
                },
                property.name
            ),
            method,
        );
    }
    for property in &source.properties {
        if property.lateinit {
            let ty = property_types[&property.name].clone();
            members.insert(
                format!("#get:{}", property.name),
                JavaMethod {
                    library: None,
                    owner: owner.clone(),
                    name: getter_name(&property.name),
                    descriptor: format!("(){}", ty.descriptor()),
                    access: if property.private { 2 } else { 1 },
                    unsupported: false,
                    source_property: Some(property.name.clone()),
                    kotlin_params: Some(Vec::new()),
                    kotlin_result: Some(ty),
                    kotlin_internal: false,
                },
            );
        }
        for (prefix, name, active) in [
            (
                "#get:",
                getter_name(&property.name),
                !property.body.is_empty(),
            ),
            (
                "#set:",
                setter_name(&property.name),
                property.setter.is_some(),
            ),
        ] {
            if active {
                if let Some(method) = members.values().find(|m| m.source_name() == name).cloned() {
                    members.insert(format!("{prefix}{}", property.name), method);
                }
            }
        }
    }
    let functions = HashMap::new();
    let signatures = Vec::new();
    let extensions = HashSet::new();
    let mut methods = delegated_methods;
    let mut names = HashSet::new();
    for (name, private, is_override, source_property, source_params, result, body, offset) in
        declarations
    {
        let source_descriptor = format!(
            "{}({})",
            name,
            source_params
                .iter()
                .map(|p| p.1.descriptor())
                .collect::<String>()
        );
        if !names.insert(source_descriptor) {
            return Err(Diagnostic::new(
                offset,
                "Member overloads and JVM getter collisions are not supported",
            ));
        }
        let result = resolve_type_checked(result, imports, offset, classpath, classes)?;
        let mut body_members = members.clone();
        if let Some(property) = source_property {
            if fields.contains_key(property) {
                let mut backing = body_members
                    .values()
                    .find(|m| m.source_property.as_deref() == Some(property))
                    .cloned()
                    .unwrap();
                backing.name = property.into();
                body_members.insert("#backing".into(), backing);
            }
        }
        let mut checker = Checker {
            lambdas: lambdas.clone(),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: owner.clone(),
            inline_result: None,
            nonlocal_result: None,
            constants,
            constructor: None,
            defaults: &Vec::new(),
            function_names: &[],
            mutable_fields: &mutable_fields,
            classpath,
            extensions: &extensions,
            classes,
            imports,
            signatures: &signatures,
            source_type_params: &[],
            source_infix: &HashSet::new(),
            source_operator: &HashSet::new(),
            receiver: Some(Local {
                slot: 0,
                ty: Type::Java(owner.clone()),
                mutable: false,
            }),
            fields: &fields,
            members: &body_members,
            companion_fields,
            functions: &functions,
            object_methods: &HashMap::new(),
            object_fields: &companion_static_fields,
            object_owner: None,
            allow_return: true,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: 1,
            result: result.clone(),
        };
        let mut method_params = Vec::new();
        if source_params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>()
            > 254
        {
            return Err(Diagnostic::new(
                offset,
                "Instance methods support at most 254 parameter slots",
            ));
        }
        for (param, ty) in source_params {
            let ty = resolve_type_checked(&ty, imports, offset, classpath, classes)?;
            if ty == Type::Unit {
                return Err(Diagnostic::new(
                    offset,
                    "Unit and Float parameters are not supported",
                ));
            }
            checker.declare(&param, ty.clone(), false, offset)?;
            method_params.push((param, ty));
        }
        let source_defaults = source
            .methods
            .iter()
            .find(|m| {
                m.function.name == name
                    && m.function.params.len() == method_params.len()
                    && m.function
                        .params
                        .iter()
                        .zip(&method_params)
                        .all(|((n, t), (rn, rt))| {
                            n == rn
                                && resolve_type_checked(t, imports, offset, classpath, classes)
                                    .is_ok_and(|t| t == *rt)
                        })
            })
            .map(|m| m.function.defaults.clone())
            .unwrap_or_else(|| vec![None; method_params.len()]);
        if source_defaults.iter().any(Option::is_some) {
            checker.next += 2;
        }
        let checked_defaults =
            checker.checked_defaults(&source_defaults, &method_params, offset)?;
        let has_body = source
            .methods
            .iter()
            .find(|m| m.function.offset == offset)
            .is_none_or(|m| m.has_body);
        if !has_body && !source.abstract_ {
            return Err(Diagnostic::new(
                offset,
                "Abstract methods require an abstract class",
            ));
        }
        let (mut body, returns) = if has_body {
            checker.block(body)?
        } else {
            (Vec::new(), true)
        };
        if !returns {
            if result != Type::Unit {
                return Err(Diagnostic::new(
                    offset,
                    "Function must return on all paths; unconditional-loop analysis is not supported",
                ));
            }
            body.push(Statement::Return(Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            }));
        }
        let source_method = source
            .methods
            .iter()
            .find(|method| method.function.offset == offset);
        let method = Function {
            jvm_name: None,
            annotations: annotations::declaration_annotations(
                source_method.map_or(&[], |method| method.function.annotations.as_slice()),
                classpath,
                imports,
                "METHOD",
                None,
            )?,
            vararg: source_method.is_some_and(|method| method.function.vararg),
            type_params: resolve_parameters(
                &source
                    .methods
                    .iter()
                    .find(|m| m.function.offset == offset)
                    .map(|m| m.function.type_params.clone())
                    .unwrap_or_default(),
                imports,
                classpath,
                offset,
                classes,
            )?,
            defaults: checked_defaults,
            name,
            private,
            is_override,
            params: method_params,
            result,
            locals: checker.next,
            body,
        };
        for descriptor in annotations::overload_descriptors(&method) {
            if required.contains_key(&member_key(&method.name, &descriptor)) {
                return Err(Diagnostic::new(
                    offset,
                    "Generated JVM overload clashes with an inherited declaration",
                )
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "JVM_SIGNATURE_CLASH"));
            }
        }
        match required.get(&member_key(&method.name, &method.descriptor())) {
            Some(required) => {
                if required.access & 0x0010 != 0 {
                    return Err(Diagnostic::new(offset, "Cannot override a final member"));
                }
                if !is_override
                    || private
                    || method.descriptor() != required.descriptor
                    || !override_signature(
                        required,
                        &method
                            .params
                            .iter()
                            .map(|p| p.1.clone())
                            .collect::<Vec<_>>(),
                        &method.result,
                    )?
                    || required.source_property.as_deref() != source_property
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Interface implementation requires override and an exact supported JVM signature",
                    ));
                }
            }
            None if is_override => {
                return Err(Diagnostic::new(
                    offset,
                    "Member overrides no supported interface declaration",
                ));
            }
            None => {}
        }
        methods.push(method);
    }
    for field in &class_fields {
        let property_override = source
            .properties
            .iter()
            .find(|p| p.name == field.name)
            .is_some_and(|p| p.is_override)
            || source.override_params.contains(&field.name);
        let getter = getter_name(&field.name);
        if field.mutable {
            if let Some(setter) = required.get(&member_key(
                &setter_name(&field.name),
                &format!("({})V", field.ty.descriptor()),
            )) {
                let (params, result) = class_method_types(setter)?;
                if params != vec![field.ty.clone()] || result != Type::Unit {
                    return Err(Diagnostic::new(
                        source.offset,
                        "Mutable property overrides require an invariant property type",
                    ));
                }
            }
        }
        if let Some(required) = required.get(&member_key(&getter, "()V")) {
            if !property_override
                || field.private
                || !override_signature(required, &[], &field.ty)?
                || required.source_property.as_deref() != Some(field.name.as_str())
            {
                return Err(Diagnostic::new(
                    source.offset,
                    "Interface property implementation requires override and an exact supported signature",
                ));
            }
        } else if property_override {
            return Err(Diagnostic::new(
                source.offset,
                "Property overrides no supported interface declaration",
            ));
        }
        if !field.private
            || source
                .properties
                .iter()
                .any(|p| p.name == field.name && p.lateinit)
        {
            let receiver = || Value {
                ty: Type::Java(owner.clone()),
                kind: ValueKind::Local(0),
            };
            if !methods
                .iter()
                .any(|m| m.name == getter && m.params.is_empty())
            {
                methods.push(Function {
                    jvm_name: None,
                    annotations: Vec::new(),
                    vararg: false,
                    type_params: Vec::new(),
                    defaults: Vec::new(),
                    name: getter,
                    private: field.private,
                    is_override: property_override,
                    params: Vec::new(),
                    result: field.ty.clone(),
                    locals: 1,
                    body: {
                        let read = Value {
                            ty: field.ty.clone(),
                            kind: ValueKind::Field(field.name.clone(), Box::new(receiver())),
                        };
                        if source
                            .properties
                            .iter()
                            .any(|p| p.name == field.name && p.lateinit)
                        {
                            lateinit_getter(read, &field.name)
                        } else {
                            vec![Statement::Return(read)]
                        }
                    },
                });
            }
            if field.mutable
                && !methods
                    .iter()
                    .any(|m| m.name == setter_name(&field.name) && m.params.len() == 1)
            {
                methods.push(Function {
                    jvm_name: None,
                    annotations: Vec::new(),
                    vararg: false,
                    type_params: Vec::new(),
                    defaults: Vec::new(),
                    name: setter_name(&field.name),
                    private: field.private,
                    is_override: false,
                    params: vec![("<set-?>".into(), field.ty.clone())],
                    result: Type::Unit,
                    locals: 1 + field.ty.slots() as u16,
                    body: vec![
                        Statement::FieldStore {
                            name: field.name.clone(),
                            receiver: receiver(),
                            value: Value {
                                ty: field.ty.clone(),
                                kind: ValueKind::Local(1),
                            },
                        },
                        Statement::Return(Value {
                            ty: Type::Unit,
                            kind: ValueKind::Unit,
                        }),
                    ],
                });
            }
        }
    }
    let mut bridges = members
        .values()
        .filter(|m| m.owner == owner && m.access & 4 != 0 && !m.name.starts_with('<'))
        .cloned()
        .collect::<Vec<_>>();
    bridges.sort_by(|a, b| a.name.cmp(&b.name).then(a.descriptor.cmp(&b.descriptor)));
    bridges.dedup_by(|a, b| a.name == b.name && a.descriptor == b.descriptor);
    let mut required: Vec<_> = required.into_values().collect();
    required.sort_by(|a, b| a.name.cmp(&b.name));
    for method in required {
        if method.access & 4 != 0 {
            bridges.push(method.clone());
        }
        if methods
            .iter()
            .any(|m| m.name == method.name && m.descriptor() == method.descriptor)
        {
            continue;
        }
        if method.access & 0x0400 != 0 && !source.abstract_ {
            return Err(Diagnostic::new(
                source.offset,
                format!("Missing interface implementation: {}", method.name),
            ));
        }
        if classes
            .get(&method.owner)
            .is_some_and(|c| c.kotlin_interface)
        {
            bridges.push(direct_default_bridge(
                method,
                &interfaces,
                classpath,
                source.offset,
            )?);
        }
    }
    if methods.len() + bridges.len() + 1 > 65535 {
        return Err(Diagnostic::new(
            source.offset,
            "Too many methods for a JVM class",
        ));
    }
    let mut constructor_body = Vec::new();
    let mut context = DeclarationContext::new(&owner, classpath, imports, classes, lambdas);
    context.constants = constants.clone();
    context.fields = fields.clone();
    context.mutable_fields = fields.keys().cloned().collect();
    context.members = members.clone();
    context.static_fields = companion_static_fields.clone();
    let mut ordered = class_fields
        .iter_mut()
        .filter_map(|f| {
            f.initializer.take().map(|v| {
                (
                    source
                        .properties
                        .iter()
                        .find(|p| p.name == f.name || format!("{}$delegate", p.name) == f.name)
                        .unwrap()
                        .offset,
                    Some((f.name.clone(), v)),
                    None,
                )
            })
        })
        .chain(source.initializers.iter().map(|(o, b)| (*o, None, Some(b))))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|e| e.0);
    for (_, field, body) in ordered {
        if let Some((name, value)) = field {
            constructor_body.push(Statement::FieldStore {
                name,
                receiver: Value {
                    ty: Type::Java(owner.clone()),
                    kind: ValueKind::Local(0),
                },
                value,
            });
        }
        if let Some(body) = body {
            let mut checker = context.checker(true, 1, Type::Unit);
            for (name, ty) in &params {
                checker.declare(name, ty.clone(), false, source.offset)?;
            }
            checker.next = constructor_locals;
            checker.allow_return = false;
            let (body, _) = checker.block(body)?;
            constructor_locals = checker.next;
            constructor_body.extend(body);
        }
    }
    let mut secondary_constructors = Vec::new();
    let primary_descriptor = format!(
        "({})V",
        params.iter().map(|p| p.1.descriptor()).collect::<String>()
    );
    let own_class = classpath.load(&owner).ok();
    let mut constructor_descriptors = HashSet::new();
    if source.primary_constructor {
        constructor_descriptors.insert(primary_descriptor.clone());
    }
    for secondary in &source.secondary_constructors {
        if secondary.defaults.iter().any(Option::is_some) {
            return Err(Diagnostic::new(
                secondary.offset,
                "Default arguments on secondary constructors are unsupported",
            ));
        }
        if source.primary_constructor && secondary.delegation != "this" {
            return Err(Diagnostic::new(
                secondary.offset,
                "A secondary constructor must delegate to the primary constructor",
            ));
        }
        let mut checker = context.checker(true, 1, Type::Unit);
        let mut secondary_params = Vec::new();
        for (name, ty) in &secondary.params {
            let ty = resolve_type_checked(ty, imports, secondary.offset, classpath, classes)?;
            checker.declare(name, ty.clone(), false, secondary.offset)?;
            secondary_params.push((name.clone(), ty));
        }
        let descriptor = format!(
            "({})V",
            secondary_params
                .iter()
                .map(|p| p.1.descriptor())
                .collect::<String>()
        );
        if !constructor_descriptors.insert(descriptor) {
            return Err(Diagnostic::new(
                secondary.offset,
                "Conflicting constructor signature",
            ));
        }
        let delegates_to_this = secondary.delegation == "this";
        let delegation_owner = if delegates_to_this {
            owner.clone()
        } else {
            super_class.clone()
        };
        let target = if delegates_to_this {
            own_class.clone().ok_or_else(|| {
                Diagnostic::new(
                    secondary.offset,
                    "Source constructor headers are unavailable",
                )
            })?
        } else {
            classpath.load(&super_class)?
        };
        let constructors = target
            .methods
            .iter()
            .filter(|m| m.name == "<init>" && m.access & 5 != 0)
            .cloned()
            .collect::<Vec<_>>();
        checker.receiver = None;
        let selected =
            checker.select_method(&constructors, &secondary.arguments, secondary.offset)?;
        let (types, _) = class_method_types(&selected)?;
        let call_params = selected
            .kotlin_params
            .as_ref()
            .map(|p| {
                p.iter()
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
        let (delegation_args, lets, mask) = checker.arguments(
            &call_params,
            &vec![false; call_params.len()],
            &secondary.arguments,
            selected.kotlin_params.is_some(),
            secondary.offset,
        )?;
        if !lets.is_empty() || mask != 0 {
            return Err(Diagnostic::new(
                secondary.offset,
                "Reordered and default constructor delegation arguments are unsupported",
            ));
        }
        checker.receiver = Some(Local {
            slot: 0,
            ty: Type::Java(owner.clone()),
            mutable: false,
        });
        let defaults =
            checker.checked_defaults(&secondary.defaults, &secondary_params, secondary.offset)?;
        checker.allow_return = false;
        let (body, _) = checker.block(&secondary.body)?;
        let mut checked_body = body;
        let initializer_local_base = secondary_params
            .iter()
            .map(|(_, t)| t.slots() as u16)
            .sum::<u16>();
        if constructor_locals + initializer_local_base > 256 {
            return Err(Diagnostic::new(
                secondary.offset,
                "More than 256 constructor local slots are unsupported",
            ));
        }
        checked_body.push(Statement::Return(Value {
            ty: Type::Unit,
            kind: ValueKind::Unit,
        }));
        secondary_constructors.push(SecondaryConstructor {
            initializers: if delegates_to_this {
                Vec::new()
            } else {
                constructor_body.clone()
            },
            initializer_local_base,
            function: Function {
                jvm_name: None,
                annotations: Vec::new(),
                vararg: false,
                type_params: Vec::new(),
                defaults,
                name: "<init>".into(),
                private: false,
                is_override: false,
                params: secondary_params,
                result: Type::Unit,
                locals: checker
                    .next
                    .max(constructor_locals + initializer_local_base),
                body: checked_body,
            },
            delegation_owner,
            delegation_descriptor: selected.descriptor,
            delegation_args,
            delegates_to_this,
        });
    }
    for constructor in &secondary_constructors {
        let mut visited = HashSet::new();
        let mut current = constructor;
        while current.delegates_to_this {
            if !visited.insert(current.function.descriptor()) {
                return Err(Diagnostic::new(
                    source.offset,
                    "Cyclic secondary constructor delegation",
                ));
            }
            if source.primary_constructor && current.delegation_descriptor == primary_descriptor {
                break;
            }
            current = secondary_constructors
                .iter()
                .find(|c| c.function.descriptor() == current.delegation_descriptor)
                .ok_or_else(|| {
                    Diagnostic::new(
                        source.offset,
                        "Constructor delegation target is unavailable",
                    )
                })?;
        }
    }
    let method_keys = |predicate: fn(&syntax::ClassMethod) -> bool| {
        source
            .methods
            .iter()
            .filter(|m| predicate(m))
            .filter_map(|m| {
                methods
                    .iter()
                    .find(|f| {
                        f.name == m.function.name
                            && f.params.len() == m.function.params.len()
                            && f.params.iter().zip(&m.function.params).all(
                                |((_, actual), (_, source))| {
                                    resolve_type_checked(
                                        source,
                                        imports,
                                        m.function.offset,
                                        classpath,
                                        classes,
                                    )
                                    .is_ok_and(|source| source == *actual)
                                },
                            )
                    })
                    .map(|f| format!("{}{}", f.name, f.descriptor()))
            })
            .collect::<HashSet<_>>()
    };
    let mut open_methods = method_keys(|m| m.open || !m.has_body);
    for property in &source.properties {
        if property.open {
            for method in &methods {
                if method.name == getter_name(&property.name)
                    || method.name == setter_name(&property.name)
                {
                    open_methods.insert(format!("{}{}", method.name, method.descriptor()));
                }
            }
        }
    }
    for name in &source.override_params {
        for method in &methods {
            if method.name == getter_name(name) || method.name == setter_name(name) {
                open_methods.insert(format!("{}{}", method.name, method.descriptor()));
            }
        }
    }
    let mut protected_methods = method_keys(|m| m.function.protected);
    for property in &source.properties {
        if property.protected {
            for method in &methods {
                if method.name == getter_name(&property.name)
                    || method.name == setter_name(&property.name)
                {
                    protected_methods.insert(format!("{}{}", method.name, method.descriptor()));
                }
            }
        }
    }
    let abstract_methods = method_keys(|m| !m.has_body);
    Ok(ClassDecl {
        exposed_fields: source
            .properties
            .iter()
            .filter(|p| p.lateinit)
            .map(|p| {
                (
                    p.name.clone(),
                    if p.private {
                        2
                    } else if p.protected {
                        4
                    } else {
                        1
                    },
                )
            })
            .collect(),
        delegated_properties,
        primary_constructor: source.primary_constructor,
        abstract_: source.abstract_,
        open_methods,
        protected_methods,
        abstract_methods,
        constructor_body,
        super_class,
        super_args,
        open: source.open,
        secondary_constructors,
        fields: class_fields,
        defaults,
        constructor_locals,
        companion: checked_companion,
        params,
        data: source.data,
        interfaces,
        methods,
        bridges,
    })
}

#[derive(Clone)]
pub(super) struct DeclarationContext<'a> {
    pub classpath: &'a Classpath,
    pub imports: &'a HashMap<String, String>,
    pub classes: &'a HashMap<String, JavaClass>,
    pub owner: String,
    pub fields: HashMap<String, Type>,
    pub mutable_fields: HashSet<String>,
    pub members: HashMap<String, JavaMethod>,
    pub static_fields: HashMap<String, (String, Type)>,
    pub object_methods: HashMap<String, Vec<JavaMethod>>,
    pub constants: HashMap<String, (Type, Expr)>,
    pub lambdas: Rc<RefCell<Vec<LambdaDecl>>>,
    empty_fields: HashMap<String, (String, Type)>,
    functions: HashMap<String, usize>,
    signatures: Vec<(Vec<(String, Type)>, Type)>,
    extensions: HashSet<usize>,
    defaults: Vec<Vec<Option<Expr>>>,
    infix: HashSet<usize>,
}
impl<'a> DeclarationContext<'a> {
    pub fn new(
        owner: &str,
        classpath: &'a Classpath,
        imports: &'a HashMap<String, String>,
        classes: &'a HashMap<String, JavaClass>,
        lambdas: &Rc<RefCell<Vec<LambdaDecl>>>,
    ) -> Self {
        Self {
            classpath,
            imports,
            classes,
            owner: owner.into(),
            fields: HashMap::new(),
            mutable_fields: HashSet::new(),
            members: HashMap::new(),
            static_fields: HashMap::new(),
            object_methods: HashMap::new(),
            constants: HashMap::new(),
            lambdas: lambdas.clone(),
            empty_fields: HashMap::new(),
            functions: HashMap::new(),
            signatures: Vec::new(),
            extensions: HashSet::new(),
            defaults: Vec::new(),
            infix: HashSet::new(),
        }
    }
    pub fn checker(&self, receiver: bool, next: u16, result: Type) -> Checker<'_> {
        Checker {
            lambdas: self.lambdas.clone(),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: self.owner.clone(),
            inline_result: None,
            nonlocal_result: None,
            classpath: self.classpath,
            constants: &self.constants,
            constructor: None,
            defaults: &self.defaults,
            function_names: &[],
            mutable_fields: &self.mutable_fields,
            extensions: &self.extensions,
            classes: self.classes,
            imports: self.imports,
            signatures: &self.signatures,
            source_type_params: &[],
            source_infix: &self.infix,
            source_operator: &self.infix,
            receiver: receiver.then(|| Local {
                slot: 0,
                ty: Type::Java(self.owner.clone()),
                mutable: false,
            }),
            fields: &self.fields,
            members: &self.members,
            companion_fields: &self.empty_fields,
            functions: &self.functions,
            object_methods: &self.object_methods,
            object_fields: &self.static_fields,
            object_owner: Some(&self.owner),
            allow_return: true,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next,
            result,
        }
    }
    pub fn function(
        &self,
        source: &syntax::Function,
        receiver: bool,
        is_override: bool,
    ) -> Result<Function> {
        validate_operator(source, true)?;
        validate_overriding_defaults(source, is_override)?;

        let result = resolve_type_checked(
            &source.result,
            self.imports,
            source.offset,
            self.classpath,
            self.classes,
        )?;
        let mut checker = self.checker(receiver, u16::from(receiver), result.clone());
        let mut params = Vec::new();
        for (name, ty) in &source.params {
            let ty = resolve_type_checked(
                ty,
                self.imports,
                source.offset,
                self.classpath,
                self.classes,
            )?;
            if ty == Type::Unit {
                return Err(Diagnostic::new(
                    source.offset,
                    "Unit parameters are unsupported",
                ));
            }
            checker.declare(name, ty.clone(), false, source.offset)?;
            params.push((name.clone(), ty));
        }
        if params.iter().map(|(_, ty)| ty.slots()).sum::<usize>() > if receiver { 254 } else { 255 }
        {
            return Err(Diagnostic::new(
                source.offset,
                "Too many JVM method parameter slots",
            ));
        }
        if source.defaults.iter().any(Option::is_some) {
            checker.next += 2;
        }
        let defaults = checker.checked_defaults(&source.defaults, &params, source.offset)?;
        let (mut body, returns) = checker.block(&source.body)?;
        if !returns {
            if result != Type::Unit {
                return Err(Diagnostic::new(
                    source.offset,
                    "Function must return on all paths",
                ));
            }
            body.push(Statement::Return(Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            }));
        }
        Ok(Function {
            jvm_name: None,
            annotations: annotations::declaration_annotations(
                &source.annotations,
                self.classpath,
                self.imports,
                "METHOD",
                None,
            )?,
            vararg: source.vararg,
            type_params: resolve_parameters(
                &source.type_params,
                self.imports,
                self.classpath,
                source.offset,
                self.classes,
            )?,
            defaults,
            name: source.name.clone(),
            private: source.private,
            is_override,
            params,
            result,
            locals: checker.next,
            body,
        })
    }
}

pub(super) fn lateinit_getter(read: Value, name: &str) -> Vec<Statement> {
    vec![
        Statement::If(
            Value {
                ty: Type::Boolean,
                kind: ValueKind::IsNull(Box::new(read.clone())),
            },
            vec![Statement::ThrowException {
                owner: "kotlin/UninitializedPropertyAccessException".into(),
                message: Some(Value {
                    ty: Type::String,
                    kind: ValueKind::String(
                        format!("lateinit property {name} has not been initialized")
                            .encode_utf16()
                            .collect(),
                    ),
                }),
            }],
            Vec::new(),
        ),
        Statement::Return(read),
    ]
}

pub fn unresolved_source_dependencies(
    source: &syntax::Module,
) -> Vec<(String, usize, HashSet<String>)> {
    let mut declarations = Vec::new();
    for function in &source.functions {
        if function.result_inferred {
            let parameters = function
                .params
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();
            declarations.push((
                function.name.clone(),
                function.offset,
                captures::free_names(&function.body, &parameters),
            ));
        }
    }
    for property in &source.properties {
        if property.inferred {
            if let Some(initializer) = &property.initializer {
                let body = vec![syntax::Statement {
                    offset: initializer.offset,
                    kind: StatementKind::Expression(initializer.clone()),
                }];
                declarations.push((
                    property.name.clone(),
                    property.offset,
                    captures::free_names(&body, &[]),
                ));
            }
        }
    }
    declarations
}

pub fn infer_source_results(
    source: &mut syntax::Module,
    classpath: &Classpath,
    owner: &str,
) -> Result<usize> {
    infer_source_results_impl(source, classpath, owner, &mut Vec::new())
}
pub fn source_inference_error(
    source: &syntax::Module,
    classpath: &Classpath,
    owner: &str,
) -> Result<Option<Diagnostic>> {
    let mut errors = Vec::new();
    infer_source_results_impl(&mut source.clone(), classpath, owner, &mut errors)?;
    Ok(errors.into_iter().next())
}
fn infer_source_results_impl(
    source: &mut syntax::Module,
    classpath: &Classpath,
    owner: &str,
    errors: &mut Vec<Diagnostic>,
) -> Result<usize> {
    if let Some(mut enumeration) = source.enum_decl.take() {
        let mut context = source.clone();
        context.class_decl = Some(enumeration.body.clone());
        let progress = infer_source_results_impl(&mut context, classpath, owner, errors);
        enumeration.body = context.class_decl.unwrap();
        source.enum_decl = Some(enumeration);
        return progress;
    }
    let mut unresolved_names = source
        .functions
        .iter()
        .filter(|f| f.result_inferred)
        .map(|f| f.name.clone())
        .collect::<HashSet<_>>();
    if let Some(class) = &source.class_decl {
        unresolved_names.extend(
            class
                .methods
                .iter()
                .filter(|m| m.function.result_inferred)
                .map(|m| m.function.name.clone()),
        );
    }
    if let Some(object) = &source.object_decl {
        unresolved_names.extend(
            object
                .methods
                .iter()
                .filter(|m| m.function.result_inferred)
                .map(|m| m.function.name.clone()),
        );
    }
    if let Some(interface) = &source.interface_decl {
        unresolved_names.extend(
            interface
                .methods
                .iter()
                .filter(|m| m.function.result_inferred)
                .map(|m| m.function.name.clone()),
        );
    }

    let mut imports = HashMap::new();
    let mut classes = HashMap::new();
    for (alias, path, import_offset) in &source.imports {
        if let Some((package, member)) = path.rsplit_once('/') {
            if classpath.type_alias(package, member)?.is_some() {
                imports.insert(alias.clone(), path.clone());
                continue;
            }
        }
        if alias == "*" {
            for class in classpath.package_classes(path.trim_end_matches("/*"))? {
                imports.insert(
                    class.name.rsplit('/').next().unwrap().into(),
                    class.name.clone(),
                );
                classes.insert(class.name.clone(), class);
            }
            continue;
        }
        if let Ok(class) = classpath.load(path) {
            imports.insert(alias.clone(), path.clone());
            classes.insert(path.clone(), class);
        } else if let Some((package, member)) = path.rsplit_once('/') {
            if let Some(class) =
                library::static_import_owner(classpath, package, member, *import_offset)?
            {
                imports.insert(alias.clone(), format!("{package}::{member}"));
                classes.insert(package.into(), class);
                continue;
            }
            for class in classpath.package_classes(package)? {
                if class.methods.iter().any(|m| {
                    m.library.as_ref().is_some_and(|l| l.source_name == member)
                        || m.source_property.as_deref() == Some(member)
                }) || class
                    .fields
                    .iter()
                    .any(|field| field.name == member && field.constant.is_some())
                {
                    imports.insert(alias.clone(), format!("{}::{member}", class.name));
                    classes.insert(class.name.clone(), class);
                }
            }
        }
    }
    if let Ok(class) = classpath.load(owner) {
        imports.insert(owner.rsplit('/').next().unwrap().into(), owner.into());
        classes.insert(owner.into(), class);
    }
    if let Some(name) = source
        .class_decl
        .as_ref()
        .map(|d| &d.name)
        .or_else(|| source.object_decl.as_ref().map(|d| &d.name))
        .or_else(|| source.interface_decl.as_ref().map(|d| &d.name))
    {
        imports.insert(name.clone(), owner.into());
    }
    fn type_names(ty: &Type, out: &mut HashSet<String>) {
        match ty {
            Type::Java(n) => {
                out.insert(n.clone());
            }
            Type::Nullable(t) | Type::List(t) | Type::MutableList(t) | Type::Array(t) => {
                type_names(t, out)
            }
            Type::Parameterized { name, arguments } => {
                out.insert(name.clone());
                for a in arguments {
                    if let Some(t) = &a.ty {
                        type_names(t, out);
                    }
                }
            }
            Type::Function { params, result } => {
                for t in params {
                    type_names(t, out);
                }
                type_names(result, out);
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => {
                type_names(receiver, out);
                for t in params {
                    type_names(t, out);
                }
                type_names(result, out);
            }
            _ => {}
        }
    }
    let mut names = HashSet::new();
    for function in &source.functions {
        for (_, ty) in &function.params {
            type_names(ty, &mut names);
        }
    }
    if let Some(class) = &source.class_decl {
        for (_, ty) in &class.params {
            type_names(ty, &mut names);
        }
        for method in &class.methods {
            for (_, ty) in &method.function.params {
                type_names(ty, &mut names);
            }
        }
        for property in &class.properties {
            if !property.inferred {
                type_names(&property.ty, &mut names);
            }
        }
    }
    for name in names {
        if imports.contains_key(&name) {
            continue;
        }
        let path = if name.contains('/') || source.package.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", source.package.replace('.', "/"), name)
        };
        if let Ok(class) = classpath.load(&path) {
            imports.insert(name, path.clone());
            classes.insert(path, class);
        }
    }
    let lambdas = Rc::new(RefCell::new(Vec::new()));
    let mut context = DeclarationContext::new(owner, classpath, &imports, &classes, &lambdas);
    for constant in &source.constants {
        let (ty, kind) = constant_expr(&constant.value);
        context.constants.insert(
            constant.name.clone(),
            (
                ty,
                Expr {
                    offset: constant.offset,
                    kind,
                },
            ),
        );
    }
    if let Some(class) = classes.get(owner) {
        for method in &class.methods {
            if method.name.starts_with('<') {
                continue;
            }
            context.members.insert(
                format!("{}{}", method.name, method.descriptor),
                method.clone(),
            );
        }
    }
    for function in &source.functions {
        if !function.result_inferred {
            context
                .functions
                .entry(function.name.clone())
                .or_insert(context.signatures.len());
            let params = function
                .params
                .iter()
                .map(|(n, t)| {
                    Ok((
                        n.clone(),
                        resolve_type_checked(t, &imports, function.offset, classpath, &classes)?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            let result = resolve_type_checked(
                &function.result,
                &imports,
                function.offset,
                classpath,
                &classes,
            )?;
            context.signatures.push((params, result));
        }
    }
    let mut progress = 0;
    for property in &source.properties {
        if !property.inferred {
            if let Ok(ty) =
                resolve_type_checked(&property.ty, &imports, property.offset, classpath, &classes)
            {
                context
                    .static_fields
                    .insert(property.name.clone(), (owner.into(), ty));
            }
        }
    }
    for property in &mut source.properties {
        if property.inferred {
            if let Some(expr) = &property.initializer {
                let inferred = context.checker(false, 0, Type::Unit).value(expr);
                if let Err(error) = &inferred {
                    errors.push(error.clone());
                }
                if let Ok(value) = inferred {
                    if value.ty != Type::Null && value.ty != Type::Unit {
                        property.ty = value.ty.clone();
                        property.inferred = false;
                        progress += 1;
                        context
                            .static_fields
                            .insert(property.name.clone(), (owner.into(), value.ty));
                    }
                }
            }
        }
    }
    for property in &mut source.properties {
        if property.inferred && property.delegate.is_some() {
            if let Ok(delegated) =
                delegates::check_property(property, &context, owner, owner, true, false, &[], 0)
            {
                property.ty = delegated.property_type.clone();
                property.inferred = false;
                context.static_fields.insert(
                    property.name.clone(),
                    (owner.into(), delegated.property_type),
                );
                progress += 1;
            }
        }
    }
    if let Some(class) = &mut source.class_decl {
        for (name, ty) in &class.params {
            if class.param_properties.contains(name) {
                if let Ok(ty) =
                    resolve_type_checked(ty, &imports, class.offset, classpath, &classes)
                {
                    context.fields.insert(name.clone(), ty);
                }
            }
        }
        for property in &class.properties {
            if !property.inferred {
                if let Ok(ty) = resolve_type_checked(
                    &property.ty,
                    &imports,
                    property.offset,
                    classpath,
                    &classes,
                ) {
                    context.fields.insert(property.name.clone(), ty);
                }
            }
        }
        for property in &mut class.properties {
            if property.inferred {
                if let Some(expr) = &property.initializer {
                    let mut checker = context.checker(true, 1, Type::Unit);
                    for (name, ty) in &class.params {
                        if let Ok(ty) =
                            resolve_type_checked(ty, &imports, class.offset, classpath, &classes)
                        {
                            let _ = checker.declare(name, ty, false, class.offset);
                        }
                    }
                    let inferred = checker.value(expr);
                    if let Err(error) = &inferred {
                        errors.push(error.clone());
                    }
                    if let Ok(value) = inferred {
                        if value.ty != Type::Null && value.ty != Type::Unit {
                            property.ty = value.ty.clone();
                            property.inferred = false;
                            progress += 1;
                            drop(checker);
                            context.fields.insert(property.name.clone(), value.ty);
                        }
                    }
                }
            }
        }
        let constructor_params = class
            .params
            .iter()
            .map(|(n, t)| {
                Ok((
                    n.clone(),
                    resolve_type_checked(t, &imports, class.offset, classpath, &classes)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        for property in &mut class.properties {
            if property.inferred && property.delegate.is_some() {
                if let Ok(delegated) = delegates::check_property(
                    property,
                    &context,
                    owner,
                    owner,
                    false,
                    true,
                    &constructor_params,
                    1,
                ) {
                    property.ty = delegated.property_type.clone();
                    property.inferred = false;
                    progress += 1;
                }
            }
        }
        for method in &mut class.methods {
            if infer_function_record(
                &context,
                &mut method.function,
                true,
                &unresolved_names,
                errors,
            ) {
                progress += 1;
            }
        }
        if let Some(companion) = &mut class.companion {
            let companion_owner = format!("{owner}$Companion");
            let mut comp_context =
                DeclarationContext::new(&companion_owner, classpath, &imports, &classes, &lambdas);
            load_context_members(&mut comp_context);
            progress += infer_static_properties(
                &mut companion.properties,
                &mut comp_context,
                owner,
                true,
                errors,
            )?;
            for method in &mut companion.methods {
                if infer_function_record(
                    &comp_context,
                    &mut method.function,
                    true,
                    &unresolved_names,
                    errors,
                ) {
                    progress += 1;
                }
            }
        }
    }
    if let Some(object) = &mut source.object_decl {
        for field in &object.fields {
            if let Some(ty) = &field.ty {
                let ty = if matches!(ty, Type::ThreadLocal(_)) {
                    Ok(ty.clone())
                } else {
                    resolve_type_checked(ty, &imports, field.offset, classpath, &classes)
                };
                if let Ok(ty) = ty {
                    context
                        .static_fields
                        .insert(field.name.clone(), (owner.into(), ty));
                }
            }
        }
        let original = object.clone();
        for field in &mut object.fields {
            if field.ty.is_none() {
                let inferred = if let Some(expr) = &field.initializer {
                    context
                        .checker(false, 0, Type::Unit)
                        .value(expr)
                        .map(|v| v.ty)
                        .or_else(|_| {
                            objects::infer_legacy_field(
                                &original,
                                field,
                                &source.package,
                                classpath,
                                &imports,
                                &classes,
                            )
                        })
                } else if field.delegate.is_some() {
                    let property = syntax::PropertyDecl {
                        annotations: field.annotations.clone(),
                        name: field.name.clone(),
                        private: field.private,
                        protected: false,
                        mutable: field.mutable,
                        inferred: true,
                        initializer: None,
                        offset: field.offset,
                        ty: Type::Unit,
                        body: field.body.clone(),
                        is_override: field.is_override,
                        setter: field.setter.clone(),
                        delegate: field.delegate.clone(),
                        lateinit: field.lateinit,
                        open: false,
                    };
                    delegates::check_property(&property, &context, owner, owner, true, true, &[], 0)
                        .map(|v| v.property_type)
                } else {
                    Err(Diagnostic::new(
                        field.offset,
                        "Cannot infer object property type",
                    ))
                };
                if let Ok(ty) = inferred {
                    if ty != Type::Null && ty != Type::Unit {
                        field.ty = Some(ty.clone());
                        progress += 1;
                        context
                            .static_fields
                            .insert(field.name.clone(), (owner.into(), ty));
                    }
                }
            }
        }
        for method in &mut object.methods {
            if infer_function_record(
                &context,
                &mut method.function,
                !method.jvm_static,
                &unresolved_names,
                errors,
            ) {
                progress += 1;
            }
        }
    }
    if let Some(interface) = &mut source.interface_decl {
        for method in &mut interface.methods {
            if infer_function_record(
                &context,
                &mut method.function,
                true,
                &unresolved_names,
                errors,
            ) {
                progress += 1;
            }
        }
        if let Some(companion) = &mut interface.companion {
            let companion_owner = format!("{owner}$Companion");
            let mut comp_context =
                DeclarationContext::new(&companion_owner, classpath, &imports, &classes, &lambdas);
            load_context_members(&mut comp_context);
            progress += infer_static_properties(
                &mut companion.fields,
                &mut comp_context,
                &companion_owner,
                true,
                errors,
            )?;
            for method in &mut companion.methods {
                if infer_function_record(
                    &comp_context,
                    &mut method.function,
                    true,
                    &unresolved_names,
                    errors,
                ) {
                    progress += 1;
                }
            }
        }
    }
    for function in &mut source.functions {
        if infer_function_record(&context, function, false, &unresolved_names, errors) {
            progress += 1;
        }
    }
    Ok(progress)
}
fn infer_function_record(
    context: &DeclarationContext<'_>,
    function: &mut syntax::Function,
    receiver: bool,
    unresolved: &HashSet<String>,
    errors: &mut Vec<Diagnostic>,
) -> bool {
    match infer_function(context, function, receiver) {
        Ok(progress) => progress,
        Err(error) => {
            let mut dependencies = HashSet::new();
            body_call_names(&function.body, &mut dependencies);
            if dependencies.is_disjoint(unresolved) {
                errors.push(error);
            }
            false
        }
    }
}
fn infer_function(
    context: &DeclarationContext<'_>,
    function: &mut syntax::Function,
    receiver: bool,
) -> Result<bool> {
    if !function.result_inferred {
        return Ok(false);
    }
    let [
        syntax::Statement {
            kind: StatementKind::Return(Some(expr)),
            ..
        },
    ] = function.body.as_slice()
    else {
        return Err(Diagnostic::new(
            function.offset,
            "Inferred return type requires an expression body",
        ));
    };
    let mut checker = context.checker(receiver, u16::from(receiver), Type::Unit);
    if let Some(ty) = &function.receiver {
        let ty = resolve_type_checked(
            ty,
            context.imports,
            function.offset,
            context.classpath,
            context.classes,
        )?;
        let slot = checker.declare("<extension-this>", ty.clone(), false, function.offset)?;
        checker.receiver = Some(Local {
            slot,
            ty,
            mutable: false,
        });
    }
    for (name, ty) in &function.params {
        let ty = resolve_type_checked(
            ty,
            context.imports,
            function.offset,
            context.classpath,
            context.classes,
        )?;
        checker.declare(name, ty, false, function.offset)?;
    }
    let value = checker.value(expr)?;
    if value.ty == Type::Null {
        return Err(Diagnostic::new(
            function.offset,
            "Cannot infer a supported return type from null",
        ));
    }
    function.result = value.ty;
    function.result_inferred = false;
    Ok(true)
}

pub(super) fn accessor_source(
    name: String,
    property: &syntax::ClassCompanionProperty,
    params: Vec<(String, Type)>,
    result: Type,
    body: Vec<syntax::Statement>,
) -> syntax::Function {
    syntax::Function {
        param_annotations: vec![Vec::new(); params.len()],
        annotations: Vec::new(),
        vararg: false,
        name,
        offset: property.offset,
        private: property.private,
        protected: false,
        internal: false,
        infix: false,
        operator: false,
        receiver: None,
        type_params: Vec::new(),
        defaults: vec![None; params.len()],
        params,
        result,
        result_inferred: false,
        body,
    }
}
#[allow(clippy::too_many_arguments)]
pub(super) fn check_static_state(
    owner: &str,
    storage_owner: &str,
    source_properties: &[syntax::ClassCompanionProperty],
    source_methods: &[syntax::ClassMethod],
    source_initializers: &[(usize, Vec<syntax::Statement>)],
    receiver: bool,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    lambdas: &Rc<RefCell<Vec<LambdaDecl>>>,
) -> Result<ClassCompanion> {
    let mut context = DeclarationContext::new(owner, classpath, imports, classes, lambdas);
    if let Ok(class) = classpath.load(owner) {
        for method in class.methods {
            if method.name.starts_with('<') {
                continue;
            }
            if method.access & 8 != 0 {
                context
                    .object_methods
                    .entry(method.name.clone())
                    .or_default()
                    .push(method);
            } else {
                context
                    .members
                    .insert(format!("{}{}", method.name, method.descriptor), method);
            }
        }
    }
    for property in source_properties {
        if let Some(ty) = &property.ty {
            let ty = resolve_type_checked(ty, imports, property.offset, classpath, classes)?;
            context
                .static_fields
                .insert(property.name.clone(), (storage_owner.into(), ty));
        }
        if property.mutable {
            context.mutable_fields.insert(property.name.clone());
        }
    }
    let mut properties = Vec::new();
    let mut initializers = Vec::new();
    let mut locals = 0;
    let mut delegated_accessors = Vec::new();
    let mut ordered = source_properties
        .iter()
        .map(|p| (p.offset, Some(p), None))
        .chain(source_initializers.iter().map(|(o, b)| (*o, None, Some(b))))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|e| e.0);
    let mut names = HashSet::new();
    for (_, property, block) in ordered {
        if let Some(block) = block {
            let mut checker = context.checker(false, locals, Type::Unit);
            checker.allow_return = false;
            let (body, _) = checker.block(block)?;
            locals = checker.next;
            initializers.extend(body);
            continue;
        }
        let property = property.unwrap();
        if !names.insert(property.name.clone()) {
            return Err(Diagnostic::new(
                property.offset,
                "Conflicting static property",
            ));
        }
        if property.delegate.is_some() {
            let source = syntax::PropertyDecl {
                annotations: property.annotations.clone(),
                name: property.name.clone(),
                private: property.private,
                protected: false,
                mutable: property.mutable,
                inferred: property.ty.is_none(),
                initializer: None,
                offset: property.offset,
                ty: property.ty.clone().unwrap_or(Type::Unit),
                body: Vec::new(),
                is_override: false,
                setter: None,
                delegate: property.delegate.clone(),
                lateinit: false,
                open: false,
            };
            let delegated = delegates::check_property(
                &source,
                &context,
                owner,
                storage_owner,
                true,
                receiver,
                &[],
                locals,
            )?;
            let metadata_type = delegated.metadata.ty.clone();
            initializers.push(Statement::JavaFieldStore {
                owner: storage_owner.into(),
                name: delegated.metadata_name.clone(),
                ty: metadata_type.clone(),
                receiver: None,
                value: delegated.metadata,
            });
            properties.push(ClassCompanionProperty {
                name: delegated.metadata_name,
                ty: metadata_type.clone(),
                private: true,
                mutable: false,
                value: objects::default_value(&metadata_type),
            });
            let delegate_type = delegated.field.ty.clone();
            initializers.push(Statement::JavaFieldStore {
                owner: storage_owner.into(),
                name: delegated.field.name.clone(),
                ty: delegate_type.clone(),
                receiver: None,
                value: delegated.field.initializer.unwrap(),
            });
            properties.push(ClassCompanionProperty {
                name: delegated.field.name,
                ty: delegate_type.clone(),
                private: true,
                mutable: false,
                value: objects::default_value(&delegate_type),
            });
            context.static_fields.insert(
                property.name.clone(),
                (storage_owner.into(), delegated.property_type),
            );
            locals = delegated.locals;
            delegated_accessors.extend(delegated.methods);
            continue;
        }
        let mut checker = context.checker(false, locals, Type::Unit);
        checker.allow_return = false;
        let value = property
            .initializer
            .as_ref()
            .map(|e| {
                if let Some(ty) = &property.ty {
                    let ty =
                        resolve_type_checked(ty, imports, property.offset, classpath, classes)?;
                    checker.expected_value(e, &ty)
                } else {
                    checker.value(e)
                }
            })
            .transpose()?;
        let ty = property
            .ty
            .as_ref()
            .map(|t| resolve_type_checked(t, imports, property.offset, classpath, classes))
            .transpose()?
            .or_else(|| value.as_ref().map(|v| v.ty.clone()))
            .ok_or_else(|| {
                Diagnostic::new(property.offset, "Property requires a type or initializer")
            })?;
        if ty == Type::Unit || ty == Type::Null {
            return Err(Diagnostic::new(
                property.offset,
                "Property requires a supported type",
            ));
        }
        if property.lateinit
            && (!property.mutable
                || !ty.is_reference()
                || ty.is_nullable()
                || property.initializer.is_some()
                || !property.body.is_empty()
                || property.setter.is_some())
        {
            return Err(Diagnostic::new(
                property.offset,
                "lateinit requires an uninitialized mutable non-null reference with default accessors",
            ));
        }
        if let Some(value) = value {
            let value = checker.coerce(value, ty.clone(), property.offset)?;
            initializers.push(Statement::JavaFieldStore {
                owner: storage_owner.into(),
                name: property.name.clone(),
                ty: ty.clone(),
                receiver: None,
                value,
            });
        }
        locals = checker.next;
        drop(checker);
        context
            .static_fields
            .insert(property.name.clone(), (storage_owner.into(), ty.clone()));
        properties.push(ClassCompanionProperty {
            name: property.name.clone(),
            ty: ty.clone(),
            private: property.private,
            mutable: property.mutable,
            value: objects::default_value(&ty),
        });
    }
    for property in source_properties {
        let ty = context
            .static_fields
            .get(&property.name)
            .map(|(_, t)| t.clone())
            .unwrap_or(Type::Unit);
        for (prefix, name, result, params, active) in [
            (
                "#get:",
                getter_name(&property.name),
                ty.clone(),
                Vec::<(String, Type)>::new(),
                !property.body.is_empty() || property.lateinit || property.delegate.is_some(),
            ),
            (
                "#set:",
                setter_name(&property.name),
                Type::Unit,
                vec![("value".into(), ty.clone())],
                property.setter.is_some() || property.delegate.is_some(),
            ),
        ] {
            if active {
                context.members.insert(
                    format!("{prefix}{}", property.name),
                    JavaMethod {
                        library: None,
                        owner: owner.into(),
                        name,
                        descriptor: format!(
                            "({}){}",
                            params.iter().map(|p| p.1.descriptor()).collect::<String>(),
                            result.descriptor()
                        ),
                        access: if receiver { 1 } else { 9 },
                        unsupported: false,
                        source_property: Some(property.name.clone()),
                        kotlin_params: Some(
                            params
                                .iter()
                                .map(|(n, t)| (n.clone(), t.clone(), false))
                                .collect(),
                        ),
                        kotlin_result: Some(result),
                        kotlin_internal: false,
                    },
                );
            }
        }
    }
    let mut methods = delegated_accessors;
    for source in source_methods {
        methods.push(context.function(&source.function, receiver, source.is_override)?);
    }
    for property in source_properties {
        if property.delegate.is_some() {
            continue;
        }
        let ty = context.static_fields[&property.name].1.clone();
        let mut accessor_context =
            DeclarationContext::new(owner, classpath, imports, classes, lambdas);
        accessor_context.static_fields = context.static_fields.clone();
        accessor_context.mutable_fields = context.mutable_fields.clone();
        accessor_context.members = context.members.clone();
        accessor_context.object_methods = context.object_methods.clone();
        let backing = JavaMethod {
            library: None,
            owner: storage_owner.into(),
            name: property.name.clone(),
            descriptor: format!("(){}", ty.descriptor()),
            access: 8,
            unsupported: false,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        };
        accessor_context.members.insert("#backing".into(), backing);
        let read = Value {
            ty: ty.clone(),
            kind: if owner != storage_owner {
                ValueKind::StaticCall {
                    owner: storage_owner.into(),
                    name: format!("access${}$cp", getter_name(&property.name)),
                    descriptor: format!("(){}", ty.descriptor()),
                    args: Vec::new(),
                }
            } else {
                ValueKind::StaticField {
                    owner: storage_owner.into(),
                    name: property.name.clone(),
                }
            },
        };
        let mut getter = if property.body.is_empty() {
            Function {
                jvm_name: None,
                annotations: Vec::new(),
                vararg: false,
                type_params: Vec::new(),
                defaults: Vec::new(),
                name: getter_name(&property.name),
                private: property.private,
                is_override: false,
                params: Vec::new(),
                result: ty.clone(),
                locals: u16::from(receiver),
                body: if property.lateinit {
                    lateinit_getter(read, &property.name)
                } else {
                    vec![Statement::Return(read)]
                },
            }
        } else {
            accessor_context.function(
                &accessor_source(
                    getter_name(&property.name),
                    property,
                    Vec::new(),
                    ty.clone(),
                    property.body.clone(),
                ),
                receiver,
                false,
            )?
        };
        getter.private = property.private;
        methods.push(getter);
        if property.mutable {
            let setter = if let Some((name, body)) = &property.setter {
                accessor_context.function(
                    &accessor_source(
                        setter_name(&property.name),
                        property,
                        vec![(name.clone(), ty.clone())],
                        Type::Unit,
                        body.clone(),
                    ),
                    receiver,
                    false,
                )?
            } else {
                let value = Value {
                    ty: ty.clone(),
                    kind: ValueKind::Local(u8::from(receiver)),
                };
                let store = if owner != storage_owner {
                    Statement::Expression(Value {
                        ty: Type::Unit,
                        kind: ValueKind::StaticCall {
                            owner: storage_owner.into(),
                            name: format!("access${}$cp", setter_name(&property.name)),
                            descriptor: format!("({})V", ty.descriptor()),
                            args: vec![value],
                        },
                    })
                } else {
                    Statement::JavaFieldStore {
                        owner: storage_owner.into(),
                        name: property.name.clone(),
                        ty: ty.clone(),
                        receiver: None,
                        value,
                    }
                };
                Function {
                    jvm_name: None,
                    annotations: Vec::new(),
                    vararg: false,
                    type_params: Vec::new(),
                    defaults: Vec::new(),
                    name: setter_name(&property.name),
                    private: property.private,
                    is_override: false,
                    params: vec![("value".into(), ty.clone())],
                    result: Type::Unit,
                    locals: u16::from(receiver) + ty.slots() as u16,
                    body: vec![
                        store,
                        Statement::Return(Value {
                            ty: Type::Unit,
                            kind: ValueKind::Unit,
                        }),
                    ],
                }
            };
            methods.push(setter);
        }
    }
    Ok(ClassCompanion {
        exposed_fields: source_properties
            .iter()
            .filter(|p| p.lateinit)
            .map(|p| (p.name.clone(), if p.private { 2 } else { 1 }))
            .collect(),
        owner: owner.into(),
        properties,
        methods,
        initializers,
        locals,
    })
}

pub(super) fn function_method(
    function: &Function,
    owner: &str,
    access: u16,
    property: Option<&str>,
) -> JavaMethod {
    JavaMethod {
        library: None,
        owner: owner.into(),
        name: function.name.clone(),
        descriptor: function.descriptor(),
        access: access | if function.private { 2 } else { 1 },
        unsupported: false,
        source_property: property.map(str::to_owned),
        kotlin_params: Some(
            function
                .params
                .iter()
                .map(|(n, t)| (n.clone(), t.clone(), false))
                .collect(),
        ),
        kotlin_result: Some(function.result.clone()),
        kotlin_internal: false,
    }
}

pub(super) fn validate_operator(function: &syntax::Function, member: bool) -> Result<()> {
    if function.vararg
        && function
            .params
            .last()
            .is_some_and(|(_, ty)| matches!(ty, Type::Array(_) | Type::StringArray))
    {
        return Err(Diagnostic::new(
            function.offset,
            "Source reference vararg declarations require unsupported Array<out T> projections",
        ));
    }

    if !function.operator {
        return Ok(());
    }
    let count = function.params.len();
    if !member && function.receiver.is_none() {
        return Err(Diagnostic::new(
            function.offset,
            "Operator functions require a member or extension receiver",
        ));
    }
    let valid = match function.name.as_str() {
        "contains" => count == 1 && (function.result_inferred || function.result == Type::Boolean),
        "get" => count >= 1,
        "set" => count >= 2 && (function.result_inferred || function.result == Type::Unit),
        "getValue" | "provideDelegate" => count == 2,
        "setValue" => count == 3 && (function.result_inferred || function.result == Type::Unit),
        "invoke" => true,
        name if name
            .strip_prefix("component")
            .is_some_and(|n| !n.is_empty() && n.parse::<usize>().is_ok_and(|n| n > 0)) =>
        {
            count == 0
        }
        _ => {
            return Err(Diagnostic::new(
                function.offset,
                "Unsupported operator convention name",
            ));
        }
    };
    if !valid {
        return Err(
            Diagnostic::new(function.offset, "Invalid operator function signature").with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "INVALID_OPERATOR_SIGNATURE",
            ),
        );
    }
    if function.defaults.iter().any(Option::is_some)
        && matches!(
            function.name.as_str(),
            "contains" | "getValue" | "setValue" | "provideDelegate"
        )
    {
        return Err(Diagnostic::new(
            function.offset,
            "This operator cannot have default parameters",
        ));
    }
    Ok(())
}

pub(super) fn override_signature(
    method: &JavaMethod,
    params: &[Type],
    result: &Type,
) -> Result<bool> {
    let (expected_params, expected_result) = class_method_types(method)?;
    fn compatible(expected: &Type, actual: &Type) -> bool {
        match expected {
            Type::Platform(inner) => {
                inner.as_ref() == actual || matches!(actual,Type::Nullable(actual) if actual==inner)
            }
            _ => expected == actual,
        }
    }
    Ok(expected_params.len() == params.len()
        && expected_params
            .iter()
            .zip(params)
            .all(|(e, a)| compatible(e, a))
        && (compatible(&expected_result, result)
            || matches!(&expected_result,Type::Nullable(inner) if inner.as_ref()==result)))
}

fn load_context_members(context: &mut DeclarationContext<'_>) {
    if let Ok(class) = context.classpath.load(&context.owner) {
        for method in class.methods {
            if !method.name.starts_with('<') {
                context
                    .members
                    .insert(format!("{}{}", method.name, method.descriptor), method);
            }
        }
    }
}
fn infer_static_properties(
    properties: &mut [syntax::ClassCompanionProperty],
    context: &mut DeclarationContext<'_>,
    storage_owner: &str,
    receiver: bool,
    errors: &mut Vec<Diagnostic>,
) -> Result<usize> {
    let mut progress = 0;
    for property in properties.iter() {
        if let Some(ty) = &property.ty {
            if let Ok(ty) = resolve_type_checked(
                ty,
                context.imports,
                property.offset,
                context.classpath,
                context.classes,
            ) {
                context
                    .static_fields
                    .insert(property.name.clone(), (storage_owner.into(), ty));
            }
        }
    }
    for property in properties.iter_mut() {
        if property.ty.is_none() {
            let inferred = if let Some(expr) = &property.initializer {
                context
                    .checker(false, 0, Type::Unit)
                    .value(expr)
                    .map(|v| v.ty)
            } else if property.delegate.is_some() {
                let source = syntax::PropertyDecl {
                    annotations: property.annotations.clone(),
                    name: property.name.clone(),
                    private: property.private,
                    protected: false,
                    mutable: property.mutable,
                    inferred: true,
                    initializer: None,
                    offset: property.offset,
                    ty: Type::Unit,
                    body: Vec::new(),
                    is_override: false,
                    setter: None,
                    delegate: property.delegate.clone(),
                    lateinit: false,
                    open: false,
                };
                delegates::check_property(
                    &source,
                    context,
                    &context.owner,
                    storage_owner,
                    true,
                    receiver,
                    &[],
                    0,
                )
                .map(|d| d.property_type)
            } else {
                Err(Diagnostic::new(
                    property.offset,
                    "Cannot infer a property type",
                ))
            };
            if let Err(error) = &inferred {
                errors.push(error.clone());
            }
            if let Ok(ty) = inferred {
                if ty != Type::Null && ty != Type::Unit {
                    property.ty = Some(ty.clone());
                    context
                        .static_fields
                        .insert(property.name.clone(), (storage_owner.into(), ty));
                    progress += 1;
                }
            }
        }
    }
    Ok(progress)
}

#[derive(Clone)]
struct ConstructorAssignments {
    definite: HashSet<String>,
    possible: HashSet<String>,
    terminated: bool,
}
fn validate_constructor_assignments(source: &syntax::ClassDecl) -> Result<()> {
    let mut fields = HashMap::new();
    let mut initial = HashSet::new();
    for name in &source.param_properties {
        fields.insert(name.clone(), source.mutable_params.contains(name));
        initial.insert(name.clone());
    }
    let mut required = HashSet::new();
    for property in &source.properties {
        if property.delegate.is_none()
            && (property.initializer.is_some() || property.body.is_empty())
        {
            fields.insert(property.name.clone(), property.mutable);
            if !property.lateinit {
                required.insert(property.name.clone());
            } else {
                initial.insert(property.name.clone());
            }
        }
    }
    let mut state = ConstructorAssignments {
        definite: initial.clone(),
        possible: initial,
        terminated: false,
    };
    let mut entries = source
        .properties
        .iter()
        .filter(|p| p.initializer.is_some())
        .map(|p| (p.offset, Some(p), None))
        .chain(source.initializers.iter().map(|(o, b)| (*o, None, Some(b))))
        .collect::<Vec<_>>();
    entries.sort_by_key(|e| e.0);
    let primary_locals = source
        .params
        .iter()
        .map(|p| p.0.clone())
        .collect::<HashSet<_>>();
    for (_, property, body) in entries {
        if let Some(property) = property {
            if let Some(initializer) = &property.initializer {
                constructor_read(initializer, &fields, &primary_locals, &state)?;
            }
            state.definite.insert(property.name.clone());
            state.possible.insert(property.name.clone());
        }
        if let Some(body) = body {
            constructor_assignment_block(body, &fields, &primary_locals, &mut state, false)?;
        }
    }
    let complete =
        |state: &ConstructorAssignments| state.terminated || required.is_subset(&state.definite);
    if source.primary_constructor && !complete(&state) {
        return Err(Diagnostic::new(
            source.offset,
            "A property must be initialized on every constructor path",
        ));
    }
    for constructor in &source.secondary_constructors {
        let mut current = if constructor.delegation == "this" {
            ConstructorAssignments {
                definite: fields.keys().cloned().collect(),
                possible: fields.keys().cloned().collect(),
                terminated: false,
            }
        } else {
            state.clone()
        };
        let locals = constructor.params.iter().map(|p| p.0.clone()).collect();
        constructor_assignment_block(&constructor.body, &fields, &locals, &mut current, false)?;
        if !complete(&current) {
            return Err(Diagnostic::new(
                constructor.offset,
                "A property must be initialized on every constructor path",
            ));
        }
    }
    Ok(())
}
fn constructor_assignment_block(
    body: &[syntax::Statement],
    fields: &HashMap<String, bool>,
    outer_locals: &HashSet<String>,
    state: &mut ConstructorAssignments,
    in_loop: bool,
) -> Result<()> {
    let mut locals = outer_locals.clone();
    for statement in body {
        if state.terminated {
            break;
        }
        match &statement.kind {
            StatementKind::Let(_, _, _, value)
            | StatementKind::Assign(_, value)
            | StatementKind::AddAssign(_, value)
            | StatementKind::CompoundAssign(_, _, value)
            | StatementKind::Expression(value)
            | StatementKind::Throw(value)
            | StatementKind::Destructure(_, _, value) => {
                constructor_read(value, fields, &locals, state)?
            }
            StatementKind::PropertyAssign(receiver, _, value) => {
                constructor_read(receiver, fields, &locals, state)?;
                constructor_read(value, fields, &locals, state)?;
            }
            StatementKind::If(condition, _, _)
            | StatementKind::While(condition, _)
            | StatementKind::DoWhile(condition, _)
            | StatementKind::ForEach(_, condition, _)
            | StatementKind::When(condition, _, _)
            | StatementKind::WhenBinding(_, condition, _, _) => {
                constructor_read(condition, fields, &locals, state)?
            }
            StatementKind::ForUntil(_, start, end, _) => {
                constructor_read(start, fields, &locals, state)?;
                constructor_read(end, fields, &locals, state)?;
            }
            StatementKind::IndexAssign(array, index, value) => {
                constructor_read(array, fields, &locals, state)?;
                constructor_read(index, fields, &locals, state)?;
                constructor_read(value, fields, &locals, state)?;
            }
            StatementKind::IndicesAssign(array, indices, value) => {
                constructor_read(array, fields, &locals, state)?;
                for index in indices {
                    constructor_read(index, fields, &locals, state)?;
                }
                constructor_read(value, fields, &locals, state)?;
            }
            _ => {}
        }
        let assigned = match &statement.kind {
            StatementKind::Assign(name, _)
            | StatementKind::AddAssign(name, _)
            | StatementKind::CompoundAssign(name, _, _)
                if !locals.contains(name) =>
            {
                Some(name)
            }
            StatementKind::PropertyAssign(receiver, name, _) if matches!(&receiver.kind,ExprKind::Name(n)if n=="this") => {
                Some(name)
            }
            _ => None,
        };
        if let Some(name) = assigned {
            if let Some(mutable) = fields.get(name) {
                if matches!(
                    statement.kind,
                    StatementKind::AddAssign(..) | StatementKind::CompoundAssign(..)
                ) && !state.definite.contains(name)
                {
                    return Err(Diagnostic::new(
                        statement.offset,
                        "Property must be initialized before use",
                    ));
                }
                if !mutable
                    && (in_loop
                        || state.possible.contains(name)
                        || !matches!(
                            statement.kind,
                            StatementKind::Assign(..) | StatementKind::PropertyAssign(..)
                        ))
                {
                    return Err(Diagnostic::new(
                        statement.offset,
                        "Cannot reassign val property in a constructor",
                    ));
                }
                state.definite.insert(name.clone());
                state.possible.insert(name.clone());
            }
        }
        match &statement.kind {
            StatementKind::Let(name, _, _, _) | StatementKind::Declare(name, _, _) => {
                locals.insert(name.clone());
            }
            StatementKind::Destructure(names, _, _) => {
                locals.extend(names.iter().filter_map(|entry| entry.name.clone()));
            }
            StatementKind::Block(body) => {
                constructor_assignment_block(body, fields, &locals, state, in_loop)?
            }
            StatementKind::Labeled(_, body) => constructor_assignment_block(
                std::slice::from_ref(body.as_ref()),
                fields,
                &locals,
                state,
                in_loop,
            )?,
            StatementKind::If(_, yes, no) => {
                let mut y = state.clone();
                let mut n = state.clone();
                constructor_assignment_block(yes, fields, &locals, &mut y, in_loop)?;
                constructor_assignment_block(no, fields, &locals, &mut n, in_loop)?;
                *state = merge_constructor_paths(vec![y, n]);
            }
            StatementKind::When(_, branches, otherwise)
            | StatementKind::WhenBinding(_, _, branches, otherwise) => {
                let mut paths = Vec::new();
                for (_, body) in branches {
                    let mut branch = state.clone();
                    constructor_assignment_block(body, fields, &locals, &mut branch, in_loop)?;
                    paths.push(branch);
                }
                let mut branch = state.clone();
                if let Some(body) = otherwise {
                    constructor_assignment_block(body, fields, &locals, &mut branch, in_loop)?;
                }
                paths.push(branch);
                *state = merge_constructor_paths(paths);
            }
            StatementKind::While(_, body)
            | StatementKind::DoWhile(_, body)
            | StatementKind::ForUntil(_, _, _, body)
            | StatementKind::ForEach(_, _, body) => {
                let mut loop_state = state.clone();
                constructor_assignment_block(body, fields, &locals, &mut loop_state, true)?;
                state.possible.extend(loop_state.possible);
            }
            StatementKind::Throw(_) => state.terminated = true,
            StatementKind::Expression(Expr {
                kind:
                    ExprKind::Try {
                        body,
                        catches,
                        finally,
                    },
                ..
            }) => {
                let mut paths = Vec::new();
                let mut branch = state.clone();
                constructor_assignment_block(body, fields, &locals, &mut branch, in_loop)?;
                paths.push(branch);
                for catch in catches {
                    let mut branch = state.clone();
                    constructor_assignment_block(
                        &catch.body,
                        fields,
                        &locals,
                        &mut branch,
                        in_loop,
                    )?;
                    paths.push(branch);
                }
                *state = merge_constructor_paths(paths);
                constructor_assignment_block(finally, fields, &locals, state, in_loop)?;
            }
            _ => {}
        }
    }
    Ok(())
}
fn merge_constructor_paths(paths: Vec<ConstructorAssignments>) -> ConstructorAssignments {
    let mut continuing = paths.into_iter().filter(|p| !p.terminated);
    let Some(mut merged) = continuing.next() else {
        return ConstructorAssignments {
            definite: HashSet::new(),
            possible: HashSet::new(),
            terminated: true,
        };
    };
    for branch in continuing {
        merged.definite.retain(|n| branch.definite.contains(n));
        merged.possible.extend(branch.possible);
    }
    merged
}

fn constructor_read(
    expr: &Expr,
    fields: &HashMap<String, bool>,
    locals: &HashSet<String>,
    state: &ConstructorAssignments,
) -> Result<()> {
    let required = |name: &str| {
        fields.contains_key(name) && !locals.contains(name) && !state.definite.contains(name)
    };
    match &expr.kind {
        ExprKind::Name(name) if required(name) => {
            return Err(Diagnostic::new(
                expr.offset,
                "Property must be initialized before use",
            ));
        }
        ExprKind::Property(receiver, name)
            if matches!(&receiver.kind,ExprKind::Name(n)if n=="this")
                && fields.contains_key(name)
                && !state.definite.contains(name) =>
        {
            return Err(Diagnostic::new(
                expr.offset,
                "Property must be initialized before use",
            ));
        }
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            constructor_read(a, fields, locals, state)?;
            constructor_read(b, fields, locals, state)?;
        }
        ExprKind::Unary(_, value)
        | ExprKind::NonNull(value)
        | ExprKind::Property(value, _)
        | ExprKind::SafeProperty(value, _)
        | ExprKind::Generic(value, _)
        | ExprKind::NamedArgument(_, value)
        | ExprKind::Throw(value)
        | ExprKind::Labeled(_, value) => constructor_read(value, fields, locals, state)?,
        ExprKind::TypeOperation(_, value, _)
        | ExprKind::Increment { value, .. }
        | ExprKind::Decrement { value, .. } => constructor_read(value, fields, locals, state)?,
        ExprKind::Call(target, args) => {
            constructor_read(target, fields, locals, state)?;
            for arg in args {
                constructor_read(arg, fields, locals, state)?;
            }
        }
        ExprKind::Indices(target, indices) => {
            constructor_read(target, fields, locals, state)?;
            for index in indices {
                constructor_read(index, fields, locals, state)?;
            }
        }
        ExprKind::If(condition, yes, no) => {
            constructor_read(condition, fields, locals, state)?;
            constructor_read(yes, fields, locals, state)?;
            constructor_read(no, fields, locals, state)?;
        }
        ExprKind::Template(parts) => {
            for part in parts {
                constructor_read(part, fields, locals, state)?;
            }
        }
        ExprKind::Return(value) | ExprKind::LabeledReturn(_, value) => {
            if let Some(value) = value {
                constructor_read(value, fields, locals, state)?;
            }
        }
        ExprKind::Block(body) => {
            let mut block = state.clone();
            constructor_assignment_block(body, fields, locals, &mut block, false)?;
        }
        ExprKind::Try {
            body,
            catches,
            finally,
        } => {
            let mut block = state.clone();
            constructor_assignment_block(body, fields, locals, &mut block, false)?;
            for catch in catches {
                let mut block = state.clone();
                let mut locals = locals.clone();
                locals.insert(catch.name.clone());
                constructor_assignment_block(&catch.body, fields, &locals, &mut block, false)?;
            }
            let mut block = state.clone();
            constructor_assignment_block(finally, fields, locals, &mut block, false)?;
        }
        ExprKind::When(subject, branches, otherwise) => {
            constructor_read(subject, fields, locals, state)?;
            for (conditions, body) in branches {
                for condition in conditions {
                    constructor_read(condition, fields, locals, state)?;
                }
                constructor_read(body, fields, locals, state)?;
            }
            constructor_read(otherwise, fields, locals, state)?;
        }
        ExprKind::WhenBinding(name, subject, branches, otherwise) => {
            constructor_read(subject, fields, locals, state)?;
            let mut locals = locals.clone();
            locals.insert(name.clone());
            for (conditions, body) in branches {
                for condition in conditions {
                    constructor_read(condition, fields, &locals, state)?;
                }
                constructor_read(body, fields, &locals, state)?;
            }
            constructor_read(otherwise, fields, &locals, state)?;
        }
        ExprKind::CallableReference {
            receiver: Some(receiver),
            ..
        } => constructor_read(receiver, fields, locals, state)?,
        _ => {}
    }
    Ok(())
}

pub(super) fn direct_default_bridge(
    method: JavaMethod,
    parents: &[String],
    classpath: &Classpath,
    offset: usize,
) -> Result<JavaMethod> {
    if parents.contains(&method.owner) {
        return Ok(method);
    }
    for parent in parents {
        if classpath.methods(parent)?.iter().any(|candidate| {
            candidate.name == method.name && candidate.descriptor == method.descriptor
        }) {
            return Ok(JavaMethod {
                owner: parent.clone(),
                ..method
            });
        }
    }
    Err(Diagnostic::new(
        offset,
        "Inherited default method requires a directly implemented interface",
    ))
}

pub(super) fn validate_overriding_defaults(
    function: &syntax::Function,
    is_override: bool,
) -> Result<()> {
    if is_override && function.defaults.iter().any(Option::is_some) {
        return Err(Diagnostic::new(
            function.offset,
            "Overriding methods cannot declare default parameter values",
        )
        .with_kind(
            crate::DiagnosticKind::InvalidKotlin,
            "OVERRIDE_DEFAULT_ARGUMENT",
        ));
    }
    Ok(())
}

pub(super) fn member_key(name: &str, descriptor: &str) -> String {
    format!(
        "{}{})",
        name,
        descriptor.split(')').next().unwrap_or(descriptor)
    )
}
