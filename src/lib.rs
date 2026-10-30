mod annotation_declarations;
mod bytecode;
mod check;
mod classpath;
mod compiled_inline;
mod const_eval;
mod jvm_descriptor;
mod metadata;
mod object_lowering;
mod visibility;

use std::path::PathBuf;
mod source_inline;
mod syntax;

mod diagnostic;
pub use diagnostic::{Diagnostic, DiagnosticKind};

type Result<T> = std::result::Result<T, Diagnostic>;

pub struct ClassFile {
    pub internal_name: String,
    pub bytes: Vec<u8>,
}

pub struct OutputFile {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone)]
pub struct SourceInput {
    pub identity: String,
    pub file_name: String,
    pub source: String,
}

pub struct SourceOutput {
    pub source_identity: String,
    pub output: OutputFile,
}

pub fn compile_outputs_with_classpath(
    source: &str,
    file_name: &str,
    classpath: &[PathBuf],
) -> Result<Vec<OutputFile>> {
    Ok(compile_batch_with_classpath(
        &[SourceInput {
            identity: file_name.into(),
            file_name: file_name.into(),
            source: source.into(),
        }],
        classpath,
        "main",
    )?
    .into_iter()
    .map(|output| output.output)
    .collect())
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
    Ok(
        compile_outputs_with_classpath(source, file_name, classpath)?
            .into_iter()
            .filter_map(|output| {
                output
                    .relative_path
                    .strip_suffix(".class")
                    .map(|name| ClassFile {
                        internal_name: name.into(),
                        bytes: output.bytes,
                    })
            })
            .collect(),
    )
}

#[derive(Clone, Debug)]
pub struct CompileOptions {
    pub module_name: String,
    pub friend_paths: Vec<PathBuf>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            module_name: "main".into(),
            friend_paths: Vec::new(),
        }
    }
}

pub fn compile_batch_with_classpath(
    inputs: &[SourceInput],
    paths: &[PathBuf],
    module_name: &str,
) -> Result<Vec<SourceOutput>> {
    compile_batch_with_options(
        inputs,
        paths,
        &CompileOptions {
            module_name: module_name.into(),
            friend_paths: Vec::new(),
        },
    )
}

fn source_facade_name(file_name: &str) -> Result<String> {
    if file_name.contains(['/', '\\', '\0']) {
        return Err(
            Diagnostic::new(0, "Source filename must be a single path component")
                .with_kind(DiagnosticKind::Setup, "INVALID_SOURCE_FILENAME"),
        );
    }
    let stem = file_name
        .strip_suffix(".kt")
        .ok_or_else(|| Diagnostic::new(0, "Expected a .kt filename"))?;
    if !stem.is_ascii() {
        return Err(
            Diagnostic::new(0, "Non-ASCII source filename facade mapping is unsupported")
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "SOURCE_FILENAME_UNSUPPORTED",
                ),
        );
    }
    let mut prefix = stem
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if prefix.is_empty() || prefix.as_bytes()[0].is_ascii_digit() {
        prefix.insert(0, '_');
    } else {
        prefix[..1].make_ascii_uppercase();
    }
    Ok(format!("{prefix}Kt"))
}

pub fn compile_batch_with_options(
    inputs: &[SourceInput],
    paths: &[PathBuf],
    options: &CompileOptions,
) -> Result<Vec<SourceOutput>> {
    let module_name = options.module_name.as_str();
    if module_name.is_empty()
        || !module_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
    {
        return Err(Diagnostic::new(0, "Unsupported module name"));
    }
    let classpath = classpath::Classpath::with_options(paths, options)?;
    let mut sources = inputs.iter().collect::<Vec<_>>();
    sources.sort_by(|a, b| a.identity.cmp(&b.identity));
    if sources
        .windows(2)
        .any(|pair| pair[0].identity == pair[1].identity)
    {
        return Err(Diagnostic::new(0, "Duplicate source identity"));
    }
    let mut modules = Vec::new();
    let mut units = Vec::new();
    let mut owner_sources = std::collections::HashMap::new();
    let mut lexical_parents = std::collections::HashMap::new();
    for input in &sources {
        let source = input.source.as_str();
        let file_name = input.file_name.as_str();
        source_facade_name(file_name).map_err(|error| error.for_source(&input.identity))?;
        if source.len() > 1_048_576 {
            return Err(Diagnostic::new(0, "Source exceeds the current 1 MiB limit")
                .for_source(&input.identity));
        }
        let module = syntax::parse(source).map_err(|error| error.for_source(&input.identity))?;
        modules.push(module);
    }
    const_eval::resolve(&mut modules, &classpath)
        .map_err(|(index, error)| error.for_source(&sources[index].identity))?;
    for (source_index, (input, module)) in sources.iter().zip(&modules).enumerate() {
        let first_unit = units.len();
        let facade = source_facade_name(&input.file_name)
            .map_err(|error| error.for_source(&input.identity))?;
        let qualify = |name: &str| {
            if module.package.is_empty() {
                name.into()
            } else {
                format!("{}/{}", module.package.replace('.', "/"), name)
            }
        };
        if !module.functions.is_empty()
            || !module.constants.is_empty()
            || !module.properties.is_empty()
            || !module.type_aliases.is_empty()
        {
            let mut package = module.clone();
            package.annotation_decl = None;
            package.class_decl = None;
            package.object_decl = None;
            package.interface_decl = None;
            package.enum_decl = None;
            package.declarations.clear();
            units.push((qualify(&facade), package));
        }
        let mut declarations = module.declarations.clone();
        declarations.extend(
            module
                .annotation_decl
                .clone()
                .map(syntax::Declaration::Annotation),
        );
        declarations.extend(module.class_decl.clone().map(syntax::Declaration::Class));
        declarations.extend(module.object_decl.clone().map(syntax::Declaration::Object));
        declarations.extend(
            module
                .interface_decl
                .clone()
                .map(syntax::Declaration::Interface),
        );
        declarations.extend(module.enum_decl.clone().map(syntax::Declaration::Enum));
        declarations.sort_by_key(declaration_offset);
        for declaration in declarations {
            let owner = qualify(declaration_name(&declaration));
            declaration_units(declaration, owner, module, &mut units, &mut lexical_parents)
                .map_err(|error| error.for_source(&input.identity))?;
        }
        for (owner, _) in &units[first_unit..] {
            if owner_sources.insert(owner.clone(), source_index).is_some() {
                return Err(Diagnostic::new(
                    0,
                    format!("Duplicate source declaration: {}", owner.replace('/', ".")),
                )
                .for_source(&input.identity));
            }
        }
    }
    validate_alias_classifiers(&modules, &sources)?;
    let type_locations = units
        .iter()
        .map(|(owner, unit)| (owner.clone(), unit.type_locations.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let locate = |mut error: Diagnostic, owner: &str| {
        if let Some(name) = &error.type_name {
            if let Some((_, offset)) = type_locations[owner]
                .iter()
                .find(|(candidate, _)| candidate.as_str() == name.as_ref())
            {
                error.offset = *offset;
            }
        }
        error.for_source(&sources[owner_sources[owner]].identity)
    };
    let mut owners = std::collections::HashSet::new();
    for (owner, unit) in &units {
        if !owners.insert(owner.clone()) {
            return Err(Diagnostic::new(
                0,
                format!("Duplicate source declaration: {}", owner.replace('/', ".")),
            ));
        }
        classpath.register_source(classpath::JavaClass {
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            kotlin_fun_interface: false,
            library: None,
            name: owner.clone(),
            access: if unit.annotation_decl.is_some() {
                0x2601
            } else if unit.enum_decl.is_some() {
                0x4001
            } else {
                1
            },
            super_name: Some("java/lang/Object".into()),
            interfaces: if unit.annotation_decl.is_some() {
                vec!["java/lang/annotation/Annotation".into()]
            } else {
                Vec::new()
            },
            methods: Vec::new(),
            fields: Vec::new(),
            kotlin_class: true,
            unsupported: false,
            kotlin_interface: unit.interface_decl.is_some(),
        });
    }
    let source_names = units
        .iter()
        .filter_map(|(owner, unit)| {
            unit.class_decl
                .as_ref()
                .map(|d| d.name.clone())
                .or_else(|| unit.object_decl.as_ref().map(|d| d.name.clone()))
                .or_else(|| unit.interface_decl.as_ref().map(|d| d.name.clone()))
                .or_else(|| unit.enum_decl.as_ref().map(|d| d.name.clone()))
                .or_else(|| unit.annotation_decl.as_ref().map(|d| d.name.clone()))
                .map(|name| (name, owner.clone()))
        })
        .collect::<Vec<_>>();
    for (owner, unit) in &units {
        classpath.register_value_scope(owner, &modules[owner_sources[owner]]);
        classpath.register_source_file(
            owner,
            &sources[owner_sources[owner]].identity,
            unit.class_decl.is_none()
                && unit.object_decl.is_none()
                && unit.interface_decl.is_none()
                && unit.enum_decl.is_none()
                && unit.annotation_decl.is_none(),
        );
    }
    for (owner, unit) in &mut units {
        let own_name = source_names
            .iter()
            .find(|(_, path)| path == owner)
            .map(|(name, _)| name.as_str());
        for (name, path) in &source_names {
            let lexical = lexical_parents.get(path).map(String::as_str);
            let visible = (lexical.is_none()
                && path.rsplit_once('/').map_or("", |(package, _)| package)
                    == unit.package.replace('.', "/"))
                || lexical
                    .is_some_and(|outer| owner == outer || owner.starts_with(&format!("{outer}$")));
            if path != owner
                && Some(name.as_str()) != own_name
                && visible
                && !unit.imports.iter().any(|(alias, _, _)| alias == name)
            {
                unit.imports.push((name.clone(), path.clone(), 0));
            }
            if lexical_parents.contains_key(path)
                && path.rsplit_once('/').map_or("", |(package, _)| package)
                    == unit.package.replace('.', "/")
            {
                let qualified = path.rsplit('/').next().unwrap().replace('$', "/");
                if !unit.imports.iter().any(|(alias, _, _)| alias == &qualified) {
                    unit.imports.push((qualified, path.clone(), 0));
                }
            }
        }
        for (source_index, context) in modules.iter().enumerate() {
            if context.package != unit.package {
                continue;
            }
            let same_file = source_index == owner_sources[owner];
            for (name, offset, private) in context
                .functions
                .iter()
                .map(|value| (&value.name, value.offset, value.private))
                .chain(
                    context
                        .properties
                        .iter()
                        .map(|value| (&value.name, value.offset, value.private)),
                )
                .chain(
                    context
                        .constants
                        .iter()
                        .map(|value| (&value.name, value.offset, value.private)),
                )
            {
                if !same_file && private {
                    continue;
                }
                if same_file && !unit.functions.is_empty() {
                    continue;
                }
                if !unit.imports.iter().any(|(alias, _, _)| alias == name) {
                    let path = if context.package.is_empty() {
                        format!("/{name}")
                    } else {
                        format!("{}/{name}", context.package.replace('.', "/"))
                    };
                    unit.imports.push((name.clone(), path, offset));
                }
            }
        }
    }
    for (owner, unit) in &mut units {
        let file_index = owner_sources[owner];
        for (index, context) in modules.iter().enumerate() {
            for alias in &context.type_aliases {
                let public_path = if context.package.is_empty() {
                    format!("/{}", alias.name)
                } else {
                    format!("{}/{}", context.package.replace('.', "/"), alias.name)
                };
                for (_, path, offset) in &unit.imports {
                    if path == &public_path && alias.private && index != file_index {
                        return Err(locate(
                            Diagnostic::new(
                                *offset,
                                format!("Cannot access private type alias {}", alias.name),
                            ),
                            owner,
                        ));
                    }
                }
                if unit.package != context.package || alias.private && index != file_index {
                    continue;
                }
                let key = if alias.private {
                    format!(
                        "{}#{}",
                        source_facade_name(&sources[index].file_name)
                            .map_err(|error| error.for_source(&sources[index].identity))?,
                        alias.name
                    )
                } else {
                    alias.name.clone()
                };
                let path = if context.package.is_empty() {
                    format!("/{key}")
                } else {
                    format!("{}/{key}", context.package.replace('.', "/"))
                };
                if let Some((_, imported, _)) = unit
                    .imports
                    .iter_mut()
                    .find(|(name, _, _)| name == &alias.name)
                {
                    if *imported == public_path && alias.private {
                        *imported = path;
                    }
                } else {
                    unit.imports.push((alias.name.clone(), path, alias.offset));
                }
            }
        }
    }
    for (owner, unit) in &units {
        classpath
            .source_header(unit, owner)
            .map_err(|error| locate(error, owner))?;
    }
    for (index, module) in modules.iter().enumerate() {
        let mut external = module.clone();
        external.imports.retain(|(_, path, _)| {
            !path.rsplit_once('/').is_some_and(|(package, member)| {
                modules.iter().any(|candidate| {
                    candidate.package.replace('.', "/") == package
                        && (candidate
                            .functions
                            .iter()
                            .any(|function| function.name == member)
                            || candidate
                                .properties
                                .iter()
                                .any(|property| property.name == member)
                            || candidate
                                .constants
                                .iter()
                                .any(|constant| constant.name == member)
                            || candidate
                                .type_aliases
                                .iter()
                                .any(|alias| alias.name == member))
                })
            })
        });
        validate_source_imports(&external, &classpath)
            .map_err(|error| error.for_source(&sources[index].identity))?;
    }
    for (owner, unit) in &mut units {
        classpath
            .source_default_imports(unit)
            .map_err(|error| locate(error, owner))?;
    }
    loop {
        for (owner, unit) in &units {
            classpath
                .source_header(unit, owner)
                .map_err(|error| locate(error, owner))?;
        }
        classpath
            .validate_source_sealed(&owners)
            .map_err(|(owner, error)| locate(error, &owner))?;
        let mut progress = 0;
        for (owner, unit) in &mut units {
            progress += check::infer_source_results(unit, &classpath, owner)
                .map_err(|error| locate(error, owner))?;
        }
        if progress == 0 {
            break;
        }
    }
    validate_inference_cycles(&units).map_err(|(owner, error)| locate(error, &owner))?;
    for (owner, unit) in &units {
        if let Err(fallback) = reject_unresolved_inference(unit) {
            if let Some(diagnostic) = check::source_inference_error(unit, &classpath, owner)
                .map_err(|error| locate(error, owner))?
            {
                return Err(locate(diagnostic, owner));
            }
            return Err(locate(fallback, owner));
        }
    }
    for (index, module) in modules.iter().enumerate() {
        let mut validated = module.clone();
        validated.imports.retain(|(_, path, _)| {
            !path.rsplit_once('/').is_some_and(|(package, member)| {
                modules.iter().any(|context| {
                    context.package.replace('.', "/") == package
                        && context
                            .type_aliases
                            .iter()
                            .any(|alias| alias.name == member)
                })
            })
        });
        validate_source_imports(&validated, &classpath)
            .map_err(|error| error.for_source(&sources[index].identity))?;
    }
    validate_batch_declarations(&units, &owner_sources, &sources)?;
    source_inline::stage(&units, &classpath, module_name)
        .map_err(|(owner, error)| locate(error, &owner))?;
    synchronize_nested_units(&mut units);
    validate_source_inheritance(&classpath, &owners)?;
    classpath
        .validate_source_sealed(&owners)
        .map_err(|(owner, error)| locate(error, &owner))?;
    for (owner, unit) in &units {
        if unit
            .interface_decl
            .as_ref()
            .is_some_and(|interface| interface.fun_interface)
        {
            let members = classpath
                .methods(owner)
                .map_err(|error| locate(error, owner))?;
            let abstract_members = members
                .iter()
                .filter(|method| {
                    method.access & 0x0408 == 0x0400
                        && method
                            .library
                            .as_ref()
                            .is_none_or(|library| library.abstract_)
                        && !matches!(
                            (method.name.as_str(), method.descriptor.as_str()),
                            ("equals", "(Ljava/lang/Object;)Z")
                                | ("hashCode", "()I")
                                | ("toString", "()Ljava/lang/String;")
                        )
                })
                .collect::<Vec<_>>();
            let offset = unit.interface_decl.as_ref().unwrap().offset;
            if abstract_members
                .iter()
                .any(|method| method.source_property.is_some())
            {
                return Err(locate(
                    Diagnostic::new(
                        offset,
                        "Functional interfaces cannot contain abstract properties",
                    ),
                    owner,
                ));
            }
            if abstract_members.len() != 1 {
                return Err(locate(
                    Diagnostic::new(
                        offset,
                        "Functional interfaces require exactly one abstract function",
                    ),
                    owner,
                ));
            }
            if abstract_members[0]
                .library
                .as_ref()
                .is_some_and(|library| !library.type_parameters.is_empty())
            {
                return Err(locate(
                    Diagnostic::new(
                        offset,
                        "Functional interface abstract methods cannot declare type parameters",
                    ),
                    owner,
                ));
            }
        }
    }
    let mut checked = units
        .iter()
        .map(|(owner, unit)| {
            check_unit(unit, &classpath, owner).map_err(|error| locate(error, owner))
        })
        .collect::<Result<Vec<_>>>()?;
    let bridges = visibility::plan(&units, &mut checked, &classpath)
        .map_err(|(owner, error)| locate(error, &owner))?;
    classpath.apply_source_names();
    let mut files = Vec::new();
    let mut file_sources = Vec::new();
    for ((owner, unit), checked) in units.iter().zip(&checked) {
        for file in emit_unit(
            unit,
            checked,
            &bridges,
            &classpath,
            owner,
            &sources[owner_sources[owner]].file_name,
            module_name,
            &owners,
        )
        .map_err(|error| locate(error, owner))?
        {
            let emitting_owner = owners
                .iter()
                .filter(|candidate| {
                    file.internal_name == **candidate
                        || file.internal_name.starts_with(&format!("{candidate}$"))
                })
                .max_by_key(|candidate| candidate.len());
            if let Some(emitting_owner) = emitting_owner.filter(|candidate| *candidate != owner) {
                let mut parent = lexical_parents.get(emitting_owner);
                while let Some(ancestor) = parent {
                    if ancestor == owner {
                        break;
                    }
                    parent = lexical_parents.get(ancestor);
                }
                if parent.is_some() {
                    continue;
                }
                return Err(locate(
                    Diagnostic::new(
                        0,
                        format!(
                            "Generated JVM class conflicts with source declaration: {}",
                            file.internal_name
                        ),
                    ),
                    owner,
                ));
            }
            if files
                .iter()
                .any(|previous: &ClassFile| previous.internal_name == file.internal_name)
            {
                return Err(Diagnostic::new(
                    0,
                    format!("Duplicate emitted class: {}", file.internal_name),
                ));
            }
            files.push(file);
            file_sources.push(owner_sources[owner]);
        }
    }
    for file in &files {
        classpath.register_source_output(file)?;
    }
    add_nested_attributes(&mut files, &units, &lexical_parents)?;
    let mut outputs = files
        .into_iter()
        .zip(file_sources)
        .map(|(file, source_index)| SourceOutput {
            source_identity: sources[source_index].identity.clone(),
            output: OutputFile {
                relative_path: format!("{}.class", file.internal_name),
                bytes: file.bytes,
            },
        })
        .collect::<Vec<_>>();
    for (owner, unit) in &units {
        if unit.class_decl.is_some()
            || unit.object_decl.is_some()
            || unit.interface_decl.is_some()
            || unit.enum_decl.is_some()
            || unit.annotation_decl.is_some()
        {
            continue;
        }
        let module_prefix = if module_name == "main" {
            String::new()
        } else {
            format!("{module_name}\0")
        };
        let hash = module_prefix
            .bytes()
            .chain(owner.bytes())
            .fold(0x6c62272e07bb014262b821756295c58du128, |hash, byte| {
                (hash ^ u128::from(byte)).wrapping_mul(0x1000000000000000000013b)
            });
        let mapping_name = format!("META-INF/korrust_{hash:032x}.kotlin_module");
        outputs.push(SourceOutput {
            source_identity: sources[owner_sources[owner]].identity.clone(),
            output: OutputFile {
                relative_path: mapping_name,
                bytes: metadata::module_mapping(std::slice::from_ref(owner)),
            },
        });
    }
    Ok(outputs)
}

fn validate_alias_classifiers(modules: &[syntax::Module], sources: &[&SourceInput]) -> Result<()> {
    let mut declarations = Vec::new();
    for (file, module) in modules.iter().enumerate() {
        for alias in &module.type_aliases {
            declarations.push((
                file,
                &module.package,
                alias.name.as_str(),
                alias.offset,
                alias.private,
                true,
            ));
        }
        for declaration in &module.declarations {
            declarations.push((
                file,
                &module.package,
                declaration_name(declaration),
                declaration_offset(declaration),
                false,
                false,
            ));
        }
        if let Some(decl) = &module.class_decl {
            declarations.push((file, &module.package, &decl.name, decl.offset, false, false));
        }
        if let Some(decl) = &module.object_decl {
            declarations.push((file, &module.package, &decl.name, decl.offset, false, false));
        }
        if let Some(decl) = &module.interface_decl {
            declarations.push((file, &module.package, &decl.name, decl.offset, false, false));
        }
        if let Some(decl) = &module.enum_decl {
            declarations.push((file, &module.package, &decl.name, decl.offset, false, false));
        }
    }
    for (index, (file, package, name, offset, private, alias)) in declarations.iter().enumerate() {
        if declarations[..index].iter().any(
            |(other_file, other_package, other_name, _, other_private, other_alias)| {
                package == other_package
                    && name == other_name
                    && (*alias || *other_alias)
                    && (file == other_file || !private && !other_private)
            },
        ) {
            return Err(Diagnostic::new(
                *offset,
                format!("Conflicting source classifier declaration: {name}"),
            )
            .for_source(&sources[*file].identity));
        }
    }
    Ok(())
}

fn validate_inference_cycles(
    units: &[(String, syntax::Module)],
) -> std::result::Result<(), (String, Diagnostic)> {
    let nodes = units
        .iter()
        .flat_map(|(owner, module)| {
            check::unresolved_source_dependencies(module)
                .into_iter()
                .map(move |(name, offset, dependencies)| {
                    (owner, module, name, offset, dependencies)
                })
        })
        .collect::<Vec<_>>();
    let mut edges = vec![Vec::new(); nodes.len()];
    for (index, (owner, module, _, _, dependencies)) in nodes.iter().enumerate() {
        for dependency in dependencies {
            let imported = module
                .imports
                .iter()
                .find(|(alias, _, _)| alias == dependency)
                .and_then(|(_, path, _)| path.rsplit_once('/'));
            let (package, name) = imported
                .map(|(package, name)| (package.replace('/', "."), name))
                .unwrap_or_else(|| (module.package.clone(), dependency.as_str()));
            for (target, (other_owner, other_module, other_name, _, _)) in nodes.iter().enumerate()
            {
                if other_name != name || other_module.package != package {
                    continue;
                }
                let private = other_module
                    .functions
                    .iter()
                    .any(|function| function.name == *other_name && function.private)
                    || other_module
                        .properties
                        .iter()
                        .any(|property| property.name == *other_name && property.private);
                if !private || owner == other_owner {
                    edges[index].push(target);
                }
            }
        }
        edges[index].sort_unstable();
        edges[index].dedup();
    }
    fn visit(index: usize, edges: &[Vec<usize>], state: &mut [u8]) -> Option<usize> {
        if state[index] == 1 {
            return Some(index);
        }
        if state[index] == 2 {
            return None;
        }
        state[index] = 1;
        for target in &edges[index] {
            if let Some(cycle) = visit(*target, edges, state) {
                return Some(cycle);
            }
        }
        state[index] = 2;
        None
    }
    let mut state = vec![0; nodes.len()];
    for index in 0..nodes.len() {
        if let Some(cycle) = visit(index, &edges, &mut state) {
            let (owner, _, name, offset, _) = &nodes[cycle];
            return Err((
                (*owner).clone(),
                Diagnostic::new(
                    *offset,
                    format!("Cannot infer a cyclic source declaration signature involving {name}"),
                ),
            ));
        }
    }
    Ok(())
}

fn validate_batch_declarations(
    units: &[(String, syntax::Module)],
    provenance: &std::collections::HashMap<String, usize>,
    sources: &[&SourceInput],
) -> Result<()> {
    let mut functions = std::collections::HashSet::new();
    let mut properties = std::collections::HashSet::new();
    for (owner, unit) in units {
        for function in &unit.functions {
            let scope = if function.private {
                owner.as_str()
            } else {
                unit.package.as_str()
            };
            if !functions.insert((
                scope.to_owned(),
                function.name.clone(),
                format!(
                    "{:?}",
                    function.params.iter().map(|(_, ty)| ty).collect::<Vec<_>>()
                ),
            )) {
                return Err(Diagnostic::new(
                    function.offset,
                    format!("Conflicting source function declaration: {}", function.name),
                )
                .for_source(&sources[provenance[owner]].identity));
            }
        }
        for property in &unit.properties {
            let scope = if property.private {
                owner.as_str()
            } else {
                unit.package.as_str()
            };
            if !properties.insert((scope.to_owned(), property.name.clone())) {
                return Err(Diagnostic::new(
                    property.offset,
                    format!("Conflicting source property declaration: {}", property.name),
                )
                .for_source(&sources[provenance[owner]].identity));
            }
        }
    }
    Ok(())
}

fn declaration_name(declaration: &syntax::Declaration) -> &str {
    match declaration {
        syntax::Declaration::Annotation(d) => &d.name,
        syntax::Declaration::Class(d) => &d.name,
        syntax::Declaration::Object(d) => &d.name,
        syntax::Declaration::Interface(d) => &d.name,
        syntax::Declaration::Enum(d) => &d.name,
    }
}

fn declaration_offset(declaration: &syntax::Declaration) -> usize {
    match declaration {
        syntax::Declaration::Annotation(d) => d.offset,
        syntax::Declaration::Class(d) => d.offset,
        syntax::Declaration::Object(d) => d.offset,
        syntax::Declaration::Interface(d) => d.offset,
        syntax::Declaration::Enum(d) => d.offset,
    }
}

fn declaration_units(
    declaration: syntax::Declaration,
    owner: String,
    context: &syntax::Module,
    units: &mut Vec<(String, syntax::Module)>,
    lexical_parents: &mut std::collections::HashMap<String, String>,
) -> Result<()> {
    if units.len() >= 256 {
        return Err(
            Diagnostic::new(0, "Source exceeds the current 256-declaration limit").with_kind(
                DiagnosticKind::UnsupportedLanguage,
                "SOURCE_DECLARATION_LIMIT_UNSUPPORTED",
            ),
        );
    }
    let mut unit = syntax::Module {
        annotation_decl: None,
        type_locations: context.type_locations.clone(),
        type_aliases: context.type_aliases.clone(),
        package: context.package.clone(),
        imports: context.imports.clone(),
        functions: Vec::new(),
        constants: context.constants.clone(),
        properties: Vec::new(),
        enum_decl: None,
        object_decl: None,
        class_decl: None,
        interface_decl: None,
        declarations: Vec::new(),
    };
    let nested = match declaration {
        syntax::Declaration::Annotation(d) => {
            unit.annotation_decl = Some(d);
            Vec::new()
        }
        syntax::Declaration::Class(d) => {
            let nested = d.nested.clone();
            unit.class_decl = Some(d);
            nested
        }
        syntax::Declaration::Object(mut d) => {
            object_lowering::lower(&mut d, &owner)?;
            let mut nested = d.nested.clone();
            nested.extend(d.objects.iter().cloned().map(syntax::Declaration::Object));
            unit.object_decl = Some(d);
            nested
        }
        syntax::Declaration::Interface(d) => {
            let mut nested = d.declarations.clone();
            nested.extend(d.nested.iter().cloned().map(syntax::Declaration::Interface));
            unit.interface_decl = Some(d);
            nested
        }
        syntax::Declaration::Enum(d) => {
            let nested = d.body.nested.clone();
            for (entry, offset) in &d.entries {
                unit.imports
                    .push((entry.clone(), format!("{owner}/{entry}"), *offset));
            }
            unit.imports
                .push(("entries".into(), format!("{owner}/entries"), d.offset));
            unit.enum_decl = Some(d);
            nested
        }
    };
    units.push((owner.clone(), unit));
    for declaration in nested {
        let nested_owner = format!("{owner}${}", declaration_name(&declaration));
        lexical_parents.insert(nested_owner.clone(), owner.clone());
        declaration_units(declaration, nested_owner, context, units, lexical_parents)?;
    }
    Ok(())
}

enum CheckedUnit {
    Module(Box<check::Module>),
    Enum(Box<check::enums::CheckedEnum>),
}

fn check_unit(
    module: &syntax::Module,
    classpath: &classpath::Classpath,
    owner: &str,
) -> Result<CheckedUnit> {
    if module.enum_decl.is_some() {
        check::enums::check_enum(module, classpath, owner)
            .map(|value| CheckedUnit::Enum(Box::new(value)))
    } else {
        check::check_with_owner(module, classpath, owner)
            .map(|value| CheckedUnit::Module(Box::new(value)))
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_unit(
    module: &syntax::Module,
    checked: &CheckedUnit,
    bridges: &[visibility::Bridge],
    classpath: &classpath::Classpath,
    internal_name: &str,
    file_name: &str,
    module_name: &str,
    declared_owners: &std::collections::HashSet<String>,
) -> Result<Vec<ClassFile>> {
    if module.annotation_decl.is_some() {
        let annotation = annotation_declarations::check(module, internal_name, classpath)?;
        let metadata =
            metadata::for_annotation(&annotation, internal_name)?.in_module(module_name)?;
        return Ok(vec![bytecode::annotation_classes::emit(
            &annotation,
            internal_name,
            file_name,
            &metadata,
        )?]);
    }
    let placements = check::annotation_placements(module, classpath, internal_name)?;
    let metadata_source = metadata::annotation_source(module, classpath)?;
    let mut files = emit_unit_without_annotations(
        &metadata_source,
        checked,
        bridges,
        classpath,
        internal_name,
        file_name,
        module_name,
        declared_owners,
    )?;
    for file in &mut files {
        if let Some(annotations) = placements.get(&file.internal_name) {
            file.bytes = bytecode::annotation_files::apply(&file.bytes, annotations)?;
        }
    }
    Ok(files)
}

#[allow(clippy::too_many_arguments)]
fn emit_unit_without_annotations(
    module: &syntax::Module,
    checked: &CheckedUnit,
    bridges: &[visibility::Bridge],
    _classpath: &classpath::Classpath,
    internal_name: &str,
    file_name: &str,
    module_name: &str,
    declared_owners: &std::collections::HashSet<String>,
) -> Result<Vec<ClassFile>> {
    if let Some(source) = &module.enum_decl {
        let CheckedUnit::Enum(checked) = checked else {
            return Err(Diagnostic::new(0, "Expected checked enum"));
        };
        let mut generated = checked
            .lambdas
            .iter()
            .map(|lambda| lambda.owner.clone())
            .collect::<Vec<_>>();
        if source.body.companion.is_some() {
            generated.push(format!("{internal_name}$Companion"));
        }
        if let Some(collision) = generated
            .iter()
            .find(|owner| declared_owners.contains(*owner))
        {
            return Err(Diagnostic::new(
                source.offset,
                format!("Generated JVM class conflicts with source declaration: {collision}"),
            ));
        }
        let companion_metadata = source
            .body
            .companion
            .as_ref()
            .zip(checked.class.companion.as_ref())
            .map(|(source, checked)| {
                metadata::for_class_companion(source, checked, &module.imports)
                    .and_then(|value| value.in_module(module_name))
            })
            .transpose()?;
        return bytecode::emit_checked_enum(
            checked,
            internal_name,
            file_name,
            &metadata::for_enum(source, checked, internal_name, &module.imports)?
                .in_module(module_name)?,
            companion_metadata.as_ref(),
        );
    }
    let CheckedUnit::Module(typed) = checked else {
        return Err(Diagnostic::new(0, "Expected checked declaration"));
    };
    let mut generated_owners = typed
        .lambdas
        .iter()
        .map(|lambda| lambda.owner.clone())
        .collect::<Vec<_>>();
    let companion = module
        .class_decl
        .as_ref()
        .is_some_and(|decl| decl.companion.is_some())
        || module
            .interface_decl
            .as_ref()
            .is_some_and(|decl| decl.companion.is_some())
        || module
            .enum_decl
            .as_ref()
            .is_some_and(|decl| decl.body.companion.is_some());
    if companion {
        generated_owners.push(format!("{internal_name}$Companion"));
    }
    if typed.interface_decl.as_ref().is_some_and(|decl| {
        decl.methods.iter().any(|method| method.has_body) || !decl.defaults.is_empty()
    }) {
        generated_owners.push(format!("{internal_name}$DefaultImpls"));
    }
    for generated in generated_owners {
        if declared_owners.contains(&generated) {
            return Err(Diagnostic::new(
                0,
                format!("Generated JVM class conflicts with source declaration: {generated}"),
            ));
        }
    }
    let with_lambdas = |mut files: Vec<ClassFile>| -> Result<Vec<ClassFile>> {
        files.extend(bytecode::emit_lambdas(typed, internal_name, file_name)?);
        Ok(files)
    };
    if module.interface_decl.is_some() {
        let declaration = typed
            .interface_decl
            .as_ref()
            .ok_or_else(|| Diagnostic::new(0, "Interface checking did not produce an interface"))?;
        let metadata = metadata::for_interfaces(module, internal_name, Some(declaration))?
            .into_iter()
            .map(|(owner, value)| Ok((owner, value.in_module(module_name)?)))
            .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
        return with_lambdas(bytecode::emit_interfaces(
            declaration,
            file_name,
            &metadata,
            &typed.lambdas,
        )?);
    }
    if let Some(declaration) = &typed.object_decl {
        let field_types = declaration
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.ty.clone()))
            .collect::<Vec<_>>();
        return with_lambdas(bytecode::emit_dynamic_object(
            declaration,
            internal_name,
            file_name,
            &metadata::for_checked_object(
                module,
                internal_name,
                &field_types,
                &declaration.interfaces,
                declaration.serializable,
                declaration,
            )?
            .in_module(module_name)?,
            &typed.lambdas,
        )?);
    }
    if let Some(declaration) = &module.object_decl {
        let metadata = metadata::for_constant_objects(module, internal_name)?
            .into_iter()
            .map(|(owner, metadata)| Ok((owner, metadata.in_module(module_name)?)))
            .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
        return with_lambdas(bytecode::emit_objects(
            declaration,
            internal_name,
            file_name,
            &metadata,
        )?);
    }
    if let Some(source_declaration) = &module.class_decl {
        let declaration = typed
            .class_decl
            .as_ref()
            .ok_or_else(|| Diagnostic::new(0, "Class checking did not produce a class"))?;
        let companion_metadata = declaration
            .companion
            .as_ref()
            .map(|companion| {
                metadata::for_class_companion(
                    source_declaration.companion.as_ref().ok_or_else(|| {
                        Diagnostic::new(0, "Missing source companion declaration")
                    })?,
                    companion,
                    &module.imports,
                )
            })
            .transpose()?
            .map(|metadata| metadata.in_module(module_name))
            .transpose()?;
        return with_lambdas(bytecode::emit_class_files(
            declaration,
            internal_name,
            file_name,
            &metadata::for_checked_class(
                source_declaration,
                declaration,
                internal_name,
                &module.imports,
            )?
            .in_module(module_name)?,
            companion_metadata.as_ref(),
            &typed.lambdas,
        )?);
    }
    let bytes = match &module.enum_decl {
        Some(declaration) => bytecode::emit_enum(declaration, internal_name, file_name)?,
        None => bytecode::emit_with_bridges(
            typed,
            internal_name,
            file_name,
            &metadata::for_checked_package(module, typed)?.in_module(module_name)?,
            bridges,
        )?,
    };
    with_lambdas(vec![ClassFile {
        internal_name: internal_name.to_owned(),
        bytes,
    }])
}

fn add_nested_attributes(
    files: &mut [ClassFile],
    units: &[(String, syntax::Module)],
    lexical_parents: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let entries = units
        .iter()
        .filter_map(|(owner, unit)| {
            let outer = lexical_parents.get(owner)?;
            let short = owner.strip_prefix(&format!("{outer}$"))?;
            let access = if unit.interface_decl.is_some() {
                0x609
            } else if unit.enum_decl.is_some() {
                0x4019
            } else if unit.class_decl.as_ref().is_some_and(|c| c.abstract_) {
                0x409
            } else if unit.class_decl.as_ref().is_some_and(|c| c.open) {
                9
            } else {
                0x19
            };
            Some((owner.clone(), outer.to_owned(), short.to_owned(), access))
        })
        .collect::<Vec<_>>();
    for file in files {
        let relevant = entries
            .iter()
            .filter(|(owner, outer, _, _)| {
                owner == &file.internal_name || outer == &file.internal_name
            })
            .cloned()
            .collect::<Vec<_>>();
        file.bytes = classpath::nested::attributes(&file.bytes, &relevant)?;
    }
    Ok(())
}

fn reject_unresolved_inference(module: &syntax::Module) -> Result<()> {
    if let Some(enumeration) = &module.enum_decl {
        let mut context = module.clone();
        context.enum_decl = None;
        context.class_decl = Some(enumeration.body.clone());
        return reject_unresolved_inference(&context);
    }
    let mut unresolved = module.functions.iter().any(|f| f.result_inferred)
        || module.properties.iter().any(|p| p.inferred);
    if let Some(class) = &module.class_decl {
        unresolved |= class.methods.iter().any(|m| m.function.result_inferred)
            || class.properties.iter().any(|p| p.inferred)
            || class.companion.as_ref().is_some_and(|c| {
                c.methods.iter().any(|m| m.function.result_inferred)
                    || c.properties.iter().any(|p| p.ty.is_none())
            });
    }
    if let Some(object) = &module.object_decl {
        unresolved |= object.methods.iter().any(|m| m.function.result_inferred)
            || object.fields.iter().any(|p| p.ty.is_none());
    }
    if let Some(interface) = &module.interface_decl {
        unresolved |= interface.methods.iter().any(|m| m.function.result_inferred)
            || interface.companion.as_ref().is_some_and(|companion| {
                companion
                    .fields
                    .iter()
                    .any(|property| property.ty.is_none())
                    || companion
                        .methods
                        .iter()
                        .any(|method| method.function.result_inferred)
            });
    }
    if unresolved {
        Err(Diagnostic::new(
            0,
            "Cannot infer a source declaration signature: cyclic or unsupported initializer or expression body; add an explicit type",
        ))
    } else {
        Ok(())
    }
}

fn validate_source_inheritance(
    classpath: &classpath::Classpath,
    owners: &std::collections::HashSet<String>,
) -> Result<()> {
    fn visit(
        name: &str,
        classpath: &classpath::Classpath,
        owners: &std::collections::HashSet<String>,
        active: &mut std::collections::HashSet<String>,
        done: &mut std::collections::HashSet<String>,
    ) -> Result<()> {
        if done.contains(name) {
            return Ok(());
        }
        if !active.insert(name.into()) {
            return Err(Diagnostic::new(
                0,
                format!(
                    "Cyclic source inheritance involving {}",
                    name.replace('/', ".")
                ),
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "INHERITANCE_CYCLE_INVALID"));
        }
        let class = classpath.load(name)?;
        for parent in class
            .super_name
            .iter()
            .chain(&class.interfaces)
            .filter(|parent| owners.contains(*parent))
        {
            visit(parent, classpath, owners, active, done)?;
        }
        active.remove(name);
        done.insert(name.into());
        Ok(())
    }
    let mut done = std::collections::HashSet::new();
    for owner in owners {
        visit(
            owner,
            classpath,
            owners,
            &mut std::collections::HashSet::new(),
            &mut done,
        )?;
    }
    Ok(())
}

fn synchronize_nested_units(units: &mut [(String, syntax::Module)]) {
    let declarations = units
        .iter()
        .filter_map(|(owner, module)| {
            module
                .class_decl
                .clone()
                .map(syntax::Declaration::Class)
                .or_else(|| module.object_decl.clone().map(syntax::Declaration::Object))
                .or_else(|| {
                    module
                        .interface_decl
                        .clone()
                        .map(syntax::Declaration::Interface)
                })
                .or_else(|| module.enum_decl.clone().map(syntax::Declaration::Enum))
                .or_else(|| {
                    module
                        .annotation_decl
                        .clone()
                        .map(syntax::Declaration::Annotation)
                })
                .map(|declaration| (owner.clone(), declaration))
        })
        .collect::<std::collections::HashMap<_, _>>();
    fn declaration(
        owner: &str,
        declarations: &std::collections::HashMap<String, syntax::Declaration>,
    ) -> syntax::Declaration {
        let mut source = declarations[owner].clone();
        let update = |children: &mut Vec<syntax::Declaration>| {
            for child in children {
                let name = format!("{owner}${}", declaration_name(child));
                *child = declaration(&name, declarations);
            }
        };
        match &mut source {
            syntax::Declaration::Class(source) => update(&mut source.nested),
            syntax::Declaration::Object(source) => {
                update(&mut source.nested);
                for child in &mut source.objects {
                    if let syntax::Declaration::Object(updated) =
                        declaration(&format!("{owner}${}", child.name), declarations)
                    {
                        *child = updated;
                    }
                }
            }
            syntax::Declaration::Interface(source) => {
                update(&mut source.declarations);
                for child in &mut source.nested {
                    if let syntax::Declaration::Interface(updated) =
                        declaration(&format!("{owner}${}", child.name), declarations)
                    {
                        *child = updated;
                    }
                }
            }
            syntax::Declaration::Annotation(_) | syntax::Declaration::Enum(_) => {}
        }
        source
    }
    for (owner, module) in units {
        if !declarations.contains_key(owner) {
            continue;
        }
        match declaration(owner, &declarations) {
            syntax::Declaration::Class(source) => module.class_decl = Some(source),
            syntax::Declaration::Object(source) => module.object_decl = Some(source),
            syntax::Declaration::Interface(source) => module.interface_decl = Some(source),
            syntax::Declaration::Annotation(source) => module.annotation_decl = Some(source),
            syntax::Declaration::Enum(source) => module.enum_decl = Some(source),
        }
    }
}

fn validate_source_imports(
    module: &syntax::Module,
    classpath: &classpath::Classpath,
) -> Result<()> {
    for (alias, path, offset) in &module.imports {
        let own = module
            .class_decl
            .as_ref()
            .filter(|declaration| declaration.name == *alias)
            .map(|_| "class")
            .or_else(|| {
                module
                    .object_decl
                    .as_ref()
                    .filter(|declaration| declaration.name == *alias)
                    .map(|_| "object")
            })
            .or_else(|| {
                module
                    .interface_decl
                    .as_ref()
                    .filter(|declaration| declaration.name == *alias)
                    .map(|_| "interface")
            })
            .or_else(|| {
                module
                    .enum_decl
                    .as_ref()
                    .filter(|declaration| declaration.name == *alias)
                    .map(|_| "enum")
            })
            .or_else(|| {
                module
                    .declarations
                    .iter()
                    .find(|declaration| declaration_name(declaration) == alias)
                    .map(|declaration| match declaration {
                        syntax::Declaration::Class(_) => "class",
                        syntax::Declaration::Object(_) => "object",
                        syntax::Declaration::Interface(_) => "interface",
                        syntax::Declaration::Enum(_) => "enum",
                        syntax::Declaration::Annotation(_) => "annotation",
                    })
            });
        if let Some(kind) = own {
            return Err(Diagnostic::new(
                *offset,
                format!("Class import conflicts with {kind} declaration"),
            ));
        }
        if matches!(
            alias.as_str(),
            "Int"
                | "Long"
                | "List"
                | "Byte"
                | "ByteArray"
                | "ThreadLocal"
                | "Char"
                | "Float"
                | "Boolean"
                | "String"
                | "IntArray"
                | "CharArray"
                | "StringArray"
                | "Array"
                | "Unit"
        ) {
            return Err(Diagnostic::new(
                *offset,
                "Imports shadowing builtin types are not supported",
            ));
        }
        if alias == "*" {
            if classpath.load(path.trim_end_matches("/*")).is_ok() {
                return Err(Diagnostic::new(
                    *offset,
                    "Static-member wildcard imports are unsupported",
                )
                .with_kind(
                    DiagnosticKind::UnsupportedLanguage,
                    "STATIC_MEMBER_WILDCARD_UNSUPPORTED",
                ));
            }
            if classpath
                .package_classes(path.trim_end_matches("/*"))?
                .is_empty()
            {
                return Err(Diagnostic::new(
                    *offset,
                    "Unresolved wildcard import package",
                ));
            }
            continue;
        }
        if let Err(mut diagnostic) = classpath.load(path) {
            if !path.contains('/')
                && classpath
                    .package_constants("", path)?
                    .iter()
                    .any(|field| !field.unsupported && classpath.kotlin_field_visible("", field))
            {
                continue;
            }
            if let Some((package, member)) = path.rsplit_once('/') {
                if classpath.type_alias(package, member)?.is_some()
                    || classpath.package_classes(package)?.iter().any(|class| {
                        class.methods.iter().any(|method| {
                            method.source_property.as_deref() == Some(member)
                                || method
                                    .library
                                    .as_ref()
                                    .is_some_and(|library| library.source_name == member)
                        }) || class
                            .fields
                            .iter()
                            .any(|field| field.name == member && field.constant.is_some())
                    })
                {
                    continue;
                }
                if classpath.load(package).is_ok_and(|class| {
                    class.access & 1 != 0
                        && (classpath
                            .field_candidates(package, member)
                            .is_ok_and(|fields| fields.iter().any(|field| field.access & 9 == 9))
                            || class.methods.iter().any(|method| {
                                method.source_name() == member && method.access & 9 == 9
                            })
                            || classpath.methods(package).is_ok_and(|methods| {
                                methods.iter().any(|method| {
                                    method.source_name() == member && method.access & 9 == 9
                                })
                            }))
                }) {
                    continue;
                }
            }
            diagnostic.offset = *offset;
            return Err(diagnostic);
        }
    }
    Ok(())
}

#[cfg(test)]
mod module_tests {
    use super::*;

    #[test]
    fn resolves_cyclic_type_signatures_and_emits_the_package_facade_first() {
        let classes=compile_all("package sample\nclass Left(val right: Right?)\nclass Right(val left: Left?)\nfun answer(): Int = 42", "Module.kt").unwrap();
        assert_eq!(
            classes
                .iter()
                .map(|class| class.internal_name.as_str())
                .collect::<Vec<_>>(),
            ["sample/ModuleKt", "sample/Left", "sample/Right"]
        );
    }

    #[test]
    fn nested_simple_names_follow_the_containing_declaration() {
        compile_all("class First { class Item; fun make(): Item = Item() }\nclass Second { class Item; fun make(): Item = Item() }", "Nested.kt").unwrap();
        let error = compile_all(
            "class Outer { class Hidden }\nfun invalid(): Hidden = Hidden()",
            "Nested.kt",
        )
        .err()
        .unwrap();
        assert!(error.message.contains("Hidden"));
    }

    #[test]
    fn rejects_duplicate_owners_and_cyclic_inferred_signatures() {
        assert!(
            compile_all("class Duplicate\nclass Duplicate", "Duplicate.kt")
                .err()
                .unwrap()
                .message
                .contains("Duplicate source declaration")
        );
        assert!(
            compile_all("fun first() = second()\nfun second() = first()", "Cycle.kt")
                .err()
                .unwrap()
                .message
                .contains("Cannot infer")
        );
    }
}
