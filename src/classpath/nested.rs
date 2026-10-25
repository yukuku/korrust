use super::{Reader, invalid, modified_utf8_units};
use crate::Result;

pub(crate) fn attributes(
    bytes: &[u8],
    entries: &[(String, String, String, u16)],
) -> Result<Vec<u8>> {
    if entries.is_empty() {
        return Ok(bytes.to_vec());
    }
    let mut reader = Reader { bytes, offset: 8 };
    let mut count = reader.u2()?;
    let mut names = std::collections::HashMap::new();
    let mut classes = std::collections::HashMap::new();
    let mut index = 1;
    while index < count {
        let tag = reader.u1()?;
        match tag {
            1 => {
                let length = usize::from(reader.u2()?);
                names.insert(
                    index,
                    String::from_utf16_lossy(&modified_utf8_units(reader.take(length)?)?),
                );
            }
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => {
                reader.take(4)?;
            }
            5 | 6 => {
                reader.take(8)?;
                index += 1;
            }
            7 => {
                classes.insert(index, reader.u2()?);
            }
            8 | 16 | 19 | 20 => {
                reader.take(2)?;
            }
            15 => {
                reader.take(3)?;
            }
            _ => return Err(invalid()),
        }
        index += 1;
    }
    let pool_end = reader.offset;
    reader.take(6)?;
    let interfaces = usize::from(reader.u2()?);
    reader.take(interfaces * 2)?;
    for _ in 0..2 {
        let members = reader.u2()?;
        for _ in 0..members {
            reader.take(6)?;
            let attrs = reader.u2()?;
            for _ in 0..attrs {
                reader.take(2)?;
                let length = reader.u4()? as usize;
                reader.take(length)?;
            }
        }
    }
    let attr_start = reader.offset;
    let attrs = reader.u2()?;
    let mut kept = Vec::new();
    let mut existing = Vec::new();
    for _ in 0..attrs {
        let start = reader.offset;
        let name = reader.u2()?;
        let length = reader.u4()? as usize;
        let data = reader.take(length)?;
        if names.get(&name).is_some_and(|name| name == "InnerClasses") {
            let mut inner = Reader {
                bytes: data,
                offset: 0,
            };
            let rows = inner.u2()?;
            for _ in 0..rows {
                let row = inner.take(8)?;
                let index = u16::from_be_bytes([row[0], row[1]]);
                let replaced = classes
                    .get(&index)
                    .and_then(|id| names.get(id))
                    .is_some_and(|name| entries.iter().any(|(entry, _, _, _)| entry == name));
                if !replaced {
                    existing.push(row.to_vec());
                }
            }
        } else {
            kept.push(bytes[start..reader.offset].to_vec());
        }
    }
    if reader.offset != bytes.len() {
        return Err(invalid());
    }
    let mut pool = Vec::new();
    fn text(pool: &mut Vec<u8>, count: &mut u16, value: &str) -> Result<u16> {
        let id = *count;
        *count = count.checked_add(1).ok_or_else(invalid)?;
        let mut data = Vec::new();
        for value in value.encode_utf16() {
            match value {
                1..=127 => data.push(value as u8),
                0..=2047 => {
                    data.push(0xc0 | (value >> 6) as u8);
                    data.push(0x80 | (value & 63) as u8);
                }
                _ => {
                    data.push(0xe0 | (value >> 12) as u8);
                    data.push(0x80 | ((value >> 6) & 63) as u8);
                    data.push(0x80 | (value & 63) as u8);
                }
            }
        }
        pool.push(1);
        pool.extend(
            u16::try_from(data.len())
                .map_err(|_| invalid())?
                .to_be_bytes(),
        );
        pool.extend(data);
        Ok(id)
    }
    fn class(pool: &mut Vec<u8>, count: &mut u16, value: &str) -> Result<u16> {
        let name = text(pool, count, value)?;
        let id = *count;
        *count = count.checked_add(1).ok_or_else(invalid)?;
        pool.push(7);
        pool.extend(name.to_be_bytes());
        Ok(id)
    }
    let attr_name = text(&mut pool, &mut count, "InnerClasses")?;
    let mut rows = existing;
    for (name, outer, short, access) in entries {
        let mut row = Vec::new();
        row.extend(class(&mut pool, &mut count, name)?.to_be_bytes());
        row.extend(class(&mut pool, &mut count, outer)?.to_be_bytes());
        row.extend(text(&mut pool, &mut count, short)?.to_be_bytes());
        row.extend(access.to_be_bytes());
        rows.push(row);
    }
    let mut result = bytes[..8].to_vec();
    result.extend(count.to_be_bytes());
    result.extend(&bytes[10..pool_end]);
    result.extend(pool);
    result.extend(&bytes[pool_end..attr_start]);
    result.extend(
        u16::try_from(kept.len() + 1)
            .map_err(|_| invalid())?
            .to_be_bytes(),
    );
    for attr in kept {
        result.extend(attr);
    }
    result.extend(attr_name.to_be_bytes());
    result.extend(
        u32::try_from(2 + 8 * rows.len())
            .map_err(|_| invalid())?
            .to_be_bytes(),
    );
    result.extend(
        u16::try_from(rows.len())
            .map_err(|_| invalid())?
            .to_be_bytes(),
    );
    for row in rows {
        result.extend(row);
    }
    Ok(result)
}
