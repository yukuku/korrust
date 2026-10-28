use crate::classpath::{Classpath, JavaConstant, JavaField};
use crate::syntax::{
    AnnotationUse, ClassDecl, CompanionValue, ConstDecl, ConstValue, Declaration, Expr, ExprKind,
    Function, InterfaceDecl, Module, ObjectDecl, Type,
};
use crate::{Diagnostic, DiagnosticKind, Result};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

#[derive(Clone, Debug)]
enum Value {
    Constant(ConstValue),
    Null,
}

fn unsupported(offset: usize, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message).with_kind(
        DiagnosticKind::UnsupportedLanguage,
        "CONSTANT_EXPRESSION_UNSUPPORTED",
    )
}

fn invalid(offset: usize, message: &str) -> Diagnostic {
    Diagnostic::new(offset, message)
        .with_kind(DiagnosticKind::InvalidKotlin, "INVALID_CONSTANT_EXPRESSION")
}

fn numeric_literal(value: i128, long: bool, offset: usize) -> Result<ConstValue> {
    if !long && let Ok(value) = i32::try_from(value) {
        return Ok(ConstValue::Int(value));
    }
    i64::try_from(value)
        .map(ConstValue::Long)
        .map_err(|_| invalid(offset, "Integer constant is out of range"))
}

fn constant_string_size(units: &[u16], offset: usize) -> Result<usize> {
    let mut size = 0;
    for unit in units {
        size += match unit {
            1..=127 => 1,
            0 | 128..=2047 => 2,
            _ => 3,
        };
        if size > 65535 {
            return Err(unsupported(
                offset,
                "Constant string exceeds the JVM modified UTF-8 limit",
            )
            .with_kind(
                DiagnosticKind::UnsupportedLanguage,
                "CONSTANT_STRING_LIMIT_UNSUPPORTED",
            ));
        }
    }
    Ok(size)
}

fn extend_constant_string(target: &mut Vec<u16>, units: &[u16], offset: usize) -> Result<()> {
    if constant_string_size(target, offset)? + constant_string_size(units, offset)? > 65535 {
        return Err(unsupported(
            offset,
            "Constant string exceeds the JVM modified UTF-8 limit",
        )
        .with_kind(
            DiagnosticKind::UnsupportedLanguage,
            "CONSTANT_STRING_LIMIT_UNSUPPORTED",
        ));
    }
    target.extend_from_slice(units);
    Ok(())
}

fn validate_constant(value: &ConstValue, offset: usize) -> Result<()> {
    if let ConstValue::String(units) = value {
        constant_string_size(units, offset)?;
    }
    Ok(())
}

fn string_units(value: &Value, offset: usize) -> Result<Vec<u16>> {
    let text = match value {
        Value::Null => "null".into(),
        Value::Constant(ConstValue::String(units)) => return Ok(units.clone()),
        Value::Constant(ConstValue::Char(unit)) => return Ok(vec![*unit]),
        Value::Constant(ConstValue::Int(value)) => value.to_string(),
        Value::Constant(ConstValue::Long(value)) => value.to_string(),
        Value::Constant(ConstValue::Boolean(value)) => value.to_string(),
        Value::Constant(ConstValue::Float(_) | ConstValue::Double(_)) => {
            return Err(unsupported(
                offset,
                "Floating-point formatting in constant strings is unsupported",
            ));
        }
        Value::Constant(ConstValue::Expression { .. }) => {
            return Err(unsupported(offset, "Unresolved constant initializer"));
        }
    };
    Ok(text.encode_utf16().collect())
}

fn numeric_rank(value: &ConstValue) -> Option<u8> {
    match value {
        ConstValue::Int(_) => Some(0),
        ConstValue::Long(_) => Some(1),
        ConstValue::Float(_) => Some(2),
        ConstValue::Double(_) => Some(3),
        _ => None,
    }
}

fn as_double(value: &ConstValue) -> f64 {
    match value {
        ConstValue::Int(value) => f64::from(*value),
        ConstValue::Long(value) => *value as f64,
        ConstValue::Float(bits) => f64::from(f32::from_bits(*bits)),
        ConstValue::Double(bits) => f64::from_bits(*bits),
        _ => unreachable!(),
    }
}

fn as_float(value: &ConstValue) -> f32 {
    match value {
        ConstValue::Int(value) => *value as f32,
        ConstValue::Long(value) => *value as f32,
        ConstValue::Float(bits) => f32::from_bits(*bits),
        _ => unreachable!(),
    }
}

fn as_long(value: &ConstValue) -> i64 {
    match value {
        ConstValue::Int(value) => i64::from(*value),
        ConstValue::Long(value) => *value,
        _ => unreachable!(),
    }
}

fn arithmetic(
    operator: &str,
    left: &ConstValue,
    right: &ConstValue,
    offset: usize,
    evaluate: bool,
) -> Result<ConstValue> {
    let rank = numeric_rank(left)
        .zip(numeric_rank(right))
        .map(|(left, right)| left.max(right))
        .ok_or_else(|| {
            invalid(
                offset,
                "Constant arithmetic operands have incompatible types",
            )
        })?;
    if !matches!(operator, "+" | "-" | "*" | "/" | "%") {
        return Err(unsupported(offset, "This constant operator is unsupported"));
    }
    if rank == 3 {
        let (left, right) = (as_double(left), as_double(right));
        let result = if !evaluate {
            0.0
        } else {
            match operator {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                "/" => left / right,
                "%" => left % right,
                _ => unreachable!(),
            }
        };
        return Ok(ConstValue::Double(result.to_bits()));
    }
    if rank == 2 {
        let (left, right) = (as_float(left), as_float(right));
        let result = if !evaluate {
            0.0
        } else {
            match operator {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                "/" => left / right,
                "%" => left % right,
                _ => unreachable!(),
            }
        };
        return Ok(ConstValue::Float(result.to_bits()));
    }
    let (left, right) = (as_long(left), as_long(right));
    if evaluate && matches!(operator, "/" | "%") && right == 0 {
        return Err(invalid(
            offset,
            "Cannot evaluate integer division or remainder by zero",
        ));
    }
    if rank == 1 {
        let result = if !evaluate {
            0
        } else {
            match operator {
                "+" => left.wrapping_add(right),
                "-" => left.wrapping_sub(right),
                "*" => left.wrapping_mul(right),
                "/" => left.wrapping_div(right),
                "%" => left.wrapping_rem(right),
                _ => unreachable!(),
            }
        };
        Ok(ConstValue::Long(result))
    } else {
        let (left, right) = (left as i32, right as i32);
        let result = if !evaluate {
            0
        } else {
            match operator {
                "+" => left.wrapping_add(right),
                "-" => left.wrapping_sub(right),
                "*" => left.wrapping_mul(right),
                "/" => left.wrapping_div(right),
                "%" => left.wrapping_rem(right),
                _ => unreachable!(),
            }
        };
        Ok(ConstValue::Int(result))
    }
}

fn comparison(
    operator: &str,
    left: &ConstValue,
    right: &ConstValue,
    offset: usize,
) -> Result<bool> {
    let equality = matches!(operator, "==" | "!=");
    if equality && std::mem::discriminant(left) != std::mem::discriminant(right) {
        return Err(invalid(
            offset,
            "Constant equality operands have incompatible types",
        ));
    }
    let ordering = match (left, right) {
        (ConstValue::Int(left), ConstValue::Int(right)) => Some(left.cmp(right)),
        (ConstValue::Long(left), ConstValue::Long(right)) => Some(left.cmp(right)),
        (ConstValue::Int(left), ConstValue::Long(right)) => Some(i64::from(*left).cmp(right)),
        (ConstValue::Long(left), ConstValue::Int(right)) => Some(left.cmp(&i64::from(*right))),
        (ConstValue::Char(left), ConstValue::Char(right)) => Some(left.cmp(right)),
        (ConstValue::Boolean(left), ConstValue::Boolean(right)) => Some(left.cmp(right)),
        (ConstValue::String(left), ConstValue::String(right)) => Some(left.cmp(right)),
        _ => {
            let rank = numeric_rank(left)
                .zip(numeric_rank(right))
                .map(|(left, right)| left.max(right))
                .ok_or_else(|| {
                    invalid(
                        offset,
                        "Constant comparison operands have incompatible types",
                    )
                })?;
            if rank == 2 {
                as_float(left).partial_cmp(&as_float(right))
            } else {
                as_double(left).partial_cmp(&as_double(right))
            }
        }
    };
    Ok(match operator {
        "==" => ordering == Some(std::cmp::Ordering::Equal),
        "!=" => ordering != Some(std::cmp::Ordering::Equal),
        "<" => ordering == Some(std::cmp::Ordering::Less),
        ">" => ordering == Some(std::cmp::Ordering::Greater),
        "<=" => matches!(
            ordering,
            Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
        ),
        ">=" => matches!(
            ordering,
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ),
        _ => {
            return Err(unsupported(
                offset,
                "This constant comparison is unsupported",
            ));
        }
    })
}

fn name_path(expression: &Expr) -> Option<Vec<String>> {
    match &expression.kind {
        ExprKind::Name(name) => Some(vec![name.clone()]),
        ExprKind::Property(receiver, name) => {
            let mut path = name_path(receiver)?;
            path.push(name.clone());
            Some(path)
        }
        _ => None,
    }
}

fn evaluate(
    expression: &Expr,
    resolve: &mut dyn FnMut(&[String], usize) -> Result<ConstValue>,
    enabled: bool,
    depth: &Cell<usize>,
) -> Result<Value> {
    let previous = depth.get();
    if previous >= 128 {
        return Err(unsupported(
            expression.offset,
            "Constant expression evaluation depth exceeds the supported limit",
        ));
    }
    depth.set(previous + 1);
    let result = evaluate_inner(expression, resolve, enabled, depth);
    depth.set(previous);
    result
}

fn evaluate_inner(
    expression: &Expr,
    resolve: &mut dyn FnMut(&[String], usize) -> Result<ConstValue>,
    enabled: bool,
    depth: &Cell<usize>,
) -> Result<Value> {
    let offset = expression.offset;
    let constant = match &expression.kind {
        ExprKind::ClassLiteral { .. } => {
            return Err(invalid(
                offset,
                "Class literals cannot be compile-time scalar constants",
            ));
        }
        ExprKind::Int(value) => numeric_literal(i128::from(*value), false, offset)?,
        ExprKind::Long(value) => numeric_literal(*value, true, offset)?,
        ExprKind::Float(value) => ConstValue::Float(*value),
        ExprKind::Double(value) => ConstValue::Double(*value),
        ExprKind::Char(value) => ConstValue::Char(*value),
        ExprKind::Boolean(value) => ConstValue::Boolean(*value),
        ExprKind::String(value) => ConstValue::String(value.clone()),
        ExprKind::Null => return Ok(Value::Null),
        ExprKind::Name(_) | ExprKind::Property(_, _) => {
            let path = name_path(expression).ok_or_else(|| {
                unsupported(
                    offset,
                    "Computed property access in constant expressions is unsupported",
                )
            })?;
            resolve(&path, offset)?
        }
        ExprKind::Unary(operator, value) => {
            if operator == "-" {
                match value.kind {
                    ExprKind::Int(number) => {
                        return Ok(Value::Constant(numeric_literal(
                            -i128::from(number),
                            false,
                            offset,
                        )?));
                    }
                    ExprKind::Long(number) => {
                        if number > i128::from(i64::MAX) {
                            return Err(invalid(offset, "Long constant literal is out of range"));
                        }
                        return Ok(Value::Constant(numeric_literal(-number, true, offset)?));
                    }
                    _ => {}
                }
            }
            let Value::Constant(value) = evaluate(value, resolve, enabled, depth)? else {
                return Err(invalid(
                    offset,
                    "Unary constant operators require nonnull operands",
                ));
            };
            match (operator.as_str(), value) {
                (
                    "+",
                    value @ (ConstValue::Int(_)
                    | ConstValue::Long(_)
                    | ConstValue::Float(_)
                    | ConstValue::Double(_)),
                ) => value,
                ("-", ConstValue::Int(value)) => ConstValue::Int(value.wrapping_neg()),
                ("-", ConstValue::Long(value)) => ConstValue::Long(value.wrapping_neg()),
                ("-", ConstValue::Float(bits)) => {
                    ConstValue::Float((-f32::from_bits(bits)).to_bits())
                }
                ("-", ConstValue::Double(bits)) => {
                    ConstValue::Double((-f64::from_bits(bits)).to_bits())
                }
                ("!", ConstValue::Boolean(value)) => ConstValue::Boolean(!value),
                _ => {
                    return Err(invalid(
                        offset,
                        "Unary constant operator has an incompatible operand",
                    ));
                }
            }
        }
        ExprKind::Binary(operator, left, right) if matches!(operator.as_str(), "&&" | "||") => {
            let Value::Constant(ConstValue::Boolean(left)) =
                evaluate(left, resolve, enabled, depth)?
            else {
                return Err(invalid(
                    offset,
                    "Logical constant operators require Boolean operands",
                ));
            };
            let right_enabled = enabled && if operator == "&&" { left } else { !left };
            let Value::Constant(ConstValue::Boolean(right)) =
                evaluate(right, resolve, right_enabled, depth)?
            else {
                return Err(invalid(
                    offset,
                    "Logical constant operators require Boolean operands",
                ));
            };
            ConstValue::Boolean(if operator == "&&" {
                left && right
            } else {
                left || right
            })
        }
        ExprKind::Binary(operator, left, right) => {
            let left = evaluate(left, resolve, enabled, depth)?;
            let right = evaluate(right, resolve, enabled, depth)?;
            if operator == "+" && matches!(left, Value::Constant(ConstValue::String(_))) {
                if matches!(right, Value::Null) {
                    return Err(invalid(
                        offset,
                        "String concatenation with null is not a const initializer",
                    ));
                }
                let mut units = string_units(&left, offset)?;
                extend_constant_string(&mut units, &string_units(&right, offset)?, offset)?;
                ConstValue::String(units)
            } else {
                let (Value::Constant(left), Value::Constant(right)) = (left, right) else {
                    return Err(invalid(
                        offset,
                        "Null operations are not constant property initializers",
                    ));
                };
                match (operator.as_str(), &left, &right) {
                    ("+", ConstValue::Char(left), ConstValue::Int(right)) => {
                        ConstValue::Char(left.wrapping_add(*right as u16))
                    }
                    ("-", ConstValue::Char(left), ConstValue::Int(right)) => {
                        ConstValue::Char(left.wrapping_sub(*right as u16))
                    }
                    ("-", ConstValue::Char(left), ConstValue::Char(right)) => {
                        ConstValue::Int(i32::from(*left) - i32::from(*right))
                    }
                    ("==" | "!=" | "<" | ">" | "<=" | ">=", _, _) => {
                        ConstValue::Boolean(comparison(operator, &left, &right, offset)?)
                    }
                    _ => arithmetic(operator, &left, &right, offset, enabled)?,
                }
            }
        }
        ExprKind::Template(parts) => {
            let mut units = Vec::new();
            for part in parts {
                extend_constant_string(
                    &mut units,
                    &string_units(&evaluate(part, resolve, enabled, depth)?, part.offset)?,
                    part.offset,
                )?;
            }
            ConstValue::String(units)
        }
        _ => {
            return Err(unsupported(
                offset,
                "This expression is unsupported in constant initializers",
            ));
        }
    };
    Ok(Value::Constant(constant))
}

pub(crate) fn fold_expression(
    expression: &Expr,
    resolve: &mut dyn FnMut(&[String], usize) -> Result<ConstValue>,
) -> Result<ConstValue> {
    fold_with_budget(expression, resolve, &Cell::new(0))
}

fn fold_with_budget(
    expression: &Expr,
    resolve: &mut dyn FnMut(&[String], usize) -> Result<ConstValue>,
    depth: &Cell<usize>,
) -> Result<ConstValue> {
    match evaluate(expression, resolve, true, depth)? {
        Value::Constant(value) => {
            validate_constant(&value, expression.offset)?;
            Ok(value)
        }
        Value::Null => Err(invalid(
            expression.offset,
            "Const properties cannot have null values",
        )),
    }
}

fn integer_literal_expression(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Int(_) => true,
        ExprKind::Unary(operator, value) if matches!(operator.as_str(), "+" | "-") => {
            integer_literal_expression(value)
        }
        ExprKind::Binary(operator, left, right)
            if matches!(operator.as_str(), "+" | "-" | "*" | "/" | "%") =>
        {
            integer_literal_expression(left) && integer_literal_expression(right)
        }
        _ => false,
    }
}

fn declared_value(
    value: ConstValue,
    declared: &Option<Type>,
    offset: usize,
    literal_coercion: bool,
) -> Result<ConstValue> {
    match (declared, value) {
        (None, value) => Ok(value),
        (Some(Type::Int), value @ ConstValue::Int(_))
        | (Some(Type::Long), value @ ConstValue::Long(_))
        | (Some(Type::Float), value @ ConstValue::Float(_))
        | (Some(Type::Double), value @ ConstValue::Double(_))
        | (Some(Type::Boolean), value @ ConstValue::Boolean(_))
        | (Some(Type::Char), value @ ConstValue::Char(_))
        | (Some(Type::String), value @ ConstValue::String(_)) => Ok(value),
        (Some(Type::Long), ConstValue::Int(value)) if literal_coercion => {
            Ok(ConstValue::Long(i64::from(value)))
        }
        (Some(Type::Byte), _) => Err(unsupported(
            offset,
            "Byte const property declarations are unsupported",
        )),
        (
            Some(
                Type::Int
                | Type::Long
                | Type::Float
                | Type::Double
                | Type::Boolean
                | Type::Char
                | Type::String,
            ),
            _,
        ) => Err(invalid(
            offset,
            "Const initializer type does not match its declared type",
        )),
        (Some(_), _) => Err(unsupported(
            offset,
            "This declared const type is unsupported, including unresolved type aliases",
        )),
    }
}

#[derive(Clone)]
struct Entry {
    module: usize,
    scope: String,
    declaration: ConstDecl,
}

#[derive(Clone, Copy)]
enum BindingKind {
    Value,
    Object,
    Parameter,
    EnumEntry,
}

struct Binding {
    module: usize,
    scope: String,
    private: bool,
    kind: BindingKind,
}

type Bindings = HashMap<String, Vec<Binding>>;

fn bind(
    bindings: &mut Bindings,
    module: usize,
    scope: &str,
    name: &str,
    private: bool,
    kind: BindingKind,
) {
    bindings
        .entry(qualified(scope, name))
        .or_default()
        .push(Binding {
            module,
            scope: scope.into(),
            private,
            kind,
        });
}

#[derive(Clone, Copy)]
struct LookupContext<'a> {
    module: usize,
    scope: &'a str,
    offset: usize,
    initializer: bool,
}

struct ModuleContext {
    package: String,
    imports: Vec<(String, String, usize)>,
}

struct Resolver<'a> {
    modules: Vec<ModuleContext>,
    classpath: &'a Classpath,
    entries: Vec<Entry>,
    names: HashMap<String, Vec<usize>>,
    bindings: Bindings,
    inherited_scopes: HashSet<String>,
    states: Vec<u8>,
    values: Vec<Option<ConstValue>>,
    error_module: Option<usize>,
    evaluation_depth: Rc<Cell<usize>>,
}

fn qualified(scope: &str, name: &str) -> String {
    if scope.is_empty() {
        name.into()
    } else {
        format!("{scope}.{name}")
    }
}

fn collect_object(
    object: &ObjectDecl,
    scope: &str,
    module: usize,
    entries: &mut Vec<Entry>,
    bindings: &mut Bindings,
    inherited_scopes: &mut HashSet<String>,
) {
    bind(
        bindings,
        module,
        scope,
        &object.name,
        false,
        BindingKind::Object,
    );
    let scope = qualified(scope, &object.name);
    if !object.interfaces.is_empty() {
        inherited_scopes.insert(scope.clone());
    }
    for field in &object.fields {
        bind(
            bindings,
            module,
            &scope,
            &field.name,
            field.private,
            BindingKind::Value,
        );
    }
    for declaration in &object.constants {
        entries.push(Entry {
            module,
            scope: scope.clone(),
            declaration: declaration.clone(),
        });
    }
    for object in &object.objects {
        collect_object(object, &scope, module, entries, bindings, inherited_scopes);
    }
    for declaration in &object.nested {
        collect_declaration(
            declaration,
            &scope,
            module,
            entries,
            bindings,
            inherited_scopes,
        );
    }
}

fn collect_class(
    class: &ClassDecl,
    scope: &str,
    module: usize,
    entries: &mut Vec<Entry>,
    bindings: &mut Bindings,
    inherited_scopes: &mut HashSet<String>,
) {
    let outer = scope;
    let scope = qualified(scope, &class.name);
    if class.super_class.is_some() || !class.interfaces.is_empty() {
        inherited_scopes.insert(scope.clone());
    }
    for property in &class.properties {
        bind(
            bindings,
            module,
            &scope,
            &property.name,
            property.private,
            BindingKind::Value,
        );
    }
    for name in &class.param_properties {
        bind(bindings, module, &scope, name, false, BindingKind::Value);
    }
    let header_scope = qualified(&scope, "$constructor");
    for (name, _) in &class.params {
        bind(
            bindings,
            module,
            &header_scope,
            name,
            false,
            BindingKind::Parameter,
        );
    }
    if let Some(companion) = &class.companion {
        bind(
            bindings,
            module,
            outer,
            &class.name,
            false,
            BindingKind::Object,
        );
        let companion_scope = qualified(&scope, "Companion");
        for property in &companion.properties {
            bind(
                bindings,
                module,
                &scope,
                &property.name,
                property.private,
                BindingKind::Value,
            );
            bind(
                bindings,
                module,
                &companion_scope,
                &property.name,
                property.private,
                BindingKind::Value,
            );
        }
    }
    for declaration in &class.nested {
        collect_declaration(
            declaration,
            &scope,
            module,
            entries,
            bindings,
            inherited_scopes,
        );
    }
}

fn collect_interface(
    interface: &InterfaceDecl,
    scope: &str,
    module: usize,
    entries: &mut Vec<Entry>,
    bindings: &mut Bindings,
    inherited_scopes: &mut HashSet<String>,
) {
    let scope = qualified(scope, &interface.name);
    if !interface.interfaces.is_empty() {
        inherited_scopes.insert(scope.clone());
    }
    for property in &interface.properties {
        bind(
            bindings,
            module,
            &scope,
            &property.name,
            false,
            BindingKind::Value,
        );
    }
    if let Some(companion) = &interface.companion {
        let companion_scope = qualified(&scope, "Companion");
        for property in &companion.properties {
            bind(
                bindings,
                module,
                &scope,
                &property.name,
                property.private,
                BindingKind::Value,
            );
            bind(
                bindings,
                module,
                &companion_scope,
                &property.name,
                property.private,
                BindingKind::Value,
            );
        }
        for field in &companion.fields {
            bind(
                bindings,
                module,
                &scope,
                &field.name,
                field.private,
                BindingKind::Value,
            );
            bind(
                bindings,
                module,
                &companion_scope,
                &field.name,
                field.private,
                BindingKind::Value,
            );
        }
    }
    for interface in &interface.nested {
        collect_interface(
            interface,
            &scope,
            module,
            entries,
            bindings,
            inherited_scopes,
        );
    }
    for declaration in &interface.declarations {
        collect_declaration(
            declaration,
            &scope,
            module,
            entries,
            bindings,
            inherited_scopes,
        );
    }
}

fn collect_enum(
    value: &crate::syntax::EnumDecl,
    scope: &str,
    module: usize,
    entries: &mut Vec<Entry>,
    bindings: &mut Bindings,
    inherited_scopes: &mut HashSet<String>,
) {
    collect_class(
        &value.body,
        scope,
        module,
        entries,
        bindings,
        inherited_scopes,
    );
    let scope = qualified(scope, &value.name);
    inherited_scopes.insert(scope.clone());
    for (name, _) in &value.entries {
        bind(
            bindings,
            module,
            &scope,
            name,
            false,
            BindingKind::EnumEntry,
        );
    }
}

fn collect_declaration(
    declaration: &Declaration,
    scope: &str,
    module: usize,
    entries: &mut Vec<Entry>,
    bindings: &mut Bindings,
    inherited_scopes: &mut HashSet<String>,
) {
    match declaration {
        Declaration::Object(value) => {
            collect_object(value, scope, module, entries, bindings, inherited_scopes)
        }
        Declaration::Class(value) => {
            collect_class(value, scope, module, entries, bindings, inherited_scopes)
        }
        Declaration::Enum(value) => {
            collect_enum(value, scope, module, entries, bindings, inherited_scopes)
        }
        Declaration::Interface(value) => {
            collect_interface(value, scope, module, entries, bindings, inherited_scopes)
        }
    }
}

impl Resolver<'_> {
    fn entry(&mut self, index: usize) -> Result<ConstValue> {
        let result = (|| {
            if self.states[index] == 2 {
                return Ok(self.values[index].as_ref().unwrap().clone());
            }
            if self.states[index] == 1 {
                return Err(invalid(
                    self.entries[index].declaration.offset,
                    "Cyclic constant initializer",
                ));
            }
            if self.states.iter().filter(|state| **state == 1).count() >= 128 {
                return Err(unsupported(
                    self.entries[index].declaration.offset,
                    "Constant reference nesting exceeds the supported limit",
                ));
            }
            self.states[index] = 1;
            let entry = self.entries[index].clone();
            let value = match &entry.declaration.value {
                ConstValue::Expression {
                    expression,
                    declared_type,
                } => {
                    let depth = Rc::clone(&self.evaluation_depth);
                    let value = fold_with_budget(
                        expression,
                        &mut |path, offset| {
                            self.reference(entry.module, &entry.scope, path, offset, true)
                        },
                        &depth,
                    )?;
                    declared_value(
                        value,
                        declared_type,
                        expression.offset,
                        integer_literal_expression(expression),
                    )?
                }
                value => value.clone(),
            };
            validate_constant(&value, entry.declaration.offset)?;
            self.values[index] = Some(value.clone());
            self.states[index] = 2;
            Ok(value)
        })();
        if result.is_err() {
            self.states[index] = 0;
            if self.error_module.is_none() {
                self.error_module = Some(self.entries[index].module);
            }
        }
        result
    }

    fn source_reference(
        &mut self,
        module: usize,
        scope: &str,
        name: &str,
        offset: usize,
        initializer: bool,
    ) -> Result<Option<ConstValue>> {
        let Some(indices) = self.names.get(name).cloned() else {
            return Ok(None);
        };
        let package = &self.modules[module].package;
        let available: Vec<_> = indices
            .into_iter()
            .filter(|index| {
                let entry = &self.entries[*index];
                !entry.declaration.private
                    || (entry.module == module
                        && (entry.scope == *package
                            || scope == entry.scope
                            || scope.starts_with(&format!("{}.", entry.scope))))
            })
            .collect();
        let same_file: Vec<_> = available
            .iter()
            .copied()
            .filter(|index| self.entries[*index].module == module)
            .collect();
        let selected = if same_file.is_empty() {
            &available
        } else {
            &same_file
        };
        match selected.as_slice() {
            [] => Err(invalid(
                offset,
                "Private constant is inaccessible from this source or declaration",
            )),
            [index] => {
                let entry = &self.entries[*index];
                if initializer
                    && entry.module == module
                    && entry.scope == scope
                    && entry.declaration.offset > offset
                {
                    return Err(invalid(
                        offset,
                        "Constant property must be initialized before use in this declaration scope",
                    ));
                }
                self.entry(*index).map(Some)
            }
            _ => Err(invalid(offset, "Ambiguous constant reference")),
        }
    }

    fn library_field(&self, field: &JavaField, offset: usize) -> Result<ConstValue> {
        let class = self.classpath.load(&field.owner)?;
        if class.kotlin_class
            || class
                .library
                .as_ref()
                .is_some_and(|library| library.package.is_some())
        {
            return Err(
                unsupported(offset, "Compiled Kotlin constant visibility is unsupported")
                    .with_kind(
                        DiagnosticKind::UnsupportedLanguage,
                        "COMPILED_KOTLIN_CONSTANT_UNSUPPORTED",
                    ),
            );
        }
        if class.access & 1 == 0 || field.access & 0x19 != 0x19 {
            return Err(invalid(
                offset,
                "Constant field must be accessible, static and final",
            ));
        }
        if class.unsupported || field.unsupported {
            return Err(unsupported(
                offset,
                "Constant field carries unsupported metadata",
            ));
        }
        match (&field.constant, field.descriptor.as_str()) {
            (Some(JavaConstant::Int(value)), "I") => Ok(ConstValue::Int(*value)),
            (Some(JavaConstant::Int(value @ (0 | 1))), "Z") => Ok(ConstValue::Boolean(*value == 1)),
            (Some(JavaConstant::Int(_)), "Z") => {
                Err(invalid(offset, "Boolean ConstantValue must be zero or one"))
            }
            (Some(JavaConstant::Int(value)), "C") => u16::try_from(*value)
                .map(ConstValue::Char)
                .map_err(|_| invalid(offset, "Char ConstantValue is out of range")),
            (Some(JavaConstant::Long(value)), "J") => Ok(ConstValue::Long(*value)),
            (Some(JavaConstant::Float(value)), "F") => Ok(ConstValue::Float(*value)),
            (Some(JavaConstant::Double(value)), "D") => Ok(ConstValue::Double(*value)),
            (Some(JavaConstant::String(value)), "Ljava/lang/String;") => {
                Ok(ConstValue::String(value.clone()))
            }
            (Some(_), "B" | "S") => Err(unsupported(
                offset,
                "Byte and Short constant fields are unsupported",
            )),
            _ => Err(unsupported(
                offset,
                "Field does not declare a scalar compile-time ConstantValue",
            )),
        }
    }

    fn library_reference(&self, name: &str, offset: usize) -> Result<Option<ConstValue>> {
        let Some((owner, field_name)) = name.rsplit_once('.') else {
            return Ok(None);
        };
        if let Ok(class) = self.classpath.load(&owner.replace('.', "/")) {
            if let Some(field) = class.fields.iter().find(|field| field.name == field_name) {
                return self.library_field(field, offset).map(Some);
            }
        }
        let fields = self.classpath.package_constants(owner, field_name)?;
        match fields.as_slice() {
            [] => Ok(None),
            [field] => self.library_field(field, offset).map(Some),
            _ => Err(invalid(offset, "Ambiguous imported constant field")),
        }
    }

    fn nested_annotation(&self, module: usize, expression: &Expr) -> bool {
        let ExprKind::Call(target, _) = &expression.kind else {
            return false;
        };
        let Some(path) = name_path(target) else {
            return false;
        };
        let name = path.join(".");
        let mut names = vec![name.replace('.', "/")];
        for (alias, import, _) in &self.modules[module].imports {
            if alias == &path[0] {
                let suffix = path[1..].join("/");
                names.push(if suffix.is_empty() {
                    import.clone()
                } else {
                    format!("{import}/{suffix}")
                });
            }
        }
        names.into_iter().any(|name| {
            self.classpath
                .load(&name)
                .is_ok_and(|class| class.annotation_definition.is_some())
        })
    }

    fn source_path(
        &mut self,
        context: &LookupContext<'_>,
        candidate: &str,
        head: &str,
        qualified_path: bool,
    ) -> Result<Option<ConstValue>> {
        let LookupContext {
            module,
            scope,
            offset,
            initializer,
        } = *context;
        let visible = |binding: &&Binding| {
            !binding.private
                || binding.scope != self.modules[module].package
                || binding.module == module
        };
        if let Some(binding) = self
            .bindings
            .get(candidate)
            .and_then(|values| values.iter().find(visible))
        {
            return Err(match binding.kind {
                BindingKind::Value => invalid(offset, "Reference resolves to a non-const property"),
                BindingKind::Parameter => unsupported(
                    offset,
                    "Constructor-parameter-dependent constant annotation values are unsupported",
                ),
                BindingKind::Object => invalid(offset, "Object values are not scalar constants"),
                BindingKind::EnumEntry => unsupported(
                    offset,
                    "Enum values are unsupported scalar constant references",
                ),
            });
        }
        if let Some(binding) = self
            .bindings
            .get(head)
            .and_then(|values| values.iter().find(visible))
        {
            match binding.kind {
                BindingKind::Value => {
                    return Err(invalid(
                        offset,
                        "Reference is shadowed by a non-const property",
                    ));
                }
                BindingKind::Parameter => {
                    return Err(unsupported(
                        offset,
                        "Constructor-parameter-dependent constant annotation values are unsupported",
                    ));
                }
                BindingKind::EnumEntry => {
                    return Err(unsupported(
                        offset,
                        "Enum values are unsupported scalar constant references",
                    ));
                }
                BindingKind::Object if qualified_path => {
                    return self
                        .source_reference(module, scope, candidate, offset, initializer)?
                        .map(Some)
                        .ok_or_else(|| {
                            unsupported(
                                offset,
                                "This source object member is unsupported as a scalar constant",
                            )
                        });
                }
                BindingKind::Object => {
                    return Err(invalid(offset, "Object values are not scalar constants"));
                }
            }
        }
        if let Some(value) = self.source_reference(module, scope, candidate, offset, initializer)? {
            return Ok(Some(value));
        }
        if qualified_path
            && self
                .source_reference(module, scope, head, offset, initializer)?
                .is_some()
        {
            return Err(unsupported(
                offset,
                "Property access on a scalar constant is unsupported",
            ));
        }
        Ok(None)
    }

    fn reference(
        &mut self,
        module: usize,
        scope: &str,
        path: &[String],
        offset: usize,
        initializer: bool,
    ) -> Result<ConstValue> {
        let context = LookupContext {
            module,
            scope,
            offset,
            initializer,
        };
        let name = path.join(".");
        let package = self.modules[module].package.clone();
        let mut current = Some(scope);
        while let Some(candidate_scope) = current {
            if candidate_scope == package {
                break;
            }
            let candidate = qualified(candidate_scope, &name);
            let head = qualified(candidate_scope, &path[0]);
            if let Some(value) = self.source_path(&context, &candidate, &head, path.len() > 1)? {
                return Ok(value);
            }
            if self.inherited_scopes.contains(candidate_scope) {
                return Err(unsupported(
                    offset,
                    "Inherited value-name lookup in constant expressions is unsupported",
                ));
            }
            current = candidate_scope
                .rsplit_once('.')
                .map(|(parent, _)| parent)
                .or(Some(""));
        }
        let imports = self.modules[module].imports.clone();
        for (alias, import, _) in imports.iter().filter(|(alias, _, _)| alias != "*") {
            if alias != &path[0] {
                continue;
            }
            let imported = import.replace('/', ".");
            let candidate = qualified(&imported, &path[1..].join("."))
                .trim_end_matches('.')
                .to_owned();
            if let Some(value) =
                self.source_path(&context, &candidate, &imported, path.len() > 1)?
            {
                return Ok(value);
            }
            if let Some(value) = self.library_reference(&candidate, offset)? {
                return Ok(value);
            }
            return Err(unsupported(
                offset,
                "Explicit import does not resolve to a supported constant declaration",
            ));
        }
        let candidate = qualified(&package, &name);
        let head = qualified(&package, &path[0]);
        if let Some(value) = self.source_path(&context, &candidate, &head, path.len() > 1)? {
            return Ok(value);
        }
        let mut star_values = Vec::new();
        for (_, import, _) in imports.iter().filter(|(alias, _, _)| alias == "*") {
            let imported = import.trim_end_matches("/*").replace('/', ".");
            let candidate = qualified(&imported, &name);
            let head = qualified(&imported, &path[0]);
            if let Some(value) = self.source_path(&context, &candidate, &head, path.len() > 1)? {
                star_values.push(value);
            } else if let Some(value) = self.library_reference(&candidate, offset)? {
                star_values.push(value);
            }
        }
        if star_values.len() > 1 {
            return Err(invalid(
                offset,
                "Ambiguous star-imported constant reference",
            ));
        }
        if let Some(value) = star_values.pop() {
            return Ok(value);
        }
        if let Some(value) = self.source_path(&context, &name, &path[0], path.len() > 1)? {
            return Ok(value);
        }
        if let Some(value) = self.library_reference(&name, offset)? {
            return Ok(value);
        }
        if path.len() == 1 && !imports.iter().any(|(alias, _, _)| alias == "*") {
            return Err(invalid(offset, "Unresolved constant reference"));
        }
        Err(unsupported(
            offset,
            "Reference cannot be resolved to a supported constant declaration",
        ))
    }
}

fn update_object(object: &mut ObjectDecl, values: &HashMap<usize, ConstValue>) {
    for constant in &mut object.constants {
        constant.value = values[&constant.offset].clone();
    }
    for object in &mut object.objects {
        update_object(object, values);
    }
    for declaration in &mut object.nested {
        update_declaration(declaration, values);
    }
}

fn update_class(class: &mut ClassDecl, values: &HashMap<usize, ConstValue>) {
    for declaration in &mut class.nested {
        update_declaration(declaration, values);
    }
}

fn update_interface(interface: &mut InterfaceDecl, values: &HashMap<usize, ConstValue>) {
    for interface in &mut interface.nested {
        update_interface(interface, values);
    }
    for declaration in &mut interface.declarations {
        update_declaration(declaration, values);
    }
}

fn update_declaration(declaration: &mut Declaration, values: &HashMap<usize, ConstValue>) {
    match declaration {
        Declaration::Object(value) => update_object(value, values),
        Declaration::Class(value) => update_class(value, values),
        Declaration::Enum(value) => update_class(&mut value.body, values),
        Declaration::Interface(value) => update_interface(value, values),
    }
}

pub(crate) fn resolve(
    modules: &mut [Module],
    classpath: &Classpath,
) -> std::result::Result<(), (usize, Diagnostic)> {
    let mut entries = Vec::new();
    let mut bindings = Bindings::new();
    let mut inherited_scopes = HashSet::new();
    for (module, source) in modules.iter().enumerate() {
        for property in &source.properties {
            bind(
                &mut bindings,
                module,
                &source.package,
                &property.name,
                property.private,
                BindingKind::Value,
            );
        }
        for declaration in &source.constants {
            entries.push(Entry {
                module,
                scope: source.package.clone(),
                declaration: declaration.clone(),
            });
        }
        if let Some(value) = &source.object_decl {
            collect_object(
                value,
                &source.package,
                module,
                &mut entries,
                &mut bindings,
                &mut inherited_scopes,
            );
        }
        if let Some(value) = &source.class_decl {
            collect_class(
                value,
                &source.package,
                module,
                &mut entries,
                &mut bindings,
                &mut inherited_scopes,
            );
        }
        if let Some(value) = &source.enum_decl {
            collect_enum(
                value,
                &source.package,
                module,
                &mut entries,
                &mut bindings,
                &mut inherited_scopes,
            );
        }
        if let Some(value) = &source.interface_decl {
            collect_interface(
                value,
                &source.package,
                module,
                &mut entries,
                &mut bindings,
                &mut inherited_scopes,
            );
        }
        for declaration in &source.declarations {
            collect_declaration(
                declaration,
                &source.package,
                module,
                &mut entries,
                &mut bindings,
                &mut inherited_scopes,
            );
        }
    }
    let mut names: HashMap<String, Vec<usize>> = HashMap::new();
    for (index, entry) in entries.iter().enumerate() {
        names
            .entry(qualified(&entry.scope, &entry.declaration.name))
            .or_default()
            .push(index);
    }
    let mut resolver = Resolver {
        modules: modules
            .iter()
            .map(|module| ModuleContext {
                package: module.package.clone(),
                imports: module.imports.clone(),
            })
            .collect(),
        classpath,
        states: vec![0; entries.len()],
        values: vec![None; entries.len()],
        error_module: None,
        evaluation_depth: Rc::new(Cell::new(0)),
        entries,
        names,
        bindings,
        inherited_scopes,
    };
    for index in 0..resolver.entries.len() {
        let module = resolver.entries[index].module;
        if let Err(error) = resolver.entry(index) {
            return Err((resolver.error_module.unwrap_or(module), error));
        }
    }
    let resolved: Vec<HashMap<usize, ConstValue>> = modules
        .iter()
        .enumerate()
        .map(|(module, _)| {
            resolver
                .entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| entry.module == module)
                .map(|(index, entry)| {
                    (
                        entry.declaration.offset,
                        resolver.values[index].as_ref().unwrap().clone(),
                    )
                })
                .collect()
        })
        .collect();
    for (source, values) in modules.iter_mut().zip(resolved) {
        for constant in &mut source.constants {
            constant.value = values[&constant.offset].clone();
        }
        if let Some(value) = &mut source.object_decl {
            update_object(value, &values);
        }
        if let Some(value) = &mut source.class_decl {
            update_class(value, &values);
        }
        if let Some(value) = &mut source.enum_decl {
            update_class(&mut value.body, &values);
        }
        if let Some(value) = &mut source.interface_decl {
            update_interface(value, &values);
        }
        for declaration in &mut source.declarations {
            update_declaration(declaration, &values);
        }
    }
    for (module, source) in modules.iter_mut().enumerate() {
        let package = source.package.clone();
        visit_module_annotations(source, &package, &mut |scope, annotation| {
            for (_, expression) in &mut annotation.arguments {
                if !scalar_annotation_shape(expression)
                    || resolver.nested_annotation(module, expression)
                {
                    continue;
                }
                let value = match fold_expression(expression, &mut |path, offset| {
                    resolver.reference(module, scope, path, offset, false)
                }) {
                    Ok(value) => value,
                    Err(error)
                        if error.kind == DiagnosticKind::UnsupportedLanguage
                            && matches!(
                                expression.kind,
                                ExprKind::Name(_) | ExprKind::Property(_, _)
                            )
                            && error.code != "COMPILED_KOTLIN_CONSTANT_UNSUPPORTED" =>
                    {
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                expression.kind = literal_kind(value, expression.offset)?;
            }
            Ok(())
        })
        .map_err(|error| (module, error))?;
    }
    Ok(())
}

fn literal_kind(value: ConstValue, offset: usize) -> Result<ExprKind> {
    Ok(match value {
        ConstValue::Int(value) => ExprKind::Int(i64::from(value)),
        ConstValue::Long(value) => ExprKind::Long(i128::from(value)),
        ConstValue::Float(value) => ExprKind::Float(value),
        ConstValue::Double(value) => ExprKind::Double(value),
        ConstValue::Boolean(value) => ExprKind::Boolean(value),
        ConstValue::Char(value) => ExprKind::Char(value),
        ConstValue::String(value) => ExprKind::String(value),
        ConstValue::Expression { .. } => {
            return Err(unsupported(
                offset,
                "Unresolved annotation constant initializer",
            ));
        }
    })
}

fn scalar_annotation_shape(expression: &Expr) -> bool {
    match &expression.kind {
        ExprKind::Int(_)
        | ExprKind::Long(_)
        | ExprKind::Float(_)
        | ExprKind::Double(_)
        | ExprKind::Char(_)
        | ExprKind::MissingWhenElse
        | ExprKind::WhenSubjectless
        | ExprKind::Boolean(_)
        | ExprKind::String(_)
        | ExprKind::Null
        | ExprKind::Name(_)
        | ExprKind::Call(_, _) => true,
        ExprKind::Property(_, _) => name_path(expression).is_some(),
        ExprKind::Unary(_, value) => scalar_annotation_shape(value),
        ExprKind::Binary(_, left, right) => {
            scalar_annotation_shape(left) && scalar_annotation_shape(right)
        }
        ExprKind::Template(parts) => parts.iter().all(scalar_annotation_shape),
        _ => false,
    }
}

type AnnotationVisitor<'a> = dyn FnMut(&str, &mut AnnotationUse) -> Result<()> + 'a;

fn visit_annotations(
    values: &mut [AnnotationUse],
    scope: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    for value in values {
        visit(scope, value)?;
    }
    Ok(())
}

fn visit_function_annotations(
    function: &mut Function,
    scope: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    visit_annotations(&mut function.annotations, scope, visit)?;
    for values in &mut function.param_annotations {
        visit_annotations(values, scope, visit)?;
    }
    Ok(())
}

fn visit_class_annotations(
    class: &mut ClassDecl,
    outer: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    visit_annotations(&mut class.annotations, outer, visit)?;
    let scope = qualified(outer, &class.name);
    let header_scope = qualified(&scope, "$constructor");
    for values in &mut class.param_annotations {
        visit_annotations(values, &header_scope, visit)?;
    }
    for property in &mut class.properties {
        visit_annotations(&mut property.annotations, &scope, visit)?;
    }
    for method in &mut class.methods {
        visit_function_annotations(&mut method.function, &scope, visit)?;
    }
    if let Some(companion) = &mut class.companion {
        let scope = qualified(&scope, "Companion");
        for property in &mut companion.properties {
            visit_annotations(&mut property.annotations, &scope, visit)?;
        }
        for method in &mut companion.methods {
            visit_function_annotations(&mut method.function, &scope, visit)?;
        }
    }
    for declaration in &mut class.nested {
        visit_declaration_annotations(declaration, &scope, visit)?;
    }
    Ok(())
}

fn visit_object_annotations(
    object: &mut ObjectDecl,
    outer: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    visit_annotations(&mut object.annotation_uses, outer, visit)?;
    let scope = qualified(outer, &object.name);
    for constant in &mut object.constants {
        visit_annotations(&mut constant.annotations, &scope, visit)?;
    }
    for field in &mut object.fields {
        visit_annotations(&mut field.annotations, &scope, visit)?;
    }
    for method in &mut object.methods {
        visit_function_annotations(&mut method.function, &scope, visit)?;
    }
    for object in &mut object.objects {
        visit_object_annotations(object, &scope, visit)?;
    }
    for declaration in &mut object.nested {
        visit_declaration_annotations(declaration, &scope, visit)?;
    }
    Ok(())
}

fn visit_interface_annotations(
    interface: &mut InterfaceDecl,
    outer: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    visit_annotations(&mut interface.annotations, outer, visit)?;
    let scope = qualified(outer, &interface.name);
    for property in &mut interface.properties {
        visit_annotations(&mut property.annotations, &scope, visit)?;
    }
    for method in &mut interface.methods {
        visit_function_annotations(&mut method.function, &scope, visit)?;
    }
    if let Some(companion) = &mut interface.companion {
        let scope = qualified(&scope, "Companion");
        for property in &mut companion.properties {
            visit_annotations(&mut property.annotations, &scope, visit)?;
            if let CompanionValue::Object(class) = &mut property.value {
                visit_class_annotations(class, &scope, visit)?;
            }
        }
        for field in &mut companion.fields {
            visit_annotations(&mut field.annotations, &scope, visit)?;
        }
        for method in &mut companion.methods {
            visit_function_annotations(&mut method.function, &scope, visit)?;
        }
    }
    for interface in &mut interface.nested {
        visit_interface_annotations(interface, &scope, visit)?;
    }
    for declaration in &mut interface.declarations {
        visit_declaration_annotations(declaration, &scope, visit)?;
    }
    Ok(())
}

fn visit_declaration_annotations(
    declaration: &mut Declaration,
    scope: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    match declaration {
        Declaration::Object(value) => visit_object_annotations(value, scope, visit),
        Declaration::Class(value) => visit_class_annotations(value, scope, visit),
        Declaration::Interface(value) => visit_interface_annotations(value, scope, visit),
        Declaration::Enum(value) => {
            visit_annotations(&mut value.annotations, scope, visit)?;
            visit_class_annotations(&mut value.body, scope, visit)
        }
    }
}

fn visit_module_annotations(
    module: &mut Module,
    scope: &str,
    visit: &mut AnnotationVisitor<'_>,
) -> Result<()> {
    for function in &mut module.functions {
        visit_function_annotations(function, scope, visit)?;
    }
    for constant in &mut module.constants {
        visit_annotations(&mut constant.annotations, scope, visit)?;
    }
    for property in &mut module.properties {
        visit_annotations(&mut property.annotations, scope, visit)?;
    }
    if let Some(value) = &mut module.object_decl {
        visit_object_annotations(value, scope, visit)?;
    }
    if let Some(value) = &mut module.class_decl {
        visit_class_annotations(value, scope, visit)?;
    }
    if let Some(value) = &mut module.interface_decl {
        visit_interface_annotations(value, scope, visit)?;
    }
    if let Some(value) = &mut module.enum_decl {
        visit_annotations(&mut value.annotations, scope, visit)?;
        visit_class_annotations(&mut value.body, scope, visit)?;
    }
    for declaration in &mut module.declarations {
        visit_declaration_annotations(declaration, scope, visit)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fold(source: &str) -> Result<ConstValue> {
        let mut modules = vec![crate::syntax::parse(&format!(
            "const val VALUE = {source}"
        ))?];
        let classpath = Classpath::new(&[])?;
        resolve(&mut modules, &classpath).map_err(|(_, error)| error)?;
        Ok(modules.remove(0).constants.remove(0).value)
    }

    #[test]
    fn class_literals_are_invalid_scalar_constant_initializers() {
        let error = fold("String::class").unwrap_err();
        assert_eq!(error.kind, crate::DiagnosticKind::InvalidKotlin);
        assert_eq!(error.code, "INVALID_CONSTANT_EXPRESSION");
    }

    #[test]
    fn scalar_constant_operations_preserve_wrapping_types_and_utf16() {
        for (source, expected) in [
            ("2 * 21", 42),
            ("2147483647 + 1", i32::MIN),
            ("-2147483648 / -1", i32::MIN),
            ("-2147483648 % -1", 0),
            ("'z' - 'a'", 25),
        ] {
            assert!(
                matches!(fold(source).unwrap(), ConstValue::Int(value) if value == expected),
                "{source}"
            );
        }
        assert!(matches!(
            fold("9223372036854775807L + 1L").unwrap(),
            ConstValue::Long(i64::MIN)
        ));
        assert!(matches!(
            fold("'a' + 2147483647").unwrap(),
            ConstValue::Char(96)
        ));
        assert!(
            matches!(fold("1.25f + 2").unwrap(), ConstValue::Float(bits) if bits == 3.25f32.to_bits())
        );
        assert!(
            matches!(fold("1.5 + 2L").unwrap(), ConstValue::Double(bits) if bits == 3.5f64.to_bits())
        );
        assert!(
            matches!(fold("0.0 / 0.0").unwrap(), ConstValue::Double(bits) if f64::from_bits(bits).is_nan())
        );
        assert!(
            matches!(fold(r#""a${1 + 2}${'\uD800'}${null}""#).unwrap(), ConstValue::String(units) if units == [97, 51, 0xd800, 110, 117, 108, 108])
        );
    }

    #[test]
    fn short_circuit_checks_constant_operands_without_dividing_skipped_values() {
        assert!(matches!(
            fold("false && (1 / 0 > 1)").unwrap(),
            ConstValue::Boolean(false)
        ));
        assert!(matches!(
            fold("true || (1 / 0 > 1)").unwrap(),
            ConstValue::Boolean(true)
        ));
        assert_eq!(
            fold("true && (1 / 0 > 1)").unwrap_err().kind,
            DiagnosticKind::InvalidKotlin
        );
        assert_eq!(
            fold("false && 3").unwrap_err().kind,
            DiagnosticKind::InvalidKotlin
        );
        assert_eq!(
            fold("false && runtime()").unwrap_err().kind,
            DiagnosticKind::UnsupportedLanguage
        );
        assert_eq!(
            fold("1 == 1L").unwrap_err().kind,
            DiagnosticKind::InvalidKotlin
        );
    }

    #[test]
    fn constant_resolution_preserves_lexical_shadowing_and_explicit_import_priority() {
        let classpath = Classpath::new(&[]).unwrap();
        for source in [
            "const val X = 1\nobject Holder { val X = 2; const val BAD = X }",
            "const val X = 1\nobject Holder { val X = 2; @Marker(X) fun method() {} }",
            "const val X = 1\nclass Holder(val X: Int) { @Marker(X) fun method() {} }",
            "object X { const val VALUE = 1 }\nobject Holder { val X = 2; const val BAD = X.VALUE }",
        ] {
            let mut modules = vec![crate::syntax::parse(source).unwrap()];
            assert_eq!(
                resolve(&mut modules, &classpath).unwrap_err().1.kind,
                DiagnosticKind::InvalidKotlin,
                "{source}"
            );
        }
        let mut enums = vec![
            crate::syntax::parse("const val X = 1\nenum class E { X; @Marker(X) fun method() {} }")
                .unwrap(),
        ];
        resolve(&mut enums, &classpath).unwrap();
        assert!(
            matches!(&enums[0].enum_decl.as_ref().unwrap().body.methods[0].function.annotations[0].arguments[0].1.kind, ExprKind::Name(name) if name == "X")
        );
        let mut modules = vec![
            crate::syntax::parse("package imported\nconst val X = 7").unwrap(),
            crate::syntax::parse("package same\nconst val X = 1").unwrap(),
            crate::syntax::parse("package same\nimport imported.X\nconst val CHOICE = X + 1\nobject Holder { const val X = 2; const val CHOICE = X + 1 }").unwrap(),
            crate::syntax::parse("package samefile\nimport imported.X\nconst val X = 1\nconst val CHOICE = X + 1").unwrap(),
        ];
        resolve(&mut modules, &classpath).unwrap();
        assert!(matches!(modules[2].constants[0].value, ConstValue::Int(8)));
        assert!(matches!(
            modules[2].object_decl.as_ref().unwrap().constants[1].value,
            ConstValue::Int(3)
        ));
        assert!(matches!(modules[3].constants[1].value, ConstValue::Int(8)));
    }

    #[test]
    fn absent_source_names_are_invalid_without_claiming_external_resolution() {
        for source in ["missing", "missing + 1", r#""$missing""#] {
            let error = fold(source).unwrap_err();
            assert_eq!(error.kind, DiagnosticKind::InvalidKotlin, "{source}");
            assert_eq!(error.code, "INVALID_CONSTANT_EXPRESSION");
            assert!(error.message.contains("Unresolved constant reference"));
        }
        let classpath = Classpath::new(&[]).unwrap();
        for source in [
            "import dependency.*\nconst val VALUE = missing",
            "import dependency.VALUE\nconst val COPY = VALUE",
            "const val VALUE = External.MEMBER",
        ] {
            let mut modules = vec![crate::syntax::parse(source).unwrap()];
            assert_eq!(
                resolve(&mut modules, &classpath).unwrap_err().1.kind,
                DiagnosticKind::UnsupportedLanguage,
                "{source}"
            );
        }
    }

    #[test]
    fn computed_string_growth_is_bounded_before_large_allocations() {
        let mut source = "const val A0 = \"x\"\n".to_owned();
        for index in 1..=30 {
            source.push_str(&format!(
                "const val A{index} = A{} + A{}\n",
                index - 1,
                index - 1
            ));
        }
        let classpath = Classpath::new(&[]).unwrap();
        let mut modules = vec![crate::syntax::parse(&source).unwrap()];
        let (_, error) = resolve(&mut modules, &classpath).unwrap_err();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert_eq!(error.code, "CONSTANT_STRING_LIMIT_UNSUPPORTED");
        let expression = Expr {
            offset: 0,
            kind: ExprKind::String(vec![0; 32767]),
        };
        assert!(
            fold_expression(&expression, &mut |_, offset| Err(unsupported(
                offset,
                "Unexpected reference"
            )))
            .is_ok()
        );
        let expression = Expr {
            offset: 0,
            kind: ExprKind::String(vec![0; 32768]),
        };
        assert_eq!(
            fold_expression(&expression, &mut |_, offset| Err(unsupported(
                offset,
                "Unexpected reference"
            )))
            .unwrap_err()
            .code,
            "CONSTANT_STRING_LIMIT_UNSUPPORTED"
        );
    }

    #[test]
    fn constant_reference_and_expression_depth_are_bounded_across_owner_scopes() {
        fn source(count: usize) -> String {
            let mut source = String::new();
            for index in 0..count {
                source.push_str(&format!(
                    "object Node{index} {{ const val VALUE: Int = Node{}.VALUE + 1 }}\n",
                    index + 1
                ));
            }
            source.push_str(&format!(
                "object Node{count} {{ const val VALUE: Int = 1 }}"
            ));
            source
        }
        let classpath = Classpath::new(&[]).unwrap();
        let mut modules = vec![crate::syntax::parse(&source(10)).unwrap()];
        resolve(&mut modules, &classpath).unwrap();
        assert!(matches!(
            modules[0].object_decl.as_ref().unwrap().constants[0].value,
            ConstValue::Int(11)
        ));
        let mut modules = vec![crate::syntax::parse(&source(160)).unwrap()];
        let (_, error) = resolve(&mut modules, &classpath).unwrap_err();
        assert_eq!(error.kind, DiagnosticKind::UnsupportedLanguage);
        assert!(error.message.contains("depth") || error.message.contains("nesting"));
    }

    #[test]
    fn source_constants_resolve_forward_and_qualified_references_with_declared_types() {
        let mut modules = vec![crate::syntax::parse("package example\nconst val LATER = 2\nconst val DECLARED: Long = LATER + 1L\nconst val QUALIFIED = Owner.VALUE + 1\nobject Owner { const val VALUE = DECLARED + 3 }").unwrap()];
        let classpath = Classpath::new(&[]).unwrap();
        resolve(&mut modules, &classpath).unwrap();
        assert!(matches!(modules[0].constants[1].value, ConstValue::Long(3)));
        for source in [
            "const val BASE = 1\nconst val BAD: Long = BASE",
            "const val BASE = 1\nconst val BAD: Long = BASE + 1",
        ] {
            let mut invalid_modules = vec![crate::syntax::parse(source).unwrap()];
            assert_eq!(
                resolve(&mut invalid_modules, &classpath)
                    .unwrap_err()
                    .1
                    .kind,
                DiagnosticKind::InvalidKotlin
            );
        }

        assert!(matches!(
            modules[0].object_decl.as_ref().unwrap().constants[0].value,
            ConstValue::Long(6)
        ));
    }

    #[test]
    fn private_references_cycles_and_cross_file_error_locations_fail_closed() {
        let classpath = Classpath::new(&[]).unwrap();
        let mut modules = vec![
            crate::syntax::parse("package example\nconst val OUTER = INNER + 1").unwrap(),
            crate::syntax::parse("package example\nconst val INNER = 1 / 0").unwrap(),
        ];
        let (module, error) = resolve(&mut modules, &classpath).unwrap_err();
        assert_eq!(module, 1);
        assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
        let mut modules = vec![
            crate::syntax::parse("package example\nprivate const val SECRET = 1").unwrap(),
            crate::syntax::parse("package example\nconst val COPY = SECRET + 1").unwrap(),
        ];
        assert_eq!(
            resolve(&mut modules, &classpath).unwrap_err().1.kind,
            DiagnosticKind::InvalidKotlin
        );
        let mut modules = vec![
            crate::syntax::parse("const val FIRST = SECOND + 1\nconst val SECOND = FIRST + 1")
                .unwrap(),
        ];
        assert_eq!(
            resolve(&mut modules, &classpath).unwrap_err().1.kind,
            DiagnosticKind::InvalidKotlin
        );
        let mut modules = vec![crate::syntax::parse("object Owner { private const val SECRET = 1; const val VISIBLE = SECRET + 1 }\nconst val COPY = Owner.SECRET + 1").unwrap()];
        assert_eq!(
            resolve(&mut modules, &classpath).unwrap_err().1.kind,
            DiagnosticKind::InvalidKotlin
        );
    }
}
