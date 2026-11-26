use super::{Classpath, Diagnostic, Reader, Result, invalid, modified_utf8_units};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Default)]
struct Descriptor {
    name: String,
    requires: Vec<(String, bool)>,
    exports: BTreeSet<String>,
    packages: BTreeSet<String>,
}

#[derive(Default)]
struct Modules {
    exported: BTreeMap<String, PathBuf>,
    all: BTreeMap<String, PathBuf>,
}

enum Entry {
    Empty,
    Utf8(String),
    Name(u8, u16),
}

fn name(pool: &[Entry], index: u16, tag: u8) -> Result<&str> {
    let Some(Entry::Name(actual, index)) = pool.get(usize::from(index)) else {
        return Err(invalid());
    };
    if *actual != tag {
        return Err(invalid());
    }
    text(pool, *index)
}

fn text(pool: &[Entry], index: u16) -> Result<&str> {
    match pool.get(usize::from(index)) {
        Some(Entry::Utf8(value)) => Ok(value),
        _ => Err(invalid()),
    }
}

fn descriptor(bytes: &[u8]) -> Result<Descriptor> {
    let mut reader = Reader { bytes, offset: 0 };
    if reader.u4()? != 0xcafebabe {
        return Err(invalid());
    }
    reader.u2()?;
    if reader.u2()? < 53 {
        return Err(invalid());
    }
    let count = usize::from(reader.u2()?);
    let mut pool = vec![Entry::Empty];
    while pool.len() < count {
        let tag = reader.u1()?;
        let entry = match tag {
            1 => {
                let length = usize::from(reader.u2()?);
                Entry::Utf8(String::from_utf16_lossy(&modified_utf8_units(
                    reader.take(length)?,
                )?))
            }
            7 | 19 | 20 => Entry::Name(tag, reader.u2()?),
            3 | 4 | 9..=12 | 17 | 18 => {
                reader.take(4)?;
                Entry::Empty
            }
            5 | 6 => {
                reader.take(8)?;
                pool.push(Entry::Empty);
                Entry::Empty
            }
            8 | 16 => {
                reader.take(2)?;
                Entry::Empty
            }
            15 => {
                reader.take(3)?;
                Entry::Empty
            }
            _ => return Err(invalid()),
        };
        pool.push(entry);
    }
    if pool.len() != count
        || reader.u2()? != 0x8000
        || name(&pool, reader.u2()?, 7)? != "module-info"
    {
        return Err(invalid());
    }
    for _ in 0..4 {
        if reader.u2()? != 0 {
            return Err(invalid());
        }
    }
    let mut result = Descriptor::default();
    for _ in 0..reader.u2()? {
        let attribute = text(&pool, reader.u2()?)?;
        let length = reader.u4()? as usize;
        let mut value = Reader {
            bytes: reader.take(length)?,
            offset: 0,
        };
        match attribute {
            "Module" => {
                result.name = name(&pool, value.u2()?, 19)?.into();
                value.take(4)?;
                for _ in 0..value.u2()? {
                    let required = name(&pool, value.u2()?, 19)?.into();
                    let transitive = value.u2()? & 0x20 != 0;
                    value.u2()?;
                    result.requires.push((required, transitive));
                }
                for _ in 0..value.u2()? {
                    let package = name(&pool, value.u2()?, 20)?.to_owned();
                    value.u2()?;
                    let targets = value.u2()?;
                    value.take(usize::from(targets) * 2)?;
                    if targets == 0 {
                        result.exports.insert(package);
                    }
                }
            }
            "ModulePackages" => {
                for _ in 0..value.u2()? {
                    result.packages.insert(name(&pool, value.u2()?, 20)?.into());
                }
            }
            _ => {}
        }
    }
    if result.name.is_empty() || reader.offset != bytes.len() {
        return Err(invalid());
    }
    result.packages.extend(result.exports.iter().cloned());
    Ok(result)
}

impl Modules {
    fn load(classpath: &Classpath, home: &Path) -> Result<Self> {
        let directory = home.join("jmods");
        if !directory.is_dir() {
            return Ok(Self::default());
        }
        let mut paths = std::fs::read_dir(&directory)
            .map_err(|e| Diagnostic::new(0, e.to_string()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| Diagnostic::new(0, e.to_string()))?;
        paths.sort();
        let mut descriptors = BTreeMap::new();
        for path in paths
            .into_iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "jmod"))
        {
            let bytes = classpath
                .archives
                .borrow_mut()
                .system_module_descriptor(&path)?
                .ok_or_else(invalid)?;
            let info = descriptor(&bytes).map_err(|mut error| {
                error.message = format!("{}: {}", path.display(), error.message);
                error
            })?;
            descriptors.insert(info.name.clone(), (path, info));
        }
        let has_java_se = descriptors.contains_key("java.se");
        let roots: Vec<_> = descriptors
            .iter()
            .filter_map(|(name, (_, info))| {
                (name == "java.se"
                    || ((!has_java_se || !name.starts_with("java.")) && !info.exports.is_empty()))
                .then_some(name.clone())
            })
            .collect();
        let mut resolved: BTreeSet<_> = roots.iter().cloned().collect();
        resolved.insert("java.base".into());
        let mut pending = Vec::new();
        for root in &roots {
            if let Some((_, info)) = descriptors.get(root) {
                for (dependency, _) in &info.requires {
                    if resolved.insert(dependency.clone()) {
                        pending.push(dependency.clone());
                    }
                }
            }
        }
        while let Some(module) = pending.pop() {
            if let Some((_, info)) = descriptors.get(&module) {
                for (dependency, transitive) in &info.requires {
                    if *transitive && resolved.insert(dependency.clone()) {
                        pending.push(dependency.clone());
                    }
                }
            }
        }
        let mut result = Self::default();
        for (module, (path, info)) in descriptors {
            for package in info.packages {
                result.all.insert(package, path.clone());
            }
            if resolved.contains(&module) {
                for package in info.exports {
                    result.exported.insert(package, path.clone());
                }
            }
        }
        Ok(result)
    }
}

impl Classpath {
    pub(super) fn jdk_class_path(&self, owner: &str, exported: bool) -> Result<Option<PathBuf>> {
        let Some(home) = std::env::var_os("JAVA_HOME").map(PathBuf::from) else {
            return Ok(None);
        };
        let Some((package, _)) = owner.rsplit_once('/') else {
            return Ok(None);
        };
        static CACHE: OnceLock<Mutex<BTreeMap<PathBuf, Arc<Modules>>>> = OnceLock::new();
        let cache = CACHE.get_or_init(Mutex::default);
        let existing = cache.lock().unwrap().get(&home).cloned();
        let modules = match existing {
            Some(modules) => modules,
            None => {
                let modules = Arc::new(Modules::load(self, &home)?);
                cache.lock().unwrap().insert(home, modules.clone());
                modules
            }
        };
        let packages = if exported {
            &modules.exported
        } else {
            &modules.all
        };
        Ok(packages.get(package).cloned())
    }

    pub(super) fn read_jdk_class(&self, owner: &str, exported: bool) -> Result<Option<Vec<u8>>> {
        let Some(path) = self.jdk_class_path(owner, exported)? else {
            return Ok(None);
        };
        self.read_resource(&path, &format!("classes/{owner}.class"))
    }
}
