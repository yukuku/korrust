use super::*;

fn upper_bounds(ty: &Type, depth: usize) -> Type {
    if depth > 64 {
        return ty.clone();
    }
    match ty {
        Type::Parameter { upper_bound, .. } => upper_bounds(upper_bound, depth + 1),
        Type::Nullable(inner) => Type::Nullable(Box::new(upper_bounds(inner, depth + 1))),
        Type::Platform(inner) => Type::Platform(Box::new(upper_bounds(inner, depth + 1))),
        Type::List(inner) => Type::List(Box::new(upper_bounds(inner, depth + 1))),
        Type::MutableList(inner) => Type::MutableList(Box::new(upper_bounds(inner, depth + 1))),
        Type::ThreadLocal(inner) => Type::ThreadLocal(Box::new(upper_bounds(inner, depth + 1))),
        Type::Parameterized { name, arguments } => Type::Parameterized {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|argument| syntax::TypeArgument {
                    variance: argument.variance,
                    ty: argument.ty.as_ref().map(|ty| upper_bounds(ty, depth + 1)),
                })
                .collect(),
        },
        Type::Function { params, result } => Type::Function {
            params: params
                .iter()
                .map(|ty| upper_bounds(ty, depth + 1))
                .collect(),
            result: Box::new(upper_bounds(result, depth + 1)),
        },
        _ => ty.clone(),
    }
}

impl Checker<'_> {
    pub(super) fn most_specific(
        &self,
        candidates: &[Vec<(String, Type)>],
        defaults: &[Vec<bool>],
        named: &[bool],
        args: &[Expr],
        generic: &[bool],
        offset: usize,
    ) -> Result<usize> {
        let count = candidates.len();
        if count == 0 || defaults.len() != count || named.len() != count || generic.len() != count {
            return Err(Diagnostic::new(
                offset,
                "Overload candidate information is inconsistent",
            ));
        }
        let mapping = candidates
            .iter()
            .enumerate()
            .map(|(index, params)| {
                Self::argument_mapping(params, &defaults[index], args, named[index], offset)
            })
            .collect::<Result<Vec<_>>>()?;
        let parameters = candidates
            .iter()
            .map(|params| {
                params
                    .iter()
                    .map(|(_, ty)| upper_bounds(ty, 0))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let omitted = candidates
            .iter()
            .map(|params| params.len().saturating_sub(args.len()))
            .collect::<Vec<_>>();
        let forwards = |first: usize, second: usize| {
            mapping[first].iter().zip(&mapping[second]).all(
                |(first_parameter, second_parameter)| {
                    let first = &parameters[first][*first_parameter];
                    let second = &parameters[second][*second_parameter];
                    if matches!(first, Type::Int | Type::Byte | Type::Long)
                        && matches!(second, Type::Int | Type::Byte | Type::Long)
                    {
                        first == second || first == &Type::Int
                    } else {
                        self.subtype(first, second)
                    }
                },
            )
        };
        let preferred = |first: usize, second: usize| {
            let forward = forwards(first, second);
            let backward = forwards(second, first);
            match (forward, backward) {
                (true, false) => true,
                (false, true) => false,
                (false, false) => !generic[first] && generic[second],
                (true, true) => {
                    if generic[first] != generic[second] {
                        !generic[first]
                    } else {
                        omitted[first] < omitted[second]
                    }
                }
            }
        };
        let survivors = (0..count)
            .filter(|candidate| {
                (0..count).all(|other| other == *candidate || !preferred(other, *candidate))
            })
            .collect::<Vec<_>>();
        match survivors.as_slice() {
            [only] => Ok(*only),
            _ => Err(Diagnostic::new(
                offset,
                "Ambiguous overload: no unique most specific declaration",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected(source: &str) -> usize {
        fn call(value: &Value) -> Option<usize> {
            match &value.kind {
                ValueKind::Call(id, _) | ValueKind::DefaultCall(id, _, _) => Some(*id),
                ValueKind::Let { body, .. }
                | ValueKind::Box(body)
                | ValueKind::Unbox(body)
                | ValueKind::NotNull(body, _) => call(body),
                _ => None,
            }
        }
        let module = syntax::parse(source).unwrap();
        let checked = super::super::check(&module, &Classpath::new(&[]).unwrap()).unwrap();
        let Statement::Return(value) = &checked.functions.last().unwrap().body[0] else {
            panic!("missing call return");
        };
        call(value).expect("return did not contain a source call")
    }

    #[test]
    fn narrower_nullable_parameter_precedes_default_argument_tiebreak() {
        assert_eq!(
            selected(
                "fun f(value: String?, unused: Int = 1): Int = 11\nfun f(value: Any?): Int = 22\nfun use(): Int = f(null)"
            ),
            0
        );
    }

    #[test]
    fn declared_generic_bounds_precede_specialized_argument_types() {
        assert_eq!(
            selected(
                "fun <T> f(value: T): Int = 11\nfun f(value: CharSequence): Int = 22\nfun use(): Int = f(\"text\")"
            ),
            1
        );
    }

    #[test]
    fn crossed_parameter_specificity_is_ambiguous() {
        let module=syntax::parse("fun f(a: String, b: Any, c: Any): Int = 11\nfun f(a: Any, b: String, c: String): Int = 22\nfun use(): Int = f(\"a\",\"b\",\"c\")").unwrap();
        let error = match super::super::check(&module, &Classpath::new(&[]).unwrap()) {
            Err(error) => error,
            Ok(_) => panic!("incomparable overloads were accepted"),
        };
        assert!(error.message.contains("Ambiguous"), "{}", error.message);
    }
}
