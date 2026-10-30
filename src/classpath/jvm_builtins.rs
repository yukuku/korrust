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

#[cfg(test)]
mod tests {
    use super::*;

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
