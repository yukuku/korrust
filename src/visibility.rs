use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CheckedUnit, Diagnostic, DiagnosticKind,
    check::{self, Function, Statement, Value, ValueKind, ir_walk},
    classpath::{Classpath, JavaMethod},
    syntax::{self, Type},
};

type Key = (String, String, String);
type Failure = (String, Diagnostic);

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum BridgeTarget {
    Call {
        owner: String,
        name: String,
        descriptor: String,
    },
    FieldGet {
        owner: String,
        name: String,
        ty: Type,
    },
    FieldSet {
        owner: String,
        name: String,
        ty: Type,
    },
}

pub(crate) struct Bridge {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) descriptor: String,
    pub(crate) params: Vec<(String, Type)>,
    pub(crate) result: Type,
    pub(crate) target: BridgeTarget,
}

#[derive(Clone)]
struct FunctionInfo {
    owner: String,
    name: String,
    descriptor: String,
    params: Vec<(String, Type)>,
    result: Type,
    is_static: bool,
    defaults: bool,
    mask_count: usize,
}

impl FunctionInfo {
    fn new(function: &Function, owner: &str, is_static: bool) -> Self {
        Self {
            owner: owner.into(),
            name: function.name.clone(),
            descriptor: function.descriptor(),
            params: function.params.clone(),
            result: function.result.clone(),
            is_static,
            defaults: function.defaults.iter().any(Option::is_some),
            mask_count: (function.params.len()
                - usize::from(
                    function
                        .params
                        .first()
                        .is_some_and(|(name, _)| name == "<this>"),
                ))
            .div_ceil(32),
        }
    }

    fn key(&self) -> Key {
        (
            self.owner.clone(),
            self.name.clone(),
            self.descriptor.clone(),
        )
    }
}

#[derive(Clone)]
struct FieldInfo {
    ty: Type,
    private: bool,
}

struct Planner<'a> {
    cp: &'a Classpath,
    names: BTreeMap<Key, String>,
    methods: BTreeMap<Key, JavaMethod>,
    functions: BTreeMap<Key, FunctionInfo>,
    virtual_accessors: BTreeSet<(String, String)>,
    fields: BTreeMap<(String, String), FieldInfo>,
    occupied: BTreeSet<Key>,
    bridges: BTreeMap<Key, Bridge>,
    current_owner: String,
    current_functions: Vec<FunctionInfo>,
    failure: Option<Failure>,
}

fn clash(key: &Key) -> Failure {
    (
        key.0.clone(),
        Diagnostic::new(
            0,
            format!("JVM signature clash: {}.{}{}", key.0, key.1, key.2),
        )
        .with_kind(DiagnosticKind::InvalidKotlin, "JVM_SIGNATURE_CLASH"),
    )
}

fn default_descriptor(
    owner: &str,
    descriptor: &str,
    is_static: bool,
    constructor: bool,
    mask_count: usize,
) -> String {
    let marker = if constructor {
        "Lkotlin/jvm/internal/DefaultConstructorMarker;"
    } else {
        "Ljava/lang/Object;"
    };
    let descriptor = descriptor.replacen(')', &format!("{}{marker})", "I".repeat(mask_count)), 1);
    if is_static || constructor {
        descriptor
    } else {
        format!("(L{owner};{}", &descriptor[1..])
    }
}

fn visit_functions(
    module: &mut check::Module,
    owner: &str,
    mut visitor: impl FnMut(&mut Function, &str, bool),
) {
    fn companion(
        node: &mut check::ClassCompanion,
        visitor: &mut impl FnMut(&mut Function, &str, bool),
    ) {
        for function in &mut node.methods {
            visitor(function, &node.owner, false);
        }
    }
    fn class(
        node: &mut check::ClassDecl,
        owner: &str,
        visitor: &mut impl FnMut(&mut Function, &str, bool),
    ) {
        for function in &mut node.methods {
            visitor(function, owner, false);
        }
        if let Some(node) = &mut node.companion {
            companion(node, visitor);
        }
    }
    fn interface(
        node: &mut check::InterfaceDecl,
        visitor: &mut impl FnMut(&mut Function, &str, bool),
    ) {
        for method in &mut node.methods {
            visitor(&mut method.function, &node.owner, false);
        }
        for child in &mut node.nested {
            interface(child, visitor);
        }
        if let Some(node) = &mut node.companion {
            if let Some(state) = &mut node.state {
                companion(state, visitor);
            }
            for property in &mut node.properties {
                if let check::CompanionValue::Object { owner, class_decl } = &mut property.value {
                    class(class_decl, owner, visitor);
                }
            }
        }
    }
    for function in &mut module.functions {
        visitor(function, owner, true);
    }
    if let Some(node) = &mut module.class_decl {
        class(node, owner, &mut visitor);
    }
    if let Some(node) = &mut module.interface_decl {
        interface(node, &mut visitor);
    }
    if let Some(node) = &mut module.object_decl {
        for method in &mut node.methods {
            visitor(&mut method.function, &node.owner, method.jvm_static);
        }
    }
}

fn enum_functions(
    node: &mut check::enums::CheckedEnum,
    owner: &str,
    mut visitor: impl FnMut(&mut Function, &str, bool),
) {
    for function in &mut node.class.methods {
        visitor(function, owner, false);
    }
    if let Some(node) = &mut node.class.companion {
        for function in &mut node.methods {
            visitor(function, &node.owner, false);
        }
    }
}

impl Planner<'_> {
    fn reserve(&mut self, key: Key) -> std::result::Result<(), Failure> {
        if self.occupied.insert(key.clone()) {
            Ok(())
        } else {
            Err(clash(&key))
        }
    }

    fn physical(&self, key: &Key) -> String {
        self.names
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.1.clone())
    }

    fn bridge(&mut self, bridge: Bridge) -> std::result::Result<Key, Failure> {
        let key = (
            bridge.owner.clone(),
            bridge.name.clone(),
            bridge.descriptor.clone(),
        );
        if let Some(previous) = self.bridges.get(&key) {
            if previous.target == bridge.target {
                return Ok(key);
            }
            return Err(clash(&key));
        }
        self.reserve(key.clone())?;
        self.bridges.insert(key.clone(), bridge);
        Ok(key)
    }

    fn field_bridge(
        &mut self,
        owner: &str,
        name: &str,
        store: bool,
    ) -> std::result::Result<Option<Key>, Failure> {
        let Some(field) = self.fields.get(&(owner.into(), name.into())).cloned() else {
            return Ok(None);
        };
        if !field.private || !self.cp.same_source_file(&self.current_owner, owner) {
            return Ok(None);
        }
        let result = if store { Type::Unit } else { field.ty.clone() };
        let params = if store {
            vec![("value".into(), field.ty.clone())]
        } else {
            Vec::new()
        };
        let descriptor = format!(
            "({}){}",
            params
                .iter()
                .map(|(_, ty)| ty.descriptor())
                .collect::<String>(),
            result.descriptor()
        );
        self.bridge(Bridge {
            owner: owner.into(),
            name: format!(
                "access${}$p",
                if store {
                    check::setter_name(name)
                } else {
                    check::getter_name(name)
                }
            ),
            descriptor,
            params,
            result,
            target: if store {
                BridgeTarget::FieldSet {
                    owner: owner.into(),
                    name: name.into(),
                    ty: field.ty,
                }
            } else {
                BridgeTarget::FieldGet {
                    owner: owner.into(),
                    name: name.into(),
                    ty: field.ty,
                }
            },
        })
        .map(Some)
    }

    fn call_bridge(
        &mut self,
        key: &Key,
        caller: &str,
    ) -> std::result::Result<Option<Key>, Failure> {
        if caller == key.0 {
            return Ok(None);
        }
        let Some(method) = self.methods.get(key).cloned() else {
            return Ok(None);
        };
        if method.access & 8 == 0 || !self.cp.file_private_visible(&self.current_owner, &method) {
            return Ok(None);
        }
        if self
            .virtual_accessors
            .contains(&(key.0.clone(), key.1.clone()))
        {
            if let Some(property) = &method.source_property {
                return self.field_bridge(
                    &key.0,
                    property,
                    method.name == check::setter_name(property),
                );
            }
        }
        let info = self.functions.get(key).cloned();
        let (params, result) = if let Some(info) = info {
            (info.params, info.result)
        } else {
            let (mut params, result) =
                check::class_method_types(&method).map_err(|error| (key.0.clone(), error))?;
            if let Some(receiver) = method
                .library
                .as_ref()
                .and_then(|library| library.receiver.as_ref())
            {
                params.insert(0, receiver.clone());
            }
            (
                params
                    .into_iter()
                    .enumerate()
                    .map(|(index, ty)| (format!("p{index}"), ty))
                    .collect(),
                result,
            )
        };
        self.bridge(Bridge {
            owner: key.0.clone(),
            name: format!("access${}", self.physical(key)),
            descriptor: key.2.clone(),
            params,
            result,
            target: BridgeTarget::Call {
                owner: key.0.clone(),
                name: self.physical(key),
                descriptor: key.2.clone(),
            },
        })
        .map(Some)
    }

    fn static_call(key: Key, args: Vec<Value>) -> ValueKind {
        ValueKind::StaticCall {
            owner: key.0,
            name: key.1,
            descriptor: key.2,
            args,
        }
    }

    fn rewrite(&mut self, value: &mut Value, caller: &str) -> std::result::Result<(), Failure> {
        match &mut value.kind {
            ValueKind::Call(id, args) => {
                let Some(info) = self.current_functions.get(*id).cloned() else {
                    return Ok(());
                };
                let key = info.key();
                if let Some(bridge) = self.call_bridge(&key, caller)? {
                    value.kind = Self::static_call(bridge, std::mem::take(args));
                } else if caller != info.owner || self.physical(&key) != info.name {
                    let physical = (key.0.clone(), self.physical(&key), key.2.clone());
                    value.kind = Self::static_call(physical, std::mem::take(args));
                }
            }
            ValueKind::DefaultCall(id, args, mask) => {
                let Some(info) = self.current_functions.get(*id).cloned() else {
                    return Ok(());
                };
                let key = info.key();
                if caller != info.owner || self.physical(&key) != info.name {
                    let mut values = std::mem::take(args);
                    for index in 0..info.mask_count {
                        values.push(Value {
                            ty: Type::Int,
                            kind: ValueKind::Int(if index == 0 { *mask as i32 } else { 0 }),
                        });
                    }
                    values.push(Value {
                        ty: Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
                        kind: ValueKind::Null,
                    });
                    value.kind = Self::static_call(
                        (
                            info.owner.clone(),
                            format!("{}$default", self.physical(&key)),
                            default_descriptor(
                                &info.owner,
                                &info.descriptor,
                                info.is_static,
                                false,
                                info.mask_count,
                            ),
                        ),
                        values,
                    );
                }
            }
            ValueKind::StaticCall {
                owner,
                name,
                descriptor,
                args,
            } => {
                let key = (owner.clone(), name.clone(), descriptor.clone());
                if let Some(bridge) = self.call_bridge(&key, caller)? {
                    value.kind = Self::static_call(bridge, std::mem::take(args));
                } else {
                    *name = self.physical(&key);
                }
            }
            ValueKind::JavaCall { method, args } => {
                let key = (
                    method.owner.clone(),
                    method.name.clone(),
                    method.descriptor.clone(),
                );
                if let Some(bridge) = self.call_bridge(&key, caller)? {
                    value.kind = Self::static_call(bridge, std::mem::take(args));
                } else {
                    method.name = self.physical(&key);
                }
            }
            ValueKind::MemberCall { method, .. }
            | ValueKind::SuperCall { method, .. }
            | ValueKind::CompiledInline { method, .. } => {
                let key = (
                    method.owner.clone(),
                    method.name.clone(),
                    method.descriptor.clone(),
                );
                method.name = self.physical(&key);
            }
            ValueKind::StaticField { owner, name } if caller != owner => {
                if let Some(bridge) = self.field_bridge(owner, name, false)? {
                    value.kind = Self::static_call(bridge, Vec::new());
                }
            }
            ValueKind::JavaField {
                owner,
                name,
                receiver: None,
            } if caller != owner => {
                if let Some(bridge) = self.field_bridge(owner, name, false)? {
                    value.kind = Self::static_call(bridge, Vec::new());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ir_walk::Visitor for Planner<'_> {
    fn value(&mut self, node: &mut Value, owner: &str) {
        if self.failure.is_none() {
            if let Err(error) = self.rewrite(node, owner) {
                self.failure = Some(error);
            }
        }
    }

    fn statement(&mut self, node: &mut Statement, caller: &str) {
        if self.failure.is_some() {
            return;
        }
        if let Statement::JavaFieldStore {
            owner,
            name,
            receiver: None,
            value,
            ..
        } = node
        {
            if caller != owner {
                match self.field_bridge(owner, name, true) {
                    Ok(Some(key)) => {
                        *node = Statement::Expression(Value {
                            ty: Type::Unit,
                            kind: Self::static_call(key, vec![value.clone()]),
                        })
                    }
                    Ok(None) => {}
                    Err(error) => self.failure = Some(error),
                }
            }
        }
    }
}

pub(crate) fn plan(
    units: &[(String, syntax::Module)],
    checked: &mut [CheckedUnit],
    cp: &Classpath,
) -> std::result::Result<Vec<Bridge>, Failure> {
    let unit_owners = units
        .iter()
        .map(|(owner, _)| owner.as_str())
        .collect::<BTreeSet<_>>();
    let mut planner = Planner {
        cp,
        names: BTreeMap::new(),
        methods: BTreeMap::new(),
        functions: BTreeMap::new(),
        virtual_accessors: BTreeSet::new(),
        fields: BTreeMap::new(),
        occupied: BTreeSet::new(),
        bridges: BTreeMap::new(),
        current_owner: String::new(),
        current_functions: Vec::new(),
        failure: None,
    };
    for ((owner, source), unit) in units.iter().zip(checked.iter_mut()) {
        if let CheckedUnit::Module(module) = unit {
            if source.class_decl.is_none()
                && source.interface_decl.is_none()
                && source.object_decl.is_none()
            {
                for property in &source.properties {
                    if property.delegate.is_none() && !property.lateinit {
                        if property.getter_visibility() == syntax::PropertyVisibility::Private
                            && property.body.is_empty()
                        {
                            planner
                                .virtual_accessors
                                .insert((owner.clone(), check::getter_name(&property.name)));
                        }
                        if property.mutable
                            && property.setter_visibility() == syntax::PropertyVisibility::Private
                            && property.setter.is_none()
                        {
                            planner
                                .virtual_accessors
                                .insert((owner.clone(), check::setter_name(&property.name)));
                        }
                    }
                }
                let mut index = 0;
                module.functions.retain(|function| {
                    let generated = index >= source.functions.len();
                    index += 1;
                    !(generated
                        && planner
                            .virtual_accessors
                            .contains(&(owner.clone(), function.name.clone())))
                });
            }
        }
        let mut collect = |function: &mut Function, function_owner: &str, is_static| {
            if function_owner != owner && unit_owners.contains(function_owner) {
                return;
            }
            let info = FunctionInfo::new(function, function_owner, is_static);
            let key = info.key();
            if planner.functions.insert(key.clone(), info).is_some() {
                planner.failure = Some(clash(&key));
            }
        };
        match unit {
            CheckedUnit::Module(module) => {
                visit_functions(module, owner, &mut collect);
                if module.class_decl.is_none()
                    && module.interface_decl.is_none()
                    && module.object_decl.is_none()
                {
                    for property in &module.properties {
                        planner.fields.insert(
                            (owner.clone(), property.name.clone()),
                            FieldInfo {
                                ty: property.ty.clone(),
                                private: module
                                    .exposed_fields
                                    .get(&property.name)
                                    .is_none_or(|access| access & 2 != 0),
                            },
                        );
                    }
                }
            }
            CheckedUnit::Enum(enumeration) => enum_functions(enumeration, owner, &mut collect),
        }
    }
    if let Some(error) = planner.failure.take() {
        return Err(error);
    }
    let mut methods = cp.source_methods();
    methods.sort_by(|a, b| {
        (&a.owner, &a.name, &a.descriptor).cmp(&(&b.owner, &b.name, &b.descriptor))
    });
    for method in methods {
        let key = (
            method.owner.clone(),
            method.name.clone(),
            method.descriptor.clone(),
        );
        let name = if method.name.starts_with('<') {
            method.name.clone()
        } else {
            cp.source_jvm_name(&method.owner, &method.name, method.kotlin_internal)
        };
        planner.names.insert(key.clone(), name.clone());
        let virtual_private_accessor = method.access & 2 != 0
            && method.source_property.is_some()
            && (cp.default_source_accessor(&method).is_some()
                || !planner.functions.contains_key(&key)
                || planner
                    .virtual_accessors
                    .contains(&(key.0.clone(), key.1.clone())));
        if !virtual_private_accessor {
            planner.reserve((
                method.owner.clone(),
                name.clone(),
                method.descriptor.clone(),
            ))?;
        }
        if method
            .kotlin_params
            .as_ref()
            .is_some_and(|params| params.iter().any(|(_, _, default)| *default))
        {
            let constructor = method.name == "<init>";
            let default_name = if constructor {
                name
            } else {
                format!("{name}$default")
            };
            let default_desc = default_descriptor(
                &method.owner,
                &method.descriptor,
                method.access & 8 != 0,
                constructor,
                method
                    .kotlin_params
                    .as_ref()
                    .map_or(0, |params| params.len().div_ceil(32)),
            );
            let default_key = (
                method.owner.clone(),
                if constructor {
                    method.name.clone()
                } else {
                    format!("{}$default", method.name)
                },
                default_desc.clone(),
            );
            planner
                .names
                .insert(default_key.clone(), default_name.clone());
            planner.reserve((method.owner.clone(), default_name, default_desc))?;
            if constructor
                && method.kotlin_params.as_ref().is_some_and(|params| {
                    !params.is_empty() && params.iter().all(|(_, _, default)| *default)
                })
            {
                planner.reserve((method.owner.clone(), "<init>".into(), "()V".into()))?;
            }
        }
        if !virtual_private_accessor || !planner.methods.contains_key(&key) {
            planner.methods.insert(key, method);
        }
    }
    for info in planner.functions.values().cloned().collect::<Vec<_>>() {
        let key = info.key();
        if !planner.methods.contains_key(&key) {
            planner.reserve((
                info.owner.clone(),
                planner.physical(&key),
                info.descriptor.clone(),
            ))?;
            if info.defaults {
                planner.reserve((
                    info.owner.clone(),
                    format!("{}$default", planner.physical(&key)),
                    default_descriptor(
                        &info.owner,
                        &info.descriptor,
                        info.is_static,
                        false,
                        info.mask_count,
                    ),
                ))?;
            }
        }
    }
    for ((owner, _source), unit) in units.iter().zip(checked.iter_mut()) {
        let mut rename = |function: &mut Function, owner: &str, _is_static| {
            let key = (owner.into(), function.name.clone(), function.descriptor());
            let name = planner.physical(&key);
            function.jvm_name = (name != function.name).then_some(name);
        };
        match unit {
            CheckedUnit::Module(module) => visit_functions(module, owner, &mut rename),
            CheckedUnit::Enum(enumeration) => enum_functions(enumeration, owner, &mut rename),
        }
    }
    for ((owner, _source), unit) in units.iter().zip(checked.iter_mut()) {
        planner.current_owner.clone_from(owner);
        planner.current_functions = match unit {
            CheckedUnit::Module(module) => module
                .functions
                .iter()
                .map(|function| FunctionInfo::new(function, owner, true))
                .collect(),
            CheckedUnit::Enum(_) => Vec::new(),
        };
        match unit {
            CheckedUnit::Module(module) => ir_walk::module(module, owner, &mut planner),
            CheckedUnit::Enum(enumeration) => {
                ir_walk::class(&mut enumeration.class, owner, &mut planner);
                for (_, entry) in &mut enumeration.entries {
                    ir_walk::value(entry, owner, &mut planner);
                }
                ir_walk::lambdas(&mut enumeration.lambdas, &mut planner);
            }
        }
        if let Some(error) = planner.failure.take() {
            return Err(error);
        }
    }
    Ok(planner.bridges.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepare(
        source: &str,
        owner: &str,
        facade: bool,
    ) -> (Vec<(String, syntax::Module)>, Vec<CheckedUnit>, Classpath) {
        let source = syntax::parse(source).unwrap();
        let cp = Classpath::with_options(
            &[],
            &crate::CompileOptions {
                module_name: "feature-module".into(),
                friend_paths: Vec::new(),
            },
        )
        .unwrap();
        cp.register_source_file(owner, "Source.kt", facade);
        cp.source_header(&source, owner).unwrap();
        let checked = check::check_with_owner(&source, &cp, owner).unwrap();
        (
            vec![(owner.into(), source)],
            vec![CheckedUnit::Module(Box::new(checked))],
            cp,
        )
    }

    #[test]
    fn private_facade_bridges_follow_actual_lambda_uses() {
        let (units, mut checked, cp) = prepare(
            "private fun used(value: Int): Int = value + 1\nprivate fun unused(value: Int): Int = value\nfun direct(): Int = used(1)\nfun maker(): () -> Int = { used(2) }",
            "SourceKt",
            true,
        );
        let bridges = plan(&units, &mut checked, &cp).unwrap();
        assert_eq!(bridges.len(), 1);
        assert_eq!(bridges[0].name, "access$used");
        assert_eq!(bridges[0].descriptor, "(I)I");
        assert!(matches!(&bridges[0].target, BridgeTarget::Call { name, .. } if name == "used"));
        let CheckedUnit::Module(module) = &mut checked[0] else {
            unreachable!()
        };
        assert!(matches!(
            module.functions[2].body[0],
            Statement::Return(Value {
                kind: ValueKind::Call(0, _),
                ..
            })
        ));
        let mut calls = Vec::new();
        ir_walk::lambdas(
            &mut module.lambdas,
            &mut |value: &mut Value, _owner: &str| {
                if let ValueKind::StaticCall { name, .. } = &value.kind {
                    calls.push(name.clone());
                }
            },
        );
        assert_eq!(calls, ["access$used"]);
    }

    #[test]
    fn unrelated_lambdas_do_not_create_private_bridges() {
        let (units, mut checked, cp) = prepare(
            "private val unused: Int = 3\nprivate fun secret(value: Int): Int = value\nfun maker(): () -> Int = { 2 }",
            "SourceKt",
            true,
        );
        assert!(plan(&units, &mut checked, &cp).unwrap().is_empty());
    }

    #[test]
    fn private_field_bridges_are_requested_for_reads_and_writes_separately() {
        let (units, mut checked, cp) = prepare(
            "private var seed: Int = 1\nfun reader(): () -> Int = { seed }\nfun writer(): () -> Unit = { seed = 4 }",
            "SourceKt",
            true,
        );
        let bridges = plan(&units, &mut checked, &cp).unwrap();
        assert_eq!(bridges.len(), 2);
        assert_eq!(bridges[0].name, "access$getSeed$p");
        assert_eq!(bridges[0].descriptor, "()I");
        assert!(matches!(bridges[0].target, BridgeTarget::FieldGet { .. }));
        assert_eq!(bridges[1].name, "access$setSeed$p");
        assert_eq!(bridges[1].descriptor, "(I)V");
        assert!(matches!(bridges[1].target, BridgeTarget::FieldSet { .. }));
    }

    #[test]
    fn a_custom_private_getter_uses_a_method_bridge() {
        let (units, mut checked, cp) = prepare(
            "private val seed: Int get() = 2\nfun reader(): () -> Int = { seed }",
            "SourceKt",
            true,
        );
        let bridges = plan(&units, &mut checked, &cp).unwrap();
        assert_eq!(bridges.len(), 1);
        assert_eq!(bridges[0].name, "access$getSeed");
        assert!(matches!(&bridges[0].target, BridgeTarget::Call { name, .. } if name == "getSeed"));
    }

    #[test]
    fn internal_member_names_and_default_calls_share_the_module_suffix() {
        let (units, mut checked, cp) = prepare(
            "class Vault { internal fun hidden(value: Int = 2): Int = value\nfun read(): Int = hidden() }",
            "Vault",
            false,
        );
        assert!(plan(&units, &mut checked, &cp).unwrap().is_empty());
        let CheckedUnit::Module(module) = &mut checked[0] else {
            unreachable!()
        };
        let class = module.class_decl.as_ref().unwrap();
        let method = class
            .methods
            .iter()
            .find(|method| method.name == "hidden")
            .unwrap();
        assert_eq!(method.jvm_name(), "hidden$feature_module");
        let mut names = Vec::new();
        ir_walk::module(module, "Vault", &mut |value: &mut Value, _owner: &str| {
            if let ValueKind::StaticCall { name, .. } = &value.kind {
                names.push(name.clone());
            }
        });
        assert_eq!(names, ["hidden$feature_module$default"]);
    }

    #[test]
    fn wide_default_descriptors_keep_every_value_parameter_mask() {
        let descriptor = format!("({})I", "I".repeat(34));
        assert_eq!(
            default_descriptor("Wide", &descriptor, false, false, 34_usize.div_ceil(32)),
            format!("(LWide;{}IILjava/lang/Object;)I", "I".repeat(34))
        );
        let (_, mut checked, _) = prepare("fun Int.ext(value: Int): Int = value", "SourceKt", true);
        let CheckedUnit::Module(module) = &mut checked[0] else {
            unreachable!()
        };
        let function = &mut module.functions[0];
        function
            .params
            .extend((1..32).map(|index| (format!("value{index}"), Type::Int)));
        assert_eq!(function.params.len(), 33);
        assert_eq!(FunctionInfo::new(function, "SourceKt", true).mask_count, 1);
    }

    #[test]
    fn top_level_internal_functions_keep_their_source_names() {
        let (units, mut checked, cp) = prepare(
            "internal fun hidden(value: Int): Int = value",
            "SourceKt",
            true,
        );
        assert!(plan(&units, &mut checked, &cp).unwrap().is_empty());
        let CheckedUnit::Module(module) = &checked[0] else {
            unreachable!()
        };
        assert_eq!(module.functions[0].jvm_name(), "hidden");
    }

    #[test]
    fn mangled_internal_names_cannot_overwrite_a_source_method() {
        let (units, mut checked, cp) = prepare(
            "class Vault { internal fun hidden(value: Int): Int = value\nfun `hidden$feature_module`(value: Int): Int = value }",
            "Vault",
            false,
        );
        let Err((owner, error)) = plan(&units, &mut checked, &cp) else {
            panic!("Expected internal method signature clash")
        };
        assert_eq!(owner, "Vault");
        assert_eq!(error.code, "JVM_SIGNATURE_CLASH");
    }

    #[test]
    fn default_methods_cannot_overwrite_a_source_method() {
        let (units, mut checked, cp) = prepare(
            "fun declared(value: Int = 1): Int = value\nfun `declared$default`(value: Int, mask: Int, marker: Any?): Int = value",
            "SourceKt",
            true,
        );
        let Err((owner, error)) = plan(&units, &mut checked, &cp) else {
            panic!("Expected default method signature clash")
        };
        assert_eq!(owner, "SourceKt");
        assert_eq!(error.code, "JVM_SIGNATURE_CLASH");
    }

    #[test]
    fn a_required_private_bridge_cannot_overwrite_a_source_method() {
        let (units, mut checked, cp) = prepare(
            "private fun secret(value: Int): Int = value\nfun `access$secret`(value: Int): Int = value\nfun maker(): () -> Int = { secret(1) }",
            "SourceKt",
            true,
        );
        let Err((owner, error)) = plan(&units, &mut checked, &cp) else {
            panic!("Expected bridge signature clash")
        };
        assert_eq!(owner, "SourceKt");
        assert_eq!(error.kind, DiagnosticKind::InvalidKotlin);
        assert_eq!(error.code, "JVM_SIGNATURE_CLASH");
    }
}
