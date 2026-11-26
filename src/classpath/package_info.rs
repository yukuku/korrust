use super::{Classpath, Result, invalid, parse_details_impl, signature};

impl Classpath {
    pub(super) fn passive_package_info(&self, bytes: &[u8], resource: &str) -> Result<bool> {
        let owner = resource.strip_suffix(".class").ok_or_else(invalid)?;
        let (class, attributes, _, _) = parse_details_impl(bytes, owner, false)?;
        if class.access & 0x200 == 0
            || !class.fields.is_empty()
            || !class.methods.is_empty()
            || attributes.unsupported
            || !attributes.type_annotation_targets.is_empty()
        {
            return Ok(false);
        }
        for annotation in attributes.annotations {
            if signature::type_semantic_annotation(&annotation.name) {
                return Ok(false);
            }
            let owner = annotation
                .name
                .strip_prefix('L')
                .and_then(|name| name.strip_suffix(';'))
                .ok_or_else(invalid)?;
            let resource = format!("{owner}.class");
            let mut definition = None;
            for path in &self.paths {
                if let Some(bytes) = self.read_resource(path, &resource)? {
                    definition = Some(bytes);
                    break;
                }
            }
            let Some(definition) = definition else {
                return Ok(false);
            };
            let (class, meta, _, _) = parse_details_impl(&definition, owner, false)?;
            if class.access & 0x2000 == 0
                || meta.unsupported
                || meta.annotations.iter().any(|a| {
                    !matches!(
                        a.name.as_str(),
                        "Ljava/lang/annotation/Target;"
                            | "Ljava/lang/annotation/Retention;"
                            | "Ljava/lang/annotation/Documented;"
                            | "Ljava/lang/annotation/Inherited;"
                    )
                })
            {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
