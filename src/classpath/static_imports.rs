use super::*;

impl Classpath {
    pub(crate) fn static_wildcard_owner(
        &self,
        owner: &str,
        offset: usize,
    ) -> Result<Option<JavaClass>> {
        let Ok(mut class) = self.load(owner) else {
            return Ok(None);
        };
        if class.access & 1 == 0 {
            return Err(
                Diagnostic::new(offset, "Wildcard import owner is inaccessible")
                    .with_kind(crate::DiagnosticKind::InvalidKotlin, "INVISIBLE_REFERENCE"),
            );
        }
        if class.kotlin_class
            || class
                .library
                .as_ref()
                .is_some_and(|library| library.package.is_some())
            || class.unsupported
            || class.access & 0x0200 != 0
        {
            return Err(Diagnostic::new(
                offset,
                "Wildcard member imports require a supported Java class",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "STATIC_MEMBER_WILDCARD_OWNER_UNSUPPORTED",
            ));
        }
        class.methods = self
            .methods(owner)?
            .into_iter()
            .filter(|method| {
                method.access & 9 == 9
                    && (method.owner == class.name
                        || self
                            .load(&method.owner)
                            .is_ok_and(|declaring| declaring.access & 0x0200 == 0))
            })
            .map(|mut method| {
                if self
                    .load(&method.owner)
                    .is_ok_and(|declaring| declaring.access & 1 == 0)
                {
                    method.owner = class.name.clone();
                }
                method
            })
            .collect();
        Ok(Some(class))
    }
}
