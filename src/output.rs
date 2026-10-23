use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

const HEADER: &str = "korrust-output-v1";
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
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'$')
        })
}

fn read_index(path: &Path) -> Result<Index, String> {
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
    if first != format!("{HEADER}\t{:016x}", hash(body.as_bytes()))
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
    let mut body = String::new();
    for (class, (source, digest)) in index {
        body.push_str(&format!(
            "{}\t{}\t{digest:016x}\n",
            encode(source),
            encode(class)
        ));
    }
    let content = format!("{HEADER}\t{:016x}\n{body}", hash(body.as_bytes())).into_bytes();
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
