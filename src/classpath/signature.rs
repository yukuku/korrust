use super::*;
use crate::syntax::{TypeArgument, TypeParameter, Variance};
use std::collections::BTreeMap;

pub(super) struct Signature<'a> {
    bytes: &'a [u8],
    offset: usize,
    parameters: BTreeMap<String, Type>,
    depth: usize,
    scanning_formals: bool,
}
impl<'a> Signature<'a> {
    fn new(value: &'a str, parameters: &[TypeParameter]) -> Self {
        Self {
            bytes: value.as_bytes(),
            offset: 0,
            parameters: parameters
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        Type::Parameter {
                            name: p.name.clone(),
                            upper_bound: Box::new(
                                p.bounds
                                    .first()
                                    .cloned()
                                    .unwrap_or(Type::Java("java/lang/Object".into())),
                            ),
                        },
                    )
                })
                .collect(),
            depth: 0,
            scanning_formals: false,
        }
    }
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.offset).copied()
    }
    fn next(&mut self) -> Result<u8> {
        let v = self.peek().ok_or_else(invalid)?;
        self.offset += 1;
        Ok(v)
    }
    fn expect(&mut self, v: u8) -> Result<()> {
        if self.next()? == v {
            Ok(())
        } else {
            Err(invalid())
        }
    }
    fn name(&mut self, ends: &[u8]) -> Result<String> {
        let start = self.offset;
        while self.peek().is_some_and(|b| !ends.contains(&b)) {
            self.offset += 1;
        }
        if start == self.offset {
            return Err(invalid());
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.offset])
            .map_err(|_| invalid())?
            .into())
    }
    fn type_params(&mut self) -> Result<Vec<TypeParameter>> {
        if self.peek() != Some(b'<') {
            return Ok(Vec::new());
        }
        let start = self.offset;
        self.scanning_formals = true;
        let formals = self.read_formals()?;
        self.scanning_formals = false;
        // All names are in scope throughout the bound list. Seed their erasures
        // before resolving any nested bound or member signature.
        let mut bounds = BTreeMap::new();
        for formal in &formals {
            if bounds
                .insert(formal.name.clone(), formal.bounds.first())
                .is_some()
            {
                return Err(invalid());
            }
        }
        let mut seeded = self.parameters.clone();
        for formal in &formals {
            let mut bound = bounds[&formal.name];
            let mut seen = std::collections::HashSet::new();
            while let Some(Type::Parameter { name, .. }) = bound {
                if !seen.insert(name) {
                    return Err(invalid());
                }
                bound = match bounds.get(name) {
                    Some(next) => *next,
                    None => self.parameters.get(name).map(|ty| match ty {
                        Type::Parameter { upper_bound, .. } => upper_bound.as_ref(),
                        other => other,
                    }),
                };
            }
            seeded.insert(
                formal.name.clone(),
                Type::Parameter {
                    name: formal.name.clone(),
                    upper_bound: Box::new(
                        bound
                            .cloned()
                            .unwrap_or(Type::Java("java/lang/Object".into())),
                    ),
                },
            );
        }
        self.parameters = seeded;
        self.offset = start;
        let result = self.read_formals()?;
        for formal in &result {
            self.parameters.insert(
                formal.name.clone(),
                Type::Parameter {
                    name: formal.name.clone(),
                    upper_bound: Box::new(
                        formal
                            .bounds
                            .first()
                            .cloned()
                            .unwrap_or(Type::Java("java/lang/Object".into())),
                    ),
                },
            );
        }
        Ok(result)
    }
    fn read_formals(&mut self) -> Result<Vec<TypeParameter>> {
        let mut result = Vec::new();
        self.next()?;
        while self.peek() != Some(b'>') {
            let name = self.name(b":")?;
            self.expect(b':')?;
            let mut bounds = Vec::new();
            if self.peek() != Some(b':') {
                bounds.push(self.ty()?);
            }
            while self.peek() == Some(b':') {
                self.next()?;
                bounds.push(self.ty()?);
            }
            result.push(TypeParameter {
                only_input_types: false,
                reified: false,
                name,
                variance: Variance::Invariant,
                bounds,
            });
            if result.len() > 256 {
                return Err(invalid());
            }
        }
        self.next()?;
        Ok(result)
    }
    fn ty(&mut self) -> Result<Type> {
        if self.depth >= 64 {
            return Err(invalid());
        }
        self.depth += 1;
        let ty = match self.next()? {
            b'I' => Type::Int,
            b'J' => Type::Long,
            b'B' => Type::Byte,
            b'S' => Type::Short,
            b'C' => Type::Char,
            b'Z' => Type::Boolean,
            b'F' => Type::Float,
            b'D' => Type::Double,
            b'V' => Type::Unit,
            b'T' => {
                let name = self.name(b";")?;
                self.expect(b';')?;
                if self.scanning_formals {
                    Type::Parameter {
                        name,
                        upper_bound: Box::new(Type::Java("java/lang/Object".into())),
                    }
                } else {
                    self.parameters.get(&name).cloned().ok_or_else(invalid)?
                }
            }
            b'[' => match self.ty()? {
                Type::Int => Type::IntArray,
                Type::Byte => Type::ByteArray,
                Type::Short => Type::ShortArray,
                Type::Char => Type::CharArray,
                Type::Long => Type::LongArray,
                Type::Boolean => Type::BooleanArray,
                Type::Float => Type::FloatArray,
                Type::Double => Type::DoubleArray,

                Type::String => Type::StringArray,
                element if element.is_reference() => Type::Array(Box::new(element)),
                _ => return Err(invalid()),
            },
            b'L' => {
                let mut name = self.name(b";<.")?;
                let mut arguments = Vec::new();
                loop {
                    if self.peek() == Some(b'<') {
                        self.next()?;
                        while self.peek() != Some(b'>') {
                            let (variance, ty) = match self.peek() {
                                Some(b'*') => {
                                    self.next()?;
                                    (Variance::Out, None)
                                }
                                Some(b'+') => {
                                    self.next()?;
                                    (Variance::Out, Some(self.ty()?))
                                }
                                Some(b'-') => {
                                    self.next()?;
                                    (Variance::In, Some(self.ty()?))
                                }
                                _ => (Variance::Invariant, Some(self.ty()?)),
                            };
                            let ty = ty.map(|ty| {
                                if ty.is_reference() {
                                    Type::Platform(Box::new(ty))
                                } else {
                                    ty
                                }
                            });
                            arguments.push(TypeArgument { variance, ty });
                            if arguments.len() > 256 {
                                return Err(invalid());
                            }
                        }
                        self.next()?;
                    }
                    if self.peek() != Some(b'.') {
                        break;
                    }
                    if !arguments.is_empty() {
                        return Err(invalid());
                    }
                    self.next()?;
                    name.push('$');
                    name.push_str(&self.name(b";<.")?);
                }
                self.expect(b';')?;
                if arguments.is_empty() {
                    match name.as_str() {
                        "java/lang/String" => Type::String,
                        _ => Type::Java(name),
                    }
                } else if name == "java/util/List"
                    && arguments.len() == 1
                    && arguments[0].ty.is_some()
                    && arguments[0].variance == Variance::Invariant
                {
                    Type::FlexibleList(Box::new(arguments.remove(0).ty.unwrap()))
                } else {
                    Type::Parameterized { name, arguments }
                }
            }
            _ => return Err(invalid()),
        };
        self.depth -= 1;
        Ok(ty)
    }
    fn finish(&self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(invalid())
        }
    }
}

pub(super) fn method(
    value: &str,
    parameters: &[TypeParameter],
) -> Result<(Vec<TypeParameter>, Vec<Type>, Type)> {
    let mut s = Signature::new(value, parameters);
    let tp = s.type_params()?;
    s.expect(b'(')?;
    let mut args = Vec::new();
    while s.peek() != Some(b')') {
        args.push(s.ty()?);
    }
    s.next()?;
    let result = s.ty()?;
    while s.peek() == Some(b'^') {
        s.next()?;
        s.ty()?;
    }
    s.finish()?;
    Ok((tp, args, result))
}
pub(super) fn field(value: &str, parameters: &[TypeParameter]) -> Result<Type> {
    let mut signature = Signature::new(value, parameters);
    let ty = signature.ty()?;
    signature.finish()?;
    if !ty.is_reference() {
        return Err(invalid());
    }
    Ok(ty)
}

pub(super) fn attach_fields(class: &mut JavaClass, attrs: &Attributes, field_attrs: &[Attributes]) {
    let _ = attach_fields_with_declarations(class, attrs, field_attrs, &mut |_| Ok(false));
}

pub(super) fn attach_fields_with_declarations(
    class: &mut JavaClass,
    attrs: &Attributes,
    field_attrs: &[Attributes],
    passive: &mut impl FnMut(&Annotation) -> Result<bool>,
) -> Result<()> {
    let parameters = match attrs.signature.as_deref().map(self::class).transpose() {
        Ok(Some((parameters, _))) => parameters,
        Ok(None) => Vec::new(),
        Err(_) => return Ok(()),
    };
    for (field, attributes) in class.fields.iter_mut().zip(field_attrs) {
        if attributes.signature.is_none()
            && attributes.annotations.is_empty()
            && attributes.return_type_annotations.is_empty()
        {
            continue;
        }
        if attributes.unsupported
            || field.access & 0x1000 != 0
            || attributes.constant.is_some() && field.constant.is_none()
        {
            continue;
        }
        let ty = match &attributes.signature {
            Some(signature) => self::field(signature, &parameters),
            None => crate::jvm_descriptor::field_type(&field.descriptor),
        };
        let Ok(ty) = ty else {
            continue;
        };
        if ty.descriptor() != field.descriptor
            || attributes
                .inner_type_paths
                .iter()
                .any(|(target, _, depth)| {
                    *target != 0x13 || !inner_annotation_at_leaf(&ty, *depth, &attrs.inner_classes)
                })
        {
            continue;
        }
        let mut declarations = Vec::new();
        for annotation in &attributes.annotations {
            if !passive(annotation)? {
                declarations.push(annotation);
            }
        }
        if !ty.is_reference()
            && (!declarations.is_empty() || !attributes.return_type_annotations.is_empty())
        {
            continue;
        }
        if let Ok(ty) = nullability(
            ty,
            declarations
                .iter()
                .copied()
                .chain(&attributes.return_type_annotations),
        ) {
            field.java_type = Some(Box::new(ty));
            field.unsupported = false;
        }
    }
    Ok(())
}

fn inner_annotation_at_leaf(
    ty: &Type,
    depth: u8,
    inner_classes: &[(String, String, String, u16)],
) -> bool {
    let mut owner = match ty.base() {
        Type::Java(name) | Type::Parameterized { name, .. } => name.as_str(),
        _ => return false,
    };
    let mut count = 0usize;
    loop {
        let mut entries = inner_classes
            .iter()
            .filter(|(inner, _, _, _)| inner == owner);
        let Some((_, outer, _, _)) = entries.next() else {
            break;
        };
        count += 1;
        if entries.next().is_some() || count > usize::from(depth) {
            return false;
        }
        owner = outer;
    }
    count == usize::from(depth)
}

pub(super) fn class(value: &str) -> Result<(Vec<TypeParameter>, Vec<Type>)> {
    let mut s = Signature::new(value, &[]);
    let parameters = s.type_params()?;
    let mut parents = Vec::new();
    while s.peek().is_some() {
        parents.push(s.ty()?);
    }
    Ok((parameters, parents))
}

pub(super) fn nullability<'a>(
    ty: Type,
    annotations: impl IntoIterator<Item = &'a Annotation>,
) -> Result<Type> {
    if !ty.is_reference() {
        return Ok(ty);
    }
    let mut state = None;
    let mut mutability = None;
    for annotation in annotations {
        if annotation.name == "Ljdk/internal/vm/annotation/IntrinsicCandidate;"
            && annotation.values.is_empty()
        {
            continue;
        }
        if matches!(
            annotation.name.as_str(),
            "Landroidx/annotation/RecentlyNonNull;" | "Landroidx/annotation/RecentlyNullable;"
        ) {
            if !annotation.values.is_empty() {
                return Err(invalid());
            }
            continue;
        }
        let mutable = match annotation.name.as_str() {
            "Lorg/jetbrains/annotations/ReadOnly;" | "Lkotlin/annotations/jvm/ReadOnly;" => {
                Some(false)
            }
            "Lorg/jetbrains/annotations/Mutable;" | "Lkotlin/annotations/jvm/Mutable;" => {
                Some(true)
            }
            _ => None,
        };
        if let Some(mutable) = mutable {
            if !annotation.values.is_empty()
                || mutability.is_some_and(|previous| previous != mutable)
            {
                return Err(invalid());
            }
            mutability = Some(mutable);
            continue;
        }
        let nullable = nullability_polarity(&annotation.name).ok_or_else(invalid)?;
        if !annotation.values.is_empty() || state.is_some_and(|s| s != nullable) {
            return Err(invalid());
        }
        state = Some(nullable);
    }
    let ty = match (ty, mutability) {
        (Type::FlexibleList(element), Some(false)) => Type::List(element),
        (Type::FlexibleList(element), Some(true)) => Type::MutableList(element),
        (ty, None) => ty,
        _ => return Err(invalid()),
    };
    Ok(match state {
        Some(true) => Type::Nullable(Box::new(ty)),
        Some(false) => ty,
        None => Type::Platform(Box::new(ty)),
    })
}

fn nullability_polarity(name: &str) -> Option<bool> {
    Some(match name {
        "Lorg/jetbrains/annotations/NotNull;"
        | "Lorg/jspecify/annotations/NonNull;"
        | "Landroid/annotation/NonNull;"
        | "Landroidx/annotation/NonNull;"
        | "Landroid/support/annotation/NonNull;"
        | "Lcom/android/annotations/NonNull;"
        | "Lorg/checkerframework/checker/nullness/compatqual/NonNullDecl;"
        | "Lorg/checkerframework/checker/nullness/qual/NonNull;"
        | "Ledu/umd/cs/findbugs/annotations/NonNull;"
        | "Lio/reactivex/annotations/NonNull;"
        | "Lio/reactivex/rxjava3/annotations/NonNull;"
        | "Lorg/eclipse/jdt/annotation/NonNull;"
        | "Llombok/NonNull;"
        | "Ljavax/annotation/Nonnull;"
        | "Ljakarta/annotation/Nonnull;" => false,
        "Lorg/jetbrains/annotations/Nullable;"
        | "Lorg/jspecify/nullness/Nullable;"
        | "Lorg/jspecify/annotations/Nullable;"
        | "Landroid/annotation/Nullable;"
        | "Landroidx/annotation/Nullable;"
        | "Landroid/support/annotation/Nullable;"
        | "Lcom/android/annotations/Nullable;"
        | "Lorg/checkerframework/checker/nullness/compatqual/NullableDecl;"
        | "Lorg/checkerframework/checker/nullness/qual/Nullable;"
        | "Ledu/umd/cs/findbugs/annotations/Nullable;"
        | "Ledu/umd/cs/findbugs/annotations/PossiblyNull;"
        | "Ledu/umd/cs/findbugs/annotations/CheckForNull;"
        | "Lio/reactivex/annotations/Nullable;"
        | "Lio/reactivex/rxjava3/annotations/Nullable;"
        | "Lorg/eclipse/jdt/annotation/Nullable;"
        | "Ljavax/annotation/Nullable;"
        | "Ljavax/annotation/CheckForNull;"
        | "Ljakarta/annotation/Nullable;"
        | "Lio/vertx/codegen/annotations/Nullable;" => true,
        _ => return None,
    })
}

pub(super) fn type_semantic_annotation(name: &str) -> bool {
    nullability_polarity(name).is_some()
        || matches!(
            name,
            "Landroidx/annotation/RecentlyNonNull;"
                | "Landroidx/annotation/RecentlyNullable;"
                | "Lorg/jspecify/nullness/NullMarked;"
                | "Lorg/jspecify/annotations/NullMarked;"
                | "Lorg/jspecify/annotations/NullUnmarked;"
                | "Lorg/jspecify/nullness/NullnessUnspecified;"
                | "Lorg/jspecify/annotations/NullnessUnspecified;"
                | "Ljavax/annotation/ParametersAreNonnullByDefault;"
                | "Ljavax/annotation/ParametersAreNullableByDefault;"
                | "Ljavax/annotation/meta/TypeQualifier;"
                | "Ljavax/annotation/meta/TypeQualifierNickname;"
                | "Ljavax/annotation/meta/TypeQualifierDefault;"
                | "Lorg/jetbrains/annotations/ReadOnly;"
                | "Lorg/jetbrains/annotations/Mutable;"
                | "Lorg/jetbrains/annotations/Unmodifiable;"
                | "Lorg/jetbrains/annotations/UnmodifiableView;"
                | "Lkotlin/annotations/jvm/ReadOnly;"
                | "Lkotlin/annotations/jvm/Mutable;"
        )
}

pub(super) fn attach(
    class: &mut JavaClass,
    attrs: &Attributes,
    method_attrs: &[Attributes],
) -> Result<()> {
    attach_with_declarations(class, attrs, method_attrs, &mut |annotation| {
        Ok(annotation.name == "Ljava/lang/FunctionalInterface;" && annotation.values.is_empty())
    })
}

pub(super) fn attach_with_declarations(
    class: &mut JavaClass,
    attrs: &Attributes,
    method_attrs: &[Attributes],
    passive: &mut impl FnMut(&Annotation) -> Result<bool>,
) -> Result<()> {
    if !attrs.return_type_annotations.is_empty() || !attrs.parameter_type_annotations.is_empty() {
        return Err(invalid());
    }
    let (type_parameters, supertypes) = if let Some(value) = &attrs.signature {
        self::class(value)?
    } else {
        (
            Vec::new(),
            class
                .super_name
                .iter()
                .chain(class.interfaces.iter())
                .map(|n| Type::Java(n.clone()))
                .collect(),
        )
    };
    let declared_parents = supertypes.iter().map(Type::descriptor).collect::<Vec<_>>();
    let actual_parents = class
        .super_name
        .iter()
        .chain(class.interfaces.iter())
        .map(|name| format!("L{name};"))
        .collect::<Vec<_>>();
    if declared_parents != actual_parents {
        return Err(invalid());
    }
    for (method, attributes) in class.methods.iter_mut().zip(method_attrs) {
        let value = attributes
            .signature
            .as_deref()
            .unwrap_or(&method.descriptor);
        if let Ok((parameters, params, result)) = self::method(value, &type_parameters) {
            let erased = format!(
                "({}){}",
                params.iter().map(Type::descriptor).collect::<String>(),
                result.descriptor()
            );
            if erased != method.descriptor
                || attributes
                    .inner_type_paths
                    .iter()
                    .any(|(target, index, depth)| {
                        let ty = match *target {
                            0x14 => Some(&result),
                            0x16 => index.and_then(|index| params.get(index)),
                            _ => None,
                        };
                        ty.is_none_or(|ty| {
                            !inner_annotation_at_leaf(ty, *depth, &attrs.inner_classes)
                        })
                    })
            {
                continue;
            }
            if attributes
                .parameter_type_annotations
                .iter()
                .any(|(index, _)| *index >= params.len())
            {
                return Err(invalid());
            }
            let mut declarations = Vec::new();
            for annotation in &attributes.annotations {
                if !passive(annotation)? {
                    declarations.push(annotation);
                }
            }
            let mut parameter_declarations = Vec::new();
            for annotations in &attributes.parameter_annotations {
                let mut selected = Vec::new();
                for annotation in annotations {
                    if !passive(annotation)? {
                        selected.push(annotation);
                    }
                }
                parameter_declarations.push(selected);
            }
            let converted = params
                .into_iter()
                .enumerate()
                .map(|(i, ty)| {
                    Ok((
                        format!("arg{i}"),
                        nullability(
                            ty,
                            parameter_declarations
                                .get(i)
                                .into_iter()
                                .flatten()
                                .copied()
                                .chain(attributes.parameter_type_annotations.iter().filter_map(
                                    |(index, annotation)| (*index == i).then_some(annotation),
                                )),
                        )?,
                        false,
                    ))
                })
                .collect::<Result<Vec<_>>>();
            let result = nullability(
                result,
                declarations
                    .iter()
                    .copied()
                    .chain(attributes.return_type_annotations.iter()),
            );
            if let (Ok(params), Ok(result)) = (converted, result) {
                if !attributes.unsupported
                    && attributes.only_input_type_parameters.is_empty()
                    && method.access & (0x40 | 0x1000) == 0
                {
                    method.unsupported = false;
                    method.kotlin_params = Some(params);
                    method.kotlin_result = Some(result);
                    method.library = Some(LibraryMethod {
                        default_code: None,
                        vararg_index: None,
                        contracts: Vec::new(),
                        value_dispatch: false,
                        suspend_: false,
                        kotlin_declared: false,
                        has_annotations: false,
                        parameter_modes: Vec::new(),
                        reified_arguments: Vec::new(),
                        abstract_: method.access & 0x400 != 0,
                        builtin_array: false,
                        kotlin_public: method.access & 1 != 0,
                        source_name: method.name.clone(),
                        type_parameters: parameters,
                        receiver: None,
                        infix: false,
                        operator: false,
                        inline: false,
                        code: attributes.code.clone(),
                    });
                }
            }
        }
    }
    let mut passive_class = true;
    for annotation in &attrs.annotations {
        passive_class &= passive(annotation)?;
    }
    if !attrs.unsupported && passive_class {
        class.unsupported = false;
    }
    class.library = Some(LibraryClass {
        value_class: None,
        companion_name: None,
        kotlin_visibility: None,
        type_aliases: Vec::new(),
        type_parameters,
        supertypes,
        package: None,
        multifile_facade: None,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forward_bounds_preserve_erasure_and_reject_undeclared_or_cyclic_formals() {
        let (_, args, result) = method("<T:TU;U:Ljava/lang/Number;>(TT;)TU;", &[]).unwrap();
        assert_eq!(args[0].descriptor(), "Ljava/lang/Number;");
        assert_eq!(result.descriptor(), "Ljava/lang/Number;");
        for invalid in [
            "<T:TU;>()V",
            "<T:Ljava/util/List<TU;>;>()V",
            "<T:TU;U:TT;>()V",
            "<T:Ljava/lang/Object;T:Ljava/lang/Object;>()V",
        ] {
            assert!(method(invalid, &[]).is_err(), "{invalid}");
        }
    }

    #[test]
    fn generic_field_signatures_preserve_variables_and_reject_malformed_inputs() {
        let (parameters, _) = class("<T:Ljava/lang/Object;>Ljava/lang/Object;").unwrap();
        let ty = field("TT;", &parameters).unwrap();
        assert!(matches!(ty, Type::Parameter { name, .. } if name == "T"));
        assert_eq!(
            field("[TT;", &parameters).unwrap().descriptor(),
            "[Ljava/lang/Object;"
        );
        for invalid in [
            "",
            "V",
            "I",
            "TU;",
            "TT",
            "TT;;",
            "()TT;",
            "Ljava/util/List<TT;>",
        ] {
            assert!(field(invalid, &parameters).is_err(), "{invalid}");
        }
    }

    #[test]
    fn generic_method_preserves_variables_and_variance() {
        let (tp, args, result) =
            method("<T:Ljava/lang/Object;>(TT;Ljava/util/List<+TT;>;)TT;", &[]).unwrap();
        assert_eq!(tp[0].name, "T");
        assert!(matches!(&args[0],Type::Parameter{name,..}if name=="T"));
        assert_eq!(args[0], result);
    }
    #[test]
    fn generic_truncations_are_diagnostics() {
        let value = "<T:Ljava/lang/Object;>(TT;)TT;";
        for end in 0..value.len() {
            assert!(method(&value[..end], &[]).is_err(), "{end}");
        }
    }
    #[test]
    fn wildcard_signatures_preserve_projection_and_platform_arguments() {
        let (_, params, _) = method("(Ljava/util/List<+Ljava/lang/String;>;)V", &[]).unwrap();
        let Type::Parameterized { arguments, .. } = &params[0] else {
            panic!()
        };
        assert_eq!(arguments[0].variance, Variance::Out);
        assert_eq!(
            arguments[0].ty,
            Some(Type::Platform(Box::new(Type::String)))
        );
    }
    #[test]
    fn nullability_rejects_unknown_and_conflicting_markers() {
        let marker = |name: &str| Annotation {
            name: name.into(),
            values: Vec::new(),
        };
        assert_eq!(
            nullability(Type::Int, &[marker("Lsample/Unknown;")]).unwrap(),
            Type::Int
        );
        assert_eq!(
            nullability(
                Type::Double,
                &[Annotation {
                    name: "Lsample/Constraint;".into(),
                    values: vec![("minimum".into(), AnnotationValue::Other)]
                }]
            )
            .unwrap(),
            Type::Double
        );
        assert!(nullability(Type::String, &[marker("Lsample/Unknown;")]).is_err());
        assert!(
            nullability(
                Type::String,
                &[
                    marker("Lorg/jetbrains/annotations/Nullable;"),
                    marker("Lorg/jetbrains/annotations/NotNull;")
                ]
            )
            .is_err()
        );
        assert_eq!(
            nullability(Type::String, &[]).unwrap(),
            Type::Platform(Box::new(Type::String))
        );
    }

    #[test]
    fn inner_type_annotations_require_the_leaf_and_consistent_nesting() {
        let nesting = vec![
            ("Outer$Middle".into(), "Outer".into(), "Middle".into(), 9),
            (
                "Outer$Middle$Leaf".into(),
                "Outer$Middle".into(),
                "Leaf".into(),
                9,
            ),
        ];
        let leaf = Type::Java("Outer$Middle$Leaf".into());
        assert!(inner_annotation_at_leaf(&leaf, 2, &nesting));
        assert!(!inner_annotation_at_leaf(&leaf, 1, &nesting));
        assert!(!inner_annotation_at_leaf(&Type::String, 1, &nesting));
        assert!(!inner_annotation_at_leaf(
            &Type::Array(Box::new(leaf.clone())),
            2,
            &nesting
        ));
        assert!(!inner_annotation_at_leaf(&leaf, 2, &[]));
        let mut conflicting = nesting.clone();
        conflicting.push(nesting[1].clone());
        assert!(!inner_annotation_at_leaf(&leaf, 2, &conflicting));
    }

    #[test]
    fn android_migration_nullability_keeps_platform_types_and_strict_markers() {
        let marker = |name: &str| Annotation {
            name: name.into(),
            values: Vec::new(),
        };
        for recent in [
            "Landroidx/annotation/RecentlyNonNull;",
            "Landroidx/annotation/RecentlyNullable;",
        ] {
            assert_eq!(
                nullability(Type::String, &[marker(recent)]).unwrap(),
                Type::Platform(Box::new(Type::String))
            );
            for (strict, expected) in [
                ("Landroidx/annotation/NonNull;", Type::String),
                (
                    "Landroidx/annotation/Nullable;",
                    Type::Nullable(Box::new(Type::String)),
                ),
            ] {
                for annotations in [
                    vec![marker(recent), marker(strict)],
                    vec![marker(strict), marker(recent)],
                ] {
                    assert_eq!(nullability(Type::String, &annotations).unwrap(), expected);
                }
            }
            assert!(
                nullability(
                    Type::String,
                    &[Annotation {
                        name: recent.into(),
                        values: vec![("value".into(), AnnotationValue::Other)],
                    }]
                )
                .is_err()
            );
        }
    }

    #[test]
    fn declaration_and_type_use_nullability_merge_without_order_dependence() {
        let not_null = Annotation {
            name: "Lorg/jetbrains/annotations/NotNull;".into(),
            values: Vec::new(),
        };
        let non_null = Annotation {
            name: "Lorg/jspecify/annotations/NonNull;".into(),
            values: Vec::new(),
        };
        let nullable = Annotation {
            name: "Lorg/jspecify/annotations/Nullable;".into(),
            values: Vec::new(),
        };
        let unknown = Annotation {
            name: "Lsample/Unknown;".into(),
            values: Vec::new(),
        };
        for annotations in [[&not_null, &non_null], [&non_null, &not_null]] {
            assert_eq!(
                nullability(Type::DoubleArray, annotations).unwrap(),
                Type::DoubleArray
            );
        }
        for annotations in [
            [&nullable, &non_null],
            [&non_null, &nullable],
            [&unknown, &non_null],
            [&non_null, &unknown],
        ] {
            assert!(nullability(Type::DoubleArray, annotations).is_err());
        }
    }

    #[test]
    fn implementation_annotation_preserves_platform_and_explicit_nullability() {
        let intrinsic = || Annotation {
            name: "Ljdk/internal/vm/annotation/IntrinsicCandidate;".into(),
            values: Vec::new(),
        };
        assert_eq!(nullability(Type::Unit, &[intrinsic()]).unwrap(), Type::Unit);
        assert_eq!(
            nullability(Type::String, &[intrinsic()]).unwrap(),
            Type::Platform(Box::new(Type::String))
        );
        assert_eq!(
            nullability(
                Type::String,
                &[
                    intrinsic(),
                    Annotation {
                        name: "Lorg/jetbrains/annotations/Nullable;".into(),
                        values: Vec::new()
                    }
                ]
            )
            .unwrap(),
            Type::Nullable(Box::new(Type::String))
        );
    }
}
