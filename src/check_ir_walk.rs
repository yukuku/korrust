use super::*;

pub(crate) trait Visitor {
    fn value(&mut self, node: &mut Value, owner: &str);

    fn statement(&mut self, _node: &mut Statement, _owner: &str) {}
}

impl<F: FnMut(&mut Value, &str)> Visitor for F {
    fn value(&mut self, node: &mut Value, owner: &str) {
        self(node, owner);
    }
}

pub(crate) fn value(node: &mut Value, owner: &str, visitor: &mut impl Visitor) {
    match &mut node.kind {
        ValueKind::SuperCall { args, .. }
        | ValueKind::CompiledInline { args, .. }
        | ValueKind::New { args, .. }
        | ValueKind::StaticCall { args, .. }
        | ValueKind::MemberCall { args, .. }
        | ValueKind::JavaCall { args, .. }
        | ValueKind::CopyInto(args)
        | ValueKind::StringCall(_, args)
        | ValueKind::Call(_, args)
        | ValueKind::DefaultCall(_, args, _)
        | ValueKind::ListFactory { elements: args, .. }
        | ValueKind::Lambda { captures: args, .. } => values(args, owner, visitor),
        ValueKind::ArrayInitialize {
            size, initializer, ..
        } => {
            value(size, owner, visitor);
            value(initializer, owner, visitor);
        }
        ValueKind::Block(body, result) => {
            statements(body, owner, visitor);
            value(result, owner, visitor);
        }
        ValueKind::Try {
            body,
            result,
            catches,
            finally,
            ..
        } => {
            statements(body, owner, visitor);
            value(result, owner, visitor);
            for catch in catches {
                statements(&mut catch.body, owner, visitor);
                value(&mut catch.result, owner, visitor);
            }
            statements(finally, owner, visitor);
        }
        ValueKind::FunctionInvoke { function, args } => {
            value(function, owner, visitor);
            values(args, owner, visitor);
        }
        ValueKind::CellIncrement { cell: inner, .. }
        | ValueKind::CellDecrement { cell: inner, .. }
        | ValueKind::TypeTest { value: inner, .. }
        | ValueKind::Cast { value: inner, .. }
        | ValueKind::Box(inner)
        | ValueKind::Unbox(inner)
        | ValueKind::IsNull(inner)
        | ValueKind::ListSize(inner)
        | ValueKind::NewArray(_, inner)
        | ValueKind::Convert(_, inner)
        | ValueKind::ThreadLocalGet(inner, _)
        | ValueKind::RequireNotNull(inner)
        | ValueKind::IntToString(inner)
        | ValueKind::NewIntArray(inner)
        | ValueKind::NotNull(inner, _)
        | ValueKind::Field(_, inner)
        | ValueKind::Negate(inner)
        | ValueKind::Not(inner)
        | ValueKind::Length(inner) => value(inner, owner, visitor),
        ValueKind::JavaField { receiver, .. } => {
            if let Some(receiver) = receiver {
                value(receiver, owner, visitor);
            }
        }
        ValueKind::Abrupt(body) => statements(body, owner, visitor),
        ValueKind::ListGet(left, right)
        | ValueKind::ThreadLocalSet(left, right)
        | ValueKind::CopyOf(left, right)
        | ValueKind::StringConcat(left, right)
        | ValueKind::Binary(_, left, right)
        | ValueKind::Index(left, right)
        | ValueKind::Compare(left, right)
        | ValueKind::Let {
            value: left,
            body: right,
            ..
        } => {
            value(left, owner, visitor);
            value(right, owner, visitor);
        }
        ValueKind::ListSet(first, second, third) | ValueKind::If(first, second, third) => {
            value(first, owner, visitor);
            value(second, owner, visitor);
            value(third, owner, visitor);
        }
        ValueKind::ClassLiteral(_)
        | ValueKind::Decrement { .. }
        | ValueKind::Float(_)
        | ValueKind::Double(_)
        | ValueKind::Unit
        | ValueKind::Long(_)
        | ValueKind::Null
        | ValueKind::StaticField { .. }
        | ValueKind::ObjectInstance(_)
        | ValueKind::Increment { .. }
        | ValueKind::CompanionArray { .. }
        | ValueKind::Int(_)
        | ValueKind::Boolean(_)
        | ValueKind::String(_)
        | ValueKind::Local(_) => {}
    }
    visitor.value(node, owner);
}

fn values(nodes: &mut [Value], owner: &str, visitor: &mut impl Visitor) {
    for node in nodes {
        value(node, owner, visitor);
    }
}

pub(crate) fn statements(nodes: &mut [Statement], owner: &str, visitor: &mut impl Visitor) {
    for node in nodes {
        match node {
            Statement::ThrowValue(inner)
            | Statement::Store(_, inner)
            | Statement::Expression(inner)
            | Statement::Return(inner)
            | Statement::NonLocalReturn(inner) => value(inner, owner, visitor),
            Statement::Labeled { body, .. } => statements(body, owner, visitor),
            Statement::JavaFieldStore {
                receiver,
                value: inner,
                ..
            } => {
                if let Some(receiver) = receiver {
                    value(receiver, owner, visitor);
                }
                value(inner, owner, visitor);
            }
            Statement::FieldStore {
                receiver,
                value: inner,
                ..
            } => {
                value(receiver, owner, visitor);
                value(inner, owner, visitor);
            }
            Statement::If(condition, yes, no) => {
                value(condition, owner, visitor);
                statements(yes, owner, visitor);
                statements(no, owner, visitor);
            }
            Statement::While(condition, body) => {
                value(condition, owner, visitor);
                statements(body, owner, visitor);
            }
            Statement::DoWhile(condition, body) => {
                statements(body, owner, visitor);
                value(condition, owner, visitor);
            }
            Statement::ForIterator { iterable, body, .. }
            | Statement::ForEach { iterable, body, .. } => {
                value(iterable, owner, visitor);
                statements(body, owner, visitor);
            }
            Statement::ThrowException { message, .. } => {
                if let Some(message) = message {
                    value(message, owner, visitor);
                }
            }
            Statement::IndexStore(target, index, inner) => {
                value(target, owner, visitor);
                value(index, owner, visitor);
                value(inner, owner, visitor);
            }
            Statement::ForUntil {
                start, end, body, ..
            } => {
                value(start, owner, visitor);
                value(end, owner, visitor);
                statements(body, owner, visitor);
            }
            Statement::BreakLabel(_)
            | Statement::ContinueLabel(_)
            | Statement::Continue
            | Statement::Break => {}
        }
        visitor.statement(node, owner);
    }
}

fn function(node: &mut Function, owner: &str, visitor: &mut impl Visitor) {
    for default in node.defaults.iter_mut().flatten() {
        value(default, owner, visitor);
    }
    statements(&mut node.body, owner, visitor);
}

pub(crate) fn lambdas(nodes: &mut [LambdaDecl], visitor: &mut impl Visitor) {
    for node in nodes {
        function(&mut node.function, &node.owner, visitor);
    }
}

fn companion(node: &mut ClassCompanion, initializer_owner: &str, visitor: &mut impl Visitor) {
    for property in &mut node.properties {
        value(&mut property.value, initializer_owner, visitor);
    }
    statements(&mut node.initializers, initializer_owner, visitor);
    for method in &mut node.methods {
        function(method, &node.owner, visitor);
    }
}

pub(crate) fn class(node: &mut ClassDecl, owner: &str, visitor: &mut impl Visitor) {
    for delegate in &mut node.interface_delegate_storage {
        value(&mut delegate.initializer, owner, visitor);
    }
    for (_, initializer) in &mut node.delegated_properties {
        value(initializer, owner, visitor);
    }
    values(&mut node.super_args, owner, visitor);
    for field in &mut node.fields {
        if let Some(initializer) = &mut field.initializer {
            value(initializer, owner, visitor);
        }
    }
    for default in node.defaults.iter_mut().flatten() {
        value(default, owner, visitor);
    }
    statements(&mut node.constructor_body, owner, visitor);
    for constructor in &mut node.secondary_constructors {
        values(&mut constructor.delegation_args, owner, visitor);
        statements(&mut constructor.initializers, owner, visitor);
        function(&mut constructor.function, owner, visitor);
    }
    for method in &mut node.methods {
        function(method, owner, visitor);
    }
    if let Some(declaration) = &mut node.companion {
        companion(declaration, owner, visitor);
    }
}

fn interface(node: &mut InterfaceDecl, visitor: &mut impl Visitor) {
    let helper_owner = format!("{}$DefaultImpls", node.owner);
    for method in &mut node.methods {
        function(&mut method.function, &helper_owner, visitor);
    }
    for nested in &mut node.nested {
        interface(nested, visitor);
    }
    if let Some(declaration) = &mut node.companion {
        if let Some(state) = &mut declaration.state {
            companion(state, &declaration.owner, visitor);
        }
        for property in &mut declaration.properties {
            match &mut property.value {
                CompanionValue::StringArray(_) => {}
                CompanionValue::Object { owner, class_decl } => class(class_decl, owner, visitor),
            }
        }
    }
}

fn object(node: &mut DynamicObject, visitor: &mut impl Visitor) {
    for field in &mut node.fields {
        match &mut field.initializer {
            ObjectInitializer::Value { value: inner, .. }
            | ObjectInitializer::ThreadLocalArray { size: inner, .. } => {
                value(inner, &node.owner, visitor)
            }
        }
    }
    statements(&mut node.initializers, &node.owner, visitor);
    for method in &mut node.methods {
        function(&mut method.function, &node.owner, visitor);
    }
}

pub(crate) fn module(node: &mut Module, owner: &str, visitor: &mut impl Visitor) {
    for property in &mut node.properties {
        value(&mut property.value, owner, visitor);
    }
    statements(&mut node.initializers, owner, visitor);
    for declaration in &mut node.functions {
        function(declaration, owner, visitor);
    }
    if let Some(declaration) = &mut node.class_decl {
        class(declaration, owner, visitor);
    }
    if let Some(declaration) = &mut node.interface_decl {
        interface(declaration, visitor);
    }
    if let Some(declaration) = &mut node.object_decl {
        object(declaration, visitor);
    }
    lambdas(&mut node.lambdas, visitor);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(number: i32) -> Value {
        Value {
            ty: Type::Int,
            kind: ValueKind::Int(number),
        }
    }

    fn function_with(number: i32) -> Function {
        Function {
            extension_receiver: false,
            class_type_params: Vec::new(),
            jvm_result: None,
            jvm_name: None,
            annotations: Vec::new(),
            vararg: false,
            type_params: Vec::new(),
            defaults: Vec::new(),
            name: "f".into(),
            private: false,
            is_override: false,
            params: Vec::new(),
            result: Type::Int,
            locals: 0,
            body: vec![Statement::Return(int(number))],
        }
    }

    fn companion_with(owner: &str, number: i32) -> ClassCompanion {
        ClassCompanion {
            exposed_fields: HashMap::new(),
            methods: vec![function_with(number + 2)],
            initializers: vec![Statement::Expression(int(number + 1))],
            owner: owner.into(),
            properties: vec![ClassCompanionProperty {
                name: "p".into(),
                ty: Type::Int,
                private: false,
                setter_visibility: syntax::PropertyVisibility::Public,
                mutable: false,
                value: int(number),
            }],
            locals: 0,
        }
    }

    #[test]
    fn statement_visitors_can_replace_stores_after_visiting_their_children() {
        struct Rewrite(Vec<&'static str>);
        impl Visitor for Rewrite {
            fn value(&mut self, node: &mut Value, owner: &str) {
                assert_eq!(owner, "FileKt$lambda$1");
                let ValueKind::Int(number) = &mut node.kind else {
                    panic!("Unexpected test value");
                };
                *number += 10;
                self.0.push("value");
            }

            fn statement(&mut self, node: &mut Statement, owner: &str) {
                assert_eq!(owner, "FileKt$lambda$1");
                match node {
                    Statement::JavaFieldStore {
                        receiver, value, ..
                    } => {
                        assert!(matches!(
                            receiver.as_ref().unwrap().kind,
                            ValueKind::Int(11)
                        ));
                        assert!(matches!(value.kind, ValueKind::Int(12)));
                        *node = Statement::Expression(int(99));
                        self.0.push("store");
                    }
                    Statement::Labeled { body, .. } => {
                        assert!(matches!(body[0], Statement::Expression(_)));
                        self.0.push("label");
                    }
                    _ => panic!("Unexpected test statement"),
                }
            }
        }

        let mut body = vec![Statement::Labeled {
            label: "scope".into(),
            body: vec![Statement::JavaFieldStore {
                owner: "FileKt".into(),
                name: "field".into(),
                ty: Type::Int,
                receiver: Some(int(1)),
                value: int(2),
            }],
        }];
        let mut rewrite = Rewrite(Vec::new());
        statements(&mut body, "FileKt$lambda$1", &mut rewrite);
        assert_eq!(rewrite.0, ["value", "value", "store", "label"]);
    }

    #[test]
    fn values_are_mutable_and_visited_after_their_children_including_abrupt_paths() {
        let mut node = Value {
            ty: Type::Int,
            kind: ValueKind::Try {
                body: vec![Statement::Expression(Value {
                    ty: Type::Int,
                    kind: ValueKind::Binary("+".into(), Box::new(int(1)), Box::new(int(2))),
                })],
                result: Box::new(Value {
                    ty: Type::Int,
                    kind: ValueKind::Block(vec![Statement::Expression(int(3))], Box::new(int(4))),
                }),
                catches: vec![Catch {
                    owner: "java/lang/Exception".into(),
                    slot: 0,
                    body: vec![Statement::Store(0, int(5))],
                    result: int(6),
                }],
                finally: vec![Statement::ThrowException {
                    owner: "java/lang/Exception".into(),
                    message: Some(int(7)),
                }],
                exception_slot: 0,
                result_slot: None,
            },
        };
        let mut visited = Vec::new();
        value(&mut node, "FileKt", &mut |node: &mut Value, owner: &str| {
            assert_eq!(owner, "FileKt");
            match &mut node.kind {
                ValueKind::Int(number) => {
                    visited.push(*number);
                    *number += 10;
                }
                ValueKind::Binary(_, left, right) => {
                    assert!(matches!(left.kind, ValueKind::Int(11)));
                    assert!(matches!(right.kind, ValueKind::Int(12)));
                    visited.push(100);
                }
                ValueKind::Block(_, _) => visited.push(101),
                ValueKind::Try { .. } => visited.push(102),
                _ => panic!("Unexpected test value"),
            }
        });
        assert_eq!(visited, [1, 2, 100, 3, 4, 101, 5, 6, 7, 102]);
    }

    #[test]
    fn declarations_report_the_class_that_emits_each_value() {
        let mut top_function = function_with(2);
        top_function.defaults = vec![Some(int(1))];
        top_function.body.push(Statement::Expression(Value {
            ty: Type::Java("Lambda".into()),
            kind: ValueKind::Lambda {
                owner: "FileKt$lambda$1".into(),
                captures: vec![int(3)],
            },
        }));
        let mut checked = Module {
            exposed_fields: HashMap::new(),
            properties: Vec::new(),
            initializers: Vec::new(),
            locals: 0,
            constants: Vec::new(),
            functions: vec![top_function],
            lambdas: vec![LambdaDecl {
                defaults: Vec::new(),
                owner: "FileKt$lambda$1".into(),
                captures: Vec::new(),
                function: function_with(4),
                interface: None,
            }],
            class_decl: Some(ClassDecl {
                sealed_: false,
                sealed_subclasses: Vec::new(),
                type_params: Vec::new(),
                super_types: Vec::new(),
                interface_delegate_storage: Vec::new(),
                interface_delegated_members: Vec::new(),
                generic_bridges: Vec::new(),
                exposed_fields: HashMap::new(),
                delegated_properties: vec![("delegate".into(), int(5))],
                primary_constructor: true,
                abstract_: false,
                open_methods: HashSet::new(),
                protected_methods: HashSet::new(),
                abstract_methods: HashSet::new(),
                constructor_body: vec![Statement::Expression(int(6))],
                super_class: "java/lang/Object".into(),
                super_args: vec![int(7)],
                open: false,
                secondary_constructors: Vec::new(),
                fields: vec![ClassField {
                    name: "p".into(),
                    ty: Type::Int,
                    private: true,
                    mutable: false,
                    param: None,
                    initializer: Some(int(8)),
                }],
                defaults: vec![Some(int(9))],
                constructor_locals: 0,
                companion: Some(companion_with("FileKt$Companion", 10)),
                params: Vec::new(),
                data: false,
                interfaces: Vec::new(),
                methods: Vec::new(),
                bridges: Vec::new(),
            }),
            interface_decl: Some(InterfaceDecl {
                sealed_: false,
                sealed_subclasses: Vec::new(),
                type_params: Vec::new(),
                super_types: Vec::new(),
                owner: "Contract".into(),
                interfaces: Vec::new(),
                methods: vec![InterfaceMethod {
                    function: function_with(13),
                    has_body: true,
                    source_property: None,
                }],
                nested: Vec::new(),
                defaults: Vec::new(),
                companion: Some(CompanionDecl {
                    state: Some(companion_with("Contract$Companion", 14)),
                    owner: "Contract$Companion".into(),
                    properties: Vec::new(),
                }),
            }),
            object_decl: Some(DynamicObject {
                bridges: Vec::new(),
                exposed_fields: HashMap::new(),
                constants: Vec::new(),
                initializers: vec![Statement::Expression(int(17))],
                locals: 0,
                owner: "Holder".into(),
                interfaces: Vec::new(),
                serializable: false,
                fields: vec![ObjectField {
                    mutable: false,
                    name: "field".into(),
                    ty: Type::IntArray,
                    initializer: ObjectInitializer::ThreadLocalArray {
                        array_type: Type::IntArray,
                        size: int(18),
                        locals: 0,
                    },
                }],
                methods: Vec::new(),
            }),
        };
        let mut visited = HashMap::new();
        module(
            &mut checked,
            "FileKt",
            &mut |node: &mut Value, owner: &str| {
                if let ValueKind::Int(number) = node.kind {
                    assert!(visited.insert(number, owner.to_owned()).is_none());
                }
            },
        );
        for number in 1..=18 {
            let expected = match number {
                4 => "FileKt$lambda$1",
                12 => "FileKt$Companion",
                13 => "Contract$DefaultImpls",
                14..=16 => "Contract$Companion",
                17..=18 => "Holder",
                _ => "FileKt",
            };
            assert_eq!(visited[&number], expected, "value {number}");
        }
    }
}
