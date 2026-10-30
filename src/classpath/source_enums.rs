use super::*;
use crate::syntax::Variance;

pub(super) fn header(module: &Module, owner: &str, cp: &Classpath) -> Result<JavaClass> {
    let source = module
        .enum_decl
        .as_ref()
        .ok_or_else(|| Diagnostic::new(0, "Enum source header requires an enum declaration"))?;
    let mut context = module.clone();
    context.enum_decl = None;
    context.class_decl = Some(source.body.clone());
    let mut class = super::header(&context, owner, cp)?;
    class.access = 0x4031;
    class.super_name = Some("java/lang/Enum".into());
    for method in &mut class.methods {
        if method.name == "<init>" {
            method.access = (method.access & !5) | 2;
        }
    }
    for (name, _) in &source.entries {
        class.fields.push(JavaField {
            kotlin: None,
            constant: None,
            owner: owner.into(),
            name: name.clone(),
            descriptor: format!("L{owner};"),
            access: 0x4019,
            unsupported: false,
        });
    }
    let element = Type::Java(owner.into());
    let entries = Type::Parameterized {
        name: "kotlin/enums/EnumEntries".into(),
        arguments: vec![TypeArgument {
            variance: Variance::Invariant,
            ty: Some(element.clone()),
        }],
    };
    for (name, params, result, property) in [
        (
            "values",
            Vec::new(),
            Type::Array(Box::new(element.clone())),
            None,
        ),
        (
            "valueOf",
            vec![("value".into(), Type::String, false)],
            element.clone(),
            None,
        ),
        ("getEntries", Vec::new(), entries, Some("entries")),
    ] {
        class.methods.push(JavaMethod {
            library: Some(LibraryMethod {
                has_annotations: false,
                abstract_: false,
                builtin_array: false,
                kotlin_public: true,
                source_name: name.into(),
                type_parameters: Vec::new(),
                receiver: None,
                infix: false,
                operator: false,
                inline: false,
                code: None,
            }),
            owner: owner.into(),
            name: name.into(),
            descriptor: format!(
                "({}){}",
                params
                    .iter()
                    .map(|(_, ty, _)| ty.descriptor())
                    .collect::<String>(),
                result.descriptor()
            ),
            access: 9,
            unsupported: false,
            source_property: property.map(str::to_owned),
            kotlin_params: Some(params),
            kotlin_result: Some(result),
            kotlin_internal: false,
        });
    }
    class.library = Some(LibraryClass {
        kotlin_visibility: Some(if source.body.internal { 0 } else { 3 }),
        type_aliases: Vec::new(),
        type_parameters: Vec::new(),
        supertypes: std::iter::once(Type::Parameterized {
            name: "java/lang/Enum".into(),
            arguments: vec![TypeArgument {
                variance: Variance::Invariant,
                ty: Some(element),
            }],
        })
        .chain(class.interfaces.iter().cloned().map(Type::Java))
        .collect(),
        package: None,
        multifile_facade: None,
    });
    Ok(class)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_header_preserves_user_members_private_constructor_and_companion() {
        let module = crate::syntax::parse(include_str!(
            "../../tests/fixtures/compatibility_enums_numeric/EnumModel.kt"
        ))
        .unwrap();
        let cp = Classpath::new(&[]).unwrap();
        let class = header(&module, "compatibility/enumsnumeric/Signal", &cp).unwrap();
        assert_eq!(class.access, 0x4031);
        assert_eq!(class.super_name.as_deref(), Some("java/lang/Enum"));
        assert!(class.methods.iter().any(|method| method.name == "<init>"
            && method.descriptor == "(ILjava/lang/String;)V"
            && method.access & 7 == 2));
        assert!(class.methods.iter().any(|method| method.name == "getCode"));
        assert!(class.methods.iter().any(|method| method.name == "caption"));
        assert!(class.fields.iter().any(|field| field.name == "RED"
            && field.descriptor == "Lcompatibility/enumsnumeric/Signal;"
            && field.access == 0x4019));
        let companion = cp
            .load("compatibility/enumsnumeric/Signal$Companion")
            .unwrap();
        assert!(
            companion
                .methods
                .iter()
                .any(|method| method.name == "fromCode")
        );
        assert!(class.library.as_ref().unwrap().package.is_none());
    }

    #[test]
    fn generated_enum_members_preserve_named_parameters_and_generic_results() {
        let module = crate::syntax::parse("enum class Choice { FIRST }").unwrap();
        let cp = Classpath::new(&[]).unwrap();
        let class = header(&module, "sample/Choice", &cp).unwrap();
        let method = |name: &str| {
            class
                .methods
                .iter()
                .find(|method| method.name == name)
                .unwrap()
        };
        assert_eq!(method("values").descriptor, "()[Lsample/Choice;");
        assert_eq!(
            method("valueOf").descriptor,
            "(Ljava/lang/String;)Lsample/Choice;"
        );
        assert_eq!(
            method("valueOf").kotlin_params.as_ref().unwrap(),
            &[("value".into(), Type::String, false)]
        );
        let entries = method("getEntries");
        assert_eq!(entries.source_property.as_deref(), Some("entries"));
        assert!(
            matches!(entries.kotlin_result.as_ref().unwrap(), Type::Parameterized { name, arguments }
            if name == "kotlin/enums/EnumEntries" && arguments[0].ty == Some(Type::Java("sample/Choice".into())))
        );
        assert!(
            matches!(&class.library.unwrap().supertypes[0], Type::Parameterized { name, arguments }
            if name == "java/lang/Enum" && arguments[0].ty == Some(Type::Java("sample/Choice".into())))
        );
    }
}
