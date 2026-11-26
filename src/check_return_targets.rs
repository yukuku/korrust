use super::*;

pub(super) type ReturnCheckpoint = Vec<(Rc<RefCell<Vec<Type>>>, usize)>;

#[derive(Clone)]
struct Target {
    owner: String,
    result: Option<Type>,
    inferred: Option<Rc<RefCell<Vec<Type>>>>,
}

#[derive(Clone, Default)]
pub(super) struct ReturnContext {
    targets: HashMap<String, Target>,
    owner: Option<String>,
    pub(super) inline_call: bool,
}

impl ReturnContext {
    pub(super) fn checkpoint(&self) -> ReturnCheckpoint {
        self.targets
            .values()
            .filter_map(|target| {
                target
                    .inferred
                    .as_ref()
                    .map(|values| (values.clone(), values.borrow().len()))
            })
            .collect()
    }
    pub(super) fn restore(checkpoint: ReturnCheckpoint) {
        for (values, length) in checkpoint {
            values.borrow_mut().truncate(length);
        }
    }

    pub(super) fn lambda(
        &self,
        owner: String,
        label: Option<&str>,
        result: Option<Type>,
        inferred: Option<Rc<RefCell<Vec<Type>>>>,
    ) -> Self {
        let mut targets = if self.inline_call {
            self.targets.clone()
        } else {
            HashMap::new()
        };
        if let Some(label) = label {
            targets.insert(
                label.into(),
                Target {
                    owner: owner.clone(),
                    result,
                    inferred,
                },
            );
        }
        Self {
            targets,
            owner: Some(owner),
            inline_call: false,
        }
    }
}

impl Checker<'_> {
    pub(super) fn labeled_return(
        &mut self,
        label: &str,
        expression: Option<&Expr>,
        offset: usize,
    ) -> Result<Value> {
        let target = self
            .label_returns
            .targets
            .get(label)
            .cloned()
            .ok_or_else(|| {
                Diagnostic::new(offset, format!("Unresolved return label: {label}")).with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "UNRESOLVED_RETURN_LABEL",
                )
            })?;
        let value = match (expression, &target.result) {
            (Some(expression), Some(expected)) => self.expected_value(expression, expected)?,
            (Some(expression), None) => self.value(expression)?,
            (None, None | Some(Type::Unit)) => Value {
                ty: Type::Unit,
                kind: ValueKind::Unit,
            },
            (None, _) => return Err(Diagnostic::new(offset, "Labeled return requires a value")),
        };
        if let Some(inferred) = target.inferred {
            inferred.borrow_mut().push(value.ty.clone());
        }
        let ty = Type::Java("kotlin/Nothing".into());
        let statement = if self.label_returns.owner.as_deref() == Some(&target.owner) {
            Statement::Return(value)
        } else {
            Statement::TargetReturn {
                target: target.owner,
                value,
            }
        };
        Ok(Value {
            ty,
            kind: ValueKind::Abrupt(vec![statement]),
        })
    }
}
