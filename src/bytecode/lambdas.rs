use super::*;

pub fn emit_lambdas(module: &Module, facade: &str, file: &str) -> Result<Vec<crate::ClassFile>> {
    module
        .lambdas
        .iter()
        .map(|lambda| emit(lambda, &module.functions, facade, file, &module.lambdas))
        .collect()
}

fn emit(
    lambda: &crate::check::LambdaDecl,
    functions: &[crate::check::Function],
    facade: &str,
    file: &str,
    lambdas: &[crate::check::LambdaDecl],
) -> Result<crate::ClassFile> {
    let function = &lambda.function;
    if function.params.len() > 22 {
        return Err(failure("Lambda arities greater than 22 are unsupported"));
    }
    if function
        .params
        .iter()
        .chain(&lambda.captures)
        .any(|(_, ty)| ty.base() == &Type::Unit)
    {
        return Err(failure(
            "Unit-valued lambda parameters and captures are unsupported",
        ));
    }
    let owner = &lambda.owner;
    let mut pool = Pool::default();
    let this = pool.class(owner)?;
    let parent = pool.class("java/lang/Object")?;
    let interface = lambda.interface.as_ref().map_or_else(
        || format!("kotlin/jvm/functions/Function{}", function.params.len()),
        |method| method.owner.clone(),
    );
    let interface_class = pool.class(&interface)?;
    let mut fields = Vec::new();
    for (name, ty) in &lambda.captures {
        fields.extend(class_field(&mut pool, name, ty, 0x1010)?);
    }
    let constructor_descriptor = format!(
        "({})V",
        lambda
            .captures
            .iter()
            .map(|(_, ty)| ty.descriptor())
            .collect::<String>()
    );
    let capture_slots = 1 + lambda
        .captures
        .iter()
        .map(|(_, ty)| ty.slots())
        .sum::<usize>();
    if capture_slots > 255 {
        return Err(failure("Lambda captures exceed the JVM parameter limit"));
    }
    let mut constructor = SimpleCode::new(3, capture_slots as u16);
    constructor.bytes.push(0x2a);
    constructor.reference(0xb7, pool.method("java/lang/Object", "<init>", "()V")?);
    for ((name, ty), slot) in lambda
        .captures
        .iter()
        .zip(parameter_slots(&lambda.captures, 1))
    {
        constructor
            .bytes
            .extend([0x2a, load_opcode(ty), slot as u8]);
        constructor.reference(0xb5, pool.field(owner, name, &ty.descriptor())?);
    }
    constructor.bytes.push(0xb1);
    let constructor = simple_method(
        &mut pool,
        0x0001,
        "<init>",
        &constructor_descriptor,
        constructor,
        None,
        false,
    )?;
    let mut code = Code {
        pool: &mut pool,
        functions,
        owner: facade,
        bytes: Vec::new(),
        labels: Vec::new(),
        branches: Vec::new(),
        loops: Vec::new(),
        lambdas,
        inline_local_maps: Vec::new(),
        inline_returns: Vec::new(),
        max_stack: 2,
    };
    for ((name, ty), slot) in function
        .params
        .iter()
        .zip(parameter_slots(&function.params, 1))
    {
        if nonnull(ty) {
            code.bytes.extend([load_opcode(ty), slot as u8]);
            let name = code.pool.string(name)?;
            code.reference(0x13, name);
            code.method_reference(
                0xb8,
                "kotlin/jvm/internal/Intrinsics",
                "checkNotNullParameter",
                "(Ljava/lang/Object;Ljava/lang/String;)V",
            )?;
        }
    }
    if inline::contains_nonlocal(function, lambdas) {
        code.bytes.extend([0x01, 0xbf]);
    } else {
        code.block(&function.body)?;
    }
    let stack = code.max_stack as u16;
    let bytes = code.finish()?;
    let implementation = simple_method(
        &mut pool,
        0x0011,
        &function.name,
        &function.descriptor(),
        SimpleCode {
            bytes,
            locals: function.locals,
            max_stack: stack,
        },
        generic_function(function).as_deref(),
        nonnull(&function.result),
    )?;
    let mut methods = vec![constructor, implementation];
    let (bridge_name, bridge_descriptor) = lambda.interface.as_ref().map_or_else(
        || {
            (
                "invoke".to_string(),
                format!(
                    "({})Ljava/lang/Object;",
                    "Ljava/lang/Object;".repeat(function.params.len())
                ),
            )
        },
        |method| (method.name.clone(), method.descriptor.clone()),
    );
    if bridge_name != function.name || bridge_descriptor != function.descriptor() {
        let (params, result) = invocation_types(&bridge_descriptor)?;
        if params.len() != function.params.len() {
            return Err(failure("SAM bridge parameter count mismatch"));
        }
        let locals = 1 + params.iter().map(Type::slots).sum::<usize>();
        let mut code = Code {
            pool: &mut pool,
            functions,
            owner: facade,
            bytes: vec![0x2a],
            labels: Vec::new(),
            branches: Vec::new(),
            loops: Vec::new(),
            lambdas,
            inline_local_maps: Vec::new(),
            inline_returns: Vec::new(),
            max_stack: 3 + function
                .params
                .iter()
                .map(|(_, ty)| ty.slots())
                .sum::<usize>(),
        };
        let mut slot = 1;
        for (erased, (_, actual)) in params.iter().zip(&function.params) {
            code.bytes.extend([load_opcode(erased), slot as u8]);
            code.adapt_value(erased, actual)?;
            slot += erased.slots();
        }
        code.method_reference(0xb6, owner, &function.name, &function.descriptor())?;
        code.adapt_value(&function.result, &result)?;
        code.bytes.push(return_opcode(&result));
        let stack = code.max_stack as u16;
        let bytes = code.finish()?;
        methods.push(simple_method(
            &mut pool,
            0x1041,
            &bridge_name,
            &bridge_descriptor,
            SimpleCode {
                bytes,
                locals: locals as u16,
                max_stack: stack,
            },
            None,
            false,
        )?);
    }
    let source_name = pool.utf8("SourceFile")?;
    let source_file = pool.utf8(file)?;
    let mut bytes = Vec::new();
    u4(&mut bytes, 0xcafebabe);
    u2(&mut bytes, 0);
    u2(&mut bytes, 49);
    pool.write(&mut bytes);
    u2(&mut bytes, 0x1031);
    u2(&mut bytes, this);
    u2(&mut bytes, parent);
    u2(&mut bytes, 1);
    u2(&mut bytes, interface_class);
    u2(&mut bytes, lambda.captures.len() as u16);
    bytes.extend(fields);
    u2(&mut bytes, methods.len() as u16);
    for method in methods {
        bytes.extend(method);
    }
    u2(&mut bytes, 1);
    u2(&mut bytes, source_name);
    u4(&mut bytes, 2);
    u2(&mut bytes, source_file);
    Ok(crate::ClassFile {
        internal_name: owner.clone(),
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(slot: u8, ty: Type) -> Value {
        Value {
            ty,
            kind: ValueKind::Local(slot),
        }
    }

    fn function(
        params: Vec<(String, Type)>,
        result: Type,
        body: Vec<Statement>,
        locals: u16,
    ) -> crate::check::Function {
        crate::check::Function {
            type_params: Vec::new(),
            defaults: Vec::new(),
            name: "invoke".into(),
            private: false,
            is_override: false,
            params,
            result,
            locals,
            body,
        }
    }

    #[test]
    fn compiled_inline_methods_verify_in_classes_objects_and_interfaces() {
        let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let compiler = project.join("references/kotlinc/bin/kotlinc");
        let stdlib = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
        let root = std::env::temp_dir().join(format!(
            "korrust-inline-method-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let library = root.join("library");
        let official = root.join("official");
        let candidate = root.join("candidate");
        let driver = root.join("driver");
        fn success(command: &mut std::process::Command) -> Vec<u8> {
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        }
        let library_source = root.join("InlineLibrary.kt");
        std::fs::write(&library_source, "package inlinetest\ninline fun gate(block: () -> Int): Int = block() + 9\ninline fun twice(block: () -> Int): Int = block() + block()\ninline fun transform(value: Int, block: (Int) -> Int): Int = block(value) + 7\n").unwrap();
        success(
            std::process::Command::new(&compiler)
                .arg(&library_source)
                .arg("-d")
                .arg(&library),
        );
        let inputs = [
            (
                "InlineMethods.kt",
                "import inlinetest.*\nclass InlineMethods { fun capture(base: Int): Int { val offset = base + 2; return transform(3) { value -> value + offset } }; fun nonlocal(): Long { gate { return 1234567890123L }; return -1L }; fun mutable(): Int { var count = 0; val result = twice { count++; count }; return result * 10 + count } }",
            ),
            (
                "InlineObject.kt",
                "import inlinetest.*\nobject InlineObject { fun run(value: Int): Int = transform(value) { it + 1 } }",
            ),
            (
                "InlineInterface.kt",
                "import inlinetest.*\ninterface InlineInterface { fun run(value: Int): Int = transform(value) { it + 1 } }",
            ),
        ];
        let cp = std::env::join_paths([library.as_path(), stdlib.as_path()]).unwrap();
        let mut official_command = std::process::Command::new(&compiler);
        for (name, source) in inputs {
            let path = root.join(name);
            std::fs::write(&path, source).unwrap();
            official_command.arg(path);
            for file in crate::compile_outputs_with_classpath(
                source,
                name,
                &[library.clone(), stdlib.clone()],
            )
            .unwrap()
            {
                let path = candidate.join(file.relative_path);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, file.bytes).unwrap();
            }
        }
        success(
            official_command
                .arg("-cp")
                .arg(&cp)
                .arg("-d")
                .arg(&official),
        );
        let caller = root.join("InlineMethodCheck.java");
        std::fs::write(&caller, "class InlineMethodCheck { public static void main(String[] args) { InlineMethods value = new InlineMethods(); if (value.capture(5) != 17 || value.nonlocal() != 1234567890123L || value.mutable() != 32 || InlineObject.INSTANCE.run(4) != 12 || new InlineInterface() {}.run(4) != 12) throw new AssertionError(); System.out.println(\"OK\"); } }").unwrap();
        let cp = std::env::join_paths([official.as_path(), library.as_path(), stdlib.as_path()])
            .unwrap();
        success(
            std::process::Command::new("javac")
                .arg("-cp")
                .arg(cp)
                .arg("-d")
                .arg(&driver)
                .arg(&caller),
        );
        for provider in [&official, &candidate] {
            let cp = std::env::join_paths([
                provider.as_path(),
                driver.as_path(),
                library.as_path(),
                stdlib.as_path(),
            ])
            .unwrap();
            let stdout = success(
                std::process::Command::new("java")
                    .arg("-Xverify:all")
                    .arg("-cp")
                    .arg(cp)
                    .arg("InlineMethodCheck"),
            );
            assert_eq!(stdout, b"OK\n");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn nullable_type_arguments_survive_official_kotlin_consumers() {
        let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let compiler = project.join("references/kotlinc/bin/kotlinc");
        let stdlib = project.join("references/kotlinc/lib/kotlin-stdlib.jar");
        let root = std::env::temp_dir().join(format!(
            "korrust-generic-null-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let candidate = root.join("candidate");
        let official = root.join("official");
        let consumer = root.join("consumer");
        std::fs::create_dir_all(&root).unwrap();
        let source = "fun <T> id(value: T): T = value\nfun <T: Any> strict(value: T): T = value\nfun probe(): String? = id<String?>(null)\n";
        let provider = root.join("GenericNull.kt");
        std::fs::write(&provider, source).unwrap();
        for file in crate::compile_outputs_with_classpath(source, "GenericNull.kt", &[]).unwrap() {
            let path = candidate.join(file.relative_path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, file.bytes).unwrap();
        }
        fn success(command: &mut std::process::Command) -> Vec<u8> {
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            output.stdout
        }
        success(
            std::process::Command::new(&compiler)
                .arg(&provider)
                .arg("-d")
                .arg(&official),
        );
        let caller = root.join("GenericNullConsumer.kt");
        std::fs::write(&caller, "fun main() { check(probe() == null); check(id<String?>(null) == null); check(id(\"OK\") == \"OK\"); check(strict(\"OK\") == \"OK\"); println(\"OK\") }").unwrap();
        success(
            std::process::Command::new(&compiler)
                .arg(&caller)
                .arg("-cp")
                .arg(&candidate)
                .arg("-d")
                .arg(&consumer),
        );
        for provider in [&official, &candidate] {
            let cp =
                std::env::join_paths([provider.as_path(), consumer.as_path(), stdlib.as_path()])
                    .unwrap();
            let stdout = success(
                std::process::Command::new("java")
                    .arg("-Xverify:all")
                    .arg("-cp")
                    .arg(cp)
                    .arg("GenericNullConsumerKt"),
            );
            assert_eq!(stdout, b"OK\n");
        }
        let cp = std::env::join_paths([candidate.as_path(), stdlib.as_path()]).unwrap();
        let output = success(
            std::process::Command::new("javap")
                .arg("-v")
                .arg("-cp")
                .arg(cp)
                .arg("GenericNullKt"),
        );
        let output = String::from_utf8(output).unwrap();
        let identity = output
            .split(" T id(T);")
            .nth(1)
            .unwrap()
            .split("public static")
            .next()
            .unwrap();
        assert!(!identity.contains("RuntimeInvisibleAnnotations"));
        assert!(!identity.contains("RuntimeInvisibleParameterAnnotations"));
        let strict = output
            .split(" T strict(T);")
            .nth(1)
            .unwrap()
            .split("public static")
            .next()
            .unwrap();
        assert!(strict.contains("checkNotNullParameter"));
        assert!(strict.contains("org.jetbrains.annotations.NotNull"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generated_bridges_execute_with_jvm_verification() {
        let long = crate::check::LambdaDecl {
            owner: "CaptureLong".into(),
            captures: vec![("base".into(), Type::Long)],
            function: function(
                vec![("value".into(), Type::Long)],
                Type::Long,
                vec![Statement::Return(Value {
                    ty: Type::Long,
                    kind: ValueKind::Binary(
                        "+".into(),
                        Box::new(local(1, Type::Long)),
                        Box::new(Value {
                            ty: Type::Long,
                            kind: ValueKind::JavaField {
                                owner: "CaptureLong".into(),
                                name: "base".into(),
                                receiver: Some(Box::new(local(
                                    0,
                                    Type::Java("CaptureLong".into()),
                                ))),
                            },
                        }),
                    ),
                })],
                3,
            ),
            interface: None,
        };
        let unit = crate::check::LambdaDecl {
            owner: "ReturnUnit".into(),
            captures: Vec::new(),
            function: function(
                Vec::new(),
                Type::Unit,
                vec![Statement::Return(Value {
                    ty: Type::Unit,
                    kind: ValueKind::Unit,
                })],
                1,
            ),
            interface: None,
        };
        let sam = crate::check::LambdaDecl {
            owner: "StringSam".into(),
            captures: Vec::new(),
            function: function(
                vec![("value".into(), Type::String)],
                Type::String,
                vec![Statement::Return(local(1, Type::String))],
                2,
            ),
            interface: Some(crate::classpath::JavaMethod {
                library: None,
                owner: "java/util/function/UnaryOperator".into(),
                name: "apply".into(),
                descriptor: "(Ljava/lang/Object;)Ljava/lang/Object;".into(),
                access: 0x0401,
                unsupported: false,
                source_property: None,
                kotlin_params: None,
                kotlin_result: None,
                kotlin_internal: false,
            }),
        };
        let root = std::env::temp_dir().join(format!(
            "korrust-lambda-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        for lambda in [long, unit, sam] {
            let class = emit(&lambda, &[], "Unused", "Lambda.kt", &[]).unwrap();
            std::fs::write(
                root.join(format!("{}.class", class.internal_name)),
                class.bytes,
            )
            .unwrap();
        }
        let source = root.join("Check.java");
        std::fs::write(&source, r#"class Check {
            public static void main(String[] args) {
                kotlin.jvm.functions.Function1<Long, Long> longFn = new CaptureLong(7L);
                if (longFn.invoke(3L) != 10L) throw new AssertionError("long capture or bridge");
                kotlin.jvm.functions.Function0<kotlin.Unit> unitFn = new ReturnUnit();
                if (unitFn.invoke() != kotlin.Unit.INSTANCE) throw new AssertionError("Unit bridge");
                java.util.function.UnaryOperator<String> sam = new StringSam();
                if (!sam.apply("OK").equals("OK")) throw new AssertionError("SAM bridge");
            }
        }"#).unwrap();
        let stdlib = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("references/kotlinc/lib/kotlin-stdlib.jar");
        let classpath = std::env::join_paths([root.as_path(), stdlib.as_path()]).unwrap();
        let output = std::process::Command::new("java")
            .arg("-Xverify:all")
            .arg("-cp")
            .arg(classpath)
            .arg(&source)
            .output()
            .expect("JDK must be installed");
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
