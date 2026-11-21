use super::*;
use crate::classpath::ContractReturn;

#[derive(Clone)]
pub(super) struct Fact {
    returned: ContractReturn,
    slot: u8,
    ty: Type,
    boolean_result: bool,
}

impl Checker<'_> {
    fn stable_contract_local(&self, value: &Value, lets: &[(u8, Value)]) -> Option<(u8, Type)> {
        match &value.kind {
            ValueKind::Local(slot) => {
                if let Some((_, value)) = lets.iter().find(|(saved, _)| saved == slot) {
                    return self.stable_contract_local(value, &[]);
                }
                self.scopes
                    .iter()
                    .rev()
                    .flat_map(|scope| scope.values())
                    .find_map(|local| {
                        (local.slot == *slot && !local.mutable && !self.cells.contains_key(slot))
                            .then(|| (*slot, local.ty.clone()))
                    })
            }
            ValueKind::Cast {
                value, safe: false, ..
            }
            | ValueKind::Box(value) => self.stable_contract_local(value, lets),
            _ => None,
        }
    }

    pub(super) fn call_contract_facts(
        &self,
        method: &JavaMethod,
        values: &[Value],
        lets: &[(u8, Value)],
    ) -> Vec<Fact> {
        let boolean_result = match method.kotlin_result.as_ref() {
            Some(Type::Boolean) => true,
            Some(Type::Nullable(inner)) if inner.as_ref() == &Type::Boolean => false,
            _ => return Vec::new(),
        };
        let mut facts = Vec::new();
        if let Some(library) = &method.library {
            for effect in &library.contracts {
                for (parameter, target) in &effect.requirements {
                    let index = if library.receiver.is_some() {
                        *parameter
                    } else if let Some(index) = parameter.checked_sub(1) {
                        index
                    } else {
                        continue;
                    };
                    let Some((slot, original)) = values
                        .get(index)
                        .and_then(|value| self.stable_contract_local(value, lets))
                    else {
                        continue;
                    };
                    let ty = target.clone().unwrap_or_else(|| match original.base() {
                        Type::Nullable(inner) => inner.as_ref().clone(),
                        ty => ty.clone(),
                    });
                    if !ty.is_nullable() && ty != Type::Null {
                        facts.push(Fact {
                            returned: effect.returned,
                            slot,
                            ty,
                            boolean_result,
                        });
                    }
                }
            }
        }
        facts
    }

    pub(super) fn remap_contract_receiver(
        &mut self,
        offset: usize,
        receiver_slot: u8,
        receiver: &Value,
    ) {
        let original = self.stable_contract_local(receiver, &[]);
        if let Some(facts) = self.contract_facts.get_mut(&offset) {
            facts.retain_mut(|fact| {
                if fact.slot != receiver_slot {
                    return true;
                }
                if let Some((original, _)) = &original {
                    fact.slot = *original;
                    true
                } else {
                    false
                }
            });
        }
    }

    pub(super) fn refine_contract(&mut self, condition: &Expr, truth: bool) {
        let (call, returned, exact) = match &condition.kind {
            ExprKind::Call(..) => (condition, ContractReturn::Boolean(truth), false),
            ExprKind::Binary(operator, left, right)
                if matches!(operator.as_str(), "==" | "!=" | "===" | "!==") =>
            {
                let (call, constant) = match (&left.kind, &right.kind) {
                    (ExprKind::Call(..), _) => (left.as_ref(), &right.kind),
                    (_, ExprKind::Call(..)) => (right.as_ref(), &left.kind),
                    _ => return,
                };
                let equal = truth == matches!(operator.as_str(), "==" | "===");
                let returned = match constant {
                    ExprKind::Boolean(value) => ContractReturn::Boolean(*value == equal),
                    _ => return,
                };
                (call, returned, equal)
            }
            _ => return,
        };
        if let Some(facts) = self.contract_facts.get(&call.offset) {
            for fact in facts {
                if fact.returned == returned && (exact || fact.boolean_result) {
                    match self.refinements.get(&fact.slot.into()) {
                        Some(previous) if self.subtype(previous, &fact.ty) => {}
                        _ => {
                            self.refinements.insert(fact.slot.into(), fact.ty.clone());
                        }
                    }
                }
            }
        }
    }
}
