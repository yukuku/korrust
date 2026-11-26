use super::*;
use std::collections::{HashMap, HashSet};

impl Classpath {
    pub(super) fn scope_parent(&self, owner: &str) -> Option<String> {
        self.lexical_parents
            .borrow()
            .get(owner)
            .cloned()
            .or_else(|| owner.rsplit_once('$').map(|(outer, _)| outer.into()))
    }

    fn inheritance_parent(&self, class: &JavaClass) -> Result<Option<String>> {
        let Some(parent) = &class.super_name else {
            return Ok(None);
        };
        if self
            .load(parent)
            .is_err_and(|error| error.code == "CLASS_NOT_FOUND")
        {
            if let Some(ty) = class.library.as_ref().and_then(|library| library.supertypes.iter().find(|ty| matches!(ty.base(), Type::Java(name) | Type::Parameterized { name, .. } if name == parent))) {
                let expanded = self.expand_type_alias(ty, &HashMap::new(), DEFAULT_KOTLIN_PACKAGES)?;
                if let Type::Java(name) | Type::Parameterized { name, .. } = expanded.base() {
                    return Ok(Some(name.clone()));
                }
            }
        }
        Ok(Some(parent.clone()))
    }

    pub(crate) fn inheritance_scope(&self, caller: &str, declaring: &str) -> bool {
        let mut scope = Some(caller.to_owned());
        for _ in 0..64 {
            let Some(owner) = scope else { return false };
            let mut current = Some(owner.clone());
            let mut visited = HashSet::new();
            for _ in 0..64 {
                let Some(name) = current else { break };
                if name == declaring {
                    return true;
                }
                if !visited.insert(name.clone()) {
                    break;
                }
                let Ok(class) = self.load(&name) else { break };
                current = self.inheritance_parent(&class).ok().flatten();
            }
            scope = self.scope_parent(&owner);
        }
        false
    }

    pub(crate) fn type_parameter_in_scope(&self, caller: &str, name: &str) -> bool {
        let mut scope = Some(caller.to_owned());
        for _ in 0..64 {
            let Some(owner) = scope else { return false };
            if let Ok(class) = self.load(&owner) {
                if class.library.as_ref().is_some_and(|library| {
                    library
                        .type_parameters
                        .iter()
                        .any(|parameter| parameter.name == name)
                }) {
                    return true;
                }
                if class.enclosing_instance().is_none() {
                    return false;
                }
            }
            scope = self.scope_parent(&owner);
        }
        false
    }

    pub(crate) fn member_classifier_visible(
        &self,
        caller: &str,
        class: &JavaClass,
        declaring: &str,
        access: u16,
    ) -> bool {
        if access & 2 != 0 && !self.lexically_within(caller, declaring)
            || !self.classifier_visible(caller, &class.name)
        {
            return false;
        }
        if class.kotlin_class {
            return class.library.as_ref().is_none_or(|library| {
                library.kotlin_visibility != Some(2) || self.inheritance_scope(caller, declaring)
            });
        }
        self.check_java_import_visibility(caller, &class.name, 0)
            .is_ok()
    }

    pub(crate) fn inherited_member_class(
        &self,
        caller: &str,
        scope: &str,
        name: &str,
    ) -> Result<Option<JavaClass>> {
        if self.is_source_facade(scope) {
            return Ok(None);
        }
        let Ok(class) = self.load(scope) else {
            return Ok(None);
        };
        if class.name.rsplit(['/', '$']).next() == Some(name)
            || class.library.as_ref().is_some_and(|library| {
                library
                    .type_parameters
                    .iter()
                    .any(|parameter| parameter.name == name)
            })
        {
            return Ok(None);
        }
        let mut current = self.inheritance_parent(&class)?;
        let mut visited = HashSet::new();
        for _ in 0..64 {
            let Some(owner) = current else {
                return Ok(None);
            };
            if !visited.insert(owner.clone()) {
                return Err(Diagnostic::new(0, "Cyclic classifier inheritance")
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "CYCLIC_INHERITANCE"));
            }
            let class = self.load(&owner)?;
            if let Some((candidate, access)) = self.member_class_entry(&owner, name)?
                && access & 2 == 0
            {
                let candidate = self.load(&candidate)?;
                if self.member_classifier_visible(caller, &candidate, &owner, access) {
                    return Ok(Some(candidate));
                }
            }
            current = self.inheritance_parent(&class)?;
        }
        Err(Diagnostic::new(
            0,
            "Inherited classifier lookup exceeds the supported hierarchy depth",
        )
        .with_kind(
            crate::DiagnosticKind::UnsupportedLanguage,
            "INHERITED_CLASSIFIER_DEPTH_UNSUPPORTED",
        ))
    }

    pub(crate) fn scoped_member_classifier(
        &self,
        caller: &str,
        name: &str,
    ) -> Result<Option<JavaClass>> {
        if self.type_parameter_in_scope(caller, name) {
            return Ok(None);
        }
        let mut scope = Some(caller.to_owned());
        for _ in 0..64 {
            let Some(owner) = scope else { return Ok(None) };
            if let Ok(class) = self.load(&owner) {
                if class.name.rsplit(['/', '$']).next() == Some(name) {
                    return Ok(None);
                }
                if let Some((candidate, access)) = self.member_class_entry(&owner, name)? {
                    let candidate = self.load(&candidate)?;
                    if self.member_classifier_visible(caller, &candidate, &owner, access) {
                        return Ok(Some(candidate));
                    }
                }
                if let Some(candidate) = self.inherited_member_class(caller, &owner, name)? {
                    return Ok(Some(candidate));
                }
            }
            scope = self.scope_parent(&owner);
        }
        Err(Diagnostic::new(
            0,
            "Classifier lexical lookup exceeds the supported hierarchy depth",
        ))
    }
}
