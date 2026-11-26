use super::*;

impl Checker<'_> {
    pub(super) fn infer_constructor_alias(
        &mut self,
        class: &JavaClass,
        declaration: &crate::classpath::LibraryTypeAlias,
        expanded: &Type,
        args: &[Expr],
        expected: Option<&Type>,
        offset: usize,
    ) -> Result<JavaMethod> {
        if class.methods.iter().any(|method| {
            method.name == "<init>"
                && self
                    .classpath
                    .constructor_visible(&self.lambda_owner, method)
                && method
                    .library
                    .as_ref()
                    .is_some_and(|library| !library.type_parameters.is_empty())
        }) {
            return Err(Diagnostic::new(
                offset,
                "Type alias calls to constructor-owned generic parameters are unsupported",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "TYPE_ALIAS_GENERIC_CONSTRUCTOR_UNSUPPORTED",
            ));
        }
        let mut parameters = declaration.type_parameters.clone();
        for (index, parameter) in parameters.iter_mut().enumerate() {
            parameter.name = format!("<typealias-constructor-{index}>");
        }
        let mut renaming = declaration
            .type_parameters
            .iter()
            .zip(&parameters)
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
            .collect::<HashMap<_, _>>();
        for parameter in &mut parameters {
            parameter.bounds = parameter
                .bounds
                .iter()
                .map(|bound| relations::substitute(bound, &renaming))
                .collect();
        }
        for (original, parameter) in declaration.type_parameters.iter().zip(&parameters) {
            renaming.insert(
                original.name.clone(),
                Type::Parameter {
                    name: parameter.name.clone(),
                    upper_bound: Box::new(parameter.bounds.first().cloned().unwrap_or(
                        Type::Nullable(Box::new(Type::Java("java/lang/Object".into()))),
                    )),
                },
            );
        }
        let mut result = relations::substitute(expanded, &renaming);
        while let Type::Nullable(inner) = result {
            result = *inner;
        }
        let Type::Parameterized { name, arguments } = &result else {
            return Err(Diagnostic::new(
                offset,
                "Inferred alias construction requires a generic class expansion",
            )
            .with_kind(
                crate::DiagnosticKind::UnsupportedLanguage,
                "TYPE_ALIAS_CONSTRUCTOR_TARGET_UNSUPPORTED",
            ));
        };
        if name != &class.name
            || arguments.iter().any(|argument| {
                argument.variance != syntax::Variance::Invariant || argument.ty.is_none()
            })
        {
            return Err(Diagnostic::new(
                offset,
                "Projected type alias expansions cannot be constructed",
            )
            .with_kind(
                crate::DiagnosticKind::InvalidKotlin,
                "TYPE_ALIAS_CONSTRUCTOR_PROJECTION",
            ));
        }
        let class_parameters = &class.library.as_ref().unwrap().type_parameters;
        if class_parameters.len() != arguments.len() {
            return Err(Diagnostic::new(
                offset,
                "Wrong number of alias expansion arguments",
            ));
        }
        let substitutions = class_parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, argument)| (parameter.name.clone(), argument.ty.clone().unwrap()))
            .collect::<HashMap<_, _>>();
        let mut template = class.clone();
        for method in &mut template.methods {
            if method.name != "<init>"
                || method.unsupported
                || !self
                    .classpath
                    .constructor_visible(&self.lambda_owner, method)
                || class.kotlin_class && method.kotlin_params.is_none()
            {
                continue;
            }
            let (params, defaults) = Self::method_parameters(method)?;
            method.kotlin_params = Some(
                params
                    .into_iter()
                    .zip(defaults)
                    .map(|((name, ty), default)| {
                        (name, relations::substitute(&ty, &substitutions), default)
                    })
                    .collect(),
            );
        }
        template.library.as_mut().unwrap().type_parameters = parameters;
        self.infer_constructor_template(&template, Some(&result), args, expected, offset)
    }
}
