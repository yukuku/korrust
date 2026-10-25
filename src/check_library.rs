pub(super) use super::relations::{specialize_receiver, type_owner};
use super::*;

impl Checker<'_> {
    pub(super) fn library_candidates(&self, name: &str) -> Vec<JavaMethod> {
        let Some(binding) = self.imports.get(name) else {
            return Vec::new();
        };
        let Some((owner, source_name)) = binding.rsplit_once("::") else {
            return Vec::new();
        };
        let package = self
            .classes
            .get(owner)
            .and_then(|c| c.library.as_ref())
            .and_then(|l| l.package.as_deref());
        self.classes
            .values()
            .filter(|c| {
                c.library.as_ref().and_then(|l| l.package.as_deref()) == package
                    && package.is_some()
            })
            .flat_map(|c| &c.methods)
            .filter(|m| {
                m.access & 9 == 9
                    && (m.source_property.as_deref() == Some(source_name)
                        || m.library
                            .as_ref()
                            .is_some_and(|l| l.source_name == source_name))
            })
            .cloned()
            .collect()
    }

    pub(super) fn library_static_call(
        &mut self,
        method: JavaMethod,
        args: &[Expr],
        offset: usize,
    ) -> Result<Value> {
        if self
            .classpath
            .load(&method.owner)
            .is_ok_and(|c| c.access & 0x0200 != 0)
        {
            return Err(Diagnostic::new(
                offset,
                "Static interface methods are unsupported",
            ));
        }
        let (types, result) = class_method_types(&method)?;
        let params = method
            .kotlin_params
            .as_ref()
            .map(|p| p.iter().map(|p| (p.0.clone(), p.1.clone())).collect())
            .unwrap_or_else(|| {
                types
                    .into_iter()
                    .enumerate()
                    .map(|(i, t)| (format!("p{i}"), java_parameter(t)))
                    .collect::<Vec<_>>()
            });
        let defaults = method
            .kotlin_params
            .as_ref()
            .map(|p| p.iter().map(|p| p.2).collect())
            .unwrap_or_else(|| vec![false; params.len()]);
        let named = self.method_named(&method);
        let inline = method.library.as_ref().is_some_and(|m| m.inline);
        if inline {
            crate::compiled_inline::validate(&method)?;
        }
        let old_inline = self.inline_result.clone();
        self.inline_result = if inline && (self.allow_return || self.nonlocal_result.is_some()) {
            Some(
                self.nonlocal_result
                    .clone()
                    .unwrap_or_else(|| self.result.clone()),
            )
        } else {
            None
        };
        let checked = self.arguments(&params, &defaults, args, named, offset);
        self.inline_result = old_inline;
        let (mut values, lets, mut mask) = checked?;
        if method
            .library
            .as_ref()
            .is_some_and(|l| l.receiver.is_some())
        {
            mask >>= 1;
        }
        if inline && !lets.is_empty() {
            return Err(Diagnostic::new(
                offset,
                "Reordered named arguments in compiled inline calls are unsupported",
            ));
        }
        let kind = if inline && mask == 0 {
            let code = method
                .library
                .as_ref()
                .and_then(|l| l.code.as_ref())
                .ok_or_else(|| Diagnostic::new(offset, "Compiled inline bytecode unavailable"))?;
            let local_base = self.next;
            self.next = self
                .next
                .checked_add(code.max_locals)
                .ok_or_else(|| Diagnostic::new(offset, "Inline locals overflow"))?;
            let mut lambda_bases = Vec::new();
            for value in &values {
                if let ValueKind::Lambda { owner, .. } = &value.kind {
                    let lambdas = self.lambdas.borrow();
                    let lambda = lambdas.iter().find(|l| l.owner == *owner).ok_or_else(|| {
                        Diagnostic::new(offset, "Inline lambda declaration missing")
                    })?;
                    lambda_bases.push(Some(self.next));
                    self.next = self
                        .next
                        .checked_add(lambda.function.locals)
                        .ok_or_else(|| Diagnostic::new(offset, "Inline lambda locals overflow"))?;
                } else {
                    lambda_bases.push(None);
                }
            }
            if self.next > 256 {
                return Err(Diagnostic::new(
                    offset,
                    "Compiled inline locals exceed current limit",
                ));
            }
            ValueKind::CompiledInline {
                method,
                args: values,
                local_base,
                lambda_bases,
            }
        } else if mask == 0 {
            ValueKind::JavaCall {
                method,
                args: values,
            }
        } else {
            if inline {
                return Err(Diagnostic::new(
                    offset,
                    "Compiled inline default arguments are unsupported",
                ));
            }
            let split = method.descriptor.find(')').unwrap();
            let descriptor = format!(
                "({}ILjava/lang/Object;){}",
                &method.descriptor[1..split],
                &method.descriptor[split + 1..]
            );
            values.push(Value {
                ty: Type::Int,
                kind: ValueKind::Int(mask as i32),
            });
            values.push(Value {
                ty: Type::Java("java/lang/Object".into()),
                kind: ValueKind::Null,
            });
            ValueKind::StaticCall {
                owner: method.owner,
                name: format!("{}$default", method.name),
                descriptor,
                args: values,
            }
        };
        Ok(Self::wrap_lets(Value { ty: result, kind }, lets))
    }

    pub(super) fn extension_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        types: &[Type],
        offset: usize,
    ) -> Result<Value> {
        self.resolved_extension_call(
            receiver,
            name,
            args,
            types,
            operations::ExtensionKind::Ordinary,
            offset,
        )
    }
}

impl Checker<'_> {
    pub(super) fn source_call(
        &mut self,
        name: &str,
        args: &[Expr],
        explicit: &[Type],
        offset: usize,
    ) -> Result<Value> {
        let mut candidates = self
            .function_names
            .iter()
            .enumerate()
            .filter(|(_, n)| *n == name)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            if let Some(id) = self.functions.get(name) {
                candidates.push(*id);
            }
        }
        let id = if explicit.is_empty() {
            self.select_function(&candidates, args, offset)?
        } else {
            self.select_function_types(&candidates, args, explicit, offset)?
        };
        if self.extensions.contains(&id) {
            return Err(Diagnostic::new(
                offset,
                "Calling declared Kotlin extensions is not supported",
            ));
        }
        let (params, result) = self.specialize_source(id, args, explicit, offset)?;
        let defaults = self
            .defaults
            .get(id)
            .map(|v| v.iter().map(Option::is_some).collect::<Vec<_>>())
            .unwrap_or_else(|| vec![false; params.len()]);
        let (values, lets, mask) = self.arguments(&params, &defaults, args, true, offset)?;
        Ok(Self::wrap_lets(
            Value {
                ty: result,
                kind: if mask == 0 {
                    ValueKind::Call(id, values)
                } else {
                    ValueKind::DefaultCall(id, values, mask)
                },
            },
            lets,
        ))
    }
}
