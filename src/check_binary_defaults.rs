use super::*;

impl Checker<'_> {
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
        let (mut storage, _) = crate::jvm_descriptor::method_types(&method.descriptor)?;
        if library.suspend_
            && storage.last() == Some(&Type::Java("kotlin/coroutines/Continuation".into()))
        {
            storage.pop();
        }
        let Some(dispatch) = values.len().checked_sub(storage.len()).filter(|n| *n <= 1) else {
            return Err(Diagnostic::new(
                offset,
                "Kotlin default argument storage does not match its JVM declaration",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "KOTLIN_DEFAULT_ARGUMENT_ABI_UNSUPPORTED",
            ));
        };
        let reserved =
            usize::from(library.receiver.is_some()) + usize::from(library.value_dispatch);
        for (index, storage) in storage.into_iter().enumerate().skip(reserved) {
            let parameter = index - reserved;
            if parameter < 64 && mask & (1u64 << parameter) != 0 {
                values[dispatch + index] = zero_value(&storage);
            }
        }
        Ok(())
    }
}
