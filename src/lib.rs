mod bytecode;
mod check;
mod classpath;
mod metadata;

use std::path::PathBuf;
mod syntax;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub offset: usize,
    pub message: String,
}

impl Diagnostic {
    fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
        }
    }

    pub fn render(&self, path: &str, source: &str) -> String {
        let prefix = &source[..self.offset.min(source.len())];
        let line = prefix.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        format!("{path}:{line}:{column}: error: {}", self.message)
    }
}

type Result<T> = std::result::Result<T, Diagnostic>;

pub struct ClassFile {
    pub internal_name: String,
    pub bytes: Vec<u8>,
}

pub struct OutputFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

pub fn compile_outputs_with_classpath(
    source: &str,
    file_name: &str,
    classpath: &[PathBuf],
) -> Result<Vec<OutputFile>> {
    let classes = compile_all_with_classpath(source, file_name, classpath)?;
    let module = syntax::parse(source)?;
    let mapping = if module.functions.is_empty() {
        None
    } else {
        let facade = &classes[0].internal_name;
        let hash = facade
            .bytes()
            .fold(0x6c62272e07bb014262b821756295c58du128, |hash, byte| {
                (hash ^ u128::from(byte)).wrapping_mul(0x1000000000000000000013b)
            });
        Some(OutputFile {
            relative_path: format!("META-INF/korrust_{hash:032x}.kotlin_module"),
            bytes: metadata::module_mapping(std::slice::from_ref(facade)),
        })
    };
    let mut outputs = classes
        .into_iter()
        .map(|class| OutputFile {
            relative_path: format!("{}.class", class.internal_name),
            bytes: class.bytes,
        })
        .collect::<Vec<_>>();
    outputs.extend(mapping);
    Ok(outputs)
}

pub fn compile(source: &str, file_name: &str) -> Result<ClassFile> {
    compile_with_classpath(source, file_name, &[])
}

pub fn compile_with_classpath(
    source: &str,
    file_name: &str,
    classpath: &[PathBuf],
) -> Result<ClassFile> {
    let mut classes = compile_all_with_classpath(source, file_name, classpath)?;
    if classes.len() != 1 {
        return Err(Diagnostic::new(
            0,
            "Source produces multiple classes; use compile_all or compile_all_with_classpath",
        ));
    }
    Ok(classes.remove(0))
}

pub fn compile_all(source: &str, file_name: &str) -> Result<Vec<ClassFile>> {
    compile_all_with_classpath(source, file_name, &[])
}

pub fn compile_all_with_classpath(
    source: &str,
    file_name: &str,
    classpath: &[PathBuf],
) -> Result<Vec<ClassFile>> {
    let stem = file_name
        .strip_suffix(".kt")
        .ok_or_else(|| Diagnostic::new(0, "Expected a .kt filename"))?;
    if stem.is_empty()
        || !stem
            .bytes()
            .enumerate()
            .all(|(i, b)| b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit()))
    {
        return Err(Diagnostic::new(
            0,
            "Unsupported filename: use an ASCII Kotlin identifier followed by .kt",
        ));
    }
    if source.len() > 1_048_576 {
        return Err(Diagnostic::new(0, "Source exceeds the current 1 MiB limit"));
    }
    let module = syntax::parse(source)?;
    let classpath = classpath::Classpath::new(classpath)?;
    let typed = check::check(&module, &classpath)?;
    let class_name = module
        .enum_decl
        .as_ref()
        .map(|declaration| &declaration.name)
        .or_else(|| {
            module
                .object_decl
                .as_ref()
                .map(|declaration| &declaration.name)
        })
        .or_else(|| {
            module
                .class_decl
                .as_ref()
                .map(|declaration| &declaration.name)
        })
        .or_else(|| {
            module
                .interface_decl
                .as_ref()
                .map(|declaration| &declaration.name)
        })
        .cloned()
        .unwrap_or_else(|| format!("{}{}Kt", stem[..1].to_ascii_uppercase(), &stem[1..]));
    let internal_name = if module.package.is_empty() {
        class_name
    } else {
        format!("{}/{}", module.package.replace('.', "/"), class_name)
    };
    if module.interface_decl.is_some() {
        let declaration = typed
            .interface_decl
            .as_ref()
            .ok_or_else(|| Diagnostic::new(0, "Interface checking did not produce an interface"))?;
        return bytecode::emit_interfaces(
            declaration,
            file_name,
            &metadata::for_interfaces(&module, &internal_name)?,
        );
    }
    if let Some(declaration) = &typed.object_decl {
        let field_types = declaration
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.ty.clone()))
            .collect::<Vec<_>>();
        return bytecode::emit_dynamic_object(
            declaration,
            &internal_name,
            file_name,
            &metadata::for_object(
                &module,
                &internal_name,
                &field_types,
                &declaration.interfaces,
                declaration.serializable,
            )?,
        );
    }
    if let Some(declaration) = &module.object_decl {
        return bytecode::emit_objects(declaration, &internal_name, file_name);
    }
    if let Some(source_declaration) = &module.class_decl {
        let declaration = typed
            .class_decl
            .as_ref()
            .ok_or_else(|| Diagnostic::new(0, "Class checking did not produce a class"))?;
        return Ok(vec![ClassFile {
            internal_name: internal_name.clone(),
            bytes: bytecode::emit_class(
                declaration,
                &internal_name,
                file_name,
                &metadata::for_class(source_declaration, &internal_name, &module.imports)?,
            )?,
        }]);
    }
    let bytes = match &module.enum_decl {
        Some(declaration) => bytecode::emit_enum(declaration, &internal_name, file_name)?,
        None => bytecode::emit(
            &typed,
            &internal_name,
            file_name,
            &metadata::for_package(&module)?,
        )?,
    };
    Ok(vec![ClassFile {
        internal_name,
        bytes,
    }])
}
