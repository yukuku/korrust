use super::*;

impl Checker<'_> {
    pub(super) fn constructor_argument_with_context(
        &mut self,
        arg: &Expr,
        expected: &Type,
    ) -> Result<Value> {
        let previous = self.clone();
        let count = self.lambdas.borrow().len();
        let value = self.value(arg);
        if value
            .as_ref()
            .is_ok_and(|value| !unresolved_list(&value.ty))
        {
            return value;
        }
        *self = previous;
        self.lambdas.borrow_mut().truncate(count);
        self.argument_value(arg, expected)
    }

    pub(super) fn constructor_storage_value(
        &self,
        value: Value,
        storage: Type,
        offset: usize,
    ) -> Result<Value> {
        fn assignable(classpath: &Classpath, actual: &str, expected: &str) -> bool {
            if actual == expected {
                return true;
            }
            if expected == "Ljava/lang/Object;"
                && matches!(actual.as_bytes().first(), Some(b'L' | b'['))
            {
                return true;
            }
            if let (Some(actual), Some(expected)) =
                (actual.strip_prefix('['), expected.strip_prefix('['))
            {
                return assignable(classpath, actual, expected);
            }
            match (
                actual
                    .strip_prefix('L')
                    .and_then(|name| name.strip_suffix(';')),
                expected
                    .strip_prefix('L')
                    .and_then(|name| name.strip_suffix(';')),
            ) {
                (Some(actual), Some(expected)) => relations::subtype_at(
                    classpath,
                    &Type::Java(actual.into()),
                    &Type::Java(expected.into()),
                    0,
                ),
                _ => false,
            }
        }
        match self.coerce(value.clone(), storage.clone(), offset) {
            Ok(value) => Ok(value),
            // The specialized constructor parameters have already checked source types.
            Err(_)
                if value.ty.is_reference()
                    && storage.is_reference()
                    && assignable(
                        self.classpath,
                        &value.ty.descriptor(),
                        &storage.descriptor(),
                    ) =>
            {
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    pub(super) fn infer_constructor_method(
        &mut self,
        class: &JavaClass,
        args: &[Expr],
        expected: Option<&Type>,
        offset: usize,
    ) -> Result<JavaMethod> {
        let class_parameters = &class.library.as_ref().unwrap().type_parameters;
        let mut constructors = Vec::new();
        for (method_index, original) in class.methods.iter().enumerate().filter(|(_, method)| {
            method.name == "<init>"
                && self
                    .classpath
                    .constructor_visible(&self.lambda_owner, method)
                && !method.unsupported
                && (!class.kotlin_class || method.kotlin_params.is_some())
        }) {
            let mut method = original.clone();
            let original_method_parameters = method
                .library
                .as_ref()
                .map(|library| library.type_parameters.clone())
                .unwrap_or_default();
            let mut parameters = class_parameters.clone();
            for (index, parameter) in parameters.iter_mut().enumerate() {
                parameter.name = format!("<constructor-class-{index}>");
            }
            let renaming = |originals: &[syntax::TypeParameter],
                            parameters: &[syntax::TypeParameter]|
             -> HashMap<String, Type> {
                originals
                    .iter()
                    .zip(parameters)
                    .map(|(original, parameter)| {
                        (
                            original.name.clone(),
                            Type::Parameter {
                                name: parameter.name.clone(),
                                upper_bound: Box::new(parameter.bounds.first().cloned().unwrap_or(
                                    Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
                                )),
                            },
                        )
                    })
                    .collect()
            };
            let substitutions = renaming(class_parameters, &parameters);
            for parameter in &mut parameters {
                parameter.bounds = parameter
                    .bounds
                    .iter()
                    .map(|bound| relations::substitute(bound, &substitutions))
                    .collect();
            }
            let mut own_parameters = original_method_parameters.clone();
            for (index, parameter) in own_parameters.iter_mut().enumerate() {
                parameter.name = format!("<constructor-method-{method_index}-{index}>");
            }
            let mut member_substitutions = renaming(class_parameters, &parameters);
            member_substitutions.extend(renaming(&original_method_parameters, &own_parameters));
            for parameter in &mut own_parameters {
                parameter.bounds = parameter
                    .bounds
                    .iter()
                    .map(|bound| relations::substitute(bound, &member_substitutions))
                    .collect();
            }
            member_substitutions.extend(renaming(&original_method_parameters, &own_parameters));
            let result = declarations::classifier_type(&class.name, &parameters);
            let (params, defaults) = Self::method_parameters(&method)?;
            method.kotlin_params = Some(
                params
                    .into_iter()
                    .zip(defaults)
                    .map(|((name, ty), default)| {
                        (
                            name,
                            relations::substitute(&ty, &member_substitutions),
                            default,
                        )
                    })
                    .collect(),
            );
            method.kotlin_result = Some(result);
            let metadata = method
                .library
                .get_or_insert_with(|| crate::classpath::LibraryMethod {
                    source_name: "<init>".into(),
                    kotlin_declared: class.kotlin_class,
                    kotlin_public: true,
                    ..Default::default()
                });
            parameters.extend(own_parameters);
            metadata.type_parameters = parameters;
            constructors.push(method);
        }
        if constructors.is_empty()
            && class.methods.iter().any(|method| {
                method.name == "<init>"
                    && !self
                        .classpath
                        .constructor_visible(&self.lambda_owner, method)
            })
        {
            return Err(
                Diagnostic::new(offset, "Constructor is inaccessible").with_kind(
                    crate::DiagnosticKind::InvalidKotlin,
                    "INACCESSIBLE_CONSTRUCTOR",
                ),
            );
        }
        let selected = self.select_method(&constructors, args, offset);
        let Some(expected) = expected else {
            return selected;
        };
        let selected = match selected {
            Ok(method) => {
                let declaration = constructors
                    .iter()
                    .find(|candidate| {
                        candidate.owner == method.owner
                            && candidate.name == method.name
                            && candidate.descriptor == method.descriptor
                    })
                    .unwrap();
                self.select_method_expected(
                    std::slice::from_ref(declaration),
                    args,
                    &[],
                    Some(expected),
                    offset,
                )?
            }
            Err(error) if error.code == "AMBIGUOUS_OVERLOAD" => return Err(error),
            Err(_) => {
                self.select_method_expected(&constructors, args, &[], Some(expected), offset)?
            }
        };
        if selected
            .kotlin_result
            .as_ref()
            .is_none_or(|result| !self.subtype(result, expected))
        {
            return Err(Diagnostic::new(
                offset,
                "Constructor result is incompatible with the expected type",
            )
            .with_kind(crate::DiagnosticKind::InvalidKotlin, "TYPE_MISMATCH"));
        }
        Ok(selected)
    }
}
