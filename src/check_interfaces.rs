use super::*;

pub(super) fn check(
    source: &syntax::InterfaceDecl,
    package: &str,
    classpath: &Classpath,
    mut imports: HashMap<String, String>,
    mut classes: HashMap<String, JavaClass>,
    owner_override: Option<&str>,
) -> Result<Module> {
    let owner = if let Some(owner) = owner_override {
        owner.into()
    } else if package.is_empty() {
        source.name.clone()
    } else {
        format!("{}/{}", package.replace('.', "/"), source.name)
    };
    fn remove_own(
        source: &syntax::InterfaceDecl,
        owner: &str,
        imports: &mut HashMap<String, String>,
        classes: &mut HashMap<String, JavaClass>,
        classpath: &Classpath,
    ) {
        if classpath.is_source(owner) {
            classes.remove(owner);
            imports.retain(|_, v| v != owner);
        }
        for nested in &source.nested {
            remove_own(
                nested,
                &format!("{owner}${}", nested.name),
                imports,
                classes,
                classpath,
            );
        }
    }
    remove_own(source, &owner, &mut imports, &mut classes, classpath);
    register_names(source, &owner, "", &mut imports)?;
    register_classes(source, &owner, &imports, &mut classes)?;
    let lambdas = Rc::new(RefCell::new(Vec::new()));
    let interface = check_interface(source, &owner, classpath, &imports, &classes, &lambdas)?;
    Ok(Module {
        exposed_fields: HashMap::new(),
        properties: Vec::new(),
        initializers: Vec::new(),
        locals: 0,
        lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
        constants: Vec::new(),
        functions: Vec::new(),
        class_decl: None,
        interface_decl: Some(interface),
        object_decl: None,
    })
}

fn register_names(
    source: &syntax::InterfaceDecl,
    owner: &str,
    prefix: &str,
    imports: &mut HashMap<String, String>,
) -> Result<()> {
    if matches!(
        source.name.as_str(),
        "Int"
            | "Long"
            | "List"
            | "Char"
            | "Float"
            | "Boolean"
            | "String"
            | "IntArray"
            | "CharArray"
            | "StringArray"
            | "Array"
            | "Unit"
    ) {
        return Err(Diagnostic::new(
            source.offset,
            "Source interfaces shadowing builtin types are not supported",
        ));
    }
    let qualified = if prefix.is_empty() {
        source.name.clone()
    } else {
        format!("{prefix}.{}", source.name)
    };
    if source.companion.is_some()
        && source
            .nested
            .iter()
            .any(|nested| nested.name == "Companion")
    {
        return Err(Diagnostic::new(
            source.offset,
            "Companion conflicts with nested interface declaration",
        ));
    }
    if source
        .nested
        .iter()
        .any(|nested| nested.name == "DefaultImpls")
    {
        return Err(Diagnostic::new(
            source.offset,
            "Nested DefaultImpls declarations are not supported",
        ));
    }
    if imports.insert(qualified.clone(), owner.into()).is_some() {
        return Err(Diagnostic::new(
            source.offset,
            "Conflicting source interface name",
        ));
    }
    for nested in &source.nested {
        register_names(
            nested,
            &format!("{owner}${}", nested.name),
            &qualified,
            imports,
        )?;
    }
    Ok(())
}

fn body_imports(
    source: &syntax::InterfaceDecl,
    owner: &str,
    imports: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut imports = imports.clone();
    for nested in &source.nested {
        imports.insert(nested.name.clone(), format!("{owner}${}", nested.name));
    }
    imports
}

fn register_classes(
    source: &syntax::InterfaceDecl,
    owner: &str,
    imports: &HashMap<String, String>,
    classes: &mut HashMap<String, JavaClass>,
) -> Result<()> {
    let mut interfaces = Vec::new();
    for alias in &source.interfaces {
        let name = imports.get(alias).ok_or_else(|| {
            Diagnostic::new(source.offset, "Unresolved source interface supertype")
        })?;
        if name == owner || interfaces.contains(name) {
            return Err(Diagnostic::new(
                source.offset,
                "Duplicate or cyclic source interface supertype",
            ));
        }
        interfaces.push(name.clone());
    }
    let imports = body_imports(source, owner, imports);
    let mut methods = Vec::new();
    let mut names = HashSet::new();
    for property in &source.properties {
        if loses_member_type(&property.ty) {
            return Err(Diagnostic::new(
                property.offset,
                "Nullable and List interface member signatures are not supported",
            ));
        }
        let ty = resolve_type(&property.ty, &imports, property.offset)?;
        if ty == Type::Unit {
            return Err(Diagnostic::new(
                property.offset,
                "Unit properties are not supported",
            ));
        }
        add_method(
            &mut methods,
            &mut names,
            JavaMethod {
                library: None,
                kotlin_params: Some(Vec::new()),
                kotlin_result: Some(ty.clone()),
                kotlin_internal: false,
                owner: owner.into(),
                name: getter_name(&property.name),
                descriptor: format!("(){}", ty.descriptor()),
                access: if property.body.is_some() { 1 } else { 0x0401 },
                unsupported: false,
                source_property: Some(property.name.clone()),
            },
            property.offset,
        )?;
        if property.mutable {
            add_method(
                &mut methods,
                &mut names,
                JavaMethod {
                    library: None,
                    kotlin_params: Some(vec![("value".into(), ty.clone(), false)]),
                    kotlin_result: Some(Type::Unit),
                    kotlin_internal: false,
                    owner: owner.into(),
                    name: setter_name(&property.name),
                    descriptor: format!("({})V", ty.descriptor()),
                    access: 0x0401,
                    unsupported: false,
                    source_property: Some(property.name.clone()),
                },
                property.offset,
            )?;
        }
    }
    for method in &source.methods {
        let function = &method.function;
        if function.defaults.iter().any(Option::is_some) {
            return Err(Diagnostic::new(
                function.offset,
                "Default arguments on source interface methods are unsupported",
            ));
        }
        if loses_member_type(&function.result)
            || function.params.iter().any(|(_, ty)| loses_member_type(ty))
        {
            return Err(Diagnostic::new(
                function.offset,
                "Nullable and List interface member signatures are not supported",
            ));
        }
        if function.private || function.internal || function.receiver.is_some() {
            return Err(Diagnostic::new(
                function.offset,
                "Source interface members require public non-extension declarations",
            ));
        }
        if function
            .params
            .iter()
            .map(|(_, ty)| ty.slots())
            .sum::<usize>()
            > 254
        {
            return Err(Diagnostic::new(
                function.offset,
                "Instance methods support at most 254 parameter slots",
            ));
        }
        let mut parameters = String::new();
        for (_, ty) in &function.params {
            let ty = resolve_type(ty, &imports, function.offset)?;
            if ty == Type::Unit {
                return Err(Diagnostic::new(
                    function.offset,
                    "Unit parameters are not supported",
                ));
            }
            parameters.push_str(&ty.descriptor());
        }
        let result = resolve_type(&function.result, &imports, function.offset)?;
        add_method(
            &mut methods,
            &mut names,
            JavaMethod {
                library: (function.infix || function.operator).then(|| {
                    crate::classpath::LibraryMethod {
                        source_name: function.name.clone(),
                        operator: function.operator,
                        kotlin_public: !function.private
                            && !function.protected
                            && !function.internal,
                        type_parameters: Vec::new(),
                        receiver: None,
                        infix: function.infix,
                        inline: false,
                        code: None,
                    }
                }),
                kotlin_params: Some(
                    function
                        .params
                        .iter()
                        .map(|(n, t)| {
                            Ok((
                                n.clone(),
                                resolve_type(t, &imports, function.offset)?,
                                false,
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?,
                ),
                kotlin_result: Some(result.clone()),
                kotlin_internal: false,
                owner: owner.into(),
                name: function.name.clone(),
                descriptor: format!("({parameters}){}", result.descriptor()),
                access: if method.has_body { 1 } else { 0x0401 },
                unsupported: false,
                source_property: None,
            },
            function.offset,
        )?;
    }
    if classes
        .insert(
            owner.into(),
            JavaClass {
                library: None,
                fields: Vec::new(),
                kotlin_class: false,
                name: owner.into(),
                access: 0x0601,
                super_name: Some("java/lang/Object".into()),
                interfaces,
                methods,
                unsupported: false,
                kotlin_interface: true,
            },
        )
        .is_some()
    {
        return Err(Diagnostic::new(
            source.offset,
            "Source interface conflicts with imported class identity",
        ));
    }
    for nested in &source.nested {
        register_classes(
            nested,
            &format!("{owner}${}", nested.name),
            &imports,
            classes,
        )?;
    }
    Ok(())
}

fn add_method(
    methods: &mut Vec<JavaMethod>,
    names: &mut HashSet<String>,
    method: JavaMethod,
    offset: usize,
) -> Result<()> {
    if inherited_object_method(&method.name) {
        return Err(Diagnostic::new(
            offset,
            "Any and Object inherited members are not supported",
        ));
    }
    if !names.insert(method.name.clone()) {
        return Err(Diagnostic::new(
            offset,
            "Member overloads and JVM getter collisions are not supported",
        ));
    }
    class_method_types(&method)?;
    methods.push(method);
    Ok(())
}

pub(super) fn interface_methods(
    class: &JavaClass,
    classes: &HashMap<String, JavaClass>,
    offset: usize,
) -> Result<HashMap<String, JavaMethod>> {
    fn collect(
        class: &JavaClass,
        classes: &HashMap<String, JavaClass>,
        visiting: &mut HashSet<String>,
        offset: usize,
    ) -> Result<HashMap<String, JavaMethod>> {
        if class.access & 0x0201 != 0x0201
            || class.unsupported
            || !visiting.insert(class.name.clone())
            || visiting.len() > 128
        {
            return Err(Diagnostic::new(
                offset,
                "Unsupported or cyclic interface inheritance or metadata",
            ));
        }
        let mut methods: HashMap<String, JavaMethod> = HashMap::new();
        for name in &class.interfaces {
            let parent = classes.get(name).ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    "Inherited interfaces require explicit imports or source declarations",
                )
            })?;
            for (name, method) in collect(parent, classes, visiting, offset)? {
                if let Some(previous) = methods.get(&name) {
                    if previous.owner != method.owner
                        && (previous.descriptor != method.descriptor
                            || previous.source_property != method.source_property
                            || previous.access & 0x0400 == 0
                            || method.access & 0x0400 == 0)
                    {
                        return Err(Diagnostic::new(
                            offset,
                            "Conflicting inherited interface methods are not supported",
                        ));
                    }
                } else {
                    methods.insert(name, method);
                }
            }
        }
        for method in &class.methods {
            if method.access & 8 != 0 || method.name == "<clinit>" {
                continue;
            }
            if method.access & 1 == 0 || method.unsupported || method.name.starts_with('<') {
                return Err(Diagnostic::new(offset, "Unsupported interface member"));
            }
            class_method_types(method)?;
            if let Some(previous) = methods.get(&method.name) {
                if previous.descriptor != method.descriptor
                    || previous.source_property != method.source_property
                {
                    return Err(Diagnostic::new(
                        offset,
                        "Conflicting inherited interface signature",
                    ));
                }
            }
            methods.insert(method.name.clone(), method.clone());
        }
        visiting.remove(&class.name);
        Ok(methods)
    }
    collect(class, classes, &mut HashSet::new(), offset)
}

fn check_interface(
    source: &syntax::InterfaceDecl,
    owner: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    lambdas: &Rc<RefCell<Vec<LambdaDecl>>>,
) -> Result<InterfaceDecl> {
    let imports = body_imports(source, owner, imports);
    let class = &classes[owner];
    let members = interface_methods(class, classes, source.offset)?;
    let mut inherited = HashMap::new();
    for parent in &class.interfaces {
        inherited.extend(interface_methods(&classes[parent], classes, source.offset)?);
    }
    let mut companion_fields = HashMap::new();
    let mut companion_names = HashSet::new();
    if let Some(companion) = &source.companion {
        for property in &companion.properties {
            if property.name == "IllegalArgumentException" {
                return Err(Diagnostic::new(
                    property.offset,
                    "Companion properties shadowing supported exception constructors are not supported",
                ));
            }
            if !companion_names.insert(property.name.clone()) {
                return Err(Diagnostic::new(
                    property.offset,
                    "Conflicting companion property declaration",
                ));
            }
            if let syntax::CompanionValue::StringArray(_) = &property.value {
                if !property.private {
                    return Err(Diagnostic::new(
                        property.offset,
                        "Companion String arrays currently require private visibility",
                    ));
                }
                companion_fields.insert(
                    property.name.clone(),
                    (format!("{owner}$Companion"), Type::StringArray),
                );
            }
        }
    }
    let mut methods = Vec::new();
    for property in &source.properties {
        let name = getter_name(&property.name);
        let setter = setter_name(&property.name);
        if !property.mutable
            && inherited
                .get(&setter)
                .is_some_and(|method| method.source_property.as_deref() == Some(&property.name))
        {
            return Err(Diagnostic::new(
                property.offset,
                "A mutable interface property cannot be overridden by val",
            ));
        }
        let method = &members[&name];
        check_override(
            method,
            property.is_override,
            inherited.get(&name),
            property.offset,
        )?;
        let function = check_function(
            &name,
            &[],
            &property.ty,
            property.body.as_deref(),
            property.is_override,
            property.offset,
            owner,
            classpath,
            &imports,
            classes,
            &members,
            &companion_fields,
            lambdas,
        )?;
        methods.push(InterfaceMethod {
            function,
            has_body: property.body.is_some(),
            source_property: Some(property.name.clone()),
        });
        if property.mutable {
            let method = &members[&setter];
            let is_override = property.is_override && inherited.contains_key(&setter);
            check_override(method, is_override, inherited.get(&setter), property.offset)?;
            methods.push(InterfaceMethod {
                function: check_function(
                    &setter,
                    &[("value".into(), property.ty.clone())],
                    &Type::Unit,
                    None,
                    is_override,
                    property.offset,
                    owner,
                    classpath,
                    &imports,
                    classes,
                    &members,
                    &companion_fields,
                    lambdas,
                )?,
                has_body: false,
                source_property: Some(property.name.clone()),
            });
        }
    }
    for method in &source.methods {
        let source_function = &method.function;
        declarations::validate_operator(source_function, true)?;
        let signature = &members[&source_function.name];
        check_override(
            signature,
            method.is_override,
            inherited.get(&source_function.name),
            source_function.offset,
        )?;
        let mut function = check_function(
            &source_function.name,
            &source_function.params,
            &source_function.result,
            method.has_body.then_some(source_function.body.as_slice()),
            method.is_override,
            source_function.offset,
            owner,
            classpath,
            &imports,
            classes,
            &members,
            &companion_fields,
            lambdas,
        )?;
        function.type_params = resolve_parameters(
            &source_function.type_params,
            &imports,
            classpath,
            source_function.offset,
            classes,
        )?;
        methods.push(InterfaceMethod {
            function,
            has_body: method.has_body,
            source_property: None,
        });
    }
    let companion = if let Some(companion) = &source.companion {
        let mut properties = Vec::new();
        for property in &companion.properties {
            let (ty, value) = match &property.value {
                syntax::CompanionValue::StringArray(values) => (
                    Type::StringArray,
                    CompanionValue::StringArray(values.clone()),
                ),
                syntax::CompanionValue::Object(object) => {
                    if property.private || object.interfaces.len() != 1 || !object.params.is_empty()
                    {
                        return Err(Diagnostic::new(
                            property.offset,
                            "Anonymous companion objects require a public property and exactly one supported interface",
                        ));
                    }
                    let interface = imports.get(&object.interfaces[0]).ok_or_else(|| {
                        Diagnostic::new(property.offset, "Unresolved anonymous object interface")
                    })?;
                    let object_owner = format!("{owner}$Companion${}$1", property.name);
                    let class_decl = check_class(
                        object,
                        "",
                        classpath,
                        &imports,
                        classes,
                        &companion_fields,
                        Some(&object_owner),
                        lambdas,
                    )?;
                    (
                        Type::Java(interface.clone()),
                        CompanionValue::Object {
                            owner: object_owner,
                            class_decl: Box::new(class_decl),
                        },
                    )
                }
            };
            properties.push(CompanionProperty {
                name: property.name.clone(),
                private: property.private,
                ty,
                value,
            });
        }
        Some(CompanionDecl {
            state: if companion.fields.is_empty()
                && companion.methods.is_empty()
                && companion.initializers.is_empty()
            {
                None
            } else {
                Some(declarations::check_static_state(
                    &format!("{owner}$Companion"),
                    &format!("{owner}$Companion"),
                    &companion.fields,
                    &companion.methods,
                    &companion.initializers,
                    true,
                    classpath,
                    &imports,
                    classes,
                    lambdas,
                )?)
            },
            owner: format!("{owner}$Companion"),
            properties,
        })
    } else {
        None
    };
    let mut nested = Vec::new();
    for declaration in &source.nested {
        nested.push(check_interface(
            declaration,
            &format!("{owner}${}", declaration.name),
            classpath,
            &imports,
            classes,
            lambdas,
        )?);
    }
    let mut defaults: Vec<_> = members
        .values()
        .filter(|method| method.owner != owner && method.access & 0x0400 == 0)
        .cloned()
        .collect();
    defaults.sort_by(|a, b| a.name.cmp(&b.name));
    for method in &mut defaults {
        *method = declarations::direct_default_bridge(
            method.clone(),
            &class.interfaces,
            classpath,
            source.offset,
        )?;
    }
    Ok(InterfaceDecl {
        owner: owner.into(),
        interfaces: class.interfaces.clone(),
        methods,
        nested,
        companion,
        defaults,
    })
}

fn check_override(
    method: &JavaMethod,
    is_override: bool,
    inherited: Option<&JavaMethod>,
    offset: usize,
) -> Result<()> {
    match inherited {
        Some(parent)
            if is_override
                && method.descriptor == parent.descriptor
                && method.source_property == parent.source_property =>
        {
            Ok(())
        }
        None if !is_override => Ok(()),
        _ => Err(Diagnostic::new(
            offset,
            "Interface override requires an inherited declaration with an exact supported signature and property kind",
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn check_function(
    name: &str,
    source_params: &[(String, Type)],
    result: &Type,
    body: Option<&[syntax::Statement]>,
    is_override: bool,
    offset: usize,
    owner: &str,
    classpath: &Classpath,
    imports: &HashMap<String, String>,
    classes: &HashMap<String, JavaClass>,
    members: &HashMap<String, JavaMethod>,
    companion_fields: &HashMap<String, (String, Type)>,
    lambdas: &Rc<RefCell<Vec<LambdaDecl>>>,
) -> Result<Function> {
    let result = resolve_type_checked(result, imports, offset, classpath, classes)?;
    let fields = HashMap::new();
    let functions = HashMap::new();
    let signatures = Vec::new();
    let extensions = HashSet::new();
    let mut checker = Checker {
        lambdas: lambdas.clone(),
        captured: HashMap::new(),
        captured_cells: HashMap::new(),
        cells: HashMap::new(),
        cell_names: HashSet::new(),
        lambda_owner: owner.into(),
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
        receiver: Some(Local {
            slot: 0,
            ty: Type::Java(owner.into()),
            mutable: false,
        }),
        fields: &fields,
        members,
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
        result: result.clone(),
    };
    let mut params = Vec::new();
    for (param, ty) in source_params {
        let ty = resolve_type_checked(ty, imports, offset, classpath, classes)?;
        if ty == Type::Unit {
            return Err(Diagnostic::new(offset, "Unit parameters are unsupported"));
        }
        checker.declare(param, ty.clone(), false, offset)?;
        params.push((param.clone(), ty));
    }
    let body = if let Some(body) = body {
        let (mut body, returns) = checker.block(body)?;
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
        body
    } else {
        Vec::new()
    };
    Ok(Function {
        type_params: Vec::new(),
        defaults: Vec::new(),
        name: name.into(),
        private: false,
        is_override,
        params,
        result,
        locals: checker.next,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const CATALOG: &str = r#"
package sample
interface Catalog {
    val size: Int
    fun item(index: Int): String
    fun label(index: Int): String {
        val number = index + 1
        return if (number < 3) { LABELS[number] } else { number.toString() }
    }
    interface Zoom : Catalog { fun scale(index: Int): Float }
    companion object {
        private val LABELS = arrayOf("zero", "one", "two")
        val EMPTY = object : Catalog {
            override val size: Int get() = 0
            override fun item(index: Int): String {
                throw IllegalArgumentException("missing $index size=$size")
            }
        }
    }
}
"#;
    fn checked(source: &str) -> Result<Module> {
        let source = syntax::parse(source)?;
        super::super::check(&source, &Classpath::new(&[])?)
    }
    fn rejects(source: &str, message: &str) {
        let error = checked(source)
            .err()
            .unwrap_or_else(|| panic!("expected interface rejection: {source}"));
        assert!(error.message.contains(message), "{}", error.message);
    }
    #[test]
    fn mutable_properties_emit_both_accessors_and_reads_select_the_getter() {
        let interface = checked("interface Surface { var count: Int; var isReady: Boolean; var isTitle: String; fun current(): Int = count; fun own(): Boolean = this.isReady }").unwrap().interface_decl.unwrap();
        let signatures = interface
            .methods
            .iter()
            .map(|method| (method.function.name.as_str(), method.function.descriptor()))
            .collect::<Vec<_>>();
        assert_eq!(
            signatures,
            [
                ("getCount", "()I".into()),
                ("setCount", "(I)V".into()),
                ("isReady", "()Z".into()),
                ("setReady", "(Z)V".into()),
                ("isTitle", "()Ljava/lang/String;".into()),
                ("setTitle", "(Ljava/lang/String;)V".into()),
                ("current", "()I".into()),
                ("own", "()Z".into()),
            ]
        );
        let inherited = checked("interface Root { val count: Int; interface Child : Root { override var count: Int; interface Grandchild : Child { override var count: Int } } }").unwrap().interface_decl.unwrap();
        assert_eq!(inherited.nested[0].methods.len(), 2);
        assert_eq!(inherited.nested[0].nested[0].methods.len(), 2);
        for (source, diagnostic) in [
            (
                "interface Root { var count: Int; interface Child : Root { override val count: Int } }",
                "cannot be overridden by val",
            ),
            (
                "interface Root { var count: Int; interface Child : Root { override var count: Byte } }",
                "Conflicting inherited interface signature",
            ),
            (
                "interface Root { var count: Int; fun setCount(value: Int) }",
                "collisions",
            ),
            (
                "interface Root { var isReady: Boolean; var ready: Boolean }",
                "collisions",
            ),
            (
                "interface Root { var count: Int; fun write() { count = 1 } }",
                "Property assignments",
            ),
            (
                "interface Root { var count: Int; fun write() { count += 1 } }",
                "Property assignments",
            ),
        ] {
            rejects(source, diagnostic);
        }
    }
    #[test]
    fn source_interfaces_share_expression_checks_and_preserve_nested_inherited_defaults() {
        let module = checked(CATALOG).unwrap();
        let interface = module.interface_decl.unwrap();
        assert_eq!(interface.owner, "sample/Catalog");
        assert_eq!(interface.methods.len(), 3);
        assert!(!interface.methods[0].has_body);
        assert_eq!(
            interface.methods[0].source_property.as_deref(),
            Some("size")
        );
        assert!(interface.methods[2].has_body);
        assert_eq!(interface.nested[0].interfaces, ["sample/Catalog"]);
        assert_eq!(interface.nested[0].methods[0].function.result, Type::Float);
        assert_eq!(interface.nested[0].defaults[0].name, "label");
        let companion = interface.companion.unwrap();
        assert!(companion.properties[0].private);
        assert_eq!(
            companion.properties[1].ty,
            Type::Java("sample/Catalog".into())
        );
        let CompanionValue::Object { owner, class_decl } = &companion.properties[1].value else {
            panic!()
        };
        assert_eq!(owner, "sample/Catalog$Companion$EMPTY$1");
        assert_eq!(class_decl.bridges[0].name, "label");
        assert!(matches!(
            class_decl.methods[1].body[0],
            Statement::ThrowValue(_)
        ));
    }
    #[test]
    fn source_interface_implementations_require_exact_property_and_function_identity() {
        for (before, after, message) in [
            (
                "override val size: Int get() = 0",
                "override fun getSize(): Int = 0",
                "exact supported JVM signature",
            ),
            (
                "override fun item(index: Int): String",
                "fun item(index: Int): String",
                "requires override",
            ),
            (
                "override val size: Int get() = 0",
                "val wrong: Int get() = 0",
                "Missing interface implementation",
            ),
            (
                "override val size: Int get() = 0",
                "override val size: Boolean get() = false",
                "exact supported JVM signature",
            ),
        ] {
            rejects(&CATALOG.replace(before, after), message);
        }
    }
    #[test]
    fn source_interface_scope_and_cycles_are_checked() {
        rejects(
            "interface Root : Child { interface Child }",
            "Unresolved source interface supertype",
        );
        rejects(
            "interface Root { interface A { interface Hidden }; interface B : Hidden }",
            "Unresolved source interface supertype",
        );
        rejects(
            "interface Root { interface A : B; interface B : A }",
            "cyclic",
        );
        assert!(checked("interface Root { interface Child : Root }").is_ok());
    }
    #[test]
    fn source_interface_overrides_and_conflicts_are_rejected() {
        rejects(
            "interface Root { fun f(): Int; interface Child : Root { fun f(): Int } }",
            "Interface override requires",
        );
        rejects(
            "interface Root { fun f(): Int; interface Child : Root { override val f: Int } }",
            "Interface override requires",
        );
        rejects(
            "interface Root { fun getX(): Int; val x: Int }",
            "getter collisions",
        );
        rejects(
            "interface Root { val x: Int; interface Child : Root { override fun getX(): Int } }",
            "Conflicting inherited interface signature",
        );
    }
    #[test]
    fn unsupported_interface_bodies_and_companion_shapes_fail_closed() {
        assert!(checked("interface Root { fun f(value: Float): Float = value }").is_ok());
        rejects("interface Root { val f: Float get() = 1 }", "Float");
        assert!(checked("interface Root { fun f(): String? }").is_ok());
        rejects(
            r#"interface Root { companion object { val VALUES = arrayOf("a") } }"#,
            "private",
        );
        rejects(
            &CATALOG.replace("val EMPTY =", "private val EMPTY ="),
            "public",
        );
        rejects(
            &CATALOG.replace("number < 3", "number + 3"),
            "Type mismatch",
        );
        rejects(&CATALOG.replace("LABELS[number]", "true"), "Type mismatch");
    }
    #[test]
    fn source_interface_member_access_uses_getters_and_argument_types() {
        assert!(
            checked("interface Root { val value: Int; fun text(): String = value.toString() }")
                .is_ok()
        );
        assert!(
            checked(
                "interface Root { fun item(index: Int): String; fun text(): String = item(1) }"
            )
            .is_ok()
        );
        rejects(
            "interface Root { fun item(index: Int): String; fun text(): String = item(true) }",
            "Type mismatch",
        );
        rejects(
            "interface Root { val value: Int; fun text(): String = getValue().toString() }",
            "properties cannot be invoked",
        );
        rejects(
            r#"interface Root { val IllegalArgumentException: Int; fun fail(): String { throw IllegalArgumentException("bad") } }"#,
            "No applicable",
        );
    }
    #[test]
    fn if_expressions_reject_mixed_platform_branches_in_either_order() {
        let classes = HashMap::from([(
            "Foreign".into(),
            JavaClass {
                library: None,
                fields: Vec::new(),
                kotlin_class: false,
                name: "Foreign".into(),
                access: 1,
                super_name: Some("java/lang/Object".into()),
                interfaces: vec![],
                methods: vec![JavaMethod {
                    library: None,
                    kotlin_params: None,
                    kotlin_result: None,
                    kotlin_internal: false,
                    owner: "Foreign".into(),
                    name: "text".into(),
                    descriptor: "()Ljava/lang/String;".into(),
                    access: 1,
                    unsupported: false,
                    source_property: None,
                }],
                unsupported: false,
                kotlin_interface: false,
            },
        )]);
        let classpath = Classpath::new(&[]).unwrap();
        let empty_functions = HashMap::new();
        let empty_members = HashMap::new();
        let empty_fields = HashMap::new();
        let imports = HashMap::new();
        let signatures = Vec::new();
        let extensions = HashSet::new();
        let companion_fields = HashMap::new();
        for (yes, no) in [("\"ok\"", "foreign.text()"), ("foreign.text()", "\"ok\"")] {
            let source = syntax::parse(&format!(
                "fun f(): String = if (true) {{ {yes} }} else {{ {no} }}"
            ))
            .unwrap();
            let StatementKind::Return(Some(expr)) = &source.functions[0].body[0].kind else {
                panic!()
            };
            let mut checker = Checker {
                lambdas: Rc::new(RefCell::new(Vec::new())),
                captured: HashMap::new(),
                captured_cells: HashMap::new(),
                cells: HashMap::new(),
                cell_names: HashSet::new(),
                lambda_owner: "Korrust".into(),
                inline_result: None,
                nonlocal_result: None,
                constants: &HashMap::new(),
                constructor: None,
                defaults: &Vec::new(),
                function_names: &[],
                mutable_fields: &HashSet::new(),
                classpath: &classpath,
                extensions: &extensions,
                classes: &classes,
                imports: &imports,
                signatures: &signatures,
                source_type_params: &[],
                source_infix: &HashSet::new(),
                source_operator: &HashSet::new(),
                receiver: None,
                fields: &empty_fields,
                members: &empty_members,
                companion_fields: &companion_fields,
                functions: &empty_functions,
                object_methods: &HashMap::new(),
                object_fields: &HashMap::new(),
                object_owner: None,
                allow_return: true,
                refinements: HashMap::new(),
                initialized: HashSet::new(),
                loop_depth: 0,
                scopes: vec![HashMap::new()],
                next: 0,
                result: Type::String,
            };
            checker
                .declare("foreign", Type::Java("Foreign".into()), false, 0)
                .unwrap();
            let error = checker
                .value(expr)
                .err()
                .expect("mixed platform branches must reject");
            assert!(error.message.contains("common type inference"));
        }
    }
    #[test]
    fn builtin_resolution_rejects_member_and_classifier_shadowing() {
        rejects("interface Int", "shadowing builtin");
        rejects(
            "interface Root { val Int: Int; fun f(): Int = Int.MAX_VALUE }",
            "Unsupported property",
        );
        rejects(
            r#"interface Root { fun IllegalArgumentException(message: String): String; fun f(): String { throw IllegalArgumentException("bad") } }"#,
            "Throwable",
        );
        rejects(
            r#"class IllegalArgumentException() { fun f(): String { throw IllegalArgumentException() } }"#,
            "Throwable",
        );
    }
    #[test]
    fn source_default_reference_parameters_share_regular_body_checking() {
        for parameter in ["String", "IntArray", "CharArray", "Array<String>"] {
            assert!(
                checked(&format!(
                    "interface Root {{ fun size(value: {parameter}): Int = 0 }}"
                ))
                .is_ok()
            );
            assert!(
                checked(&format!(
                    "interface Root {{ fun size(value: {parameter}): Int }}"
                ))
                .is_ok()
            );
        }
        assert!(checked(r#"interface Root { fun label(index: Int): String = "text" }"#).is_ok());
    }
    #[test]
    fn inherited_reference_defaults_share_the_supported_signatures() {
        for (descriptor, access, accepted) in [
            ("(Ljava/lang/String;)I", 1, true),
            ("([I)I", 1, true),
            ("(I)I", 1, true),
            ("(Ljava/lang/String;)I", 0x0401, true),
        ] {
            let imports = HashMap::from([("Parent".into(), "foreign/Parent".into())]);
            let classes = HashMap::from([(
                "foreign/Parent".into(),
                JavaClass {
                    library: None,
                    fields: Vec::new(),
                    kotlin_class: false,
                    name: "foreign/Parent".into(),
                    access: 0x0601,
                    super_name: Some("java/lang/Object".into()),
                    interfaces: vec![],
                    methods: vec![JavaMethod {
                        library: None,
                        kotlin_params: None,
                        kotlin_result: None,
                        kotlin_internal: false,
                        owner: "foreign/Parent".into(),
                        name: "size".into(),
                        descriptor: descriptor.into(),
                        access,
                        unsupported: false,
                        source_property: None,
                    }],
                    unsupported: false,
                    kotlin_interface: true,
                },
            )]);
            let source = syntax::parse("interface Child : Parent").unwrap();
            let classpath = Classpath::new(&[]).unwrap();
            let result = super::check(
                source.interface_decl.as_ref().unwrap(),
                "",
                &classpath,
                imports.clone(),
                classes.clone(),
                None,
            );
            assert_eq!(result.is_ok(), accepted);
            if !accepted {
                assert!(
                    result
                        .err()
                        .unwrap()
                        .message
                        .contains("Inherited interface defaults with reference parameters")
                );
                let source = syntax::parse("class Child() : Parent {}").unwrap();
                let class = check_class(
                    source.class_decl.as_ref().unwrap(),
                    "",
                    &classpath,
                    &imports,
                    &classes,
                    &HashMap::new(),
                    None,
                    &Rc::new(RefCell::new(Vec::new())),
                )
                .unwrap();
                assert_eq!(class.bridges.len(), 1);
            }
        }
    }
}
