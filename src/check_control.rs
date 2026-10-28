use super::*;

impl Checker<'_> {
    fn result_block(
        &mut self,
        statements: &[syntax::Statement],
        _offset: usize,
    ) -> Result<(Vec<Statement>, Value)> {
        let tail = statements.last().and_then(|statement| {
            let block = |body: &[syntax::Statement]| Expr {
                offset: statement.offset,
                kind: ExprKind::Block(body.to_vec()),
            };
            let kind = match &statement.kind {
                StatementKind::Expression(expr) => return Some(expr.clone()),
                StatementKind::When(subject, branches, otherwise) => ExprKind::When(
                    Box::new(subject.clone()),
                    branches
                        .iter()
                        .map(|(conditions, body)| (conditions.clone(), block(body)))
                        .collect(),
                    Box::new(otherwise.as_ref().map(|body| block(body)).unwrap_or(Expr {
                        offset: statement.offset,
                        kind: ExprKind::MissingWhenElse,
                    })),
                ),
                StatementKind::WhenBinding(name, subject, branches, otherwise) => {
                    ExprKind::WhenBinding(
                        name.clone(),
                        Box::new(subject.clone()),
                        branches
                            .iter()
                            .map(|(conditions, body)| (conditions.clone(), block(body)))
                            .collect(),
                        Box::new(otherwise.as_ref().map(|body| block(body)).unwrap_or(Expr {
                            offset: statement.offset,
                            kind: ExprKind::MissingWhenElse,
                        })),
                    )
                }
                _ => return None,
            };
            Some(Expr {
                offset: statement.offset,
                kind,
            })
        });
        let prefix = if tail.is_some() {
            &statements[..statements.len() - 1]
        } else {
            statements
        };
        let (body, abrupt) = self.block_scoped(prefix, false)?;
        let result = if abrupt {
            Value {
                ty: Type::Unit,
                kind: ValueKind::Abrupt(Vec::new()),
            }
        } else if let Some(expr) = tail {
            self.value(&expr)?
        } else {
            Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            }
        };
        self.scopes.pop();
        Ok((body, result))
    }

    pub(super) fn control_value(&mut self, expr: &Expr) -> Result<Value> {
        let offset = expr.offset;
        match &expr.kind {
            ExprKind::Block(statements) => {
                let (mut body, result) = self.result_block(statements, offset)?;
                if let ValueKind::Abrupt(tail) = result.kind {
                    body.extend(tail);
                    return Ok(Value {
                        ty: result.ty,
                        kind: ValueKind::Abrupt(body),
                    });
                }
                Ok(Value {
                    ty: result.ty.clone(),
                    kind: ValueKind::Block(body, Box::new(result)),
                })
            }
            ExprKind::WhenBinding(name, subject, branches, otherwise) => {
                let value = self.value(subject)?;
                self.scopes.push(HashMap::new());
                let slot = self.declare(name, value.ty.clone(), false, offset)?;
                let checked = self.value(&Expr {
                    offset,
                    kind: ExprKind::When(
                        Box::new(Expr {
                            offset,
                            kind: ExprKind::Name(name.clone()),
                        }),
                        branches.clone(),
                        otherwise.clone(),
                    ),
                });
                self.scopes.pop();
                checked.map(|result| Self::wrap_lets(result, vec![(slot, value)]))
            }
            ExprKind::Labeled(label, value) => {
                let expected = self.result.clone();
                if matches!(
                    value.kind,
                    ExprKind::Lambda(_) | ExprKind::LambdaWithParams { .. }
                ) {
                    return Err(Diagnostic::new(
                        offset,
                        "Labeled lambda requires an expected function type",
                    ));
                }
                let _ = (label, expected);
                self.value(value)
            }
            ExprKind::LabeledReturn(label, value) => {
                if !self
                    .scopes
                    .iter()
                    .any(|scope| scope.contains_key(&format!("<return@{label}>")))
                {
                    return Err(Diagnostic::new(
                        offset,
                        format!("Unresolved return label: {label}"),
                    ));
                }
                let value = if let Some(value) = value {
                    self.expected_value(value, &self.result.clone())?
                } else if self.result == Type::Unit {
                    Value {
                        ty: Type::Unit,
                        kind: ValueKind::Unit,
                    }
                } else {
                    return Err(Diagnostic::new(offset, "Labeled return requires a value"));
                };
                Ok(Value {
                    ty: self.result.clone(),
                    kind: ValueKind::Abrupt(vec![Statement::Return(value)]),
                })
            }
            ExprKind::Try {
                body,
                catches,
                finally,
            } => {
                let before = self.initialized.clone();
                let refinements = self.refinements.clone();
                let (body, mut result) = self.result_block(body, offset)?;
                let mut initialized = if matches!(result.kind, ValueKind::Abrupt(_)) {
                    None
                } else {
                    Some(self.initialized.clone())
                };
                let mut checked_catches = Vec::new();
                for catch in catches {
                    self.initialized = before.clone();
                    self.refinements = refinements.clone();
                    let ty = self.resolve(&catch.ty, offset)?;
                    let owner = library::type_owner(&ty)
                        .ok_or_else(|| {
                            Diagnostic::new(offset, "Catch type must be a throwable class")
                        })?
                        .to_owned();
                    if !self.subtype(&ty, &Type::Java("java/lang/Throwable".into())) {
                        return Err(Diagnostic::new(offset, "Catch type must inherit Throwable"));
                    }
                    self.scopes.push(HashMap::new());
                    let slot = self.declare(&catch.name, ty, false, offset)?;
                    let (catch_body, catch_result) = self.result_block(&catch.body, offset)?;
                    self.scopes.pop();
                    if !matches!(catch_result.kind, ValueKind::Abrupt(_)) {
                        initialized = Some(match initialized {
                            Some(previous) => {
                                previous.intersection(&self.initialized).copied().collect()
                            }
                            None => self.initialized.clone(),
                        });
                    }
                    checked_catches.push(Catch {
                        owner,
                        slot,
                        body: catch_body,
                        result: catch_result,
                    });
                }
                let mut ty = result.ty.clone();
                for catch in &checked_catches {
                    if matches!(catch.result.kind, ValueKind::Abrupt(_)) {
                        continue;
                    }
                    if matches!(result.kind, ValueKind::Abrupt(_))
                        || self.subtype(&ty, &catch.result.ty)
                    {
                        ty = catch.result.ty.clone();
                    } else if !self.subtype(&catch.result.ty, &ty) {
                        ty = Type::Java("java/lang/Object".into());
                    }
                }
                if !matches!(result.kind, ValueKind::Abrupt(_)) {
                    result = self.coerce(result, ty.clone(), offset)?;
                }
                for catch in &mut checked_catches {
                    if !matches!(catch.result.kind, ValueKind::Abrupt(_)) {
                        let value = std::mem::replace(
                            &mut catch.result,
                            Value {
                                ty: Type::Unit,
                                kind: ValueKind::Unit,
                            },
                        );
                        catch.result = self.coerce(value, ty.clone(), offset)?;
                    }
                }
                self.initialized = before.clone();
                self.refinements = refinements;
                let (finally, _) = self.block(finally)?;
                if let Some(initialized) = initialized {
                    self.initialized.extend(initialized);
                }
                let exception_slot = self.allocate_slot(offset)?;
                let result_slot = if ty == Type::Unit {
                    None
                } else {
                    Some(self.allocate_typed_slot(&ty, offset)?)
                };
                Ok(Value {
                    ty,
                    kind: ValueKind::Try {
                        body,
                        result: Box::new(result),
                        catches: checked_catches,
                        finally,
                        exception_slot,
                        result_slot,
                    },
                })
            }
            _ => Err(Diagnostic::new(offset, "Unsupported control expression")),
        }
    }

    pub(super) fn control_statement(
        &mut self,
        statement: &syntax::Statement,
    ) -> Result<(Vec<Statement>, bool)> {
        let offset = statement.offset;
        match &statement.kind {
            StatementKind::Block(body) => self.block(body),
            StatementKind::LocalFunction(function) => self.local_function(function),
            StatementKind::Labeled(label, statement) => {
                if !matches!(
                    statement.kind,
                    StatementKind::While(..)
                        | StatementKind::DoWhile(..)
                        | StatementKind::ForEach(..)
                        | StatementKind::ForUntil(..)
                ) {
                    return Err(Diagnostic::new(offset, "Loop labels must target a loop"));
                }
                let marker = format!("<loop@{label}>");
                self.scopes.push(HashMap::from([(
                    marker,
                    Local {
                        slot: 0,
                        ty: Type::Unit,
                        mutable: false,
                    },
                )]));
                let (body, _) = self.block(std::slice::from_ref(statement))?;
                self.scopes.pop();
                Ok((
                    vec![Statement::Labeled {
                        label: label.clone(),
                        body,
                    }],
                    false,
                ))
            }
            StatementKind::BreakLabel(label) | StatementKind::ContinueLabel(label) => {
                if !self
                    .scopes
                    .iter()
                    .any(|scope| scope.contains_key(&format!("<loop@{label}>")))
                {
                    return Err(Diagnostic::new(
                        offset,
                        format!("Unresolved loop label: {label}"),
                    ));
                }
                Ok((
                    vec![if matches!(statement.kind, StatementKind::BreakLabel(_)) {
                        Statement::BreakLabel(label.clone())
                    } else {
                        Statement::ContinueLabel(label.clone())
                    }],
                    true,
                ))
            }
            StatementKind::Destructure(names, mutable, expression) => {
                let value = self.value(expression)?;
                let slot = self.allocate_typed_slot(&value.ty, offset)?;
                let mut body = vec![Statement::Store(slot, value.clone())];
                for (index, entry) in names.iter().enumerate() {
                    if entry.name.is_none() && entry.ty.is_none() {
                        continue;
                    }
                    let mut component = self.operator_call(
                        Value {
                            ty: value.ty.clone(),
                            kind: ValueKind::Local(slot),
                        },
                        &format!("component{}", index + 1),
                        &[],
                        offset,
                    )?;
                    if let Some(annotation) = &entry.ty {
                        let expected = self.resolve(annotation, offset)?;
                        component = self.coerce(component, expected, offset)?;
                    }
                    let Some(name) = &entry.name else {
                        continue;
                    };
                    if component.ty == Type::Unit {
                        return Err(Diagnostic::new(
                            offset,
                            "Unit destructuring variables are unsupported",
                        ));
                    }
                    let target = self.declare(name, component.ty.clone(), *mutable, offset)?;
                    if *mutable && self.cell_names.contains(name) {
                        let owner = Self::cell_owner(&component.ty);
                        self.cells.insert(target, owner.clone());
                        body.push(Statement::Store(
                            target,
                            Value {
                                ty: Type::Java(owner.clone()),
                                kind: ValueKind::New {
                                    owner: owner.clone(),
                                    descriptor: "()V".into(),
                                    args: Vec::new(),
                                },
                            },
                        ));
                        body.push(self.cell_store(
                            Value {
                                ty: Type::Java(owner),
                                kind: ValueKind::Local(target),
                            },
                            component.ty.clone(),
                            component,
                            offset,
                        )?);
                    } else {
                        body.push(Statement::Store(target, component));
                    }
                }
                Ok((body, false))
            }
            StatementKind::WhenBinding(name, subject, branches, otherwise) => {
                let value = self.value(subject)?;
                self.scopes.push(HashMap::new());
                let slot = self.declare(name, value.ty.clone(), false, offset)?;
                let (mut body, returns) = self.block(&[syntax::Statement {
                    offset,
                    kind: StatementKind::When(
                        Expr {
                            offset,
                            kind: ExprKind::Name(name.clone()),
                        },
                        branches.clone(),
                        otherwise.clone(),
                    ),
                }])?;
                self.scopes.pop();
                body.insert(0, Statement::Store(slot, value));
                Ok((body, returns))
            }
            _ => Err(Diagnostic::new(offset, "Unsupported control statement")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checked(source: &str) -> Result<Module> {
        check(&syntax::parse(source)?, &Classpath::new(&[])?)
    }

    #[test]
    fn local_functions_capture_mutable_cells_and_recursive_self() {
        let module = checked("fun f(): Int { var calls=0; fun fact(n:Int):Int { calls += 1; if(n==0) return 1; return n * fact(n-1) }; val answer=fact(4); return answer+calls }").unwrap();
        assert_eq!(module.lambdas.len(), 1);
        assert!(
            module.lambdas[0]
                .captures
                .iter()
                .any(|(name, _)| name == "fact")
        );
        assert!(
            module.lambdas[0]
                .captures
                .iter()
                .any(|(name, ty)| name == "calls"
                    && matches!(ty,Type::Java(owner) if owner.ends_with("IntRef")))
        );
    }

    #[test]
    fn array_initializer_expressions_require_stable_function_values() {
        checked("fun f():Int { val init:(Int)->Int={it}; val values=Array<Int>(2,init); return values[1] }").unwrap();
        let error =
            checked("fun factory():(Int)->Int={it}\nfun f():Int=Array<Int>(2,factory())[0]")
                .err()
                .unwrap();
        assert!(error.message.contains("specification/oracle discrepancy"));
    }

    #[test]
    fn local_function_self_cells_keep_declaration_immutability() {
        assert!(
            checked("fun f():Int { fun helper():Int=1; helper={2}; return helper() }").is_err()
        );
        assert!(
            checked("fun f():Int { fun helper():Int { helper={2}; return 1 }; return helper() }")
                .is_err()
        );
        assert!(
            checked("fun f():Int { var n:Int; val read:()->Int={n}; n=1; return read() }").is_err()
        );
    }

    #[test]
    fn array_constructor_and_receiver_lambda_keep_semantic_element_types() {
        let module=checked("fun f():Int { val a=Array<Int>(3) { it+1 }; a[1]=7; var n=0; for(v in a) { n += v }; val block: Int.(Int) -> Int = { x -> this+x }; return n+block(2,3) }").unwrap();
        assert_eq!(module.lambdas.len(), 2);
        assert_eq!(module.lambdas[1].function.params[0].1, Type::Int);
    }

    #[test]
    fn mutable_destructured_values_share_capture_cells() {
        let module=checked("class Parts(val value:Int) { operator fun component1():Int=value; fun f():Int { var (n)=Parts(2); val next:()->Int={ n += 1; n }; return next()+n } }").unwrap();
        assert!(
            module.lambdas[0]
                .captures
                .iter()
                .any(|(name, ty)| name == "n"
                    && matches!(ty,Type::Java(owner) if owner.ends_with("IntRef")))
        );
    }

    #[test]
    fn when_bindings_do_not_escape_their_branch_scope() {
        checked("fun f():Int { when(val n=3) { 3 -> return n; else -> return 0 } }").unwrap();
        assert!(checked("fun f():Int { when(val n=3) { 3 -> 1; else -> 2 }; return n }").is_err());
    }

    #[test]
    fn loop_labels_cannot_cross_lambda_boundaries() {
        assert!(checked("fun f():Int { outer@ while(true) { val block: () -> Unit = { break@outer }; block() }; return 1 }").is_err());
    }

    #[test]
    fn throwable_values_and_catch_bindings_can_be_rethrown() {
        checked("import java.lang.Exception\nfun f(error:Exception):Int { try { throw error } catch(next:Exception) { throw next } }").unwrap();
        assert!(checked("fun f():Int { throw null }").is_err());
    }

    #[test]
    fn local_expression_functions_infer_returns_and_literals_keep_context() {
        checked("fun f(seed:Int):Int { fun add(n:Int)=n+seed; return add(3) }\nfun positive():Long=42\nfun negative():Long=-42").unwrap();
        assert!(checked("fun invalid(n:Int):Long=n").is_err());
    }

    #[test]
    fn explicitly_labeled_returns_are_local_to_the_lambda() {
        let module=checked("fun f():Int { val block: (Int) -> Int = done@ { n -> if(n==0) return@done 4; n+1 }; return block(0) }").unwrap();
        assert_eq!(module.lambdas.len(), 1);
    }
}

pub(super) fn expression_terminates(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Return(_) | ExprKind::Throw(_) | ExprKind::LabeledReturn(..) => true,
        ExprKind::Block(body) => statements_terminate(body),
        ExprKind::Labeled(_, value) => expression_terminates(value),
        ExprKind::If(_, yes, no) => expression_terminates(yes) && expression_terminates(no),
        ExprKind::When(_, branches, tail) | ExprKind::WhenBinding(_, _, branches, tail) => {
            branches
                .iter()
                .all(|(_, value)| expression_terminates(value))
                && expression_terminates(tail)
        }
        ExprKind::Try {
            body,
            catches,
            finally,
        } => {
            statements_terminate(finally)
                || statements_terminate(body)
                    && catches
                        .iter()
                        .all(|catch| statements_terminate(&catch.body))
        }
        _ => false,
    }
}

fn statements_terminate(body: &[syntax::Statement]) -> bool {
    body.last().is_some_and(|statement| match &statement.kind {
        StatementKind::Return(_) | StatementKind::Throw(_) => true,
        StatementKind::Expression(expr) => expression_terminates(expr),
        StatementKind::Block(body) => statements_terminate(body),
        StatementKind::If(_, yes, no) => statements_terminate(yes) && statements_terminate(no),
        StatementKind::When(_, branches, Some(tail))
        | StatementKind::WhenBinding(_, _, branches, Some(tail)) => {
            branches.iter().all(|(_, body)| statements_terminate(body))
                && statements_terminate(tail)
        }
        _ => false,
    })
}

#[cfg(test)]
mod java_lang_tests {
    use super::*;
    #[test]
    fn imported_java_lang_noarg_constructors_resolve() {
        let cp = Classpath::new(&[]).unwrap();
        let class = cp.load("java/lang/StringBuilder").unwrap();
        assert!(class.methods.iter().any(|method| method.name == "<init>"
            && method.descriptor == "()V"
            && !method.unsupported));
        check(
            &syntax::parse("import java.lang.StringBuilder\nfun f():StringBuilder=StringBuilder()")
                .unwrap(),
            &cp,
        )
        .unwrap();
    }
}
