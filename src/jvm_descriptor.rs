use crate::{Diagnostic, Result, syntax::Type};

pub(crate) fn method_types(descriptor: &str) -> Result<(Vec<Type>, Type)> {
    fn ty(bytes: &[u8], cursor: &mut usize, dimensions: usize) -> Result<Type> {
        let start = *cursor;
        let tag = *bytes.get(start).ok_or_else(invalid)?;
        *cursor += 1;
        Ok(match tag {
            b'I' => Type::Int,
            b'J' => Type::Long,
            b'F' => Type::Float,
            b'D' => Type::Double,
            b'B' => Type::Byte,
            b'C' => Type::Char,
            b'Z' => Type::Boolean,
            b'V' => Type::Unit,
            b'L' => {
                let end = bytes[*cursor..]
                    .iter()
                    .position(|byte| *byte == b';')
                    .ok_or_else(invalid)?
                    + *cursor;
                let name = std::str::from_utf8(&bytes[*cursor..end]).map_err(|_| invalid())?;
                if name.split('/').any(|part| {
                    part.is_empty() || part.bytes().any(|byte| matches!(byte, b'.' | b'['))
                }) {
                    return Err(invalid());
                }
                *cursor = end + 1;
                if name == "java/lang/String" {
                    Type::String
                } else {
                    Type::Java(name.into())
                }
            }
            b'[' => {
                if dimensions == 255 {
                    return Err(invalid());
                }
                let element = ty(bytes, cursor, dimensions + 1)?;
                match &bytes[start..*cursor] {
                    b"[I" => Type::IntArray,
                    b"[B" => Type::ByteArray,
                    b"[C" => Type::CharArray,
                    b"[J" => Type::LongArray,
                    b"[Z" => Type::BooleanArray,
                    b"[F" => Type::FloatArray,
                    b"[D" => Type::DoubleArray,
                    b"[Ljava/lang/String;" => Type::StringArray,
                    _ if element.is_reference() => Type::Array(Box::new(element)),
                    _ => return Err(invalid()),
                }
            }
            b'S' => Type::Short,
            _ => return Err(invalid()),
        })
    }

    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return Err(invalid());
    }
    let mut cursor = 1;
    let mut parameters = Vec::new();
    let mut slots = 0;
    while bytes.get(cursor) != Some(&b')') {
        let parameter = ty(bytes, &mut cursor, 0)?;
        slots += parameter.slots();
        if parameter == Type::Unit || slots > 255 {
            return Err(invalid());
        }
        parameters.push(parameter);
    }
    cursor += 1;
    let result = ty(bytes, &mut cursor, 0)?;
    if cursor != bytes.len() {
        return Err(invalid());
    }
    Ok((parameters, result))
}

fn invalid() -> Diagnostic {
    Diagnostic::new(0, "Invalid JVM method descriptor")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_parameters_preserve_width_boxing_and_nested_arrays() {
        let (parameters, result) =
            method_types("(JDLjava/lang/Integer;[[I[[Ljava/lang/String;)[Ljava/lang/Object;")
                .unwrap();
        assert_eq!(
            parameters,
            vec![
                Type::Long,
                Type::Double,
                Type::Java("java/lang/Integer".into()),
                Type::Array(Box::new(Type::IntArray)),
                Type::Array(Box::new(Type::StringArray)),
            ]
        );
        assert_eq!(parameters.iter().map(Type::slots).sum::<usize>(), 7);
        assert_eq!(
            result,
            Type::Array(Box::new(Type::Java("java/lang/Object".into())))
        );
    }

    #[test]
    fn malformed_descriptors_and_invalid_void_positions_are_rejected() {
        for descriptor in [
            "",
            "I",
            "(",
            "(I",
            "()",
            "(V)V",
            "([V)V",
            "()[V",
            "()VV",
            "(L;)V",
            "(Lfoo//Bar;)V",
            "(Lfoo.Bar;)V",
            "(Ljava/lang/String)V",
            "(Q)V",
        ] {
            assert!(method_types(descriptor).is_err(), "{descriptor}");
        }
        assert!(method_types("()V").is_ok());
        assert_eq!(
            method_types("(S)S").unwrap(),
            (vec![Type::Short], Type::Short)
        );
        assert!(method_types("()[S").is_err());
    }

    #[test]
    fn array_dimensions_and_parameter_slots_have_jvm_bounds() {
        assert!(method_types(&format!("({}I)V", "[".repeat(255))).is_ok());
        assert!(method_types(&format!("({}I)V", "[".repeat(256))).is_err());
        assert!(method_types(&format!("({})V", "J".repeat(127))).is_ok());
        assert!(method_types(&format!("({})V", "J".repeat(128))).is_err());
    }
}
