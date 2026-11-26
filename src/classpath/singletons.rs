use super::{Classpath, JavaField, Result};

impl Classpath {
    pub(crate) fn singleton_field(&self, owner: &str) -> Result<Option<JavaField>> {
        let class = self.load(owner)?;
        if !class.kotlin_class {
            return Ok(None);
        }
        let descriptor = format!("L{owner};");
        if let Some(field) = class.fields.iter().find(|field| {
            self.singletons.borrow().contains(owner)
                && field.name == "INSTANCE"
                && field.access & 8 != 0
                && field.descriptor == descriptor
        }) {
            return Ok(Some(field.clone()));
        }
        let Some((parent, _)) = owner.rsplit_once('$') else {
            return Ok(None);
        };
        let parent = self.load(parent)?;
        let Some(name) = parent
            .library
            .as_ref()
            .and_then(|library| library.companion_name.as_ref())
        else {
            return Ok(None);
        };
        Ok(parent
            .fields
            .iter()
            .find(|field| {
                field.name == *name && field.access & 8 != 0 && field.descriptor == descriptor
            })
            .cloned())
    }
}
