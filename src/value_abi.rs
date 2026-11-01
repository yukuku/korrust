use crate::syntax::Type;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ValueClassInfo {
    pub property: String,
    pub carrier: Type,
}

impl ValueClassInfo {
    pub(crate) fn physical(&self, owner: &str, nullable: bool) -> Type {
        if !nullable {
            self.carrier.clone()
        } else if self.carrier.is_nullable() || !self.carrier.is_reference() {
            Type::Nullable(Box::new(Type::Java(owner.into())))
        } else {
            Type::Nullable(Box::new(self.carrier.clone()))
        }
    }
}

pub(crate) fn mangled_name(
    name: &str,
    parameters: &[Type],
    result: Option<&Type>,
    lookup: impl Fn(&str) -> Option<ValueClassInfo>,
    value_member: bool,
) -> String {
    fn element(ty: &Type, lookup: &impl Fn(&str) -> Option<ValueClassInfo>) -> (String, bool) {
        let (ty, nullable) = match ty.base() {
            Type::Nullable(inner) => (inner.base(), true),
            ty => (ty, false),
        };
        match ty {
            Type::Java(owner) if lookup(owner).is_some() => (
                format!(
                    "L{}{};",
                    owner.replace(['/', '$'], "."),
                    if nullable { "?" } else { "" }
                ),
                true,
            ),
            Type::Parameter { upper_bound, .. } => element(upper_bound, lookup),
            _ => ("_".into(), false),
        }
    }
    let entries = parameters
        .iter()
        .map(|ty| element(ty, &lookup))
        .collect::<Vec<_>>();
    let returns = result
        .map(|ty| element(ty, &lookup))
        .filter(|(_, value)| *value);
    if entries.iter().any(|(_, value)| *value) || returns.is_some() {
        let mut signature = entries
            .into_iter()
            .map(|(part, _)| part)
            .collect::<String>();
        if let Some((part, _)) = returns {
            signature.push(':');
            signature.push_str(&part);
        }
        format!("{name}-{}", hash_suffix(signature.as_bytes()))
    } else if value_member {
        format!("{name}-impl")
    } else {
        name.into()
    }
}

pub(crate) fn hash_suffix(bytes: &[u8]) -> String {
    const SHIFTS: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const CONSTANTS: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x2441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x4881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend((bytes.len() as u64).wrapping_mul(8).to_le_bytes());
    let mut state = [0x67452301u32, 0xefcdab89, 0x98badcfe, 0x10325476];
    for chunk in padded.chunks_exact(64) {
        let words = std::array::from_fn::<_, 16, _>(|i| {
            u32::from_le_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap())
        });
        let [mut a, mut b, mut c, mut d] = state;
        for i in 0..64 {
            let (f, g) = match i {
                0..16 => ((b & c) | (!b & d), i),
                16..32 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..48 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };
            let next = b.wrapping_add(
                a.wrapping_add(f)
                    .wrapping_add(CONSTANTS[i])
                    .wrapping_add(words[g])
                    .rotate_left(SHIFTS[i]),
            );
            a = d;
            d = c;
            c = b;
            b = next;
        }
        for (old, value) in state.iter_mut().zip([a, b, c, d]) {
            *old = old.wrapping_add(value);
        }
    }
    let digest = state
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .take(5)
        .collect::<Vec<_>>();
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();
    let mut bits = 0u32;
    let mut count = 0;
    for byte in digest {
        bits = (bits << 8) | u32::from(byte);
        count += 8;
        while count >= 6 {
            count -= 6;
            result.push(alphabet[((bits >> count) & 63) as usize] as char);
        }
    }
    if count != 0 {
        result.push(alphabet[((bits << (6 - count)) & 63) as usize] as char);
    }
    result
}

pub(crate) type Registry = std::collections::BTreeMap<String, ValueClassInfo>;

pub(crate) fn physical(ty: &Type, registry: &Registry) -> Type {
    let (base, nullable) = match ty.base() {
        Type::Nullable(inner) => (inner.base(), true),
        base => (base, false),
    };
    if let Type::Java(owner) = base {
        if let Some(value) = registry.get(owner) {
            return value.physical(owner, nullable);
        }
    }
    ty.clone()
}

pub(crate) fn nominal_owner(ty: &Type) -> Option<&str> {
    match ty.base() {
        Type::Java(owner) => Some(owner),
        Type::Nullable(inner) => nominal_owner(inner),
        _ => None,
    }
}

pub(crate) fn value_parameter(ty: &Type, registry: &Registry) -> bool {
    nominal_owner(ty).is_some_and(|owner| registry.contains_key(owner))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullable_carriers_preserve_wrapper_null_identity() {
        let primitive = ValueClassInfo {
            property: "raw".into(),
            carrier: Type::Int,
        };
        let reference = ValueClassInfo {
            property: "raw".into(),
            carrier: Type::String,
        };
        let nullable = ValueClassInfo {
            property: "raw".into(),
            carrier: Type::Nullable(Box::new(Type::String)),
        };
        assert_eq!(primitive.physical("V", false), Type::Int);
        assert_eq!(primitive.physical("V", true).descriptor(), "LV;");
        assert_eq!(
            reference.physical("V", true).descriptor(),
            "Ljava/lang/String;"
        );
        assert_eq!(nullable.physical("V", true).descriptor(), "LV;");
    }

    #[test]
    fn mangling_matches_pinned_jvm_signature_hashes() {
        assert_eq!(hash_suffix(b""), "1B2M2Y8");
        assert_eq!(hash_suffix(b"_:Lvalueprobe.Id;"), "CB8Reg4");
        let lookup = |name: &str| {
            (name == "valueprobe/Id").then(|| ValueClassInfo {
                property: "value".into(),
                carrier: Type::Int,
            })
        };
        assert_eq!(
            mangled_name(
                "add",
                &[Type::Int],
                Some(&Type::Java("valueprobe/Id".into())),
                lookup,
                true
            ),
            "add-CB8Reg4"
        );
        assert_eq!(
            mangled_name(
                "getValue",
                &[],
                Some(&Type::Java("valueprobe/Id".into())),
                lookup,
                false
            ),
            "getValue-3joXSjg"
        );
    }
}
