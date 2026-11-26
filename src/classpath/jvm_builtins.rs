pub(crate) struct Mapping {
    pub(crate) owner: &'static str,
    pub(crate) classifier: &'static str,
    pub(crate) source_name: &'static str,
    pub(crate) descriptor: &'static str,
    pub(crate) jvm_name: &'static str,
    pub(crate) inherited_only: bool,
}

pub(crate) const MAPPINGS: &[Mapping] = &[
    Mapping {
        owner: "java/util/List",
        classifier: "kotlin/collections/MutableList",
        source_name: "removeAt",
        descriptor: "(I)Ljava/lang/Object;",
        jvm_name: "remove",
        inherited_only: false,
    },
    Mapping {
        owner: "java/lang/CharSequence",
        classifier: "kotlin/CharSequence",
        source_name: "get",
        descriptor: "(I)C",
        jvm_name: "charAt",
        inherited_only: true,
    },
];

pub(crate) const PROPERTY_MAPPINGS: &[Mapping] = &[
    Mapping {
        owner: "java/lang/CharSequence",
        classifier: "kotlin/CharSequence",
        source_name: "length",
        descriptor: "()I",
        jvm_name: "length",
        inherited_only: false,
    },
    Mapping {
        owner: "java/util/Collection",
        classifier: "kotlin/collections/Collection",
        source_name: "size",
        descriptor: "()I",
        jvm_name: "size",
        inherited_only: false,
    },
    Mapping {
        owner: "java/util/Map",
        classifier: "kotlin/collections/Map",
        source_name: "size",
        descriptor: "()I",
        jvm_name: "size",
        inherited_only: false,
    },
    Mapping {
        owner: "java/util/Map",
        classifier: "kotlin/collections/Map",
        source_name: "keys",
        descriptor: "()Ljava/util/Set;",
        jvm_name: "keySet",
        inherited_only: false,
    },
    Mapping {
        owner: "java/util/Map",
        classifier: "kotlin/collections/Map",
        source_name: "values",
        descriptor: "()Ljava/util/Collection;",
        jvm_name: "values",
        inherited_only: false,
    },
    Mapping {
        owner: "java/util/Map",
        classifier: "kotlin/collections/Map",
        source_name: "entries",
        descriptor: "()Ljava/util/Set;",
        jvm_name: "entrySet",
        inherited_only: false,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_property_contract_matches_the_pinned_table() {
        let reference = include_str!(
            "../../references/kotlin/core/compiler.common.jvm/src/org/jetbrains/kotlin/load/java/BuiltinSpecialProperties.kt"
        );
        assert!(reference.contains("StandardNames.FqNames.charSequence.childSafe(\"length\") to Name.identifier(\"length\")"));
        for mapping in PROPERTY_MAPPINGS {
            let classifier = match mapping.classifier {
                "kotlin/CharSequence" => "charSequence.childSafe",
                "kotlin/collections/Collection" => "collection.child",
                "kotlin/collections/Map" => "map.child",
                _ => panic!("unverified builtin property classifier"),
            };
            assert!(reference.contains(&format!(
                "StandardNames.FqNames.{classifier}(\"{}\") to Name.identifier(\"{}\")",
                mapping.source_name, mapping.jvm_name,
            )));
        }
    }

    #[test]
    fn mapped_properties_retain_builtin_generic_parameters() {
        let standard = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        let classpath = crate::classpath::Classpath::new(&[standard]).unwrap();
        for mapping in PROPERTY_MAPPINGS {
            let result = classpath
                .builtin_property_type(mapping.classifier, mapping.source_name)
                .unwrap();
            assert_eq!(format!("(){}", result.descriptor()), mapping.descriptor);
            if mapping.source_name == "keys" {
                let crate::syntax::Type::Parameterized { name, arguments } = result else {
                    panic!("keys must retain its generic set type");
                };
                assert_eq!(name, "java/util/Set");
                assert!(matches!(
                    arguments[0].ty.as_ref(),
                    Some(crate::syntax::Type::Parameter { name, .. }) if name == "K"
                ));
            }
        }
    }

    #[test]
    fn supported_mappings_match_the_pinned_jvm_contract_table() {
        let reference = include_str!(
            "../../references/kotlin/core/compiler.common.jvm/src/org/jetbrains/kotlin/load/java/SpecialGenericSignatures.kt"
        );
        for mapping in MAPPINGS {
            assert!(reference.contains(&format!("\"{}\"", mapping.source_name)));
            assert!(reference.contains(&format!("Name.identifier(\"{}\")", mapping.jvm_name)));
        }
        let compact = reference
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact.contains("\"java/util/List\".method(\"removeAt\",JvmPrimitiveType.INT.desc,\"Ljava/lang/Object;\")"));
        assert!(compact.contains("REMOVE_AT_NAME_AND_SIGNATUREtoName.identifier(\"remove\")"));
        assert!(compact.contains("javaLang(\"CharSequence\").method(\"get\",JvmPrimitiveType.INT.desc,JvmPrimitiveType.CHAR.desc)toName.identifier(\"charAt\")"));
    }
}
