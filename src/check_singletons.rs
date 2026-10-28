use super::*;

fn resolve(name: &str, imports: &HashMap<String, String>, package: &str) -> String {
    if name.contains('/') {
        name.into()
    } else if let Some(owner) = imports.get(name) {
        owner.clone()
    } else if package.is_empty() {
        name.into()
    } else {
        format!("{}/{}", package.replace('.', "/"), name)
    }
}

fn marker(
    name: &str,
    classpath: &Classpath,
    visiting: &mut HashSet<String>,
    checked: &mut HashSet<String>,
    offset: usize,
) -> Result<()> {
    if checked.contains(name) {
        return Ok(());
    }
    if !visiting.insert(name.into()) || visiting.len() + checked.len() > 128 {
        return Err(Diagnostic::new(
            offset,
            "Unsupported or cyclic marker interface hierarchy",
        ));
    }
    let class = classpath.load(name)?;
    if class.access != 0x0601
        || class.unsupported
        || class.super_name.as_deref() != Some("java/lang/Object")
        || class.methods.iter().any(|method| method.access & 8 == 0)
    {
        return Err(Diagnostic::new(
            offset,
            "Object supertypes must be supported public marker interfaces without instance members",
        ));
    }
    for parent in &class.interfaces {
        marker(parent, classpath, visiting, checked, offset)?;
    }
    visiting.remove(name);
    checked.insert(name.into());
    Ok(())
}

#[cfg(test)]
pub(super) fn check(source: &syntax::Module, classpath: &Classpath) -> Result<Module> {
    let object = source.object_decl.as_ref().unwrap();
    let owner = if source.package.is_empty() {
        object.name.clone()
    } else {
        format!("{}/{}", source.package.replace('.', "/"), object.name)
    };
    check_with_owner(source, classpath, &owner)
}
pub(super) fn check_with_owner(
    source: &syntax::Module,
    classpath: &Classpath,
    owner: &str,
) -> Result<Module> {
    let object = source.object_decl.as_ref().unwrap();
    if !object.fields.is_empty()
        || !object.methods.is_empty()
        || !object.constants.is_empty()
        || !object.objects.is_empty()
    {
        return Err(Diagnostic::new(
            object.offset,
            "Annotated or interface-bearing objects support only empty singleton declarations",
        ));
    }
    let mut imports = HashMap::new();
    for (alias, name, offset) in &source.imports {
        if alias == &object.name || imports.insert(alias.clone(), name.clone()).is_some() {
            return Err(Diagnostic::new(*offset, "Conflicting singleton import"));
        }
        if !object
            .interfaces
            .iter()
            .chain(&object.annotations)
            .any(|used| used == alias)
        {
            continue;
        }
        let class = classpath.load(name)?;
        if class.access & 1 == 0
            || (class.unsupported && name != "kotlinx/serialization/Serializable")
        {
            return Err(Diagnostic::new(
                *offset,
                "Imported singleton dependency is inaccessible or unsupported",
            ));
        }
    }
    if object.annotations.len() > 1 {
        return Err(Diagnostic::new(
            object.offset,
            "Only one kotlinx.serialization.Serializable object annotation is supported",
        ));
    }
    let serializable = if let Some(annotation) = object.annotations.first() {
        let name = resolve(annotation, &imports, &source.package);
        if name != "kotlinx/serialization/Serializable" {
            return Err(Diagnostic::new(
                object.offset,
                "Only kotlinx.serialization.Serializable object annotations are supported",
            ));
        }
        let class = classpath.load(&name)?;
        if class.access & 0x2601 != 0x2601 {
            return Err(Diagnostic::new(
                object.offset,
                "Serializable must resolve to a public annotation class",
            ));
        }
        for (name, interface, method, descriptor) in [
            ("kotlinx/serialization/KSerializer", true, "", ""),
            (
                "kotlinx/serialization/internal/ObjectSerializer",
                false,
                "<init>",
                "(Ljava/lang/String;Ljava/lang/Object;[Ljava/lang/annotation/Annotation;)V",
            ),
        ] {
            let class = classpath.load(name)?;
            if class.access & 1 == 0
                || (class.access & 0x0200 != 0) != interface
                || (!interface
                    && (class.access & 0x0400 != 0
                        || !class
                            .interfaces
                            .contains(&"kotlinx/serialization/KSerializer".into())))
                || (!method.is_empty()
                    && !class.methods.iter().any(|candidate| {
                        candidate.name == method
                            && candidate.descriptor == descriptor
                            && candidate.access & 1 != 0
                    }))
            {
                return Err(Diagnostic::new(
                    object.offset,
                    "Unsupported kotlinx.serialization runtime ABI",
                ));
            }
        }
        true
    } else {
        false
    };
    let mut interfaces = Vec::new();
    let mut checked = HashSet::new();
    for name in &object.interfaces {
        if name == &object.name {
            return Err(Diagnostic::new(
                object.offset,
                "Object cannot inherit itself",
            ));
        }
        let name = resolve(name, &imports, &source.package);
        if interfaces.contains(&name) {
            return Err(Diagnostic::new(object.offset, "Repeated object supertype"));
        }
        marker(
            &name,
            classpath,
            &mut HashSet::new(),
            &mut checked,
            object.offset,
        )?;
        interfaces.push(name);
    }
    Ok(Module {
        exposed_fields: HashMap::new(),
        properties: Vec::new(),
        initializers: Vec::new(),
        locals: 0,
        lambdas: Vec::new(),
        constants: Vec::new(),
        functions: Vec::new(),
        class_decl: None,
        interface_decl: None,
        object_decl: Some(DynamicObject {
            bridges: Vec::new(),
            exposed_fields: HashMap::new(),
            constants: object.constants.clone(),
            initializers: Vec::new(),
            locals: 0,
            owner: owner.into(),
            interfaces,
            serializable,
            fields: Vec::new(),
            methods: Vec::new(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn interface(name: &str, parents: &[&str], abstract_member: bool) -> Vec<u8> {
        fn word(bytes: &mut Vec<u8>, value: u16) {
            bytes.extend(value.to_be_bytes());
        }
        let names = [
            vec![name, "java/lang/Object", "required", "()V"],
            parents.to_vec(),
        ]
        .concat();
        let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 49];
        word(&mut bytes, (names.len() * 2 + 1) as u16);
        for (index, name) in names.iter().enumerate() {
            bytes.push(1);
            word(&mut bytes, name.len() as u16);
            bytes.extend(name.as_bytes());
            bytes.push(7);
            word(&mut bytes, (index * 2 + 1) as u16);
        }
        for value in [0x0601, 2, 4, parents.len() as u16] {
            word(&mut bytes, value);
        }
        for index in 0..parents.len() {
            word(&mut bytes, (10 + index * 2) as u16);
        }
        word(&mut bytes, 0);
        word(&mut bytes, u16::from(abstract_member));
        if abstract_member {
            for value in [0x0401, 5, 7, 0] {
                word(&mut bytes, value);
            }
        }
        word(&mut bytes, 0);
        bytes
    }

    #[test]
    fn marker_hierarchies_preserve_shared_parents_and_reject_behavior_cycles_and_limits() {
        let directory = std::env::temp_dir().join(format!(
            "korrust-marker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        for (name, parents, member) in [
            ("Base", vec![], false),
            ("Left", vec!["Base"], false),
            ("Right", vec!["Base"], false),
            ("Diamond", vec!["Left", "Right"], false),
            ("Required", vec![], true),
            ("Child", vec!["Required"], false),
            ("CycleA", vec!["CycleB"], false),
            ("CycleB", vec!["CycleA"], false),
        ] {
            std::fs::write(
                directory.join(format!("{name}.class")),
                interface(name, &parents, member),
            )
            .unwrap();
        }
        let classpath = Classpath::new(std::slice::from_ref(&directory)).unwrap();
        let source = syntax::parse("import Diamond\nobject Example : Diamond").unwrap();
        let checked = check(&source, &classpath).unwrap().object_decl.unwrap();
        assert_eq!(checked.interfaces, ["Diamond"]);
        assert!(!checked.serializable);
        for source in [
            "import Child\nobject Example : Child",
            "import CycleA\nobject Example : CycleA",
            "import Base\nobject Example : Base, Base",
        ] {
            assert!(check(&syntax::parse(source).unwrap(), &classpath).is_err());
        }
        for index in 0..129 {
            let name = format!("Marker{index}");
            let parent = format!("Marker{}", index + 1);
            let parents = if index == 128 {
                vec![]
            } else {
                vec![parent.as_str()]
            };
            std::fs::write(
                directory.join(format!("{name}.class")),
                interface(&name, &parents, false),
            )
            .unwrap();
        }
        assert!(
            check(
                &syntax::parse("import Marker0\nobject Example : Marker0").unwrap(),
                &classpath
            )
            .is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn annotation_and_singleton_extensions_fail_closed() {
        for source in [
            "@Serializable data object Example",
            "object Example : Base()",
            "object Example : Base<String>",
            "object Example : Base by delegate",
            "object Outer { object Inner : Base }",
        ] {
            assert!(syntax::parse(source).is_err(), "{source}");
        }
        // Ordinary annotation grammar is valid independently of whether its
        // declaration/dependencies and selected lowering are supported.
        for source in [
            "@Serializable() object Example",
            "@Serializable(with = Custom::class) object Example",
            "@Serializable class Example",
        ] {
            assert!(syntax::parse(source).is_ok(), "{source}");
            assert!(
                crate::compile_all(source, "Example.kt").is_err(),
                "{source}"
            );
        }
        let classpath = Classpath::new(&[]).unwrap();
        for source in [
            "@Other object Example",
            "@Serializable @Serializable object Example",
            "@Serializable object Example { const val answer = 42 }",
            "object Example : Base { fun answer(): Int = 42 }",
            "object Example : Base { object Inner }",
        ] {
            assert!(
                check(&syntax::parse(source).unwrap(), &classpath).is_err(),
                "{source}"
            );
        }
    }
}
