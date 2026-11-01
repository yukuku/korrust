use crate::{Diagnostic, DiagnosticKind, syntax};
use std::collections::HashMap;

pub(crate) fn validate(
    units: &[(String, syntax::Module)],
    parents: &HashMap<String, String>,
) -> Result<(), (String, Diagnostic)> {
    for (owner, unit) in units {
        let Some(class) = unit.class_decl.as_ref().filter(|class| class.inner) else {
            continue;
        };
        let invalid = |message: &str| {
            (
                owner.clone(),
                Diagnostic::new(class.offset, message)
                    .with_kind(DiagnosticKind::InvalidKotlin, "INNER_CLASS_CONTEXT_INVALID"),
            )
        };
        let unsupported = |message: &str| {
            (
                owner.clone(),
                Diagnostic::new(class.offset, message).with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "INNER_CLASS_SHAPE_UNSUPPORTED",
                ),
            )
        };
        let Some(parent) = parents
            .get(owner)
            .and_then(|parent| units.iter().find(|(owner, _)| owner == parent))
            .map(|(_, unit)| unit)
        else {
            return Err(invalid("Inner classes require an enclosing class instance"));
        };
        if parent.object_decl.is_some() || parent.interface_decl.is_some() {
            return Err(invalid(
                "Inner classes cannot occur in objects or interfaces",
            ));
        }
        if class.data || class.sealed_ {
            return Err(invalid("Inner classes cannot be data or sealed classes"));
        }
        if class.companion.is_some() || class.nested.iter().any(|declaration| !matches!(declaration, syntax::Declaration::Class(class) if class.inner)) { return Err(invalid("Static nested declarations cannot occur inside inner classes")); }
        let Some(outer) = &parent.class_decl else {
            return Err(unsupported(
                "Inner classes inside enum declarations are unsupported",
            ));
        };
        let mut generated = outer
            .methods
            .iter()
            .filter(|method| method.function.private)
            .map(|method| format!("access${}", method.function.name))
            .collect::<Vec<_>>();
        for name in outer
            .param_properties
            .iter()
            .filter(|name| !outer.public_params.contains(name))
            .chain(
                outer
                    .properties
                    .iter()
                    .filter(|property| property.private)
                    .map(|property| &property.name),
            )
        {
            generated.push(format!("access$get${name}$p"));
            generated.push(format!("access$set${name}$p"));
        }
        if outer
            .methods
            .iter()
            .any(|method| generated.contains(&method.function.name))
        {
            return Err(unsupported(
                "Outer member names conflict with generated lexical access bridges",
            ));
        }
        if class
            .param_annotations
            .iter()
            .any(|annotations| !annotations.is_empty())
            || outer.inner
            || !outer.type_params.is_empty()
            || !class.type_params.is_empty()
            || !class.nested.is_empty()
            || class.companion.is_some()
            || !class.secondary_constructors.is_empty()
            || class.defaults.iter().any(Option::is_some)
            || class.abstract_
            || class.open
        {
            return Err(unsupported(
                "Generic, chained, open, abstract, annotated, defaulted and secondary inner constructors are unsupported",
            ));
        }
        if class.params.iter().any(|(name, _)| name == "this$0")
            || class
                .properties
                .iter()
                .any(|property| property.name == "this$0")
        {
            return Err(unsupported(
                "An inner member conflicts with the enclosing instance field",
            ));
        }
    }
    Ok(())
}
