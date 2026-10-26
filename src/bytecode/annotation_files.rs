use super::{Constant, Pool, annotations, failure, u2};
use crate::Result;
use crate::check::DeclarationAnnotation;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct MemberAnnotations {
    pub name: String,
    pub descriptor: String,
    pub annotations: Vec<DeclarationAnnotation>,
    pub parameters: Vec<Vec<DeclarationAnnotation>>,
}

#[derive(Default)]
pub(crate) struct ClassAnnotations {
    pub annotations: Vec<DeclarationAnnotation>,
    pub fields: Vec<MemberAnnotations>,
    pub methods: Vec<MemberAnnotations>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| failure("Malformed JVM classfile annotation input"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| failure("Truncated JVM classfile annotation input"))?;
        self.offset = end;
        Ok(value)
    }

    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn short(&mut self) -> Result<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn length(&mut self) -> Result<usize> {
        let bytes = self.take(4)?;
        usize::try_from(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
            .map_err(|_| failure("JVM classfile annotation input size exceeded"))
    }

    fn attributes(&mut self) -> Result<()> {
        for _ in 0..self.short()? {
            self.short()?;
            let length = self.length()?;
            self.take(length)?;
        }
        Ok(())
    }

    fn finish(self) -> Result<()> {
        if self.offset != self.bytes.len() {
            return Err(failure(
                "Unexpected trailing JVM classfile annotation input",
            ));
        }
        Ok(())
    }
}

fn original_pool(reader: &mut Reader<'_>) -> Result<(Pool, HashMap<u16, Vec<u8>>)> {
    if reader.take(4)? != [0xca, 0xfe, 0xba, 0xbe] {
        return Err(failure("Invalid JVM classfile annotation input magic"));
    }
    reader.take(4)?;
    let count = reader.short()?;
    if count == 0 {
        return Err(failure("Invalid JVM annotation input constant pool count"));
    }
    let mut pool = Pool {
        entries: vec![Constant::Reserved; usize::from(count) - 1],
        indices: HashMap::new(),
    };
    let mut strings = HashMap::new();
    let mut index = 1u16;
    while index < count {
        match reader.byte()? {
            1 => {
                let length = usize::from(reader.short()?);
                let bytes = reader.take(length)?.to_vec();
                strings.insert(index, bytes.clone());
                pool.indices.entry(Constant::Utf8(bytes)).or_insert(index);
            }
            3 => {
                let bytes = reader.take(4)?;
                let value = i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                pool.indices
                    .entry(Constant::Integer(value))
                    .or_insert(index);
            }
            4 => {
                reader.take(4)?;
            }
            5 | 6 => {
                reader.take(8)?;
                index = index
                    .checked_add(1)
                    .filter(|index| *index < count)
                    .ok_or_else(|| failure("Invalid two-slot JVM annotation input constant"))?;
            }
            7 | 8 | 16 | 19 | 20 => {
                reader.take(2)?;
            }
            9 | 10 | 11 | 12 | 17 | 18 => {
                reader.take(4)?;
            }
            15 => {
                reader.take(3)?;
            }
            _ => return Err(failure("Invalid JVM annotation input constant pool tag")),
        }
        index += 1;
    }
    Ok((pool, strings))
}

fn canonical(pool: &Pool, strings: &HashMap<u16, Vec<u8>>, index: u16) -> Result<u16> {
    let value = strings
        .get(&index)
        .ok_or_else(|| failure("Expected JVM annotation input UTF8 constant"))?;
    pool.indices
        .get(&Constant::Utf8(value.clone()))
        .copied()
        .ok_or_else(|| failure("Missing JVM annotation input UTF8 constant"))
}

fn normalize_annotation_names(
    pool: &Pool,
    strings: &HashMap<u16, Vec<u8>>,
    member: &mut [u8],
) -> Result<()> {
    let mut reader = Reader::new(member);
    reader.take(6)?;
    let mut replacements = Vec::new();
    for _ in 0..reader.short()? {
        let offset = reader.offset;
        let name = reader.short()?;
        let bytes = strings
            .get(&name)
            .ok_or_else(|| failure("Expected JVM annotation input attribute name"))?;
        if [
            b"RuntimeVisibleAnnotations".as_slice(),
            b"RuntimeInvisibleAnnotations".as_slice(),
            b"RuntimeVisibleParameterAnnotations".as_slice(),
            b"RuntimeInvisibleParameterAnnotations".as_slice(),
        ]
        .contains(&bytes.as_slice())
        {
            replacements.push((offset, canonical(pool, strings, name)?));
        }
        let length = reader.length()?;
        reader.take(length)?;
    }
    reader.finish()?;
    for (offset, name) in replacements {
        member[offset..offset + 2].copy_from_slice(&name.to_be_bytes());
    }
    Ok(())
}

fn parameter_count(descriptor: &str) -> Result<usize> {
    fn field(bytes: &[u8], offset: &mut usize) -> Option<()> {
        let mut dimensions = 0usize;
        while bytes.get(*offset) == Some(&b'[') {
            dimensions += 1;
            if dimensions > 255 {
                return None;
            }
            *offset += 1;
        }
        match bytes.get(*offset)? {
            b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' => *offset += 1,
            b'L' => {
                *offset += 1;
                let start = *offset;
                while *bytes.get(*offset)? != b';' {
                    if matches!(bytes[*offset], b'.' | b'[' | b'(' | b')' | 0) {
                        return None;
                    }
                    *offset += 1;
                }
                if *offset == start {
                    return None;
                }
                *offset += 1;
            }
            _ => return None,
        }
        Some(())
    }
    let bytes = descriptor.as_bytes();
    let invalid = || failure("Invalid JVM method descriptor for annotation parameters");
    if bytes.first() != Some(&b'(') {
        return Err(invalid());
    }
    let mut offset = 1;
    let mut count = 0;
    while bytes.get(offset) != Some(&b')') {
        field(bytes, &mut offset).ok_or_else(invalid)?;
        count += 1;
    }
    offset += 1;
    if bytes.get(offset) == Some(&b'V') {
        offset += 1;
    } else {
        field(bytes, &mut offset).ok_or_else(invalid)?;
    }
    if offset != bytes.len() {
        return Err(invalid());
    }
    Ok(count)
}

fn members(
    reader: &mut Reader<'_>,
    pool: &mut Pool,
    strings: &HashMap<u16, Vec<u8>>,
    additions: &[MemberAnnotations],
    is_method: bool,
) -> Result<Vec<u8>> {
    let mut requested = HashMap::new();
    for addition in additions {
        let name = pool.utf8(&addition.name)?;
        let descriptor = pool.utf8(&addition.descriptor)?;
        if requested.insert((name, descriptor), addition).is_some() {
            return Err(failure("Duplicate JVM annotation member target"));
        }
        if !addition.parameters.is_empty()
            && (!is_method || parameter_count(&addition.descriptor)? != addition.parameters.len())
        {
            return Err(failure(
                "JVM annotation descriptor parameter count mismatch",
            ));
        }
    }
    let count = reader.short()?;
    let mut out = Vec::new();
    u2(&mut out, count);
    for _ in 0..count {
        let start = reader.offset;
        reader.short()?;
        let name = canonical(pool, strings, reader.short()?)?;
        let descriptor = canonical(pool, strings, reader.short()?)?;
        reader.attributes()?;
        let mut member = reader.bytes[start..reader.offset].to_vec();
        if let Some(addition) = requested.remove(&(name, descriptor)) {
            if !addition.annotations.is_empty() || !addition.parameters.is_empty() {
                normalize_annotation_names(pool, strings, &mut member)?;
                annotations::append_annotations(pool, &mut member, &addition.annotations)?;
                if !addition.parameters.is_empty() {
                    annotations::append_parameter_annotations(
                        pool,
                        &mut member,
                        &addition.parameters,
                    )?;
                }
            }
        }
        out.extend(member);
    }
    if !requested.is_empty() {
        return Err(failure("Requested JVM annotation member was not found"));
    }
    Ok(out)
}

pub(crate) fn apply(bytes: &[u8], additions: &ClassAnnotations) -> Result<Vec<u8>> {
    let mut reader = Reader::new(bytes);
    let (mut pool, strings) = original_pool(&mut reader)?;
    let pool_end = reader.offset;
    reader.take(6)?;
    let interfaces = usize::from(reader.short()?);
    reader.take(interfaces * 2)?;
    let mut body = bytes[pool_end..reader.offset].to_vec();
    body.extend(members(
        &mut reader,
        &mut pool,
        &strings,
        &additions.fields,
        false,
    )?);
    body.extend(members(
        &mut reader,
        &mut pool,
        &strings,
        &additions.methods,
        true,
    )?);
    let start = reader.offset;
    reader.attributes()?;
    reader.finish()?;
    let mut attributes = vec![0; 6];
    attributes.extend(&bytes[start..]);
    if !additions.annotations.is_empty() {
        normalize_annotation_names(&pool, &strings, &mut attributes)?;
        annotations::append_annotations(&mut pool, &mut attributes, &additions.annotations)?;
    }
    body.extend(&attributes[6..]);
    let mut encoded_pool = Vec::new();
    pool.write(&mut encoded_pool);
    let mut out = bytes[..8].to_vec();
    out.extend(&encoded_pool[..2]);
    out.extend(&bytes[10..pool_end]);
    out.extend(&encoded_pool[2..]);
    out.extend(body);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::DeclarationAnnotationValue;

    fn mark(runtime: bool) -> DeclarationAnnotation {
        DeclarationAnnotation {
            descriptor: "Lsample/Mark;".into(),
            runtime,
            values: vec![("value".into(), DeclarationAnnotationValue::Int(7))],
        }
    }

    fn attr(pool: &mut Pool, name: &str, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        u2(&mut out, pool.utf8(name).unwrap());
        super::super::u4(&mut out, payload.len() as u32);
        out.extend(payload);
        out
    }

    fn fixture() -> (Vec<u8>, usize, Vec<u8>, Vec<u8>) {
        let mut pool = Pool::default();
        let owner = pool.class("sample/Owner").unwrap();
        let base = pool.class("java/lang/Object").unwrap();
        let mut field = vec![0, 1];
        u2(&mut field, pool.utf8("value").unwrap());
        u2(&mut field, pool.utf8("Ljava/lang/String;").unwrap());
        u2(&mut field, 0);
        annotations::append_annotations(
            &mut pool,
            &mut field,
            &[DeclarationAnnotation {
                descriptor: "Lorg/jetbrains/annotations/NotNull;".into(),
                runtime: false,
                values: Vec::new(),
            }],
        )
        .unwrap();
        let mut method = vec![0x04, 1];
        u2(&mut method, pool.utf8("echo").unwrap());
        u2(
            &mut method,
            pool.utf8("(Ljava/lang/String;JD)Ljava/lang/String;")
                .unwrap(),
        );
        u2(&mut method, 0);
        annotations::append_parameter_annotations(
            &mut pool,
            &mut method,
            &[
                vec![DeclarationAnnotation {
                    descriptor: "Lorg/jetbrains/annotations/NotNull;".into(),
                    runtime: false,
                    values: Vec::new(),
                }],
                vec![],
                vec![],
            ],
        )
        .unwrap();
        let metadata = super::super::metadata_attribute_with_annotations(
            &mut pool,
            &crate::metadata::Metadata {
                kind: 1,
                data1: vec![vec![0, 1, 2]],
                data2: vec!["sample/Owner".into()],
            },
            &[],
        )
        .unwrap();
        let unknown = attr(&mut pool, "UnknownPayload", &[9, 0, 7, 255]);
        let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 52];
        pool.write(&mut bytes);
        let pool_end = bytes.len();
        u2(&mut bytes, 0x0421);
        u2(&mut bytes, owner);
        u2(&mut bytes, base);
        u2(&mut bytes, 0);
        u2(&mut bytes, 1);
        bytes.extend(field);
        u2(&mut bytes, 1);
        bytes.extend(method);
        u2(&mut bytes, 2);
        bytes.extend(&metadata);
        bytes.extend(&unknown);
        (bytes, pool_end, metadata, unknown)
    }

    fn member_payloads(bytes: &[u8], method: bool, name: &str) -> Vec<Vec<u8>> {
        let mut reader = Reader::new(bytes);
        let (_, strings) = original_pool(&mut reader).unwrap();
        reader.take(6).unwrap();
        let interfaces = reader.short().unwrap();
        reader.take(usize::from(interfaces) * 2).unwrap();
        for methods in [false, true] {
            for _ in 0..reader.short().unwrap() {
                reader.take(6).unwrap();
                let mut out = Vec::new();
                for _ in 0..reader.short().unwrap() {
                    let key = reader.short().unwrap();
                    let length = reader.length().unwrap();
                    let payload = reader.take(length).unwrap();
                    if strings[&key] == name.as_bytes() {
                        out.push(payload.to_vec());
                    }
                }
                if method == methods {
                    return out;
                }
            }
        }
        Vec::new()
    }

    #[test]
    fn rewrite_preserves_old_pool_metadata_unknown_attributes_and_member_placement() {
        let (original, pool_end, metadata, unknown) = fixture();
        let updated = apply(
            &original,
            &ClassAnnotations {
                annotations: vec![mark(true)],
                fields: vec![MemberAnnotations {
                    name: "value".into(),
                    descriptor: "Ljava/lang/String;".into(),
                    annotations: vec![mark(false)],
                    parameters: Vec::new(),
                }],
                methods: vec![MemberAnnotations {
                    name: "echo".into(),
                    descriptor: "(Ljava/lang/String;JD)Ljava/lang/String;".into(),
                    annotations: vec![mark(true)],
                    parameters: vec![vec![mark(false)], vec![], vec![mark(true)]],
                }],
            },
        )
        .unwrap();
        assert_eq!(&updated[10..pool_end], &original[10..pool_end]);
        assert!(updated.windows(unknown.len()).any(|bytes| bytes == unknown));
        assert!(
            updated
                .windows(metadata.len() - 8)
                .any(|bytes| bytes == &metadata[8..])
        );
        let fields = member_payloads(&updated, false, "RuntimeInvisibleAnnotations");
        assert_eq!(fields.len(), 1);
        assert_eq!(&fields[0][..2], &[0, 2]);
        assert!(member_payloads(&updated, false, "RuntimeVisibleAnnotations").is_empty());
        let methods = member_payloads(&updated, true, "RuntimeVisibleAnnotations");
        assert_eq!(methods.len(), 1);
        assert_eq!(&methods[0][..2], &[0, 1]);
        let parameters = member_payloads(&updated, true, "RuntimeInvisibleParameterAnnotations");
        assert_eq!(parameters.len(), 1);
        assert_eq!(&parameters[0][..3], &[3, 0, 2]);
        assert_eq!(
            apply(&original, &ClassAnnotations::default()).unwrap(),
            original
        );
    }

    #[test]
    fn missing_member_and_incorrect_descriptor_parameter_count_reject() {
        let (original, _, _, _) = fixture();
        for member in [
            MemberAnnotations {
                name: "absent".into(),
                descriptor: "()V".into(),
                ..MemberAnnotations::default()
            },
            MemberAnnotations {
                name: "echo".into(),
                descriptor: "(Ljava/lang/String;JD)Ljava/lang/String;".into(),
                parameters: vec![vec![mark(true)]],
                ..MemberAnnotations::default()
            },
        ] {
            assert!(
                apply(
                    &original,
                    &ClassAnnotations {
                        methods: vec![member],
                        ..ClassAnnotations::default()
                    }
                )
                .is_err()
            );
        }
        assert_eq!(parameter_count("(JD[ILjava/lang/String;)V").unwrap(), 4);
        assert!(parameter_count("(V)V").is_err());
        assert!(parameter_count("([Ljava/lang/String;)Vtrailing").is_err());
    }

    #[test]
    fn original_constant_tags_and_two_slot_positions_survive() {
        let (original, pool_end, _, _) = fixture();
        let mut extra = Vec::new();
        extra.extend([4, 0x3f, 0x80, 0, 0]);
        for tag in [5, 6] {
            extra.push(tag);
            extra.extend([0; 8]);
        }
        for tag in [7, 8, 16, 19, 20] {
            extra.extend([tag, 0, 1]);
        }
        for tag in [9, 10, 11, 12, 17, 18] {
            extra.extend([tag, 0, 1, 0, 1]);
        }
        extra.extend([15, 1, 0, 1]);
        let count = u16::from_be_bytes([original[8], original[9]]);
        let mut extended = original[..8].to_vec();
        u2(&mut extended, count + 17);
        extended.extend(&original[10..pool_end]);
        extended.extend(&extra);
        extended.extend(&original[pool_end..]);
        assert_eq!(
            apply(&extended, &ClassAnnotations::default()).unwrap(),
            extended
        );
        let updated = apply(
            &extended,
            &ClassAnnotations {
                annotations: vec![mark(true)],
                ..ClassAnnotations::default()
            },
        )
        .unwrap();
        assert_eq!(
            &updated[10..pool_end + extra.len()],
            &extended[10..pool_end + extra.len()]
        );
    }

    #[test]
    fn duplicate_utf8_attribute_name_still_merges_existing_metadata() {
        let (original, pool_end, metadata, _) = fixture();
        let count = u16::from_be_bytes([original[8], original[9]]);
        let mut extra = vec![1];
        let name = b"RuntimeVisibleAnnotations";
        u2(&mut extra, name.len() as u16);
        extra.extend(name);
        let mut extended = original[..8].to_vec();
        u2(&mut extended, count + 1);
        extended.extend(&original[10..pool_end]);
        extended.extend(&extra);
        extended.extend(&original[pool_end..]);
        let offset = extended
            .windows(metadata.len())
            .position(|bytes| bytes == metadata)
            .unwrap();
        extended[offset..offset + 2].copy_from_slice(&count.to_be_bytes());
        let updated = apply(
            &extended,
            &ClassAnnotations {
                annotations: vec![mark(true)],
                ..ClassAnnotations::default()
            },
        )
        .unwrap();
        assert_eq!(
            &updated[10..pool_end + extra.len()],
            &extended[10..pool_end + extra.len()]
        );
        let mut reader = Reader::new(&updated);
        let (_, strings) = original_pool(&mut reader).unwrap();
        reader.take(6).unwrap();
        let interfaces = reader.short().unwrap();
        reader.take(usize::from(interfaces) * 2).unwrap();
        for _ in 0..2 {
            for _ in 0..reader.short().unwrap() {
                reader.take(6).unwrap();
                reader.attributes().unwrap();
            }
        }
        let mut visible = Vec::new();
        for _ in 0..reader.short().unwrap() {
            let key = reader.short().unwrap();
            let length = reader.length().unwrap();
            let payload = reader.take(length).unwrap();
            if strings[&key] == name {
                visible.push(payload.to_vec());
            }
        }
        reader.finish().unwrap();
        assert_eq!(visible.len(), 1);
        assert_eq!(&visible[0][..2], &[0, 2]);
        assert_eq!(&visible[0][2..metadata.len() - 6], &metadata[8..]);
    }

    #[test]
    fn malformed_pool_and_classfile_boundaries_reject_without_panics() {
        let (original, _, _, _) = fixture();
        for length in [0, 7, 9, 11, original.len() - 1] {
            assert!(apply(&original[..length], &ClassAnnotations::default()).is_err());
        }
        let mut invalid = original.clone();
        invalid[10] = 2;
        assert!(apply(&invalid, &ClassAnnotations::default()).is_err());
        let mut invalid = original.clone();
        invalid[8..10].copy_from_slice(&0u16.to_be_bytes());
        assert!(apply(&invalid, &ClassAnnotations::default()).is_err());
    }
}
