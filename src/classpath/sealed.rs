use super::*;
use std::collections::{HashMap, HashSet};

pub(super) fn validate(
    classpath: &Classpath,
    owners: &HashSet<String>,
) -> std::result::Result<(), (String, Diagnostic)> {
    let mut classes = Vec::new();
    for owner in owners {
        classes.push(
            classpath
                .load(owner)
                .map_err(|error| (owner.clone(), error))?,
        );
    }
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for class in &classes {
        for owner in class.super_name.iter().chain(&class.interfaces) {
            let parent = classpath
                .load(owner)
                .map_err(|error| (class.name.clone(), error))?;
            if !parent.sealed_ {
                continue;
            }
            if !owners.contains(owner)
                || owner.rsplit_once('/').map(|p| p.0) != class.name.rsplit_once('/').map(|p| p.0)
            {
                return Err((
                    class.name.clone(),
                    Diagnostic::new(
                        0,
                        "Sealed subclasses must be declared in the same package and source module",
                    )
                    .with_kind(
                        crate::DiagnosticKind::InvalidKotlin,
                        "SEALED_INHERITANCE_INVALID",
                    ),
                ));
            }
            children
                .entry(owner.clone())
                .or_default()
                .push(class.name.clone());
        }
    }
    for mut class in classes {
        if class.sealed_ {
            class.sealed_subclasses = children.remove(&class.name).unwrap_or_default();
            class.sealed_subclasses.sort();
            class.sealed_subclasses.dedup();
            if class.sealed_subclasses.len() > 256 {
                return Err((
                    class.name,
                    Diagnostic::new(
                        0,
                        "Sealed hierarchies exceed the current direct-subclass limit",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "SEALED_HIERARCHY_LIMIT_UNSUPPORTED",
                    ),
                ));
            }
            classpath.register_source(class);
        }
    }
    Ok(())
}
