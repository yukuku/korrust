mod output;

use korrust::{Diagnostic, DiagnosticKind};
use std::{env, fs, path::PathBuf, process::ExitCode};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DiagnosticFormat {
    Human,
    Json,
}

fn render(
    diagnostic: Diagnostic,
    format: DiagnosticFormat,
    path: Option<&str>,
    source: Option<&str>,
) -> String {
    match format {
        DiagnosticFormat::Human => {
            diagnostic.render(path.unwrap_or("<module>"), source.unwrap_or(""))
        }
        DiagnosticFormat::Json => diagnostic.render_json(path, source),
    }
}

fn run() -> std::result::Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--version" {
        println!("korrust {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let format = if args
        .windows(2)
        .any(|pair| pair[0] == "--diagnostic-format" && pair[1] == "json")
    {
        DiagnosticFormat::Json
    } else {
        DiagnosticFormat::Human
    };
    let setup = |code, message| {
        render(
            Diagnostic::new(0, message).with_kind(DiagnosticKind::Setup, code),
            format,
            None,
            None,
        )
    };
    let usage = "Usage: korrust [source.kt ...] -d <output-directory> [-cp <classpath>] [--module-name <name>] [--friend-paths <classpath>] [--diagnostic-format human|json]";
    let mut inputs = Vec::new();
    let mut module_name = None;
    let mut destination = None;
    let mut classpath = Vec::new();
    let mut friend_paths = Vec::new();
    let mut index = 0;
    let mut has_classpath = false;
    let mut has_friend_paths = false;
    let mut has_diagnostic_format = false;
    while index < args.len() {
        if args[index] == "-d"
            || args[index] == "-cp"
            || args[index] == "--classpath"
            || args[index] == "--module-name"
            || args[index] == "--friend-paths"
            || args[index] == "--diagnostic-format"
        {
            let option = &args[index];
            index += 1;
            let value = args
                .get(index)
                .ok_or_else(|| setup("INVALID_ARGUMENTS", usage))?;
            if option == "-d" {
                if destination.is_some() {
                    return Err(setup("INVALID_ARGUMENTS", usage));
                }
                destination = Some(PathBuf::from(value));
            } else if option == "--module-name" {
                if module_name.is_some() {
                    return Err(setup("INVALID_ARGUMENTS", usage));
                }
                let name = value
                    .to_str()
                    .ok_or_else(|| setup("INVALID_MODULE_NAME", "Invalid module name"))?;
                if !output::valid_module_name(name) {
                    return Err(setup(
                        "INVALID_MODULE_NAME",
                        "Invalid module name; expected 1–120 ASCII letters, digits, _, . or -",
                    ));
                }
                module_name = Some(name.to_owned());
            } else if option == "--friend-paths" {
                if has_friend_paths || value.is_empty() {
                    return Err(setup("INVALID_ARGUMENTS", usage));
                }
                has_friend_paths = true;
                friend_paths = env::split_paths(value).collect();
            } else if option == "--diagnostic-format" {
                if has_diagnostic_format || !(value == "human" || value == "json") {
                    return Err(setup(
                        "INVALID_DIAGNOSTIC_FORMAT",
                        "Expected --diagnostic-format human or json",
                    ));
                }
                has_diagnostic_format = true;
            } else {
                if has_classpath || value.is_empty() {
                    return Err(setup("INVALID_ARGUMENTS", usage));
                }
                has_classpath = true;
                classpath = env::split_paths(value).collect();
            }
        } else {
            if args[index].to_string_lossy().starts_with('-') {
                return Err(setup("INVALID_ARGUMENTS", usage));
            }
            inputs.push(PathBuf::from(&args[index]));
        }
        index += 1;
    }
    let destination = destination.ok_or_else(|| setup("INVALID_ARGUMENTS", usage))?;
    if inputs.is_empty() && module_name.is_none() {
        return Err(setup("INVALID_ARGUMENTS", usage));
    }
    let batch = inputs.len() != 1 || module_name.is_some();
    let mut sources = Vec::new();
    let mut display_paths = std::collections::BTreeMap::new();
    for input in &inputs {
        let io_error = |error: std::io::Error| {
            let code = if error.kind() == std::io::ErrorKind::NotFound {
                "SOURCE_FILE_NOT_FOUND"
            } else {
                "SOURCE_READ_FAILED"
            };
            render(
                Diagnostic::new(0, error.to_string())
                    .with_kind(DiagnosticKind::Io, code)
                    .for_source(&input.to_string_lossy()),
                format,
                Some(&input.display().to_string()),
                None,
            )
        };
        let source = fs::read_to_string(input).map_err(io_error)?;
        let identity = fs::canonicalize(input).map_err(io_error)?;
        let identity = identity
            .to_str()
            .ok_or_else(|| {
                setup(
                    "INVALID_SOURCE_PATH",
                    "Non-UTF-8 source paths are unsupported",
                )
            })?
            .to_owned();
        if display_paths.insert(identity.clone(), input).is_some() {
            return Err(render(
                Diagnostic::new(0, "Duplicate source identity")
                    .with_kind(DiagnosticKind::Setup, "DUPLICATE_SOURCE_IDENTITY")
                    .for_source(&identity),
                format,
                Some(&input.display().to_string()),
                None,
            ));
        }
        let file_name = input
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| setup("INVALID_SOURCE_FILENAME", "Invalid source filename"))?
            .to_owned();
        sources.push(korrust::SourceInput {
            identity,
            file_name,
            source,
        });
    }
    let compile_error = |diagnostic: Diagnostic| {
        let source = diagnostic
            .source_identity
            .as_deref()
            .and_then(|identity| sources.iter().find(|source| source.identity == identity));
        match source {
            Some(source) => render(
                diagnostic,
                format,
                Some(&display_paths[&source.identity].display().to_string()),
                Some(&source.source),
            ),
            None => render(diagnostic, format, None, None),
        }
    };
    let name = module_name.as_deref().unwrap_or("main");
    let paths = if batch {
        let options = korrust::CompileOptions {
            module_name: name.into(),
            friend_paths,
        };
        let artifacts = korrust::compile_batch_with_options(&sources, &classpath, &options)
            .map_err(compile_error)?;
        output::write_module_outputs(name, &destination, artifacts)
    } else {
        let source = &sources[0];
        let artifacts = if has_friend_paths {
            let options = korrust::CompileOptions {
                module_name: name.into(),
                friend_paths,
            };
            korrust::compile_batch_with_options(&sources, &classpath, &options).map(|artifacts| {
                artifacts
                    .into_iter()
                    .map(|artifact| artifact.output)
                    .collect()
            })
        } else {
            korrust::compile_outputs_with_classpath(&source.source, &source.file_name, &classpath)
        }
        .map_err(|diagnostic| compile_error(diagnostic.for_source(&source.identity)))?;
        output::write_outputs(&inputs[0], &destination, artifacts)
    }
    .map_err(|message| {
        render(
            Diagnostic::new(0, message).with_kind(DiagnosticKind::Io, "OUTPUT_WRITE_FAILED"),
            format,
            Some(&destination.display().to_string()),
            None,
        )
    })?;
    for path in paths {
        println!("{}", path.display());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}
