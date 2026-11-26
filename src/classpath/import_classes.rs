use super::{Classpath, Diagnostic, JavaClass, Result};

fn package(owner: &str) -> &str {
    owner.rsplit_once('/').map_or("", |(package, _)| package)
}

impl Classpath {
    pub(crate) fn java_nested_access(&self, owner: &str) -> Option<u16> {
        self.nested_access.borrow().get(owner).copied()
    }

    pub(crate) fn check_java_import_visibility(
        &self,
        caller: &str,
        owner: &str,
        offset: usize,
    ) -> Result<()> {
        let mut current = owner.to_owned();
        for _ in 0..64 {
            let class = self.load(&current)?;
            let access = self.java_nested_access(&current).unwrap_or(class.access);
            if access & 2 != 0 || access & 1 == 0 && package(caller) != package(&current) {
                return Err(Diagnostic::new(
                    offset,
                    format!("Imported Java classifier is inaccessible: {current}"),
                )
                .with_kind(crate::DiagnosticKind::InvalidKotlin, "INVISIBLE_REFERENCE"));
            }
            let parent = self.lexical_parents.borrow().get(&current).cloned();
            let Some(parent) = parent else {
                return Ok(());
            };
            current = parent;
        }
        Err(Diagnostic::new(
            offset,
            "Nested Java classifier hierarchy exceeds the supported depth",
        ))
    }

    pub(crate) fn java_import_class(
        &self,
        name: &str,
        caller: &str,
        offset: usize,
    ) -> Result<Option<JavaClass>> {
        let parts = name.split('/').collect::<Vec<_>>();
        let mut found = None;
        for end in (1..=parts.len()).rev() {
            match self.load(&parts[..end].join("/")) {
                Ok(class) => {
                    if class.kotlin_class {
                        return Ok(None);
                    }
                    found = Some((class, end));
                    break;
                }
                Err(error) if error.code == "CLASS_NOT_FOUND" => {}
                Err(mut error) => {
                    error.offset = offset;
                    return Err(error);
                }
            }
        }
        let Some((mut class, cursor)) = found else {
            return Ok(None);
        };
        self.check_java_import_visibility(caller, &class.name, offset)?;
        for member in &parts[cursor..] {
            let Some((nested, _)) = self.java_member_class(&class.name, member)? else {
                return Ok(None);
            };
            self.check_java_import_visibility(caller, &nested.name, offset)?;
            class = nested;
        }
        Ok(Some(class))
    }
}
