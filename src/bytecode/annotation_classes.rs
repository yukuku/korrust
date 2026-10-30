use super::*;
use crate::annotation_declarations::CheckedAnnotation;

pub(crate) fn emit(
    annotation: &CheckedAnnotation,
    owner: &str,
    file: &str,
    metadata: &crate::metadata::Metadata,
) -> Result<crate::ClassFile> {
    let mut class = interfaces::Builder::new(
        owner,
        52,
        0x2601,
        &["java/lang/annotation/Annotation".into()],
    )?;
    for parameter in &annotation.parameters {
        let mut method = Vec::new();
        u2(&mut method, 0x401);
        u2(&mut method, class.pool.utf8(&parameter.name)?);
        u2(
            &mut method,
            class.pool.utf8(&format!("(){}", parameter.descriptor))?,
        );
        let signature = if parameter.descriptor == "Ljava/lang/Class;" {
            Some("()Ljava/lang/Class<*>;")
        } else if parameter.descriptor == "[Ljava/lang/Class;" {
            Some("()[Ljava/lang/Class<*>;")
        } else {
            None
        };
        u2(
            &mut method,
            u16::from(parameter.default.is_some()) + u16::from(signature.is_some()),
        );
        if let Some(value) = &parameter.default {
            let payload = annotations::element(&mut class.pool, value, 0)?;
            u2(&mut method, class.pool.utf8("AnnotationDefault")?);
            u4(&mut method, payload.len() as u32);
            method.extend(payload);
        }
        if let Some(signature) = signature {
            u2(&mut method, class.pool.utf8("Signature")?);
            u4(&mut method, 2);
            u2(&mut method, class.pool.utf8(signature)?);
        }
        class.methods.push(method);
    }
    let mut file = class.finish(file, metadata, &[])?;
    let annotations = annotation_files::ClassAnnotations {
        annotations: annotation.annotations.clone(),
        fields: Vec::new(),
        methods: Vec::new(),
    };
    file.bytes = annotation_files::apply(&file.bytes, &annotations)?;
    Ok(file)
}
