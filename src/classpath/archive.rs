use super::{Diagnostic, Result};
use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

const MAX_ENTRY: u64 = 64 * 1024 * 1024;

fn error(path: &Path, error: impl std::fmt::Display) -> Diagnostic {
    Diagnostic::new(0, format!("Classpath {}: {error}", path.display()))
}

fn open(path: &Path) -> Result<zip::ZipArchive<BufReader<fs::File>>> {
    zip::ZipArchive::new(BufReader::new(
        fs::File::open(path).map_err(|e| error(path, e))?,
    ))
    .map_err(|e| error(path, e))
}

fn module_descriptor(name: &str) -> bool {
    name.rsplit('/').next() == Some("module-info.class")
}

// The pinned Kotlin compiler resolves base entries even for newer JVM targets.
fn versioned_entry(name: &str) -> bool {
    name.starts_with("META-INF/versions/")
}

fn valid_entry_name(name: &str) -> bool {
    let name = name.strip_suffix('/').unwrap_or(name);
    !name.is_empty()
        && !name.contains(['\\', '\0'])
        && name.as_bytes().get(1) != Some(&b':')
        && name.split('/').all(|part| !matches!(part, "" | "." | ".."))
}

// Keep the central directory and open reader for the lifetime of a classpath.
// Missing-class probes must not reopen and rescan every dependency archive.
#[derive(Default)]
pub(super) struct Index {
    archives: HashMap<PathBuf, IndexedArchive>,
}

struct IndexedArchive {
    reader: zip::ZipArchive<BufReader<fs::File>>,
    entries: HashMap<String, usize>,
    packages: HashMap<String, Vec<String>>,
    metadata: Vec<String>,
}

impl IndexedArchive {
    fn open(path: &Path) -> Result<Self> {
        let mut reader = open(path)?;
        let mut entries = HashMap::new();
        let mut packages: HashMap<String, Vec<String>> = HashMap::new();
        let mut metadata = Vec::new();
        for i in 0..reader.len() {
            let entry = reader.by_index(i).map_err(|e| error(path, e))?;
            let name = entry.name();
            if !valid_entry_name(name) {
                return Err(error(path, "malformed ZIP entry name"));
            }
            if entries.insert(name.to_owned(), i).is_some() {
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
            if !entry.is_dir() && !module_descriptor(name) && !versioned_entry(name) {
                let package = name.rsplit_once('/').map_or("", |(package, _)| package);
                packages
                    .entry(package.into())
                    .or_default()
                    .push(name.into());
                if name.starts_with("META-INF/") && name.ends_with(".kotlin_module") {
                    metadata.push(name.into());
                }
            }
        }
        for names in packages.values_mut() {
            names.sort();
        }
        metadata.sort();
        Ok(Self {
            reader,
            entries,
            packages,
            metadata,
        })
    }

    fn read(&mut self, path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(&index) = self.entries.get(name) else {
            return Ok(None);
        };
        let mut entry = self.reader.by_index(index).map_err(|e| error(path, e))?;
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

    fn names(&self, package: &str) -> Vec<String> {
        let mut names = self.packages.get(package).cloned().unwrap_or_default();
        names.extend(self.metadata.iter().cloned());
        names.sort();
        names.dedup();
        names
    }
}

impl Index {
    pub(super) fn new(paths: &[PathBuf]) -> Result<Self> {
        let mut index = Self::default();
        for path in paths {
            if !path.is_dir() {
                if !path.is_file() {
                    return Err(error(
                        path,
                        "must be an existing directory or JAR/ZIP archive",
                    ));
                }
                index.archive(path)?;
            }
        }
        Ok(index)
    }

    fn archive(&mut self, path: &Path) -> Result<&mut IndexedArchive> {
        if !self.archives.contains_key(path) {
            self.archives
                .insert(path.into(), IndexedArchive::open(path)?);
        }
        Ok(self.archives.get_mut(path).unwrap())
    }

    pub(super) fn read(&mut self, path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
        if module_descriptor(name) || versioned_entry(name) {
            return Ok(None);
        }
        if let Some(archive) = self.archives.get_mut(path) {
            return archive.read(path, name);
        }
        if !path.is_dir() {
            return self.archive(path)?.read(path, name);
        }
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
        Ok(Some(bytes))
    }

    pub(super) fn names(&mut self, path: &Path, package: &str) -> Result<Vec<String>> {
        if let Some(archive) = self.archives.get(path) {
            return Ok(archive.names(package));
        }
        if !path.is_dir() {
            return Ok(self.archive(path)?.names(package));
        }
        let mut names = Vec::new();
        collect(path, package, false, &mut names)?;
        let mut metadata = Vec::new();
        collect(path, "META-INF", true, &mut metadata)?;
        names.extend(
            metadata
                .into_iter()
                .filter(|name| name.ends_with(".kotlin_module")),
        );
        names.retain(|name| !module_descriptor(name) && !versioned_entry(name));
        names.sort();
        names.dedup();
        Ok(names)
    }
}

#[cfg(test)]
pub(super) fn validate(paths: &[PathBuf]) -> Result<()> {
    Index::new(paths).map(|_| ())
}

#[cfg(test)]
pub(super) fn read(path: &Path, name: &str) -> Result<Option<Vec<u8>>> {
    Index::default().read(path, name)
}

#[cfg(test)]
fn names(path: &Path, package: &str) -> Result<Vec<String>> {
    Index::default().names(path, package)
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
            if self.0.is_dir() {
                let _ = fs::remove_dir_all(&self.0);
            } else {
                let _ = fs::remove_file(&self.0);
            }
        }
    }
    fn directory(entries: &[(&str, &[u8])]) -> Temp {
        let temp = Temp::new(&[]);
        fs::remove_file(&temp.0).unwrap();
        fs::create_dir(&temp.0).unwrap();
        for (name, bytes) in entries {
            let path = temp.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        temp
    }
    fn entries(entries: &[(&str, &[u8])]) -> Temp {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in entries {
            writer
                .start_file(*name, zip::write::FileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        Temp::new(&writer.finish().unwrap().into_inner())
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
    fn archive_reads_preserve_payloads_across_buffer_boundaries_and_seeks() {
        let mut state = 17u32;
        let payload = (0..32_769)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state as u8
            })
            .collect::<Vec<_>>();
        for method in [
            zip::CompressionMethod::Stored,
            zip::CompressionMethod::Deflated,
        ] {
            let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
            for name in ["p/First.class", "p/Middle.class", "p/Last.class"] {
                writer
                    .start_file(
                        name,
                        zip::write::FileOptions::default().compression_method(method),
                    )
                    .unwrap();
                writer.write_all(&payload).unwrap();
            }
            let file = Temp::new(&writer.finish().unwrap().into_inner());
            let mut index = Index::new(std::slice::from_ref(&file.0)).unwrap();
            for name in [
                "p/Last.class",
                "p/First.class",
                "p/Middle.class",
                "p/First.class",
            ] {
                assert_eq!(index.read(&file.0, name).unwrap().unwrap(), payload);
            }
            assert!(index.read(&file.0, "p/Missing.class").unwrap().is_none());
            assert_eq!(
                index.names(&file.0, "p").unwrap(),
                ["p/First.class", "p/Last.class", "p/Middle.class"]
            );
        }
    }
    #[test]
    fn indexed_archives_reuse_validated_entries_for_reads_and_package_names() {
        let file = entries(&[
            ("p/A.class", b"first"),
            ("q/B.class", b"second"),
            ("META-INF/library.kotlin_module", b"metadata"),
            ("META-INF/versions/21/p/A.class", b"override"),
            ("module-info.class", b"module"),
        ]);
        let mut index = Index::new(&[file.0.clone(), file.0.clone()]).unwrap();
        assert_eq!(index.archives.len(), 1);
        // Removing the path proves that both hits and misses use the same
        // validated central directory and reader rather than reopening it.
        fs::remove_file(&file.0).unwrap();
        for _ in 0..3 {
            assert_eq!(
                index.read(&file.0, "q/B.class").unwrap().unwrap(),
                b"second"
            );
            assert_eq!(index.read(&file.0, "p/A.class").unwrap().unwrap(), b"first");
            assert!(index.read(&file.0, "p/Missing.class").unwrap().is_none());
            assert!(index.read(&file.0, "module-info.class").unwrap().is_none());
            assert!(
                index
                    .read(&file.0, "META-INF/versions/21/p/A.class")
                    .unwrap()
                    .is_none()
            );
            assert_eq!(
                index.names(&file.0, "p").unwrap(),
                ["META-INF/library.kotlin_module", "p/A.class"]
            );
            assert_eq!(
                index.names(&file.0, "missing").unwrap(),
                ["META-INF/library.kotlin_module"]
            );
            assert_eq!(
                index.names(&file.0, "META-INF").unwrap(),
                ["META-INF/library.kotlin_module"]
            );
        }
    }

    #[test]
    fn directory_resources_stay_live_with_an_archive_index() {
        let directory = directory(&[("p/A.class", b"original")]);
        let mut index = Index::new(std::slice::from_ref(&directory.0)).unwrap();
        assert!(index.archives.is_empty());
        assert_eq!(
            index.read(&directory.0, "p/A.class").unwrap().unwrap(),
            b"original"
        );
        fs::write(directory.0.join("p/A.class"), b"changed").unwrap();
        fs::write(directory.0.join("p/B.class"), b"added").unwrap();
        assert_eq!(
            index.read(&directory.0, "p/A.class").unwrap().unwrap(),
            b"changed"
        );
        assert_eq!(
            index.names(&directory.0, "p").unwrap(),
            ["p/A.class", "p/B.class"]
        );
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
    fn metadata_and_module_descriptors_follow_the_same_directory_and_archive_policy() {
        let contents: &[(&str, &[u8])] = &[
            ("p/A.class", b"class data"),
            ("META-INF/library.kotlin_module", b"metadata"),
            ("META-INF/resource.class", b"resource"),
            ("module-info.class", b"descriptor"),
            ("META-INF/versions/9/module-info.class", b"descriptor"),
        ];
        for path in [directory(contents), entries(contents)] {
            validate(std::slice::from_ref(&path.0)).unwrap();
            assert_eq!(
                names(&path.0, "p").unwrap(),
                ["META-INF/library.kotlin_module", "p/A.class"]
            );
            assert_eq!(
                names(&path.0, "").unwrap(),
                ["META-INF/library.kotlin_module"]
            );
            assert_eq!(
                read(&path.0, "META-INF/library.kotlin_module")
                    .unwrap()
                    .unwrap(),
                b"metadata"
            );
            assert!(read(&path.0, "module-info.class").unwrap().is_none());
            assert!(
                read(&path.0, "META-INF/versions/9/module-info.class")
                    .unwrap()
                    .is_none()
            );
            assert!(read(&path.0, "p/Missing.class").unwrap().is_none());
        }
    }
    #[test]
    fn multi_release_entries_preserve_the_base_view() {
        for manifest in [
            b"Manifest-Version: 1.0\nMulti-Release: true\n\n".as_slice(),
            b"Manifest-Version: 1.0\nMulti-Release: false\n\n".as_slice(),
            b"".as_slice(),
        ] {
            let contents: &[(&str, &[u8])] = &[
                ("p/A.class", b"base"),
                ("META-INF/base.kotlin_module", b"base metadata"),
                ("META-INF/MANIFEST.MF", manifest),
                ("META-INF/versions/9/p/A.class", b"override"),
                ("META-INF/versions/21/p/Only.class", b"version only"),
                ("META-INF/versions/99/p/A.class", b"future override"),
                (
                    "META-INF/versions/9/META-INF/base.kotlin_module",
                    b"override metadata",
                ),
            ];
            for path in [directory(contents), entries(contents)] {
                validate(std::slice::from_ref(&path.0)).unwrap();
                assert_eq!(read(&path.0, "p/A.class").unwrap().unwrap(), b"base");
                assert!(read(&path.0, "p/Only.class").unwrap().is_none());
                assert!(
                    read(&path.0, "META-INF/versions/9/p/A.class")
                        .unwrap()
                        .is_none()
                );
                assert_eq!(
                    names(&path.0, "p").unwrap(),
                    ["META-INF/base.kotlin_module", "p/A.class"]
                );
                assert_eq!(
                    names(&path.0, "META-INF/versions/9/p").unwrap(),
                    ["META-INF/base.kotlin_module"]
                );
                assert_eq!(
                    read(&path.0, "META-INF/base.kotlin_module")
                        .unwrap()
                        .unwrap(),
                    b"base metadata"
                );
            }
        }
    }
    #[test]
    fn malformed_archive_names_are_diagnosed() {
        for name in [
            "../A.class",
            "/A.class",
            "p/./A.class",
            "p//A.class",
            "p/../A.class",
            "p\\A.class",
            "C:/A.class",
        ] {
            let path = entries(&[(name, b"data")]);
            let diagnostic = validate(std::slice::from_ref(&path.0)).unwrap_err();
            assert!(
                diagnostic.message.contains("malformed ZIP entry name"),
                "{name}: {}",
                diagnostic.message
            );
        }
    }
    #[test]
    fn corrupt_entry_payload_is_diagnosed_when_read() {
        let mut bytes = zip(zip::CompressionMethod::Stored);
        let payload = bytes
            .windows(10)
            .position(|part| part == b"class data")
            .unwrap();
        bytes[payload] ^= 1;
        let path = Temp::new(&bytes);
        assert!(read(&path.0, "p/A.class").is_err());
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
