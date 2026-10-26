use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const HEADER: &str = "korrust-output-v1";
const MODULE_HEADER: &str = "korrust-module-v1";
const MAX_INDEX: u64 = 16 * 1024 * 1024;
type Index = BTreeMap<String, (String, u64)>;

fn error(path: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", path.display())
}

fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |value, byte| {
        (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

fn file_hash(path: &Path) -> Result<u64, String> {
    let mut file = File::open(path).map_err(|e| error(path, e))?;
    let mut value = 0xcbf29ce484222325u64;
    let mut buffer = [0; 8192];
    loop {
        let count = file.read(&mut buffer).map_err(|e| error(path, e))?;
        if count == 0 {
            return Ok(value);
        }
        for byte in &buffer[..count] {
            value = (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    }
}

fn encode(value: &str) -> String {
    value.bytes().map(|byte| format!("{byte:02x}")).collect()
}

fn decode(value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.len() > 65536
        || value.len() & 1 != 0
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("Invalid output ownership index encoding".into());
    }
    let bytes: Result<Vec<_>, _> = (0..value.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&value[offset..offset + 2], 16))
        .collect();
    String::from_utf8(bytes.map_err(|_| "Invalid output ownership index encoding")?)
        .map_err(|_| "Invalid output ownership index UTF-8".into())
}

fn valid_output(path: &str) -> bool {
    if let Some(name) = path
        .strip_prefix("META-INF/korrust_")
        .and_then(|p| p.strip_suffix(".kotlin_module"))
    {
        return !name.is_empty()
            && path.len() <= 4096
            && name.len() % 2 == 0
            && name
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    }
    let Some(stem) = path.strip_suffix(".class") else {
        return false;
    };
    path.len() <= 4096
        && stem.split('/').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| !c.is_control() && !".;[]<>:\\".contains(c))
        })
}

fn read_index(path: &Path) -> Result<Index, String> {
    read_index_with_header(path, HEADER)
}

fn read_index_with_header(path: &Path, header: &str) -> Result<Index, String> {
    match fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Index::new()),
        Err(e) => return Err(error(path, e)),
        Ok(meta) if !meta.is_file() || meta.len() > MAX_INDEX => {
            return Err(error(path, "Invalid output ownership index file"));
        }
        Ok(_) => {}
    }
    let content = fs::read_to_string(path).map_err(|e| error(path, e))?;
    let (first, body) = content
        .split_once('\n')
        .ok_or("Invalid output ownership index")?;
    if first != format!("{header}\t{:016x}", hash(body.as_bytes()))
        || !body.ends_with('\n') && !body.is_empty()
    {
        return Err("Invalid output ownership index checksum or format".into());
    }
    let mut index = Index::new();
    for line in body.lines() {
        let columns: Vec<_> = line.split('\t').collect();
        if columns.len() != 3 {
            return Err("Invalid output ownership index record".into());
        }
        let source = decode(columns[0])?;
        let class = decode(columns[1])?;
        let source_path = Path::new(&source);
        if !source_path.is_absolute()
            || source_path
                .components()
                .any(|c| matches!(c, Component::CurDir | Component::ParentDir))
            || !valid_output(&class)
            || columns[2].len() != 16
        {
            return Err("Invalid output ownership index path or hash".into());
        }
        let digest =
            u64::from_str_radix(columns[2], 16).map_err(|_| "Invalid output ownership hash")?;
        if index.insert(class, (source, digest)).is_some() {
            return Err("Duplicate class in output ownership index".into());
        }
    }
    Ok(index)
}

fn serialize(index: &Index) -> Result<Vec<u8>, String> {
    serialize_with_header(index, HEADER)
}

fn serialize_with_header(index: &Index, header: &str) -> Result<Vec<u8>, String> {
    let mut body = String::new();
    for (class, (source, digest)) in index {
        body.push_str(&format!(
            "{}\t{}\t{digest:016x}\n",
            encode(source),
            encode(class)
        ));
    }
    let content = format!("{header}\t{:016x}\n{body}", hash(body.as_bytes())).into_bytes();
    if content.len() as u64 > MAX_INDEX {
        return Err("Output ownership index exceeds 16 MiB limit".into());
    }
    Ok(content)
}

fn physical_destination(path: &Path) -> Result<PathBuf, String> {
    let absolute = std::path::absolute(path).map_err(|e| error(path, e))?;
    if absolute
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err(error(
            path,
            "Parent traversal is unsupported in output paths",
        ));
    }
    let absolute: PathBuf = absolute.components().collect();
    if matches!(fs::symlink_metadata(&absolute), Ok(meta) if meta.file_type().is_symlink()) {
        return Err(error(
            &absolute,
            "Symlinks are unsupported for the output directory",
        ));
    }
    let mut ancestor = absolute.as_path();
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(ancestor) {
            Ok(_) => {
                let mut physical = fs::canonicalize(ancestor).map_err(|e| error(ancestor, e))?;
                if !physical.is_dir() {
                    return Err(error(ancestor, "Expected an output directory"));
                }
                for component in missing.iter().rev() {
                    physical.push(component);
                }
                return Ok(physical);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or("Invalid output directory")?
                        .to_owned(),
                );
                ancestor = ancestor.parent().ok_or("Invalid output directory")?;
            }
            Err(e) => return Err(error(ancestor, e)),
        }
    }
}

fn check_path(path: &Path, leaf_directory: bool) -> Result<bool, String> {
    let mut current = PathBuf::new();
    let components: Vec<_> = path.components().collect();
    if components
        .iter()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(error(
            path,
            "Parent traversal is unsupported in output paths",
        ));
    }
    for (position, component) in components.iter().enumerate() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(error(&current, "Symlinks are unsupported in output paths"));
                }
                if (position + 1 < components.len() || leaf_directory) && !meta.is_dir() {
                    return Err(error(&current, "Expected an output directory"));
                }
                if position + 1 == components.len() && !leaf_directory && !meta.is_file() {
                    return Err(error(&current, "Expected a regular output file"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(error(&current, e)),
        }
    }
    Ok(true)
}

struct Pending {
    lock: PathBuf,
    staged: Vec<PathBuf>,
}

impl Pending {
    fn stage(&mut self, path: PathBuf, bytes: &[u8]) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| error(&path, e))?;
        self.staged.push(path.clone());
        file.write_all(bytes).map_err(|e| error(&path, e))
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        for path in &self.staged {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_file(&self.lock);
    }
}

pub fn write_outputs(
    source: &Path,
    destination: &Path,
    artifacts: Vec<korrust::OutputFile>,
) -> Result<Vec<PathBuf>, String> {
    let source = fs::canonicalize(source).map_err(|e| error(source, e))?;
    let source = source
        .to_str()
        .ok_or("Non-UTF-8 source paths are unsupported")?;
    let mut outputs = BTreeMap::new();
    for artifact in artifacts {
        let path = artifact.relative_path;
        if !valid_output(&path) || outputs.insert(path, artifact.bytes).is_some() {
            return Err("Invalid or duplicate compiler output path".into());
        }
    }
    let destination = physical_destination(destination)?;
    check_path(&destination, true)?;
    let metadata = destination.join(".korrust");
    check_path(&metadata, true)?;
    fs::create_dir_all(&metadata).map_err(|e| error(&metadata, e))?;
    let lock = metadata.join("lock");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| error(&lock, format!("Cannot acquire output lock: {e}")))?;
    let mut pending = Pending {
        lock,
        staged: Vec::new(),
    };
    let index_path = metadata.join("index");
    let mut index = read_index(&index_path)?;
    let modules = read_module_indexes(&metadata)?;
    let mut previous = BTreeSet::new();
    for (class, (owner, digest)) in &index {
        let path = destination.join(class);
        let exists = check_path(&path, false)?;
        if owner == source {
            if !exists || file_hash(&path)? != *digest {
                return Err(error(
                    &path,
                    "Owned output is missing or changed; refusing to overwrite or remove it",
                ));
            }
            previous.insert(class.clone());
        }
    }
    for class in outputs.keys() {
        if modules.values().any(|index| index.contains_key(class)) {
            return Err(error(
                &destination.join(class),
                "Output is owned by a different module",
            ));
        }
        let path = destination.join(class);
        let exists = check_path(&path, false)?;
        match index.get(class) {
            Some((owner, _)) if owner != source => {
                return Err(error(&path, "Output is owned by a different source"));
            }
            None if exists => return Err(error(&path, "Existing output has no owner")),
            _ => {}
        }
    }
    index.retain(|_, (owner, _)| owner != source);
    for (class, bytes) in &outputs {
        index.insert(class.clone(), (source.to_owned(), hash(bytes)));
    }
    let serialized = serialize(&index)?;
    for (position, bytes) in outputs.values().enumerate() {
        pending.stage(metadata.join(format!("class-{position}.next")), bytes)?;
    }
    let next_index = metadata.join("index.next");
    pending.stage(next_index.clone(), &serialized)?;
    for (position, class) in outputs.keys().enumerate() {
        let path = destination.join(class);
        fs::create_dir_all(path.parent().ok_or("Invalid output parent")?)
            .map_err(|e| error(&path, e))?;
        fs::rename(&pending.staged[position], &path).map_err(|e| error(&path, e))?;
    }
    for class in previous.difference(&outputs.keys().cloned().collect()) {
        let path = destination.join(class);
        fs::remove_file(&path).map_err(|e| error(&path, e))?;
    }
    fs::rename(&next_index, &index_path).map_err(|e| error(&index_path, e))?;
    Ok(outputs
        .keys()
        .map(|class| destination.join(class))
        .collect())
}

pub fn valid_module_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
}

fn module_header(name: &str) -> String {
    format!("{MODULE_HEADER}\t{}", encode(name))
}

fn read_module_indexes(metadata: &Path) -> Result<BTreeMap<String, Index>, String> {
    let mut modules = BTreeMap::new();
    let entries = fs::read_dir(metadata).map_err(|e| error(metadata, e))?;
    let mut owners = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|e| error(metadata, e))?;
        let file_name = entry.file_name();
        let Some(encoded) = file_name
            .to_str()
            .and_then(|name| name.strip_prefix("module-"))
            .and_then(|name| name.strip_suffix(".index"))
        else {
            continue;
        };
        let name = decode(encoded)?;
        if !valid_module_name(&name) {
            return Err("Invalid module identity in output ownership index".into());
        }
        let index = read_index_with_header(&entry.path(), &module_header(&name))?;
        for path in index.keys() {
            if !owners.insert(path.clone()) {
                return Err("Output is owned by multiple modules".into());
            }
        }
        modules.insert(name, index);
    }
    Ok(modules)
}

pub fn write_module_outputs(
    module_name: &str,
    destination: &Path,
    artifacts: Vec<korrust::SourceOutput>,
) -> Result<Vec<PathBuf>, String> {
    if !valid_module_name(module_name) {
        return Err("Invalid module name".into());
    }
    let mut outputs = BTreeMap::new();
    for artifact in artifacts {
        let source = artifact.source_identity;
        let path = artifact.output.relative_path;
        if !Path::new(&source).is_absolute()
            || Path::new(&source)
                .components()
                .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
            || !valid_output(&path)
            || outputs
                .insert(path, (source, artifact.output.bytes))
                .is_some()
        {
            return Err("Invalid source identity or duplicate compiler output path".into());
        }
    }
    let destination = physical_destination(destination)?;
    check_path(&destination, true)?;
    let metadata = destination.join(".korrust");
    check_path(&metadata, true)?;
    fs::create_dir_all(&metadata).map_err(|e| error(&metadata, e))?;
    let lock = metadata.join("lock");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| error(&lock, format!("Cannot acquire output lock: {e}")))?;
    let mut pending = Pending {
        lock,
        staged: Vec::new(),
    };
    let legacy = read_index(&metadata.join("index"))?;
    let mut modules = read_module_indexes(&metadata)?;
    let previous = modules.remove(module_name).unwrap_or_default();
    for (relative, (_, digest)) in &previous {
        let path = destination.join(relative);
        if !check_path(&path, false)? || file_hash(&path)? != *digest {
            return Err(error(
                &path,
                "Owned output is missing or changed; refusing to overwrite or remove it",
            ));
        }
    }
    for relative in outputs.keys() {
        let path = destination.join(relative);
        let exists = check_path(&path, false)?;
        if legacy.contains_key(relative) {
            return Err(error(&path, "Output is owned by a different source"));
        }
        if modules.values().any(|index| index.contains_key(relative)) {
            return Err(error(&path, "Output is owned by a different module"));
        }
        if exists && !previous.contains_key(relative) {
            return Err(error(&path, "Existing output has no owner"));
        }
    }
    let index = outputs
        .iter()
        .map(|(path, (source, bytes))| (path.clone(), (source.clone(), hash(bytes))))
        .collect();
    let serialized = serialize_with_header(&index, &module_header(module_name))?;
    for (position, (_, bytes)) in outputs.values().enumerate() {
        pending.stage(metadata.join(format!("class-{position}.next")), bytes)?;
    }
    let index_path = metadata.join(format!("module-{}.index", encode(module_name)));
    let next_index = metadata.join("index.next");
    pending.stage(next_index.clone(), &serialized)?;
    for (position, relative) in outputs.keys().enumerate() {
        let path = destination.join(relative);
        fs::create_dir_all(path.parent().ok_or("Invalid output parent")?)
            .map_err(|e| error(&path, e))?;
        fs::rename(&pending.staged[position], &path).map_err(|e| error(&path, e))?;
    }
    for relative in previous.keys().filter(|path| !outputs.contains_key(*path)) {
        let path = destination.join(relative);
        fs::remove_file(&path).map_err(|e| error(&path, e))?;
    }
    fs::rename(&next_index, &index_path).map_err(|e| error(&index_path, e))?;
    Ok(outputs.keys().map(|path| destination.join(path)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(source: &str, path: &str) -> korrust::SourceOutput {
        korrust::SourceOutput {
            source_identity: source.into(),
            output: korrust::OutputFile {
                relative_path: path.into(),
                bytes: vec![1, 2, 3],
            },
        }
    }

    #[test]
    fn duplicate_batch_outputs_reject_before_destination_creation() {
        let destination = std::env::temp_dir().join(format!(
            "korrust-duplicate-output-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let result = write_module_outputs(
            "sample",
            &destination,
            vec![
                artifact("/sources/First.kt", "Same.class"),
                artifact("/sources/Second.kt", "Same.class"),
            ],
        );
        assert!(result.unwrap_err().contains("duplicate compiler output"));
        assert!(!destination.exists());
    }

    #[test]
    fn unsafe_batch_artifact_paths_reject_before_filesystem_access() {
        for path in [
            "../Bad.class",
            "/Bad.class",
            ".korrust/index",
            "META-INF/../Bad.class",
        ] {
            let result = write_module_outputs(
                "sample",
                Path::new("/"),
                vec![artifact("/sources/First.kt", path)],
            );
            assert!(result.is_err(), "Accepted {path}");
        }
    }
}
