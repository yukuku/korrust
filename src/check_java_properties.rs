use super::*;

fn property_name(method: &str) -> Option<String> {
    if let Some(suffix) = method.strip_prefix("get") {
        let first = suffix.as_bytes().first()?;
        if first.is_ascii_lowercase() {
            return None;
        }
        let mut bytes = suffix.as_bytes().to_vec();
        if !bytes[0].is_ascii_uppercase() {
            return Some(suffix.into());
        }
        let end = bytes
            .iter()
            .position(|value| !value.is_ascii_uppercase())
            .unwrap_or(bytes.len());
        let end = if end > 1 && end < bytes.len() {
            end - 1
        } else {
            end
        };
        for byte in &mut bytes[..end] {
            byte.make_ascii_lowercase();
        }
        return String::from_utf8(bytes).ok();
    }
    let suffix = method.strip_prefix("is")?;
    (!suffix.as_bytes().first()?.is_ascii_lowercase()).then(|| method.into())
}

fn java_origin(
    classpath: &Classpath,
    owner: &str,
    method: &str,
    seen: &mut HashSet<String>,
) -> Result<bool> {
    if !seen.insert(owner.into()) {
        return Ok(false);
    }
    if seen.len() > 256 {
        return Err(Diagnostic::new(
            0,
            "Java property origin hierarchy exceeds the supported limit",
        ));
    }
    let class = classpath.load(owner)?;
    if !class.kotlin_class
        && class.methods.iter().any(|candidate| {
            candidate.name == method
                && candidate.descriptor.starts_with("()")
                && candidate.access & 8 == 0
        })
    {
        return Ok(true);
    }
    for parent in class.super_name.iter().chain(&class.interfaces) {
        if parent != "java/lang/Object" && java_origin(classpath, parent, method, seen)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn accessibility_bridge(
    classpath: &Classpath,
    owner: &str,
    method: &JavaMethod,
    seen: &mut HashSet<String>,
) -> Result<Option<JavaMethod>> {
    if !seen.insert(owner.into()) || seen.len() > 256 {
        return Ok(None);
    }
    let class = classpath.load(owner)?;
    for parent in class.super_name.iter().chain(&class.interfaces) {
        let declaring = classpath.load(parent)?;
        if !declaring.kotlin_class && declaring.access & 1 == 0 {
            if let Some(candidate) = declaring.methods.iter().find(|candidate| {
                candidate.name == method.name
                    && candidate.descriptor == method.descriptor
                    && candidate.access & 9 == 1
                    && !candidate.unsupported
            }) {
                return Ok(Some(candidate.clone()));
            }
        }
        if let Some(candidate) = accessibility_bridge(classpath, parent, method, seen)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

impl Checker<'_> {
    pub(super) fn java_property_getter(
        &self,
        receiver: &Type,
        name: &str,
        offset: usize,
    ) -> Result<Option<JavaMethod>> {
        if !matches!(receiver.base(), Type::Java(_) | Type::Parameterized { .. }) {
            return Ok(None);
        }
        let Some(owner) = library::type_owner(receiver) else {
            return Ok(None);
        };
        let class = self.classpath.load(owner)?;
        let mut getters = Vec::new();
        let mut unsupported_getter = false;
        let methods = self.classpath.methods(owner)?;
        for method in &methods {
            if method.access & 8 != 0
                || method.access & 1 == 0
                || method.access & 0x1000 != 0 && method.access & 0x40 == 0
                || property_name(&method.name).as_deref() != Some(name)
                || method
                    .library
                    .as_ref()
                    .is_some_and(|library| !library.type_parameters.is_empty())
            {
                continue;
            }
            let method = if method.access & 0x40 != 0 {
                if methods.iter().any(|candidate| {
                    candidate.name == method.name
                        && candidate.access & 0x1040 == 0
                        && candidate.descriptor.starts_with("()")
                }) {
                    continue;
                }
                let Some(declaration) =
                    accessibility_bridge(self.classpath, owner, method, &mut HashSet::new())?
                else {
                    continue;
                };
                declaration
            } else {
                method.clone()
            };
            if !java_origin(
                self.classpath,
                &method.owner,
                &method.name,
                &mut HashSet::new(),
            )? {
                continue;
            }
            if method.unsupported
                && method.descriptor.starts_with("()")
                && method.descriptor != "()V"
            {
                unsupported_getter = true;
                continue;
            }
            let method =
                library::specialize_receiver(method.clone(), receiver, &class, self.classpath);
            let (parameters, result) = class_method_types(&method)?;
            if parameters.is_empty() && result != Type::Unit && !method.unsupported {
                if class.kotlin_class
                    || class.access & 0x4200 != 0
                    || crate::classpath::kotlin_mapped_classifier(&class.name).is_some()
                    || crate::classpath::kotlin_mapped_classifier(&method.owner).is_some()
                {
                    return Err(Diagnostic::new(offset, "Synthetic properties on mapped, interface, enum, or Kotlin receiver classes are unsupported").with_kind(crate::DiagnosticKind::UnsupportedLanguage, "JAVA_SYNTHETIC_RECEIVER_UNSUPPORTED"));
                }
                getters.push(method);
            }
        }
        if unsupported_getter {
            return Err(Diagnostic::new(
                offset,
                "Java property getter has an unsupported declaration or type signature",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "JAVA_PROPERTY_DECLARATION_UNSUPPORTED",
            ));
        }
        match getters.len() {
            0 => Ok(None),
            1 => Ok(getters.pop()),
            _ => Err(
                Diagnostic::new(offset, "Ambiguous Java synthetic property getter").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "AMBIGUOUS_JAVA_PROPERTY",
                ),
            ),
        }
    }

    pub(super) fn java_property_setter(
        &self,
        receiver: &Type,
        name: &str,
        offset: usize,
    ) -> Result<Option<JavaMethod>> {
        let Some(getter) = self.java_property_getter(receiver, name, offset)? else {
            return Ok(None);
        };
        let owner = library::type_owner(receiver).unwrap();
        let class = self.classpath.load(owner)?;
        let suffix = getter
            .name
            .strip_prefix("get")
            .or_else(|| getter.name.strip_prefix("is"))
            .unwrap();
        let setter = format!("set{suffix}");
        let property_type = class_method_types(&getter)?.1;
        let mut setters = Vec::new();
        for method in self.classpath.methods(owner)? {
            if method.name != setter
                || method.access & 8 != 0
                || method.access & 1 == 0
                || method.access & 0x10c0 != 0
                || method
                    .library
                    .as_ref()
                    .is_some_and(|library| !library.type_parameters.is_empty())
                || relations::projected_input(&method, receiver, &class, self.classpath)
            {
                continue;
            }
            let method = library::specialize_receiver(method, receiver, &class, self.classpath);
            let (parameters, _) = class_method_types(&method)?;
            if parameters == [property_type.clone()] && !method.unsupported {
                setters.push(method);
            }
        }
        match setters.len() {
            0 => Ok(None),
            1 => Ok(setters.pop()),
            _ => Err(
                Diagnostic::new(offset, "Ambiguous Java synthetic property setter").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "AMBIGUOUS_JAVA_PROPERTY",
                ),
            ),
        }
    }
}
