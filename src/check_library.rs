pub(super) use super::relations::{specialize_receiver, type_owner};
use super::*;

pub(super) use crate::classpath::DEFAULT_KOTLIN_PACKAGES as DEFAULT_PACKAGES;

impl Checker<'_> {
    pub(super) fn library_candidates(&self, name: &str) -> Vec<JavaMethod> {
        if name == "this" && self.receiver.is_some()
            || self.local(name).is_some()
            || self.captured.contains_key(name)
            || self.fields.contains_key(name)
            || self.object_fields.contains_key(name)
            || self.companion_fields.contains_key(name)
            || self.constants.contains_key(name)
        {
            return Vec::new();
        }
        let Some(binding) = self.imports.get(name) else {
            if self
                .receiver
                .as_ref()
                .and_then(|receiver| library::type_owner(&receiver.ty))
                .is_some_and(|owner| {
                    self.classpath.methods(owner).is_ok_and(|methods| {
                        methods
                            .iter()
                            .any(|method| method.name == name && method.access & 8 == 0)
                    })
                })
            {
                return Vec::new();
            }
            if self.callable_name_was_renamed(name)
                || self.functions.contains_key(name)
                || self.object_methods.contains_key(name)
                || self.members.values().any(|method| {
                    method.name == name || method.source_property.as_deref() == Some(name)
                })
            {
                return Vec::new();
            }
            let mut methods = Vec::new();
            for package in DEFAULT_PACKAGES {
                if let Ok(package_methods) = self.classpath.package_methods(package, name) {
                    methods.extend(package_methods.into_iter().filter(|method| {
                        method.access & 8 != 0
                            && (method.access & 1 != 0
                                || method
                                    .library
                                    .as_ref()
                                    .is_some_and(|library| library.inline && library.kotlin_public))
                            && !method.kotlin_internal
                            && method.library.as_ref().is_some_and(|library| {
                                library.receiver.is_none() && library.source_name == name
                            })
                    }));
                }
            }
            methods.sort_by(|a, b| {
                (&a.owner, &a.name, &a.descriptor).cmp(&(&b.owner, &b.name, &b.descriptor))
            });
            methods.dedup_by(|a, b| {
                a.owner == b.owner && a.name == b.name && a.descriptor == b.descriptor
            });
            return methods;
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
                m.access & 8 != 0
                    && (m.access & 1 != 0
                        || m.library
                            .as_ref()
                            .is_some_and(|library| library.inline && library.kotlin_public))
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
        if method.access & 1 == 0
            && (!inline
                || mask != 0
                || !method
                    .library
                    .as_ref()
                    .is_some_and(|library| library.kotlin_public))
        {
            return Err(Diagnostic::new(
                offset,
                "Private JVM inline declaration requires complete inline expansion",
            ));
        }
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

pub(super) fn default_class(classpath: &Classpath, name: &str) -> Option<JavaClass> {
    if name.contains('/') || name.contains('.') || name.contains('$') {
        return None;
    }
    classpath
        .load(&format!("java/lang/{name}"))
        .ok()
        .filter(|class| class.access & 1 != 0)
}

pub(super) fn default_type_imports(
    ty: &Type,
    imports: &HashMap<String, String>,
    classpath: &Classpath,
) -> HashMap<String, String> {
    fn visit(ty: &Type, imports: &mut HashMap<String, String>, classpath: &Classpath) {
        match ty {
            Type::Java(name) => {
                if !imports.contains_key(name) {
                    if let Some(class) = default_class(classpath, name) {
                        imports.insert(name.clone(), class.name);
                    }
                }
            }
            Type::Parameterized { name, arguments } => {
                visit(&Type::Java(name.clone()), imports, classpath);
                for arg in arguments {
                    if let Some(ty) = &arg.ty {
                        visit(ty, imports, classpath);
                    }
                }
            }
            Type::Array(inner)
            | Type::Nullable(inner)
            | Type::Platform(inner)
            | Type::List(inner)
            | Type::MutableList(inner)
            | Type::ThreadLocal(inner) => visit(inner, imports, classpath),
            Type::Parameter { upper_bound, .. } => visit(upper_bound, imports, classpath),
            Type::Function { params, result } => {
                for param in params {
                    visit(param, imports, classpath);
                }
                visit(result, imports, classpath);
            }
            Type::ReceiverFunction {
                receiver,
                params,
                result,
            } => {
                visit(receiver, imports, classpath);
                for param in params {
                    visit(param, imports, classpath);
                }
                visit(result, imports, classpath);
            }
            _ => (),
        }
    }
    let mut resolved = imports.clone();
    visit(ty, &mut resolved, classpath);
    resolved
}

impl Checker<'_> {
    pub(super) fn default_constructor(
        &mut self,
        name: &str,
        args: &[Expr],
        offset: usize,
    ) -> Result<Option<Value>> {
        if self.imports.contains_key(name)
            || self.functions.contains_key(name)
            || self.local(name).is_some()
            || self.captured.contains_key(name)
            || self.object_methods.contains_key(name)
            || self.fields.contains_key(name)
            || self.object_fields.contains_key(name)
            || self.companion_fields.contains_key(name)
            || self.constants.contains_key(name)
            || self.members.values().any(|method| {
                method.name == name || method.source_property.as_deref() == Some(name)
            })
        {
            return Ok(None);
        }
        let Some(class) = default_class(self.classpath, name) else {
            return Ok(None);
        };
        let constructors = class
            .methods
            .iter()
            .filter(|method| {
                method.name == "<init>" && method.access & 1 != 0 && !method.unsupported
            })
            .cloned()
            .collect::<Vec<_>>();
        let method = self.select_method(&constructors, args, offset)?;
        let (params, defaults) = Self::method_parameters(&method)?;
        self.constructor_call(&class.name, &params, &defaults, args, false, offset)
            .map(Some)
    }
}

impl Checker<'_> {
    pub(super) fn generic_constructor(
        &mut self,
        name: &str,
        types: &[Type],
        args: &[Expr],
        offset: usize,
    ) -> Result<Option<Value>> {
        if self
            .imports
            .get(name)
            .is_some_and(|binding| binding.contains("::"))
        {
            return Ok(None);
        }
        let requested = Type::Parameterized {
            name: name.into(),
            arguments: types
                .iter()
                .map(|ty| {
                    Ok(syntax::TypeArgument {
                        variance: syntax::Variance::Invariant,
                        ty: Some(self.resolve(ty, offset)?),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        };
        let expanded =
            self.classpath
                .expand_type_alias(&requested, self.imports, DEFAULT_PACKAGES)?;
        let alias = expanded != requested;
        let class = if let Some(owner) = self.imports.get(name) {
            self.classes
                .get(owner)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| self.classpath.load(owner))?
        } else if alias {
            let owner = library::type_owner(&expanded).ok_or_else(|| {
                Diagnostic::new(offset, "Type alias does not name a constructible class")
            })?;
            self.classpath.load(owner)?
        } else if let Some(class) = default_class(self.classpath, name) {
            class
        } else {
            return Ok(None);
        };
        let ty = if alias {
            self.resolve(&expanded, offset)?
        } else {
            let Type::Parameterized { arguments, .. } = requested else {
                unreachable!()
            };
            Type::Parameterized {
                name: class.name.clone(),
                arguments,
            }
        };
        let Some(library) = &class.library else {
            return Err(Diagnostic::new(
                offset,
                "Class has no supported type parameters",
            ));
        };
        let Type::Parameterized { arguments, .. } = &ty else {
            return Err(Diagnostic::new(
                offset,
                "Generic constructor requires a parameterized class",
            ));
        };
        if library.type_parameters.len() != arguments.len() {
            return Err(Diagnostic::new(
                offset,
                "Wrong number of constructor type arguments",
            ));
        }
        relations::validate_type_arguments_with_classes(&ty, self.classpath, self.classes, offset)?;
        let constructors = class
            .methods
            .iter()
            .filter(|method| {
                method.name == "<init>" && method.access & 1 != 0 && !method.unsupported
            })
            .cloned()
            .map(|method| library::specialize_receiver(method, &ty, &class, self.classpath))
            .collect::<Vec<_>>();
        let method = self.select_method(&constructors, args, offset)?;
        let (params, defaults) = Self::method_parameters(&method)?;
        let (values, lets, mask) =
            self.arguments(&params, &defaults, args, self.method_named(&method), offset)?;
        let mut raw = method.clone();
        raw.kotlin_params = None;
        raw.kotlin_result = None;
        let (storage, _) = class_method_types(&raw)?;
        let mut values = values
            .into_iter()
            .zip(storage)
            .map(|(value, ty)| self.coerce(value, java_parameter(ty), offset))
            .collect::<Result<Vec<_>>>()?;
        let descriptor = if mask == 0 {
            method.descriptor.clone()
        } else {
            values.push(Value {
                ty: Type::Int,
                kind: ValueKind::Int(mask as i32),
            });
            values.push(Value {
                ty: Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
                kind: ValueKind::Null,
            });
            format!(
                "{}ILkotlin/jvm/internal/DefaultConstructorMarker;)V",
                method.descriptor.trim_end_matches(")V")
            )
        };
        Ok(Some(Self::wrap_lets(
            Value {
                ty,
                kind: ValueKind::New {
                    owner: class.name,
                    descriptor,
                    args: values,
                },
            },
            lets,
        )))
    }
}
