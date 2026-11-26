use super::*;

#[derive(Clone)]
pub(super) struct ValueScope {
    package: String,
    imports: Vec<(String, String, usize)>,
    lexical_values: Vec<Vec<(String, String)>>,
}

#[derive(Clone)]
pub(crate) enum ValueSymbol {
    Classifier(Box<JavaClass>),
    Field(JavaField),
    Property(Vec<JavaMethod>),
}

impl Classpath {
    pub(crate) fn register_value_scope(
        &self,
        owner: &str,
        module: &crate::syntax::Module,
        lexical_values: Vec<Vec<(String, String)>>,
    ) {
        self.value_scopes
            .borrow_mut()
            .entry(owner.into())
            .or_insert_with(|| ValueScope {
                package: module.package.replace('.', "/"),
                imports: module.imports.clone(),
                lexical_values,
            });
    }

    pub(crate) fn check_companion_import_visibility(
        &self,
        owner: &str,
        offset: usize,
    ) -> Result<()> {
        if let Ok(class) = self.load(owner)
            && class
                .library
                .as_ref()
                .is_some_and(|library| matches!(library.kotlin_visibility, Some(1 | 4)))
            && let Some((outer, name)) = class.name.rsplit_once('$')
            && self.load(outer).is_ok_and(|outer| {
                outer
                    .library
                    .as_ref()
                    .and_then(|library| library.companion_name.as_deref())
                    == Some(name)
            })
        {
            self.check_classifier_visibility("<imports>", &class.name, offset)?;
        }
        Ok(())
    }

    pub(crate) fn package_field_visible(&self, caller: &str, field: &JavaField) -> bool {
        field.kotlin.is_none()
            && self.java_package_member_visible(caller, &field.owner, field.access)
    }
    pub(crate) fn java_package_member_visible(
        &self,
        caller: &str,
        owner: &str,
        access: u16,
    ) -> bool {
        access & 7 == 0
            && caller.rsplit_once('/').map(|(p, _)| p) == owner.rsplit_once('/').map(|(p, _)| p)
            && self.load(owner).is_ok_and(|class| !class.kotlin_class)
    }

    pub(crate) fn kotlin_field_visible(&self, caller: &str, field: &JavaField) -> bool {
        if field.kotlin.is_some()
            && let Ok(owner) = self.load(&field.owner)
            && let Some(name) = owner
                .library
                .as_ref()
                .and_then(|library| library.companion_name.as_ref())
            && let Ok(companion) = self.load(&format!("{}${name}", owner.name))
            && companion
                .fields
                .iter()
                .any(|candidate| candidate.name == field.name && candidate.kotlin.is_some())
            && !self.classifier_visible(caller, &companion.name)
        {
            return false;
        }
        match field.kotlin.as_ref().map(|field| field.visibility) {
            Some(0) => self.internal_visible(&field.owner),
            Some(1 | 4) => self.same_source_file(caller, &field.owner),
            Some(3) => field.access & 1 != 0,
            None => field.access & 1 != 0 || self.package_field_visible(caller, field),
            _ => false,
        }
    }

    pub(crate) fn value_symbol(
        &self,
        caller: &str,
        name: &str,
        offset: usize,
    ) -> Result<Option<ValueSymbol>> {
        if self.type_parameter_in_scope(caller, name) {
            return Ok(None);
        }
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
                    lexical_values: Vec::new(),
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
        let mut groups = Vec::new();
        let mut lexical_owner = Some(owner.to_owned());
        for classifiers in &scope.lexical_values {
            groups.push((
                classifiers
                    .iter()
                    .filter(|(alias, _)| alias == name)
                    .map(|(_, path)| path.clone())
                    .collect::<Vec<_>>(),
                None,
            ));
            if let Some(owner) = lexical_owner {
                groups.push((Vec::new(), Some(owner.clone())));
                lexical_owner = self.scope_parent(&owner);
            }
        }
        let import_priority = groups.len();
        groups.extend(
            [explicit, vec![qualified(&scope.package, name)]]
                .into_iter()
                .map(|paths| (paths, None)),
        );
        groups.push((
            scope
                .imports
                .iter()
                .filter(|(alias, _, _)| alias == "*")
                .map(|(_, path, _)| qualified(path.trim_end_matches("/*"), name))
                .collect(),
            None,
        ));
        groups.push((
            DEFAULT_KOTLIN_PACKAGES
                .iter()
                .copied()
                .chain(["java/lang"])
                .map(|package| qualified(package, name))
                .collect(),
            None,
        ));
        let mut inaccessible = false;
        for (priority, (mut paths, inherited)) in groups.into_iter().enumerate() {
            if let Some(owner) = inherited {
                paths = self
                    .inherited_member_class(caller, &owner, name)?
                    .into_iter()
                    .map(|class| class.name)
                    .collect();
            }
            let explicit = priority == import_priority && !paths.is_empty();
            let mut symbols = std::collections::BTreeMap::new();
            for path in paths {
                if priority > import_priority && renamed(&path) {
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
                    let declaring = self.lexical_parents.borrow().get(&class.name).cloned();
                    let classifier_visible = self.classifier_visible(caller, &class.name)
                        && (visibility != Some(2)
                            || declaring.as_ref().is_some_and(|declaring| {
                                self.inheritance_scope(caller, declaring)
                            }));
                    let jvm_visible = if class.kotlin_class {
                        class.access & 1 != 0
                    } else {
                        (class.access & 1 != 0 || self.java_nested_access(&class.name).is_some())
                            && self
                                .check_java_import_visibility(caller, &class.name, offset)
                                .is_ok()
                    };
                    if !jvm_visible
                        || !classifier_visible
                        || !internal_visible
                        || matches!(visibility, Some(5))
                    {
                        inaccessible = true;
                    }
                    if jvm_visible
                        && !class.unsupported
                        && internal_visible
                        && classifier_visible
                        && !matches!(visibility, Some(5))
                    {
                        symbols.insert(
                            format!("class:{}", class.name),
                            ValueSymbol::Classifier(Box::new(class)),
                        );
                    }
                }
                let (package, member) = path.rsplit_once('/').unwrap_or(("", &path));
                if priority <= import_priority || priority == import_priority + 2 {
                    if let Ok(class) = self.load(package) {
                        if class.access & 1 != 0 && !class.unsupported {
                            if let Some((nested, access)) =
                                self.java_member_class(package, member)?
                            {
                                if access & 9 == 9
                                    && !nested.unsupported
                                    && self.classifier_visible(caller, &nested.name)
                                {
                                    symbols.insert(
                                        format!("class:{}", nested.name),
                                        ValueSymbol::Classifier(Box::new(nested)),
                                    );
                                }
                            }
                            for mut field in self.field_candidates(package, member)? {
                                if field.access & 8 != 0
                                    && !field.unsupported
                                    && self.kotlin_field_visible(caller, &field)
                                {
                                    let identity = format!("field:{}:{}", field.owner, field.name);
                                    if !class.kotlin_class
                                        && self
                                            .load(&field.owner)
                                            .is_ok_and(|declaring| declaring.access & 1 == 0)
                                    {
                                        field.owner = class.name.clone();
                                    }
                                    symbols.insert(identity, ValueSymbol::Field(field));
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
