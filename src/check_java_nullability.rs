use super::*;

impl Checker<'_> {
    fn java_nonnull_message(&self, value: &Value) -> Option<String> {
        if !value.ty.is_reference()
            || value.ty.is_nullable()
            || matches!(value.ty, Type::Platform(_))
        {
            return None;
        }
        match &value.kind {
            ValueKind::JavaCall { method, .. } | ValueKind::MemberCall { method, .. }
                if method
                    .library
                    .as_ref()
                    .is_some_and(|library| !library.kotlin_declared) =>
            {
                Some(format!("{}(...)", method.name))
            }
            ValueKind::JavaField { owner, name, .. } => self
                .classpath
                .field_candidates(owner, name)
                .ok()?
                .iter()
                .find(|field| {
                    field.kotlin.is_none()
                        && field.java_type.as_deref().is_some_and(|ty| {
                            ty.is_reference()
                                && !ty.is_nullable()
                                && !matches!(ty, Type::Platform(_))
                        })
                })
                .map(|_| name.clone()),
            ValueKind::Let { body, .. } => self.java_nonnull_message(body),
            _ => None,
        }
    }

    pub(super) fn enhanced_java_nonnull(&self, value: Value) -> Value {
        let Some(message) = self.java_nonnull_message(&value) else {
            return value;
        };
        Value {
            ty: value.ty.clone(),
            kind: ValueKind::NotNull(Box::new(value), message),
        }
    }
}
