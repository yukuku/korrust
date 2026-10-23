mod bytecode;
mod check;
mod classpath;

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

pub fn compile(source: &str, file_name: &str) -> Result<ClassFile> {
    compile_with_classpath(source, file_name, &[])
}

pub fn compile_with_classpath(
    source: &str,
    file_name: &str,
    classpath: &[PathBuf],
) -> Result<ClassFile> {
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
    let facade = format!("{}{}Kt", stem[..1].to_ascii_uppercase(), &stem[1..]);
    let internal_name = if module.package.is_empty() {
        facade
    } else {
        format!("{}/{}", module.package.replace('.', "/"), facade)
    };
    let bytes = bytecode::emit(&typed, &internal_name, file_name)?;
    Ok(ClassFile {
        internal_name,
        bytes,
    })
}
