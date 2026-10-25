use super::*;

pub(super) fn check(
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
            || function.internal
            || method.jvm_static && function.private
            || method.inline && !function.private
        {
            return Err(Diagnostic::new(
                function.offset,
                "Object member extensions, internal members, and private JvmStatic methods are not supported",
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
                source_name: function.name.clone(),
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
        let ExprKind::Call(target, arguments) = &field.initializer.kind else {
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
        lambdas: Rc::try_unwrap(lambdas).ok().unwrap().into_inner(),
        constants: Vec::new(),
        functions: Vec::new(),
        class_decl: None,
        interface_decl: None,
        object_decl: Some(DynamicObject {
            owner,
            interfaces: Vec::new(),
            serializable: false,
            fields,
            methods,
        }),
    })
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
        let options = self.object_methods[name].clone();
        let mut values = Vec::new();
        for arg in args {
            values.push(self.value(arg)?);
        }
        let mut matching = Vec::new();
        for option in options {
            let (params, result) = self.method_types(&option, offset)?;
            if params.len() == values.len()
                && params.iter().zip(&values).all(|(param, value)| {
                    param == &value.ty
                        || matches!(value.ty, Type::Platform(_)) && value.ty.base() == param
                })
            {
                matching.push((option, params, result));
            }
        }
        if matching.len() != 1 {
            return Err(Diagnostic::new(
                offset,
                "Object overload requires one exact supported argument signature",
            ));
        }
        let (method, params, result) = matching.pop().unwrap();
        let mut args = Vec::new();
        for (index, (value, param)) in values.into_iter().zip(params).enumerate() {
            args.push(self.coerce(value, param, offset + index)?);
        }
        let kind = if method.access & 8 != 0 {
            let call = Value {
                ty: result.clone(),
                kind: ValueKind::StaticCall {
                    owner: method.owner,
                    name: method.name,
                    descriptor: method.descriptor,
                    args,
                },
            };
            if let Some(receiver) = receiver {
                let slot = self.allocate_slot(offset)?;
                ValueKind::Let {
                    slot,
                    value: Box::new(receiver),
                    body: Box::new(call),
                }
            } else {
                call.kind
            }
        } else {
            let receiver = receiver.unwrap_or_else(|| {
                let kind = self
                    .receiver
                    .as_ref()
                    .filter(|receiver| receiver.ty == Type::Java(method.owner.clone()))
                    .map(|receiver| ValueKind::Local(receiver.slot))
                    .unwrap_or_else(|| ValueKind::ObjectInstance(method.owner.clone()));
                Value {
                    ty: Type::Java(method.owner.clone()),
                    kind,
                }
            });
            args.insert(0, receiver);
            ValueKind::MemberCall {
                method,
                interface: false,
                args,
            }
        };
        Ok(Value { ty: result, kind })
    }
    pub(super) fn method_types(
        &self,
        method: &JavaMethod,
        offset: usize,
    ) -> Result<(Vec<Type>, Type)> {
        let mut cursor = 1;
        if !method.descriptor.starts_with('(') {
            return Err(Diagnostic::new(offset, "Invalid Java method descriptor"));
        }
        let mut params = Vec::new();
        while method.descriptor.as_bytes().get(cursor) != Some(&b')') {
            params.push(self.java_type(&method.descriptor, &mut cursor, offset)?);
        }
        cursor += 1;
        let result = self.java_type(&method.descriptor, &mut cursor, offset)?;
        if cursor != method.descriptor.len() || params.contains(&Type::Unit) {
            return Err(Diagnostic::new(offset, "Invalid Java method descriptor"));
        }
        Ok((params, result))
    }
    pub(super) fn throw_exception(&mut self, expr: &Expr, offset: usize) -> Result<Statement> {
        let ExprKind::Call(target, args) = &expr.kind else {
            return Err(Diagnostic::new(
                offset,
                "Only throwing supported standard exception constructors is supported",
            ));
        };
        let ExprKind::Name(name) = &target.kind else {
            return Err(Diagnostic::new(
                offset,
                "Only throwing supported standard exception constructors is supported",
            ));
        };
        let imported = self.imports.get(name);
        let owner =
            exception_owner(imported.map(String::as_str).unwrap_or(name)).ok_or_else(|| {
                Diagnostic::new(
                    offset,
                    "Only throwing supported standard exception constructors is supported",
                )
            })?;
        let mut shadowed = !self.unshadowed(name);
        if imported.is_some_and(|value| value == owner) {
            shadowed = self.local(name).is_some()
                || self.functions.contains_key(name)
                || self.object_methods.contains_key(name)
                || self.object_fields.contains_key(name)
                || self.fields.contains_key(name)
                || self.members.contains_key(name)
                || self
                    .members
                    .values()
                    .any(|method| method.source_property.as_deref() == Some(name))
                || self.companion_fields.contains_key(name);
        }
        if shadowed || args.len() > 1 {
            return Err(Diagnostic::new(
                offset,
                "Only throwing unshadowed supported exception constructors is supported",
            ));
        }
        let message = if let Some(arg) = args.first() {
            let value = self.value(arg)?;
            if value.ty == Type::Null && owner != "java/lang/IndexOutOfBoundsException" {
                return Err(Diagnostic::new(
                    arg.offset,
                    "Ambiguous null exception constructor argument",
                ));
            }
            Some(self.coerce(value, Type::Nullable(Box::new(Type::String)), arg.offset)?)
        } else {
            None
        };
        if owner == "java/lang/IllegalArgumentException" {
            if let Some(message) = message {
                return Ok(Statement::Throw(message));
            }
        }
        Ok(Statement::ThrowException {
            owner: owner.into(),
            message,
        })
    }
    pub(super) fn range_contains(
        &mut self,
        value: Value,
        range: &Expr,
        offset: usize,
    ) -> Result<Value> {
        let ExprKind::Binary(operator, start, end) = &range.kind else {
            return Err(Diagnostic::new(
                offset,
                "Membership requires a bounded inclusive Int or Char range",
            ));
        };
        if operator != ".." || !matches!(value.ty, Type::Int | Type::Char) {
            return Err(Diagnostic::new(
                offset,
                "Membership requires a bounded inclusive Int or Char range",
            ));
        }
        let start = self.value(start)?;
        let end = self.value(end)?;
        self.require(&start, value.ty.clone(), offset)?;
        self.require(&end, value.ty.clone(), offset)?;
        let value_slot = self.allocate_slot(offset)?;
        let start_slot = self.allocate_slot(offset)?;
        let end_slot = self.allocate_slot(offset)?;
        let local = |slot| Value {
            ty: value.ty.clone(),
            kind: ValueKind::Local(slot),
        };
        let yes = Value {
            ty: Type::Boolean,
            kind: ValueKind::Binary(
                ">=".into(),
                Box::new(local(value_slot)),
                Box::new(local(start_slot)),
            ),
        };
        let no = Value {
            ty: Type::Boolean,
            kind: ValueKind::Binary(
                "<=".into(),
                Box::new(local(value_slot)),
                Box::new(local(end_slot)),
            ),
        };
        let body = Value {
            ty: Type::Boolean,
            kind: ValueKind::Binary("&&".into(), Box::new(yes), Box::new(no)),
        };
        let body = Value {
            ty: Type::Boolean,
            kind: ValueKind::Let {
                slot: value_slot,
                value: Box::new(value),
                body: Box::new(body),
            },
        };
        let body = Value {
            ty: Type::Boolean,
            kind: ValueKind::Let {
                slot: end_slot,
                value: Box::new(end),
                body: Box::new(body),
            },
        };
        Ok(Value {
            ty: Type::Boolean,
            kind: ValueKind::Let {
                slot: start_slot,
                value: Box::new(start),
                body: Box::new(body),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn checked(source: &str) -> Result<Module> {
        let source = syntax::parse(source)?;
        super::super::check(&source, &Classpath::new(&[])?)
    }
    fn rejects(source: &str, message: &str) {
        let error = checked(source).err().expect("expected rejection");
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
            "exact supported argument signature",
        );
        rejects(
            &SCRATCH.replace("convert(input, 0, input.size)", "convert(input, 0)"),
            "exact supported argument signature",
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
        for source in [
            "object Scratch { private val buffer = ThreadLocal.withInitial { ByteArray(4) }; fun ByteArray(size: Int): CharArray = CharArray(size) }",
            "object Scratch { private val ByteArray = ThreadLocal.withInitial { ByteArray(4) } }",
        ] {
            rejects(source, "Shadowed array constructors");
        }
        rejects(
            "object Scratch { private val x = ThreadLocal.withInitial { missing } }",
            "single array allocation",
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
        rejects(
            "object Scratch { internal fun f(): Int = 1 }",
            "Internal object methods",
        );
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
        rejects(
            "fun f(): IntArray { var result: ThreadLocal<ByteArray>; return IntArray(1) }",
            "Explicit ThreadLocal",
        );
        rejects(
            "fun f(value: Boolean): Boolean = value in true..false",
            "bounded inclusive",
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
        assert!(matches!(call.kind, ValueKind::StaticCall { .. }));
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
        let ValueKind::MemberCall { args: instance, .. } = &instance.kind else {
            panic!()
        };
        let ValueKind::MemberCall {
            args: static_args, ..
        } = &static_call.kind
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
        let ValueKind::Let {
            value: low, body, ..
        } = &contains.kind
        else {
            panic!()
        };
        let ValueKind::Let {
            value: high, body, ..
        } = &body.kind
        else {
            panic!()
        };
        let ValueKind::Let { value: subject, .. } = &body.kind else {
            panic!()
        };
        for (value, expected) in [(low, "low"), (high, "high"), (subject, "subject")] {
            let ValueKind::MemberCall { method, .. } = &value.kind else {
                panic!()
            };
            assert_eq!(method.name, expected);
        }
        let Statement::Return(selected) = &object.methods[4].function.body[0] else {
            panic!()
        };
        let ValueKind::Let { value: subject, .. } = &selected.kind else {
            panic!()
        };
        let ValueKind::MemberCall { method, .. } = &subject.kind else {
            panic!()
        };
        assert_eq!(method.name, "subject");
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
