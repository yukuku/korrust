use super::*;

#[derive(Clone)]
pub(super) struct ValueScope {
    package: String,
    imports: Vec<(String, String, usize)>,
}

#[derive(Clone)]
pub(crate) enum ValueSymbol {
    Classifier(Box<JavaClass>),
    Field(JavaField),
    Property(Vec<JavaMethod>),
}

impl Classpath {
    pub(crate) fn register_value_scope(&self, owner: &str, module: &crate::syntax::Module) {
        self.value_scopes
            .borrow_mut()
            .entry(owner.into())
            .or_insert_with(|| ValueScope {
                package: module.package.replace('.', "/"),
                imports: module.imports.clone(),
            });
    }

    pub(crate) fn kotlin_field_visible(&self, caller: &str, field: &JavaField) -> bool {
        match field.kotlin.as_ref().map(|field| field.visibility) {
            Some(0) => self.internal_visible(&field.owner),
            Some(1 | 4) => self.same_source_file(caller, &field.owner),
            Some(3) | None => field.access & 1 != 0,
            _ => false,
        }
    }

    pub(crate) fn value_symbol(
        &self,
        caller: &str,
        name: &str,
        offset: usize,
    ) -> Result<Option<ValueSymbol>> {
        let scopes = self.value_scopes.borrow();
        let mut owner = caller;
        let scope = loop {
            if let Some(scope) = scopes.get(owner) {
                break scope.clone();
            }
            let Some((parent, _)) = owner.rsplit_once('$') else {
                break ValueScope {
                    package: caller
                        .rsplit_once('/')
                        .map_or("", |(package, _)| package)
                        .into(),
                    imports: Vec::new(),
                };
            };
            owner = parent;
        };
        drop(scopes);
        let qualified = |package: &str, name: &str| {
            if package.is_empty() {
                name.into()
            } else {
                format!("{package}/{name}")
            }
        };
        let explicit = scope
            .imports
            .iter()
            .filter(|(alias, _, _)| alias == name)
            .map(|(_, path, _)| path.clone())
            .collect::<Vec<_>>();
        let renamed = |path: &String| {
            scope
                .imports
                .iter()
                .any(|(alias, imported, _)| imported == path && alias != name && alias != "*")
                && !scope
                    .imports
                    .iter()
                    .any(|(alias, imported, _)| imported == path && alias == name)
        };
        let mut groups = vec![explicit, vec![qualified(&scope.package, name)]];
        groups.push(
            scope
                .imports
                .iter()
                .filter(|(alias, _, _)| alias == "*")
                .map(|(_, path, _)| qualified(path.trim_end_matches("/*"), name))
                .collect(),
        );
        groups.push(
            DEFAULT_KOTLIN_PACKAGES
                .iter()
                .copied()
                .chain(["java/lang"])
                .map(|package| qualified(package, name))
                .collect(),
        );
        let mut inaccessible = false;
        for (priority, paths) in groups.into_iter().enumerate() {
            let explicit = priority == 0 && !paths.is_empty();
            let mut symbols = std::collections::BTreeMap::new();
            for path in paths {
                if priority != 0 && renamed(&path) {
                    continue;
                }
                if let Ok(class) = self.load(&path) {
                    let visibility = class
                        .library
                        .as_ref()
                        .and_then(|library| library.kotlin_visibility);
                    let internal_visible =
                        visibility != Some(0) || self.internal_visible(&class.name);
                    if visibility == Some(0) && internal_visible && class.unsupported {
                        return Err(Diagnostic::new(offset, "Compiled internal classifier values require unsupported class metadata")
                            .with_kind(crate::DiagnosticKind::UnsupportedLanguage, "INTERNAL_CLASSIFIER_VALUE_UNSUPPORTED"));
                    }
                    if class.access & 1 == 0
                        || !internal_visible
                        || matches!(visibility, Some(1 | 2 | 4 | 5))
                    {
                        inaccessible = true;
                    }
                    if class.access & 1 != 0
                        && !class.unsupported
                        && internal_visible
                        && !matches!(visibility, Some(1 | 2 | 4 | 5))
                    {
                        symbols.insert(
                            format!("class:{}", class.name),
                            ValueSymbol::Classifier(Box::new(class)),
                        );
                    }
                }
                let (package, member) = path.rsplit_once('/').unwrap_or(("", &path));
                if priority == 0 {
                    if let Ok(class) = self.load(package) {
                        if class.access & 1 != 0 && !class.unsupported {
                            for field in self.field_candidates(package, member)? {
                                if field.access & 8 != 0
                                    && !field.unsupported
                                    && self.kotlin_field_visible(caller, &field)
                                {
                                    symbols.insert(
                                        format!("field:{}:{}", field.owner, field.name),
                                        ValueSymbol::Field(field),
                                    );
                                }
                            }
                        }
                    }
                }
                for field in self.package_constants(package, member)? {
                    if !self.kotlin_field_visible(caller, &field) {
                        inaccessible = true;
                    }
                    if !field.unsupported && self.kotlin_field_visible(caller, &field) {
                        symbols.insert(
                            format!("field:{}:{}", field.owner, field.name),
                            ValueSymbol::Field(field),
                        );
                    }
                }
                let methods = self
                    .package_methods(package, member)?
                    .into_iter()
                    .filter(|method| {
                        method.source_property.as_deref() == Some(member)
                            && method.kotlin_params.as_ref().is_some_and(Vec::is_empty)
                            && method
                                .library
                                .as_ref()
                                .is_some_and(|library| library.receiver.is_none())
                            && !method.unsupported
                            && method.access & 8 != 0
                            && (!method.kotlin_internal || self.internal_visible(&method.owner))
                            && (method.access & 1 != 0 || self.file_private_visible(caller, method))
                    })
                    .collect::<Vec<_>>();
                for method in methods {
                    symbols.insert(
                        format!(
                            "property:{}:{}{}",
                            method.owner, method.name, method.descriptor
                        ),
                        ValueSymbol::Property(vec![method]),
                    );
                }
            }
            match symbols.len() {
                0 if explicit && inaccessible => {
                    return Err(Diagnostic::new(
                        offset,
                        "Imported value declaration is inaccessible",
                    )
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "INVISIBLE_REFERENCE"));
                }
                0 if explicit => return Ok(None),
                0 => {}
                1 => return Ok(symbols.into_values().next()),
                _ => {
                    return Err(Diagnostic::new(offset, "Ambiguous imported value name")
                        .with_kind(crate::DiagnosticKind::InvalidKotlin, "AMBIGUOUS_VALUE_NAME"));
                }
            }
        }
        if inaccessible {
            return Err(Diagnostic::new(offset, "Value declaration is inaccessible")
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "INVISIBLE_REFERENCE"));
        }
        Ok(None)
    }
}
