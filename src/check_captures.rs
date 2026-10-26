use super::*;

pub(super) fn expression_names(expr: &Expr, names: &mut HashSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            names.insert(name.clone());
        }
        ExprKind::Call(target, args) => {
            expression_names(target, names);
            for arg in args {
                expression_names(arg, names);
            }
        }
        ExprKind::Generic(expr, _)
        | ExprKind::NamedArgument(_, expr)
        | ExprKind::Unary(_, expr)
        | ExprKind::NonNull(expr)
        | ExprKind::Lambda(expr)
        | ExprKind::Throw(expr)
        | ExprKind::Increment { value: expr, .. }
        | ExprKind::Decrement { value: expr, .. }
        | ExprKind::TypeOperation(_, expr, _)
        | ExprKind::Property(expr, _)
        | ExprKind::SafeProperty(expr, _) => expression_names(expr, names),
        ExprKind::LambdaWithParams { body, .. } | ExprKind::Block(body) => {
            statement_names(body, names)
        }
        ExprKind::Labeled(_, value) => expression_names(value, names),
        ExprKind::LabeledReturn(_, Some(value)) => expression_names(value, names),
        ExprKind::Indices(receiver, indices) => {
            expression_names(receiver, names);
            for index in indices {
                expression_names(index, names);
            }
        }
        ExprKind::Try {
            body,
            catches,
            finally,
        } => {
            statement_names(body, names);
            for catch in catches {
                statement_names(&catch.body, names);
            }
            statement_names(finally, names);
        }
        ExprKind::WhenBinding(_, subject, branches, otherwise) => {
            expression_names(subject, names);
            for (conditions, body) in branches {
                for condition in conditions {
                    expression_names(condition, names);
                }
                expression_names(body, names);
            }
            expression_names(otherwise, names);
        }
        ExprKind::CallableReference {
            receiver: Some(receiver),
            ..
        } => expression_names(receiver, names),
        ExprKind::Return(Some(expr)) => expression_names(expr, names),
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            expression_names(a, names);
            expression_names(b, names);
        }
        ExprKind::If(a, b, c) => {
            expression_names(a, names);
            expression_names(b, names);
            expression_names(c, names);
        }
        ExprKind::Template(parts) => {
            for part in parts {
                expression_names(part, names);
            }
        }
        ExprKind::When(subject, branches, otherwise) => {
            expression_names(subject, names);
            for (conditions, body) in branches {
                for condition in conditions {
                    expression_names(condition, names);
                }
                expression_names(body, names);
            }
            expression_names(otherwise, names);
        }
        _ => {}
    }
}
pub(super) fn statement_names(statements: &[syntax::Statement], names: &mut HashSet<String>) {
    for statement in statements {
        match &statement.kind {
            StatementKind::LocalFunction(function) => statement_names(&function.body, names),
            StatementKind::Block(body) => statement_names(body, names),
            StatementKind::Labeled(_, body) => statement_names(std::slice::from_ref(body), names),
            StatementKind::Destructure(_, _, expr) => expression_names(expr, names),
            StatementKind::IndicesAssign(receiver, indices, value) => {
                expression_names(receiver, names);
                for index in indices {
                    expression_names(index, names);
                }
                expression_names(value, names);
            }
            StatementKind::WhenBinding(_, subject, branches, otherwise) => {
                expression_names(subject, names);
                for (conditions, body) in branches {
                    for condition in conditions {
                        expression_names(condition, names);
                    }
                    statement_names(body, names);
                }
                if let Some(body) = otherwise {
                    statement_names(body, names);
                }
            }
            StatementKind::Let(_, _, _, expr)
            | StatementKind::Throw(expr)
            | StatementKind::Expression(expr) => expression_names(expr, names),
            StatementKind::Assign(name, expr)
            | StatementKind::AddAssign(name, expr)
            | StatementKind::CompoundAssign(name, _, expr) => {
                names.insert(name.clone());
                expression_names(expr, names);
            }
            StatementKind::Return(Some(expr)) => expression_names(expr, names),
            StatementKind::IndexAssign(a, b, c) => {
                expression_names(a, names);
                expression_names(b, names);
                expression_names(c, names);
            }
            StatementKind::PropertyAssign(a, _, b) => {
                expression_names(a, names);
                expression_names(b, names);
            }
            StatementKind::If(a, b, c) => {
                expression_names(a, names);
                statement_names(b, names);
                statement_names(c, names);
            }
            StatementKind::While(a, b) | StatementKind::DoWhile(a, b) => {
                expression_names(a, names);
                statement_names(b, names);
            }
            StatementKind::ForEach(_, a, b) => {
                expression_names(a, names);
                statement_names(b, names);
            }
            StatementKind::ForUntil(_, a, b, c) => {
                expression_names(a, names);
                expression_names(b, names);
                statement_names(c, names);
            }
            StatementKind::When(a, b, c) => {
                expression_names(a, names);
                for (conditions, body) in b {
                    for condition in conditions {
                        expression_names(condition, names);
                    }
                    statement_names(body, names);
                }
                if let Some(body) = c {
                    statement_names(body, names);
                }
            }
            _ => {}
        }
    }
}
pub(super) fn lambda_capture_names(statements: &[syntax::Statement]) -> HashSet<String> {
    fn expr(expr: &Expr, names: &mut HashSet<String>) {
        match &expr.kind {
            ExprKind::Labeled(_, value) => expr_visit(value, names),
            ExprKind::Block(body) => visit(body, names),
            ExprKind::Try {
                body,
                catches,
                finally,
            } => {
                visit(body, names);
                for catch in catches {
                    visit(&catch.body, names);
                }
                visit(finally, names);
            }
            ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. } => {
                expression_names(expr, names)
            }
            ExprKind::Call(target, args) => {
                expr_visit(target, names);
                for arg in args {
                    expr_visit(arg, names);
                }
            }
            ExprKind::Generic(a, _)
            | ExprKind::NamedArgument(_, a)
            | ExprKind::Unary(_, a)
            | ExprKind::NonNull(a)
            | ExprKind::Throw(a)
            | ExprKind::Increment { value: a, .. }
            | ExprKind::Decrement { value: a, .. }
            | ExprKind::TypeOperation(_, a, _)
            | ExprKind::Property(a, _)
            | ExprKind::SafeProperty(a, _) => expr_visit(a, names),
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                expr_visit(a, names);
                expr_visit(b, names);
            }
            ExprKind::If(a, b, c) => {
                expr_visit(a, names);
                expr_visit(b, names);
                expr_visit(c, names);
            }
            ExprKind::Return(Some(a)) => expr_visit(a, names),
            ExprKind::Template(parts) => {
                for part in parts {
                    expr_visit(part, names);
                }
            }
            ExprKind::When(subject, branches, otherwise) => {
                expr_visit(subject, names);
                for (conditions, value) in branches {
                    for condition in conditions {
                        expr_visit(condition, names);
                    }
                    expr_visit(value, names);
                }
                expr_visit(otherwise, names);
            }
            _ => {}
        }
    }
    fn expr_visit(e: &Expr, names: &mut HashSet<String>) {
        expr(e, names);
    }
    fn visit(statements: &[syntax::Statement], names: &mut HashSet<String>) {
        for s in statements {
            match &s.kind {
                StatementKind::LocalFunction(function) => statement_names(&function.body, names),
                StatementKind::Block(body) => visit(body, names),
                StatementKind::Labeled(_, body) => visit(std::slice::from_ref(body), names),
                StatementKind::Destructure(_, _, value) => expr(value, names),
                StatementKind::Let(_, _, _, e)
                | StatementKind::Assign(_, e)
                | StatementKind::AddAssign(_, e)
                | StatementKind::CompoundAssign(_, _, e)
                | StatementKind::Throw(e)
                | StatementKind::Expression(e) => expr(e, names),
                StatementKind::Return(Some(e)) => expr(e, names),
                StatementKind::If(e, a, b) => {
                    expr(e, names);
                    visit(a, names);
                    visit(b, names);
                }
                StatementKind::While(e, b)
                | StatementKind::DoWhile(e, b)
                | StatementKind::ForEach(_, e, b) => {
                    expr(e, names);
                    visit(b, names);
                }
                StatementKind::ForUntil(_, a, b, c) => {
                    expr(a, names);
                    expr(b, names);
                    visit(c, names);
                }
                StatementKind::IndexAssign(a, b, c) => {
                    expr(a, names);
                    expr(b, names);
                    expr(c, names);
                }
                StatementKind::PropertyAssign(a, _, b) => {
                    expr(a, names);
                    expr(b, names);
                }
                StatementKind::When(subject, branches, otherwise) => {
                    expr(subject, names);
                    for (conditions, body) in branches {
                        for condition in conditions {
                            expr(condition, names);
                        }
                        visit(body, names);
                    }
                    if let Some(body) = otherwise {
                        visit(body, names);
                    }
                }
                _ => {}
            }
        }
    }
    let mut names = HashSet::new();
    visit(statements, &mut names);
    names
}

impl Checker<'_> {
    pub(super) fn cell_binding(&self, name: &str) -> Option<(Value, Type)> {
        if let Some(local) = self.local(name) {
            if let Some(owner) = self.cells.get(&local.slot) {
                return Some((
                    Value {
                        ty: Type::Java(owner.clone()),
                        kind: ValueKind::Local(local.slot),
                    },
                    local.ty,
                ));
            }
            return None;
        }
        let semantic = self.captured_cells.get(name)?;
        let (owner, ty) = self.captured.get(name)?;
        Some((
            Value {
                ty: ty.clone(),
                kind: ValueKind::JavaField {
                    owner: owner.clone(),
                    name: name.into(),
                    receiver: Some(Box::new(Value {
                        ty: Type::Java(owner.clone()),
                        kind: ValueKind::Local(0),
                    })),
                },
            },
            semantic.clone(),
        ))
    }
    pub(super) fn cell_read(&self, cell: Value, ty: Type) -> Value {
        let owner = library::type_owner(&cell.ty).unwrap().to_owned();
        Value {
            ty,
            kind: ValueKind::JavaField {
                owner,
                name: "element".into(),
                receiver: Some(Box::new(cell)),
            },
        }
    }
    pub(super) fn cell_store(
        &self,
        cell: Value,
        ty: Type,
        value: Value,
        offset: usize,
    ) -> Result<Statement> {
        let owner = library::type_owner(&cell.ty).unwrap().to_owned();
        let storage = if owner.ends_with("$ObjectRef") {
            Type::Nullable(Box::new(Type::Java("java/lang/Object".into())))
        } else {
            ty
        };
        let value = self.coerce(value, storage.clone(), offset)?;
        Ok(Statement::JavaFieldStore {
            owner,
            name: "element".into(),
            ty: storage,
            receiver: Some(cell),
            value,
        })
    }
    pub(super) fn cell_owner(ty: &Type) -> String {
        let suffix = match ty {
            Type::Int => "Int",
            Type::Long => "Long",
            Type::Float => "Float",
            Type::Double => "Double",
            Type::Byte => "Byte",
            Type::Char => "Char",
            Type::Boolean => "Boolean",
            _ => "Object",
        };
        format!("kotlin/jvm/internal/Ref${suffix}Ref")
    }
}

pub(super) fn free_names(
    statements: &[syntax::Statement],
    parameters: &[String],
) -> HashSet<String> {
    fn expression(expr: &Expr, bound: &HashSet<String>, free: &mut HashSet<String>) {
        match &expr.kind {
            ExprKind::Name(name) => {
                if !bound.contains(name) {
                    free.insert(name.clone());
                }
            }
            ExprKind::Block(body) => body_names(body, &mut bound.clone(), free),
            ExprKind::Labeled(_, value) => expression(value, bound, free),
            ExprKind::LabeledReturn(_, Some(value)) => expression(value, bound, free),
            ExprKind::Indices(receiver, indices) => {
                expression(receiver, bound, free);
                for index in indices {
                    expression(index, bound, free);
                }
            }
            ExprKind::Try {
                body,
                catches,
                finally,
            } => {
                body_names(body, &mut bound.clone(), free);
                for catch in catches {
                    let mut bound = bound.clone();
                    bound.insert(catch.name.clone());
                    body_names(&catch.body, &mut bound, free);
                }
                body_names(finally, &mut bound.clone(), free);
            }
            ExprKind::WhenBinding(name, subject, branches, otherwise) => {
                expression(subject, bound, free);
                let mut bound = bound.clone();
                bound.insert(name.clone());
                for (conditions, body) in branches {
                    for condition in conditions {
                        expression(condition, &bound, free);
                    }
                    expression(body, &bound, free);
                }
                expression(otherwise, &bound, free);
            }
            ExprKind::LambdaWithParams { params, body } => {
                let mut bound = bound.clone();
                if let Some(params) = params {
                    bound.extend(params.iter().map(|p| p.name.clone()));
                } else {
                    bound.insert("it".into());
                }
                body_names(body, &mut bound, free);
            }
            ExprKind::Lambda(body) => {
                let mut bound = bound.clone();
                bound.insert("it".into());
                expression(body, &bound, free);
            }
            ExprKind::Call(target, args) => {
                expression(target, bound, free);
                for arg in args {
                    expression(arg, bound, free);
                }
            }
            ExprKind::Generic(a, _)
            | ExprKind::NamedArgument(_, a)
            | ExprKind::Unary(_, a)
            | ExprKind::NonNull(a)
            | ExprKind::Throw(a)
            | ExprKind::Increment { value: a, .. }
            | ExprKind::Decrement { value: a, .. }
            | ExprKind::TypeOperation(_, a, _)
            | ExprKind::Property(a, _)
            | ExprKind::SafeProperty(a, _) => expression(a, bound, free),
            ExprKind::CallableReference {
                receiver: Some(a), ..
            }
            | ExprKind::Return(Some(a)) => expression(a, bound, free),
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                expression(a, bound, free);
                expression(b, bound, free);
            }
            ExprKind::If(a, b, c) => {
                expression(a, bound, free);
                expression(b, bound, free);
                expression(c, bound, free);
            }
            ExprKind::Template(parts) => {
                for part in parts {
                    expression(part, bound, free);
                }
            }
            ExprKind::When(a, branches, otherwise) => {
                expression(a, bound, free);
                for (conditions, value) in branches {
                    for condition in conditions {
                        expression(condition, bound, free);
                    }
                    expression(value, bound, free);
                }
                expression(otherwise, bound, free);
            }
            _ => {}
        }
    }
    fn body_names(
        statements: &[syntax::Statement],
        bound: &mut HashSet<String>,
        free: &mut HashSet<String>,
    ) {
        for statement in statements {
            match &statement.kind {
                StatementKind::LocalFunction(function) => {
                    bound.insert(function.name.clone());
                    let mut nested = bound.clone();
                    nested.extend(function.params.iter().map(|p| p.0.clone()));
                    body_names(&function.body, &mut nested, free);
                }
                StatementKind::Block(body) => body_names(body, &mut bound.clone(), free),
                StatementKind::Labeled(_, body) => {
                    body_names(std::slice::from_ref(body), &mut bound.clone(), free)
                }
                StatementKind::Destructure(names, _, value) => {
                    expression(value, bound, free);
                    bound.extend(names.iter().filter_map(|entry| entry.name.clone()));
                }
                StatementKind::IndicesAssign(receiver, indices, value) => {
                    expression(receiver, bound, free);
                    for index in indices {
                        expression(index, bound, free);
                    }
                    expression(value, bound, free);
                }
                StatementKind::WhenBinding(name, subject, branches, otherwise) => {
                    expression(subject, bound, free);
                    let mut bound = bound.clone();
                    bound.insert(name.clone());
                    for (conditions, body) in branches {
                        for condition in conditions {
                            expression(condition, &bound, free);
                        }
                        body_names(body, &mut bound.clone(), free);
                    }
                    if let Some(body) = otherwise {
                        body_names(body, &mut bound, free);
                    }
                }
                StatementKind::Let(name, _, _, expr) => {
                    expression(expr, bound, free);
                    bound.insert(name.clone());
                }
                StatementKind::Declare(name, _, _) => {
                    bound.insert(name.clone());
                }
                StatementKind::Assign(name, expr)
                | StatementKind::AddAssign(name, expr)
                | StatementKind::CompoundAssign(name, _, expr) => {
                    if !bound.contains(name) {
                        free.insert(name.clone());
                    }
                    expression(expr, bound, free);
                }
                StatementKind::Return(Some(expr))
                | StatementKind::Throw(expr)
                | StatementKind::Expression(expr) => expression(expr, bound, free),
                StatementKind::IndexAssign(a, b, c) => {
                    expression(a, bound, free);
                    expression(b, bound, free);
                    expression(c, bound, free);
                }
                StatementKind::PropertyAssign(a, _, b) => {
                    expression(a, bound, free);
                    expression(b, bound, free);
                }
                StatementKind::If(a, b, c) => {
                    expression(a, bound, free);
                    body_names(b, &mut bound.clone(), free);
                    body_names(c, &mut bound.clone(), free);
                }
                StatementKind::While(a, b) | StatementKind::DoWhile(a, b) => {
                    expression(a, bound, free);
                    body_names(b, &mut bound.clone(), free);
                }
                StatementKind::ForEach(name, a, b) => {
                    expression(a, bound, free);
                    let mut bound = bound.clone();
                    bound.insert(name.clone());
                    body_names(b, &mut bound, free);
                }
                StatementKind::ForUntil(name, a, b, c) => {
                    expression(a, bound, free);
                    expression(b, bound, free);
                    let mut bound = bound.clone();
                    bound.insert(name.clone());
                    body_names(c, &mut bound, free);
                }
                StatementKind::When(a, branches, otherwise) => {
                    expression(a, bound, free);
                    for (conditions, body) in branches {
                        for condition in conditions {
                            expression(condition, bound, free);
                        }
                        body_names(body, &mut bound.clone(), free);
                    }
                    if let Some(body) = otherwise {
                        body_names(body, &mut bound.clone(), free);
                    }
                }
                _ => {}
            }
        }
    }
    let mut bound = parameters.iter().cloned().collect();
    let mut free = HashSet::new();
    body_names(statements, &mut bound, &mut free);
    free
}

impl Checker<'_> {
    pub(super) fn cell_is_readonly(&self, name: &str) -> bool {
        if let Some(local) = self.local(name) {
            return !local.mutable && self.cells.contains_key(&local.slot);
        }
        self.scopes
            .iter()
            .any(|scope| scope.contains_key(&format!("<readonlyCell:{name}>")))
    }
}
