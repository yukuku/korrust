use super::*;

fn legacy_check(
    source: &syntax::ObjectDecl,
    package: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
) -> Result<Module> {
    if !source.constants.is_empty() || !source.objects.is_empty() {
        return Err(Diagnostic::new(
            source.offset,
            "Objects with methods cannot combine constants or nested objects yet",
        ));
    }
    let owner = if package.is_empty() {
        source.name.clone()
    } else {
        format!("{}/{}", package.replace('.', "/"), source.name)
    };
    let lambdas = Rc::new(RefCell::new(Vec::new()));
    let mut object_methods: HashMap<String, Vec<JavaMethod>> = HashMap::new();
    for method in &source.methods {
        let function = &method.function;
        if function.defaults.iter().any(Option::is_some) {
            return Err(Diagnostic::new(
                function.offset,
                "Default arguments on object methods are unsupported",
            ));
        }
        if loses_member_type(&function.result)
            || function.params.iter().any(|(_, ty)| loses_member_type(ty))
        {
            return Err(Diagnostic::new(
                function.offset,
                "Nullable and List object method signatures are not supported",
            ));
        }
        for (annotation, active) in [
            ("JvmStatic", method.jvm_static),
            ("Throws", !method.throws.is_empty()),
            ("Suppress", !method.suppress.is_empty()),
        ] {
            if active && (source.name == annotation || imports.contains_key(annotation)) {
                return Err(Diagnostic::new(
                    function.offset,
                    "Shadowed builtin object annotations are not supported",
                ));
            }
        }
        if function.receiver.is_some()
            || method.jvm_static && function.private
            || method.inline && !function.private
        {
            return Err(Diagnostic::new(
                function.offset,
                "Object member extensions and private JvmStatic methods are not supported",
            ));
        }
        if function
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>()
            > if method.jvm_static { 255 } else { 254 }
        {
            return Err(Diagnostic::new(
                function.offset,
                "Too many object method parameters",
            ));
        }
        let mut descriptor = String::from("(");
        for (_, ty) in &function.params {
            let ty = resolve_type_checked(ty, imports, function.offset, classpath, classes)?;
            if matches!(ty, Type::Unit | Type::Float | Type::ThreadLocal(_)) {
                return Err(Diagnostic::new(
                    function.offset,
                    "Unsupported object method parameter type",
                ));
            }
            descriptor.push_str(&ty.descriptor());
        }
        descriptor.push(')');
        let result = resolve_type_checked(
            &function.result,
            imports,
            function.offset,
            classpath,
            classes,
        )?;
        if matches!(result, Type::Float | Type::ThreadLocal(_)) {
            return Err(Diagnostic::new(
                function.offset,
                "Unsupported object method return type",
            ));
        }
        descriptor.push_str(&result.descriptor());
        if inherited_object_method(&function.name)
            && (function.name != "toString" || function.params.is_empty())
        {
            return Err(Diagnostic::new(
                function.offset,
                "Any and Object inherited members are not supported",
            ));
        }
        let overloads = object_methods.entry(function.name.clone()).or_default();
        if overloads
            .iter()
            .any(|method| method.descriptor.split(')').next() == descriptor.split(')').next())
        {
            return Err(Diagnostic::new(
                function.offset,
                "Conflicting object method parameter signature",
            ));
        }
        overloads.push(JavaMethod {
            library: function.infix.then(|| crate::classpath::LibraryMethod {
                abstract_: false,
                builtin_array: false,
                source_name: function.name.clone(),
                operator: function.operator,
                kotlin_public: !function.private && !function.protected && !function.internal,
                type_parameters: Vec::new(),
                receiver: None,
                infix: true,
                inline: false,
                code: None,
            }),
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
            owner: owner.clone(),
            name: function.name.clone(),
            descriptor,
            access: (if function.private { 2 } else { 1 }) | if method.jvm_static { 8 } else { 0 },
            unsupported: false,
            source_property: None,
        });
    }
    let mut fields = Vec::new();
    let mut object_fields = HashMap::new();
    for field in &source.fields {
        if !field.private || field.name == "INSTANCE" || object_fields.contains_key(&field.name) {
            return Err(Diagnostic::new(
                field.offset,
                "Object initializer fields must be unique private vals",
            ));
        }
        let ExprKind::Call(target, arguments) = &field
            .initializer
            .as_ref()
            .ok_or_else(|| Diagnostic::new(field.offset, "Property requires an initializer"))?
            .kind
        else {
            return Err(Diagnostic::new(
                field.offset,
                "Object fields require ThreadLocal.withInitial array suppliers",
            ));
        };
        let ExprKind::Property(receiver, name) = &target.kind else {
            return Err(Diagnostic::new(
                field.offset,
                "Object fields require ThreadLocal.withInitial array suppliers",
            ));
        };
        if !matches!(&receiver.kind, ExprKind::Name(name) if name == "ThreadLocal")
            || name != "withInitial"
            || arguments.len() != 1
            || imports.contains_key("ThreadLocal")
            || source.name == "ThreadLocal"
            || source.fields.iter().any(|f| f.name == "ThreadLocal")
            || object_methods.contains_key("ThreadLocal")
        {
            return Err(Diagnostic::new(
                field.offset,
                "Only unshadowed ThreadLocal.withInitial array suppliers are supported",
            ));
        }
        let ExprKind::Lambda(array) = &arguments[0].kind else {
            return Err(Diagnostic::new(
                field.offset,
                "ThreadLocal suppliers require a zero-argument array allocation lambda",
            ));
        };
        let ExprKind::Call(array, sizes) = &array.kind else {
            return Err(Diagnostic::new(
                field.offset,
                "ThreadLocal suppliers require a single array allocation",
            ));
        };
        let array_type = match &array.kind {
            ExprKind::Name(name) if name == "ByteArray" => Type::ByteArray,
            ExprKind::Name(name) if name == "CharArray" => Type::CharArray,
            _ => {
                return Err(Diagnostic::new(
                    field.offset,
                    "ThreadLocal suppliers support ByteArray and CharArray allocations",
                ));
            }
        };
        let ExprKind::Name(array_name) = &array.kind else {
            unreachable!()
        };
        if imports.contains_key(array_name)
            || source.name == *array_name
            || source.fields.iter().any(|field| field.name == *array_name)
            || object_methods.contains_key(array_name)
        {
            return Err(Diagnostic::new(
                field.offset,
                "Shadowed array constructors in ThreadLocal suppliers are not supported",
            ));
        }
        if sizes.len() != 1 {
            return Err(Diagnostic::new(
                field.offset,
                "Array allocation requires one size argument",
            ));
        }
        let empty_members = HashMap::new();
        let empty_fields = HashMap::new();
        let empty_object_fields = HashMap::new();
        let empty_methods = HashMap::new();
        let empty_functions = HashMap::new();
        let signatures = Vec::new();
        let extensions = HashSet::new();
        let mut checker = Checker {
            exhaustiveness_flow: false,
            lambdas: lambdas.clone(),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: owner.clone(),
            inline_result: None,
            nonlocal_result: None,
            constants: &HashMap::new(),
            constructor: None,
            defaults: &Vec::new(),
            function_names: &[],
            mutable_fields: &HashSet::new(),
            classpath,
            extensions: &extensions,
            classes,
            imports,
            signatures: &signatures,
            source_type_params: &[],
            source_infix: &HashSet::new(),
            source_operator: &HashSet::new(),
            receiver: None,
            fields: &empty_fields,
            members: &empty_members,
            companion_fields: &empty_object_fields,
            functions: &empty_functions,
            object_methods: &empty_methods,
            object_fields: &empty_object_fields,
            object_owner: None,
            allow_return: true,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: 0,
            result: array_type.clone(),
        };
        let size = checker.value(&sizes[0])?;
        checker.require(&size, Type::Int, sizes[0].offset)?;
        let ty = Type::ThreadLocal(Box::new(array_type.clone()));
        object_fields.insert(field.name.clone(), (owner.clone(), ty.clone()));
        fields.push(ObjectField {
            mutable: field.mutable,
            name: field.name.clone(),
            ty,
            initializer: ObjectInitializer::ThreadLocalArray {
                array_type,
                size,
                locals: checker.next,
            },
        });
    }
    let mut methods = Vec::new();
    let empty_fields = HashMap::new();
    let empty_members = HashMap::new();
    let empty_functions = HashMap::new();
    let companion_fields = HashMap::new();
    let signatures = Vec::new();
    let extensions = HashSet::new();
    for method in &source.methods {
        let function = &method.function;
        if function.defaults.iter().any(Option::is_some) {
            return Err(Diagnostic::new(
                function.offset,
                "Default arguments on object methods are unsupported",
            ));
        }
        let result = resolve_type_checked(
            &function.result,
            imports,
            function.offset,
            classpath,
            classes,
        )?;
        let mut checker = Checker {
            exhaustiveness_flow: false,
            lambdas: lambdas.clone(),
            captured: HashMap::new(),
            captured_cells: HashMap::new(),
            cells: HashMap::new(),
            cell_names: HashSet::new(),
            lambda_owner: owner.clone(),
            inline_result: None,
            nonlocal_result: None,
            constants: &HashMap::new(),
            constructor: None,
            defaults: &Vec::new(),
            function_names: &[],
            mutable_fields: &HashSet::new(),
            classpath,
            extensions: &extensions,
            classes,
            imports,
            signatures: &signatures,
            source_type_params: &[],
            source_infix: &HashSet::new(),
            source_operator: &HashSet::new(),
            receiver: if method.jvm_static {
                None
            } else {
                Some(Local {
                    slot: 0,
                    ty: Type::Java(owner.clone()),
                    mutable: false,
                })
            },
            fields: &empty_fields,
            members: &empty_members,
            companion_fields: &companion_fields,
            functions: &empty_functions,
            object_methods: &object_methods,
            object_fields: &object_fields,
            object_owner: Some(&owner),
            allow_return: true,
            refinements: HashMap::new(),
            initialized: HashSet::new(),
            loop_depth: 0,
            scopes: vec![HashMap::new()],
            next: if method.jvm_static { 0 } else { 1 },
            result: result.clone(),
        };
        let mut params = Vec::new();
        for (name, ty) in &function.params {
            let ty = resolve_type_checked(ty, imports, function.offset, classpath, classes)?;
            checker.declare(name, ty.clone(), false, function.offset)?;
            params.push((name.clone(), ty));
        }
        let (mut body, returns) = checker.block(&function.body)?;
        if !returns {
            if result != Type::Unit {
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
        let mut throws = Vec::new();
        for name in &method.throws {
            if source.name == *name
                || source.fields.iter().any(|field| field.name == *name)
                || object_methods.contains_key(name)
            {
                return Err(Diagnostic::new(
                    function.offset,
                    "Throws exception class literals cannot be shadowed by object declarations",
                ));
            }
            let owner = imports.get(name).map(String::as_str).unwrap_or(name);
            let owner = exception_owner(owner).ok_or_else(|| {
                Diagnostic::new(
                    function.offset,
                    "Throws annotations require supported standard exception classes",
                )
            })?;
            if throws.iter().any(|previous| previous == owner) {
                return Err(Diagnostic::new(
                    function.offset,
                    "Duplicate Throws exception class",
                ));
            }
            throws.push(owner.to_owned());
        }
        methods.push(ObjectMethod {
            function: Function {
                class_type_params: Vec::new(),
                jvm_result: None,
                jvm_name: None,
                annotations: Vec::new(),
                vararg: false,
                type_params: resolve_parameters(
                    &function.type_params,
                    imports,
                    classpath,
                    function.offset,
                    classes,
                )?,
                defaults: Vec::new(),
                name: function.name.clone(),
                private: function.private,
                is_override: false,
                params,
                result,
                locals: checker.next,
                body,
            },
            jvm_static: method.jvm_static,
            throws,
            inline: method.inline,
        });
    }
    Ok(Module {
        exposed_fields: HashMap::new(),
        properties: Vec::new(),
        initializers: Vec::new(),
        locals: 0,
        lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
        constants: Vec::new(),
        functions: Vec::new(),
        class_decl: None,
        interface_decl: None,
        object_decl: Some(DynamicObject {
            bridges: Vec::new(),
            exposed_fields: HashMap::new(),
            constants: source.constants.clone(),
            initializers: Vec::new(),
            locals: 0,
            owner,
            interfaces: Vec::new(),
            serializable: false,
            fields,
            methods,
        }),
    })
}

pub(super) fn infer_legacy_field(
    source: &syntax::ObjectDecl,
    field: &syntax::ObjectField,
    package: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
) -> Result<Type> {
    let mut isolated = source.clone();
    isolated.fields = vec![field.clone()];
    isolated.methods.clear();
    isolated.constants.clear();
    isolated.objects.clear();
    let checked = legacy_check(&isolated, package, classpath, imports, classes)?;
    Ok(checked.object_decl.unwrap().fields.pop().unwrap().ty)
}

pub(super) fn check(
    source: &syntax::ObjectDecl,
    package: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    owner_override: Option<&str>,
) -> Result<Module> {
    let owner = owner_override.map(str::to_owned).unwrap_or_else(|| {
        if package.is_empty() {
            source.name.clone()
        } else {
            format!("{}/{}", package.replace('.', "/"), source.name)
        }
    });
    let lambdas = Rc::new(RefCell::new(Vec::new()));
    let mut context =
        declarations::DeclarationContext::new(&owner, classpath, imports, classes, &lambdas);
    for constant in &source.constants {
        let (ty, kind) = constant_expr(&constant.value)?;
        context.constants.insert(
            constant.name.clone(),
            (
                ty,
                Expr {
                    kind,
                    offset: constant.offset,
                },
            ),
        );
    }
    for method in &source.methods {
        let function = &method.function;
        if method.jvm_static && method.is_override {
            return Err(Diagnostic::new(
                function.offset,
                "A JvmStatic method cannot implement an instance override",
            ));
        }
        for (name, active) in [
            ("JvmStatic", method.jvm_static),
            ("Throws", !method.throws.is_empty()),
            ("Suppress", !method.suppress.is_empty()),
        ] {
            if active && (source.name == name || imports.contains_key(name)) {
                return Err(Diagnostic::new(
                    function.offset,
                    "Shadowed builtin object annotations are not supported",
                ));
            }
        }
        for name in &method.throws {
            if source.name == *name
                || source.fields.iter().any(|f| f.name == *name)
                || source.methods.iter().any(|m| m.function.name == *name)
            {
                return Err(Diagnostic::new(
                    function.offset,
                    "Throws exception class literals cannot be shadowed by object declarations",
                ));
            }
        }
        if method.inline && !function.private {
            return Err(Diagnostic::new(
                function.offset,
                "Only private inline object methods are supported",
            ));
        }

        if function.receiver.is_some() {
            return Err(Diagnostic::new(
                function.offset,
                "Member extensions are unsupported",
            ));
        }
        let params = function
            .params
            .iter()
            .map(|(_, t)| resolve_type_checked(t, imports, function.offset, classpath, classes))
            .collect::<Result<Vec<_>>>()?;
        let result = resolve_type_checked(
            &function.result,
            imports,
            function.offset,
            classpath,
            classes,
        )?;
        let descriptor = format!(
            "({}){}",
            params.iter().map(Type::descriptor).collect::<String>(),
            result.descriptor()
        );
        let methods = context
            .object_methods
            .entry(function.name.clone())
            .or_default();
        if methods
            .iter()
            .any(|m| m.descriptor.split(')').next() == descriptor.split(')').next())
        {
            return Err(Diagnostic::new(
                function.offset,
                "Conflicting object method parameter signature",
            ));
        }
        methods.push(JavaMethod {
            owner: owner.clone(),
            name: function.name.clone(),
            descriptor,
            access: (if function.private { 2 } else { 1 }) | if method.jvm_static { 8 } else { 0 },
            unsupported: false,
            source_property: None,
            kotlin_params: Some(
                function
                    .params
                    .iter()
                    .zip(&params)
                    .enumerate()
                    .map(|(i, ((n, _), ty))| {
                        (
                            n.clone(),
                            ty.clone(),
                            function.defaults.get(i).is_some_and(Option::is_some),
                        )
                    })
                    .collect(),
            ),
            kotlin_result: Some(result),
            kotlin_internal: function.internal,
            library: Some(crate::classpath::LibraryMethod {
                abstract_: false,
                builtin_array: false,
                source_name: function.name.clone(),
                type_parameters: function.type_params.clone(),
                receiver: None,
                infix: function.infix,
                operator: function.operator,
                kotlin_public: !function.private && !function.protected && !function.internal,
                inline: method.inline,
                code: None,
            }),
        });
    }
    let mut implemented_interfaces = Vec::new();
    let mut required = HashMap::new();
    for name in &source.interfaces {
        let name = imports.get(name).cloned().unwrap_or_else(|| name.clone());
        let class = classes
            .get(&name)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| classpath.load(&name))?;
        for method in interfaces::interface_methods(&class, classes, source.offset)?.into_values() {
            required.insert(
                declarations::member_key(&method.name, &method.descriptor),
                method,
            );
        }
        implemented_interfaces.push(name);
    }
    if let Ok(object) = classpath.load("java/lang/Object") {
        for method in object.methods {
            if source
                .methods
                .iter()
                .any(|m| m.function.name == method.name)
                && method.access & 8 == 0
            {
                required.insert(
                    declarations::member_key(&method.name, &method.descriptor),
                    method,
                );
            }
        }
    }
    for method in required.values() {
        context.members.insert(
            declarations::member_key(&method.name, &method.descriptor),
            method.clone(),
        );
    }
    let mut fields = Vec::new();
    let mut initializers = Vec::new();
    let mut locals = 0;
    let mut delegated_accessors = Vec::new();
    let mut ordered = source
        .fields
        .iter()
        .map(|f| (f.offset, Some(f), None))
        .chain(source.initializers.iter().map(|(o, b)| (*o, None, Some(b))))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|entry| entry.0);
    for (_, field, block) in ordered {
        if let Some(block) = block {
            let mut checker = context.checker(false, locals, Type::Unit);
            checker.allow_return = false;
            let (body, _) = checker.block(block)?;
            locals = checker.next;
            initializers.extend(body);
            continue;
        }
        let field = field.unwrap();
        if field.name == "INSTANCE" || context.static_fields.contains_key(&field.name) {
            return Err(Diagnostic::new(field.offset, "Conflicting object property"));
        }
        if field.delegate.is_some() {
            let property = syntax::PropertyDecl {
                annotations: field.annotations.clone(),
                name: field.name.clone(),
                private: field.private,
                protected: false,
                mutable: field.mutable,
                inferred: field.ty.is_none(),
                initializer: None,
                offset: field.offset,
                ty: field.ty.clone().unwrap_or(Type::Unit),
                body: field.body.clone(),
                is_override: field.is_override,
                setter: field.setter.clone(),
                delegate: field.delegate.clone(),
                lateinit: field.lateinit,
                open: false,
            };
            let delegated = delegates::check_property(
                &property,
                &context,
                &owner,
                &owner,
                true,
                true,
                &[],
                locals,
            )?;
            let metadata_type = delegated.metadata.ty.clone();
            initializers.push(Statement::JavaFieldStore {
                owner: owner.clone(),
                name: delegated.metadata_name.clone(),
                ty: metadata_type.clone(),
                receiver: None,
                value: delegated.metadata,
            });
            fields.push(ObjectField {
                name: delegated.metadata_name,
                ty: metadata_type.clone(),
                mutable: false,
                initializer: ObjectInitializer::Value {
                    value: default_value(&metadata_type),
                    locals: 0,
                },
            });
            let delegate_type = delegated.field.ty.clone();
            initializers.push(Statement::JavaFieldStore {
                owner: owner.clone(),
                name: delegated.field.name.clone(),
                ty: delegate_type.clone(),
                receiver: None,
                value: delegated.field.initializer.unwrap(),
            });
            fields.push(ObjectField {
                name: delegated.field.name,
                ty: delegate_type.clone(),
                mutable: false,
                initializer: ObjectInitializer::Value {
                    value: default_value(&delegate_type),
                    locals: 0,
                },
            });
            context
                .static_fields
                .insert(field.name.clone(), (owner.clone(), delegated.property_type));
            if field.mutable {
                context.mutable_fields.insert(field.name.clone());
            }
            locals = delegated.locals;
            delegated_accessors.extend(delegated.methods);
            continue;
        }
        let thread_local = field.initializer.as_ref().is_some_and(|e|matches!(&e.kind,ExprKind::Call(target,_) if matches!(&target.kind,ExprKind::Property(receiver,name) if name=="withInitial" && matches!(&receiver.kind,ExprKind::Name(n) if n=="ThreadLocal"))));
        if thread_local
            && !source.methods.iter().any(|m| {
                matches!(
                    m.function.name.as_str(),
                    "ByteArray" | "CharArray" | "IntArray"
                )
            })
            && field
                .ty
                .as_ref()
                .is_none_or(|ty| matches!(ty, Type::ThreadLocal(_)))
        {
            let mut isolated = source.clone();
            isolated.fields = vec![field.clone()];
            isolated.methods.clear();
            isolated.constants.clear();
            isolated.objects.clear();
            if let Ok(checked) = legacy_check(&isolated, package, classpath, imports, classes) {
                let mut checked = checked.object_decl.unwrap().fields;
                let field = checked.pop().unwrap();
                context
                    .static_fields
                    .insert(field.name.clone(), (owner.clone(), field.ty.clone()));
                fields.push(field);
                continue;
            }
        }
        context
            .static_fields
            .entry(field.name.clone())
            .or_insert_with(|| (owner.clone(), Type::Unit));
        let mut checker = context.checker(false, locals, Type::Unit);
        checker.allow_return = false;
        let value = field
            .initializer
            .as_ref()
            .map(|e| {
                if let Some(ty) = &field.ty {
                    let ty = resolve_type_checked(ty, imports, field.offset, classpath, classes)?;
                    checker.expected_value(e, &ty)
                } else {
                    checker.value(e)
                }
            })
            .transpose()?;
        let ty = if let Some(ty) = &field.ty {
            resolve_type_checked(ty, imports, field.offset, classpath, classes)?
        } else {
            value.as_ref().map(|v| v.ty.clone()).ok_or_else(|| {
                Diagnostic::new(field.offset, "Property requires a type or initializer")
            })?
        };
        if ty == Type::Unit || ty == Type::Null {
            return Err(Diagnostic::new(
                field.offset,
                "Property requires a supported type",
            ));
        }
        if field.lateinit
            && (!field.mutable
                || !ty.is_reference()
                || ty.is_nullable()
                || field.initializer.is_some())
        {
            return Err(Diagnostic::new(
                field.offset,
                "lateinit requires an uninitialized mutable non-null reference property",
            ));
        }
        if let Some(value) = value {
            let value = checker.coerce(value, ty.clone(), field.offset)?;
            initializers.push(Statement::JavaFieldStore {
                owner: owner.clone(),
                name: field.name.clone(),
                ty: ty.clone(),
                receiver: None,
                value,
            });
        }
        locals = checker.next;
        drop(checker);
        context
            .static_fields
            .insert(field.name.clone(), (owner.clone(), ty.clone()));
        if field.mutable {
            context.mutable_fields.insert(field.name.clone());
        }
        fields.push(ObjectField {
            name: field.name.clone(),
            ty: ty.clone(),
            mutable: field.mutable,
            initializer: ObjectInitializer::Value {
                value: default_value(&ty),
                locals: 0,
            },
        });
    }
    for field in &source.fields {
        let ty = context.static_fields[&field.name].1.clone();
        for (prefix, name, result, params, active) in [
            (
                "#get:",
                getter_name(&field.name),
                ty.clone(),
                Vec::<(String, Type)>::new(),
                !field.body.is_empty() || field.lateinit || field.delegate.is_some(),
            ),
            (
                "#set:",
                setter_name(&field.name),
                Type::Unit,
                vec![("value".into(), ty.clone())],
                field.setter.is_some() || field.delegate.is_some(),
            ),
        ] {
            if active {
                context.members.insert(
                    format!("{prefix}{}", field.name),
                    JavaMethod {
                        library: None,
                        owner: owner.clone(),
                        name,
                        descriptor: format!(
                            "({}){}",
                            params.iter().map(|p| p.1.descriptor()).collect::<String>(),
                            result.descriptor()
                        ),
                        access: if field.private { 2 } else { 1 },
                        unsupported: false,
                        source_property: Some(field.name.clone()),
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
    let mut methods = delegated_accessors
        .into_iter()
        .map(|function| ObjectMethod {
            function,
            jvm_static: false,
            throws: Vec::new(),
            inline: false,
        })
        .collect::<Vec<_>>();
    for method in &source.methods {
        let function =
            context.function(&method.function, !method.jvm_static, method.is_override)?;
        if function
            .annotations
            .iter()
            .any(|annotation| annotation.descriptor == "Lkotlin/jvm/JvmStatic;")
        {
            return Err(Diagnostic::new(
                method.function.offset,
                "Qualified or aliased JvmStatic object annotations are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "JVM_STATIC_ANNOTATION_FORM_UNSUPPORTED",
            ));
        }
        for descriptor in annotations::overload_descriptors(&function) {
            if required.contains_key(&declarations::member_key(&function.name, &descriptor)) {
                return Err(Diagnostic::new(
                    method.function.offset,
                    "Generated JVM overload clashes with an inherited declaration",
                )
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "JVM_SIGNATURE_CLASH"));
            }
        }
        let inherited_key = declarations::member_key(&function.name, &function.descriptor());
        if let Some(parent) = required.get(&inherited_key) {
            if !method.is_override || parent.access & 16 != 0 {
                return Err(Diagnostic::new(
                    method.function.offset,
                    "Inherited method implementation requires override of an open member",
                ));
            }
        } else if method.is_override {
            return Err(Diagnostic::new(
                method.function.offset,
                "Member overrides no inherited declaration",
            ));
        }
        if let Some(parent) = required.get(&inherited_key) {
            if function.descriptor() != parent.descriptor
                || parent.source_property.is_some()
                || !declarations::override_signature(
                    parent,
                    &function
                        .params
                        .iter()
                        .map(|p| p.1.clone())
                        .collect::<Vec<_>>(),
                    &function.result,
                )?
            {
                return Err(Diagnostic::new(
                    method.function.offset,
                    "Object override requires an exact supported inherited signature",
                ));
            }
        }
        let throws = method
            .throws
            .iter()
            .map(|name| {
                exception_owner(imports.get(name).map(String::as_str).unwrap_or(name))
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            method.function.offset,
                            "Unsupported Throws annotation class",
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        methods.push(ObjectMethod {
            function,
            jvm_static: method.jvm_static,
            throws,
            inline: method.inline,
        });
    }
    for field in &source.fields {
        if field.delegate.is_some() {
            continue;
        }
        let ty = context.static_fields[&field.name].1.clone();
        if field.private && field.body.is_empty() && field.setter.is_none() && !field.lateinit {
            continue;
        }
        if let Some(required) =
            required.get(&declarations::member_key(&getter_name(&field.name), "()V"))
        {
            if !field.is_override || class_method_types(required)? != (Vec::new(), ty.clone()) {
                return Err(Diagnostic::new(
                    field.offset,
                    "Interface property implementation requires override and an exact signature",
                ));
            }
        }
        let mut accessor_context = context.clone();
        accessor_context.members.insert(
            "#backing".into(),
            JavaMethod {
                library: None,
                owner: owner.clone(),
                name: field.name.clone(),
                descriptor: format!("(){}", ty.descriptor()),
                access: 8,
                unsupported: false,
                source_property: None,
                kotlin_params: None,
                kotlin_result: None,
                kotlin_internal: false,
            },
        );
        let source_property = syntax::ClassCompanionProperty {
            annotations: field.annotations.clone(),
            name: field.name.clone(),
            offset: field.offset,
            private: field.private,
            mutable: field.mutable,
            ty: Some(ty.clone()),
            initializer: field.initializer.clone(),
            body: field.body.clone(),
            setter: field.setter.clone(),
            delegate: field.delegate.clone(),
            lateinit: field.lateinit,
        };
        let read = Value {
            ty: ty.clone(),
            kind: ValueKind::StaticField {
                owner: owner.clone(),
                name: field.name.clone(),
            },
        };
        let body = if field.lateinit {
            declarations::lateinit_getter(read, &field.name)
        } else {
            vec![Statement::Return(read)]
        };
        let function = if field.body.is_empty() {
            Function {
                class_type_params: Vec::new(),
                jvm_result: None,
                jvm_name: None,
                annotations: Vec::new(),
                vararg: false,
                type_params: Vec::new(),
                defaults: Vec::new(),
                name: getter_name(&field.name),
                private: field.private,
                is_override: field.is_override,
                params: Vec::new(),
                result: ty.clone(),
                locals: 1,
                body,
            }
        } else {
            accessor_context.function(
                &declarations::accessor_source(
                    getter_name(&field.name),
                    &source_property,
                    Vec::new(),
                    ty.clone(),
                    field.body.clone(),
                ),
                true,
                field.is_override,
            )?
        };
        methods.push(ObjectMethod {
            function,
            jvm_static: false,
            throws: Vec::new(),
            inline: false,
        });
        if field.mutable {
            if let Some((parameter, body)) = &field.setter {
                let function = accessor_context.function(
                    &declarations::accessor_source(
                        setter_name(&field.name),
                        &source_property,
                        vec![(parameter.clone(), ty.clone())],
                        Type::Unit,
                        body.clone(),
                    ),
                    true,
                    field.is_override,
                )?;
                methods.push(ObjectMethod {
                    function,
                    jvm_static: false,
                    throws: Vec::new(),
                    inline: false,
                });
                continue;
            }
            methods.push(ObjectMethod {
                function: Function {
                    class_type_params: Vec::new(),
                    jvm_result: None,
                    jvm_name: None,
                    annotations: Vec::new(),
                    vararg: false,
                    type_params: Vec::new(),
                    defaults: Vec::new(),
                    name: setter_name(&field.name),
                    private: false,
                    is_override: false,
                    params: vec![("value".into(), ty.clone())],
                    result: Type::Unit,
                    locals: 1 + ty.slots() as u16,
                    body: vec![
                        Statement::JavaFieldStore {
                            owner: owner.clone(),
                            name: field.name.clone(),
                            ty: ty.clone(),
                            receiver: None,
                            value: Value {
                                ty,
                                kind: ValueKind::Local(1),
                            },
                        },
                        Statement::Return(Value {
                            ty: Type::Unit,
                            kind: ValueKind::Unit,
                        }),
                    ],
                },
                jvm_static: false,
                throws: Vec::new(),
                inline: false,
            });
        }
    }
    let mut bridges = Vec::new();
    for method in required.into_values() {
        if methods
            .iter()
            .any(|m| m.function.name == method.name && m.function.descriptor() == method.descriptor)
        {
            continue;
        }
        if method.access & 0x0400 != 0 {
            return Err(Diagnostic::new(
                source.offset,
                format!("Missing interface implementation: {}", method.name),
            ));
        }
        if classpath
            .load(&method.owner)
            .is_ok_and(|c| c.kotlin_interface)
        {
            bridges.push(declarations::direct_default_bridge(
                method,
                &implemented_interfaces,
                classpath,
                source.offset,
            )?);
        }
    }
    bridges.sort_by(|a, b| a.name.cmp(&b.name).then(a.descriptor.cmp(&b.descriptor)));
    drop(context);
    Ok(Module {
        exposed_fields: HashMap::new(),
        properties: Vec::new(),
        initializers: Vec::new(),
        locals: 0,
        lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
        constants: source.constants.clone(),
        functions: Vec::new(),
        class_decl: None,
        interface_decl: None,
        object_decl: Some(DynamicObject {
            bridges,
            exposed_fields: source
                .fields
                .iter()
                .filter(|f| f.lateinit)
                .map(|f| (f.name.clone(), if f.private { 2 } else { 1 }))
                .collect(),
            constants: source.constants.clone(),
            owner,
            interfaces: implemented_interfaces,
            serializable: false,
            fields,
            methods,
            initializers,
            locals,
        }),
    })
}

pub(super) fn default_value(ty: &Type) -> Value {
    let kind = match ty {
        Type::Long => ValueKind::Long(0),
        Type::Float => ValueKind::Float(0),
        Type::Double => ValueKind::Double(0),
        Type::Boolean => ValueKind::Boolean(false),
        Type::Char => ValueKind::Int(0),
        Type::Int | Type::Byte => ValueKind::Int(0),
        _ => ValueKind::Null,
    };
    Value {
        ty: ty.clone(),
        kind,
    }
}

fn exception_owner(name: &str) -> Option<&'static str> {
    match name {
        "IllegalArgumentException" | "java/lang/IllegalArgumentException" => {
            Some("java/lang/IllegalArgumentException")
        }
        "IndexOutOfBoundsException" | "java/lang/IndexOutOfBoundsException" => {
            Some("java/lang/IndexOutOfBoundsException")
        }
        "IllegalStateException" | "java/lang/IllegalStateException" => {
            Some("java/lang/IllegalStateException")
        }
        "UnsupportedOperationException" | "java/lang/UnsupportedOperationException" => {
            Some("java/lang/UnsupportedOperationException")
        }
        "java/io/IOException" => Some("java/io/IOException"),
        _ => None,
    }
}

impl Checker<'_> {
    pub(super) fn unshadowed(&self, name: &str) -> bool {
        self.local(name).is_none() && !self.constants.contains_key(name) && !self.functions.contains_key(name) && !self.object_methods.contains_key(name)
            && !self.fields.contains_key(name) && !self.object_fields.contains_key(name) && !self.members.contains_key(name)
            && !self.members.values().any(|method| method.source_property.as_deref() == Some(name))
            && !self.companion_fields.contains_key(name) && !self.imports.contains_key(name)
            && self.object_owner.is_none_or(|owner| owner.rsplit(['/', '$']).next() != Some(name))
            && !self.receiver.as_ref().is_some_and(|receiver| matches!(receiver.ty.base(), Type::Java(owner) if owner.rsplit(['/', '$']).next() == Some(name)))
    }
    pub(super) fn object_call(
        &mut self,
        name: &str,
        args: &[Expr],
        receiver: Option<Value>,
        offset: usize,
    ) -> Result<Value> {
        let mut options = self.object_methods[name].clone();
        for inherited in self
            .members
            .values()
            .filter(|method| method.source_name() == name)
        {
            if !options.iter().any(|method| {
                declarations::member_key(&method.name, &method.descriptor)
                    == declarations::member_key(&inherited.name, &inherited.descriptor)
            }) {
                options.push(inherited.clone());
            }
        }
        let method = self.select_method(&options, args, offset)?;
        if method.access & 8 != 0 {
            let call = self.library_static_call(method, args, offset)?;
            if let Some(receiver) = receiver {
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
            Ok(call)
        } else {
            let receiver = if let Some(receiver) = receiver {
                receiver
            } else {
                self.receiver_value(offset)?
            };
            self.member_call(method, receiver, args, offset)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checked(source: &str) -> Result<Module> {
        let source = syntax::parse(source)?;
        super::super::check(
            &source,
            &Classpath::new(&[std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("references/kotlinc/lib/kotlin-stdlib.jar")])?,
        )
    }
    fn rejects(source: &str, message: &str) {
        let error = checked(source)
            .err()
            .unwrap_or_else(|| panic!("expected rejection: {source}"));
        assert!(error.message.contains(message), "{}", error.message);
    }
    const SCRATCH: &str = r#"
object Scratch {
    private val chars = ThreadLocal.withInitial { CharArray(4) }
    @JvmStatic
    fun convert(input: ByteArray): String = convert(input, 0, input.size)
    @JvmStatic
    fun convert(input: ByteArray, start: Int, count: Int): String {
        var buffer = chars.get()!!
        if (buffer.size < count) { buffer = buffer.copyOf(count); chars.set(buffer) }
        var pos = 0
        for (index in start until start + count) {
            val byte = input[index].toInt() and 0xff
            if (byte == 0) { continue }
            buffer[pos++] = (if (byte in 65..90) { byte or 0x20 } else { byte }).toChar()
        }
        return String(buffer, 0, pos)
    }
}
"#;
    #[test]
    fn object_buffers_and_exact_overloads_share_regular_expression_checking() {
        let module = checked(SCRATCH).unwrap();
        let object = module.object_decl.unwrap();
        assert_eq!(object.owner, "Scratch");
        assert_eq!(
            object.fields[0].ty,
            Type::ThreadLocal(Box::new(Type::CharArray))
        );
        assert_eq!(
            object.methods[0].function.descriptor(),
            "([B)Ljava/lang/String;"
        );
        assert_eq!(
            object.methods[1].function.descriptor(),
            "([BII)Ljava/lang/String;"
        );
        assert!(object.methods.iter().all(|method| method.jvm_static));
        assert!(object.methods[1].function.locals > 5);
        rejects(
            &SCRATCH.replace(
                "convert(input, 0, input.size)",
                "convert(input, true, input.size)",
            ),
            "No applicable",
        );
        rejects(
            &SCRATCH.replace("convert(input, 0, input.size)", "convert(input, 0)"),
            "No applicable",
        );
    }
    #[test]
    fn uninitialized_locals_require_assignment_on_every_reachable_branch() {
        for source in [
            "fun f(flag: Boolean): Int { var x: Int; if (flag) { x = 1 } else { x = 2 }; return x }",
            "fun f(flag: Boolean): Int { var x: Int; if (flag) { x = 1 } else { return 2 }; return x }",
            "fun f(n: Int): Int { var x: Int; when (n) { 1 -> { x = 1 }; else -> { x = 2 } }; return x }",
            "fun f(n: Int): Int { var result = 0; var i = 0; while (i < n) { i++; var x: Int; if (i == 1) { x = 2 } else { continue }; result += x }; return result }",
        ] {
            assert!(checked(source).is_ok(), "{source}");
        }
        for source in [
            "fun f(): Int { var x: Int; return x }",
            "fun f(flag: Boolean): Int { var x: Int; if (flag) { x = 1 }; return x }",
            "fun f(flag: Boolean): Int { var x: Int; while (flag) { x = 1 }; return x }",
            "fun f(n: Int): Int { var x: Int; when (n) { 1 -> { x = 1 }; else -> {} }; return x }",
            "fun f(n: Int): Int { var x: Int; when (n) { 1 -> { x = 1 }; 2 -> { return x }; else -> { return 0 } }; return x }",
            "fun f(): Int { var x: Int; x += 1; return x }",
        ] {
            rejects(source, "initialized");
        }
        rejects(
            "fun f(): Int { return 1; var x: Int; x = 2 }",
            "Unreachable",
        );
    }
    #[test]
    fn loop_control_and_indices_keep_types_and_scope() {
        assert!(checked("fun f(a: ByteArray): Int { var p = 0; while (p < a.size) { a[p++] = 1.toByte(); if (p == 2) { break } }; return p }").is_ok());
        rejects("fun f() { continue }", "enclosing loop");
        rejects("fun f() { break }", "enclosing loop");
        rejects(
            "fun f() { for (index in 0 until 2) { index++ } }",
            "mutable Int",
        );
        rejects("fun f(a: ByteArray) { a[0] = 128 }", "Type mismatch");
        rejects("fun f(a: CharArray) { a[0] = 65 }", "Type mismatch");
        rejects(
            "fun f(): Int { val index = 0; return index++ }",
            "mutable Int",
        );
    }
    #[test]
    fn suppliers_and_builtin_annotation_shadowing_fail_closed() {
        assert!(checked("object Scratch { private val buffer = ThreadLocal.withInitial { ByteArray(4) }; fun ByteArray(size: Int): CharArray = CharArray(size) }").is_ok());
        assert!(checked("object Scratch { private val ByteArray = ThreadLocal.withInitial { ByteArray(4) } }").is_err());
        rejects(
            "object Scratch { private val x = ThreadLocal.withInitial { missing } }",
            "Unresolved",
        );
        rejects(
            "object Scratch { private val x = ThreadLocal.withInitial { ByteArray(missing) } }",
            "Unresolved",
        );
        rejects(
            "object JvmStatic { @JvmStatic fun f(): Int = 1 }",
            "Shadowed builtin",
        );
        rejects(
            "object Throws { @Throws(IllegalArgumentException::class) fun f(): Int = 1 }",
            "Shadowed builtin",
        );
        assert!(checked("object Scratch { internal fun f(): Int = 1 }").is_ok());
        rejects(
            "object Scratch { inline fun f(): Int = 1 }",
            "private inline",
        );
    }
    #[test]
    fn exceptions_conversions_and_range_expressions_have_bounded_types() {
        let object = checked("object Scratch { @JvmStatic @Throws(IllegalArgumentException::class) fun f() { throw IllegalArgumentException() } }").unwrap().object_decl.unwrap();
        assert_eq!(
            object.methods[0].throws,
            ["java/lang/IllegalArgumentException"]
        );
        assert!(checked("fun f(a: ByteArray): Int = a[0].toInt() and 0xff").is_ok());
        assert!(checked("fun f(value: Int): Char = Character.highSurrogate(value)").is_ok());
        assert!(
            checked("fun f(value: Int): Int = when (value) { in 1..4 -> 2; 8 -> 3; else -> 4 }")
                .is_ok()
        );
        assert!(checked("fun f(value: Char): Boolean = value in 'A'..'Z'").is_ok());
        assert!(
            checked("fun f(): IntArray { var result: ThreadLocal<ByteArray>; return IntArray(1) }")
                .is_ok()
        );
        rejects(
            "fun f(value: Boolean): Boolean = value in true..false",
            "Ranges require",
        );
    }
    #[test]
    fn explicit_receivers_of_static_object_methods_are_evaluated() {
        let module = checked(r#"object Scratch { private fun ping(): Boolean { throw IllegalArgumentException("receiver") }; @JvmStatic fun f(): Int = 1; @JvmStatic fun run(): Int = (if (ping()) { this } else { this }).f() }"#).unwrap();
        let object = module.object_decl.unwrap();
        let Statement::Return(value) = &object.methods[2].function.body[0] else {
            panic!()
        };
        let ValueKind::Let {
            value: receiver,
            body: call,
            ..
        } = &value.kind
        else {
            panic!("static calls must preserve explicit receivers")
        };
        assert!(matches!(receiver.kind, ValueKind::If(_, _, _)));
        assert!(
            matches!(&call.kind, ValueKind::StaticCall { .. })
                || matches!(&call.kind,ValueKind::JavaCall{method,..} if method.access&8!=0&&method.name=="f")
        );
    }
    #[test]
    fn object_and_class_overloads_do_not_override_unrelated_inherited_signatures() {
        let inherited = "open class Parent { open fun choice(value:Int):Int=value; open fun choice(value:String):Int=value.length }; class Child:Parent(){override fun choice(value:Int):Int=super.choice(value)+1; override fun choice(value:String):Int=super.choice(value)+2}";
        assert!(crate::compile_all(inherited, "InheritedOverloads.kt").is_ok());
        for source in [
            "object Text { @JvmStatic fun toString(bytes:ByteArray):String=String(CharArray(bytes.size),0,bytes.size) }",
            "object Text { override fun toString():String=\"text\"; fun toString(value:Int):String=value.toString(); fun current():String=toString() }",
            "class Text { override fun toString():String=\"text\"; fun toString(value:Int):String=value.toString(); fun current():String=toString() }",
        ] {
            assert!(checked(source).is_ok(), "{source}");
        }
        for source in [
            "object Text { fun toString():String=\"text\" }",
            "object Text { override fun toString(value:Int):String=value.toString() }",
            "class Text { override fun notify() {} }",
        ] {
            assert!(checked(source).is_err(), "{source}");
        }
    }
    #[test]
    fn inferred_thread_local_buffers_support_structural_reference_equality() {
        let object = checked("object Scratch { private val buffer = ThreadLocal.withInitial { ByteArray(1) }; @JvmStatic fun same(): Boolean = buffer == buffer }").unwrap().object_decl.unwrap();
        let Statement::Return(value) = &object.methods[0].function.body[0] else {
            panic!()
        };
        assert!(
            matches!(&value.kind, ValueKind::StaticCall { owner, name, descriptor, .. } if owner == "kotlin/jvm/internal/Intrinsics" && name == "areEqual" && descriptor == "(Ljava/lang/Object;Ljava/lang/Object;)Z")
        );
    }
    #[test]
    fn throws_class_literals_cannot_resolve_to_values_or_the_declared_object() {
        rejects(
            "object Scratch { private val IllegalArgumentException = ThreadLocal.withInitial { ByteArray(1) }; @JvmStatic @Throws(IllegalArgumentException::class) fun f(): Int = 1 }",
            "class literals cannot be shadowed",
        );
        rejects(
            "object IllegalArgumentException { @Throws(IllegalArgumentException::class) fun f(): Int = 1 }",
            "class literals cannot be shadowed",
        );
    }
    #[test]
    fn object_reference_equality_and_instance_calls_keep_the_actual_receiver() {
        for source in [
            "object Scratch { @JvmStatic fun same(): Boolean = this == this }",
            "object Scratch { fun same(): Boolean = this == this }",
        ] {
            let object = checked(source).unwrap().object_decl.unwrap();
            let Statement::Return(value) = &object.methods[0].function.body[0] else {
                panic!()
            };
            assert!(
                matches!(&value.kind, ValueKind::StaticCall { name, descriptor, .. } if name == "areEqual" && descriptor == "(Ljava/lang/Object;Ljava/lang/Object;)Z")
            );
        }
        let object = checked("object Scratch { private fun value(): Int = 1; fun instance(): Int = value(); @JvmStatic fun static(): Int = value() }").unwrap().object_decl.unwrap();
        let Statement::Return(instance) = &object.methods[1].function.body[0] else {
            panic!()
        };
        let Statement::Return(static_call) = &object.methods[2].function.body[0] else {
            panic!()
        };
        let (ValueKind::MemberCall { args: instance, .. }
        | ValueKind::JavaCall { args: instance, .. }) = &instance.kind
        else {
            panic!()
        };
        let (ValueKind::MemberCall {
            args: static_args, ..
        }
        | ValueKind::JavaCall {
            args: static_args, ..
        }) = &static_call.kind
        else {
            panic!()
        };
        assert!(matches!(instance[0].kind, ValueKind::Local(0)));
        assert!(matches!(static_args[0].kind, ValueKind::ObjectInstance(_)));
    }
    #[test]
    fn standalone_range_bounds_precede_subject_but_when_subject_is_cached_first() {
        let object = checked("object Scratch { private fun subject(): Int = 2; private fun low(): Int = 1; private fun high(): Int = 3; @JvmStatic fun contains(): Boolean = subject() in low()..high(); @JvmStatic fun selected(): Int = when (subject()) { in low()..high() -> 1; else -> 0 } }").unwrap().object_decl.unwrap();
        let Statement::Return(contains) = &object.methods[3].function.body[0] else {
            panic!()
        };
        fn calls(value: &Value, names: &mut Vec<String>) {
            match &value.kind {
                ValueKind::Let { value, body, .. } => {
                    calls(value, names);
                    calls(body, names);
                }
                ValueKind::New { args, .. } | ValueKind::StaticCall { args, .. } => {
                    for arg in args {
                        calls(arg, names);
                    }
                }
                ValueKind::JavaCall { method, args }
                | ValueKind::MemberCall { method, args, .. } => {
                    for arg in args {
                        calls(arg, names);
                    }
                    if matches!(method.name.as_str(), "subject" | "low" | "high") {
                        names.push(method.name.clone());
                    }
                }
                ValueKind::If(condition, yes, no) => {
                    calls(condition, names);
                    calls(yes, names);
                    calls(no, names);
                }
                _ => {}
            }
        }
        let mut names = Vec::new();
        calls(contains, &mut names);
        assert_eq!(names, ["low", "high", "subject"]);
        let Statement::Return(selected) = &object.methods[4].function.body[0] else {
            panic!()
        };
        let mut names = Vec::new();
        calls(selected, &mut names);
        assert_eq!(names, ["subject", "low", "high"]);
    }
    #[test]
    fn implicit_thread_local_get_assertions_preserve_the_expression_name() {
        let object = checked("object Scratch { private val buffer = ThreadLocal.withInitial { CharArray(1) }; @JvmStatic fun string(): String = String(buffer.get(), 0, 0) }").unwrap().object_decl.unwrap();
        let Statement::Return(value) = &object.methods[0].function.body[0] else {
            panic!()
        };
        let ValueKind::StringFromChars(args) = &value.kind else {
            panic!()
        };
        let ValueKind::NotNull(_, message) = &args[0].kind else {
            panic!()
        };
        assert_eq!(message, "get(...)");
    }
}
