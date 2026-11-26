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
                if !self.classpath.stable_kotlin_getter(method, name) {
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

    fn implicit_getter_subject(&self, expression: &Expr, value: &Value) -> Option<Subject> {
        let ExprKind::Name(name) = &expression.kind else {
            return None;
        };
        let (method, slot) = match &value.kind {
            ValueKind::JavaCall { method, args } | ValueKind::MemberCall { method, args, .. } => {
                let [
                    Value {
                        kind: ValueKind::Local(slot),
                        ..
                    },
                ] = args.as_slice()
                else {
                    return None;
                };
                (method.clone(), *slot)
            }
            ValueKind::Field(field, receiver) if field == name => {
                let ValueKind::Local(slot) = receiver.kind else {
                    return None;
                };
                let class = self
                    .classpath
                    .load(library::type_owner(&receiver.ty)?)
                    .ok()?;
                let getter = class.methods.into_iter().find(|m| {
                    m.source_property.as_ref() == Some(name)
                        && m.name == getter_name(name)
                        && m.descriptor.starts_with("()")
                })?;
                (getter, slot)
            }
            _ => return None,
        };
        if method.source_property.as_ref() != Some(name)
            || !method.instance_dispatch()
            || !self.classpath.stable_kotlin_getter(&method, name)
        {
            return None;
        }
        Some(Subject::Property(
            Box::new(Subject::Local(slot)),
            method.owner.clone(),
            method.name.clone(),
        ))
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
            (ExprKind::Property(..) | ExprKind::Name(_), ExprKind::Null) => left,
            (ExprKind::Null, ExprKind::Property(..) | ExprKind::Name(_)) => right,
            _ => return,
        };
        let explicit = match &value.kind {
            ExprKind::Property(..) => {
                let Some(subject) = self.stable_property_subject(value) else {
                    return;
                };
                Some(subject)
            }
            ExprKind::Name(name) if self.local(name).is_none() => None,
            _ => return,
        };
        if let Ok(checked) = self.value(value)
            && let Some(subject) =
                explicit.or_else(|| self.implicit_getter_subject(value, &checked))
            && let Type::Nullable(inner) = checked.ty
        {
            self.refinements.insert(subject, *inner);
        }
    }

    pub(super) fn refined_property_value(
        &mut self,
        expression: &Expr,
        value: Value,
    ) -> Result<Value> {
        if !matches!(expression.kind, ExprKind::Property(..) | ExprKind::Name(_))
            || !self
                .refinements
                .keys()
                .any(|subject| matches!(subject, Subject::Property(..)))
        {
            return Ok(value);
        }
        let subject = match &expression.kind {
            ExprKind::Name(_) => self.implicit_getter_subject(expression, &value),
            _ => self.stable_property_subject(expression),
        };
        if let Some(ty) = subject.and_then(|key| self.refinements.get(&key)) {
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
