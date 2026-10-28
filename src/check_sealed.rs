use super::*;

impl Checker<'_> {
    pub(super) fn closed_when_type(&self, ty: &Type) -> bool {
        let ty = match ty.base() {
            Type::Nullable(inner) => inner.as_ref(),
            ty => ty,
        };
        ty == &Type::Boolean
            || library::type_owner(ty)
                .and_then(|owner| self.classpath.load(owner).ok())
                .is_some_and(|class| class.sealed_ || class.access & 0x4000 != 0)
    }

    pub(super) fn unmatched_when(&self, ty: Type, offset: usize) -> Result<Value> {
        self.classpath
            .load("kotlin/NoWhenBranchMatchedException")
            .map_err(|mut error| {
                error.offset = offset;
                error
            })?;
        Ok(Value {
            ty,
            kind: ValueKind::Abrupt(vec![Statement::ThrowValue(Value {
                ty: Type::Java("kotlin/NoWhenBranchMatchedException".into()),
                kind: ValueKind::New {
                    owner: "kotlin/NoWhenBranchMatchedException".into(),
                    descriptor: "()V".into(),
                    args: Vec::new(),
                },
            })]),
        })
    }

    pub(super) fn require_when_exhaustive(
        &mut self,
        ty: &Type,
        conditions: &[&Expr],
        offset: usize,
    ) -> Result<()> {
        let nullable = ty.is_nullable();
        let base = match ty.base() {
            Type::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        let mut total_type_test = false;
        let mut total_null = !nullable;
        for condition in conditions {
            if matches!(condition.kind, ExprKind::Null) {
                total_null = true;
            }
            if let ExprKind::TypeOperation(op, _, target) = &condition.kind {
                if op != "is" || matches!(target, Type::Parameterized { .. }) {
                    continue;
                }
                let target = self.resolve(target, condition.offset)?;
                let inner = match &target {
                    Type::Nullable(inner) => inner.as_ref(),
                    ty => ty,
                };
                if self.subtype(base, inner) {
                    total_type_test = true;
                    total_null |= target.is_nullable();
                }
            }
        }
        if total_type_test && total_null {
            return Ok(());
        }
        let mut leaves = Vec::new();
        let mut pending = Vec::new();
        if base == &Type::Boolean {
            leaves.extend([("<false>".into(), None), ("<true>".into(), None)]);
        } else if let Some(owner) = library::type_owner(base) {
            let class = self.classpath.load(owner)?;
            if !class.sealed_ && class.access & 0x4000 == 0 {
                return Err(Diagnostic::new(
                    offset,
                    "When expressions require an else branch or a supported closed subject type",
                )
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_EXHAUSTIVE_WHEN"));
            }
            pending.push((owner.to_owned(), Vec::<String>::new()));
        } else {
            return Err(Diagnostic::new(
                offset,
                "When expressions require an else branch or a supported closed subject type",
            )
            .with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_EXHAUSTIVE_WHEN"));
        }
        let mut visited = HashSet::new();
        while let Some((owner, ancestors)) = pending.pop() {
            if ancestors.len() > 64 || visited.len() > 256 {
                return Err(Diagnostic::new(
                    offset,
                    "Sealed hierarchy exceeds the supported traversal limit",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "SEALED_HIERARCHY_LIMIT_UNSUPPORTED",
                ));
            }
            if ancestors.contains(&owner) {
                return Err(
                    Diagnostic::new(offset, "Malformed or excessive sealed hierarchy").with_kind(
                        crate::DiagnosticKind::MissingDependency,
                        "INVALID_SEALED_HIERARCHY",
                    ),
                );
            }
            if !visited.insert(owner.clone()) {
                continue;
            }
            let class = self.classpath.load(&owner)?;
            if class
                .library
                .as_ref()
                .is_some_and(|l| !l.type_parameters.is_empty())
            {
                return Err(Diagnostic::new(
                    offset,
                    "Exhaustiveness over generic sealed hierarchies is unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "SEALED_GENERIC_EXHAUSTIVENESS_UNSUPPORTED",
                ));
            }
            if class.sealed_ {
                if class.sealed_subclasses.is_empty() {
                    return Err(Diagnostic::new(
                        offset,
                        "Exhaustiveness over uninhabited sealed hierarchies is unsupported",
                    )
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_EXHAUSTIVE_WHEN"));
                }
                let mut ancestors = ancestors;
                ancestors.push(owner.clone());
                for child in class.sealed_subclasses {
                    let declared = self.classpath.load(&child)?;
                    if !declared
                        .super_name
                        .iter()
                        .chain(&declared.interfaces)
                        .any(|p| p == &owner)
                    {
                        return Err(Diagnostic::new(
                            offset,
                            "Sealed metadata names a non-direct subclass",
                        )
                        .with_kind(
                            crate::DiagnosticKind::MissingDependency,
                            "INVALID_SEALED_HIERARCHY",
                        ));
                    }
                    pending.push((child, ancestors.clone()));
                }
            } else if class.access & 0x4000 != 0 {
                for field in class.fields.into_iter().filter(|f| f.access & 0x4000 != 0) {
                    leaves.push((
                        format!("{owner}::{}", field.name),
                        Some(Type::Java(owner.clone())),
                    ));
                }
            } else {
                leaves.push((owner.clone(), Some(Type::Java(owner))));
            }
        }
        let mut covered = HashSet::new();
        let mut null_covered = !nullable;
        for condition in conditions {
            if let ExprKind::TypeOperation(op, _, target) = &condition.kind {
                if matches!(target, Type::Parameterized { .. }) {
                    return Err(Diagnostic::new(
                        condition.offset,
                        "Parameterized exhaustive when type tests are unsupported",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "SEALED_GENERIC_EXHAUSTIVENESS_UNSUPPORTED",
                    ));
                }
                let target = self.resolve(target, condition.offset)?;
                let inner = match &target {
                    Type::Nullable(t) => t.as_ref(),
                    ty => ty,
                };
                if !self.subtype(inner, base) && !self.subtype(base, inner) {
                    return Err(Diagnostic::new(
                        condition.offset,
                        "Exhaustive type tests outside the subject hierarchy are unsupported",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "WHEN_TYPE_INTERSECTION_UNSUPPORTED",
                    ));
                }
                let present = leaves
                    .iter()
                    .any(|(_, leaf)| leaf.as_ref().is_some_and(|leaf| self.subtype(leaf, inner)));
                for (key, leaf) in &leaves {
                    let Some(leaf) = leaf else {
                        continue;
                    };
                    if op == "is" && self.subtype(leaf, inner)
                        || op == "!is" && present && !self.subtype(leaf, inner)
                    {
                        covered.insert(key.clone());
                    }
                }
                if nullable
                    && (op == "is" && target.is_nullable() || op == "!is" && !target.is_nullable())
                {
                    null_covered = true;
                }
            } else if matches!(condition.kind, ExprKind::Null) {
                null_covered = true;
            } else if matches!(
                condition.kind,
                ExprKind::Boolean(_) | ExprKind::Name(_) | ExprKind::Property(..)
            ) {
                let value = self.value(condition)?;
                let key = match &value.kind {
                    ValueKind::Boolean(value) => Some(format!("<{value}>")),
                    ValueKind::ObjectInstance(owner) => Some(owner.clone()),
                    ValueKind::JavaField {
                        owner,
                        name,
                        receiver: None,
                        ..
                    } => Some(format!("{owner}::{name}")),
                    _ => None,
                };
                if let Some(key) = key {
                    if leaves.iter().any(|(leaf, _)| leaf == &key) {
                        covered.insert(key);
                    }
                }
            }
        }
        if !null_covered || leaves.iter().any(|(key, _)| !covered.contains(key)) {
            if self.exhaustiveness_flow {
                return Err(Diagnostic::new(
                    offset,
                    "Flow-sensitive when exhaustiveness is unsupported",
                )
                .with_kind(
                    crate::DiagnosticKind::UnsupportedLanguage,
                    "WHEN_FLOW_EXHAUSTIVENESS_UNSUPPORTED",
                ));
            }
            return Err(Diagnostic::new(
                offset,
                "Non-exhaustive when: an else branch or all subject cases are required",
            )
            .with_kind(crate::DiagnosticKind::InvalidKotlin, "NON_EXHAUSTIVE_WHEN"));
        }
        Ok(())
    }
}
