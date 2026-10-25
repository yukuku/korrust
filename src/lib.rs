mod bytecode;
mod check;
mod classpath;
mod compiled_inline;
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
    let mapping = if module.functions.is_empty()
        && module.constants.is_empty()
        && module.properties.is_empty()
    {
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
    let facade = format!("{}{}Kt", stem[..1].to_ascii_uppercase(), &stem[1..]);
    let qualify = |name: &str| {
        if module.package.is_empty() {
            name.into()
        } else {
            format!("{}/{}", module.package.replace('.', "/"), name)
        }
    };
    let mut units = Vec::new();
    if !module.functions.is_empty() || !module.constants.is_empty() || !module.properties.is_empty()
    {
        let mut package = module.clone();
        package.class_decl = None;
        package.object_decl = None;
        package.interface_decl = None;
        package.enum_decl = None;
        package.declarations.clear();
        units.push((qualify(&facade), package));
    }
    let mut declarations = module.declarations.clone();
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
        declaration_units(declaration, owner, &module, &mut units)?;
    }
    let mut owners = std::collections::HashSet::new();
    for (owner, unit) in &units {
        if !owners.insert(owner.clone()) {
            return Err(Diagnostic::new(
                0,
                format!("Duplicate source declaration: {}", owner.replace('/', ".")),
            ));
        }
        classpath.register_source(classpath::JavaClass {
            library: None,
            name: owner.clone(),
            access: 1,
            super_name: Some("java/lang/Object".into()),
            interfaces: Vec::new(),
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
                .map(|name| (name, owner.clone()))
        })
        .collect::<Vec<_>>();
    for (owner, unit) in &mut units {
        let own_name = source_names
            .iter()
            .find(|(_, path)| path == owner)
            .map(|(name, _)| name.as_str());
        for (name, path) in &source_names {
            let lexical = path.rsplit_once('$').map(|(outer, _)| outer);
            let visible = lexical.is_none()
                || lexical
                    .is_some_and(|outer| owner == outer || owner.starts_with(&format!("{outer}$")));
            if path != owner
                && Some(name.as_str()) != own_name
                && visible
                && !unit.imports.iter().any(|(alias, _, _)| alias == name)
            {
                unit.imports.push((name.clone(), path.clone(), 0));
            }
            if path.contains('$') {
                let qualified = path.rsplit('/').next().unwrap().replace('$', "/");
                if !unit.imports.iter().any(|(alias, _, _)| alias == &qualified) {
                    unit.imports.push((qualified, path.clone(), 0));
                }
            }
        }
        if unit.functions.is_empty() {
            for function in &module.functions {
                if !unit
                    .imports
                    .iter()
                    .any(|(alias, _, _)| alias == &function.name)
                {
                    let path = if module.package.is_empty() {
                        format!("/{}", function.name)
                    } else {
                        format!("{}/{}", module.package.replace('.', "/"), function.name)
                    };
                    unit.imports
                        .push((function.name.clone(), path, function.offset));
                }
            }
            for property in &module.properties {
                if !unit
                    .imports
                    .iter()
                    .any(|(alias, _, _)| alias == &property.name)
                {
                    let path = if module.package.is_empty() {
                        format!("/{}", property.name)
                    } else {
                        format!("{}/{}", module.package.replace('.', "/"), property.name)
                    };
                    unit.imports
                        .push((property.name.clone(), path, property.offset));
                }
            }
        }
    }
    validate_source_imports(&module, &classpath)?;
    for (_, unit) in &mut units {
        classpath.source_default_imports(unit)?;
    }
    loop {
        for (owner, unit) in &units {
            classpath.source_header(unit, owner)?;
        }
        let mut progress = 0;
        for (owner, unit) in &mut units {
            progress += check::infer_source_results(unit, &classpath, owner)?;
        }
        if progress == 0 {
            break;
        }
    }
    for (owner, unit) in &units {
        if let Err(fallback) = reject_unresolved_inference(unit) {
            if let Some(diagnostic) = check::source_inference_error(unit, &classpath, owner)? {
                return Err(diagnostic);
            }
            return Err(fallback);
        }
    }
    synchronize_nested_units(&mut units);
    validate_source_inheritance(&classpath, &owners)?;
    let mut files = Vec::new();
    for (owner, unit) in &units {
        for file in emit_unit(unit, &classpath, owner, file_name)? {
            let emitting_owner = owners
                .iter()
                .filter(|candidate| {
                    file.internal_name == **candidate
                        || file.internal_name.starts_with(&format!("{candidate}$"))
                })
                .max_by_key(|candidate| candidate.len());
            if emitting_owner.is_some_and(|emitting_owner| emitting_owner != owner) {
                continue;
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
        }
    }
    for file in &files {
        classpath.register_source_output(file)?;
    }
    add_nested_attributes(&mut files, &units)?;
    Ok(files)
}

fn declaration_name(declaration: &syntax::Declaration) -> &str {
    match declaration {
        syntax::Declaration::Class(d) => &d.name,
        syntax::Declaration::Object(d) => &d.name,
        syntax::Declaration::Interface(d) => &d.name,
        syntax::Declaration::Enum(d) => &d.name,
    }
}

fn declaration_offset(declaration: &syntax::Declaration) -> usize {
    match declaration {
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
) -> Result<()> {
    if units.len() >= 256 {
        return Err(Diagnostic::new(
            0,
            "Source exceeds the current 256-declaration limit",
        ));
    }
    let mut unit = syntax::Module {
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
        syntax::Declaration::Class(d) => {
            let nested = d.nested.clone();
            unit.class_decl = Some(d);
            nested
        }
        syntax::Declaration::Object(d) => {
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
            unit.enum_decl = Some(d);
            Vec::new()
        }
    };
    units.push((owner.clone(), unit));
    for declaration in nested {
        let nested_owner = format!("{owner}${}", declaration_name(&declaration));
        declaration_units(declaration, nested_owner, context, units)?;
    }
    Ok(())
}

fn emit_unit(
    module: &syntax::Module,
    classpath: &classpath::Classpath,
    internal_name: &str,
    file_name: &str,
) -> Result<Vec<ClassFile>> {
    let typed = check::check_with_owner(module, classpath, internal_name)?;
    let with_lambdas = |mut files: Vec<ClassFile>| -> Result<Vec<ClassFile>> {
        files.extend(bytecode::emit_lambdas(&typed, internal_name, file_name)?);
        Ok(files)
    };
    if module.interface_decl.is_some() {
        let declaration = typed
            .interface_decl
            .as_ref()
            .ok_or_else(|| Diagnostic::new(0, "Interface checking did not produce an interface"))?;
        return with_lambdas(bytecode::emit_interfaces(
            declaration,
            file_name,
            &metadata::for_interfaces(module, internal_name, Some(declaration))?,
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
            &metadata::for_object(
                module,
                internal_name,
                &field_types,
                &declaration.interfaces,
                declaration.serializable,
            )?,
            &typed.lambdas,
        )?);
    }
    if let Some(declaration) = &module.object_decl {
        return with_lambdas(bytecode::emit_objects(
            declaration,
            internal_name,
            file_name,
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
            )?,
            companion_metadata.as_ref(),
            &typed.lambdas,
        )?);
    }
    let bytes = match &module.enum_decl {
        Some(declaration) => bytecode::emit_enum(declaration, internal_name, file_name)?,
        None => bytecode::emit(
            &typed,
            internal_name,
            file_name,
            &metadata::for_checked_package(module, &typed)?,
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
) -> Result<()> {
    let entries = units
        .iter()
        .filter_map(|(owner, unit)| {
            let (outer, short) = owner.rsplit_once('$')?;
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
            ));
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
            syntax::Declaration::Enum(_) => {}
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
            if let Some((package, member)) = path.rsplit_once('/') {
                if classpath.type_alias(package, member)?.is_some()
                    || classpath.package_classes(package)?.iter().any(|class| {
                        class.methods.iter().any(|method| {
                            method.source_property.as_deref() == Some(member)
                                || method
                                    .library
                                    .as_ref()
                                    .is_some_and(|library| library.source_name == member)
                        })
                    })
                {
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
