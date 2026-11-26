use super::*;

impl Checker<'_> {
    pub(super) fn binary_marker_constructor(
        &mut self,
        class: &JavaClass,
        method: &JavaMethod,
        args: &[Expr],
        result: Type,
        offset: usize,
    ) -> Result<Option<Value>> {
        const MARKER: &str = "Lkotlin/jvm/internal/DefaultConstructorMarker;)V";
        if self.classpath.is_source(&class.name)
            || class
                .library
                .as_ref()
                .is_some_and(|library| library.value_class.is_some())
            || !method
                .library
                .as_ref()
                .is_some_and(|library| library.kotlin_declared)
        {
            return Ok(None);
        }
        let Some(prefix) = method.descriptor.strip_suffix(MARKER) else {
            return Ok(None);
        };
        let (params, defaults) = Self::method_parameters(method)?;
        let base_descriptor = format!("{prefix})V");
        let (storage, _) = crate::jvm_descriptor::method_types(&base_descriptor).map_err(|_| {
            Diagnostic::new(offset, "Malformed Kotlin constructor bridge descriptor")
        })?;
        if storage.len() != params.len() {
            return Ok(None);
        }
        if !self
            .classpath
            .constructor_visible(&self.lambda_owner, method)
        {
            return Err(
                Diagnostic::new(offset, "Constructor is inaccessible").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "INACCESSIBLE_CONSTRUCTOR",
                ),
            );
        }
        if self.public_inline_body && method.kotlin_internal {
            return Err(Diagnostic::new(
                offset,
                "Public inline functions cannot access nonpublic constructors",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "PUBLIC_INLINE_NON_PUBLIC_ACCESS",
            ));
        }
        let (values, lets, mask) = self.arguments_checked(
            &params,
            &defaults,
            args,
            self.method_named(method),
            false,
            offset,
        )?;
        let storage = if mask == 0 {
            storage
        } else {
            let original = class
                .methods
                .iter()
                .find(|candidate| {
                    candidate.name == method.name && candidate.descriptor == method.descriptor
                })
                .unwrap_or(method);
            crate::value_abi::default_storage(
                original.kotlin_params.as_ref().ok_or_else(|| {
                    Diagnostic::new(
                        offset,
                        "Kotlin constructor default declaration is unavailable",
                    )
                })?,
                &storage,
                0,
                &self.classpath.value_classes(),
            )
            .ok_or_else(|| {
                Diagnostic::new(offset, "Kotlin constructor default storage is invalid")
            })?
        };
        let default_parameters = storage.iter().map(Type::descriptor).collect::<String>();
        let mut values = values
            .into_iter()
            .zip(storage)
            .enumerate()
            .map(|(index, (value, storage))| {
                if mask != 0 && index < 64 && mask & (1u64 << index) != 0 {
                    zero_value(&storage)
                } else {
                    value
                }
            })
            .collect::<Vec<_>>();
        let descriptor = if mask == 0 {
            method.descriptor.clone()
        } else {
            let words = params.len().div_ceil(32);
            let descriptor = format!("({default_parameters}{}{MARKER}", "I".repeat(words));
            let bridge = class
                .methods
                .iter()
                .find(|candidate| {
                    candidate.name == "<init>"
                        && candidate.descriptor == descriptor
                        && candidate.access & 0x1009 == 0x1001
                })
                .ok_or_else(|| {
                    Diagnostic::new(
                        offset,
                        "Accessible Kotlin default constructor bridge is unavailable",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "KOTLIN_CONSTRUCTOR_DEFAULT_ABI_UNSUPPORTED",
                    )
                })?;
            for word in 0..words {
                values.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int((mask >> (word * 32)) as i32),
                });
            }
            bridge.descriptor.clone()
        };
        values.push(Value {
            ty: Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
            kind: ValueKind::Null,
        });
        Ok(Some(Self::wrap_lets(
            Value {
                ty: result,
                kind: ValueKind::New {
                    owner: class.name.clone(),
                    descriptor,
                    args: values,
                },
            },
            lets,
        )))
    }
}
