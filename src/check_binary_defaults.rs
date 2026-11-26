use super::*;

impl Checker<'_> {
    pub(super) fn binary_default_descriptor(
        &self,
        method: &JavaMethod,
        offset: usize,
    ) -> Result<Option<String>> {
        if self.classpath.is_source(&method.owner)
            || !method
                .library
                .as_ref()
                .is_some_and(|library| library.kotlin_declared)
        {
            return Ok(None);
        }
        let class = self.classpath.load(&method.owner)?;
        let original = class
            .methods
            .iter()
            .find(|candidate| {
                candidate.name == method.name
                    && candidate.descriptor == method.descriptor
                    && candidate.kotlin_params.is_some()
            })
            .unwrap_or(method);
        let parameters = original
            .kotlin_params
            .as_ref()
            .ok_or_else(|| Diagnostic::new(offset, "Kotlin default declaration is unavailable"))?;
        let (storage, _) = crate::jvm_descriptor::method_types(&method.descriptor)?;
        let trailing = usize::from(
            method
                .library
                .as_ref()
                .is_some_and(|library| library.suspend_),
        );
        let storage = crate::value_abi::default_storage(
            parameters,
            &storage,
            trailing,
            &self.classpath.value_classes(),
        )
        .ok_or_else(|| Diagnostic::new(offset, "Kotlin default argument storage is invalid"))?;
        let dispatch = if method.access & 8 == 0 {
            format!("L{};", method.owner)
        } else {
            String::new()
        };
        let result = &method.descriptor[method.descriptor.find(')').unwrap() + 1..];
        let descriptor = format!(
            "({dispatch}{}{}Ljava/lang/Object;){result}",
            storage.iter().map(Type::descriptor).collect::<String>(),
            "I".repeat(parameters.len().div_ceil(32))
        );
        let name = format!("{}$default", method.name);
        if !self
            .classpath
            .methods(&method.owner)?
            .iter()
            .any(|candidate| {
                candidate.name == name
                    && candidate.descriptor == descriptor
                    && candidate.access & 0x1008 == 0x1008
            })
        {
            return Err(Diagnostic::new(
                offset,
                "Kotlin default bridge does not match the resolved binary",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "KOTLIN_DEFAULT_ARGUMENT_ABI_UNSUPPORTED",
            ));
        }
        Ok(Some(descriptor))
    }

    pub(super) fn physical_default_values(
        &self,
        method: &JavaMethod,
        values: &mut [Value],
        mask: u64,
        offset: usize,
    ) -> Result<()> {
        let Some(library) = &method.library else {
            return Ok(());
        };
        if mask == 0 || !library.kotlin_declared || self.classpath.is_source(&method.owner) {
            return Ok(());
        }
        let Some(descriptor) = self.binary_default_descriptor(method, offset)? else {
            return Ok(());
        };
        let (mut storage, _) = crate::jvm_descriptor::method_types(&descriptor)?;
        let count = method.kotlin_params.as_ref().map_or(0, Vec::len);
        storage.truncate(storage.len() - count.div_ceil(32) - 1);
        if library.suspend_ {
            storage.pop();
        }
        if values.len() != storage.len() {
            return Err(Diagnostic::new(
                offset,
                "Kotlin default argument storage does not match its JVM declaration",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "KOTLIN_DEFAULT_ARGUMENT_ABI_UNSUPPORTED",
            ));
        }
        let reserved = usize::from(method.access & 8 == 0)
            + usize::from(library.receiver.is_some())
            + usize::from(library.value_dispatch);
        for (index, storage) in storage.into_iter().enumerate().skip(reserved) {
            let parameter = index - reserved;
            if parameter < 64 && mask & (1u64 << parameter) != 0 {
                values[index] = zero_value(&storage);
            }
        }
        Ok(())
    }
}
