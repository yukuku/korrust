use super::*;
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Debug, PartialEq, Eq)]
enum Verification {
    Top,
    Int,
    Float,
    Long,
    Null,
    Object(String),
    New(usize, String),
    UninitializedThis(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Frame {
    locals: Vec<Verification>,
    stack: Vec<Verification>,
}
fn utf8(pool: &Pool, index: u16) -> Result<String> {
    let Some(Constant::Utf8(bytes)) = pool.entries.get(index as usize - 1) else {
        return Err(failure("Invalid frame constant"));
    };
    String::from_utf8(bytes.clone()).map_err(|_| failure("Unsupported frame constant encoding"))
}
fn class_name(pool: &Pool, index: u16) -> Result<String> {
    let Some(Constant::Class(name)) = pool.entries.get(index as usize - 1) else {
        return Err(failure("Invalid frame class constant"));
    };
    utf8(pool, *name)
}
fn member(pool: &Pool, index: u16) -> Result<(String, String, String)> {
    let (owner, pair) = match pool.entries.get(index as usize - 1) {
        Some(
            Constant::Field(owner, pair)
            | Constant::Method(owner, pair)
            | Constant::InterfaceMethod(owner, pair),
        ) => (*owner, *pair),
        _ => return Err(failure("Invalid frame member constant")),
    };
    let Some(Constant::NameType(name, desc)) = pool.entries.get(pair as usize - 1) else {
        return Err(failure("Invalid frame member descriptor"));
    };
    Ok((
        class_name(pool, owner)?,
        utf8(pool, *name)?,
        utf8(pool, *desc)?,
    ))
}
fn descriptor_type(bytes: &[u8], cursor: &mut usize) -> Result<Option<Verification>> {
    let start = *cursor;
    let tag = *bytes
        .get(*cursor)
        .ok_or_else(|| failure("Invalid frame descriptor"))?;
    *cursor += 1;
    Ok(Some(match tag {
        b'V' => return Ok(None),
        b'J' => Verification::Long,
        b'F' => Verification::Float,
        b'B' | b'C' | b'I' | b'S' | b'Z' => Verification::Int,
        b'L' => {
            let end = bytes[*cursor..]
                .iter()
                .position(|b| *b == b';')
                .ok_or_else(|| failure("Invalid frame descriptor"))?
                + *cursor;
            let name = String::from_utf8(bytes[*cursor..end].to_vec())
                .map_err(|_| failure("Invalid frame descriptor"))?;
            *cursor = end + 1;
            Verification::Object(name)
        }
        b'[' => {
            descriptor_type(bytes, cursor)?;
            Verification::Object(
                String::from_utf8(bytes[start..*cursor].to_vec())
                    .map_err(|_| failure("Invalid frame descriptor"))?,
            )
        }
        _ => return Err(failure("Unsupported frame descriptor")),
    }))
}
fn pop(frame: &mut Frame) -> Result<Verification> {
    let value = frame
        .stack
        .pop()
        .ok_or_else(|| failure("Operand stack underflow during frame generation"))?;
    if value == Verification::Top {
        return Err(failure(
            "Incompatible operand types during frame generation",
        ));
    }
    Ok(value)
}
fn merge_type(a: &Verification, b: &Verification) -> Verification {
    if a == b {
        return a.clone();
    }
    match (a, b) {
        (Verification::Null, Verification::Object(_)) => b.clone(),
        (Verification::Object(_), Verification::Null) => a.clone(),
        (Verification::Object(_), Verification::Object(_)) => {
            Verification::Object("java/lang/Object".into())
        }
        _ => Verification::Top,
    }
}
fn merge(
    states: &mut BTreeMap<usize, Frame>,
    offset: usize,
    frame: Frame,
    queue: &mut VecDeque<usize>,
) -> Result<()> {
    if let Some(existing) = states.get_mut(&offset) {
        if existing.stack.len() != frame.stack.len() {
            return Err(failure(
                "Incompatible stack heights during frame generation",
            ));
        }
        let old = existing.clone();
        for (a, b) in existing.locals.iter_mut().zip(&frame.locals) {
            *a = merge_type(a, b);
        }
        for (a, b) in existing.stack.iter_mut().zip(&frame.stack) {
            *a = merge_type(a, b);
            if *a == Verification::Top {
                return Err(failure(
                    "Incompatible operand types during frame generation",
                ));
            }
        }
        if *existing != old {
            queue.push_back(offset);
        }
    } else {
        states.insert(offset, frame);
        queue.push_back(offset);
    }
    Ok(())
}
fn instruction_len(op: u8) -> Result<usize> {
    Ok(match op {
        0x10 | 0x15..=0x19 | 0x36..=0x3a | 0xbc => 2,
        0x11
        | 0x13
        | 0x14
        | 0x84
        | 0x99..=0xa7
        | 0xb2..=0xb8
        | 0xbb
        | 0xbd
        | 0xc0
        | 0xc1
        | 0xc6
        | 0xc7 => 3,
        0xb9 => 5,
        0x00..=0x0f
        | 0x1a..=0x35
        | 0x3b..=0x83
        | 0x85
        | 0x91..=0x94
        | 0xac..=0xb1
        | 0xbe
        | 0xbf => 1,
        _ => return Err(failure("Unsupported instruction during frame generation")),
    })
}
fn transfer(
    pool: &Pool,
    bytes: &[u8],
    offset: usize,
    mut frame: Frame,
) -> Result<(Frame, Vec<usize>)> {
    let op = bytes[offset];
    let len = instruction_len(op)?;
    let next = offset + len;
    let mut successors = vec![next];
    let index = || u16::from_be_bytes([bytes[offset + 1], bytes[offset + 2]]);
    match op {
        0x00 => {}
        0x01 => frame.stack.push(Verification::Null),
        0x02..=0x08 | 0x10 | 0x11 => frame.stack.push(Verification::Int),
        0x09..=0x0a => frame.stack.push(Verification::Long),
        0x0b..=0x0d => frame.stack.push(Verification::Float),
        0x13 | 0x14 => frame
            .stack
            .push(match pool.entries.get(index() as usize - 1) {
                Some(Constant::Long(_)) => Verification::Long,
                Some(Constant::Integer(_)) => Verification::Int,
                Some(Constant::Float(_)) => Verification::Float,
                Some(Constant::String(_)) => Verification::Object("java/lang/String".into()),
                _ => return Err(failure("Unsupported constant during frame generation")),
            }),
        0x15..=0x19 => frame
            .stack
            .push(frame.locals[bytes[offset + 1] as usize].clone()),
        0x1a..=0x2d => {
            let slot = ((op - 0x1a) % 4) as usize;
            frame.stack.push(frame.locals[slot].clone());
        }
        0x2e..=0x35 => {
            pop(&mut frame)?;
            let array = pop(&mut frame)?;
            frame.stack.push(match op {
                0x2f => Verification::Long,
                0x30 => Verification::Float,
                0x32 => match array {
                    Verification::Object(name) if name.starts_with("[L") => {
                        Verification::Object(name[2..name.len() - 1].into())
                    }
                    _ => Verification::Object("java/lang/Object".into()),
                },
                _ => Verification::Int,
            });
        }
        0x36..=0x4e => {
            let slot = if op <= 0x3a {
                bytes[offset + 1] as usize
            } else {
                ((op - 0x3b) % 4) as usize
            };
            let value = pop(&mut frame)?;
            let wide = value == Verification::Long;
            frame.locals[slot] = value;
            if wide {
                frame.locals[slot + 1] = Verification::Top;
            }
        }
        0x4f..=0x56 => {
            pop(&mut frame)?;
            pop(&mut frame)?;
            pop(&mut frame)?;
        }
        0x57 => {
            pop(&mut frame)?;
        }
        0x58 => {
            if pop(&mut frame)? != Verification::Long {
                pop(&mut frame)?;
            }
        }
        0x59 => {
            let value = pop(&mut frame)?;
            frame.stack.extend([value.clone(), value]);
        }
        0x60..=0x73 | 0x78..=0x83 => {
            pop(&mut frame)?;
            let value = pop(&mut frame)?;
            frame.stack.push(value);
        }
        0x74..=0x77 | 0x84 | 0x91..=0x93 => {}
        0x85 => {
            pop(&mut frame)?;
            frame.stack.push(Verification::Long);
        }
        0x94 => {
            pop(&mut frame)?;
            pop(&mut frame)?;
            frame.stack.push(Verification::Int);
        }
        0x99..=0xa7 | 0xc6 | 0xc7 => {
            if op != 0xa7 {
                pop(&mut frame)?;
            }
            if (0x9f..=0xa6).contains(&op) {
                pop(&mut frame)?;
            }
            let target = (offset as isize
                + i16::from_be_bytes([bytes[offset + 1], bytes[offset + 2]]) as isize)
                as usize;
            if op == 0xa7 {
                successors.clear();
            }
            successors.push(target);
        }
        0xac..=0xb1 | 0xbf => {
            if op != 0xb1 {
                pop(&mut frame)?;
            }
            successors.clear();
        }
        0xb2..=0xb5 => {
            let (_, _, desc) = member(pool, index())?;
            let ty = descriptor_type(desc.as_bytes(), &mut 0)?
                .ok_or_else(|| failure("Invalid field type"))?;
            if op == 0xb3 || op == 0xb5 {
                pop(&mut frame)?;
            }
            if op == 0xb4 || op == 0xb5 {
                pop(&mut frame)?;
            }
            if op == 0xb2 || op == 0xb4 {
                frame.stack.push(ty);
            }
        }
        0xb6..=0xb9 => {
            let (owner, name, desc) = member(pool, index())?;
            let bytes = desc.as_bytes();
            let mut cursor = 1;
            while bytes.get(cursor) != Some(&b')') {
                descriptor_type(bytes, &mut cursor)?;
                pop(&mut frame)?;
            }
            if op != 0xb8 {
                let receiver = pop(&mut frame)?;
                if name == "<init>" {
                    let initialized = Verification::Object(match &receiver {
                        Verification::UninitializedThis(current) => current.clone(),
                        _ => owner,
                    });
                    for value in frame.locals.iter_mut().chain(frame.stack.iter_mut()) {
                        if *value == receiver {
                            *value = initialized.clone();
                        }
                    }
                }
            }
            cursor += 1;
            if let Some(result) = descriptor_type(bytes, &mut cursor)? {
                frame.stack.push(result);
            }
        }
        0xbb => frame
            .stack
            .push(Verification::New(offset, class_name(pool, index())?)),
        0xbc => {
            pop(&mut frame)?;
            frame.stack.push(Verification::Object(
                match bytes[offset + 1] {
                    8 => "[B",
                    5 => "[C",
                    10 => "[I",
                    _ => return Err(failure("Unsupported frame array type")),
                }
                .into(),
            ));
        }
        0xbd => {
            pop(&mut frame)?;
            frame.stack.push(Verification::Object(format!(
                "[L{};",
                class_name(pool, index())?
            )));
        }
        0xbe | 0xc1 => {
            pop(&mut frame)?;
            frame.stack.push(Verification::Int);
        }
        0xc0 => {
            pop(&mut frame)?;
            frame
                .stack
                .push(Verification::Object(class_name(pool, index())?));
        }
        _ => return Err(failure("Unsupported instruction during frame generation")),
    }
    Ok((frame, successors))
}
fn write_type(pool: &mut Pool, bytes: &mut Vec<u8>, ty: &Verification) -> Result<()> {
    match ty {
        Verification::Top => bytes.push(0),
        Verification::Int => bytes.push(1),
        Verification::Float => bytes.push(2),
        Verification::Long => bytes.push(4),
        Verification::Null => bytes.push(5),
        Verification::UninitializedThis(_) => bytes.push(6),
        Verification::Object(name) => {
            bytes.push(7);
            u2(bytes, pool.class(name)?);
        }
        Verification::New(offset, _) => {
            bytes.push(8);
            u2(bytes, *offset as u16);
        }
    }
    Ok(())
}
pub(super) fn attach(
    pool: &mut Pool,
    method: &mut Vec<u8>,
    owner: &str,
    params: &[(String, Type)],
    locals: u16,
    is_static: bool,
) -> Result<()> {
    attach_with_constructor(pool, method, owner, params, locals, is_static, false)
}
pub(super) fn attach_constructor(
    pool: &mut Pool,
    method: &mut Vec<u8>,
    owner: &str,
    params: &[(String, Type)],
    locals: u16,
) -> Result<()> {
    attach_with_constructor(pool, method, owner, params, locals, false, true)
}
fn attach_with_constructor(
    pool: &mut Pool,
    method: &mut Vec<u8>,
    owner: &str,
    params: &[(String, Type)],
    locals: u16,
    is_static: bool,
    constructor: bool,
) -> Result<()> {
    let length = u32::from_be_bytes(method[18..22].try_into().unwrap()) as usize;
    let bytes = &method[22..22 + length];
    let mut initial = Frame {
        locals: vec![Verification::Top; locals as usize],
        stack: Vec::new(),
    };
    if !is_static {
        initial.locals[0] = if constructor {
            Verification::UninitializedThis(owner.into())
        } else {
            Verification::Object(owner.into())
        };
    }
    for (slot, (_, ty)) in parameter_slots(params, usize::from(!is_static))
        .into_iter()
        .zip(params)
    {
        initial.locals[slot] = descriptor_type(ty.descriptor().as_bytes(), &mut 0)?.unwrap();
    }
    let mut offsets = Vec::new();
    let mut offset = 0;
    while offset < length {
        offsets.push(offset);
        offset += instruction_len(bytes[offset])?;
    }
    if offset != length {
        return Err(failure(
            "Invalid instruction boundary during frame generation",
        ));
    }
    let mut states = BTreeMap::new();
    let mut queue = VecDeque::new();
    merge(&mut states, 0, initial.clone(), &mut queue)?;
    loop {
        while let Some(offset) = queue.pop_front() {
            let (frame, successors) = transfer(pool, bytes, offset, states[&offset].clone())?;
            for next in successors {
                if next >= length || offsets.binary_search(&next).is_err() {
                    return Err(failure(
                        "Invalid control-flow target during frame generation",
                    ));
                }
                merge(&mut states, next, frame.clone(), &mut queue)?;
            }
        }
        let Some(dead) = offsets
            .iter()
            .find(|offset| !states.contains_key(offset))
            .copied()
        else {
            break;
        };
        let mut frame = states
            .range(..dead)
            .next_back()
            .map(|(_, frame)| frame.clone())
            .unwrap_or_else(|| initial.clone());
        frame.stack.clear();
        merge(&mut states, dead, frame, &mut queue)?;
    }
    let mut table = Vec::new();
    u2(&mut table, (states.len() - 1) as u16);
    let mut previous: isize = -1;
    for (offset, frame) in states.iter().skip(1) {
        table.push(255);
        u2(&mut table, (*offset as isize - previous - 1) as u16);
        previous = *offset as isize;
        let mut locals = frame.locals.clone();
        while locals.last() == Some(&Verification::Top) {
            locals.pop();
        }
        let mut values = Vec::new();
        let mut slot = 0;
        while slot < locals.len() {
            values.push(locals[slot].clone());
            slot += if locals[slot] == Verification::Long {
                2
            } else {
                1
            };
        }
        u2(&mut table, values.len() as u16);
        for ty in &values {
            write_type(pool, &mut table, ty)?;
        }
        u2(&mut table, frame.stack.len() as u16);
        for ty in &frame.stack {
            write_type(pool, &mut table, ty)?;
        }
    }
    let mut attribute = Vec::new();
    u2(&mut attribute, pool.utf8("StackMapTable")?);
    u4(&mut attribute, table.len() as u32);
    attribute.extend(table);
    let nested = 24 + length;
    method[nested..nested + 2].copy_from_slice(&1u16.to_be_bytes());
    let old_length = u32::from_be_bytes(method[10..14].try_into().unwrap());
    method[10..14].copy_from_slice(&(old_length + attribute.len() as u32).to_be_bytes());
    method.splice(nested + 2..nested + 2, attribute);
    Ok(())
}
