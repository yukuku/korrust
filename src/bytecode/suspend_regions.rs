use super::*;
use std::collections::BTreeMap;

pub(super) fn gather(value: &Value, fields: &mut BTreeMap<u8, Type>) {
    if let ValueKind::Try {
        catches,
        finally,
        exception_slot,
        result_slot,
        ..
    } = &value.kind
    {
        for catch in catches {
            fields.insert(catch.slot, Type::Java(catch.owner.clone()));
        }
        if !finally.is_empty() {
            fields.insert(*exception_slot, Type::Java("java/lang/Throwable".into()));
        }
        if let Some(slot) = result_slot {
            fields.insert(*slot, value.ty.clone());
        }
    }
}

pub(super) fn field_count(body: &[Statement]) -> usize {
    let mut count = 0;
    crate::check::ir_walk::statements(&mut body.to_vec(), "", &mut |value: &mut Value, _: &str| {
        if matches!(&value.kind, ValueKind::Try { finally, .. } if !finally.is_empty()) {
            count += 1;
        }
    });
    count
}

pub(super) fn field_name(depth: usize) -> String {
    format!("pending${depth}")
}

pub(super) fn preserve(code: &mut Code<'_>, ty: &Type, depth: usize) -> Result<bool> {
    let Some(state) = code.suspension.as_ref() else {
        return Ok(false);
    };
    let suspends = code.finalizers[depth..].iter().any(|finalizer| {
        matches!(&finalizer.body, FinallyBody::Source(body)
            if super::suspend::contains_suspension(body, code.functions))
    });
    if !suspends {
        return Ok(false);
    }
    let frame = state.frame;
    let owner = state.frame_owner.clone();
    let pending_depth = state.pending_depth + 1;
    let name = field_name(pending_depth);
    let object = Type::Java("java/lang/Object".into());
    if ty.base() != &Type::Unit {
        code.adapt_value(ty, &object)?;
        code.bytes.extend([0x19, frame, 0x5f]);
        code.field_reference(0xb5, &owner, &name, "Ljava/lang/Object;")?;
    }
    code.suspension.as_mut().unwrap().pending_depth = pending_depth;
    code.emit_finalizers(depth)?;
    code.suspension.as_mut().unwrap().pending_depth -= 1;
    if ty.base() != &Type::Unit {
        code.bytes.extend([0x19, frame]);
        code.field_reference(0xb4, &owner, &name, "Ljava/lang/Object;")?;
        code.adapt_value(&object, ty)?;
    }
    Ok(true)
}
