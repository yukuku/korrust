use super::{Diagnostic, Result};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_ENTRY: u64 = 64 * 1024 * 1024;

fn error(path: &Path, error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(0, format!("Classpath {}: {error}", path.display()))
}

fn open(path: &Path) -> Result<zip::ZipArchive<fs::File>> {
    zip::ZipArchive::new(fs::File::open(path).map_err(|e| error(path, e))?)
        .map_err(|e| error(path, e))
}

pub(super) fn validate(paths: &[PathBuf]) -> Result<()> {
    for path in paths {
        if path.is_dir() {
            continue;
        }
        if !path.is_file() {
            return Err(error(
                path,
                "must be an existing directory or JAR/ZIP archive",
            ));
        }
        let mut archive = open(path)?;
        let mut names = std::collections::HashSet::new();
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| error(path, e))?;
            if entry.name().starts_with("META-INF/versions/")
                && entry.name().ends_with(".class")
                && !entry.name().ends_with("/module-info.class")
            {
                return Err(error(
                    path,
                    "multi-release JAR class overrides are unsupported",
                ));
            }
            if !names.insert(entry.name().to_owned()) {
                return Err(error(path, "duplicate ZIP entry names are unsupported"));
            }
            if !matches!(
                entry.compression(),
                zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
            ) {
                return Err(error(path, "unsupported ZIP compression method"));
            }
            if entry.size() > MAX_ENTRY {
                return Err(error(path, "ZIP entry exceeds the 64 MiB limit"));
            }
        }
    }
    Ok(())
}

pub(super) fn read(path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
    if path.is_dir() {
        let file = match fs::File::open(path.join(name)) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(error(path, e)),
        };
        if file.metadata().map_err(|e| error(path, e))?.len() > MAX_ENTRY {
            return Err(error(path, "class exceeds the 64 MiB limit"));
        }
        let mut bytes = Vec::new();
        file.take(MAX_ENTRY + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| error(path, e))?;
        if bytes.len() as u64 > MAX_ENTRY {
            return Err(error(path, "class exceeds the 64 MiB limit"));
        }
        return Ok(Some(bytes));
    }
    let mut archive = open(path)?;
    let mut entry = match archive.by_name(name) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(error(path, e)),
    };
    let size = entry.size();
    if size > MAX_ENTRY {
        return Err(error(path, "ZIP entry exceeds the 64 MiB limit"));
    }
    let mut bytes = Vec::new();
    (&mut entry)
        .take(MAX_ENTRY + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| error(path, e))?;
    if bytes.len() as u64 != size {
        return Err(error(path, "ZIP entry size mismatch"));
    }
    Ok(Some(bytes))
}

fn collect(directory: &Path, prefix: &str, recursive: bool, names: &mut Vec<String>) -> Result<()> {
    let entries = match fs::read_dir(directory.join(prefix)) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(error(directory, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| error(directory, e))?;
        let ty = entry.file_type().map_err(|e| error(directory, e))?;
        let Some(short) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let name = if prefix.is_empty() {
            short
        } else {
            format!("{prefix}/{short}")
        };
        if ty.is_dir() && recursive {
            collect(directory, &name, true, names)?;
        } else if ty.is_file() {
            names.push(name);
        }
    }
    Ok(())
}

pub(super) fn names(path: &Path, package: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    if path.is_dir() {
        collect(path, package, false, &mut result)?;
        collect(path, "META-INF", true, &mut result)?;
    } else {
        let mut archive = open(path)?;
        for i in 0..archive.len() {
            let entry = archive.by_index(i).map_err(|e| error(path, e))?;
            let name = entry.name();
            if name
                .rsplit_once('/')
                .map_or(package.is_empty(), |(p, _)| p == package)
                || name.starts_with("META-INF/") && name.ends_with(".kotlin_module")
            {
                result.push(name.into());
            }
        }
    }
    result.sort();
    result.dedup();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    fn zip(method: zip::CompressionMethod) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "p/A.class",
                zip::write::FileOptions::default().compression_method(method),
            )
            .unwrap();
        writer.write_all(b"class data").unwrap();
        writer.finish().unwrap().into_inner()
    }
    struct Temp(PathBuf);
    impl Temp {
        fn new(bytes: &[u8]) -> Self {
            static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "korrust-archive-{}-{}.jar",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::write(&path, bytes).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }
    #[test]
    fn stored_and_deflated_archives_use_classpath_reader() {
        for method in [
            zip::CompressionMethod::Stored,
            zip::CompressionMethod::Deflated,
        ] {
            let file = Temp::new(&zip(method));
            validate(std::slice::from_ref(&file.0)).unwrap();
            assert_eq!(read(&file.0, "p/A.class").unwrap().unwrap(), b"class data");
            assert!(read(&file.0, "p/Missing.class").unwrap().is_none());
            assert_eq!(names(&file.0, "p").unwrap(), ["p/A.class"]);
        }
    }
    #[test]
    fn duplicate_zip_names_are_diagnosed() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for _ in 0..2 {
            writer
                .start_file("A.class", zip::write::FileOptions::default())
                .unwrap();
            writer.write_all(b"data").unwrap();
        }
        let file = Temp::new(&writer.finish().unwrap().into_inner());
        assert!(
            validate(std::slice::from_ref(&file.0))
                .unwrap_err()
                .message
                .contains("duplicate")
        );
    }
    #[test]
    fn multi_release_class_overrides_are_diagnosed() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(
                "META-INF/versions/9/p/A.class",
                zip::write::FileOptions::default(),
            )
            .unwrap();
        writer.write_all(b"data").unwrap();
        let file = Temp::new(&writer.finish().unwrap().into_inner());
        assert!(
            validate(std::slice::from_ref(&file.0))
                .unwrap_err()
                .message
                .contains("multi-release")
        );
    }
    #[test]
    fn truncated_and_oversized_archives_are_diagnosed() {
        let mut bytes = zip(zip::CompressionMethod::Stored);
        let truncated = Temp::new(&bytes[..bytes.len() / 2]);
        assert!(validate(std::slice::from_ref(&truncated.0)).is_err());
        let central = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
        bytes[central + 24..central + 28].copy_from_slice(&((MAX_ENTRY + 1) as u32).to_le_bytes());
        let oversized = Temp::new(&bytes);
        assert!(
            validate(std::slice::from_ref(&oversized.0))
                .unwrap_err()
                .message
                .contains("64 MiB")
        );
    }
}
