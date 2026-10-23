use std::{fs, path::PathBuf};

use crate::{Diagnostic, Result};

#[derive(Clone)]
pub(crate) struct Classpath {
    paths: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub(crate) struct JavaClass {
    pub name: String,
    pub access: u16,
    pub super_name: Option<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<JavaMethod>,
    pub unsupported: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct JavaMethod {
    pub owner: String,
    pub name: String,
    pub descriptor: String,
    pub access: u16,
    pub unsupported: bool,
}

impl Classpath {
    pub fn new(paths: &[PathBuf]) -> Result<Self> {
        for path in paths {
            if !path.is_dir() {
                return Err(Diagnostic::new(
                    0,
                    format!(
                        "Classpath entry {} must be an existing directory; JAR and ZIP classpaths are unsupported",
                        path.display()
                    ),
                ));
            }
        }
        Ok(Self {
            paths: paths.to_vec(),
        })
    }

    pub fn load(&self, internal_name: &str) -> Result<JavaClass> {
        if !valid_name(internal_name) {
            return Err(Diagnostic::new(0, "Invalid Java class internal name"));
        }
        let package_info = internal_name
            .rsplit_once('/')
            .map(|(package, _)| format!("{package}/package-info.class"))
            .unwrap_or_else(|| "package-info.class".into());
        for directory in &self.paths {
            let path = directory.join(&package_info);
            match fs::metadata(&path) {
                Ok(_) => {
                    return Err(Diagnostic::new(
                        0,
                        format!(
                            "Java package metadata {} is unsupported because it may define nullability defaults",
                            path.display()
                        ),
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(Diagnostic::new(0, format!("{}: {error}", path.display())));
                }
            }
        }
        for directory in &self.paths {
            let path = directory.join(format!("{internal_name}.class"));
            match fs::read(&path) {
                Ok(bytes) => {
                    return parse(&bytes, internal_name).map_err(|mut error| {
                        error.message = format!("{}: {}", path.display(), error.message);
                        error
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(Diagnostic::new(0, format!("{}: {error}", path.display())));
                }
            }
        }
        Err(Diagnostic::new(
            0,
            format!(
                "Java class {} was not found on the classpath",
                internal_name.replace('/', ".")
            ),
        ))
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && !part
                    .chars()
                    .any(|c| matches!(c, '.' | ';' | '[' | '\\' | ':' | '\0'))
        })
}

fn invalid() -> Diagnostic {
    Diagnostic::new(0, "Malformed or unsupported Java class file")
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let end = self.offset.checked_add(count).ok_or_else(invalid)?;
        let bytes = self.bytes.get(self.offset..end).ok_or_else(invalid)?;
        self.offset = end;
        Ok(bytes)
    }
    fn u1(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u2(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(
            self.take(2)?.try_into().map_err(|_| invalid())?,
        ))
    }
    fn u4(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(
            self.take(4)?.try_into().map_err(|_| invalid())?,
        ))
    }
}

enum Constant {
    Empty,
    Utf8(String),
    Class(u16),
}
fn utf8(pool: &[Constant], index: u16) -> Result<&str> {
    match pool.get(usize::from(index)) {
        Some(Constant::Utf8(value)) => Ok(value),
        _ => Err(invalid()),
    }
}
fn class_name(pool: &[Constant], index: u16) -> Result<&str> {
    match pool.get(usize::from(index)) {
        Some(Constant::Class(value)) => utf8(pool, *value),
        _ => Err(invalid()),
    }
}

fn modified_utf8(bytes: &[u8]) -> Result<String> {
    let mut units = Vec::new();
    let mut reader = Reader { bytes, offset: 0 };
    while reader.offset < bytes.len() {
        let first = reader.u1()?;
        let unit = if (1..128).contains(&first) {
            u16::from(first)
        } else if first & 0xe0 == 0xc0 {
            let second = reader.u1()?;
            if second & 0xc0 != 0x80 {
                return Err(invalid());
            }
            let value = (u16::from(first & 31) << 6) | u16::from(second & 63);
            if value < 128 && !(first == 0xc0 && second == 0x80) {
                return Err(invalid());
            }
            value
        } else if first & 0xf0 == 0xe0 {
            let second = reader.u1()?;
            let third = reader.u1()?;
            if second & 0xc0 != 0x80 || third & 0xc0 != 0x80 {
                return Err(invalid());
            }
            let value = (u16::from(first & 15) << 12)
                | (u16::from(second & 63) << 6)
                | u16::from(third & 63);
            if value < 2048 {
                return Err(invalid());
            }
            value
        } else {
            return Err(invalid());
        };
        units.push(unit);
    }
    Ok(String::from_utf16_lossy(&units))
}

fn attributes(reader: &mut Reader<'_>, pool: &[Constant]) -> Result<bool> {
    let mut unsupported = false;
    for _ in 0..reader.u2()? {
        let name = utf8(pool, reader.u2()?)?;
        let length = usize::try_from(reader.u4()?).map_err(|_| invalid())?;
        reader.take(length)?;
        unsupported |= name == "Signature" || name.contains("Annotations");
    }
    Ok(unsupported)
}

fn parse(bytes: &[u8], expected: &str) -> Result<JavaClass> {
    let mut reader = Reader { bytes, offset: 0 };
    if reader.u4()? != 0xcafebabe {
        return Err(invalid());
    }
    reader.u2()?;
    if reader.u2()? < 45 {
        return Err(invalid());
    }
    let count = usize::from(reader.u2()?);
    if count == 0 {
        return Err(invalid());
    }
    let mut pool = vec![Constant::Empty];
    while pool.len() < count {
        let tag = reader.u1()?;
        let constant = match tag {
            1 => {
                let length = usize::from(reader.u2()?);
                Constant::Utf8(modified_utf8(reader.take(length)?)?)
            }
            7 => Constant::Class(reader.u2()?),
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => {
                reader.take(4)?;
                Constant::Empty
            }
            5 | 6 => {
                reader.take(8)?;
                pool.push(Constant::Empty);
                if pool.len() >= count {
                    return Err(invalid());
                }
                Constant::Empty
            }
            8 | 16 | 19 | 20 => {
                reader.take(2)?;
                Constant::Empty
            }
            15 => {
                reader.take(3)?;
                Constant::Empty
            }
            _ => return Err(invalid()),
        };
        pool.push(constant);
    }
    let access = reader.u2()?;
    let name = class_name(&pool, reader.u2()?)?.to_owned();
    if name != expected || !valid_name(&name) {
        return Err(Diagnostic::new(
            0,
            format!("Java class identity mismatch: expected {expected}, found {name}"),
        ));
    }
    let parent = reader.u2()?;
    let super_name = if parent == 0 {
        None
    } else {
        Some(class_name(&pool, parent)?.to_owned())
    };
    let mut interfaces = Vec::new();
    for _ in 0..reader.u2()? {
        interfaces.push(class_name(&pool, reader.u2()?)?.to_owned());
    }
    if super_name
        .iter()
        .chain(interfaces.iter())
        .any(|name| !valid_name(name))
    {
        return Err(invalid());
    }
    for _ in 0..reader.u2()? {
        reader.u2()?;
        utf8(&pool, reader.u2()?)?;
        utf8(&pool, reader.u2()?)?;
        attributes(&mut reader, &pool)?;
    }
    let mut methods = Vec::new();
    for _ in 0..reader.u2()? {
        let access = reader.u2()?;
        let method_name = utf8(&pool, reader.u2()?)?.to_owned();
        let descriptor = utf8(&pool, reader.u2()?)?.to_owned();
        let unsupported =
            attributes(&mut reader, &pool)? || access & (0x0040 | 0x0080 | 0x1000) != 0;
        methods.push(JavaMethod {
            owner: name.clone(),
            name: method_name,
            descriptor,
            access,
            unsupported,
        });
    }
    let unsupported = attributes(&mut reader, &pool)?
        || pool
            .iter()
            .any(|value| matches!(value, Constant::Utf8(value) if value == "Lkotlin/Metadata;"));
    if reader.offset != bytes.len() {
        return Err(invalid());
    }
    Ok(JavaClass {
        name,
        access,
        super_name,
        interfaces,
        methods,
        unsupported,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn minimal() -> Vec<u8> {
        let mut bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 52, 0, 3, 1, 0, 1, b'A', 7, 0, 1,
        ];
        bytes.extend_from_slice(&[0, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        bytes
    }
    #[test]
    fn truncations_are_diagnostics() {
        let bytes = minimal();
        assert!(parse(&bytes, "A").is_ok());
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end], "A").is_err(), "length {end}");
        }
    }
    #[test]
    fn invalid_identity_and_trailing_data_are_rejected() {
        let mut bytes = minimal();
        assert!(parse(&bytes, "B").is_err());
        bytes.push(0);
        assert!(parse(&bytes, "A").is_err());
    }
    #[test]
    fn oversized_attributes_and_invalid_pool_indices_are_rejected() {
        let mut bytes = minimal();
        let end = bytes.len();
        bytes[end - 2..].copy_from_slice(&[0, 1]);
        bytes.extend_from_slice(&[0, 1, 0xff, 0xff, 0xff, 0xff]);
        assert!(parse(&bytes, "A").is_err());
        let mut bytes = minimal();
        bytes[16] = 9;
        assert!(parse(&bytes, "A").is_err());
    }
    #[test]
    fn ordinary_class_annotations_are_unsupported() {
        let name = b"RuntimeInvisibleAnnotations";
        let mut bytes = minimal();
        bytes[9] = 4;
        let mut constant = vec![1, 0, name.len() as u8];
        constant.extend_from_slice(name);
        bytes.splice(17..17, constant);
        let end = bytes.len();
        bytes[end - 2..].copy_from_slice(&[0, 1]);
        bytes.extend_from_slice(&[0, 3, 0, 0, 0, 2, 0, 0]);
        assert!(parse(&bytes, "A").unwrap().unsupported);
    }
    #[test]
    fn unsafe_names_are_rejected() {
        for name in ["", "../A", "/A", "A//B", "A\\B", "A.class", "A:foo"] {
            assert!(!valid_name(name));
        }
        assert!(valid_name("a/B$Nested"));
    }
    #[test]
    fn modified_utf8_and_double_slots_are_bounded() {
        assert_eq!(modified_utf8(&[0xc0, 0x80]).unwrap(), "\0");
        assert!(modified_utf8(&[0xe0, 0x80]).is_err());
        let bytes = [
            0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 52, 0, 2, 5, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        assert!(parse(&bytes, "A").is_err());
    }
}
