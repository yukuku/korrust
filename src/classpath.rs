mod sealed;
use std::path::PathBuf;

mod aliases;
mod archive;
pub(crate) mod jvm_builtins;
mod metadata;
pub(crate) mod nested;
mod signature;
mod source;
mod value_names;
pub(crate) use value_names::ValueSymbol;

use crate::syntax::TypeParameter;

use crate::{Diagnostic, Result, syntax::Type};

pub(crate) const DEFAULT_KOTLIN_PACKAGES: &[&str] = &[
    "kotlin",
    "kotlin/annotation",
    "kotlin/collections",
    "kotlin/comparisons",
    "kotlin/io",
    "kotlin/ranges",
    "kotlin/sequences",
    "kotlin/text",
    "kotlin/jvm",
];

pub(crate) fn kotlin_mapped_classifier(owner: &str) -> Option<&'static str> {
    Some(match owner {
        "java/lang/Object" => "kotlin/Any",
        "java/lang/String" => "kotlin/String",
        "java/lang/CharSequence" => "kotlin/CharSequence",
        "java/lang/Number" => "kotlin/Number",
        "java/lang/Throwable" => "kotlin/Throwable",
        "java/lang/Cloneable" => "kotlin/Cloneable",
        "java/lang/Comparable" => "kotlin/Comparable",
        "java/lang/Enum" => "kotlin/Enum",
        "java/lang/annotation/Annotation" => "kotlin/Annotation",
        "java/lang/Boolean" => "kotlin/Boolean",
        "java/lang/Byte" => "kotlin/Byte",
        "java/lang/Short" => "kotlin/Short",
        "java/lang/Integer" => "kotlin/Int",
        "java/lang/Long" => "kotlin/Long",
        "java/lang/Character" => "kotlin/Char",
        "java/lang/Float" => "kotlin/Float",
        "java/lang/Double" => "kotlin/Double",
        "java/lang/Iterable" => "kotlin/collections/Iterable",
        "java/util/Iterator" => "kotlin/collections/Iterator",
        "java/util/Collection" => "kotlin/collections/Collection",
        "java/util/List" => "kotlin/collections/List",
        "java/util/ListIterator" => "kotlin/collections/ListIterator",
        "java/util/Set" => "kotlin/collections/Set",
        "java/util/Map" => "kotlin/collections/Map",
        "java/util/Map$Entry" => "kotlin/collections/Map.Entry",
        _ => return None,
    })
}

#[derive(Clone)]
pub(crate) struct Classpath {
    paths: Vec<PathBuf>,
    module_name: String,
    friends: std::collections::HashSet<PathBuf>,
    value_scopes: std::cell::RefCell<std::collections::HashMap<String, value_names::ValueScope>>,
    source_files: std::cell::RefCell<std::collections::HashMap<String, (String, bool)>>,
    source_default_accessors:
        std::cell::RefCell<std::collections::HashMap<(String, String), SourceBackingField>>,
    origins: std::cell::RefCell<std::collections::HashMap<String, PathBuf>>,
    cache: std::cell::RefCell<std::collections::HashMap<String, JavaClass>>,
    passive_annotations: std::cell::RefCell<std::collections::HashMap<String, bool>>,
    package_names: std::cell::RefCell<std::collections::HashMap<String, Vec<String>>>,
    sources: std::cell::RefCell<std::collections::HashSet<String>>,
    package_index: std::cell::RefCell<std::collections::HashMap<String, PackageIndex>>,
}

#[derive(Clone)]
pub(crate) struct SourceBackingField {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) is_static: bool,
    pub(crate) companion_bridge: bool,
}

#[derive(Clone, Default)]
struct PackageIndex {
    constants: std::collections::HashMap<String, Vec<JavaField>>,
    methods: std::collections::HashMap<String, Vec<JavaMethod>>,
    aliases: std::collections::HashMap<String, Vec<LibraryTypeAlias>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AnnotationRetention {
    Source,
    Class,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AnnotationDefinition {
    pub kotlin_parameters: Option<Vec<String>>,
    pub kotlin_types: Vec<Type>,
    pub kotlin_targets: Option<Vec<String>>,
    pub vararg: Option<String>,
    pub retention: AnnotationRetention,
    pub targets: Vec<String>,
    pub parameters: Vec<(String, String, bool)>,
}

#[derive(Clone, Debug)]
pub(crate) struct JavaClass {
    pub sealed_: bool,
    pub sealed_subclasses: Vec<String>,
    pub annotation_definition: Option<AnnotationDefinition>,
    pub kotlin_fun_interface: bool,
    pub library: Option<LibraryClass>,
    pub name: String,
    pub access: u16,
    pub super_name: Option<String>,
    pub interfaces: Vec<String>,
    pub methods: Vec<JavaMethod>,
    pub fields: Vec<JavaField>,
    pub kotlin_class: bool,
    pub unsupported: bool,
    pub kotlin_interface: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct JavaField {
    pub kotlin: Option<KotlinField>,
    pub constant: Option<JavaConstant>,
    pub owner: String,
    pub name: String,
    pub descriptor: String,
    pub access: u16,
    pub unsupported: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct KotlinField {
    pub name: String,
    pub ty: Type,
    pub visibility: u8,
    pub constant: bool,
}

#[derive(Clone, Debug)]
pub(crate) enum JavaConstant {
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    String(Vec<u16>),
}

#[derive(Clone, Debug)]
pub(crate) struct JavaMethod {
    pub library: Option<LibraryMethod>,
    pub owner: String,
    pub name: String,
    pub descriptor: String,
    pub access: u16,
    pub unsupported: bool,
    pub source_property: Option<String>,
    pub kotlin_params: Option<Vec<(String, Type, bool)>>,
    pub kotlin_result: Option<Type>,
    pub kotlin_internal: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct LibraryClass {
    pub kotlin_visibility: Option<u8>,
    pub type_aliases: Vec<LibraryTypeAlias>,
    pub type_parameters: Vec<TypeParameter>,
    pub supertypes: Vec<Type>,
    pub package: Option<String>,
    pub multifile_facade: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct LibraryTypeAlias {
    pub name: String,
    pub type_parameters: Vec<TypeParameter>,
    pub expanded: Type,
}

#[derive(Clone, Debug)]
pub(crate) struct LibraryMethod {
    pub has_annotations: bool,
    pub abstract_: bool,
    pub builtin_array: bool,
    pub kotlin_public: bool,
    pub source_name: String,
    pub type_parameters: Vec<TypeParameter>,
    pub receiver: Option<Type>,
    pub infix: bool,
    pub operator: bool,
    pub inline: bool,
    pub code: Option<JavaCode>,
}

#[derive(Clone, Debug)]
pub(crate) struct JavaCode {
    pub max_stack: u16,
    pub max_locals: u16,
    pub bytes: Vec<u8>,
    pub exception_handlers: u16,
    pub constants: std::sync::Arc<std::collections::BTreeMap<u16, InlineConstant>>,
}

#[derive(Clone, Debug)]
pub(crate) enum InlineConstant {
    Class(String),
    Integer(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    String(Vec<u16>),
    Field {
        owner: String,
        name: String,
        descriptor: String,
    },
    Method {
        owner: String,
        name: String,
        descriptor: String,
        interface: bool,
    },
}

impl JavaMethod {
    pub(crate) fn source_name(&self) -> &str {
        self.library
            .as_ref()
            .map_or(self.name.as_str(), |library| library.source_name.as_str())
    }
}

impl Classpath {
    pub(crate) fn expand_type_alias(
        &self,
        ty: &Type,
        imports: &std::collections::HashMap<String, String>,
        packages: &[&str],
    ) -> Result<Type> {
        aliases::expand(self, ty, imports, packages)
    }

    pub(crate) fn builtin_class_declarations(&self, classifier: &str) -> Result<Vec<JavaMethod>> {
        metadata::builtin_class_declarations(&self.builtin_class_resource(classifier)?, classifier)
    }

    pub(crate) fn builtin_constructors(&self, classifier: &str) -> Result<Vec<JavaMethod>> {
        metadata::builtin_constructors(&self.builtin_class_resource(classifier)?, classifier)
    }

    pub(crate) fn builtin_member_declarations(
        &self,
        classifier: &str,
        name: &str,
    ) -> Result<Vec<JavaMethod>> {
        metadata::builtin_member_declarations(
            &self.builtin_class_resource(classifier)?,
            classifier,
            name,
        )
    }

    fn builtin_class_resource(&self, classifier: &str) -> Result<Vec<u8>> {
        if !valid_name(classifier) {
            return Err(invalid());
        }
        let (package, _) = classifier.rsplit_once('/').ok_or_else(invalid)?;
        let resource = format!(
            "{package}/{}.kotlin_builtins",
            package.rsplit('/').next().unwrap()
        );
        for path in &self.paths {
            if let Some(bytes) = archive::read(path, &resource)? {
                return Ok(bytes);
            }
        }
        Err(Diagnostic::new(
            0,
            format!("Kotlin builtin declarations are missing for {classifier}"),
        )
        .with_kind(
            crate::DiagnosticKind::MissingDependency,
            "BUILTIN_DECLARATIONS_NOT_FOUND",
        ))
    }
    pub(crate) fn type_alias(&self, package: &str, name: &str) -> Result<Option<LibraryTypeAlias>> {
        let package = self.index_package(package)?;
        let index = self.package_index.borrow();
        let aliases = index[&package]
            .aliases
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        match aliases {
            [] => Ok(None),
            [alias] => Ok(Some(alias.clone())),
            _ => Err(Diagnostic::new(0, "Ambiguous library type alias")),
        }
    }

    pub(crate) fn package_methods(&self, package: &str, name: &str) -> Result<Vec<JavaMethod>> {
        let package = self.index_package(package)?;
        Ok(self.package_index.borrow()[&package]
            .methods
            .get(name)
            .cloned()
            .unwrap_or_default())
    }

    fn index_package(&self, package: &str) -> Result<String> {
        let package = package.replace('.', "/");
        if self.package_index.borrow().contains_key(&package) {
            return Ok(package);
        }
        let mut index = PackageIndex::default();
        for class in self.package_facades(&package)? {
            if class
                .library
                .as_ref()
                .is_some_and(|library| library.package.is_some())
            {
                for field in &class.fields {
                    if let Some(kotlin) = field
                        .kotlin
                        .as_ref()
                        .filter(|kotlin| kotlin.constant && field.constant.is_some())
                    {
                        index
                            .constants
                            .entry(kotlin.name.clone())
                            .or_default()
                            .push(field.clone());
                    }
                }
            }
            for method in class.methods {
                let mut names = std::collections::HashSet::new();
                names.insert(method.name.clone());
                if let Some(name) = &method.source_property {
                    names.insert(name.clone());
                }
                if let Some(library) = &method.library {
                    names.insert(library.source_name.clone());
                }
                for name in names {
                    index.methods.entry(name).or_default().push(method.clone());
                }
            }
            if let Some(library) = class.library {
                for alias in library.type_aliases {
                    index
                        .aliases
                        .entry(alias.name.clone())
                        .or_default()
                        .push(alias);
                }
            }
        }
        self.package_index
            .borrow_mut()
            .insert(package.clone(), index);
        Ok(package)
    }

    fn builtin_array_class(&self, package: &str) -> Result<Option<JavaClass>> {
        if package != "kotlin" {
            return Ok(None);
        }
        const OWNER: &str = "kotlin/<builtins>";
        if let Some(class) = self.cache.borrow().get(OWNER).cloned() {
            return Ok(Some(class));
        }
        for path in &self.paths {
            if let Some(bytes) = archive::read(path, "kotlin/kotlin.kotlin_builtins")? {
                let methods = metadata::builtin_array_factories(&bytes)?;
                let class = JavaClass {
                    sealed_: false,
                    sealed_subclasses: Vec::new(),
                    annotation_definition: None,
                    kotlin_fun_interface: false,
                    library: Some(LibraryClass {
                        kotlin_visibility: None,
                        type_aliases: Vec::new(),
                        type_parameters: Vec::new(),
                        supertypes: Vec::new(),
                        package: Some("kotlin".into()),
                        multifile_facade: None,
                    }),
                    name: OWNER.into(),
                    access: 1,
                    super_name: Some("java/lang/Object".into()),
                    interfaces: Vec::new(),
                    methods,
                    fields: Vec::new(),
                    kotlin_class: true,
                    unsupported: false,
                    kotlin_interface: false,
                };
                self.cache.borrow_mut().insert(OWNER.into(), class.clone());
                return Ok(Some(class));
            }
        }
        Ok(None)
    }

    pub(crate) fn package_constants(&self, package: &str, name: &str) -> Result<Vec<JavaField>> {
        let package = self.index_package(package)?;
        Ok(self
            .package_index
            .borrow()
            .get(&package)
            .unwrap()
            .constants
            .get(name)
            .cloned()
            .unwrap_or_default())
    }

    fn package_facades(&self, package: &str) -> Result<Vec<JavaClass>> {
        let mut names = std::collections::BTreeSet::new();
        if let Some(class) = self.builtin_array_class(package)? {
            names.insert(class.name);
        }
        for name in self.sources.borrow().iter() {
            if name.rsplit_once('/').map_or("", |(package, _)| package) == package
                && self.cache.borrow().get(name).is_some_and(|class| {
                    class
                        .library
                        .as_ref()
                        .is_some_and(|library| library.package.is_some())
                })
            {
                names.insert(name.clone());
            }
        }
        for path in &self.paths {
            let entries = archive::names(path, package)?;
            let mut mappings = false;
            for name in &entries {
                if name.ends_with(".kotlin_module") {
                    mappings = true;
                    let bytes = archive::read(path, name)?.ok_or_else(invalid)?;
                    names.extend(metadata::module_parts(&bytes, package)?);
                }
            }
            if !mappings {
                for name in entries {
                    if let Some(name) = name.strip_suffix(".class") {
                        names.insert(name.into());
                    }
                }
            }
        }
        names.into_iter().map(|name| self.load(&name)).collect()
    }
    pub(crate) fn register_source_file(&self, owner: &str, identity: &str, facade: bool) {
        self.source_files
            .borrow_mut()
            .insert(owner.into(), (identity.into(), facade));
    }

    pub(crate) fn same_source_file(&self, first: &str, second: &str) -> bool {
        let files = self.source_files.borrow();
        files
            .get(first)
            .zip(files.get(second))
            .is_some_and(|(a, b)| a.0 == b.0)
    }

    pub(crate) fn file_private_visible(&self, caller: &str, method: &JavaMethod) -> bool {
        method.access & 2 != 0
            && self
                .source_files
                .borrow()
                .get(&method.owner)
                .is_some_and(|(_, facade)| *facade)
            && self.same_source_file(caller, &method.owner)
    }

    pub(crate) fn internal_visible(&self, owner: &str) -> bool {
        self.is_source(owner)
            || self
                .origins
                .borrow()
                .get(owner)
                .is_some_and(|origin| self.friends.contains(origin))
    }

    pub(crate) fn source_jvm_name(&self, owner: &str, name: &str, internal: bool) -> String {
        if internal
            && !self
                .source_files
                .borrow()
                .get(owner)
                .is_some_and(|(_, facade)| *facade)
        {
            format!(
                "{name}${}",
                self.module_name
                    .chars()
                    .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                    .collect::<String>()
            )
        } else {
            name.into()
        }
    }

    pub(crate) fn source_methods(&self) -> Vec<JavaMethod> {
        self.cache
            .borrow()
            .values()
            .filter(|class| self.is_source(&class.name))
            .flat_map(|class| class.methods.clone())
            .collect()
    }

    pub(crate) fn apply_source_names(&self) {
        for class in self.cache.borrow_mut().values_mut() {
            if self.is_source(&class.name) {
                for method in &mut class.methods {
                    if method.kotlin_internal && !method.name.starts_with('<') {
                        method.name =
                            self.source_jvm_name(&method.owner, method.source_name(), true);
                    }
                }
            }
        }
        self.package_index.borrow_mut().clear();
    }

    pub(crate) fn register_source(&self, class: JavaClass) {
        if !self.source_files.borrow().contains_key(&class.name) {
            if let Some(parent) = class.name.strip_suffix("$Companion") {
                let origin = { self.source_files.borrow().get(parent).cloned() };
                if let Some((identity, _)) = origin {
                    self.source_files
                        .borrow_mut()
                        .insert(class.name.clone(), (identity, false));
                }
            }
        }
        self.sources.borrow_mut().insert(class.name.clone());
        let package = class
            .name
            .rsplit_once('/')
            .map_or("", |(package, _)| package);
        self.package_names.borrow_mut().remove(package);
        self.package_index.borrow_mut().remove(package);
        self.cache.borrow_mut().insert(class.name.clone(), class);
    }

    pub(crate) fn is_source(&self, name: &str) -> bool {
        self.sources.borrow().contains(name)
    }

    pub(crate) fn register_source_output(&self, class: &crate::ClassFile) -> Result<()> {
        self.register_source(parse(&class.bytes, &class.internal_name)?);
        Ok(())
    }

    pub(crate) fn source_default_imports(&self, module: &mut crate::syntax::Module) -> Result<()> {
        source::defaults(module, self);
        source::expand_aliases(module, self)
    }

    pub(crate) fn default_source_accessor(
        &self,
        method: &JavaMethod,
    ) -> Option<SourceBackingField> {
        (method.access & 2 != 0 && method.source_property.is_some())
            .then(|| {
                self.source_default_accessors
                    .borrow()
                    .get(&(method.owner.clone(), method.name.clone()))
                    .cloned()
            })
            .flatten()
    }
    pub(crate) fn source_header(&self, module: &crate::syntax::Module, owner: &str) -> Result<()> {
        self.register_source(source::header(module, owner, self)?);
        Ok(())
    }
    #[cfg(test)]
    pub fn new(paths: &[PathBuf]) -> Result<Self> {
        Self::with_options(paths, &crate::CompileOptions::default())
    }

    pub(crate) fn with_options(paths: &[PathBuf], options: &crate::CompileOptions) -> Result<Self> {
        archive::validate(paths)?;
        let friends = options
            .friend_paths
            .iter()
            .map(|path| {
                std::fs::canonicalize(path).map_err(|error| {
                    Diagnostic::new(
                        0,
                        format!("Invalid friend path {}: {error}", path.display()),
                    )
                    .with_kind(crate::DiagnosticKind::Setup, "INVALID_FRIEND_PATH")
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            paths: paths.to_vec(),
            module_name: options.module_name.clone(),
            friends,
            value_scopes: Default::default(),
            source_files: Default::default(),
            source_default_accessors: Default::default(),
            origins: Default::default(),
            cache: Default::default(),
            passive_annotations: Default::default(),
            package_names: Default::default(),
            sources: Default::default(),
            package_index: Default::default(),
        })
    }

    pub fn load(&self, internal_name: &str) -> Result<JavaClass> {
        if let Some(class) = self.cache.borrow().get(internal_name) {
            return Ok(class.clone());
        }
        let class = self.load_uncached(internal_name)?;
        self.cache
            .borrow_mut()
            .insert(internal_name.into(), class.clone());
        Ok(class)
    }

    fn load_uncached(&self, internal_name: &str) -> Result<JavaClass> {
        if !valid_name(internal_name) {
            return Err(Diagnostic::new(0, "Invalid Java class internal name"));
        }
        let package_info = internal_name
            .rsplit_once('/')
            .map(|(p, _)| format!("{p}/package-info.class"))
            .unwrap_or_else(|| "package-info.class".into());
        for path in &self.paths {
            if archive::read(path, &package_info)?.is_some() {
                return Err(Diagnostic::new(
                    0,
                    format!(
                        "Java package metadata {} is unsupported because it may define nullability defaults",
                        path.display()
                    ),
                ));
            }
        }
        for path in &self.paths {
            if let Some(bytes) = archive::read(path, &format!("{internal_name}.class"))? {
                let class = self
                    .parse_with_declarations(&bytes, internal_name)
                    .map_err(|mut e| {
                        e.message = format!("{}!{internal_name}: {}", path.display(), e.message);
                        e
                    })?;
                let origin = std::fs::canonicalize(path)
                    .map_err(|error| Diagnostic::new(0, error.to_string()))?;
                self.origins
                    .borrow_mut()
                    .insert(internal_name.into(), origin);
                return Ok(class);
            }
        }
        if internal_name.starts_with("java/") || internal_name.starts_with("javax/") {
            if let Some(java_home) = std::env::var_os("JAVA_HOME") {
                let jmod = PathBuf::from(java_home).join("jmods/java.base.jmod");
                if jmod.is_file() {
                    if let Some(bytes) =
                        archive::read(&jmod, &format!("classes/{internal_name}.class"))?
                    {
                        return self.parse_with_declarations(&bytes, internal_name);
                    }
                }
            }
        }
        Err(Diagnostic::new(
            0,
            format!(
                "Java class {} was not found on the classpath",
                internal_name.replace('/', ".")
            ),
        )
        .with_kind(crate::DiagnosticKind::MissingDependency, "CLASS_NOT_FOUND"))
    }

    fn parse_with_declarations(&self, bytes: &[u8], expected: &str) -> Result<JavaClass> {
        let (mut class, attrs, method_attrs) = parse_details(bytes, expected)?;
        if !attrs
            .annotations
            .iter()
            .any(|annotation| annotation.name == "Lkotlin/Metadata;")
            && class.access & 0x2000 == 0
        {
            let _ = signature::attach_with_declarations(
                &mut class,
                &attrs,
                &method_attrs,
                &mut |annotation| self.passive_declaration_annotation(annotation),
            );
        }
        Ok(class)
    }

    fn passive_declaration_annotation(&self, annotation: &Annotation) -> Result<bool> {
        if signature::type_semantic_annotation(&annotation.name) {
            return Ok(false);
        }
        if let Some(passive) = self.passive_annotations.borrow().get(&annotation.name) {
            return Ok(*passive);
        }
        let passive = self.inspect_declaration_annotation(annotation)?;
        self.passive_annotations
            .borrow_mut()
            .insert(annotation.name.clone(), passive);
        Ok(passive)
    }

    fn inspect_declaration_annotation(&self, annotation: &Annotation) -> Result<bool> {
        let owner = annotation
            .name
            .strip_prefix('L')
            .and_then(|name| name.strip_suffix(';'))
            .ok_or_else(invalid)?;
        let resource = format!("{owner}.class");
        let mut bytes = None;
        for path in &self.paths {
            if let Some(found) = archive::read(path, &resource)? {
                bytes = Some(found);
                break;
            }
        }
        if bytes.is_none()
            && let Some(java_home) = std::env::var_os("JAVA_HOME")
        {
            let jmod = PathBuf::from(java_home).join("jmods/java.base.jmod");
            if jmod.is_file() {
                bytes = archive::read(&jmod, &format!("classes/{resource}"))?;
            }
        }
        let Some(bytes) = bytes else {
            return Ok(true);
        };
        let (_, attrs, _) = parse_details(&bytes, owner)?;
        Ok(!attrs.annotations.iter().any(|annotation| {
            matches!(
                annotation.name.as_str(),
                "Ljavax/annotation/meta/TypeQualifier;"
                    | "Ljavax/annotation/meta/TypeQualifierNickname;"
                    | "Ljavax/annotation/meta/TypeQualifierDefault;"
            )
        }))
    }

    pub fn package_classes(&self, package: &str) -> Result<Vec<JavaClass>> {
        let package = package.replace('.', "/");
        let builtin = self.builtin_array_class(&package)?;
        if !package.is_empty() && !valid_name(&package) {
            return Err(invalid());
        }
        if let Some(names) = self.package_names.borrow().get(&package) {
            return names.iter().map(|name| self.load(name)).collect();
        }
        let mut names = std::collections::BTreeSet::new();
        if let Some(class) = builtin {
            names.insert(class.name);
        }

        for name in self.sources.borrow().iter() {
            if name.rsplit_once('/').map_or("", |(package, _)| package) == package {
                names.insert(name.clone());
            }
        }
        for path in &self.paths {
            for name in archive::names(path, &package)? {
                if name.ends_with(".kotlin_module") {
                    let bytes = archive::read(path, &name)?.ok_or_else(invalid)?;
                    for part in metadata::module_parts(&bytes, &package)? {
                        names.insert(part);
                    }
                } else if let Some(class) = name.strip_suffix(".class") {
                    names.insert(class.to_owned());
                }
            }
        }
        let names = names.into_iter().collect::<Vec<_>>();
        let classes = names
            .iter()
            .map(|name| self.load(name))
            .collect::<Result<_>>()?;
        self.package_names.borrow_mut().insert(package, names);
        Ok(classes)
    }

    pub(crate) fn validate_source_sealed(
        &self,
        owners: &std::collections::HashSet<String>,
    ) -> std::result::Result<(), (String, Diagnostic)> {
        sealed::validate(self, owners)
    }
    pub fn methods(&self, internal_name: &str) -> Result<Vec<JavaMethod>> {
        self.hierarchy_methods(internal_name, false)
    }

    pub(crate) fn ordinary_methods(&self, internal_name: &str) -> Result<Vec<JavaMethod>> {
        self.hierarchy_methods(internal_name, true)
    }

    fn hierarchy_methods(
        &self,
        internal_name: &str,
        include_object: bool,
    ) -> Result<Vec<JavaMethod>> {
        fn visit(
            cp: &Classpath,
            name: &str,
            include_object: bool,
            seen: &mut std::collections::HashSet<String>,
            methods: &mut Vec<JavaMethod>,
        ) -> Result<()> {
            if !seen.insert(name.into()) || !include_object && name == "java/lang/Object" {
                return Ok(());
            }
            if seen.len() > 256 {
                return Err(invalid());
            }
            let class = cp.load(name)?;
            for method in class.methods {
                if !methods
                    .iter()
                    .any(|m| m.name == method.name && m.descriptor == method.descriptor)
                {
                    methods.push(method);
                }
            }
            for parent in class.super_name.iter().chain(class.interfaces.iter()) {
                visit(cp, parent, include_object, seen, methods)?;
            }
            Ok(())
        }
        let mut methods = Vec::new();
        visit(
            self,
            internal_name,
            include_object,
            &mut std::collections::HashSet::new(),
            &mut methods,
        )?;
        Ok(methods)
    }

    pub(crate) fn field_candidates(
        &self,
        internal_name: &str,
        name: &str,
    ) -> Result<Vec<JavaField>> {
        fn visit(
            cp: &Classpath,
            owner: &str,
            name: &str,
            seen: &mut std::collections::HashSet<String>,
        ) -> Result<Vec<JavaField>> {
            if !seen.insert(owner.into()) {
                return Ok(Vec::new());
            }
            if seen.len() > 256 {
                return Err(invalid());
            }
            let class = cp.load(owner)?;
            let declared = class
                .fields
                .into_iter()
                .filter(|field| field.name == name)
                .collect::<Vec<_>>();
            if !declared.is_empty() {
                return Ok(declared);
            }
            let mut fields = Vec::new();
            for parent in class.super_name.iter().chain(class.interfaces.iter()) {
                for field in visit(cp, parent, name, seen)? {
                    if !fields.iter().any(|previous: &JavaField| {
                        previous.owner == field.owner && previous.name == field.name
                    }) {
                        fields.push(field);
                    }
                }
            }
            Ok(fields)
        }
        visit(
            self,
            internal_name,
            name,
            &mut std::collections::HashSet::new(),
        )
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
    Utf8(String, Vec<u16>),
    Class(u16),
    Integer(u32),
    Float(u32),
    Double(u64),
    Long(i64),
    String(u16),
    NameAndType(u16, u16),
    Reference(u8, u16, u16),
}
fn utf8(pool: &[Constant], index: u16) -> Result<&str> {
    match pool.get(usize::from(index)) {
        Some(Constant::Utf8(value, _)) => Ok(value),
        _ => Err(invalid()),
    }
}
fn class_name(pool: &[Constant], index: u16) -> Result<&str> {
    match pool.get(usize::from(index)) {
        Some(Constant::Class(value)) => utf8(pool, *value),
        _ => Err(invalid()),
    }
}

fn inline_constants(pool: &[Constant]) -> Result<std::collections::BTreeMap<u16, InlineConstant>> {
    let mut result = std::collections::BTreeMap::new();
    for (index, value) in pool.iter().enumerate() {
        let value = match value {
            Constant::Class(name) => Some(InlineConstant::Class(utf8(pool, *name)?.into())),
            Constant::Integer(value) => Some(InlineConstant::Integer(*value as i32)),
            Constant::Long(value) => Some(InlineConstant::Long(*value)),
            Constant::Float(value) => Some(InlineConstant::Float(*value)),
            Constant::Double(value) => Some(InlineConstant::Double(*value)),
            Constant::String(index) => match pool.get(usize::from(*index)) {
                Some(Constant::Utf8(_, units)) => Some(InlineConstant::String(units.clone())),
                _ => return Err(invalid()),
            },
            Constant::Reference(tag, owner, signature) => {
                let owner = class_name(pool, *owner)?.to_owned();
                let Some(Constant::NameAndType(name, descriptor)) =
                    pool.get(usize::from(*signature))
                else {
                    return Err(invalid());
                };
                let name = utf8(pool, *name)?.to_owned();
                let descriptor = utf8(pool, *descriptor)?.to_owned();
                Some(if *tag == 9 {
                    InlineConstant::Field {
                        owner,
                        name,
                        descriptor,
                    }
                } else {
                    InlineConstant::Method {
                        owner,
                        name,
                        descriptor,
                        interface: *tag == 11,
                    }
                })
            }
            _ => None,
        };
        if let Some(value) = value {
            result.insert(u16::try_from(index).map_err(|_| invalid())?, value);
        }
    }
    Ok(result)
}

fn modified_utf8_units(bytes: &[u8]) -> Result<Vec<u16>> {
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
    Ok(units)
}

#[cfg(test)]
fn modified_utf8(bytes: &[u8]) -> Result<String> {
    Ok(String::from_utf16_lossy(&modified_utf8_units(bytes)?))
}

#[derive(Debug)]
enum AnnotationValue {
    Integer(u32),
    String(String),
    Enum { descriptor: String, name: String },
    Array(Vec<AnnotationValue>),
    Other,
}

#[derive(Debug)]
struct Annotation {
    name: String,
    values: Vec<(String, AnnotationValue)>,
}

#[derive(Default)]
struct Attributes {
    unsupported: bool,
    annotated: bool,
    annotations: Vec<Annotation>,
    parameter_annotations: Vec<Vec<Annotation>>,
    return_type_annotations: Vec<Annotation>,
    parameter_type_annotations: Vec<(usize, Annotation)>,
    only_input_type_parameters: Vec<usize>,
    type_annotation_targets: Vec<u8>,
    type_annotation_parameter_indices: Vec<usize>,
    signature: Option<String>,
    constant: Option<u16>,
    code: Option<JavaCode>,
    annotation_default: bool,
    annotation_default_payload: Option<Vec<u8>>,
}

fn annotation_value(
    reader: &mut Reader<'_>,
    pool: &[Constant],
    depth: usize,
) -> Result<AnnotationValue> {
    if depth > 32 {
        return Err(invalid());
    }
    Ok(match reader.u1()? {
        b'I' | b'Z' => match pool.get(usize::from(reader.u2()?)) {
            Some(Constant::Integer(value)) => AnnotationValue::Integer(*value),
            _ => return Err(invalid()),
        },
        b's' => AnnotationValue::String(utf8(pool, reader.u2()?)?.to_owned()),
        b'[' => {
            let count = reader.u2()?;
            let mut values = Vec::new();
            for _ in 0..count {
                values.push(annotation_value(reader, pool, depth + 1)?);
            }
            AnnotationValue::Array(values)
        }
        b'@' => {
            annotation(reader, pool, depth + 1)?;
            AnnotationValue::Other
        }
        b'e' => AnnotationValue::Enum {
            descriptor: utf8(pool, reader.u2()?)?.to_owned(),
            name: utf8(pool, reader.u2()?)?.to_owned(),
        },
        b'c' => {
            utf8(pool, reader.u2()?)?;
            AnnotationValue::Other
        }
        tag @ (b'B' | b'C' | b'D' | b'F' | b'J' | b'S') => {
            let constant = pool.get(usize::from(reader.u2()?));
            if !matches!(
                (tag, constant),
                (b'B' | b'C' | b'S', Some(Constant::Integer(_)))
                    | (b'D', Some(Constant::Double(_)))
                    | (b'F', Some(Constant::Float(_)))
                    | (b'J', Some(Constant::Long(_)))
            ) {
                return Err(invalid());
            }
            AnnotationValue::Other
        }
        _ => return Err(invalid()),
    })
}

fn annotation(reader: &mut Reader<'_>, pool: &[Constant], depth: usize) -> Result<Annotation> {
    let name = utf8(pool, reader.u2()?)?.to_owned();
    let mut values = Vec::new();
    for _ in 0..reader.u2()? {
        let key = utf8(pool, reader.u2()?)?.to_owned();
        if values.iter().any(|(name, _)| name == &key) {
            return Err(invalid());
        }
        values.push((key, annotation_value(reader, pool, depth)?));
    }
    Ok(Annotation { name, values })
}

fn type_annotation_parameter_count(descriptor: &str) -> Result<usize> {
    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return Err(invalid());
    }
    let mut cursor = 1;
    let mut count = 0;
    while bytes.get(cursor) != Some(&b')') {
        while bytes.get(cursor) == Some(&b'[') {
            cursor += 1;
        }
        match bytes.get(cursor) {
            Some(b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z') => cursor += 1,
            Some(b'L') => {
                let end = descriptor[cursor + 1..].find(';').ok_or_else(invalid)? + cursor + 1;
                if !valid_name(&descriptor[cursor + 1..end]) {
                    return Err(invalid());
                }
                cursor = end + 1;
            }
            _ => return Err(invalid()),
        }
        count += 1;
        if count > 255 {
            return Err(invalid());
        }
    }
    Ok(count)
}

fn validate_type_annotation_parameters(descriptor: &str, attributes: &Attributes) -> Result<()> {
    if !attributes.type_annotation_parameter_indices.is_empty() {
        let count = type_annotation_parameter_count(descriptor)?;
        if attributes
            .type_annotation_parameter_indices
            .iter()
            .any(|index| *index >= count)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

fn attributes(
    reader: &mut Reader<'_>,
    pool: &[Constant],
    constants: &std::sync::Arc<std::collections::BTreeMap<u16, InlineConstant>>,
) -> Result<Attributes> {
    let mut result = Attributes::default();
    let mut type_attribute_names = std::collections::HashSet::new();
    for _ in 0..reader.u2()? {
        let name = utf8(pool, reader.u2()?)?;
        let length = usize::try_from(reader.u4()?).map_err(|_| invalid())?;
        let bytes = reader.take(length)?;
        let mut data = Reader { bytes, offset: 0 };
        result.annotated |= name.contains("Annotations");
        match name {
            "RuntimeVisibleAnnotations" | "RuntimeInvisibleAnnotations" => {
                for _ in 0..data.u2()? {
                    result.annotations.push(annotation(&mut data, pool, 0)?);
                }
            }
            "RuntimeVisibleParameterAnnotations" | "RuntimeInvisibleParameterAnnotations" => {
                let count = usize::from(data.u1()?);
                if !result.parameter_annotations.is_empty()
                    && result.parameter_annotations.len() != count
                {
                    return Err(invalid());
                }
                result.parameter_annotations.resize_with(count, Vec::new);
                for annotations in &mut result.parameter_annotations {
                    for _ in 0..data.u2()? {
                        annotations.push(annotation(&mut data, pool, 0)?);
                    }
                }
            }
            "RuntimeVisibleTypeAnnotations" | "RuntimeInvisibleTypeAnnotations" => {
                if !type_attribute_names.insert(name) {
                    return Err(invalid());
                }
                for _ in 0..data.u2()? {
                    let target = data.u1()?;
                    result.type_annotation_targets.push(target);
                    let parameter = match target {
                        0x00 | 0x01 | 0x16 => Some(usize::from(data.u1()?)),
                        0x10 | 0x17 | 0x42..=0x46 => {
                            data.u2()?;
                            None
                        }
                        0x11 | 0x12 => {
                            data.u1()?;
                            data.u1()?;
                            None
                        }
                        0x13..=0x15 => None,
                        0x40 | 0x41 => {
                            for _ in 0..data.u2()? {
                                data.u2()?;
                                data.u2()?;
                                data.u2()?;
                            }
                            None
                        }
                        0x47..=0x4b => {
                            data.u2()?;
                            data.u1()?;
                            None
                        }
                        _ => return Err(invalid()),
                    };
                    if target == 0x16 {
                        result
                            .type_annotation_parameter_indices
                            .push(parameter.ok_or_else(invalid)?);
                    }
                    let path_length = data.u1()?;
                    for _ in 0..path_length {
                        let kind = data.u1()?;
                        let index = data.u1()?;
                        if kind > 3 || kind != 3 && index != 0 {
                            return Err(invalid());
                        }
                    }
                    let value = annotation(&mut data, pool, 0)?;
                    if target == 0x01
                        && path_length == 0
                        && value.name == "Lkotlin/internal/OnlyInputTypes;"
                        && value.values.is_empty()
                    {
                        result
                            .only_input_type_parameters
                            .push(parameter.ok_or_else(invalid)?);
                    } else if path_length != 0 || !matches!(target, 0x14 | 0x16) {
                        result.unsupported = true;
                    } else if target == 0x14 {
                        result.return_type_annotations.push(value);
                    } else {
                        result
                            .parameter_type_annotations
                            .push((parameter.ok_or_else(invalid)?, value));
                    }
                }
            }
            "Signature" => {
                if result.signature.is_some() {
                    return Err(invalid());
                }
                result.signature = Some(utf8(pool, data.u2()?)?.to_owned());
            }
            "Code" => {
                if result.code.is_some() {
                    return Err(invalid());
                }
                let max_stack = data.u2()?;
                let max_locals = data.u2()?;
                let size = usize::try_from(data.u4()?).map_err(|_| invalid())?;
                if size > 65535 {
                    return Err(invalid());
                }
                let code = data.take(size)?.to_vec();
                let exception_handlers = data.u2()?;
                data.take(usize::from(exception_handlers) * 8)?;
                for _ in 0..data.u2()? {
                    utf8(pool, data.u2()?)?;
                    let size = usize::try_from(data.u4()?).map_err(|_| invalid())?;
                    data.take(size)?;
                }
                result.code = Some(JavaCode {
                    max_stack,
                    max_locals,
                    bytes: code,
                    exception_handlers,
                    constants: constants.clone(),
                });
            }
            "AnnotationDefault" => {
                if result.annotation_default {
                    return Err(invalid());
                }
                result.annotation_default_payload = Some(data.bytes.to_vec());
                annotation_value(&mut data, pool, 0)?;
                result.annotation_default = true;
            }
            "ConstantValue" => {
                if result.constant.is_some() {
                    return Err(invalid());
                }
                result.constant = Some(data.u2()?);
            }
            _ => {
                result.unsupported |= name.contains("Annotations");
                continue;
            }
        }
        if data.offset != bytes.len() {
            return Err(invalid());
        }
    }
    Ok(result)
}

#[derive(Clone, Copy)]
enum ProtoValue<'a> {
    Integer(u64),
    Bytes(&'a [u8]),
}

struct Proto<'a>(Vec<(u32, ProtoValue<'a>)>);

fn varint(reader: &mut Reader<'_>) -> Result<u64> {
    let mut value = 0;
    for shift in (0..70).step_by(7) {
        let byte = reader.u1()?;
        if shift == 63 && byte > 1 {
            return Err(invalid());
        }
        value |= u64::from(byte & 127) << shift;
        if byte & 128 == 0 {
            return Ok(value);
        }
    }
    Err(invalid())
}

impl<'a> Proto<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut reader = Reader { bytes, offset: 0 };
        let mut fields = Vec::new();
        while reader.offset < bytes.len() {
            let key = varint(&mut reader)?;
            let number = u32::try_from(key >> 3).map_err(|_| invalid())?;
            if number == 0 {
                return Err(invalid());
            }
            let value = match key & 7 {
                0 => ProtoValue::Integer(varint(&mut reader)?),
                2 => {
                    let count = usize::try_from(varint(&mut reader)?).map_err(|_| invalid())?;
                    ProtoValue::Bytes(reader.take(count)?)
                }
                _ => return Err(invalid()),
            };
            fields.push((number, value));
        }
        Ok(Self(fields))
    }
    fn allowed(&self, numbers: &[u32]) -> Result<()> {
        if self.0.iter().all(|(number, _)| numbers.contains(number)) {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn integer(&self, number: u32) -> Result<Option<u64>> {
        let mut values = self.0.iter().filter(|(key, _)| *key == number);
        let result = match values.next() {
            None => None,
            Some((_, ProtoValue::Integer(value))) => Some(*value),
            _ => return Err(invalid()),
        };
        if values.next().is_some() {
            return Err(invalid());
        }
        Ok(result)
    }
    fn bytes(&self, number: u32) -> Result<Option<&'a [u8]>> {
        let mut values = self.messages(number)?;
        let result = values.pop();
        if !values.is_empty() {
            return Err(invalid());
        }
        Ok(result)
    }
    fn messages(&self, number: u32) -> Result<Vec<&'a [u8]>> {
        self.0
            .iter()
            .filter(|(key, _)| *key == number)
            .map(|(_, value)| match value {
                ProtoValue::Bytes(bytes) => Ok(*bytes),
                _ => Err(invalid()),
            })
            .collect()
    }
}

fn metadata_strings(table: &[u8], strings: &[String]) -> Result<Vec<String>> {
    let table = Proto::parse(table)?;
    table.allowed(&[1, 5])?;
    let predefined = [
        "kotlin/Any",
        "kotlin/Nothing",
        "kotlin/Unit",
        "kotlin/Throwable",
        "kotlin/Number",
        "kotlin/Byte",
        "kotlin/Double",
        "kotlin/Float",
        "kotlin/Int",
        "kotlin/Long",
        "kotlin/Short",
        "kotlin/Boolean",
        "kotlin/Char",
        "kotlin/CharSequence",
        "kotlin/String",
        "kotlin/Comparable",
        "kotlin/Enum",
        "kotlin/Array",
        "kotlin/ByteArray",
        "kotlin/DoubleArray",
        "kotlin/FloatArray",
        "kotlin/IntArray",
        "kotlin/LongArray",
        "kotlin/ShortArray",
        "kotlin/BooleanArray",
        "kotlin/CharArray",
        "kotlin/Cloneable",
        "kotlin/Annotation",
        "kotlin/collections/Iterable",
        "kotlin/collections/MutableIterable",
        "kotlin/collections/Collection",
        "kotlin/collections/MutableCollection",
        "kotlin/collections/List",
        "kotlin/collections/MutableList",
        "kotlin/collections/Set",
        "kotlin/collections/MutableSet",
        "kotlin/collections/Map",
        "kotlin/collections/MutableMap",
        "kotlin/collections/Map.Entry",
        "kotlin/collections/MutableMap.MutableEntry",
        "kotlin/collections/Iterator",
        "kotlin/collections/MutableIterator",
        "kotlin/collections/ListIterator",
        "kotlin/collections/MutableListIterator",
    ];
    let mut result = Vec::new();
    for record in table.messages(1)? {
        let record = Proto::parse(record)?;
        record.allowed(&[1, 2, 3, 4, 5, 6])?;
        let range = record.integer(1)?.unwrap_or(1);
        if range == 0 || range > strings.len() as u64 {
            return Err(invalid());
        }
        for _ in 0..range {
            let mut value = strings.get(result.len()).ok_or_else(invalid)?.clone();
            if let Some(index) = record.integer(2)? {
                value = predefined
                    .get(usize::try_from(index).map_err(|_| invalid())?)
                    .ok_or_else(invalid)?
                    .to_string();
            }
            if let Some(bytes) = record.bytes(6)? {
                value = std::str::from_utf8(bytes)
                    .map_err(|_| invalid())?
                    .to_owned();
            }
            let substring = metadata::numbers(&record, 4)?;
            if !substring.is_empty() {
                let [start, end] = substring.as_slice() else {
                    return Err(invalid());
                };
                let units: Vec<u16> = value.encode_utf16().collect();
                value = String::from_utf16(
                    units
                        .get(*start as usize..*end as usize)
                        .ok_or_else(invalid)?,
                )
                .map_err(|_| invalid())?;
            }
            let replacement = metadata::numbers(&record, 5)?;
            if !replacement.is_empty() {
                let [from, to] = replacement.as_slice() else {
                    return Err(invalid());
                };
                let from = char::from_u32(u32::try_from(*from).map_err(|_| invalid())?)
                    .ok_or_else(invalid)?;
                let to = char::from_u32(u32::try_from(*to).map_err(|_| invalid())?)
                    .ok_or_else(invalid)?;
                value = value.replace(from, &to.to_string());
            }
            match record.integer(3)?.unwrap_or(0) {
                0 => {}
                1 => value = value.replace('$', "."),
                2 => {
                    value = value
                        .strip_prefix('L')
                        .and_then(|s| s.strip_suffix(';'))
                        .ok_or_else(invalid)?
                        .replace('$', ".");
                }
                _ => return Err(invalid()),
            }
            result.push(value);
        }
    }
    if result.len() != strings.len() {
        return Err(invalid());
    }
    Ok(result)
}

fn metadata_name(strings: &[String], index: Option<u64>) -> Result<&str> {
    strings
        .get(usize::try_from(index.ok_or_else(invalid)?).map_err(|_| invalid())?)
        .map(String::as_str)
        .ok_or_else(invalid)
}

fn metadata_type<'a>(bytes: &[u8], strings: &'a [String]) -> Result<&'a str> {
    let ty = Proto::parse(bytes)?;
    ty.allowed(&[1, 2, 3, 6])?;
    if ty.integer(1)?.unwrap_or(0) != 0 || ty.integer(3)?.unwrap_or(0) != 0 {
        return Err(invalid());
    }
    let name = metadata_name(strings, ty.integer(6)?)?;
    let arguments = ty.messages(2)?;
    if name == "kotlin/Array" {
        let [argument] = arguments.as_slice() else {
            return Err(invalid());
        };
        let argument = Proto::parse(argument)?;
        argument.allowed(&[1, 2])?;
        let element = Proto::parse(argument.bytes(2)?.ok_or_else(invalid)?)?;
        element.allowed(&[1, 3, 6])?;
        if argument.integer(1)?.unwrap_or(2) != 2
            || element.integer(1)?.unwrap_or(0) != 0
            || element.integer(3)?.unwrap_or(0) != 0
            || metadata_name(strings, element.integer(6)?)? != "kotlin/String"
        {
            return Err(invalid());
        }
        return Ok("[Ljava/lang/String;");
    }
    if !arguments.is_empty() {
        return Err(invalid());
    }
    match name {
        "kotlin/Int" => Ok("I"),
        "kotlin/Boolean" => Ok("Z"),
        "kotlin/Byte" => Ok("B"),
        "kotlin/Short" => Ok("S"),
        "kotlin/Char" => Ok("C"),
        "kotlin/Long" => Ok("J"),
        "kotlin/Float" => Ok("F"),
        "kotlin/Double" => Ok("D"),
        "kotlin/String" => Ok("Ljava/lang/String;"),
        "kotlin/IntArray" => Ok("[I"),
        "kotlin/ByteArray" => Ok("[B"),
        "kotlin/CharArray" => Ok("[C"),
        "kotlin/LongArray" => Ok("[J"),
        "kotlin/BooleanArray" => Ok("[Z"),
        "kotlin/FloatArray" => Ok("[F"),
        "kotlin/DoubleArray" => Ok("[D"),

        "kotlin/Unit" => Ok("V"),
        _ => Err(invalid()),
    }
}

fn metadata_signature<'a>(
    bytes: Option<&[u8]>,
    strings: &'a [String],
    name: &'a str,
    descriptor: &'a str,
) -> Result<(&'a str, &'a str)> {
    let Some(bytes) = bytes else {
        return Ok((name, descriptor));
    };
    let signature = Proto::parse(bytes)?;
    signature.allowed(&[1, 2])?;
    let actual_name = signature
        .integer(1)?
        .map(|n| metadata_name(strings, Some(n)))
        .transpose()?
        .unwrap_or(name);
    let actual_descriptor = signature
        .integer(2)?
        .map(|n| metadata_name(strings, Some(n)))
        .transpose()?
        .unwrap_or(descriptor);
    if actual_name != name || actual_descriptor != descriptor {
        return Err(invalid());
    }
    Ok((actual_name, actual_descriptor))
}

fn nullability_annotations(attributes: &Attributes) -> bool {
    attributes
        .annotations
        .iter()
        .chain(attributes.parameter_annotations.iter().flatten())
        .all(|annotation| {
            annotation.name == "Lorg/jetbrains/annotations/NotNull;" && annotation.values.is_empty()
        })
}

fn kotlin_interface(
    metadata: &Annotation,
    class: &JavaClass,
    attributes: &[Attributes],
) -> Result<Vec<Option<String>>> {
    let mut version = None;
    let mut kind = None;
    let mut extra = None;
    let mut data = None;
    let mut strings = None;
    for (key, value) in &metadata.values {
        match (key.as_str(), value) {
            ("mv", AnnotationValue::Array(values)) => {
                version = Some(
                    values
                        .iter()
                        .map(|v| match v {
                            AnnotationValue::Integer(n) => Ok(*n),
                            _ => Err(invalid()),
                        })
                        .collect::<Result<Vec<_>>>()?,
                );
            }
            ("k", AnnotationValue::Integer(value)) => kind = Some(*value),
            ("xi", AnnotationValue::Integer(value)) => extra = Some(*value),
            ("d1", AnnotationValue::Array(values)) | ("d2", AnnotationValue::Array(values)) => {
                let values = values
                    .iter()
                    .map(|v| match v {
                        AnnotationValue::String(s) => Ok(s.clone()),
                        _ => Err(invalid()),
                    })
                    .collect::<Result<Vec<_>>>()?;
                if key == "d1" {
                    data = Some(values);
                } else {
                    strings = Some(values);
                }
            }
            _ => return Err(invalid()),
        }
    }
    let legacy_marker = version.as_deref() == Some(&[2, 1, 0]) && class.methods.is_empty();
    if (version.as_deref() != Some(&[2, 3, 0]) && !legacy_marker)
        || kind != Some(1)
        || extra != Some(48)
        || class.access != 0x0601
        || class.super_name.as_deref() != Some("java/lang/Object")
    {
        return Err(invalid());
    }
    let data = data.ok_or_else(invalid)?.concat();
    let mut chars = data.chars();
    if chars.next() != Some('\0') {
        return Err(invalid());
    }
    let bytes = chars
        .map(|c| u8::try_from(u32::from(c)).map_err(|_| invalid()))
        .collect::<Result<Vec<_>>>()?;
    let mut reader = Reader {
        bytes: &bytes,
        offset: 0,
    };
    let size = usize::try_from(varint(&mut reader)?).map_err(|_| invalid())?;
    let strings = metadata_strings(reader.take(size)?, &strings.ok_or_else(invalid)?)?;
    let proto = Proto::parse(reader.take(bytes.len() - reader.offset)?)?;
    proto.allowed(&[1, 3, 4, 6, 7, 9, 10, 31, 32, 101, 104])?;
    if proto.integer(31)?.is_some() || proto.bytes(32)?.is_some() {
        if !class.methods.is_empty() || proto.integer(31)? != Some(0) {
            return Err(invalid());
        }
        let table = Proto::parse(proto.bytes(32)?.ok_or_else(invalid)?)?;
        table.allowed(&[1])?;
        let requirements = table.messages(1)?;
        if requirements.len() != 1 {
            return Err(invalid());
        }
        let requirement = Proto::parse(requirements[0])?;
        requirement.allowed(&[1, 2, 3, 6])?;
        let version = if let Some(full) = requirement.integer(2)? {
            (full & 255, (full >> 8) & 255, full >> 16)
        } else {
            let compact = requirement.integer(1)?.ok_or_else(invalid)?;
            (compact & 7, (compact >> 3) & 15, compact >> 7)
        };
        let kind = requirement.integer(6)?.unwrap_or(0);
        let limit = if kind == 1 { (2, 3, 21) } else { (2, 3, 0) };
        if kind > 2 || requirement.integer(3)?.unwrap_or(1) > 2 || version > limit {
            return Err(invalid());
        }
    }
    if proto.integer(1)?.unwrap_or(6) != 102
        || metadata_name(&strings, proto.integer(3)?)?.replace('.', "$") != class.name
    {
        return Err(invalid());
    }
    if let Some(module) = proto.integer(101)? {
        if metadata_name(&strings, Some(module))?.is_empty() {
            return Err(invalid());
        }
    }
    let jvm_flags = proto.integer(104)?.unwrap_or(0);
    if jvm_flags > 3 {
        return Err(invalid());
    }
    let mut supertypes = Vec::new();
    for bytes in proto.messages(6)? {
        let ty = Proto::parse(bytes)?;
        ty.allowed(&[6])?;
        supertypes.push(metadata_name(&strings, ty.integer(6)?)?.replace('.', "$"));
    }
    if class.interfaces.is_empty() {
        if supertypes != ["kotlin/Any"] {
            return Err(invalid());
        }
    } else if supertypes != class.interfaces {
        return Err(invalid());
    }
    let mut seen = vec![false; class.methods.len()];
    let mut properties = vec![None; class.methods.len()];
    let mut validate = |name: &str, descriptor: &str, flags: u64| -> Result<()> {
        if !matches!(flags, 0x16 | 0x26) {
            return Err(invalid());
        }
        let matching: Vec<_> = class
            .methods
            .iter()
            .enumerate()
            .filter(|(_, m)| m.name == name && m.descriptor == descriptor)
            .collect();
        let [(index, method)] = matching.as_slice() else {
            return Err(invalid());
        };
        if seen[*index]
            || method.access != (if flags & 0x30 == 0x20 { 0x0401 } else { 1 })
            || attributes[*index].unsupported
            || attributes[*index].signature.is_some()
            || !nullability_annotations(&attributes[*index])
            || (method.access & 0x0400 == 0 && jvm_flags & 1 == 0)
        {
            return Err(invalid());
        }
        seen[*index] = true;
        Ok(())
    };
    for bytes in proto.messages(9)? {
        let function = Proto::parse(bytes)?;
        function.allowed(&[2, 3, 6, 9, 100])?;
        let name = metadata_name(&strings, function.integer(2)?)?;
        let result = metadata_type(function.bytes(3)?.ok_or_else(invalid)?, &strings)?;
        let mut descriptor = String::from("(");
        for parameter in function.messages(6)? {
            let parameter = Proto::parse(parameter)?;
            parameter.allowed(&[1, 2, 3])?;
            if parameter.integer(1)?.unwrap_or(0) != 0 {
                return Err(invalid());
            }
            metadata_name(&strings, parameter.integer(2)?)?;
            let ty = metadata_type(parameter.bytes(3)?.ok_or_else(invalid)?, &strings)?;
            if ty == "V" {
                return Err(invalid());
            }
            descriptor.push_str(ty);
        }
        descriptor.push(')');
        descriptor.push_str(result);
        metadata_signature(function.bytes(100)?, &strings, name, &descriptor)?;
        validate(name, &descriptor, function.integer(9)?.unwrap_or(6))?;
    }
    for bytes in proto.messages(10)? {
        let property = Proto::parse(bytes)?;
        property.allowed(&[2, 3, 7, 8, 11, 100])?;
        let flags = property.integer(11)?.unwrap_or(518);
        let mutable = flags & 0x100 != 0;
        if flags & !0x736 != 0
            || flags & 0x200 == 0
            || (flags & 0x400 != 0) != mutable
            || mutable && flags & 0x30 != 0x20
            || !mutable && property.integer(8)?.is_some()
        {
            return Err(invalid());
        }
        let name = metadata_name(&strings, property.integer(2)?)?;
        if mutable && !name.is_ascii() {
            return Err(invalid());
        }
        let ty = metadata_type(property.bytes(3)?.ok_or_else(invalid)?, &strings)?;
        if ty == "V" {
            return Err(invalid());
        }
        let getter =
            if name.starts_with("is") && name.chars().nth(2).is_some_and(|c| !c.is_lowercase()) {
                name.to_owned()
            } else {
                let mut chars = name.chars();
                let first = chars.next().ok_or_else(invalid)?;
                if !first.is_ascii_alphabetic() {
                    return Err(invalid());
                }
                format!("get{}{}", first.to_ascii_uppercase(), chars.as_str())
            };
        let descriptor = format!("(){ty}");
        let signature = Proto::parse(property.bytes(100)?.ok_or_else(invalid)?)?;
        signature.allowed(if mutable { &[3, 4] } else { &[3] })?;
        metadata_signature(signature.bytes(3)?, &strings, &getter, &descriptor)?;
        let getter_flags = property.integer(7)?.unwrap_or(flags & 0x3f);
        if getter_flags & !0x76 != 0
            || getter_flags & 0x3f != flags & 0x3f
            || mutable && getter_flags != flags & 0x3f
        {
            return Err(invalid());
        }
        validate(&getter, &descriptor, flags & 0x3f)?;
        let index = class
            .methods
            .iter()
            .position(|m| m.name == getter && m.descriptor == descriptor)
            .ok_or_else(invalid)?;
        properties[index] = Some(name.to_owned());
        if mutable {
            let setter = crate::check::setter_name(name);
            let descriptor = format!("({ty})V");
            metadata_signature(signature.bytes(4)?, &strings, &setter, &descriptor)?;
            let setter_flags = property.integer(8)?.unwrap_or(flags & 0x3f);
            if setter_flags != flags & 0x3f {
                return Err(invalid());
            }
            validate(&setter, &descriptor, setter_flags)?;
            let index = class
                .methods
                .iter()
                .position(|method| method.name == setter && method.descriptor == descriptor)
                .ok_or_else(invalid)?;
            properties[index] = Some(name.to_owned());
        }
    }
    for (index, method) in class.methods.iter().enumerate() {
        if !seen[index] && method.access & 8 == 0 {
            return Err(invalid());
        }
    }
    Ok(properties)
}

fn typed_metadata_type(bytes: &[u8], strings: &[String]) -> Result<Type> {
    let proto = Proto::parse(bytes)?;
    proto.allowed(&[1, 2, 3, 6])?;
    if proto.integer(1)?.unwrap_or(0) != 0 || proto.integer(3)?.unwrap_or(0) > 1 {
        return Err(invalid());
    }
    let name = metadata_name(strings, proto.integer(6)?)?;
    let args = proto.messages(2)?;
    let mut ty = match name {
        "kotlin/collections/List" | "kotlin/collections/MutableList" | "kotlin/Array" => {
            let [argument] = args.as_slice() else {
                return Err(invalid());
            };
            let argument = Proto::parse(argument)?;
            argument.allowed(&[1, 2])?;
            if argument.integer(1)?.unwrap_or(2) != 2 {
                return Err(invalid());
            }
            let inner = typed_metadata_type(argument.bytes(2)?.ok_or_else(invalid)?, strings)?;
            if name == "kotlin/Array" {
                if inner != Type::String {
                    return Err(invalid());
                }
                Type::StringArray
            } else if name.ends_with("/MutableList") {
                Type::MutableList(Box::new(inner))
            } else {
                Type::List(Box::new(inner))
            }
        }
        _ => {
            if !args.is_empty() {
                return Err(invalid());
            }
            match name {
                "kotlin/Int" => Type::Int,
                "kotlin/Long" => Type::Long,
                "kotlin/Boolean" => Type::Boolean,
                "kotlin/Byte" => Type::Byte,
                "kotlin/Char" => Type::Char,
                "kotlin/Float" => Type::Float,
                "kotlin/Double" => Type::Double,
                "kotlin/String" => Type::String,
                "kotlin/IntArray" => Type::IntArray,
                "kotlin/ByteArray" => Type::ByteArray,
                "kotlin/CharArray" => Type::CharArray,
                "kotlin/LongArray" => Type::LongArray,
                "kotlin/BooleanArray" => Type::BooleanArray,
                "kotlin/FloatArray" => Type::FloatArray,
                "kotlin/DoubleArray" => Type::DoubleArray,

                "kotlin/Unit" => Type::Unit,
                "kotlin/Any" => Type::Java("java/lang/Object".into()),
                _ if !name.starts_with("kotlin/") && valid_name(&name.replace('.', "$")) => {
                    Type::Java(name.replace('.', "$"))
                }
                _ => return Err(invalid()),
            }
        }
    };
    if proto.integer(3)?.unwrap_or(0) != 0 {
        if ty == Type::Unit {
            return Err(invalid());
        }
        ty = Type::Nullable(Box::new(ty));
    }
    Ok(ty)
}

fn typed_metadata_parameter(bytes: &[u8], strings: &[String]) -> Result<(String, Type, bool)> {
    let parameter = Proto::parse(bytes)?;
    parameter.allowed(&[1, 2, 3])?;
    let flags = parameter.integer(1)?.unwrap_or(0);
    if flags & !2 != 0 {
        return Err(invalid());
    }
    let name = metadata_name(strings, parameter.integer(2)?)?.to_owned();
    let ty = typed_metadata_type(parameter.bytes(3)?.ok_or_else(invalid)?, strings)?;
    if ty == Type::Unit {
        return Err(invalid());
    }
    Ok((name, ty, flags & 2 != 0))
}

fn metadata_payload(metadata: &Annotation) -> Result<(Vec<String>, Vec<u8>)> {
    let mut version = None;
    let mut kind = None;
    let mut extra = None;
    let mut data = None;
    let mut strings = None;
    for (key, value) in &metadata.values {
        match (key.as_str(), value) {
            ("mv", AnnotationValue::Array(values)) => {
                version = Some(
                    values
                        .iter()
                        .map(|v| match v {
                            AnnotationValue::Integer(n) => Ok(*n),
                            _ => Err(invalid()),
                        })
                        .collect::<Result<Vec<_>>>()?,
                )
            }
            ("k", AnnotationValue::Integer(value)) => kind = Some(*value),
            ("xi", AnnotationValue::Integer(value)) => extra = Some(*value),
            ("d1", AnnotationValue::Array(values)) | ("d2", AnnotationValue::Array(values)) => {
                let values = values
                    .iter()
                    .map(|v| match v {
                        AnnotationValue::String(s) => Ok(s.clone()),
                        _ => Err(invalid()),
                    })
                    .collect::<Result<Vec<_>>>()?;
                if key == "d1" {
                    data = Some(values);
                } else {
                    strings = Some(values);
                }
            }
            _ => return Err(invalid()),
        }
    }
    if version.as_deref() != Some(&[2, 3, 0]) || kind != Some(1) || extra != Some(48) {
        return Err(invalid());
    }
    let data = data.ok_or_else(invalid)?.concat();
    let mut chars = data.chars();
    if chars.next() != Some('\0') {
        return Err(invalid());
    }
    let bytes = chars
        .map(|c| u8::try_from(u32::from(c)).map_err(|_| invalid()))
        .collect::<Result<Vec<_>>>()?;
    let mut reader = Reader {
        bytes: &bytes,
        offset: 0,
    };
    let length = usize::try_from(varint(&mut reader)?).map_err(|_| invalid())?;
    let strings = metadata_strings(reader.take(length)?, &strings.ok_or_else(invalid)?)?;
    Ok((strings, reader.take(bytes.len() - reader.offset)?.to_vec()))
}

fn kotlin_typed_class(
    metadata: &Annotation,
    class: &JavaClass,
    attributes: &[Attributes],
) -> Result<(Vec<JavaMethod>, bool)> {
    let (strings, payload) = metadata_payload(metadata)?;
    let proto = Proto::parse(&payload)?;
    proto.allowed(&[1, 3, 4, 6, 7, 8, 9, 10, 101, 104])?;
    let flags = proto.integer(1)?.unwrap_or(6);
    let is_interface = class.access & 0x200 != 0;
    let kind = if is_interface { 96 } else { 0 };
    if flags & !(1024 | 6 | 96) != 0
        || flags & 0x70 != kind
        || !matches!(flags & 14, 0 | 6)
        || metadata_name(&strings, proto.integer(3)?)?.replace('.', "$") != class.name
        || class.super_name.as_deref() != Some("java/lang/Object")
        || class.access != if is_interface { 0x601 } else { 0x31 }
    {
        return Err(invalid());
    }
    if let Some(module) = proto.integer(101)? {
        if metadata_name(&strings, Some(module))?.is_empty() {
            return Err(invalid());
        }
    }
    if proto.integer(104)?.unwrap_or(0) > 3 {
        return Err(invalid());
    }
    let mut supertypes = Vec::new();
    for bytes in proto.messages(6)? {
        let ty = Proto::parse(bytes)?;
        ty.allowed(&[6])?;
        supertypes.push(metadata_name(&strings, ty.integer(6)?)?.replace('.', "$"));
    }
    if (class.interfaces.is_empty() && supertypes != ["kotlin/Any"])
        || (!class.interfaces.is_empty() && supertypes != class.interfaces)
    {
        return Err(invalid());
    }
    fn generic_type(ty: &Type, variance: bool, argument: bool) -> String {
        match ty {
            Type::Nullable(inner) if !inner.is_reference() => ty.descriptor(),
            Type::Nullable(inner) => generic_type(inner, variance, argument),
            Type::List(inner) | Type::MutableList(inner) => {
                let prefix = if variance
                    && matches!(ty, Type::List(_))
                    && matches!(
                        inner.as_ref(),
                        Type::Java(_) | Type::List(_) | Type::MutableList(_)
                    ) {
                    "+"
                } else {
                    ""
                };
                format!(
                    "Ljava/util/List<{}{}>;",
                    prefix,
                    generic_type(inner, variance, true)
                )
            }
            _ if argument && !ty.is_reference() => {
                Type::Nullable(Box::new(ty.clone())).descriptor()
            }
            _ => ty.descriptor(),
        }
    }
    let mut methods = class.methods.clone();
    let mut seen = vec![false; methods.len()];
    let mut attach = |name: &str,
                      params: Vec<(String, Type, bool)>,
                      result: Type,
                      member_flags: u64,
                      property: Option<String>,
                      signature: Option<&[u8]>|
     -> Result<()> {
        let descriptor = format!(
            "({}){}",
            params
                .iter()
                .map(|(_, ty, _)| ty.descriptor())
                .collect::<String>(),
            result.descriptor()
        );
        metadata_signature(signature, &strings, name, &descriptor)?;
        let indices = methods
            .iter()
            .enumerate()
            .filter(|(_, m)| m.name == name && m.descriptor == descriptor)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let [index] = indices.as_slice() else {
            return Err(invalid());
        };
        let index = *index;
        let method = &mut methods[index];
        if let Some(signature) = &attributes[index].signature {
            let expected = |variance| {
                format!(
                    "({}){}",
                    params
                        .iter()
                        .map(|(_, ty, _)| generic_type(ty, variance, false))
                        .collect::<String>(),
                    generic_type(&result, false, false)
                )
            };
            if signature != &expected(false) && signature != &expected(true) {
                return Err(invalid());
            }
        }
        if params.iter().any(|(_, _, default)| *default) {
            if params.len() > 32 {
                return Err(invalid());
            }
            let parameter_descriptor = params
                .iter()
                .map(|(_, ty, _)| ty.descriptor())
                .collect::<String>();
            let (bridge_name, bridge_descriptor, bridge_access) = if name == "<init>" {
                (
                    "<init>".to_owned(),
                    format!(
                        "({parameter_descriptor}ILkotlin/jvm/internal/DefaultConstructorMarker;)V"
                    ),
                    0x1001,
                )
            } else {
                (
                    format!("{name}$default"),
                    format!(
                        "(L{};{parameter_descriptor}ILjava/lang/Object;){}",
                        class.name,
                        result.descriptor()
                    ),
                    0x1009,
                )
            };
            if !class.methods.iter().any(|method| {
                method.name == bridge_name
                    && method.descriptor == bridge_descriptor
                    && method.access == bridge_access
            }) {
                return Err(invalid());
            }
        }
        let expected_visibility = if member_flags & 14 == 2 { 2 } else { 1 };
        if seen[index]
            || method.access & 7 != expected_visibility
            || method.access & (8 | 0x40 | 0x80 | 0x1000) != 0
            || (method.access & 0x400 != 0) != (member_flags & 0x30 == 0x20)
            || attributes[index].unsupported
            || !attributes[index]
                .annotations
                .iter()
                .chain(attributes[index].parameter_annotations.iter().flatten())
                .all(|a| {
                    matches!(
                        a.name.as_str(),
                        "Lorg/jetbrains/annotations/NotNull;"
                            | "Lorg/jetbrains/annotations/Nullable;"
                    ) && a.values.is_empty()
                })
        {
            return Err(invalid());
        }
        seen[index] = true;
        method.unsupported = false;
        method.source_property = property;
        method.kotlin_params = Some(params);
        method.kotlin_result = Some(result);
        method.kotlin_internal = member_flags & 14 == 0;
        Ok(())
    };
    for bytes in proto.messages(8)? {
        if is_interface {
            return Err(invalid());
        }
        let constructor = Proto::parse(bytes)?;
        constructor.allowed(&[1, 2, 100])?;
        let flags = constructor.integer(1)?.unwrap_or(6);
        if !matches!(flags, 0 | 2 | 6) {
            return Err(invalid());
        }
        let params = constructor
            .messages(2)?
            .into_iter()
            .map(|p| typed_metadata_parameter(p, &strings))
            .collect::<Result<Vec<_>>>()?;
        attach(
            "<init>",
            params,
            Type::Unit,
            flags,
            None,
            constructor.bytes(100)?,
        )?;
    }
    for bytes in proto.messages(9)? {
        let function = Proto::parse(bytes)?;
        function.allowed(&[2, 3, 6, 9, 100])?;
        let flags = function.integer(9)?.unwrap_or(6);
        if flags & !(0x10000 | 0x100 | 0xc0 | 0x30 | 6) != 0 || !matches!(flags & 14, 0 | 2 | 6) {
            return Err(invalid());
        }
        let name = metadata_name(&strings, function.integer(2)?)?;
        let params = function
            .messages(6)?
            .into_iter()
            .map(|p| typed_metadata_parameter(p, &strings))
            .collect::<Result<Vec<_>>>()?;
        let result = typed_metadata_type(function.bytes(3)?.ok_or_else(invalid)?, &strings)?;
        attach(name, params, result, flags, None, function.bytes(100)?)?;
    }
    for bytes in proto.messages(10)? {
        let property = Proto::parse(bytes)?;
        property.allowed(&[2, 3, 7, 8, 11, 100])?;
        let flags = property.integer(11)?.unwrap_or(518);
        if flags & !0x736 != 0
            || !matches!(flags & 14, 0 | 2 | 6)
            || flags & 0x200 == 0
            || (flags & 0x100 != 0) != (flags & 0x400 != 0)
        {
            return Err(invalid());
        }
        let name = metadata_name(&strings, property.integer(2)?)?;
        if name.is_empty() || !name.is_ascii() {
            return Err(invalid());
        }
        let ty = typed_metadata_type(property.bytes(3)?.ok_or_else(invalid)?, &strings)?;
        if ty == Type::Unit {
            return Err(invalid());
        }
        let signature = Proto::parse(property.bytes(100)?.ok_or_else(invalid)?)?;
        signature.allowed(&[1, 3, 4])?;
        if let Some(field) = signature.bytes(1)? {
            metadata_signature(Some(field), &strings, name, &ty.descriptor())?;
        }
        if flags & 14 != 2 {
            let getter_flags = property.integer(7)?.unwrap_or(flags & 0x3f);
            if getter_flags & !0x76 != 0 || getter_flags & 0x3f != flags & 0x3f {
                return Err(invalid());
            }
            attach(
                &crate::check::getter_name(name),
                vec![],
                ty.clone(),
                flags & 0x3f,
                Some(name.into()),
                signature.bytes(3)?,
            )?;
            if flags & 0x100 != 0 {
                let setter_flags = property.integer(8)?.unwrap_or(flags & 0x3f);
                if setter_flags != flags & 0x3f {
                    return Err(invalid());
                }
                attach(
                    &crate::check::setter_name(name),
                    vec![("value".into(), ty, false)],
                    Type::Unit,
                    setter_flags,
                    Some(name.into()),
                    signature.bytes(4)?,
                )?;
            }
        }
    }
    for (index, method) in methods.iter().enumerate() {
        let generated_noarg = method.name == "<init>"
            && method.descriptor == "()V"
            && method.access == 1
            && methods.iter().any(|constructor| {
                constructor.name == "<init>"
                    && constructor.kotlin_params.as_ref().is_some_and(|params| {
                        !params.is_empty() && params.iter().all(|(_, _, default)| *default)
                    })
            });
        if !seen[index]
            && !generated_noarg
            && method.access & 8 == 0
            && method.access & 0x1000 == 0
            && method.name != "<clinit>"
        {
            return Err(invalid());
        }
    }
    Ok((methods, flags & 14 == 0))
}

fn validate_annotation_default(descriptor: &str, payload: &[u8], pool: &[Constant]) -> Result<()> {
    fn value(
        descriptor: &str,
        reader: &mut Reader<'_>,
        pool: &[Constant],
        depth: usize,
    ) -> Result<()> {
        if depth > 32 {
            return Err(invalid());
        }
        let start = reader.offset;
        let tag = reader.u1()?;
        if let Some(component) = descriptor.strip_prefix('[') {
            if tag != b'[' {
                return Err(invalid());
            }
            for _ in 0..reader.u2()? {
                value(component, reader, pool, depth + 1)?;
            }
            return Ok(());
        }
        let valid = match descriptor {
            "Ljava/lang/String;" => tag == b's',
            "Ljava/lang/Class;" => tag == b'c',
            "B" | "C" | "D" | "F" | "I" | "J" | "S" | "Z" => descriptor.as_bytes()[0] == tag,
            _ => matches!(tag, b'e' | b'@') && utf8(pool, reader.u2()?)? == descriptor,
        };
        if !valid {
            return Err(invalid());
        }
        reader.offset = start;
        annotation_value(reader, pool, depth)?;
        Ok(())
    }
    let mut reader = Reader {
        bytes: payload,
        offset: 0,
    };
    value(descriptor, &mut reader, pool, 0)?;
    if reader.offset != payload.len() {
        return Err(invalid());
    }
    Ok(())
}

fn annotation_definition(
    class: &JavaClass,
    attrs: &Attributes,
    methods: &[Attributes],
    pool: &[Constant],
) -> Result<Option<AnnotationDefinition>> {
    if class.access & 0x2000 == 0 {
        return Ok(None);
    }
    if class.access & 0x200 == 0
        || class.super_name.as_deref() != Some("java/lang/Object")
        || class.interfaces != ["java/lang/annotation/Annotation"]
    {
        return Err(invalid());
    }
    if attrs.unsupported || attrs.signature.is_some() || !attrs.parameter_annotations.is_empty() {
        return Ok(None);
    }
    let mut retention = AnnotationRetention::Class;
    let mut targets = [
        "TYPE",
        "FIELD",
        "METHOD",
        "PARAMETER",
        "CONSTRUCTOR",
        "LOCAL_VARIABLE",
        "ANNOTATION_TYPE",
        "PACKAGE",
        "MODULE",
        "RECORD_COMPONENT",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let mut seen = std::collections::BTreeSet::new();
    let mut supported = true;
    let mut kotlin_parameters = None;
    let mut kotlin_types = Vec::new();
    let mut kotlin_targets = None;
    let mut vararg = None;
    for annotation in &attrs.annotations {
        if !seen.insert(&annotation.name) {
            return Err(invalid());
        }
        match annotation.name.as_str() {
            "Lkotlin/Metadata;" => {
                let (parameters, types, variadic) =
                    metadata::annotation_parameters(annotation, &class.name)?;
                kotlin_parameters = Some(parameters);
                kotlin_types = types;
                vararg = variadic;
            }
            "Lkotlin/annotation/Retention;" => {}
            "Lkotlin/annotation/Target;" => {
                let [(name, AnnotationValue::Array(values))] = annotation.values.as_slice() else {
                    return Err(invalid());
                };
                if name != "allowedTargets" {
                    return Err(invalid());
                }
                let mut targets = Vec::new();
                for value in values {
                    let AnnotationValue::Enum { descriptor, name } = value else {
                        return Err(invalid());
                    };
                    if descriptor != "Lkotlin/annotation/AnnotationTarget;" {
                        return Err(invalid());
                    }
                    targets.push(name.clone());
                }
                kotlin_targets = Some(targets);
            }
            "Lkotlin/annotation/MustBeDocumented;" => {}
            "Ljava/lang/annotation/Retention;" => {
                let [(key, AnnotationValue::Enum { descriptor, name })] =
                    annotation.values.as_slice()
                else {
                    return Err(invalid());
                };
                if key != "value" || descriptor != "Ljava/lang/annotation/RetentionPolicy;" {
                    return Err(invalid());
                }
                retention = match name.as_str() {
                    "SOURCE" => AnnotationRetention::Source,
                    "CLASS" => AnnotationRetention::Class,
                    "RUNTIME" => AnnotationRetention::Runtime,
                    _ => return Err(invalid()),
                };
            }
            "Ljava/lang/annotation/Target;" => {
                let [(key, AnnotationValue::Array(values))] = annotation.values.as_slice() else {
                    return Err(invalid());
                };
                if key != "value" {
                    return Err(invalid());
                }
                targets.clear();
                for value in values {
                    let AnnotationValue::Enum { descriptor, name } = value else {
                        return Err(invalid());
                    };
                    if descriptor != "Ljava/lang/annotation/ElementType;"
                        || !matches!(
                            name.as_str(),
                            "TYPE"
                                | "FIELD"
                                | "METHOD"
                                | "PARAMETER"
                                | "CONSTRUCTOR"
                                | "LOCAL_VARIABLE"
                                | "ANNOTATION_TYPE"
                                | "PACKAGE"
                                | "TYPE_PARAMETER"
                                | "TYPE_USE"
                                | "MODULE"
                                | "RECORD_COMPONENT"
                        )
                        || targets.contains(name)
                    {
                        return Err(invalid());
                    }
                    targets.push(name.clone());
                }
            }
            "Ljava/lang/annotation/Documented;" | "Ljava/lang/annotation/Inherited;" => {
                if !annotation.values.is_empty() {
                    return Err(invalid());
                }
            }
            "Lkotlin/RequiresOptIn;"
            | "Lkotlin/DslMarker;"
            | "Lkotlin/annotation/Repeatable;"
            | "Lkotlin/jvm/JvmRepeatable;" => supported = false,
            _ => {}
        }
    }
    if !supported {
        return Ok(None);
    }
    let mut parameters = Vec::new();
    for (method, attrs) in class.methods.iter().zip(methods) {
        if method.name.starts_with('<') {
            continue;
        }
        if matches!(
            method.name.as_str(),
            "hashCode" | "toString" | "annotationType"
        ) {
            return Ok(None);
        }
        let Some(result) = method.descriptor.strip_prefix("()") else {
            return Err(invalid());
        };
        if result.is_empty()
            || result == "V"
            || method.access & 9 != 1
            || method.access & 0x400 == 0
        {
            return Err(invalid());
        }
        if attrs.code.is_some() || parameters.iter().any(|(name, _, _)| name == &method.name) {
            return Err(invalid());
        }
        if attrs.unsupported || attrs.annotated {
            return Ok(None);
        }
        if let Some(signature) = &attrs.signature {
            fn supported_annotation_signature(ty: &Type) -> bool {
                match ty {
                    Type::Parameterized { name, arguments } => {
                        name == "java/lang/Class"
                            && arguments.len() == 1
                            && arguments[0].ty.is_none()
                    }
                    Type::Array(inner) => supported_annotation_signature(inner),
                    _ => true,
                }
            }
            let (parameters, args, result) = signature::method(signature, &[])?;
            if !parameters.is_empty()
                || !args.is_empty()
                || !supported_annotation_signature(&result)
            {
                return Ok(None);
            }
        }
        if let Some(payload) = &attrs.annotation_default_payload {
            validate_annotation_default(result, payload, pool)?;
        }
        parameters.push((method.name.clone(), result.into(), attrs.annotation_default));
    }
    if let Some(order) = &kotlin_parameters {
        if order.len() != parameters.len()
            || order
                .iter()
                .any(|name| !parameters.iter().any(|(p, _, _)| p == name))
        {
            return Err(invalid());
        }
        parameters.sort_by_key(|(name, _, _)| order.iter().position(|p| p == name).unwrap());
        if kotlin_targets.is_none() {
            kotlin_targets = Some(
                [
                    "CLASS",
                    "ANNOTATION_CLASS",
                    "PROPERTY",
                    "FIELD",
                    "VALUE_PARAMETER",
                    "CONSTRUCTOR",
                    "FUNCTION",
                    "PROPERTY_GETTER",
                    "PROPERTY_SETTER",
                    "LOCAL_VARIABLE",
                    "TYPEALIAS",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            );
        }
    }
    Ok(Some(AnnotationDefinition {
        kotlin_parameters,
        kotlin_types,
        kotlin_targets,
        vararg,
        retention,
        targets,
        parameters,
    }))
}

fn parse(bytes: &[u8], expected: &str) -> Result<JavaClass> {
    parse_details(bytes, expected).map(|(class, _, _)| class)
}

fn parse_details(bytes: &[u8], expected: &str) -> Result<(JavaClass, Attributes, Vec<Attributes>)> {
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
                let units = modified_utf8_units(reader.take(length)?)?;
                Constant::Utf8(String::from_utf16_lossy(&units), units)
            }
            7 => Constant::Class(reader.u2()?),
            3 => Constant::Integer(reader.u4()?),
            4 => Constant::Float(reader.u4()?),
            8 => Constant::String(reader.u2()?),
            9..=11 => Constant::Reference(tag, reader.u2()?, reader.u2()?),
            12 => Constant::NameAndType(reader.u2()?, reader.u2()?),
            17 | 18 => {
                reader.take(4)?;
                Constant::Empty
            }
            5 | 6 => {
                let value = reader.take(8)?;
                if pool.len() + 1 >= count {
                    return Err(invalid());
                }
                if tag == 5 {
                    Constant::Long(i64::from_be_bytes(value.try_into().map_err(|_| invalid())?))
                } else {
                    Constant::Double(u64::from_be_bytes(value.try_into().map_err(|_| invalid())?))
                }
            }
            16 | 19 | 20 => {
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
        if matches!(tag, 5 | 6) {
            pool.push(Constant::Empty);
        }
    }
    let constants = std::sync::Arc::new(inline_constants(&pool)?);
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
    let mut fields = Vec::new();
    let mut field_attributes = Vec::new();
    for _ in 0..reader.u2()? {
        let access = reader.u2()?;
        let field_name = utf8(&pool, reader.u2()?)?.to_owned();
        let descriptor = utf8(&pool, reader.u2()?)?.to_owned();
        let attrs = attributes(&mut reader, &pool, &constants)?;
        if attrs
            .type_annotation_targets
            .iter()
            .any(|target| *target != 0x13)
        {
            return Err(invalid());
        }
        let constant = attrs
            .constant
            .map(
                |index| match (descriptor.as_str(), pool.get(usize::from(index))) {
                    ("I" | "Z" | "B" | "C" | "S", Some(Constant::Integer(value))) => {
                        Ok(Some(JavaConstant::Int(*value as i32)))
                    }
                    ("J", Some(Constant::Long(value))) => Ok(Some(JavaConstant::Long(*value))),
                    ("F", Some(Constant::Float(value))) => Ok(Some(JavaConstant::Float(*value))),
                    ("D", Some(Constant::Double(value))) => Ok(Some(JavaConstant::Double(*value))),
                    ("Ljava/lang/String;", Some(Constant::String(value))) => {
                        match pool.get(usize::from(*value)) {
                            Some(Constant::Utf8(_, units)) => {
                                Ok(Some(JavaConstant::String(units.clone())))
                            }
                            _ => Err(invalid()),
                        }
                    }
                    _ => Err(invalid()),
                },
            )
            .transpose()?
            .flatten();
        let unsupported_constant = attrs.constant.is_some() && constant.is_none();
        field_attributes.push(attrs);
        let attrs = field_attributes.last().unwrap();
        fields.push(JavaField {
            kotlin: None,
            owner: name.clone(),
            name: field_name,
            descriptor,
            access,
            constant,
            unsupported: attrs.unsupported
                || attrs.annotated
                || attrs.signature.is_some()
                || unsupported_constant
                || access & 0x1000 != 0,
        });
    }
    let mut methods = Vec::new();
    let mut method_attributes = Vec::new();
    for _ in 0..reader.u2()? {
        let access = reader.u2()?;
        let method_name = utf8(&pool, reader.u2()?)?.to_owned();
        let descriptor = utf8(&pool, reader.u2()?)?.to_owned();
        let attrs = attributes(&mut reader, &pool, &constants)?;
        if attrs
            .type_annotation_targets
            .iter()
            .any(|target| !matches!(target, 0x01 | 0x12 | 0x14..=0x17))
        {
            return Err(invalid());
        }
        validate_type_annotation_parameters(&descriptor, &attrs)?;
        let unsupported = attrs.unsupported
            || !attrs.only_input_type_parameters.is_empty()
            || attrs.annotated
            || attrs.signature.is_some()
            || access & (0x0040 | 0x1000) != 0;
        method_attributes.push(attrs);
        methods.push(JavaMethod {
            library: None,
            owner: name.clone(),
            name: method_name,
            descriptor,
            access,
            unsupported,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        });
    }
    let attrs = attributes(&mut reader, &pool, &constants)?;
    if attrs
        .type_annotation_targets
        .iter()
        .any(|target| !matches!(target, 0x00 | 0x10 | 0x11))
    {
        return Err(invalid());
    }
    if reader.offset != bytes.len() {
        return Err(invalid());
    }
    let mut class = JavaClass {
        sealed_: false,
        sealed_subclasses: Vec::new(),
        annotation_definition: None,
        kotlin_fun_interface: false,
        library: None,
        name,
        access,
        super_name,
        interfaces,
        methods,
        unsupported: attrs.unsupported || attrs.annotated || attrs.signature.is_some(),
        kotlin_interface: false,
        fields,
        kotlin_class: false,
    };
    if !attrs.unsupported
        && attrs.signature.is_none()
        && attrs.annotations.len() == 1
        && attrs.annotations[0].name == "Lkotlin/Metadata;"
        && attrs.parameter_annotations.is_empty()
    {
        if let Ok((methods, internal)) =
            kotlin_typed_class(&attrs.annotations[0], &class, &method_attributes)
        {
            class.unsupported = internal;
            class.kotlin_class = true;
            class.kotlin_interface = class.access & 0x200 != 0;
            class.methods = methods;
        } else if let Ok(properties) =
            kotlin_interface(&attrs.annotations[0], &class, &method_attributes)
        {
            class.unsupported = false;
            class.kotlin_interface = true;
            for (method, property) in class.methods.iter_mut().zip(properties) {
                method.source_property = property;
                if method.access & 8 == 0 {
                    method.unsupported = false;
                }
            }
        }
    }
    if let Some(metadata) = attrs
        .annotations
        .iter()
        .find(|a| a.name == "Lkotlin/Metadata;")
    {
        if class.unsupported || class.library.is_none() {
            let mut candidate = class.clone();
            if metadata::attach(
                metadata,
                &mut candidate,
                &method_attributes,
                &field_attributes,
            )
            .is_ok()
            {
                class = candidate;
            }
        }
    } else {
        let _ = signature::attach(&mut class, &attrs, &method_attributes);
    }
    if class.access & 0x2000 != 0 {
        class.annotation_definition =
            annotation_definition(&class, &attrs, &method_attributes, &pool)?;
        if let Some(definition) = &class.annotation_definition {
            if definition.kotlin_parameters.is_some() {
                class.kotlin_class = true;
            }
            for (name, _, _) in &definition.parameters {
                if let Some(method) = class.methods.iter_mut().find(|method| &method.name == name) {
                    let logical = if let Some(names) = &definition.kotlin_parameters {
                        names
                            .iter()
                            .position(|parameter| parameter == name)
                            .and_then(|index| definition.kotlin_types.get(index))
                            .cloned()
                    } else {
                        fn annotation_type(ty: Type) -> Type {
                            match ty {
                                Type::Java(name) | Type::Parameterized { name, .. }
                                    if name == "java/lang/Class" =>
                                {
                                    Type::Parameterized {
                                        name: "kotlin/reflect/KClass".into(),
                                        arguments: vec![crate::syntax::TypeArgument {
                                            variance: crate::syntax::Variance::Invariant,
                                            ty: None,
                                        }],
                                    }
                                }
                                Type::Array(inner) => {
                                    Type::Array(Box::new(annotation_type(*inner)))
                                }
                                other => other,
                            }
                        }
                        Some(annotation_type(
                            signature::method(&method.descriptor, &[])?.2,
                        ))
                    };
                    method.source_property = Some(name.clone());
                    method.kotlin_params = Some(Vec::new());
                    method.kotlin_result = logical;
                    method.unsupported = false;
                }
            }
        }
        class.unsupported = class.annotation_definition.is_none();
    }
    Ok((class, attrs, method_attributes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn type_annotation_targets_paths_and_parameter_indices_are_checked() {
        fn decode(payload: &[u8]) -> Result<Attributes> {
            decode_named(payload, "RuntimeVisibleTypeAnnotations")
        }
        fn decode_named(payload: &[u8], attribute_name: &str) -> Result<Attributes> {
            let pool = vec![
                Constant::Empty,
                Constant::Utf8(
                    attribute_name.into(),
                    attribute_name.encode_utf16().collect(),
                ),
                Constant::Utf8(
                    "Lorg/jspecify/annotations/NonNull;".into(),
                    "Lorg/jspecify/annotations/NonNull;"
                        .encode_utf16()
                        .collect(),
                ),
            ];
            let mut bytes = vec![0, 1, 0, 1];
            bytes.extend((payload.len() as u32).to_be_bytes());
            bytes.extend(payload);
            attributes(
                &mut Reader {
                    bytes: &bytes,
                    offset: 0,
                },
                &pool,
                &std::sync::Arc::new(std::collections::BTreeMap::new()),
            )
        }
        let valid = decode(&[0, 1, 0x16, 1, 0, 0, 2, 0, 0]).unwrap();
        assert!(!valid.unsupported);
        assert_eq!(valid.parameter_type_annotations[0].0, 1);
        assert!(validate_type_annotation_parameters("(J[D)V", &valid).is_ok());
        assert!(validate_type_annotation_parameters("([D)V", &valid).is_err());
        let wrong_index = decode(&[0, 1, 0x16, 255, 0, 0, 2, 0, 0]).unwrap();
        assert!(validate_type_annotation_parameters("(J[D)V", &wrong_index).is_err());
        let nested_wrong_index = decode(&[0, 1, 0x16, 255, 1, 0, 0, 0, 2, 0, 0]).unwrap();
        assert!(nested_wrong_index.unsupported);
        assert!(validate_type_annotation_parameters("(J[D)V", &nested_wrong_index).is_err());
        assert_eq!(type_annotation_parameter_count("(J[D)V").unwrap(), 2);
        assert_eq!(type_annotation_parameter_count("([D)V").unwrap(), 1);
        let returned = decode(&[0, 1, 0x14, 0, 0, 2, 0, 0]).unwrap();
        assert_eq!(returned.return_type_annotations.len(), 1);
        let invisible = decode_named(
            &[0, 1, 0x14, 0, 0, 2, 0, 0],
            "RuntimeInvisibleTypeAnnotations",
        )
        .unwrap();
        assert_eq!(invisible.return_type_annotations.len(), 1);
        assert!(decode(&[0, 1, 0x13, 0, 0, 2, 0, 0]).unwrap().unsupported);
        assert!(
            decode(&[0, 1, 0x16, 0, 1, 0, 0, 0, 2, 0, 0])
                .unwrap()
                .unsupported
        );
        assert!(decode(&[0, 1, 0x7f, 0, 0, 2, 0, 0]).is_err());
        assert!(decode(&[0, 1, 0x16, 0, 1, 4, 0, 0, 2, 0, 0]).is_err());
        assert!(decode(&[0, 1, 0x16, 0, 1, 0, 1, 0, 2, 0, 0]).is_err());
        assert!(decode(&[0, 1, 0x16, 0, 0, 0, 2, 0]).is_err());
    }

    #[test]
    fn actual_junit_test_annotation_retains_runtime_target_and_unsupported_defaulted_parameters() {
        let jar = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/compatibility-dependencies/junit-4.13.2.jar");
        if !jar.is_file() {
            return;
        }
        let bytes = archive::read(&jar, "org/junit/Test.class")
            .unwrap()
            .unwrap();
        let class = parse(&bytes, "org/junit/Test").unwrap();
        assert!(!class.unsupported);
        let annotation = class.annotation_definition.unwrap();
        assert_eq!(annotation.retention, AnnotationRetention::Runtime);
        assert_eq!(annotation.targets, ["METHOD"]);
        assert!(annotation.parameters.contains(&(
            "expected".into(),
            "Ljava/lang/Class;".into(),
            true
        )));
        assert!(
            annotation
                .parameters
                .contains(&("timeout".into(), "J".into(), true))
        );
    }

    #[test]
    fn javac_annotation_definitions_preserve_retention_targets_and_member_defaults() {
        let javac = std::env::var_os("JAVA_HOME")
            .map(PathBuf::from)
            .map(|path| path.join("bin/javac"))
            .unwrap_or_else(|| "javac".into());
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "korrust-annotation-reader-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sources = [
            (
                "Marker",
                "package sample; import java.lang.annotation.*; @Documented @Inherited @Retention(RetentionPolicy.RUNTIME) @Target({ElementType.METHOD,ElementType.FIELD}) public @interface Marker { int count(); boolean enabled() default true; String label() default \"ok\"; Class<?> expected() default Object.class; long timeout() default 0L; }",
            ),
            (
                "SourceMarker",
                "package sample; import java.lang.annotation.*; @Retention(RetentionPolicy.SOURCE) @Target(ElementType.METHOD) public @interface SourceMarker {}",
            ),
            (
                "ClassMarker",
                "package sample; public @interface ClassMarker {}",
            ),
            (
                "Nowhere",
                "package sample; import java.lang.annotation.*; @Target({}) public @interface Nowhere {}",
            ),
            (
                "PluginMarker",
                "package sample; import java.lang.annotation.*; @Target({ElementType.TYPE,ElementType.ANNOTATION_TYPE}) public @interface PluginMarker {}",
            ),
            (
                "Unsupported",
                "package sample; @PluginMarker public @interface Unsupported {}",
            ),
            (
                "Ordinary",
                "package sample; @PluginMarker public class Ordinary {}",
            ),
        ];
        let mut command = std::process::Command::new(javac);
        command.arg("-d").arg(&dir);
        for (name, source) in sources {
            let path = dir.join(format!("{name}.java"));
            std::fs::write(&path, source).unwrap();
            command.arg(path);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let read = |name: &str| {
            parse(
                &std::fs::read(dir.join(format!("sample/{name}.class"))).unwrap(),
                &format!("sample/{name}"),
            )
            .unwrap()
        };
        let marker = read("Marker");
        assert!(!marker.unsupported);
        let definition = marker.annotation_definition.unwrap();
        assert_eq!(definition.retention, AnnotationRetention::Runtime);
        assert_eq!(definition.targets, ["METHOD", "FIELD"]);
        for (name, descriptor, default) in [
            ("count", "I", false),
            ("enabled", "Z", true),
            ("label", "Ljava/lang/String;", true),
            ("expected", "Ljava/lang/Class;", true),
            ("timeout", "J", true),
        ] {
            assert!(
                definition
                    .parameters
                    .contains(&(name.into(), descriptor.into(), default))
            );
        }
        assert_eq!(
            read("SourceMarker")
                .annotation_definition
                .unwrap()
                .retention,
            AnnotationRetention::Source
        );
        let default = read("ClassMarker").annotation_definition.unwrap();
        assert_eq!(default.retention, AnnotationRetention::Class);
        assert!(default.targets.contains(&"METHOD".into()));
        assert!(!default.targets.contains(&"TYPE_USE".into()));
        assert!(
            read("Nowhere")
                .annotation_definition
                .unwrap()
                .targets
                .is_empty()
        );
        let passive = read("Unsupported");
        assert!(!passive.unsupported);
        assert!(passive.annotation_definition.is_some());
        let ordinary = read("Ordinary");
        assert!(ordinary.unsupported);
        assert!(ordinary.annotation_definition.is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn kotlin_enum_imports_retain_class_visibility_and_entry_identity() {
        let standard = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        let classpath = Classpath::new(&[standard]).unwrap();
        let class = classpath
            .load("kotlin/annotation/AnnotationRetention")
            .unwrap();
        assert!(!class.unsupported);
        assert_eq!(class.library.unwrap().kotlin_visibility, Some(3));
        assert!(class.fields.iter().any(|field| field.name == "RUNTIME"
            && field.access & 0x4019 == 0x4019
            && field.descriptor == "Lkotlin/annotation/AnnotationRetention;"
            && !field.unsupported));
    }

    #[test]
    fn annotation_numeric_tags_require_matching_pool_constants() {
        let pool = vec![
            Constant::Empty,
            Constant::Integer(1),
            Constant::Long(2),
            Constant::Float(0),
            Constant::Double(0),
            Constant::Utf8("text".into(), Vec::new()),
        ];
        for (tag, valid_index) in [
            (b'B', 1),
            (b'C', 1),
            (b'S', 1),
            (b'I', 1),
            (b'Z', 1),
            (b'J', 2),
            (b'F', 3),
            (b'D', 4),
        ] {
            for index in 0..pool.len() {
                let bytes = [tag, 0, index as u8];
                let result = annotation_value(
                    &mut Reader {
                        bytes: &bytes,
                        offset: 0,
                    },
                    &pool,
                    0,
                );
                assert_eq!(result.is_ok(), index == valid_index, "{tag} {index}");
            }
        }
    }

    #[test]
    fn annotation_defaults_match_declared_scalar_array_and_classifier_types() {
        let pool = vec![
            Constant::Empty,
            Constant::Integer(1),
            Constant::Long(2),
            Constant::Utf8("Lsample/Mode;".into(), Vec::new()),
            Constant::Utf8("FIRST".into(), Vec::new()),
            Constant::Utf8("Lsample/Nested;".into(), Vec::new()),
        ];
        assert!(validate_annotation_default("I", &[b'I', 0, 1], &pool).is_ok());
        assert!(validate_annotation_default("J", &[b'J', 0, 2], &pool).is_ok());
        assert!(validate_annotation_default("I", &[b'J', 0, 2], &pool).is_err());
        assert!(validate_annotation_default("[I", &[b'[', 0, 1, b'I', 0, 1], &pool).is_ok());
        assert!(validate_annotation_default("[J", &[b'[', 0, 1, b'I', 0, 1], &pool).is_err());
        assert!(validate_annotation_default("Lsample/Mode;", &[b'e', 0, 3, 0, 4], &pool).is_ok());
        assert!(validate_annotation_default("Lsample/Other;", &[b'e', 0, 3, 0, 4], &pool).is_err());
        assert!(validate_annotation_default("Lsample/Nested;", &[b'@', 0, 5, 0, 0], &pool).is_ok());
        assert!(validate_annotation_default("Lsample/Other;", &[b'@', 0, 5, 0, 0], &pool).is_err());
        assert!(validate_annotation_default("I", &[b'I', 0, 1, 0], &pool).is_err());
    }

    #[test]
    fn annotation_defaults_validate_boolean_constants_and_full_attribute_payloads() {
        let pool = vec![
            Constant::Empty,
            Constant::Utf8("AnnotationDefault".into(), Vec::new()),
            Constant::Integer(1),
        ];
        let constants = std::sync::Arc::new(std::collections::BTreeMap::new());
        let attribute = [0, 1, 0, 0, 0, 3, b'Z', 0, 2];
        let bytes = [&[0, 1][..], &attribute].concat();
        let parsed = attributes(
            &mut Reader {
                bytes: &bytes,
                offset: 0,
            },
            &pool,
            &constants,
        )
        .unwrap();
        assert!(parsed.annotation_default);
        let mut trailing = bytes.clone();
        trailing[7] = 4;
        trailing.push(0);
        assert!(
            attributes(
                &mut Reader {
                    bytes: &trailing,
                    offset: 0
                },
                &pool,
                &constants
            )
            .is_err()
        );
        let duplicate = [&[0, 2][..], &attribute, &attribute].concat();
        assert!(
            attributes(
                &mut Reader {
                    bytes: &duplicate,
                    offset: 0
                },
                &pool,
                &constants
            )
            .is_err()
        );
        let mut invalid_constant = bytes;
        *invalid_constant.last_mut().unwrap() = 1;
        assert!(
            attributes(
                &mut Reader {
                    bytes: &invalid_constant,
                    offset: 0
                },
                &pool,
                &constants
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_annotation_retention_is_rejected_without_unhiding_ordinary_classes() {
        let class = JavaClass {
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            kotlin_fun_interface: false,
            library: None,
            name: "sample/Marker".into(),
            access: 0x2601,
            super_name: Some("java/lang/Object".into()),
            interfaces: vec!["java/lang/annotation/Annotation".into()],
            methods: Vec::new(),
            fields: Vec::new(),
            kotlin_class: false,
            unsupported: true,
            kotlin_interface: false,
        };
        for value in [
            AnnotationValue::String("RUNTIME".into()),
            AnnotationValue::Enum {
                descriptor: "Ljava/lang/annotation/RetentionPolicy;".into(),
                name: "INVALID".into(),
            },
            AnnotationValue::Enum {
                descriptor: "Lother/Policy;".into(),
                name: "RUNTIME".into(),
            },
        ] {
            let attrs = Attributes {
                annotations: vec![Annotation {
                    name: "Ljava/lang/annotation/Retention;".into(),
                    values: vec![("value".into(), value)],
                }],
                ..Attributes::default()
            };
            assert!(annotation_definition(&class, &attrs, &[], &[]).is_err());
        }
    }

    #[test]
    fn kotlin_metadata_preserves_generic_type_alias_expansions() {
        let standard = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        if !standard.is_file() {
            return;
        }
        let classpath = Classpath::new(&[standard]).unwrap();
        let alias = classpath
            .type_alias("kotlin/collections", "ArrayList")
            .unwrap()
            .unwrap();
        assert_eq!(alias.type_parameters.len(), 1);
        let Type::Parameterized { name, arguments } = alias.expanded else {
            panic!("generic alias lost its arguments")
        };
        assert_eq!(name, "java/util/ArrayList");
        assert!(
            matches!(&arguments[0].ty,Some(Type::Parameter{name,..}) if name==&alias.type_parameters[0].name)
        );
    }
    #[test]
    fn jdk_implementation_annotations_do_not_hide_public_constructors() {
        if std::env::var_os("JAVA_HOME").is_none() {
            return;
        }
        let class = Classpath::new(&[])
            .unwrap()
            .load("java/lang/StringBuilder")
            .unwrap();
        for descriptor in ["()V", "(I)V", "(Ljava/lang/String;)V"] {
            let constructor = class
                .methods
                .iter()
                .find(|method| method.name == "<init>" && method.descriptor == descriptor)
                .unwrap();
            assert!(!constructor.unsupported, "{descriptor}");
            assert!(constructor.kotlin_params.is_some());
        }
    }
    #[test]
    fn kotlin_collection_markers_preserve_source_supertypes_and_infix_flags() {
        let standard = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        if !standard.is_file() {
            return;
        }
        let classpath = Classpath::new(&[standard]).unwrap();
        let progression = classpath.load("kotlin/ranges/IntProgression").unwrap();
        assert!(!progression.unsupported);
        assert!(progression.library.unwrap().supertypes.iter().any(|ty| {
            matches!(ty, Type::Parameterized { name, arguments } if name == "java/lang/Iterable" && arguments.len() == 1 && arguments[0].ty == Some(Type::Int))
        }));
        let extensions = classpath.load("kotlin/ranges/RangesKt___RangesKt").unwrap();
        assert!(extensions.methods.iter().any(|method| {
            method
                .library
                .as_ref()
                .is_some_and(|library| library.source_name == "downTo" && library.infix)
                && method.descriptor == "(II)Lkotlin/ranges/IntProgression;"
        }));
    }

    fn minimal() -> Vec<u8> {
        let mut bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 52, 0, 3, 1, 0, 1, b'A', 7, 0, 1,
        ];
        bytes.extend_from_slice(&[0, 1, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        bytes
    }
    #[test]
    fn package_metadata_exposes_top_level_names_and_defaults() {
        let files =
            crate::compile_all("fun answer(value: Int = 7): Int = value", "Answer.kt").unwrap();
        let file = files
            .iter()
            .find(|f| f.internal_name == "AnswerKt")
            .unwrap();
        let class = parse(&file.bytes, "AnswerKt").unwrap();
        assert!(!class.unsupported);
        assert_eq!(class.library.as_ref().unwrap().package.as_deref(), Some(""));
        let method = class.methods.iter().find(|m| m.name == "answer").unwrap();
        assert!(!method.unsupported);
        assert_eq!(
            method.kotlin_params.as_ref().unwrap(),
            &[("value".into(), Type::Int, true)]
        );
    }

    #[test]
    fn metadata_visibility_distinguishes_private_and_public_callables() {
        let classes = crate::compile_all(
            "private fun hidden(): Int=1\nfun visible(): Int=hidden()",
            "Visibility.kt",
        )
        .unwrap();
        let class = parse(&classes[0].bytes, "VisibilityKt").unwrap();
        assert!(
            !class
                .methods
                .iter()
                .find(|method| method.name == "hidden")
                .unwrap()
                .library
                .as_ref()
                .unwrap()
                .kotlin_public
        );
        assert!(
            class
                .methods
                .iter()
                .find(|method| method.name == "visible")
                .unwrap()
                .library
                .as_ref()
                .unwrap()
                .kotlin_public
        );
    }

    #[test]
    fn package_lookup_indexes_follow_replaced_source_headers() {
        let classpath = Classpath::new(&[]).unwrap();
        let before = crate::syntax::parse("fun first(): Int=1").unwrap();
        classpath.source_header(&before, "SourceKt").unwrap();
        assert_eq!(classpath.package_methods("", "first").unwrap().len(), 1);
        let after = crate::syntax::parse("fun second(): Int=2").unwrap();
        classpath.source_header(&after, "SourceKt").unwrap();
        assert!(classpath.package_methods("", "first").unwrap().is_empty());
        assert_eq!(classpath.package_methods("", "second").unwrap().len(), 1);
    }

    #[test]
    fn classpath_order_selects_the_first_class() {
        use std::io::Write;
        let directory =
            std::env::temp_dir().join(format!("korrust-classpath-order-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let first = directory.join("first.jar");
        let second = directory.join("second.jar");
        for (path, access) in [(&first, 1u8), (&second, 17u8)] {
            let mut bytes = minimal();
            bytes[18] = access;
            let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
            zip.start_file("A.class", zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(&bytes).unwrap();
            zip.finish().unwrap();
        }
        let cp = Classpath::new(&[first, second]).unwrap();
        assert_eq!(cp.load("A").unwrap().access, 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn emitted_kotlin_metadata_retains_typed_constructor_names_defaults_and_properties() {
        let files = crate::compile_all(
            "class Packet(val amount: Int? = null, val values: MutableList<String?>) { val count: Int get() = values.size }",
            "Packet.kt",
        ).unwrap();
        let file = files
            .iter()
            .find(|file| file.internal_name == "Packet")
            .unwrap();
        let class = parse(&file.bytes, "Packet").unwrap();
        assert!(!class.unsupported);
        assert!(class.kotlin_class);
        let constructor = class
            .methods
            .iter()
            .find(|method| method.name == "<init>" && !method.unsupported)
            .unwrap();
        assert_eq!(
            constructor.kotlin_params.as_ref().unwrap(),
            &[
                ("amount".into(), Type::Nullable(Box::new(Type::Int)), true),
                (
                    "values".into(),
                    Type::MutableList(Box::new(Type::Nullable(Box::new(Type::String)))),
                    false
                ),
            ]
        );
        let amount = class
            .methods
            .iter()
            .find(|method| method.name == "getAmount")
            .unwrap();
        assert_eq!(
            amount.kotlin_result,
            Some(Type::Nullable(Box::new(Type::Int)))
        );
        assert_eq!(amount.source_property.as_deref(), Some("amount"));
        assert_eq!(amount.descriptor, "()Ljava/lang/Integer;");
    }

    #[test]
    fn kotlin_metadata_keeps_internal_class_and_member_visibility_restricted() {
        let files =
            crate::compile_all("internal class Hidden(val value: Int)", "Hidden.kt").unwrap();
        assert!(parse(&files[0].bytes, "Hidden").unwrap().unsupported);
        let (metadata, class, attributes) = fixture_metadata(false, &[], 550, 0, 32);
        let (methods, _) = kotlin_typed_class(&metadata, &class, &attributes).unwrap();
        assert!(
            methods
                .iter()
                .find(|method| method.name == "item")
                .unwrap()
                .kotlin_internal
        );
    }

    #[test]
    fn constant_fields_preserve_wide_values_float_bits_and_utf16_units() {
        let file = crate::compile(
            "const val number: Long = -9223372036854775807L - 1L\nconst val text: String = \"\\uD800\\u0000\\uFFFF\"\nconst val fraction: Float = 1.25f",
            "Constants.kt",
        ).unwrap();
        let class = parse(&file.bytes, "ConstantsKt").unwrap();
        let field = |name: &str| {
            &class
                .fields
                .iter()
                .find(|field| field.name == name)
                .unwrap()
                .constant
        };
        assert!(matches!(
            field("number"),
            Some(JavaConstant::Long(i64::MIN))
        ));
        assert!(
            matches!(field("fraction"), Some(JavaConstant::Float(bits)) if *bits == 1.25_f32.to_bits())
        );
        assert!(
            matches!(field("text"), Some(JavaConstant::String(units)) if units == &[0xd800, 0, 0xffff])
        );
    }

    #[test]
    fn volatile_java_fields_retain_access_flags_and_remain_available() {
        let mut bytes = vec![
            0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 52, 0, 5, 1, 0, 1, b'A', 7, 0, 1, 1, 0, 1, b'v', 1, 0,
            1, b'I',
        ];
        bytes.extend_from_slice(&[
            0, 1, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0x41, 0, 3, 0, 4, 0, 0, 0, 0, 0, 0,
        ]);
        let class = parse(&bytes, "A").unwrap();
        let [field] = class.fields.as_slice() else {
            panic!()
        };
        assert_eq!(field.access, 0x41);
        assert!(!field.unsupported);
        assert_eq!(field.descriptor, "I");
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
    fn empty_class_annotation_tables_are_supported() {
        let name = b"RuntimeInvisibleAnnotations";
        let mut bytes = minimal();
        bytes[9] = 4;
        let mut constant = vec![1, 0, name.len() as u8];
        constant.extend_from_slice(name);
        bytes.splice(17..17, constant);
        let end = bytes.len();
        bytes[end - 2..].copy_from_slice(&[0, 1]);
        bytes.extend_from_slice(&[0, 3, 0, 0, 0, 2, 0, 0]);
        assert!(!parse(&bytes, "A").unwrap().unsupported);
        let end = bytes.len();
        bytes[end - 3] = 6;
        bytes[end - 1] = 1;
        bytes.extend_from_slice(&[0, 1, 0, 0]);
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
    fn proto_integer(number: u32, value: u64) -> Vec<u8> {
        fn write(mut value: u64, bytes: &mut Vec<u8>) {
            while value >= 128 {
                bytes.push((value as u8 & 127) | 128);
                value >>= 7;
            }
            bytes.push(value as u8);
        }
        let mut bytes = Vec::new();
        write(u64::from(number) << 3, &mut bytes);
        write(value, &mut bytes);
        bytes
    }
    fn proto_bytes(number: u32, value: &[u8]) -> Vec<u8> {
        let mut bytes = proto_integer(number, value.len() as u64);
        bytes[0] |= 2;
        bytes.extend_from_slice(value);
        bytes
    }
    fn fixture_metadata(
        nullable: bool,
        function_extra: &[u8],
        property_flags: u64,
        parameter_flags: u64,
        function_flags: u64,
    ) -> (Annotation, JavaClass, Vec<Attributes>) {
        let strings = [
            "sample/Index",
            "kotlin/Any",
            "itemCount",
            "kotlin/Int",
            "getItemCount",
            "()I",
            "item",
            "kotlin/String",
            "index",
            "label",
        ];
        let ty = |index| proto_integer(6, index);
        let mut result = ty(7);
        if nullable {
            result.extend(proto_integer(3, 1));
        }
        let parameter = [
            proto_integer(1, parameter_flags),
            proto_integer(2, 8),
            proto_bytes(3, &ty(3)),
        ]
        .concat();
        let function = [
            proto_integer(2, 6),
            proto_bytes(3, &result),
            proto_bytes(6, &parameter),
            proto_integer(9, function_flags),
            function_extra.to_vec(),
        ]
        .concat();
        let default = [
            proto_integer(2, 9),
            proto_bytes(3, &ty(7)),
            proto_bytes(6, &parameter),
            proto_integer(9, 22),
        ]
        .concat();
        let signature = [proto_integer(1, 4), proto_integer(2, 5)].concat();
        let property = [
            proto_integer(2, 2),
            proto_bytes(3, &ty(3)),
            proto_integer(11, property_flags),
            proto_bytes(100, &proto_bytes(3, &signature)),
        ]
        .concat();
        let class = [
            proto_integer(1, 102),
            proto_integer(3, 0),
            proto_bytes(6, &ty(1)),
            proto_bytes(9, &function),
            proto_bytes(9, &default),
            proto_bytes(10, &property),
            proto_integer(104, 3),
        ]
        .concat();
        let table = proto_bytes(1, &proto_integer(1, strings.len() as u64));
        let mut data = vec![0];
        let mut length = table.len();
        while length >= 128 {
            data.push((length as u8 & 127) | 128);
            length >>= 7;
        }
        data.push(length as u8);
        data.extend(table);
        data.extend(class);
        let metadata = Annotation {
            name: "Lkotlin/Metadata;".into(),
            values: vec![
                (
                    "mv".into(),
                    AnnotationValue::Array(vec![
                        AnnotationValue::Integer(2),
                        AnnotationValue::Integer(3),
                        AnnotationValue::Integer(0),
                    ]),
                ),
                ("k".into(), AnnotationValue::Integer(1)),
                ("xi".into(), AnnotationValue::Integer(48)),
                (
                    "d1".into(),
                    AnnotationValue::Array(vec![AnnotationValue::String(
                        data.into_iter().map(char::from).collect(),
                    )]),
                ),
                (
                    "d2".into(),
                    AnnotationValue::Array(
                        strings
                            .into_iter()
                            .map(|s| AnnotationValue::String(s.into()))
                            .collect(),
                    ),
                ),
            ],
        };
        let methods = [
            ("getItemCount", "()I", 0x0401),
            ("item", "(I)Ljava/lang/String;", 0x0401),
            ("label", "(I)Ljava/lang/String;", 1),
        ]
        .into_iter()
        .map(|(name, descriptor, access)| JavaMethod {
            library: None,
            owner: "sample/Index".into(),
            name: name.into(),
            descriptor: descriptor.into(),
            access,
            unsupported: false,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        })
        .collect();
        let class = JavaClass {
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            kotlin_fun_interface: false,
            library: None,
            name: "sample/Index".into(),
            access: 0x0601,
            super_name: Some("java/lang/Object".into()),
            interfaces: vec![],
            methods,
            unsupported: true,
            kotlin_interface: false,
            fields: Vec::new(),
            kotlin_class: false,
        };
        let attributes = (0..3).map(|_| Attributes::default()).collect();
        (metadata, class, attributes)
    }
    #[test]
    fn kotlin_interface_metadata_preserves_property_identity_and_default_methods() {
        let (metadata, class, attributes) = fixture_metadata(false, &[], 550, 0, 38);
        assert_eq!(
            kotlin_interface(&metadata, &class, &attributes).unwrap(),
            [Some("itemCount".into()), None, None]
        );
    }
    #[test]
    fn mutable_property_metadata_requires_matching_public_abstract_setters() {
        let (metadata, mut class, mut attributes) = fixture_metadata(false, &[], 1830, 0, 38);
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        class.methods.push(JavaMethod {
            library: None,
            owner: class.name.clone(),
            name: "setItemCount".into(),
            descriptor: "(I)V".into(),
            access: 0x0401,
            unsupported: false,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        });
        attributes.push(Attributes::default());
        assert_eq!(
            kotlin_interface(&metadata, &class, &attributes).unwrap(),
            [
                Some("itemCount".into()),
                None,
                None,
                Some("itemCount".into())
            ]
        );
        class.methods[3].descriptor = "(B)V".into();
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        class.methods[3].descriptor = "(I)V".into();
        class.methods[3].access = 1;
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
    }

    #[test]
    fn unsupported_kotlin_parameters_retain_callable_identity() {
        let (metadata, mut class, attributes) = fixture_metadata(false, &[], 550, 4, 38);
        metadata::attach(&metadata, &mut class, &attributes, &[]).unwrap();
        let method = class.methods.iter().find(|m| m.name == "item").unwrap();
        assert!(method.unsupported);
        assert_eq!(method.library.as_ref().unwrap().source_name, "item");
        assert!(method.kotlin_params.is_none());
    }

    #[test]
    fn general_metadata_retains_internal_setter_visibility() {
        let (mut metadata, mut class, mut attributes) = fixture_metadata(false, &[], 1830, 0, 38);
        class.methods.push(JavaMethod {
            library: None,
            owner: class.name.clone(),
            name: "setItemCount".into(),
            descriptor: "(I)V".into(),
            access: 0x0401,
            unsupported: true,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        });
        attributes.push(Attributes::default());
        let (strings, payload) = metadata_payload(&metadata).unwrap();
        let proto = Proto::parse(&payload).unwrap();
        let payload = proto
            .0
            .into_iter()
            .flat_map(|(number, value)| match value {
                ProtoValue::Integer(value) => proto_integer(number, value),
                ProtoValue::Bytes(bytes) => {
                    let bytes = if number == 10 {
                        [bytes, &proto_integer(8, 32)].concat()
                    } else {
                        bytes.to_vec()
                    };
                    proto_bytes(number, &bytes)
                }
            })
            .collect::<Vec<_>>();
        let table = proto_bytes(1, &proto_integer(1, strings.len() as u64));
        let data = [vec![0, table.len() as u8], table, payload]
            .concat()
            .into_iter()
            .map(char::from)
            .collect::<String>();
        metadata
            .values
            .iter_mut()
            .find(|(name, _)| name == "d1")
            .unwrap()
            .1 = AnnotationValue::Array(vec![AnnotationValue::String(data)]);
        metadata::attach(&metadata, &mut class, &attributes, &[]).unwrap();
        let getter = class
            .methods
            .iter()
            .find(|m| m.name == "getItemCount")
            .unwrap();
        let setter = class
            .methods
            .iter()
            .find(|m| m.name == "setItemCount")
            .unwrap();
        assert!(!getter.unsupported);
        assert!(!getter.kotlin_internal);
        assert!(!setter.unsupported);
        assert!(setter.kotlin_internal);
        assert_eq!(setter.source_property.as_deref(), Some("itemCount"));
    }

    #[test]
    fn mutable_property_metadata_names_reject_non_ascii_and_empty_without_panicking() {
        for name in ["isÄ", ""] {
            let (mut metadata, mut class, mut attributes) =
                fixture_metadata(false, &[], 1830, 0, 38);
            let AnnotationValue::Array(strings) = &mut metadata.values[4].1 else {
                panic!()
            };
            strings[2] = AnnotationValue::String(name.into());
            strings[4] = AnnotationValue::String(name.into());
            class.methods[0].name = name.into();
            class.methods.push(JavaMethod {
                library: None,
                owner: class.name.clone(),
                name: "setItemCount".into(),
                descriptor: "(I)V".into(),
                access: 0x0401,
                unsupported: false,
                source_property: None,
                kotlin_params: None,
                kotlin_result: None,
                kotlin_internal: false,
            });
            attributes.push(Attributes::default());
            assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        }
    }

    #[test]
    fn array_property_metadata_accepts_only_exact_nonnullable_string_elements() {
        let strings = [
            "kotlin/Array",
            "kotlin/String",
            "kotlin/ByteArray",
            "kotlin/CharArray",
        ]
        .map(str::to_owned);
        for (index, descriptor) in [(2, "[B"), (3, "[C")] {
            assert_eq!(
                metadata_type(&proto_integer(6, index), &strings).unwrap(),
                descriptor
            );
        }
        for (projection, nullable, extra, expected) in [
            (2, 0, vec![], true),
            (1, 0, vec![], false),
            (3, 0, vec![], false),
            (2, 1, vec![], false),
            (2, 0, proto_bytes(2, &[]), false),
        ] {
            let element = [proto_integer(6, 1), proto_integer(3, nullable), extra].concat();
            let argument = [proto_integer(1, projection), proto_bytes(2, &element)].concat();
            let ty = [proto_integer(6, 0), proto_bytes(2, &argument)].concat();
            assert_eq!(metadata_type(&ty, &strings).is_ok(), expected);
        }
        assert!(
            metadata_type(
                &[proto_integer(6, 2), proto_bytes(2, &[])].concat(),
                &strings
            )
            .is_err()
        );
    }
    #[test]
    fn kotlin_interface_nullable_generic_extension_suspend_and_default_arguments_are_rejected() {
        for (nullable, extra, property_flags, parameter_flags, function_flags) in [
            (true, vec![], 550, 0, 38),
            (false, proto_bytes(4, &[]), 550, 0, 38),
            (false, proto_bytes(5, &proto_integer(6, 7)), 550, 0, 38),
            (false, vec![], 550, 0, 8192 | 38),
            (false, vec![], 550 | 256, 0, 38),
            (false, vec![], 550, 2, 38),
        ] {
            let (metadata, class, attributes) = fixture_metadata(
                nullable,
                &extra,
                property_flags,
                parameter_flags,
                function_flags,
            );
            assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        }
    }
    #[test]
    fn typed_kotlin_metadata_rejects_mismatched_generic_jvm_signatures() {
        for signature in [
            "(I)Ljava/lang/Object;",
            "(I)Ljava/util/List<Ljava/lang/String;>;",
            "<T:Ljava/lang/Object;>(I)TT;",
        ] {
            let (metadata, class, mut attributes) = fixture_metadata(false, &[], 550, 0, 38);
            attributes[1].signature = Some(signature.into());
            assert!(kotlin_typed_class(&metadata, &class, &attributes).is_err());
        }
    }

    #[test]
    fn kotlin_interface_binary_mismatches_and_extra_annotations_are_rejected() {
        let (metadata, mut class, attributes) = fixture_metadata(false, &[], 550, 0, 38);
        class.methods[1].descriptor = "(I)I".into();
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        let (metadata, mut class, attributes) = fixture_metadata(false, &[], 550, 0, 38);
        class.methods[2].access |= 0x0400;
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        let (metadata, class, mut attributes) = fixture_metadata(false, &[], 550, 0, 38);
        attributes[1].annotations.push(Annotation {
            name: "Lorg/jetbrains/annotations/Nullable;".into(),
            values: vec![],
        });
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        attributes[1].annotations[0].name = "Lorg/jetbrains/annotations/NotNull;".into();
        assert!(kotlin_interface(&metadata, &class, &attributes).is_ok());
    }
    #[test]
    fn kotlin_metadata_version_and_truncated_protobuf_are_rejected() {
        let (mut metadata, class, attributes) = fixture_metadata(false, &[], 550, 0, 38);
        metadata.values[0].1 = AnnotationValue::Array(vec![AnnotationValue::Integer(3)]);
        assert!(kotlin_interface(&metadata, &class, &attributes).is_err());
        let (mut metadata, class, attributes) = fixture_metadata(false, &[], 550, 0, 38);
        let AnnotationValue::Array(values) = &mut metadata.values[3].1 else {
            panic!()
        };
        let AnnotationValue::String(data) = &mut values[0] else {
            panic!()
        };
        let full = data.clone();
        for end in full.char_indices().map(|(index, _)| index) {
            let (mut candidate, _, _) = fixture_metadata(false, &[], 550, 0, 38);
            candidate.values[3].1 =
                AnnotationValue::Array(vec![AnnotationValue::String(full[..end].into())]);
            assert!(
                kotlin_interface(&candidate, &class, &attributes).is_err(),
                "length {end}"
            );
        }
        assert!(Proto::parse(&[0x80; 11]).is_err());
        assert!(Proto::parse(&[10, 255]).is_err());
    }
    #[test]
    fn kotlin_legacy_markers_validate_version_requirements_without_accepting_members() {
        let class = JavaClass {
            sealed_: false,
            sealed_subclasses: Vec::new(),
            annotation_definition: None,
            kotlin_fun_interface: false,
            library: None,
            name: "sample/Marker".into(),
            access: 0x0601,
            super_name: Some("java/lang/Object".into()),
            interfaces: Vec::new(),
            methods: Vec::new(),
            unsupported: true,
            kotlin_interface: false,
            fields: Vec::new(),
            kotlin_class: false,
        };
        for (version, kind, expected) in
            [(33, 1, true), (26, 0, true), (3, 1, false), (33, 3, false)]
        {
            let table = proto_bytes(1, &proto_integer(1, 2));
            let requirement = [proto_integer(1, version), proto_integer(6, kind)].concat();
            let proto = [
                proto_integer(1, 102),
                proto_integer(3, 0),
                proto_bytes(6, &proto_integer(6, 1)),
                proto_integer(31, 0),
                proto_bytes(32, &proto_bytes(1, &requirement)),
            ]
            .concat();
            let data: String = [vec![0, table.len() as u8], table, proto]
                .concat()
                .into_iter()
                .map(char::from)
                .collect();
            let metadata = Annotation {
                name: "Lkotlin/Metadata;".into(),
                values: vec![
                    (
                        "mv".into(),
                        AnnotationValue::Array(
                            [2, 1, 0]
                                .into_iter()
                                .map(AnnotationValue::Integer)
                                .collect(),
                        ),
                    ),
                    ("k".into(), AnnotationValue::Integer(1)),
                    ("xi".into(), AnnotationValue::Integer(48)),
                    (
                        "d1".into(),
                        AnnotationValue::Array(vec![AnnotationValue::String(data)]),
                    ),
                    (
                        "d2".into(),
                        AnnotationValue::Array(
                            ["sample/Marker", "kotlin/Any"]
                                .into_iter()
                                .map(|value| AnnotationValue::String(value.into()))
                                .collect(),
                        ),
                    ),
                ],
            };
            assert_eq!(kotlin_interface(&metadata, &class, &[]).is_ok(), expected);
            let mut with_member = class.clone();
            with_member.methods.push(JavaMethod {
                library: None,
                owner: class.name.clone(),
                name: "required".into(),
                descriptor: "()V".into(),
                access: 0x0401,
                unsupported: false,
                source_property: None,
                kotlin_params: None,
                kotlin_result: None,
                kotlin_internal: false,
            });
            assert!(kotlin_interface(&metadata, &with_member, &[Attributes::default()]).is_err());
        }
    }
    #[test]
    fn metadata_predefined_names_and_descriptor_operations_are_bounded() {
        let table = [
            proto_bytes(1, &proto_integer(2, 8)),
            proto_bytes(1, &proto_integer(3, 2)),
        ]
        .concat();
        assert_eq!(
            metadata_strings(&table, &["".into(), "Lsample/Index$Nested;".into()]).unwrap(),
            ["kotlin/Int", "sample/Index.Nested"]
        );
        assert!(metadata_strings(&table, &["".into(), "bad".into()]).is_err());
        assert!(metadata_strings(&proto_bytes(1, &proto_integer(1, 1000)), &["".into()]).is_err());
    }
    #[test]
    fn kotlin_interface_module_names_are_validated_without_changing_member_contracts() {
        for (fields, expected) in [
            (proto_integer(101, 2), true),
            (proto_integer(101, 1000), false),
            (proto_bytes(101, b"module"), false),
            (
                [proto_integer(101, 2), proto_integer(101, 2)].concat(),
                false,
            ),
        ] {
            let (mut metadata, class, attributes) = fixture_metadata(false, &[], 550, 0, 38);
            let AnnotationValue::Array(values) = &mut metadata.values[3].1 else {
                panic!()
            };
            let AnnotationValue::String(data) = &mut values[0] else {
                panic!()
            };
            data.extend(fields.into_iter().map(char::from));
            assert_eq!(
                kotlin_interface(&metadata, &class, &attributes).is_ok(),
                expected
            );
        }
    }
}
