use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::classpath::{InlineConstant, JavaCode, JavaMethod};
use crate::{Diagnostic, Result};

#[derive(Clone, Debug)]
pub(crate) struct InlineBody {
    pub instructions: Vec<InlineInstruction>,
    pub max_stack: u16,
    pub max_locals: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct InlineInstruction {
    pub offset: usize,
    pub kind: InlineOp,
}

#[derive(Clone, Debug)]
pub(crate) enum InlineOp {
    Raw {
        opcode: u8,
        operands: Vec<u8>,
    },
    Local {
        opcode: u8,
        slot: u16,
    },
    Increment {
        slot: u16,
        amount: i16,
    },
    Constant(InlineConstant),
    Reference {
        opcode: u8,
        constant: InlineConstant,
        interface_count: u8,
    },
    Branch {
        opcode: u8,
        target: usize,
    },
    Switch {
        default: usize,
        cases: Vec<(i32, usize)>,
    },
    Return(u8),
}

fn unsupported(message: impl AsRef<str>) -> Diagnostic {
    Diagnostic::new(
        0,
        format!("Unsupported compiled inline body: {}", message.as_ref()),
    )
    .with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "COMPILED_INLINE_BODY_UNSUPPORTED",
    )
}

pub(crate) fn validate(method: &JavaMethod) -> Result<()> {
    if method.access & 8 == 0 {
        return Err(unsupported("only static inline functions are supported"));
    }
    let library = method
        .library
        .as_ref()
        .ok_or_else(|| unsupported("missing Kotlin metadata"))?;
    if !library.inline {
        return Err(unsupported("declaration is not inline"));
    }
    let code = library
        .code
        .as_ref()
        .ok_or_else(|| unsupported("missing Code attribute"))?;
    let body = decode(code)?;
    let nested_owner = format!("{}$", method.owner);
    if body.instructions.iter().any(|instruction| {
        matches!(
            &instruction.kind,
            InlineOp::Reference { opcode: 0xbb, constant: InlineConstant::Class(owner), .. }
                if owner.starts_with(&nested_owner)
        )
    }) {
        return Err(unsupported(
            "generated or nested inline-owner object allocation requires class regeneration",
        ));
    }
    Ok(())
}

pub(crate) fn decode(code: &JavaCode) -> Result<InlineBody> {
    if code.exception_handlers != 0 {
        return Err(unsupported(
            "exception handlers and finally regions are not supported",
        ));
    }
    if code.bytes.is_empty() {
        return Err(unsupported("empty bytecode"));
    }
    let mut instructions = Vec::new();
    let mut cursor = 0;
    while cursor < code.bytes.len() {
        let offset = cursor;
        let opcode = byte(&code.bytes, &mut cursor)?;
        let kind = match opcode {
            0x15..=0x19 | 0x36..=0x3a => InlineOp::Local {
                opcode,
                slot: u16::from(byte(&code.bytes, &mut cursor)?),
            },
            0x1a..=0x2d => InlineOp::Local {
                opcode: 0x15 + (opcode - 0x1a) / 4,
                slot: u16::from((opcode - 0x1a) % 4),
            },
            0x3b..=0x4e => InlineOp::Local {
                opcode: 0x36 + (opcode - 0x3b) / 4,
                slot: u16::from((opcode - 0x3b) % 4),
            },
            0x84 => InlineOp::Increment {
                slot: u16::from(byte(&code.bytes, &mut cursor)?),
                amount: i16::from(byte(&code.bytes, &mut cursor)? as i8),
            },
            0xc4 => {
                let opcode = byte(&code.bytes, &mut cursor)?;
                let slot = word(&code.bytes, &mut cursor)?;
                match opcode {
                    0x15..=0x19 | 0x36..=0x3a => InlineOp::Local { opcode, slot },
                    0x84 => InlineOp::Increment {
                        slot,
                        amount: word(&code.bytes, &mut cursor)? as i16,
                    },
                    _ => return Err(unsupported("wide instruction")),
                }
            }
            0x12..=0x14 => {
                let index = if opcode == 0x12 {
                    u16::from(byte(&code.bytes, &mut cursor)?)
                } else {
                    word(&code.bytes, &mut cursor)?
                };
                let constant = constant(code, index)?;
                match &constant {
                    InlineConstant::Integer(_)
                    | InlineConstant::Float(_)
                    | InlineConstant::String(_)
                    | InlineConstant::Class(_)
                        if opcode != 0x14 => {}
                    InlineConstant::Long(_) | InlineConstant::Double(_) if opcode == 0x14 => {}
                    _ => return Err(unsupported("ldc constant kind")),
                }
                InlineOp::Constant(constant)
            }
            0x99..=0xa7 | 0xc6 | 0xc7 => {
                let displacement = word(&code.bytes, &mut cursor)? as i16;
                let target = offset
                    .checked_add_signed(isize::from(displacement))
                    .ok_or_else(|| unsupported("branch target outside body"))?;
                InlineOp::Branch { opcode, target }
            }
            0xaa | 0xab => {
                while cursor % 4 != 0 {
                    if byte(&code.bytes, &mut cursor)? != 0 {
                        return Err(unsupported("nonzero switch padding"));
                    }
                }
                let default = branch_target(offset, signed_word(&code.bytes, &mut cursor)?)?;
                let mut cases = Vec::new();
                if opcode == 0xaa {
                    let low = signed_word(&code.bytes, &mut cursor)?;
                    let high = signed_word(&code.bytes, &mut cursor)?;
                    let count = i64::from(high) - i64::from(low) + 1;
                    if count <= 0 || count as u64 > ((code.bytes.len() - cursor) / 4) as u64 {
                        return Err(unsupported("invalid tableswitch range"));
                    }
                    for key in low..=high {
                        let target = branch_target(offset, signed_word(&code.bytes, &mut cursor)?)?;
                        cases.push((key, target));
                    }
                } else {
                    let count = signed_word(&code.bytes, &mut cursor)?;
                    if count < 0 || count as usize > (code.bytes.len() - cursor) / 8 {
                        return Err(unsupported("invalid lookupswitch count"));
                    }
                    for _ in 0..count {
                        let key = signed_word(&code.bytes, &mut cursor)?;
                        if cases.last().is_some_and(|(previous, _)| *previous >= key) {
                            return Err(unsupported("unordered lookupswitch keys"));
                        }
                        let target = branch_target(offset, signed_word(&code.bytes, &mut cursor)?)?;
                        cases.push((key, target));
                    }
                }
                InlineOp::Switch { default, cases }
            }
            0xc8 => InlineOp::Branch {
                opcode: 0xa7,
                target: branch_target(offset, signed_word(&code.bytes, &mut cursor)?)?,
            },
            0xac..=0xb1 => InlineOp::Return(opcode),
            0xb2..=0xb9 | 0xbb | 0xbd | 0xc0 | 0xc1 => {
                let constant = constant(code, word(&code.bytes, &mut cursor)?)?;
                let valid = match &constant {
                    InlineConstant::Field { .. } => (0xb2..=0xb5).contains(&opcode),
                    InlineConstant::Method { interface, .. } => {
                        (0xb6..=0xb9).contains(&opcode) && (*interface || opcode != 0xb9)
                    }
                    InlineConstant::Class(_) => matches!(opcode, 0xbb | 0xbd | 0xc0 | 0xc1),
                    _ => false,
                };
                if !valid {
                    return Err(unsupported("constant pool reference kind"));
                }
                let interface_count = if opcode == 0xb9 {
                    let count = byte(&code.bytes, &mut cursor)?;
                    if count == 0 || byte(&code.bytes, &mut cursor)? != 0 {
                        return Err(unsupported("malformed invokeinterface operands"));
                    }
                    count
                } else {
                    0
                };
                InlineOp::Reference {
                    opcode,
                    constant,
                    interface_count,
                }
            }
            0x10 | 0xbc => {
                let operand = byte(&code.bytes, &mut cursor)?;
                if opcode == 0xbc && !(4..=11).contains(&operand) {
                    return Err(unsupported("primitive array kind"));
                }
                InlineOp::Raw {
                    opcode,
                    operands: vec![operand],
                }
            }
            0x11 => {
                let operand = word(&code.bytes, &mut cursor)?;
                InlineOp::Raw {
                    opcode,
                    operands: operand.to_be_bytes().to_vec(),
                }
            }
            0x00..=0x0f | 0x2e..=0x35 | 0x4f..=0x83 | 0x85..=0x98 | 0xbe | 0xbf => InlineOp::Raw {
                opcode,
                operands: Vec::new(),
            },
            _ => {
                return Err(unsupported(format!(
                    "JVM opcode 0x{opcode:02x} at byte {offset}"
                )));
            }
        };
        let local = match &kind {
            InlineOp::Local { opcode, slot } => Some((
                *slot,
                if matches!(opcode, 0x16 | 0x18 | 0x37 | 0x39) {
                    2
                } else {
                    1
                },
            )),
            InlineOp::Increment { slot, .. } => Some((*slot, 1)),
            _ => None,
        };
        if local.is_some_and(|(slot, width)| {
            slot.checked_add(width)
                .is_none_or(|end| end > code.max_locals)
        }) {
            return Err(unsupported("local index outside max_locals"));
        }
        instructions.push(InlineInstruction { offset, kind });
    }
    let boundaries: BTreeSet<_> = instructions
        .iter()
        .map(|instruction| instruction.offset)
        .collect();
    for instruction in &instructions {
        let targets = match &instruction.kind {
            InlineOp::Branch { target, .. } => vec![*target],
            InlineOp::Switch { default, cases } => std::iter::once(*default)
                .chain(cases.iter().map(|(_, target)| *target))
                .collect(),
            _ => Vec::new(),
        };
        if targets.iter().any(|target| !boundaries.contains(target)) {
            return Err(unsupported(
                "branch does not target an instruction boundary",
            ));
        }
    }
    Ok(InlineBody {
        instructions,
        max_stack: code.max_stack,
        max_locals: code.max_locals,
    })
}

fn byte(bytes: &[u8], cursor: &mut usize) -> Result<u8> {
    let value = *bytes
        .get(*cursor)
        .ok_or_else(|| unsupported("truncated instruction"))?;
    *cursor += 1;
    Ok(value)
}

fn word(bytes: &[u8], cursor: &mut usize) -> Result<u16> {
    Ok(u16::from_be_bytes([
        byte(bytes, cursor)?,
        byte(bytes, cursor)?,
    ]))
}

fn signed_word(bytes: &[u8], cursor: &mut usize) -> Result<i32> {
    Ok(i32::from_be_bytes([
        byte(bytes, cursor)?,
        byte(bytes, cursor)?,
        byte(bytes, cursor)?,
        byte(bytes, cursor)?,
    ]))
}

fn branch_target(offset: usize, displacement: i32) -> Result<usize> {
    offset
        .checked_add_signed(displacement as isize)
        .ok_or_else(|| unsupported("branch target outside body"))
}

fn constant(code: &JavaCode, index: u16) -> Result<InlineConstant> {
    code.constants
        .get(&index)
        .cloned()
        .ok_or_else(|| unsupported("unavailable constant pool entry"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operand {
    Value(u8),
    Lambda(u16),
}

fn pop_operand(stack: &mut Vec<Operand>, allow_lambda: bool) -> Result<Operand> {
    let operand = stack
        .pop()
        .ok_or_else(|| unsupported("operand stack underflow"))?;
    if !allow_lambda && matches!(operand, Operand::Lambda(_)) {
        return Err(unsupported(
            "inline lambda escapes a directly substituted invocation",
        ));
    }
    Ok(operand)
}

fn stack_operation(stack: &mut Vec<Operand>, opcode: u8) -> Result<()> {
    fn pop(stack: &mut Vec<Operand>, width: u8) -> Result<Operand> {
        let operand = pop_operand(stack, false)?;
        if operand != Operand::Value(width) {
            return Err(unsupported("invalid category for stack instruction"));
        }
        Ok(operand)
    }
    fn pair(stack: &mut Vec<Operand>) -> Result<Vec<Operand>> {
        let first = pop_operand(stack, false)?;
        if first == Operand::Value(2) {
            Ok(vec![first])
        } else {
            let second = pop(stack, 1)?;
            Ok(vec![second, first])
        }
    }
    match opcode {
        0x57 => {
            pop(stack, 1)?;
        }
        0x58 => {
            pair(stack)?;
        }
        0x59 => {
            let value = pop(stack, 1)?;
            stack.extend([value, value]);
        }
        0x5a => {
            let first = pop(stack, 1)?;
            let second = pop(stack, 1)?;
            stack.extend([first, second, first]);
        }
        0x5b => {
            let first = pop(stack, 1)?;
            let below = pair(stack)?;
            stack.push(first);
            stack.extend(below);
            stack.push(first);
        }
        0x5c => {
            let values = pair(stack)?;
            stack.extend_from_slice(&values);
            stack.extend(values);
        }
        0x5d => {
            let values = pair(stack)?;
            let below = pop(stack, 1)?;
            stack.extend_from_slice(&values);
            stack.push(below);
            stack.extend(values);
        }
        0x5e => {
            let values = pair(stack)?;
            let below = pair(stack)?;
            stack.extend_from_slice(&values);
            stack.extend(below);
            stack.extend(values);
        }
        0x5f => {
            let first = pop(stack, 1)?;
            let second = pop(stack, 1)?;
            stack.extend([first, second]);
        }
        _ => unreachable!(),
    }
    Ok(())
}

pub(crate) fn lambda_calls(
    body: &InlineBody,
    lambda_slots: &BTreeSet<u16>,
) -> Result<BTreeMap<usize, u16>> {
    let indices: BTreeMap<_, _> = body
        .instructions
        .iter()
        .enumerate()
        .map(|(index, instruction)| (instruction.offset, index))
        .collect();
    let mut calls = BTreeMap::new();
    let mut states = BTreeMap::from([(0, Vec::<Operand>::new())]);
    let mut queue = VecDeque::from([0]);
    while let Some(index) = queue.pop_front() {
        let instruction = &body.instructions[index];
        let mut stack = states[&index].clone();
        let mut successors = Vec::new();
        match &instruction.kind {
            InlineOp::Local { opcode, slot } if *opcode < 0x36 => {
                stack.push(if lambda_slots.contains(slot) {
                    if *opcode != 0x19 {
                        return Err(unsupported("inline lambda loaded as a primitive"));
                    }
                    Operand::Lambda(*slot)
                } else {
                    Operand::Value(if matches!(*opcode, 0x16 | 0x18) { 2 } else { 1 })
                });
            }
            InlineOp::Local { slot, .. } => {
                if lambda_slots.contains(slot) {
                    return Err(unsupported("inline lambda parameter is overwritten"));
                }
                pop_operand(&mut stack, false)?;
            }
            InlineOp::Increment { slot, .. } => {
                if lambda_slots.contains(slot) {
                    return Err(unsupported("inline lambda parameter is overwritten"));
                }
            }
            InlineOp::Constant(value) => stack.push(Operand::Value(
                if matches!(value, InlineConstant::Long(_) | InlineConstant::Double(_)) {
                    2
                } else {
                    1
                },
            )),
            InlineOp::Reference {
                opcode, constant, ..
            } => match constant {
                InlineConstant::Class(_) => {
                    if *opcode != 0xbb {
                        pop_operand(&mut stack, false)?;
                    }
                    stack.push(Operand::Value(1));
                }
                InlineConstant::Field { descriptor, .. } => {
                    if *opcode == 0xb3 || *opcode == 0xb5 {
                        pop_operand(&mut stack, false)?;
                    }
                    if *opcode == 0xb4 || *opcode == 0xb5 {
                        pop_operand(&mut stack, false)?;
                    }
                    if *opcode == 0xb2 || *opcode == 0xb4 {
                        stack.push(Operand::Value(
                            descriptor_width(descriptor.as_bytes(), &mut 0, false)?
                                .ok_or_else(|| unsupported("void field"))?,
                        ));
                    }
                }
                InlineConstant::Method {
                    owner,
                    name,
                    descriptor,
                    ..
                } => {
                    let (params, result) = descriptor_widths(descriptor)?;
                    let null_check = *opcode == 0xb8
                        && owner == "kotlin/jvm/internal/Intrinsics"
                        && name == "checkNotNullParameter"
                        && descriptor == "(Ljava/lang/Object;Ljava/lang/String;)V";
                    for position in (0..params.len()).rev() {
                        pop_operand(&mut stack, null_check && position == 0)?;
                    }
                    if *opcode != 0xb8 {
                        if let Operand::Lambda(slot) = pop_operand(&mut stack, true)? {
                            let expected_owner =
                                format!("kotlin/jvm/functions/Function{}", params.len());
                            if *opcode != 0xb9
                                || owner != &expected_owner
                                || name != "invoke"
                                || descriptor
                                    != &format!(
                                        "({})Ljava/lang/Object;",
                                        "Ljava/lang/Object;".repeat(params.len())
                                    )
                            {
                                return Err(unsupported(
                                    "inline lambda used outside FunctionN.invoke",
                                ));
                            }
                            calls.insert(instruction.offset, slot);
                        }
                    }
                    if let Some(width) = result {
                        stack.push(Operand::Value(width));
                    }
                }
                _ => return Err(unsupported("invalid member reference")),
            },
            InlineOp::Branch { opcode, target } => {
                if *opcode != 0xa7 {
                    pop_operand(&mut stack, false)?;
                }
                if (0x9f..=0xa6).contains(opcode) {
                    pop_operand(&mut stack, false)?;
                }
                successors.push(indices[target]);
            }
            InlineOp::Switch { default, cases } => {
                if pop_operand(&mut stack, false)? != Operand::Value(1) {
                    return Err(unsupported("switch selector is not category one"));
                }
                successors.push(indices[default]);
                successors.extend(cases.iter().map(|(_, target)| indices[target]));
            }
            InlineOp::Return(opcode) => {
                if *opcode != 0xb1 {
                    pop_operand(&mut stack, false)?;
                }
            }
            InlineOp::Raw { opcode, .. } => match opcode {
                0x00 => {}
                0x01..=0x08 | 0x0b..=0x0d | 0x10 | 0x11 => stack.push(Operand::Value(1)),
                0x09..=0x0a | 0x0e..=0x0f => stack.push(Operand::Value(2)),
                0x2e..=0x35 => {
                    pop_operand(&mut stack, false)?;
                    pop_operand(&mut stack, false)?;
                    stack.push(Operand::Value(if matches!(*opcode, 0x2f | 0x31) {
                        2
                    } else {
                        1
                    }));
                }
                0x4f..=0x56 => {
                    for _ in 0..3 {
                        pop_operand(&mut stack, false)?;
                    }
                }
                0x57..=0x5f => stack_operation(&mut stack, *opcode)?,
                0x60..=0x73 | 0x78..=0x83 => {
                    pop_operand(&mut stack, false)?;
                    let operand = pop_operand(&mut stack, false)?;
                    stack.push(operand);
                }
                0x74..=0x77 | 0x91..=0x93 => {
                    let operand = pop_operand(&mut stack, false)?;
                    stack.push(operand);
                }
                0x85 | 0x87 | 0x8a | 0x8c..=0x8d | 0x8f => {
                    pop_operand(&mut stack, false)?;
                    stack.push(Operand::Value(2));
                }
                0x86 | 0x88..=0x89 | 0x8b | 0x8e | 0x90 => {
                    pop_operand(&mut stack, false)?;
                    stack.push(Operand::Value(1));
                }
                0x94..=0x98 => {
                    pop_operand(&mut stack, false)?;
                    pop_operand(&mut stack, false)?;
                    stack.push(Operand::Value(1));
                }
                0xbc | 0xbe => {
                    pop_operand(&mut stack, false)?;
                    stack.push(Operand::Value(1));
                }
                0xbf => {
                    pop_operand(&mut stack, false)?;
                }
                _ => return Err(unsupported("unsupported lambda flow instruction")),
            },
        }
        let terminal = matches!(
            &instruction.kind,
            InlineOp::Return(_)
                | InlineOp::Raw { opcode: 0xbf, .. }
                | InlineOp::Branch { opcode: 0xa7, .. }
                | InlineOp::Switch { .. }
        );
        if !terminal {
            if index + 1 == body.instructions.len() {
                return Err(unsupported("control flow falls outside inline body"));
            }
            successors.push(index + 1);
        }
        for successor in successors {
            if let Some(existing) = states.get(&successor) {
                if existing != &stack {
                    return Err(unsupported(
                        "inconsistent lambda operand flow at branch merge",
                    ));
                }
            } else {
                states.insert(successor, stack.clone());
                queue.push_back(successor);
            }
        }
    }
    Ok(calls)
}

fn descriptor_widths(descriptor: &str) -> Result<(Vec<u8>, Option<u8>)> {
    let bytes = descriptor.as_bytes();
    if bytes.first() != Some(&b'(') {
        return Err(unsupported("invalid method descriptor"));
    }
    let mut cursor = 1;
    let mut params = Vec::new();
    while bytes.get(cursor) != Some(&b')') {
        params.push(
            descriptor_width(bytes, &mut cursor, false)?
                .ok_or_else(|| unsupported("void parameter"))?,
        );
    }
    cursor += 1;
    let result = descriptor_width(bytes, &mut cursor, true)?;
    if cursor != bytes.len() {
        return Err(unsupported("trailing method descriptor data"));
    }
    Ok((params, result))
}

fn descriptor_width(bytes: &[u8], cursor: &mut usize, allow_void: bool) -> Result<Option<u8>> {
    let tag = byte(bytes, cursor)?;
    match tag {
        b'V' if allow_void => Ok(None),
        b'J' | b'D' => Ok(Some(2)),
        b'I' | b'B' | b'C' | b'S' | b'Z' | b'F' => Ok(Some(1)),
        b'[' => {
            descriptor_width(bytes, cursor, false)?;
            Ok(Some(1))
        }
        b'L' => {
            let start = *cursor;
            while byte(bytes, cursor)? != b';' {}
            if *cursor == start + 1 {
                return Err(unsupported("empty reference descriptor"));
            }
            Ok(Some(1))
        }
        _ => Err(unsupported("JVM descriptor kind")),
    }
}

pub(crate) fn validate_access(
    method: &JavaMethod,
    classpath: &crate::classpath::Classpath,
    caller: &str,
) -> Result<()> {
    validate(method)?;
    let package = |owner: &str| {
        owner
            .rsplit_once('/')
            .map_or("", |(package, _)| package)
            .to_owned()
    };
    let accessible = |access: u16, owner: &str| {
        access & 1 != 0 || owner == caller || access & 2 == 0 && package(owner) == package(caller)
    };
    let code = method
        .library
        .as_ref()
        .and_then(|library| library.code.as_ref())
        .ok_or_else(|| unsupported("missing inline bytecode"))?;
    for instruction in decode(code)?.instructions {
        let (opcode, constant) = match instruction.kind {
            InlineOp::Reference {
                opcode, constant, ..
            } => (opcode, constant),
            InlineOp::Constant(InlineConstant::Class(owner)) => {
                (0x12, InlineConstant::Class(owner))
            }
            _ => continue,
        };
        let member_reference = matches!(
            &constant,
            InlineConstant::Method { .. } | InlineConstant::Field { .. }
        );
        let (owner, member_access) = match constant {
            InlineConstant::Method {
                owner,
                name,
                descriptor,
                ..
            } => {
                if owner.starts_with('[') {
                    return Err(unsupported(
                        "array-owned inline method references are unsupported",
                    ));
                }
                if opcode == 0xb7 && name != "<init>" {
                    return Err(unsupported("context-sensitive invokespecial reference"));
                }
                if !accessible(classpath.load(&owner)?.access, &owner) {
                    return Err(unsupported("inaccessible JVM method owner"));
                }
                let target = classpath
                    .ordinary_methods(&owner)?
                    .into_iter()
                    .find(|target| {
                        target.name == name
                            && target.descriptor == descriptor
                            && (name != "<init>" || target.owner == owner)
                    })
                    .ok_or_else(|| {
                        unsupported(format!(
                            "unresolved inline method reference {owner}.{name}{descriptor}"
                        ))
                    })?;
                if (opcode == 0xb8) != (target.access & 8 != 0) {
                    return Err(unsupported("inline invocation kind disagrees with target"));
                }
                (target.owner, target.access)
            }
            InlineConstant::Field {
                owner,
                name,
                descriptor,
            } => {
                if !accessible(classpath.load(&owner)?.access, &owner) {
                    return Err(unsupported("inaccessible JVM field owner"));
                }
                let field =
                    inline_field(classpath, &owner, &name, &descriptor, &mut BTreeSet::new())?
                        .ok_or_else(|| unsupported("unresolved inline field reference"))?;
                if matches!(opcode, 0xb2 | 0xb3) != (field.access & 8 != 0) {
                    return Err(unsupported(
                        "inline field access kind disagrees with target",
                    ));
                }
                (field.owner, field.access)
            }
            InlineConstant::Class(owner) => {
                if owner.starts_with('[') {
                    let component = owner.trim_start_matches('[');
                    if component.len() == 1 && "BCDFIJSZ".contains(component) {
                        continue;
                    }
                    let component = component
                        .strip_prefix('L')
                        .and_then(|component| component.strip_suffix(';'))
                        .filter(|component| !component.is_empty())
                        .ok_or_else(|| unsupported("malformed array class reference"))?;
                    (component.to_owned(), 1)
                } else {
                    (owner, 1)
                }
            }
            _ => continue,
        };
        let class = classpath.load(&owner)?;
        if !member_reference && !accessible(class.access, &owner)
            || !accessible(member_access, &owner)
        {
            return Err(unsupported(format!(
                "inaccessible JVM reference in {}/{}",
                method.owner, method.name
            )));
        }
    }
    Ok(())
}

fn inline_field(
    classpath: &crate::classpath::Classpath,
    owner: &str,
    name: &str,
    descriptor: &str,
    seen: &mut BTreeSet<String>,
) -> Result<Option<crate::classpath::JavaField>> {
    if !seen.insert(owner.to_owned()) {
        return Ok(None);
    }
    if seen.len() > 256 {
        return Err(unsupported("inline field hierarchy is too deep"));
    }
    let class = classpath.load(owner)?;
    if let Some(field) = class
        .fields
        .into_iter()
        .find(|field| field.name == name && field.descriptor == descriptor)
    {
        return Ok(Some(field));
    }
    for parent in class.interfaces.iter().chain(class.super_name.iter()) {
        if let Some(field) = inline_field(classpath, parent, name, descriptor, seen)? {
            return Ok(Some(field));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(bytes: &[u8]) -> JavaCode {
        JavaCode {
            max_stack: 8,
            max_locals: 8,
            bytes: bytes.into(),
            exception_handlers: 0,
            constants: BTreeMap::new().into(),
        }
    }

    fn method(owner: &str, name: &str, descriptor: &str) -> InlineConstant {
        InlineConstant::Method {
            owner: owner.into(),
            name: name.into(),
            descriptor: descriptor.into(),
            interface: owner.starts_with("kotlin/jvm/functions/"),
        }
    }

    #[test]
    fn stack_instructions_accept_all_jvm_category_forms() {
        for (opcode, input, output) in [
            (0x57, vec![1], vec![]),
            (0x58, vec![2], vec![]),
            (0x58, vec![1, 1], vec![]),
            (0x59, vec![1], vec![1, 1]),
            (0x5a, vec![1, 1], vec![1, 1, 1]),
            (0x5b, vec![2, 1], vec![1, 2, 1]),
            (0x5b, vec![1, 1, 1], vec![1, 1, 1, 1]),
            (0x5c, vec![2], vec![2, 2]),
            (0x5c, vec![1, 1], vec![1, 1, 1, 1]),
            (0x5d, vec![1, 2], vec![2, 1, 2]),
            (0x5d, vec![1, 1, 1], vec![1, 1, 1, 1, 1]),
            (0x5e, vec![2, 2], vec![2, 2, 2]),
            (0x5e, vec![1, 1, 2], vec![2, 1, 1, 2]),
            (0x5e, vec![2, 1, 1], vec![1, 1, 2, 1, 1]),
            (0x5e, vec![1, 1, 1, 1], vec![1, 1, 1, 1, 1, 1]),
            (0x5f, vec![1, 1], vec![1, 1]),
        ] {
            let mut stack = input.into_iter().map(Operand::Value).collect();
            stack_operation(&mut stack, opcode).unwrap();
            assert_eq!(
                stack,
                output.into_iter().map(Operand::Value).collect::<Vec<_>>()
            );
        }
        for (opcode, input) in [
            (0x57, vec![Operand::Value(2)]),
            (0x58, vec![Operand::Value(2), Operand::Value(1)]),
            (0x59, vec![Operand::Value(2)]),
            (0x5a, vec![Operand::Value(2), Operand::Value(1)]),
            (0x5f, vec![Operand::Value(1), Operand::Value(2)]),
            (0x59, vec![Operand::Lambda(0)]),
        ] {
            assert!(stack_operation(&mut input.clone(), opcode).is_err());
        }
    }

    #[test]
    fn switch_decoding_checks_layout_keys_and_instruction_boundaries() {
        let mut table = vec![0x03, 0xaa, 0, 0];
        for word in [23i32, 0, 1, 23, 23] {
            table.extend(word.to_be_bytes());
        }
        table.push(0xb1);
        let body = decode(&code(&table)).unwrap();
        lambda_calls(&body, &BTreeSet::new()).unwrap();
        assert!(
            matches!(&body.instructions[1].kind, InlineOp::Switch { default: 24, cases } if cases == &[(0, 24), (1, 24)])
        );
        let mut lookup = vec![0x03, 0xab, 0, 0];
        for word in [27i32, 2, -100, 27, 100, 27] {
            lookup.extend(word.to_be_bytes());
        }
        lookup.push(0xb1);
        lambda_calls(&decode(&code(&lookup)).unwrap(), &BTreeSet::new()).unwrap();
        for position in [2, 7, 11, 19] {
            let mut invalid = table.clone();
            invalid[position] = 0xff;
            assert!(decode(&code(&invalid)).is_err(), "position {position}");
        }
        lookup[20..24].copy_from_slice(&(-101i32).to_be_bytes());
        assert!(
            decode(&code(&lookup))
                .unwrap_err()
                .message
                .contains("unordered")
        );
        for truncated in [vec![0xaa], vec![0xab, 0, 0, 0], vec![0xc8, 0, 0, 0]] {
            assert!(decode(&code(&truncated)).is_err());
        }
        let body = decode(&code(&[0xc8, 0, 0, 0, 5, 0xb1])).unwrap();
        assert!(matches!(
            body.instructions[0].kind,
            InlineOp::Branch {
                opcode: 0xa7,
                target: 5
            }
        ));
    }

    #[test]
    fn decodes_backward_branches_and_compact_and_wide_locals() {
        let body = decode(&code(&[
            0x03, 0x3c, 0x84, 1, 1, 0x1b, 0x10, 5, 0xa1, 0xff, 0xfa, 0xc4, 0x15, 0, 1, 0xac,
        ]))
        .unwrap();
        assert!(matches!(
            body.instructions[1].kind,
            InlineOp::Local {
                opcode: 0x36,
                slot: 1
            }
        ));
        assert!(matches!(
            body.instructions[5].kind,
            InlineOp::Branch { target: 2, .. }
        ));
        assert!(matches!(
            body.instructions[6].kind,
            InlineOp::Local {
                opcode: 0x15,
                slot: 1
            }
        ));
        lambda_calls(&body, &BTreeSet::new()).unwrap();
    }

    #[test]
    fn rejects_branches_into_operands_and_truncated_instructions() {
        for bytes in [
            &[0xa7, 0, 1][..],
            &[0x11, 0][..],
            &[0xb9, 0, 1, 1][..],
            &[0xc4, 0x15, 0][..],
        ] {
            assert!(decode(&code(bytes)).is_err());
        }
    }

    #[test]
    fn rejects_exception_regions_and_unsupported_stack_operations() {
        let mut exception = code(&[0xb1]);
        exception.exception_handlers = 1;
        assert!(
            decode(&exception)
                .unwrap_err()
                .message
                .contains("exception handlers")
        );
        assert!(lambda_calls(&decode(&code(&[0x5a, 0xb1])).unwrap(), &BTreeSet::new()).is_err());
        assert!(decode(&code(&[0xba, 0, 1, 0, 0, 0xb1])).is_err());
    }

    fn inline_method(code: JavaCode) -> JavaMethod {
        JavaMethod {
            library: Some(crate::classpath::LibraryMethod {
                contracts: Vec::new(),
                value_dispatch: false,
                suspend_: false,
                kotlin_declared: true,
                has_annotations: false,
                parameter_modes: Vec::new(),
                reified_arguments: Vec::new(),
                abstract_: false,
                builtin_array: false,
                kotlin_public: true,
                source_name: "construct".into(),
                type_parameters: Vec::new(),
                receiver: None,
                infix: false,
                operator: false,
                inline: true,
                code: Some(code),
            }),
            owner: "example/Library".into(),
            name: "construct".into(),
            descriptor: "(Lkotlin/jvm/functions/Function1;)Ljava/lang/Object;".into(),
            access: 9,
            unsupported: false,
            source_property: None,
            kotlin_params: None,
            kotlin_result: None,
            kotlin_internal: false,
        }
    }

    fn allocation_body(owner: &str) -> JavaCode {
        let mut input = code(&[
            0x2a, 0xbb, 0, 1, 0x59, 0x08, 0xb7, 0, 2, 0xb9, 0, 3, 2, 0, 0xb0,
        ]);
        std::sync::Arc::make_mut(&mut input.constants).extend([
            (1, InlineConstant::Class(owner.into())),
            (2, method(owner, "<init>", "(I)V")),
            (
                3,
                method(
                    "kotlin/jvm/functions/Function1",
                    "invoke",
                    "(Ljava/lang/Object;)Ljava/lang/Object;",
                ),
            ),
        ]);
        input
    }

    #[test]
    fn validates_root_virtual_references_without_exposing_protected_or_wrong_kind_members() {
        let classpath = crate::classpath::Classpath::new(&[]).unwrap();
        for (opcode, owner, name, descriptor, accepted) in [
            (
                0xb6,
                "java/lang/Object",
                "toString",
                "()Ljava/lang/String;",
                true,
            ),
            (0xb6, "java/lang/StringBuilder", "hashCode", "()I", true),
            (
                0xb6,
                "java/lang/Object",
                "clone",
                "()Ljava/lang/Object;",
                false,
            ),
            (
                0xb8,
                "java/lang/Object",
                "toString",
                "()Ljava/lang/String;",
                false,
            ),
            (0xb7, "java/lang/Integer", "<init>", "()V", false),
        ] {
            let mut input = code(&[0x2a, opcode, 0, 1, 0xb0]);
            std::sync::Arc::make_mut(&mut input.constants)
                .insert(1, method(owner, name, descriptor));
            let result = validate_access(&inline_method(input), &classpath, "external/Caller");
            assert_eq!(result.is_ok(), accepted, "{owner}.{name}: {result:?}");
        }
    }

    #[test]
    fn decodes_external_construction_and_tracks_lambdas_across_constructor_arguments() {
        for owner in ["example/Widget", "example/Other$Inner"] {
            let input = allocation_body(owner);
            let body = decode(&input).unwrap();
            assert!(matches!(&body.instructions[1].kind,
                InlineOp::Reference { opcode: 0xbb, constant: InlineConstant::Class(class), .. } if class == owner));
            assert_eq!(
                lambda_calls(&body, &BTreeSet::from([0])).unwrap(),
                BTreeMap::from([(9, 0)])
            );
            validate(&inline_method(input)).unwrap();
        }
    }

    #[test]
    fn rejects_inline_owner_nested_allocations_requiring_generated_classes() {
        for owner in ["example/Library$Local", "example/Library$construct$1"] {
            let method = inline_method(allocation_body(owner));
            let error = validate(&method).unwrap_err();
            assert!(error.message.contains("class regeneration"));
        }
    }

    #[test]
    fn rejects_allocations_without_a_class_constant() {
        let mut input = code(&[0xbb, 0, 1, 0xb0]);
        assert!(
            decode(&input)
                .unwrap_err()
                .message
                .contains("constant pool")
        );
        std::sync::Arc::make_mut(&mut input.constants).insert(1, InlineConstant::Integer(1));
        assert!(
            decode(&input)
                .unwrap_err()
                .message
                .contains("constant pool reference kind")
        );
    }

    #[test]
    fn identifies_erased_lambda_receiver_across_argument_evaluation() {
        let mut input = code(&[0x2a, 0x08, 0xb8, 0, 1, 0xb9, 0, 2, 2, 0, 0xb0]);
        std::sync::Arc::make_mut(&mut input.constants).insert(
            1,
            method("java/lang/Integer", "valueOf", "(I)Ljava/lang/Integer;"),
        );
        std::sync::Arc::make_mut(&mut input.constants).insert(
            2,
            method(
                "kotlin/jvm/functions/Function1",
                "invoke",
                "(Ljava/lang/Object;)Ljava/lang/Object;",
            ),
        );
        let calls = lambda_calls(&decode(&input).unwrap(), &BTreeSet::from([0])).unwrap();
        assert_eq!(calls, BTreeMap::from([(5, 0)]));
    }

    #[test]
    fn preserves_lambda_identity_at_branch_merge_and_multiple_calls() {
        let mut input = code(&[
            0x03, 0x99, 0, 12, 0x2a, 0xb9, 0, 1, 1, 0, 0xa7, 0, 9, 0x2a, 0xb9, 0, 1, 1, 0, 0xb0,
        ]);
        std::sync::Arc::make_mut(&mut input.constants).insert(
            1,
            method(
                "kotlin/jvm/functions/Function0",
                "invoke",
                "()Ljava/lang/Object;",
            ),
        );
        let calls = lambda_calls(&decode(&input).unwrap(), &BTreeSet::from([0])).unwrap();
        assert_eq!(calls, BTreeMap::from([(5, 0), (14, 0)]));
    }

    #[test]
    fn rejects_lambda_aliasing_and_forwarding() {
        let mut forwarded = code(&[0x2a, 0xb8, 0, 1, 0xb1]);
        std::sync::Arc::make_mut(&mut forwarded.constants).insert(
            1,
            method("example/Library", "consume", "(Ljava/lang/Object;)V"),
        );
        for input in [code(&[0x2a, 0x4c, 0xb1]), code(&[0x2a, 0xb0]), forwarded] {
            let error = lambda_calls(&decode(&input).unwrap(), &BTreeSet::from([0])).unwrap_err();
            assert!(error.message.contains("escapes"), "{}", error.message);
        }
    }

    #[test]
    fn accepts_kotlin_parameter_null_check_without_hiding_lambda_uses() {
        let mut input = code(&[0x2a, 0x12, 1, 0xb8, 0, 2, 0x2a, 0xb9, 0, 3, 1, 0, 0xb0]);
        std::sync::Arc::make_mut(&mut input.constants)
            .insert(1, InlineConstant::String("block".encode_utf16().collect()));
        std::sync::Arc::make_mut(&mut input.constants).insert(
            2,
            method(
                "kotlin/jvm/internal/Intrinsics",
                "checkNotNullParameter",
                "(Ljava/lang/Object;Ljava/lang/String;)V",
            ),
        );
        std::sync::Arc::make_mut(&mut input.constants).insert(
            3,
            method(
                "kotlin/jvm/functions/Function0",
                "invoke",
                "()Ljava/lang/Object;",
            ),
        );
        assert_eq!(
            lambda_calls(&decode(&input).unwrap(), &BTreeSet::from([0])).unwrap(),
            BTreeMap::from([(7, 0)])
        );
    }

    #[test]
    fn checks_category_two_local_bounds_and_constant_kinds() {
        let mut input = code(&[0x16, 7, 0xad]);
        assert!(decode(&input).is_err());
        input.bytes = vec![0x14, 0, 1, 0xad];
        std::sync::Arc::make_mut(&mut input.constants).insert(1, InlineConstant::Integer(123));
        assert!(decode(&input).is_err());
        std::sync::Arc::make_mut(&mut input.constants).insert(1, InlineConstant::Long(i64::MIN));
        assert!(decode(&input).is_ok());
    }
}
