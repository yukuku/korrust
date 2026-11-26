use super::*;

impl Checker<'_> {
    pub(super) fn assigned_value(&mut self, expr: &Expr, expected: &Type) -> Result<(Value, Type)> {
        let saved = self.clone();
        let count = self.lambdas.borrow().len();
        if let Ok(value) = self.value_expected(expr, Some(expected)) {
            let actual = value.ty.clone();
            if let Ok(value) = self.coerce(value, expected.clone(), expr.offset) {
                return Ok((value, actual));
            }
        }
        *self = saved;
        self.lambdas.borrow_mut().truncate(count);
        let value = self.expected_value(expr, expected)?;
        Ok((value.clone(), value.ty))
    }

    pub(super) fn refinement_stable(&self, local: &Local) -> bool {
        !local.mutable || !self.changing_cells.contains(&local.slot)
    }

    pub(super) fn assigned_refinement(&mut self, slot: u8, actual: &Type) {
        self.refinements.remove(&slot.into());
        let local = self
            .scopes
            .iter()
            .rev()
            .flat_map(|scope| scope.values())
            .find(|local| local.slot == slot)
            .cloned();
        if let Some(local) = local
            && matches!(local.ty, Type::Nullable(ref inner) if inner.is_reference())
            && self.refinement_stable(&local)
            && actual.is_reference()
            && actual != &Type::Null
        {
            self.refinements.insert(slot.into(), actual.clone());
        }
    }

    pub(super) fn refined_local_read(&self, local: &Local, mut raw: Value) -> Value {
        if self.refinement_stable(local)
            && let Some(refined) = self.refinements.get(&local.slot.into())
        {
            if raw.ty.descriptor() == refined.descriptor() {
                raw.ty = refined.clone();
                return raw;
            }
            return Value {
                ty: refined.clone(),
                kind: ValueKind::Cast {
                    value: Box::new(raw),
                    target: refined.clone(),
                    safe: false,
                },
            };
        }
        raw
    }

    pub(super) fn invalidate_written_names(&mut self, names: HashSet<String>) {
        for name in names {
            if let Some(local) = self.local(&name)
                && local.mutable
            {
                self.refinements.remove(&local.slot.into());
            }
        }
    }

    pub(super) fn invalidate_statement_writes(&mut self, statement: &syntax::Statement) {
        let names = match &statement.kind {
            StatementKind::Assign(_, value) => captures::free_written_expression(value),
            _ => captures::free_written_names(std::slice::from_ref(statement), &[]),
        };
        if matches!(
            statement.kind,
            StatementKind::While(..)
                | StatementKind::DoWhile(..)
                | StatementKind::ForEach(..)
                | StatementKind::ForUntil(..)
        ) {
            for name in &names {
                if let Some(local) = self.local(name)
                    && local.mutable
                    && self.cells.contains_key(&local.slot)
                {
                    self.changing_cells.insert(local.slot);
                }
            }
        }
        self.invalidate_written_names(names);
    }

    pub(super) fn invalidate_expression_writes(&mut self, expression: &Expr) {
        self.invalidate_written_names(captures::free_written_expression(expression));
    }

    pub(super) fn mark_changing_captures(&mut self, names: HashSet<String>) {
        for name in names {
            if let Some(local) = self.local(&name)
                && local.mutable
            {
                self.changing_cells.insert(local.slot);
                self.refinements.remove(&local.slot.into());
            }
        }
    }
}
