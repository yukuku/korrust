use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Subject {
    Local(u8),
    Property(Box<Subject>, String, String),
}
impl From<u8> for Subject {
    fn from(slot: u8) -> Self {
        Self::Local(slot)
    }
}

impl Checker<'_> {
    fn stable_property_subject(&self, expression: &Expr) -> Option<Subject> {
        self.stable_property_path(expression)
            .map(|(subject, _)| subject)
    }

    fn stable_property_path(&self, expression: &Expr) -> Option<(Subject, Type)> {
        match &expression.kind {
            ExprKind::Name(name) => {
                let local = self.local(name)?;
                (!local.mutable && !self.cells.contains_key(&local.slot)).then(|| {
                    let subject = Subject::Local(local.slot);
                    let ty = self.refinements.get(&subject).cloned().unwrap_or(local.ty);
                    (subject, ty)
                })
            }
            ExprKind::Property(receiver, name) => {
                let (subject, receiver_type) = self.stable_property_path(receiver)?;
                if receiver_type.is_nullable() {
                    return None;
                }
                let owner = library::type_owner(&receiver_type)?;
                let methods = self.classpath.methods(owner).ok()?;
                let getters = methods
                    .iter()
                    .filter(|method| {
                        method.source_property.as_deref() == Some(name)
                            && method.kotlin_params.as_ref().is_some_and(Vec::is_empty)
                            && method
                                .library
                                .as_ref()
                                .is_none_or(|lib| lib.receiver.is_none())
                            && method.instance_dispatch()
                            && !method.unsupported
                    })
                    .collect::<Vec<_>>();
                let [method] = getters.as_slice() else {
                    return None;
                };
                if !self.classpath.internal_visible(&method.owner) {
                    return None;
                }
                let class = self.classpath.load(&method.owner).ok()?;
                if method.access & 0x10 == 0 && class.access & 0x10 == 0 {
                    return None;
                }
                if !class.fields.iter().any(|field| {
                    field
                        .kotlin
                        .as_ref()
                        .is_some_and(|property| property.name == *name && property.stable)
                }) {
                    return None;
                }
                let receiver_class = self.classpath.load(owner).ok()?;
                let specialized = library::specialize_receiver(
                    (*method).clone(),
                    &receiver_type,
                    &receiver_class,
                    self.classpath,
                );
                let (_, ty) = class_method_types(&specialized).ok()?;
                let subject =
                    Subject::Property(Box::new(subject), method.owner.clone(), method.name.clone());
                let ty = self.refinements.get(&subject).cloned().unwrap_or(ty);
                Some((subject, ty))
            }
            _ => None,
        }
    }

    pub(super) fn refine_stable_property(&mut self, condition: &Expr, truth: bool) {
        let ExprKind::Binary(operator, left, right) = &condition.kind else {
            return;
        };
        if !((matches!(operator.as_str(), "!=" | "!==") && truth)
            || (matches!(operator.as_str(), "==" | "===") && !truth))
        {
            return;
        }
        let value = match (&left.kind, &right.kind) {
            (ExprKind::Property(..), ExprKind::Null) => left,
            (ExprKind::Null, ExprKind::Property(..)) => right,
            _ => return,
        };
        let Some(subject) = self.stable_property_subject(value) else {
            return;
        };
        if let Ok(Value {
            ty: Type::Nullable(inner),
            ..
        }) = self.value(value)
        {
            self.refinements.insert(subject, *inner);
        }
    }

    pub(super) fn refined_property_value(
        &mut self,
        expression: &Expr,
        value: Value,
    ) -> Result<Value> {
        if !matches!(expression.kind, ExprKind::Property(..))
            || !self
                .refinements
                .keys()
                .any(|subject| matches!(subject, Subject::Property(..)))
        {
            return Ok(value);
        }
        if let Some(subject) = self.stable_property_subject(expression)
            && let Some(ty) = self.refinements.get(&subject)
        {
            return Ok(Value {
                ty: ty.clone(),
                kind: ValueKind::Cast {
                    value: Box::new(value),
                    target: ty.clone(),
                    safe: false,
                },
            });
        }
        Ok(value)
    }
}
