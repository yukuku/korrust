use crate::syntax::{
    Expr, ExprKind, Function, ObjectDecl, ObjectMethod, Statement, StatementKind, Type,
};
use crate::{Diagnostic, DiagnosticKind, Result};

pub(crate) fn lower(object: &mut ObjectDecl, owner: &str) -> Result<()> {
    if !object.data_ {
        return Ok(());
    }
    for method in &object.methods {
        if (method.function.name == "hashCode" && method.function.params.is_empty())
            || (method.function.name == "equals"
                && (method.is_override
                    || method.function.params.len() == 1
                        && method.function.params[0].1.descriptor() == "Ljava/lang/Object;"))
        {
            return Err(Diagnostic::new(
                method.function.offset,
                "Data objects cannot declare equals or hashCode",
            )
            .with_kind(DiagnosticKind::InvalidKotlin, "DATA_OBJECT_CUSTOM_EQUALITY"));
        }
    }
    let hash = owner
        .replace(['/', '$'], ".")
        .encode_utf16()
        .fold(0i32, |value, unit| {
            value.wrapping_mul(31).wrapping_add(i32::from(unit))
        });
    let expression = |kind| Expr {
        offset: object.offset,
        kind,
    };
    let other = Type::Nullable(Box::new(Type::Java("java/lang/Object".into())));
    let mut definitions = vec![
        (
            "equals",
            vec![("other".into(), other)],
            Type::Boolean,
            expression(ExprKind::TypeOperation(
                "is".into(),
                Box::new(expression(ExprKind::Name("other".into()))),
                Type::Java(owner.into()),
            )),
        ),
        (
            "hashCode",
            Vec::new(),
            Type::Int,
            expression(ExprKind::Int(i64::from(hash))),
        ),
    ];
    if !object
        .methods
        .iter()
        .any(|method| method.function.name == "toString" && method.function.params.is_empty())
    {
        definitions.push((
            "toString",
            Vec::new(),
            Type::String,
            expression(ExprKind::String(object.name.encode_utf16().collect())),
        ));
    }
    for (name, params, result, value) in definitions {
        let count = params.len();
        object.methods.push(ObjectMethod {
            synthesized: true,
            is_override: true,
            jvm_static: false,
            throws: Vec::new(),
            inline: false,
            suppress: Vec::new(),
            function: Function {
                inline: false,
                parameter_modes: Vec::new(),
                param_annotations: vec![Vec::new(); count],
                annotations: Vec::new(),
                name: name.into(),
                offset: object.offset,
                private: false,
                protected: false,
                internal: false,
                infix: false,
                operator: false,
                vararg: false,
                receiver: None,
                type_params: Vec::new(),
                params,
                defaults: vec![None; count],
                result,
                result_inferred: false,
                body: vec![Statement {
                    offset: object.offset,
                    kind: StatementKind::Return(Some(value)),
                }],
            },
        });
    }
    Ok(())
}
