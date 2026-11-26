use super::*;

impl Checker<'_> {
    pub(super) fn lexical_companion_property(
        &mut self,
        name: &str,
        argument: Option<&Expr>,
        offset: usize,
    ) -> Result<Option<Value>> {
        if self.local(name).is_some()
            || self.captured.contains_key(name)
            || self.fields.contains_key(name)
            || self.constants.contains_key(name)
            || self
                .members
                .values()
                .any(|method| method.source_property.as_deref() == Some(name))
            || name == "field" && self.members.contains_key("#backing")
        {
            return Ok(None);
        }
        let mut owner = self.lambda_owner.clone();
        loop {
            if let Ok(class) = self.classpath.load(&owner)
                && class.access & 0x200 == 0
                && let Some(companion) = class
                    .library
                    .as_ref()
                    .and_then(|library| library.companion_name.as_deref())
            {
                let target = format!("{owner}${companion}");
                if target == self.lambda_owner {
                    return Ok(None);
                }
                let methods = self.classpath.methods(&target)?;
                let accessor = if argument.is_some() {
                    setter_name(name)
                } else {
                    getter_name(name)
                };
                let method = methods
                    .iter()
                    .find(|method| {
                        method.source_property.as_deref() == Some(name)
                            && method.source_name() == accessor
                    })
                    .cloned();
                if let Some(method) = method {
                    let receiver = self
                        .companion_receiver(&owner, offset)?
                        .ok_or_else(|| Diagnostic::new(offset, "Missing companion singleton"))?;
                    if method.access & 2 != 0 {
                        if self.classpath.default_source_accessor(&method).is_none() {
                            return Err(Diagnostic::new(offset, "Private companion custom accessors from enclosing declarations require unsupported lexical bridges").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "PRIVATE_COMPANION_ACCESSOR_SCOPE_UNSUPPORTED"));
                        }
                        if self.public_inline_body {
                            return Err(Diagnostic::new(
                                offset,
                                "Public inline bodies cannot access private companion properties",
                            )
                            .with_kind(
                                crate::DiagnosticKind::InvalidKotlin,
                                "NON_PUBLIC_CALL_FROM_PUBLIC_INLINE",
                            ));
                        }
                        let mut values = vec![receiver];
                        if let Some(argument) = argument {
                            let (params, _) = class_method_types(&method)?;
                            values.push(self.expected_value(argument, &params[0])?);
                        }
                        return self.source_backing_accessor(&method, values, offset);
                    }
                    return self
                        .member_call(
                            method,
                            receiver,
                            argument.map(std::slice::from_ref).unwrap_or(&[]),
                            offset,
                        )
                        .map(Some);
                }
                if let Some(field) = self
                    .classpath
                    .field_candidates(&target, name)?
                    .into_iter()
                    .find(|field| field.kotlin.is_some())
                {
                    return if let Some(argument) = argument {
                        let statement =
                            self.assign_java_member(field.owner, None, name, argument, offset)?;
                        Ok(Some(Value {
                            ty: Type::Unit,
                            kind: ValueKind::Block(
                                vec![statement],
                                Box::new(Value {
                                    ty: Type::Unit,
                                    kind: ValueKind::Unit,
                                }),
                            ),
                        }))
                    } else {
                        self.java_field(&field.owner, name, None, offset).map(Some)
                    };
                }
                if argument.is_some()
                    && methods
                        .iter()
                        .any(|method| method.source_property.as_deref() == Some(name))
                {
                    return Err(
                        Diagnostic::new(offset, "Cannot reassign val companion property")
                            .with_kind(
                                crate::DiagnosticKind::InvalidKotlin,
                                "VAL_PROPERTY_REASSIGNMENT",
                            ),
                    );
                }
            }
            let Some((parent, _)) = owner.rsplit_once('$') else {
                return Ok(None);
            };
            owner = parent.into();
        }
    }
}
