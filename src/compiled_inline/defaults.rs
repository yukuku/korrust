use super::*;
use crate::classpath::Classpath;

fn integer(instruction: &InlineInstruction) -> Option<i32> {
    match &instruction.kind {
        InlineOp::Constant(InlineConstant::Integer(value)) => Some(*value),
        InlineOp::Raw { opcode, operands }
            if (0x02..=0x08).contains(opcode) && operands.is_empty() =>
        {
            Some(i32::from(*opcode) - 3)
        }
        InlineOp::Raw {
            opcode: 0x10,
            operands,
        } if operands.len() == 1 => Some(i32::from(operands[0] as i8)),
        InlineOp::Raw {
            opcode: 0x11,
            operands,
        } if operands.len() == 2 => Some(i32::from(i16::from_be_bytes([operands[0], operands[1]]))),
        _ => None,
    }
}

pub(crate) fn with_defaults(
    mut method: JavaMethod,
    mask: u64,
    parameters: usize,
    default_descriptor: Option<String>,
    classpath: &Classpath,
    caller: &str,
    offset: usize,
) -> Result<JavaMethod> {
    let mut code = method
        .library
        .as_ref()
        .and_then(|library| library.default_code.clone())
        .ok_or_else(|| unsupported("default dispatcher bytecode is unavailable"))?;
    let instance = method.access & 8 == 0;
    let words = parameters.div_ceil(32);
    let closing = method
        .descriptor
        .find(')')
        .ok_or_else(|| unsupported("default dispatcher descriptor is malformed"))?;
    let dispatch = if instance {
        format!("L{};", method.owner)
    } else {
        String::new()
    };
    let descriptor = default_descriptor.unwrap_or_else(|| {
        format!(
            "({dispatch}{}{}Ljava/lang/Object;){}",
            &method.descriptor[1..closing],
            "I".repeat(words),
            &method.descriptor[closing + 1..]
        )
    });
    let (widths, _) = descriptor_widths(&descriptor)?;
    let first_mask = widths[..widths.len() - words - 1]
        .iter()
        .map(|width| u16::from(*width))
        .sum::<u16>();
    let masks = (0..words)
        .map(|word| (first_mask + word as u16, (mask >> (word * 32)) as u32))
        .collect::<BTreeMap<_, _>>();
    let body = decode(&code)?;
    let indices = body
        .instructions
        .iter()
        .enumerate()
        .map(|(index, instruction)| (instruction.offset, index))
        .collect::<BTreeMap<_, _>>();
    let mut cursor = match body.instructions.as_slice() {
        [load, branch, ..]
            if instance
                && matches!(load.kind, InlineOp::Local { opcode: 0x19, slot } if slot == first_mask + words as u16) =>
        {
            match branch.kind {
                InlineOp::Branch {
                    opcode: 0xc6,
                    target,
                } => *indices
                    .get(&target)
                    .ok_or_else(|| unsupported("invalid default marker branch"))?,
                _ => 0,
            }
        }
        _ => 0,
    };
    let mut recognized = 0u64;
    let mut erased = Vec::new();
    while cursor + 4 < body.instructions.len() {
        let instructions = &body.instructions[cursor..];
        let InlineOp::Local { opcode: 0x15, slot } = instructions[0].kind else {
            break;
        };
        let Some(mask_word) = masks.get(&slot) else {
            break;
        };
        let Some(bit) = integer(&instructions[1])
            .map(|value| value as u32)
            .filter(|bit| bit.is_power_of_two())
        else {
            break;
        };
        if !matches!(instructions[2].kind, InlineOp::Raw { opcode: 0x7e, .. }) {
            break;
        }
        let InlineOp::Branch {
            opcode: 0x99,
            target,
        } = instructions[3].kind
        else {
            break;
        };
        let start = instructions[0].offset;
        let value_start = instructions[4].offset;
        if target <= value_start {
            return Err(unsupported("default guard has no forward value region"));
        }
        let word = usize::from(slot - first_mask);
        recognized |= u64::from(bit) << (word * 32);
        code.bytes[start..value_start].fill(0);
        if mask_word & bit == 0 {
            code.bytes[value_start..target].fill(0);
            erased.push((value_start, target));
        }
        cursor = indices[&target];
    }
    if mask & !recognized != 0 {
        return Err(unsupported(
            "default dispatcher does not have supported mask guards",
        ));
    }
    for handler in &code.exception_handlers {
        if erased
            .iter()
            .any(|(start, end)| *start <= handler.handler && handler.handler < *end)
            && !erased
                .iter()
                .any(|(start, end)| *start <= handler.start && handler.end <= *end)
        {
            return Err(unsupported(
                "default dispatcher exception handler crosses a removed value region",
            ));
        }
    }
    code.exception_handlers.retain(|handler| {
        !erased
            .iter()
            .any(|(start, end)| *start <= handler.start && handler.end <= *end)
    });
    let body = decode(&code)?;
    for (index, instruction) in body.instructions.iter().enumerate() {
        if matches!(&instruction.kind, InlineOp::Reference { opcode: 0xb8, constant: InlineConstant::Method { owner, name, descriptor, .. }, .. }
            if owner == "kotlin/jvm/internal/Intrinsics" && name == "needClassReification" && descriptor == "()V")
        {
            let Some(next) = body.instructions.get(index + 1) else {
                return Err(unsupported("default lambda reification has no value"));
            };
            let InlineOp::Reference {
                opcode: 0xb2,
                constant:
                    InlineConstant::Field {
                        owner,
                        name,
                        descriptor,
                    },
                ..
            } = &next.kind
            else {
                return Err(unsupported("default lambda requires class regeneration"));
            };
            if name != "INSTANCE" || descriptor != &format!("L{owner};") {
                return Err(unsupported("default lambda requires class regeneration"));
            }
            let renamed = classpath.regenerate_inline_default_lambda(owner, caller, offset)?;
            for constant in std::sync::Arc::make_mut(&mut code.constants).values_mut() {
                match constant {
                    InlineConstant::Class(class) if class == owner => *class = renamed.clone(),
                    InlineConstant::Field {
                        owner: declaring,
                        descriptor,
                        ..
                    }
                    | InlineConstant::Method {
                        owner: declaring,
                        descriptor,
                        ..
                    } => {
                        if declaring == owner {
                            *declaring = renamed.clone();
                        }
                        *descriptor =
                            descriptor.replace(&format!("L{owner};"), &format!("L{renamed};"));
                    }
                    _ => {}
                }
            }
            code.bytes[instruction.offset..next.offset].fill(0);
        }
    }
    method.descriptor = descriptor;
    method.name.push_str("$default");
    let library = method.library.as_mut().unwrap();
    if instance {
        method.access |= 8;
        library
            .parameter_modes
            .insert(0, crate::syntax::InlineParameterMode::Ordinary);
    }
    library.code = Some(code);
    Ok(method)
}
