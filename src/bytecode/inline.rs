use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::classpath::{InlineConstant, JavaMethod};
use crate::compiled_inline::{InlineBody, InlineOp, decode, lambda_calls};

fn reified_unsupported(message: &str) -> Diagnostic {
    Diagnostic::new(0, message).with_kind(
        crate::DiagnosticKind::UnsupportedLanguage,
        "REIFIED_BYTECODE_UNSUPPORTED",
    )
}

type RegionExclusions = Rc<RefCell<Vec<(usize, usize)>>>;

#[derive(Clone)]
pub(super) struct InlineExpansion {
    body: InlineBody,
    local_base: u16,
    lambda_arguments: BTreeMap<u16, (String, u16)>,
    calls: BTreeMap<usize, u16>,
    reified: BTreeMap<usize, (i32, Type)>,
    reified_temp: Option<u16>,
    skipped: BTreeSet<usize>,
    end: usize,
    finalizers: BTreeMap<usize, (usize, usize)>,
    exclusions: BTreeMap<usize, RegionExclusions>,
}

fn finally_marker(instruction: &crate::compiled_inline::InlineInstruction, name: &str) -> bool {
    matches!(&instruction.kind, InlineOp::Reference {
        opcode: 0xb8, constant: InlineConstant::Method { owner, name: actual, descriptor, .. }, ..
    } if owner == "kotlin/jvm/internal/InlineMarker" && actual == name && descriptor == "(I)V")
}

fn finalizer_regions(body: &InlineBody) -> BTreeMap<usize, (usize, usize)> {
    let mut regions = BTreeMap::new();
    for handler in body
        .exception_handlers
        .iter()
        .filter(|handler| handler.catch_type.is_none())
    {
        let Some(index) = body
            .instructions
            .iter()
            .position(|instruction| instruction.offset == handler.handler)
        else {
            continue;
        };
        if !matches!(
            body.instructions[index].kind,
            InlineOp::Local { opcode: 0x3a, .. }
        ) {
            continue;
        }
        let Some(start) = body
            .instructions
            .get(index + 2)
            .filter(|instruction| finally_marker(instruction, "finallyStart"))
            .map(|_| index + 3)
        else {
            continue;
        };
        let mut depth = 1;
        for end in start..body.instructions.len() {
            if finally_marker(&body.instructions[end], "finallyStart") {
                depth += 1;
            }
            if finally_marker(&body.instructions[end], "finallyEnd") {
                depth -= 1;
                if depth == 0 {
                    if end > start {
                        regions.insert(
                            handler.handler,
                            (
                                body.instructions[start].offset,
                                body.instructions[end - 1].offset,
                            ),
                        );
                    }
                    break;
                }
            }
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = regions.entry(handler.handler) {
            let indices: BTreeMap<_, _> = body
                .instructions
                .iter()
                .enumerate()
                .map(|(index, instruction)| (instruction.offset, index))
                .collect();
            let mut pending = vec![start];
            let mut visited = BTreeSet::new();
            let mut valid = true;
            while let Some(index) = pending.pop() {
                if index >= body.instructions.len() {
                    valid = false;
                    break;
                }
                if !visited.insert(index) {
                    continue;
                }
                match &body.instructions[index].kind {
                    InlineOp::Return(_) | InlineOp::Raw { opcode: 0xbf, .. } => {}
                    InlineOp::Branch { opcode, target } => {
                        if *target < body.instructions[start].offset {
                            valid = false;
                            break;
                        }
                        pending.push(indices[target]);
                        if *opcode != 0xa7 {
                            pending.push(index + 1);
                        }
                    }
                    InlineOp::Switch { default, cases } => {
                        for target in
                            std::iter::once(default).chain(cases.iter().map(|(_, target)| target))
                        {
                            if *target < body.instructions[start].offset {
                                valid = false;
                                break;
                            }
                            pending.push(indices[target]);
                        }
                    }
                    _ => pending.push(index + 1),
                }
            }
            if valid {
                let end = visited.last().copied().unwrap() + 1;
                entry.insert((
                    body.instructions[start].offset,
                    body.instructions
                        .get(end)
                        .map_or(body.code_size, |instruction| instruction.offset),
                ));
            }
        }
    }
    regions
}

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
        let Some((label, depth)) = self.inline_returns.last().copied() else {
            return Ok(false);
        };
        self.value(value)?;
        if !terminal_value(value) {
            self.adapt_value(&value.ty, &Type::Java("java/lang/Object".into()))?;
            self.preserve_across_finalizers(&Type::Java("java/lang/Object".into()), depth)?;
            self.branch(0xa7, label);
        }
        Ok(true)
    }

    pub(super) fn compiled_inline(
        &mut self,
        method: &JavaMethod,
        args: &[Value],
        local_base: u16,
        lambda_bases: &[Option<(String, u16)>],
        reified_temp: Option<u16>,
        result: &Type,
    ) -> Result<()> {
        crate::compiled_inline::validate(method)?;
        let code = method
            .library
            .as_ref()
            .and_then(|library| library.code.as_ref())
            .ok_or_else(|| failure("Compiled inline bytecode unavailable"))?;
        let body = decode(code)?;
        let (mut params, erased_result) = invocation_types(&method.descriptor)?;
        if method.access & 8 == 0 {
            params.insert(0, Type::Java(method.owner.clone()));
        }
        if params.len() != args.len() || lambda_bases.len() != args.len() {
            return Err(failure("Compiled inline argument count mismatch"));
        }
        let mut slots = Vec::new();
        let mut slot = 0u16;
        let mut lambda_arguments = BTreeMap::new();
        for (index, param) in params.iter().enumerate() {
            slots.push(slot);
            if let Some((owner, base)) = &lambda_bases[index] {
                let declaration = self
                    .lambdas
                    .iter()
                    .find(|lambda| &lambda.owner == owner)
                    .ok_or_else(|| {
                        failure("Compiled inline lambda body unavailable in this declaration")
                    })?;
                lambda_arguments.insert(slot, (declaration.owner.clone(), *base));
            }
            slot = slot
                .checked_add(param.slots() as u16)
                .ok_or_else(|| failure("Inline parameter slot overflow"))?;
        }
        if slot > body.max_locals {
            return Err(failure("Compiled inline parameters exceed max_locals"));
        }
        let mut reified = BTreeMap::new();
        let mut skipped = BTreeSet::new();
        for (index, instruction) in body.instructions.iter().enumerate() {
            if !matches!(&instruction.kind, InlineOp::Reference { constant: InlineConstant::Method { owner, name, descriptor, .. }, .. } if owner == "kotlin/jvm/internal/Intrinsics" && name == "reifiedOperationMarker" && descriptor == "(ILjava/lang/String;)V")
            {
                continue;
            }
            if index < 2 || index + 1 >= body.instructions.len() {
                return Err(reified_unsupported("Malformed reified operation marker"));
            }
            let operation = match &body.instructions[index - 2].kind {
                InlineOp::Raw { opcode, operands }
                    if operands.is_empty() && (0x03..=0x08).contains(opcode) =>
                {
                    i32::from(*opcode - 0x03)
                }
                InlineOp::Constant(InlineConstant::Integer(value)) => *value,
                _ => {
                    return Err(reified_unsupported(
                        "Unsupported reified operation identifier",
                    ));
                }
            };
            let InlineOp::Constant(InlineConstant::String(parameter)) =
                &body.instructions[index - 1].kind
            else {
                return Err(reified_unsupported(
                    "Reified operation has no constant type parameter",
                ));
            };
            let parameter = String::from_utf16(parameter)
                .map_err(|_| reified_unsupported("Malformed reified type parameter name"))?;
            let (name, nullable) = parameter
                .strip_suffix('?')
                .map_or((parameter.as_str(), false), |name| (name, true));
            let mut ty = method
                .library
                .as_ref()
                .and_then(|library| {
                    library
                        .reified_arguments
                        .iter()
                        .find(|(parameter, _)| parameter == name)
                })
                .map(|(_, ty)| ty.clone())
                .ok_or_else(|| reified_unsupported("Reified type substitution is unavailable"))?;
            if nullable && !ty.is_nullable() {
                ty = Type::Nullable(Box::new(ty));
            }
            let following = &body.instructions[index + 1].kind;
            let shape = match operation {
                0 => matches!(
                    following,
                    InlineOp::Reference {
                        opcode: 0xbd,
                        constant: InlineConstant::Class(_),
                        ..
                    }
                ),
                1 | 2 => matches!(
                    following,
                    InlineOp::Reference {
                        opcode: 0xc0,
                        constant: InlineConstant::Class(_),
                        ..
                    }
                ),
                3 => matches!(
                    following,
                    InlineOp::Reference {
                        opcode: 0xc1,
                        constant: InlineConstant::Class(_),
                        ..
                    }
                ),
                4 => matches!(following, InlineOp::Constant(InlineConstant::Class(_))),
                _ => false,
            };
            if !shape {
                return Err(reified_unsupported(
                    "Unsupported reified operation bytecode shape",
                ));
            }
            reified.insert(body.instructions[index - 2].offset, (operation, ty));
            for instruction in &body.instructions[index - 1..=index + 1] {
                skipped.insert(instruction.offset);
            }
        }
        if body
            .instructions
            .iter()
            .any(|instruction| match &instruction.kind {
                InlineOp::Branch { target, .. } => skipped.contains(target),
                InlineOp::Switch { default, cases } => {
                    skipped.contains(default)
                        || cases.iter().any(|(_, target)| skipped.contains(target))
                }
                _ => false,
            })
        {
            return Err(reified_unsupported(
                "A branch enters a reified operation sequence",
            ));
        }
        let calls = lambda_calls(
            &body,
            &lambda_arguments.keys().copied().collect::<BTreeSet<_>>(),
        )?;
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
        let finalizers = finalizer_regions(&body);
        let exclusions = body
            .exception_handlers
            .iter()
            .map(|handler| (handler.handler, Rc::new(RefCell::new(Vec::new()))))
            .collect();
        let expansion = Rc::new(InlineExpansion {
            body,
            local_base,
            lambda_arguments,
            calls,
            reified,
            reified_temp,
            skipped,
            end,
            finalizers,
            exclusions,
        });
        self.emit_inline_region(&expansion, 0, expansion.body.code_size)?;
        self.mark(end);
        self.adapt_value(&erased_result, result)
    }

    pub(super) fn emit_inline_region(
        &mut self,
        expansion: &Rc<InlineExpansion>,
        start: usize,
        finish: usize,
    ) -> Result<()> {
        let instructions: Vec<_> = expansion
            .body
            .instructions
            .iter()
            .filter(|instruction| start <= instruction.offset && instruction.offset < finish)
            .collect();
        let mut labels = BTreeMap::new();
        for instruction in &instructions {
            labels.insert(instruction.offset, self.label());
        }
        labels.insert(finish, self.label());
        for instruction in &instructions {
            self.mark(labels[&instruction.offset]);
            if expansion.skipped.contains(&instruction.offset) {
                continue;
            }
            if let Some((operation, ty)) = expansion.reified.get(&instruction.offset) {
                self.materialize_reified_operation(*operation, ty, expansion.reified_temp)?;
                continue;
            }
            if let Some(slot) = expansion.calls.get(&instruction.offset) {
                let (owner, base) = &expansion.lambda_arguments[slot];
                let lambda = self
                    .lambdas
                    .iter()
                    .find(|lambda| &lambda.owner == owner)
                    .ok_or_else(|| failure("Compiled inline lambda body unavailable"))?;
                let depth = self.finalizers.len();
                let mut handlers: Vec<_> = expansion
                    .body
                    .exception_handlers
                    .iter()
                    .filter(|handler| {
                        handler.catch_type.is_none()
                            && handler.start <= instruction.offset
                            && instruction.offset < handler.end
                    })
                    .collect();
                handlers.sort_by_key(|handler| {
                    (
                        std::cmp::Reverse(handler.end - handler.start),
                        handler.start,
                    )
                });
                let mut seen = BTreeSet::new();
                for handler in handlers {
                    if !seen.insert(handler.handler) || self.finalizers.iter().any(|frame| matches!(&frame.body, FinallyBody::Compiled { expansion: active, handler: active_handler, .. } if Rc::ptr_eq(active, expansion) && *active_handler == handler.handler)) { continue; }
                    let Some((start, end)) = expansion.finalizers.get(&handler.handler).copied()
                    else {
                        if contains_nonlocal(&lambda.function, self.lambdas) {
                            return Err(Diagnostic::new(
                                0,
                                "Non-local return crosses an unsupported compiled finally region",
                            )
                            .with_kind(
                                crate::DiagnosticKind::UnsupportedLanguage,
                                "COMPILED_INLINE_FINALLY_UNSUPPORTED",
                            ));
                        }
                        continue;
                    };
                    self.finalizers.push(FinallyFrame {
                        body: FinallyBody::Compiled {
                            expansion: expansion.clone(),
                            handler: handler.handler,
                            start,
                            end,
                        },
                        exclusions: expansion.exclusions[&handler.handler].clone(),
                        local_maps: self.inline_local_maps.clone(),
                        inline_returns: self.inline_returns.clone(),
                        loops: self.loops.clone(),
                        loop_depths: self.loop_finally_depths.clone(),
                        loop_names: self.loop_names.clone(),
                    });
                }
                self.inline_lambda(lambda, *base)?;
                self.finalizers.truncate(depth);
                continue;
            }
            match &instruction.kind {
                InlineOp::Raw { opcode, operands } => {
                    self.bytes.push(*opcode);
                    self.bytes.extend(operands);
                }
                InlineOp::Local { opcode, slot } => {
                    let slot = self.inline_slot(
                        expansion
                            .local_base
                            .checked_add(*slot)
                            .ok_or_else(|| failure("Inline local overflow"))?,
                    )?;
                    self.bytes.extend([*opcode, slot]);
                }
                InlineOp::Increment { slot, amount } => {
                    let slot = self.inline_slot(
                        expansion
                            .local_base
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
                InlineOp::Branch { opcode, target } => self.branch(
                    *opcode,
                    *labels.get(target).ok_or_else(|| {
                        failure("Compiled finally branch escapes its expansion region")
                    })?,
                ),
                InlineOp::Switch { default, cases } => {
                    for (key, target) in cases {
                        self.bytes.push(0x59);
                        self.integer(*key)?;
                        let next = self.label();
                        self.branch(0xa0, next);
                        self.bytes.push(0x57);
                        self.branch(
                            0xa7,
                            *labels.get(target).ok_or_else(|| {
                                failure("Compiled finally switch escapes its expansion region")
                            })?,
                        );
                        self.mark(next);
                    }
                    self.bytes.push(0x57);
                    self.branch(
                        0xa7,
                        *labels.get(default).ok_or_else(|| {
                            failure("Compiled finally switch escapes its expansion region")
                        })?,
                    );
                }
                InlineOp::Return(_) => self.branch(0xa7, expansion.end),
            }
        }
        self.mark(labels[&finish]);
        for handler in &expansion.body.exception_handlers {
            if start <= handler.start
                && handler.end <= finish
                && start <= handler.handler
                && handler.handler < finish
            {
                let catch_type = handler
                    .catch_type
                    .as_ref()
                    .map(|owner| self.pool.class(owner))
                    .transpose()?
                    .unwrap_or(0);
                let exclusions = if handler.catch_type.is_none() {
                    expansion.exclusions[&handler.handler].clone()
                } else {
                    expansion
                        .body
                        .exception_handlers
                        .iter()
                        .find(|candidate| {
                            candidate.catch_type.is_none()
                                && candidate.start == handler.start
                                && candidate.end == handler.end
                        })
                        .map(|candidate| expansion.exclusions[&candidate.handler].clone())
                        .unwrap_or_default()
                };
                self.protected_ranges(
                    self.labels[labels[&handler.start]].unwrap(),
                    self.labels[labels[&handler.end]].unwrap(),
                    self.labels[labels[&handler.handler]].unwrap(),
                    catch_type,
                    &exclusions,
                );
            }
        }
        Ok(())
    }

    fn materialize_reified_operation(
        &mut self,
        operation: i32,
        ty: &Type,
        temporary: Option<u16>,
    ) -> Result<()> {
        if self.reified_marker(ty, operation)? {
            if operation == 4 {
                self.class_reference(0x13, "java/lang/Object")?;
            } else if operation == 0 {
                self.class_reference(0xbd, "java/lang/Object")?;
            } else {
                self.class_reference(if operation == 3 { 0xc1 } else { 0xc0 }, "java/lang/Object")?;
            }
            return Ok(());
        }
        if operation == 0 {
            let component = match ty.base() {
                Type::Nullable(inner) => inner.as_ref(),
                ty => ty,
            };
            let component = if component.is_reference() {
                reference_class(component)?
            } else if component == &Type::Unit {
                return Err(reified_unsupported(
                    "Reified Unit array components are unsupported",
                ));
            } else {
                boxed_owner(component).into()
            };
            return self.class_reference(0xbd, &component);
        }
        if operation == 4 {
            if !ty.is_reference() {
                return self.class_reference(0x13, boxed_owner(ty));
            }
            return self.value(&Value {
                ty: Type::Java("java/lang/Class".into()),
                kind: ValueKind::ClassLiteral(ty.clone()),
            });
        }
        let slot = temporary.ok_or_else(|| failure("Reified operation local is unavailable"))?;
        self.bytes.extend([0x3a, self.inline_slot(slot)?]);
        let operand = Box::new(Value {
            ty: Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
            kind: ValueKind::Local(
                u8::try_from(slot).map_err(|_| failure("Reified operation local overflow"))?,
            ),
        });
        if operation == 3 {
            self.value(&Value {
                ty: Type::Boolean,
                kind: ValueKind::TypeTest {
                    value: operand,
                    target: ty.clone(),
                    negated: false,
                },
            })
        } else {
            let result = if operation == 2 && !ty.is_nullable() {
                Type::Nullable(Box::new(ty.clone()))
            } else {
                ty.clone()
            };
            self.value(&Value {
                ty: result.clone(),
                kind: ValueKind::Cast {
                    value: operand,
                    target: ty.clone(),
                    safe: operation == 2,
                },
            })?;
            self.adapt_value(&result, &Type::Java("java/lang/Object".into()))
        }
    }

    fn inline_lambda(&mut self, lambda: &crate::check::LambdaDecl, base: u16) -> Result<()> {
        if lambda.interface.is_some() || lambda.anonymous_methods.is_some() {
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
        self.inline_returns.push((end, self.finalizers.len()));
        let saved_loops = std::mem::take(&mut self.loops);
        let saved_loop_depths = std::mem::take(&mut self.loop_finally_depths);
        let saved_loop_names = std::mem::take(&mut self.loop_names);
        let saved_pending_name = self.pending_loop_name.take();
        let emitted = self.block(&function.body);
        self.loops = saved_loops;
        self.loop_finally_depths = saved_loop_depths;
        self.loop_names = saved_loop_names;
        self.pending_loop_name = saved_pending_name;
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
            InlineConstant::Double(value) => {
                let index = self.pool.add(Constant::Double(*value))?;
                self.reference(0x14, index);
                Ok(())
            }
            InlineConstant::Long(value) => {
                let index = self.pool.add(Constant::Long(*value))?;
                self.reference(0x14, index);
                Ok(())
            }
            InlineConstant::Class(value) => self.class_reference(0x13, value),
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
        Statement::ThrowValue(value) => nonlocal_value(value, lambdas),
        Statement::If(value, yes, no) => {
            nonlocal_value(value, lambdas)
                || nonlocal_block(yes, lambdas)
                || nonlocal_block(no, lambdas)
        }
        Statement::While(value, body) | Statement::DoWhile(value, body) => {
            nonlocal_value(value, lambdas) || nonlocal_block(body, lambdas)
        }
        Statement::ForUntil {
            start, end, body, ..
        } => {
            nonlocal_value(start, lambdas)
                || nonlocal_value(end, lambdas)
                || nonlocal_block(body, lambdas)
        }
        Statement::ForEach { iterable, body, .. }
        | Statement::ForIterator { iterable, body, .. } => {
            nonlocal_value(iterable, lambdas) || nonlocal_block(body, lambdas)
        }
        Statement::Store(_, value) | Statement::Return(value) | Statement::Expression(value) => {
            nonlocal_value(value, lambdas)
        }
        Statement::ThrowException { message, .. } => message
            .as_ref()
            .is_some_and(|value| nonlocal_value(value, lambdas)),
        Statement::IndexStore(a, b, c) => {
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
        Statement::Continue
        | Statement::Break
        | Statement::ContinueLabel(_)
        | Statement::BreakLabel(_) => false,
        Statement::Labeled { body, .. } => nonlocal_block(body, lambdas),
    })
}

fn nonlocal_value(value: &Value, lambdas: &[crate::check::LambdaDecl]) -> bool {
    match &value.kind {
        ValueKind::Block(body, result) => {
            nonlocal_block(body, lambdas) || nonlocal_value(result, lambdas)
        }
        ValueKind::Try {
            body,
            result,
            catches,
            finally,
            ..
        } => {
            nonlocal_block(body, lambdas)
                || nonlocal_value(result, lambdas)
                || nonlocal_block(finally, lambdas)
                || catches.iter().any(|catch| {
                    nonlocal_block(&catch.body, lambdas) || nonlocal_value(&catch.result, lambdas)
                })
        }
        ValueKind::ArrayInitialize {
            size, initializer, ..
        } => nonlocal_value(size, lambdas) || nonlocal_value(initializer, lambdas),
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
        | ValueKind::CellDecrement { cell: value, .. }
        | ValueKind::TypeTest { value, .. }
        | ValueKind::Cast { value, .. }
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
        ValueKind::SuperCall { args, .. }
        | ValueKind::New { args, .. }
        | ValueKind::StaticCall { args, .. }
        | ValueKind::MemberCall { args, .. }
        | ValueKind::JavaCall { args, .. }
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
