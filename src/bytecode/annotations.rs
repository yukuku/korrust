use super::{Constant, Pool, failure, u2, u4};
use crate::Result;
use crate::check::{DeclarationAnnotation, DeclarationAnnotationValue};

fn count(value: usize) -> Result<u16> {
    u16::try_from(value).map_err(|_| failure("JVM annotation count limit exceeded"))
}

fn annotation(pool: &mut Pool, value: &DeclarationAnnotation) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    u2(&mut out, pool.utf8(&value.descriptor)?);
    u2(&mut out, count(value.values.len())?);
    for (name, value) in &value.values {
        u2(&mut out, pool.utf8(name)?);
        match value {
            DeclarationAnnotationValue::Int(value) => {
                out.push(b'I');
                u2(&mut out, pool.add(Constant::Integer(*value))?);
            }
            DeclarationAnnotationValue::Boolean(value) => {
                out.push(b'Z');
                u2(&mut out, pool.add(Constant::Integer(i32::from(*value)))?);
            }
            DeclarationAnnotationValue::String(value) => {
                out.push(b's');
                u2(&mut out, pool.utf8(value)?);
            }
        }
    }
    Ok(out)
}

fn declaration_payload(
    pool: &mut Pool,
    values: &[DeclarationAnnotation],
    runtime: bool,
) -> Result<Option<Vec<u8>>> {
    let selected: Vec<_> = values
        .iter()
        .filter(|value| value.runtime == runtime)
        .collect();
    if selected.is_empty() {
        return Ok(None);
    }
    let mut out = Vec::new();
    u2(&mut out, count(selected.len())?);
    for value in selected {
        out.extend(annotation(pool, value)?);
    }
    Ok(Some(out))
}

fn attribute(name: u16, payload: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    u2(&mut out, name);
    u4(
        &mut out,
        u32::try_from(payload.len())
            .map_err(|_| failure("JVM annotation attribute size exceeded"))?,
    );
    out.extend(payload);
    Ok(out)
}

#[cfg(test)]
pub(super) fn attributes(
    pool: &mut Pool,
    values: &[DeclarationAnnotation],
) -> Result<Vec<Vec<u8>>> {
    let mut out = Vec::new();
    for (runtime, name) in [
        (true, "RuntimeVisibleAnnotations"),
        (false, "RuntimeInvisibleAnnotations"),
    ] {
        if let Some(payload) = declaration_payload(pool, values, runtime)? {
            out.push(attribute(pool.utf8(name)?, &payload)?);
        }
    }
    Ok(out)
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
            .ok_or_else(|| failure("Malformed JVM annotation attribute"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| failure("Truncated JVM annotation attribute"))?;
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
            .map_err(|_| failure("JVM annotation attribute size exceeded"))
    }

    fn finish(self) -> Result<()> {
        if self.offset != self.bytes.len() {
            return Err(failure("Unexpected bytes after JVM annotation attribute"));
        }
        Ok(())
    }

    fn annotation(&mut self, depth: usize) -> Result<()> {
        if depth > 256 {
            return Err(failure("JVM annotation nesting limit exceeded"));
        }
        self.short()?;
        let pairs = self.short()?;
        for _ in 0..pairs {
            self.short()?;
            self.value(depth + 1)?;
        }
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<()> {
        if depth > 256 {
            return Err(failure("JVM annotation nesting limit exceeded"));
        }
        match self.byte()? {
            b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z' | b's' | b'c' => {
                self.short()?;
            }
            b'e' => {
                self.take(4)?;
            }
            b'@' => self.annotation(depth + 1)?,
            b'[' => {
                let length = self.short()?;
                for _ in 0..length {
                    self.value(depth + 1)?;
                }
            }
            _ => return Err(failure("Invalid JVM annotation element tag")),
        }
        Ok(())
    }

    fn annotations(&mut self) -> Result<(u16, &'a [u8])> {
        let length = self.short()?;
        let start = self.offset;
        for _ in 0..length {
            self.annotation(0)?;
        }
        Ok((length, &self.bytes[start..self.offset]))
    }
}

fn merge_annotations(existing: Option<&[u8]>, added: &[u8]) -> Result<Vec<u8>> {
    let mut out = vec![0, 0];
    let mut total = 0u16;
    for payload in existing.into_iter().chain(std::iter::once(added)) {
        let mut reader = Reader::new(payload);
        let (length, bytes) = reader.annotations()?;
        reader.finish()?;
        total = total
            .checked_add(length)
            .ok_or_else(|| failure("JVM annotation count limit exceeded"))?;
        out.extend(bytes);
    }
    out[..2].copy_from_slice(&total.to_be_bytes());
    Ok(out)
}

fn parameter_lists(payload: &[u8]) -> Result<Vec<(u16, &[u8])>> {
    let mut reader = Reader::new(payload);
    let parameters = reader.byte()?;
    let mut out = Vec::new();
    for _ in 0..parameters {
        out.push(reader.annotations()?);
    }
    reader.finish()?;
    Ok(out)
}

fn merge_parameters(existing: Option<&[u8]>, added: &[u8]) -> Result<Vec<u8>> {
    let right = parameter_lists(added)?;
    let left = existing.map(parameter_lists).transpose()?;
    if left.as_ref().is_some_and(|left| left.len() != right.len()) {
        return Err(failure("Mismatched JVM annotation parameter counts"));
    }
    let mut out = vec![right.len() as u8];
    for (index, (right_count, right_bytes)) in right.iter().enumerate() {
        let (left_count, left_bytes) = left.as_ref().map_or((0, &[][..]), |left| left[index]);
        u2(
            &mut out,
            left_count
                .checked_add(*right_count)
                .ok_or_else(|| failure("JVM annotation count limit exceeded"))?,
        );
        out.extend(left_bytes);
        out.extend(*right_bytes);
    }
    Ok(out)
}

type MergePayload = fn(Option<&[u8]>, &[u8]) -> Result<Vec<u8>>;

fn merge_member(
    member: &mut Vec<u8>,
    names: [u16; 2],
    added: [Option<Vec<u8>>; 2],
    merge: MergePayload,
) -> Result<()> {
    let mut reader = Reader::new(member);
    let header = reader.take(6)?.to_vec();
    let length = reader.short()?;
    let mut entries = Vec::new();
    let mut merged: [Option<Vec<u8>>; 2] = [None, None];
    for _ in 0..length {
        let name = reader.short()?;
        let length = reader.length()?;
        let payload = reader.take(length)?.to_vec();
        if let Some(index) = names.iter().position(|value| *value == name) {
            merged[index] = Some(merge(merged[index].as_deref(), &payload)?);
        }
        entries.push((name, payload));
    }
    reader.finish()?;
    for (index, payload) in added.iter().enumerate() {
        if let Some(payload) = payload {
            merged[index] = Some(merge(merged[index].as_deref(), payload)?);
        }
    }
    let mut output = Vec::new();
    let mut attribute_count = 0usize;
    for (name, payload) in entries {
        let payload = if let Some(index) = names.iter().position(|value| *value == name) {
            let Some(payload) = merged[index].take() else {
                continue;
            };
            payload
        } else {
            payload
        };
        output.extend(attribute(name, &payload)?);
        attribute_count += 1;
    }
    for (index, payload) in merged.into_iter().enumerate() {
        if let Some(payload) = payload {
            output.extend(attribute(names[index], &payload)?);
            attribute_count += 1;
        }
    }
    let mut updated = header;
    u2(&mut updated, count(attribute_count)?);
    updated.extend(output);
    *member = updated;
    Ok(())
}

pub(super) fn append_annotations(
    pool: &mut Pool,
    member: &mut Vec<u8>,
    values: &[DeclarationAnnotation],
) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    let added = [
        declaration_payload(pool, values, true)?,
        declaration_payload(pool, values, false)?,
    ];
    let names = [
        pool.utf8("RuntimeVisibleAnnotations")?,
        pool.utf8("RuntimeInvisibleAnnotations")?,
    ];
    merge_member(member, names, added, merge_annotations)
}

pub(super) fn append_parameter_annotations(
    pool: &mut Pool,
    member: &mut Vec<u8>,
    parameters: &[Vec<DeclarationAnnotation>],
) -> Result<()> {
    if parameters.iter().all(Vec::is_empty) {
        return Ok(());
    }
    let parameter_count =
        u8::try_from(parameters.len()).map_err(|_| failure("Too many annotated JVM parameters"))?;
    let mut added = [None, None];
    for (index, runtime) in [true, false].into_iter().enumerate() {
        if !parameters
            .iter()
            .flatten()
            .any(|annotation| annotation.runtime == runtime)
        {
            continue;
        }
        let mut payload = vec![parameter_count];
        for annotations in parameters {
            payload.extend(
                declaration_payload(pool, annotations, runtime)?.unwrap_or_else(|| vec![0, 0]),
            );
        }
        added[index] = Some(payload);
    }
    let names = [
        pool.utf8("RuntimeVisibleParameterAnnotations")?,
        pool.utf8("RuntimeInvisibleParameterAnnotations")?,
    ];
    merge_member(member, names, added, merge_parameters)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marker(descriptor: &str, runtime: bool) -> DeclarationAnnotation {
        DeclarationAnnotation {
            descriptor: descriptor.into(),
            runtime,
            values: Vec::new(),
        }
    }

    fn member(attrs: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![0, 1, 0, 2, 0, 3];
        u2(&mut out, attrs.len() as u16);
        for attr in attrs {
            out.extend(attr);
        }
        out
    }

    fn payloads(member: &[u8], name: u16) -> Vec<Vec<u8>> {
        let mut reader = Reader::new(member);
        reader.take(6).unwrap();
        let mut out = Vec::new();
        for _ in 0..reader.short().unwrap() {
            let key = reader.short().unwrap();
            let size = reader.length().unwrap();
            let value = reader.take(size).unwrap();
            if key == name {
                out.push(value.to_vec());
            }
        }
        reader.finish().unwrap();
        out
    }

    #[test]
    fn scalar_elements_have_jvm_tags_and_utf8_string_constants() {
        let mut pool = Pool::default();
        let values = [DeclarationAnnotation {
            descriptor: "Lsample/Mark;".into(),
            runtime: true,
            values: vec![
                ("number".into(), DeclarationAnnotationValue::Int(-7)),
                ("enabled".into(), DeclarationAnnotationValue::Boolean(true)),
                (
                    "text".into(),
                    DeclarationAnnotationValue::String("a\0💩".into()),
                ),
            ],
        }];
        let encoded = attributes(&mut pool, &values).unwrap();
        assert_eq!(
            encoded,
            [vec![
                0, 8, 0, 0, 0, 21, 0, 1, 0, 1, 0, 3, 0, 2, b'I', 0, 3, 0, 4, b'Z', 0, 5, 0, 6,
                b's', 0, 7,
            ]]
        );
        assert!(matches!(pool.entries[2], Constant::Integer(-7)));
        assert!(matches!(pool.entries[4], Constant::Integer(1)));
        assert!(
            matches!(&pool.entries[6], Constant::Utf8(bytes) if bytes == &[b'a', 0xc0, 0x80, 0xed, 0xa0, 0xbd, 0xed, 0xb2, 0xa9])
        );
    }

    #[test]
    fn member_merge_coalesces_both_retentions_and_preserves_other_attributes() {
        let mut pool = Pool::default();
        let first = super::super::metadata_attribute_with_annotations(
            &mut pool,
            &crate::metadata::Metadata {
                kind: 1,
                data1: vec![vec![0, 1, 0x1234]],
                data2: vec!["sample/Owner".into(), "value".into()],
            },
            &[],
        )
        .unwrap();
        let second = attributes(&mut pool, &[marker("Lsample/Existing;", true)])
            .unwrap()
            .remove(0);
        let nullability = attributes(
            &mut pool,
            &[marker("Lorg/jetbrains/annotations/NotNull;", false)],
        )
        .unwrap()
        .remove(0);
        let code = attribute(pool.utf8("Code").unwrap(), &[9, 8, 7]).unwrap();
        let mut value = member(&[
            first.clone(),
            code.clone(),
            second.clone(),
            nullability.clone(),
        ]);
        append_annotations(&mut pool, &mut value, &[marker("Lsample/Added;", false)]).unwrap();
        assert_eq!(u16::from_be_bytes([value[6], value[7]]), 3);
        assert!(value.windows(code.len()).any(|bytes| bytes == code));
        let visible = pool.utf8("RuntimeVisibleAnnotations").unwrap();
        let invisible = pool.utf8("RuntimeInvisibleAnnotations").unwrap();
        let visible_payloads = payloads(&value, visible);
        assert_eq!(visible_payloads.len(), 1);
        assert_eq!(&visible_payloads[0][..2], &[0, 2]);
        let metadata_end = 2 + first.len() - 8;
        assert_eq!(&visible_payloads[0][2..metadata_end], &first[8..]);
        assert_eq!(&visible_payloads[0][metadata_end..], &second[8..]);
        let invisible_payloads = payloads(&value, invisible);
        assert_eq!(invisible_payloads.len(), 1);
        assert_eq!(&invisible_payloads[0][..2], &[0, 2]);
        assert_eq!(&invisible_payloads[0][2..6], &nullability[8..]);
    }

    #[test]
    fn nested_array_enum_class_and_primitive_existing_values_are_preserved() {
        let mut pool = Pool::default();
        let name = pool.utf8("RuntimeVisibleAnnotations").unwrap();
        let mut old = vec![0, 1, 0, 1, 0, 1, 0, 2, b'[', 0, 5];
        old.extend([b'@', 0, 3, 0, 1, 0, 4, b's', 0, 5]);
        old.extend([
            b'e', 0, 6, 0, 7, b'c', 0, 8, b'D', 0, 9, b'[', 0, 1, b'J', 0, 10,
        ]);
        let mut value = member(&[attribute(name, &old).unwrap()]);
        append_annotations(&mut pool, &mut value, &[marker("Lsample/Added;", true)]).unwrap();
        let merged = payloads(&value, name);
        assert_eq!(merged.len(), 1);
        assert_eq!(&merged[0][2..old.len()], &old[2..]);
        assert_eq!(&merged[0][..2], &[0, 2]);
    }

    #[test]
    fn parameter_merge_keeps_indices_and_nullability_for_unannotated_parameters() {
        let mut pool = Pool::default();
        let name = pool.utf8("RuntimeInvisibleParameterAnnotations").unwrap();
        let old = vec![3, 0, 1, 0, 2, 0, 0, 0, 0, 0, 1, 0, 3, 0, 0];
        let mut value = member(&[attribute(name, &old).unwrap()]);
        append_parameter_annotations(
            &mut pool,
            &mut value,
            &[
                vec![],
                vec![marker("Lsample/Middle;", false)],
                vec![marker("Lsample/Last;", true)],
            ],
        )
        .unwrap();
        let invisible = payloads(&value, name);
        assert_eq!(invisible.len(), 1);
        let lists = parameter_lists(&invisible[0]).unwrap();
        assert_eq!(
            lists.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            [1, 1, 1]
        );
        assert_eq!(lists[0].1, &[0, 2, 0, 0]);
        assert_eq!(lists[2].1, &[0, 3, 0, 0]);
        let visible = pool.utf8("RuntimeVisibleParameterAnnotations").unwrap();
        let visible = payloads(&value, visible);
        let lists = parameter_lists(&visible[0]).unwrap();
        assert_eq!(
            lists.iter().map(|entry| entry.0).collect::<Vec<_>>(),
            [0, 0, 1]
        );
    }

    #[test]
    fn parameter_merge_preserves_nested_annotations_and_array_values() {
        let mut pool = Pool::default();
        let name = pool.utf8("RuntimeVisibleParameterAnnotations").unwrap();
        let nested = [0, 2, 0, 1, 0, 3, b'[', 0, 2, b's', 0, 4, b'@', 0, 5, 0, 0];
        let mut old = vec![2, 0, 1];
        old.extend(nested);
        old.extend([0, 0]);
        let mut value = member(&[attribute(name, &old).unwrap()]);
        append_parameter_annotations(
            &mut pool,
            &mut value,
            &[vec![], vec![marker("Lsample/Second;", true)]],
        )
        .unwrap();
        let payloads = payloads(&value, name);
        assert_eq!(payloads.len(), 1);
        let lists = parameter_lists(&payloads[0]).unwrap();
        assert_eq!(lists[0], (1, nested.as_slice()));
        assert_eq!(lists[1].0, 1);
    }

    #[test]
    fn malformed_and_misaligned_existing_attributes_leave_member_unchanged() {
        let mut pool = Pool::default();
        let name = pool.utf8("RuntimeVisibleAnnotations").unwrap();
        for payload in [
            vec![0, 1, 0, 2],
            vec![0, 1, 0, 2, 0, 1, 0, 3, b'?'],
            vec![0, 0, 1],
        ] {
            let mut value = member(&[attribute(name, &payload).unwrap()]);
            let before = value.clone();
            assert!(
                append_annotations(&mut pool, &mut value, &[marker("Lsample/New;", true)]).is_err()
            );
            assert_eq!(value, before);
        }
        let name = pool.utf8("RuntimeVisibleParameterAnnotations").unwrap();
        let mut value = member(&[attribute(name, &[2, 0, 0, 0, 0]).unwrap()]);
        let before = value.clone();
        assert!(
            append_parameter_annotations(
                &mut pool,
                &mut value,
                &[vec![marker("Lsample/New;", true)]]
            )
            .is_err()
        );
        assert_eq!(value, before);
    }
}
