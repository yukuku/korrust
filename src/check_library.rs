pub(super) use super::relations::{specialize_receiver, type_owner};
use super::*;

pub(super) use crate::classpath::DEFAULT_KOTLIN_PACKAGES as DEFAULT_PACKAGES;

pub(super) fn ordinary_member_methods(
    classpath: &Classpath,
    owner: &str,
    name: &str,
    argument_count: usize,
) -> Result<Vec<JavaMethod>> {
    let mut methods = classpath
        .ordinary_methods(owner)?
        .into_iter()
        .map(|method| {
            if method.access & 0x40 != 0 && method.access & 9 == 1 {
                if let Some(declaration) = java_properties::accessibility_bridge(
                    classpath,
                    owner,
                    &method,
                    &mut HashSet::new(),
                )? {
                    return Ok(declaration);
                }
            }
            Ok(method)
        })
        .collect::<Result<Vec<_>>>()?;
    for mapping in crate::classpath::jvm_builtins::MAPPINGS
        .iter()
        .filter(|mapping| mapping.source_name == name)
    {
        if mapping.inherited_only && owner == mapping.owner {
            continue;
        }
        if relations::as_supertype(classpath, &Type::Java(owner.into()), mapping.owner).is_none() {
            continue;
        }
        let declarations =
            classpath.builtin_member_declarations(mapping.classifier, mapping.source_name)?;
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.descriptor == mapping.descriptor)
            .ok_or_else(|| {
                Diagnostic::new(0, "Kotlin builtin JVM mapping does not match its metadata")
            })?;
        for method in methods.iter_mut().filter(|method| {
            !method.unsupported
                && method.name == mapping.jvm_name
                && method.descriptor == mapping.descriptor
                && method.access & 9 == 1
                && method.source_name() == method.name
        }) {
            method.kotlin_params = declaration.kotlin_params.clone();
            method.kotlin_result = declaration.kotlin_result.clone();
            method.library = declaration.library.clone();
            if let Some(library) = &mut method.library {
                library.abstract_ = method.access & 0x400 != 0;
            }
        }
    }
    let declarations = if methods.iter().any(|method| {
        method.owner == "java/lang/Object"
            && method.source_name() == name
            && method.access & 9 == 1
            && class_method_types(method)
                .is_ok_and(|(parameters, _)| parameters.len() == argument_count)
    }) {
        classpath.builtin_class_declarations("kotlin/Any")?
    } else {
        Vec::new()
    };
    Ok(methods
        .into_iter()
        .filter_map(|mut method| {
            if method.owner == "java/lang/Object" {
                let declaration = declarations.iter().find(|declaration| {
                    declaration.name == method.source_name()
                        && declaration.descriptor == method.descriptor
                })?;
                method.library = declaration.library.clone();
                method.kotlin_params = declaration.kotlin_params.clone();
                method.kotlin_result = declaration.kotlin_result.clone();
            }
            Some(method)
        })
        .collect())
}

pub(super) fn remove_overridden(
    methods: Vec<JavaMethod>,
    classpath: &Classpath,
) -> Vec<JavaMethod> {
    methods
        .iter()
        .filter(|method| {
            !methods.iter().any(|other| {
                other.owner != method.owner
                    && other.source_name() == method.source_name()
                    && other.access & 8 == 0
                    && method.access & 8 == 0
                    && relations::as_supertype(
                        classpath,
                        &Type::Java(other.owner.clone()),
                        &method.owner,
                    )
                    .is_some()
                    && class_method_types(other)
                        .ok()
                        .zip(class_method_types(method).ok())
                        .is_some_and(|((a, _), (b, _))| {
                            a.iter()
                                .map(Type::descriptor)
                                .eq(b.iter().map(Type::descriptor))
                        })
            })
        })
        .cloned()
        .collect()
}

pub(super) fn static_import_owner(
    classpath: &Classpath,
    owner: &str,
    member: &str,
    offset: usize,
) -> Result<Option<JavaClass>> {
    let Ok(mut class) = classpath.load(owner) else {
        return Ok(None);
    };
    if class.access & 1 == 0 || class.unsupported {
        return Ok(None);
    }
    let methods = classpath.methods(owner)?;
    let matching_method = methods.iter().any(|method| {
        method.access & 9 == 9
            && (method.source_name() == member || method.source_property.as_deref() == Some(member))
            && (method.owner == owner
                || classpath
                    .load(&method.owner)
                    .is_ok_and(|declaring| declaring.access & 0x0200 == 0))
    });
    let matching_field = classpath
        .field_candidates(owner, member)?
        .iter()
        .any(|field| field.access & 9 == 9);
    if !matching_method && !matching_field {
        return Ok(None);
    }
    if matching_method && class.access & 0x0200 != 0 {
        return Err(
            Diagnostic::new(offset, "Static interface method imports are unsupported").with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "STATIC_INTERFACE_IMPORT_UNSUPPORTED",
            ),
        );
    }
    class.methods = methods
        .into_iter()
        .filter(|method| {
            method.access & 8 == 0
                || method.owner == owner
                || classpath
                    .load(&method.owner)
                    .is_ok_and(|declaring| declaring.access & 0x0200 == 0)
        })
        .map(|mut method| {
            if method.access & 8 != 0
                && classpath
                    .load(&method.owner)
                    .is_ok_and(|declaring| !declaring.kotlin_class && declaring.access & 1 == 0)
            {
                method.owner = owner.into();
            }
            method
        })
        .collect();
    Ok(Some(class))
}

impl Checker<'_> {
    fn lexical_method_visible(&self, method: &JavaMethod) -> bool {
        (!method.kotlin_internal || self.classpath.internal_visible(&method.owner))
            && (method.access & 1 != 0
                || method.access & 2 != 0
                    && (self.lambda_owner == method.owner
                        || self.lambda_owner.starts_with(&format!("{}$", method.owner)))
                || method.access & 4 != 0
                    && self.receiver.as_ref().is_some_and(|receiver| {
                        self.subtype(&receiver.ty, &Type::Java(method.owner.clone()))
                    }))
    }

    pub(super) fn lexical_call(
        &mut self,
        name: &str,
        args: &[Expr],
        explicit: &[Type],
        expected: Option<&Type>,
        offset: usize,
    ) -> Result<Option<Value>> {
        if self.local(name).is_some() || self.captured.contains_key(name) {
            return Ok(None);
        }
        let mut methods = self.object_methods.get(name).cloned().unwrap_or_default();
        methods.extend(
            self.members
                .values()
                .filter(|method| method.source_name() == name)
                .cloned(),
        );
        if let Some(owner) = self
            .receiver
            .as_ref()
            .and_then(|receiver| type_owner(&receiver.ty))
        {
            if let Ok(inherited) = ordinary_member_methods(self.classpath, owner, name, args.len())
            {
                methods.extend(
                    inherited
                        .into_iter()
                        .filter(|method| method.source_name() == name && method.access & 8 == 0),
                );
            }
        }
        methods.retain(|method| {
            method.source_property.is_none() && self.lexical_method_visible(method)
        });
        let mut signatures = HashSet::new();
        methods
            .retain(|method| signatures.insert((method.name.clone(), method.descriptor.clone())));
        methods.sort_by(|a, b| {
            (&a.owner, &a.name, &a.descriptor).cmp(&(&b.owner, &b.name, &b.descriptor))
        });
        let mut applicable = Vec::new();
        for method in methods {
            let normalized = self.vararg_arguments(&method, args, offset)?;
            let (params, defaults) = Self::method_parameters(&method)?;
            if let Err(error) = Self::argument_mapping(
                &params,
                &defaults,
                &normalized,
                self.method_named(&method),
                offset,
            ) {
                if error.code == "ARGUMENT_MAPPING_MISMATCH" {
                    continue;
                }
                return Err(error);
            }
            match self.select_method_types(std::slice::from_ref(&method), args, explicit, offset) {
                Ok(_) => applicable.push(method),
                Err(error)
                    if error.code == "TYPE_MISMATCH"
                        && args.iter().any(|arg| {
                            let argument = if let ExprKind::NamedArgument(_, value) = &arg.kind {
                                value.as_ref()
                            } else {
                                arg
                            };
                            error.offset == argument.offset
                        }) => {}
                Err(error) => return Err(error),
            }
        }
        if applicable.is_empty() {
            return Ok(None);
        }
        let method = self.select_method_types(&applicable, args, explicit, offset)?;
        let method = if expected.is_some() {
            let original = applicable
                .iter()
                .find(|original| {
                    original.owner == method.owner
                        && original.name == method.name
                        && original.descriptor == method.descriptor
                })
                .unwrap();
            self.select_method_expected(
                std::slice::from_ref(original),
                args,
                explicit,
                expected,
                offset,
            )?
        } else {
            method
        };
        let value = if method.access & 8 != 0 {
            self.library_static_call(method, args, offset)?
        } else {
            let receiver = self.receiver_value(offset)?;
            self.member_call(method, receiver, args, offset)?
        };
        Ok(Some(value))
    }

    pub(super) fn library_candidates(&self, name: &str) -> Vec<JavaMethod> {
        self.library_candidates_with_members(name, true)
    }

    pub(super) fn library_call_candidates(&self, name: &str) -> Vec<JavaMethod> {
        self.library_candidates_with_members(name, false)
    }

    fn library_candidates_with_members(&self, name: &str, shadow_members: bool) -> Vec<JavaMethod> {
        if name == "this" && self.receiver.is_some()
            || self.local(name).is_some()
            || self.captured.contains_key(name)
            || self.fields.contains_key(name)
            || self.object_fields.contains_key(name)
            || self.companion_fields.contains_key(name)
            || self.constants.contains_key(name)
            || shadow_members
                && self.object_methods.get(name).is_some_and(|methods| {
                    methods
                        .iter()
                        .any(|method| self.lexical_method_visible(method))
                })
            || shadow_members
                && self.members.values().any(|method| {
                    method.source_name() == name
                        && method.access & 8 == 0
                        && self.lexical_method_visible(method)
                })
            || shadow_members
                && self
                    .receiver
                    .as_ref()
                    .and_then(|receiver| type_owner(&receiver.ty))
                    .is_some_and(|owner| {
                        self.classpath.methods(owner).is_ok_and(|methods| {
                            methods.iter().any(|method| {
                                method.source_name() == name
                                    && method.access & 8 == 0
                                    && self.lexical_method_visible(method)
                            })
                        })
                    })
        {
            return Vec::new();
        }
        let Some(binding) = self.imports.get(name) else {
            if shadow_members
                && self
                    .receiver
                    .as_ref()
                    .and_then(|receiver| library::type_owner(&receiver.ty))
                    .is_some_and(|owner| {
                        self.classpath.methods(owner).is_ok_and(|methods| {
                            methods.iter().any(|method| {
                                method.source_name() == name
                                    && method.access & 8 == 0
                                    && self.lexical_method_visible(method)
                            })
                        })
                    })
            {
                return Vec::new();
            }
            if self.callable_name_was_renamed(name)
                || self.functions.contains_key(name)
                || shadow_members
                    && self.object_methods.get(name).is_some_and(|methods| {
                        methods
                            .iter()
                            .any(|method| self.lexical_method_visible(method))
                    })
                || shadow_members
                    && self.members.values().any(|method| {
                        (method.source_name() == name
                            || method.source_property.as_deref() == Some(name))
                            && self.lexical_method_visible(method)
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
                                || self
                                    .classpath
                                    .file_private_visible(&self.lambda_owner, method)
                                || method
                                    .library
                                    .as_ref()
                                    .is_some_and(|library| library.inline && library.kotlin_public))
                            && (!method.kotlin_internal
                                || self.classpath.internal_visible(&method.owner))
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
                c.name == owner
                    || (c.library.as_ref().and_then(|l| l.package.as_deref()) == package
                        && package.is_some())
            })
            .flat_map(|c| &c.methods)
            .filter(|m| {
                m.access & 8 != 0
                    && (m.access & 1 != 0
                        || self.classpath.file_private_visible(&self.lambda_owner, m)
                        || m.library
                            .as_ref()
                            .is_some_and(|library| library.inline && library.kotlin_public))
                    && (m.source_property.as_deref() == Some(source_name)
                        || m.source_name() == source_name)
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
        let normalized = self.vararg_arguments(&method, args, offset)?;
        let args = normalized.as_slice();
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
        let inline = method
            .library
            .as_ref()
            .is_some_and(|m| m.inline && !m.builtin_array);
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
            .is_some_and(|library| library.builtin_array)
        {
            if mask != 0 || values.len() != 1 {
                return Err(Diagnostic::new(
                    offset,
                    "Unsupported builtin array declaration shape",
                ));
            }
            let value = values.remove(0);
            return Ok(Self::wrap_lets(self.coerce(value, result, offset)?, lets));
        }
        if method.access & 1 == 0
            && !self
                .classpath
                .file_private_visible(&self.lambda_owner, &method)
            && method.owner != self.lambda_owner
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
        if mask == 0 && self.classpath.default_source_accessor(&method).is_some() {
            return Ok(Self::wrap_lets(
                self.source_backing_accessor(&method, values, offset)?
                    .unwrap(),
                lets,
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
                "({}{}Ljava/lang/Object;){}",
                &method.descriptor[1..split],
                "I".repeat(
                    (params.len()
                        - usize::from(
                            method
                                .library
                                .as_ref()
                                .is_some_and(|library| library.receiver.is_some())
                        ))
                    .div_ceil(32)
                ),
                &method.descriptor[split + 1..]
            );
            for index in 0..(params.len()
                - usize::from(
                    method
                        .library
                        .as_ref()
                        .is_some_and(|library| library.receiver.is_some()),
                ))
            .div_ceil(32)
            {
                values.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int((mask >> (index * 32)) as i32),
                });
            }
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
        if let Ok(class) = self.classpath.load(&self.lambda_owner) {
            let methods = class
                .methods
                .into_iter()
                .filter(|method| method.source_name() == name)
                .collect::<Vec<_>>();
            if methods.iter().any(|method| method.access & 0x80 != 0) {
                let method = self.select_method_types(&methods, args, explicit, offset)?;
                return self.library_static_call(method, args, offset);
            }
        }
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
                    ValueKind::DefaultCall(id, values, mask as u32)
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
    std::iter::once("java/lang")
        .chain(DEFAULT_PACKAGES.iter().copied())
        .find_map(|package| {
            classpath
                .load(&format!("{package}/{name}"))
                .ok()
                .filter(|class| class.access & 1 != 0)
        })
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
        if self.functions.contains_key(name)
            || self.local(name).is_some()
            || self.captured.contains_key(name)
            || self.object_methods.contains_key(name)
            || self.fields.contains_key(name)
            || self.object_fields.contains_key(name)
            || self.companion_fields.contains_key(name)
            || self.constants.contains_key(name)
            || self.members.values().any(|method| {
                method.source_name() == name || method.source_property.as_deref() == Some(name)
            })
        {
            return Ok(None);
        }
        let requested = Type::Java(name.into());
        let expanded = self
            .classpath
            .expand_type_alias(&requested, self.imports, DEFAULT_PACKAGES)
            .map_err(|error| {
                if error.code == "TYPE_ALIAS_ARITY" {
                    Diagnostic::new(
                        offset,
                        "Generic alias construction requires explicit type arguments",
                    )
                    .with_kind(
                        crate::DiagnosticKind::UnsupportedLanguage,
                        "GENERIC_CONSTRUCTOR_INFERENCE_UNSUPPORTED",
                    )
                } else {
                    error
                }
            })?;
        let alias = expanded != requested;
        let mut expanded = expanded;
        while let Type::Nullable(inner) = expanded {
            expanded = *inner;
        }
        if alias && matches!(expanded, Type::Parameterized { .. }) {
            return self.generic_constructor(name, &[], args, offset);
        }
        if !alias && self.imports.contains_key(name) {
            return Ok(None);
        }
        let source_package = self
            .lambda_owner
            .rsplit_once('/')
            .map_or("", |(package, _)| package);
        let source_name = if source_package.is_empty() {
            name.into()
        } else {
            format!("{source_package}/{name}")
        };
        let class = if alias {
            let ty = self.resolve(&expanded, offset)?;
            let owner = library::type_owner(&ty).ok_or_else(|| {
                Diagnostic::new(offset, "Type alias does not name a constructible class").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "TYPE_ALIAS_CONSTRUCTOR_INVALID",
                )
            })?;
            Some(self.classpath.load(owner)?)
        } else {
            self.classpath
                .load(&source_name)
                .ok()
                .filter(|class| class.access & 1 != 0)
                .or_else(|| default_class(self.classpath, name))
        };
        let Some(class) = class else {
            return Ok(None);
        };
        if class.access & 1 == 0 {
            return Err(
                Diagnostic::new(offset, "Constructor classifier is inaccessible").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "INACCESSIBLE_CONSTRUCTOR",
                ),
            );
        }
        if class.access & (0x200 | 0x400) != 0 {
            return Err(Diagnostic::new(
                offset,
                "Abstract classes and interfaces cannot be constructed",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ABSTRACT_CONSTRUCTION_INVALID",
            ));
        }
        let constructors = class
            .methods
            .iter()
            .filter(|method| {
                method.name == "<init>" && method.access & 1 != 0 && !method.unsupported
            })
            .cloned()
            .collect::<Vec<_>>();
        let method = self.select_method(&constructors, args, offset)?;
        let normalized = self.vararg_arguments(&method, args, offset)?;
        let (params, defaults) = Self::method_parameters(&method)?;
        self.constructor_call(
            &class.name,
            &params,
            &defaults,
            &normalized,
            self.method_named(&method),
            offset,
        )
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
        let mut expanded =
            self.classpath
                .expand_type_alias(&requested, self.imports, DEFAULT_PACKAGES)?;
        let alias = expanded != requested;
        while let Type::Nullable(inner) = expanded {
            expanded = *inner;
        }
        let class = if alias {
            let owner = library::type_owner(&expanded).ok_or_else(|| {
                Diagnostic::new(offset, "Type alias does not name a constructible class").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "TYPE_ALIAS_CONSTRUCTOR_INVALID",
                )
            })?;
            self.classes
                .get(owner)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| self.classpath.load(owner))?
        } else if let Some(owner) = self.imports.get(name) {
            self.classes
                .get(owner)
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| self.classpath.load(owner))?
        } else if let Some(class) = default_class(self.classpath, name) {
            class
        } else {
            return Ok(None);
        };
        if class.access & 1 == 0 {
            return Err(
                Diagnostic::new(offset, "Constructor classifier is inaccessible").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "INACCESSIBLE_CONSTRUCTOR",
                ),
            );
        }
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
        if class.access & (0x200 | 0x400) != 0 {
            return Err(Diagnostic::new(
                offset,
                "Abstract classes and interfaces cannot be constructed",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ABSTRACT_CONSTRUCTION_INVALID",
            ));
        }
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
        let normalized = self.vararg_arguments(&method, args, offset)?;
        let args = normalized.as_slice();
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
            for index in 0..params.len().div_ceil(32) {
                values.push(Value {
                    ty: Type::Int,
                    kind: ValueKind::Int((mask >> (index * 32)) as i32),
                });
            }
            values.push(Value {
                ty: Type::Java("kotlin/jvm/internal/DefaultConstructorMarker".into()),
                kind: ValueKind::Null,
            });
            format!(
                "{}{}Lkotlin/jvm/internal/DefaultConstructorMarker;)V",
                method.descriptor.trim_end_matches(")V"),
                "I".repeat(params.len().div_ceil(32))
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

impl Checker<'_> {
    pub(super) fn qualified_classifier(&self, expression: &Expr) -> Option<JavaClass> {
        fn parts(expression: &Expr, names: &mut Vec<String>) -> bool {
            match &expression.kind {
                ExprKind::Name(name) => {
                    names.push(name.clone());
                    true
                }
                ExprKind::Property(receiver, name) if parts(receiver, names) => {
                    names.push(name.clone());
                    true
                }
                _ => false,
            }
        }
        let mut names = Vec::new();
        if !parts(expression, &mut names) || names.len() < 2 {
            return None;
        }
        let root = &names[0];
        if self.local(root).is_some()
            || self.captured.contains_key(root)
            || self.constants.contains_key(root)
            || self.fields.contains_key(root)
            || self.object_fields.contains_key(root)
            || self.companion_fields.contains_key(root)
            || self
                .members
                .values()
                .any(|method| method.source_property.as_ref() == Some(root))
        {
            return None;
        }
        let qualified = names.join("/");
        let owner = self
            .imports
            .get(&qualified)
            .cloned()
            .or_else(|| {
                self.imports
                    .get(&names[0])
                    .map(|root| format!("{root}${}", names[1..].join("$")))
            })
            .unwrap_or(qualified);
        self.classpath
            .load(&owner)
            .ok()
            .filter(|class| class.access & 1 != 0 && !class.unsupported)
    }
    pub(super) fn qualified_constructor(
        &mut self,
        expression: &Expr,
        args: &[Expr],
        offset: usize,
    ) -> Result<Option<Value>> {
        let Some(class) = self.qualified_classifier(expression) else {
            return Ok(None);
        };
        if class
            .fields
            .iter()
            .any(|f| f.name == "INSTANCE" && f.descriptor == format!("L{};", class.name))
        {
            return Ok(None);
        }
        if class.access & (0x200 | 0x400) != 0 {
            return Err(Diagnostic::new(
                offset,
                "Abstract classes and interfaces cannot be constructed",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "ABSTRACT_CONSTRUCTION_INVALID",
            ));
        }
        if class
            .library
            .as_ref()
            .is_some_and(|l| !l.type_parameters.is_empty())
        {
            return Err(Diagnostic::new(
                offset,
                "Qualified generic construction requires unsupported explicit classifier handling",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "QUALIFIED_GENERIC_CONSTRUCTOR_UNSUPPORTED",
            ));
        }
        let constructors = class
            .methods
            .iter()
            .filter(|m| m.name == "<init>" && m.access & 1 != 0 && !m.unsupported)
            .cloned()
            .collect::<Vec<_>>();
        let method = self.select_method(&constructors, args, offset)?;
        let normalized = self.vararg_arguments(&method, args, offset)?;
        let (params, defaults) = Self::method_parameters(&method)?;
        self.constructor_call(
            &class.name,
            &params,
            &defaults,
            &normalized,
            self.method_named(&method),
            offset,
        )
        .map(Some)
    }
}
