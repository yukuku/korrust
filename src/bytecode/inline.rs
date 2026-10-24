use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::classpath::{InlineConstant, JavaMethod};
use crate::compiled_inline::{InlineOp, decode, lambda_calls};

impl Code<'_> {
    pub(super) fn local_slot(&self, slot: u8) -> Result<u8> {
        self.inline_slot(u16::from(slot))
    }

    fn inline_slot(&self, slot: u16) -> Result<u8> {
        let mapped = match self.inline_local_maps.last() {
            Some(map) => *map
                .get(usize::from(slot))
                .ok_or_else(|| failure("Inline local outside reserved lambda range"))?,
            None => slot,
        };
        u8::try_from(mapped)
            .map_err(|_| failure("Compiled inline locals exceed current JVM slot limit"))
    }

    pub(super) fn inline_return(&mut self, value: &Value) -> Result<bool> {
        let Some(label) = self.inline_returns.last().copied() else {
            return Ok(false);
        };
        self.value(value)?;
        if !terminal_value(value) {
            self.adapt_value(&value.ty, &Type::Java("java/lang/Object".into()))?;
            self.branch(0xa7, label);
        }
        Ok(true)
    }

    pub(super) fn compiled_inline(
        &mut self,
        method: &JavaMethod,
        args: &[Value],
        local_base: u16,
        lambda_bases: &[Option<u16>],
        result: &Type,
    ) -> Result<()> {
        crate::compiled_inline::validate(method)?;
        let code = method
            .library
            .as_ref()
            .and_then(|library| library.code.as_ref())
            .ok_or_else(|| failure("Compiled inline bytecode unavailable"))?;
        let body = decode(code)?;
        let (params, erased_result) = invocation_types(&method.descriptor)?;
        if params.len() != args.len() || lambda_bases.len() != args.len() {
            return Err(failure("Compiled inline argument count mismatch"));
        }
        let mut slots = Vec::new();
        let mut slot = 0u16;
        let mut lambda_arguments = BTreeMap::new();
        for (index, param) in params.iter().enumerate() {
            slots.push(slot);
            if let Some(base) = lambda_bases[index] {
                let ValueKind::Lambda { owner, .. } = &args[index].kind else {
                    return Err(failure("Compiled inline lambda argument was not a literal"));
                };
                let declaration = self
                    .lambdas
                    .iter()
                    .find(|lambda| &lambda.owner == owner)
                    .ok_or_else(|| {
                        failure("Compiled inline lambda body unavailable in this declaration")
                    })?;
                lambda_arguments.insert(slot, (declaration, base));
            }
            slot = slot
                .checked_add(param.slots() as u16)
                .ok_or_else(|| failure("Inline parameter slot overflow"))?;
        }
        if slot > body.max_locals {
            return Err(failure("Compiled inline parameters exceed max_locals"));
        }
        let calls = lambda_calls(
            &body,
            &lambda_arguments.keys().copied().collect::<BTreeSet<_>>(),
        )?;
        let mut labels = BTreeMap::new();
        for instruction in &body.instructions {
            labels.insert(instruction.offset, self.label());
        }
        let end = self.label();
        self.max_stack = self.max_stack.max(usize::from(body.max_stack) + 8);
        for ((arg, param), slot) in args.iter().zip(&params).zip(&slots) {
            self.value(arg)?;
            self.adapt_value(&arg.ty, param)?;
            let target = self.inline_slot(
                local_base
                    .checked_add(*slot)
                    .ok_or_else(|| failure("Inline local overflow"))?,
            )?;
            self.bytes.extend([store_opcode(param), target]);
        }
        for instruction in &body.instructions {
            self.mark(labels[&instruction.offset]);
            if let Some(slot) = calls.get(&instruction.offset) {
                let (lambda, base) = lambda_arguments[slot];
                self.inline_lambda(lambda, base)?;
                continue;
            }
            match &instruction.kind {
                InlineOp::Raw { opcode, operands } => {
                    self.bytes.push(*opcode);
                    self.bytes.extend(operands);
                }
                InlineOp::Local { opcode, slot } => {
                    let slot = self.inline_slot(
                        local_base
                            .checked_add(*slot)
                            .ok_or_else(|| failure("Inline local overflow"))?,
                    )?;
                    self.bytes.extend([*opcode, slot]);
                }
                InlineOp::Increment { slot, amount } => {
                    let slot = self.inline_slot(
                        local_base
                            .checked_add(*slot)
                            .ok_or_else(|| failure("Inline local overflow"))?,
                    )?;
                    if let Ok(amount) = i8::try_from(*amount) {
                        self.bytes.extend([0x84, slot, amount as u8]);
                    } else {
                        self.bytes.extend([0x15, slot]);
                        self.integer(i32::from(*amount))?;
                        self.bytes.extend([0x60, 0x36, slot]);
                    }
                }
                InlineOp::Constant(constant) => self.inline_constant(constant)?,
                InlineOp::Reference {
                    opcode,
                    constant,
                    interface_count,
                } => {
                    self.inline_reference(*opcode, constant, *interface_count)?;
                }
                InlineOp::Branch { opcode, target } => self.branch(*opcode, labels[target]),
                InlineOp::Return(_) => self.branch(0xa7, end),
            }
        }
        self.mark(end);
        self.adapt_value(&erased_result, result)
    }

    fn inline_lambda(&mut self, lambda: &crate::check::LambdaDecl, base: u16) -> Result<()> {
        if lambda.interface.is_some() {
            return Err(failure(
                "SAM literals cannot substitute Kotlin inline parameters",
            ));
        }
        let function = &lambda.function;
        let mut map = Vec::new();
        for slot in 0..function.locals {
            map.push(u16::from(
                self.inline_slot(
                    base.checked_add(slot)
                        .ok_or_else(|| failure("Inline lambda local overflow"))?,
                )?,
            ));
        }
        let param_slots = parameter_slots(&function.params, 1);
        for ((_, ty), slot) in function.params.iter().zip(&param_slots).rev() {
            self.adapt_value(&Type::Java("java/lang/Object".into()), ty)?;
            self.bytes.extend([store_opcode(ty), map[*slot] as u8]);
        }
        self.bytes.extend([
            0x3a,
            *map.first()
                .ok_or_else(|| failure("Inline lambda has no receiver local"))? as u8,
        ]);
        let end = self.label();
        self.inline_local_maps.push(map);
        self.inline_returns.push(end);
        let saved_loops = std::mem::take(&mut self.loops);
        let emitted = self.block(&function.body);
        self.loops = saved_loops;
        self.inline_returns.pop();
        self.inline_local_maps.pop();
        emitted?;
        self.mark(end);
        Ok(())
    }

    fn inline_constant(&mut self, constant: &InlineConstant) -> Result<()> {
        match constant {
            InlineConstant::Integer(value) => self.integer(*value),
            InlineConstant::Float(value) => {
                let index = self.pool.add(Constant::Float(*value))?;
                self.reference(0x13, index);
                Ok(())
            }
            InlineConstant::Long(value) => {
                let index = self.pool.add(Constant::Long(*value))?;
                self.reference(0x14, index);
                Ok(())
            }
            InlineConstant::String(value) => {
                let index = self.pool.string_units(value)?;
                self.reference(0x13, index);
                Ok(())
            }
            _ => Err(failure("Unsupported compiled inline literal")),
        }
    }

    fn inline_reference(
        &mut self,
        opcode: u8,
        constant: &InlineConstant,
        interface_count: u8,
    ) -> Result<()> {
        let index = match constant {
            InlineConstant::Class(name) => self.pool.class(name)?,
            InlineConstant::Field {
                owner,
                name,
                descriptor,
            } => self.pool.field(owner, name, descriptor)?,
            InlineConstant::Method {
                owner,
                name,
                descriptor,
                interface,
            } => {
                if *interface {
                    self.pool.interface_method(owner, name, descriptor)?
                } else {
                    self.pool.method(owner, name, descriptor)?
                }
            }
            _ => return Err(failure("Unsupported compiled inline member reference")),
        };
        self.reference(opcode, index);
        if opcode == 0xb9 {
            self.bytes.extend([interface_count, 0]);
        }
        Ok(())
    }
}

pub(super) fn contains_nonlocal(
    function: &crate::check::Function,
    lambdas: &[crate::check::LambdaDecl],
) -> bool {
    nonlocal_block(&function.body, lambdas)
}
fn nonlocal_block(body: &[Statement], lambdas: &[crate::check::LambdaDecl]) -> bool {
    body.iter().any(|statement| match statement {
        Statement::NonLocalReturn(_) => true,
        Statement::If(value, yes, no) => {
            nonlocal_value(value, lambdas)
                || nonlocal_block(yes, lambdas)
                || nonlocal_block(no, lambdas)
        }
        Statement::While(value, body) => {
            nonlocal_value(value, lambdas) || nonlocal_block(body, lambdas)
        }
        Statement::ForUntil {
            start, end, body, ..
        } => {
            nonlocal_value(start, lambdas)
                || nonlocal_value(end, lambdas)
                || nonlocal_block(body, lambdas)
        }
        Statement::Store(_, value)
        | Statement::Return(value)
        | Statement::Expression(value)
        | Statement::Throw(value) => nonlocal_value(value, lambdas),
        Statement::ThrowException { message, .. } => message
            .as_ref()
            .is_some_and(|value| nonlocal_value(value, lambdas)),
        Statement::IndexStore(a, b, c) | Statement::ListSet(a, b, c) => {
            nonlocal_value(a, lambdas) || nonlocal_value(b, lambdas) || nonlocal_value(c, lambdas)
        }
        Statement::FieldStore {
            receiver, value, ..
        } => nonlocal_value(receiver, lambdas) || nonlocal_value(value, lambdas),
        Statement::JavaFieldStore {
            receiver, value, ..
        } => {
            receiver
                .as_ref()
                .is_some_and(|value| nonlocal_value(value, lambdas))
                || nonlocal_value(value, lambdas)
        }
        Statement::Continue | Statement::Break => false,
    })
}

fn nonlocal_value(value: &Value, lambdas: &[crate::check::LambdaDecl]) -> bool {
    match &value.kind {
        ValueKind::Abrupt(body) => nonlocal_block(body, lambdas),
        ValueKind::If(a, b, c) => {
            nonlocal_value(a, lambdas) || nonlocal_value(b, lambdas) || nonlocal_value(c, lambdas)
        }
        ValueKind::Let { value, body, .. } => {
            nonlocal_value(value, lambdas) || nonlocal_value(body, lambdas)
        }
        ValueKind::Box(value)
        | ValueKind::Unbox(value)
        | ValueKind::CellIncrement { cell: value, .. }
        | ValueKind::IsNull(value)
        | ValueKind::ListSize(value)
        | ValueKind::NewArray(_, value)
        | ValueKind::Convert(_, value)
        | ValueKind::ThreadLocalGet(value, _)
        | ValueKind::RequireNotNull(value)
        | ValueKind::IntToString(value)
        | ValueKind::NewIntArray(value)
        | ValueKind::NotNull(value, _)
        | ValueKind::Field(_, value)
        | ValueKind::Negate(value)
        | ValueKind::Not(value)
        | ValueKind::Length(value) => nonlocal_value(value, lambdas),
        ValueKind::StringConcat(a, b)
        | ValueKind::ListGet(a, b)
        | ValueKind::ThreadLocalSet(a, b)
        | ValueKind::CopyOf(a, b)
        | ValueKind::Binary(_, a, b)
        | ValueKind::Index(a, b)
        | ValueKind::Compare(a, b) => nonlocal_value(a, lambdas) || nonlocal_value(b, lambdas),
        ValueKind::ListSet(a, b, c) => {
            nonlocal_value(a, lambdas) || nonlocal_value(b, lambdas) || nonlocal_value(c, lambdas)
        }
        ValueKind::FunctionInvoke { function, args } => {
            nonlocal_value(function, lambdas)
                || args.iter().any(|value| nonlocal_value(value, lambdas))
        }
        ValueKind::CompiledInline { args, .. } => args.iter().any(|value| {
            if let ValueKind::Lambda { owner, .. } = &value.kind {
                if let Some(lambda) = lambdas.iter().find(|lambda| &lambda.owner == owner) {
                    if contains_nonlocal(&lambda.function, lambdas) {
                        return true;
                    }
                }
            }
            nonlocal_value(value, lambdas)
        }),
        ValueKind::New { args, .. }
        | ValueKind::StaticCall { args, .. }
        | ValueKind::MemberCall { args, .. }
        | ValueKind::JavaCall { args, .. }
        | ValueKind::StringFromChars(args)
        | ValueKind::CopyInto(args)
        | ValueKind::StringCall(_, args)
        | ValueKind::Call(_, args)
        | ValueKind::DefaultCall(_, args, _) => {
            args.iter().any(|value| nonlocal_value(value, lambdas))
        }
        ValueKind::JavaField { receiver, .. } => receiver
            .as_deref()
            .is_some_and(|value| nonlocal_value(value, lambdas)),
        ValueKind::ListFactory { elements, .. } => {
            elements.iter().any(|value| nonlocal_value(value, lambdas))
        }
        ValueKind::Lambda { captures, .. } => {
            captures.iter().any(|value| nonlocal_value(value, lambdas))
        }
        _ => false,
    }
}
