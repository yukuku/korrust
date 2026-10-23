use std::{env, fs, path::PathBuf, process::ExitCode};

fn run() -> std::result::Result<(), String> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--version" {
        println!("korrust {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let usage = "Usage: korrust <source.kt> -d <output-directory> [-cp <classpath>]";
    let mut input = None;
    let mut destination = None;
    let mut classpath = Vec::new();
    let mut index = 0;
    let mut has_classpath = false;
    while index < args.len() {
        if args[index] == "-d" || args[index] == "-cp" || args[index] == "--classpath" {
            let option = &args[index];
            index += 1;
            let value = args.get(index).ok_or(usage)?;
            if option == "-d" {
                if destination.is_some() {
                    return Err(usage.into());
                }
                destination = Some(PathBuf::from(value));
            } else {
                if has_classpath || value.is_empty() {
                    return Err(usage.into());
                }
                has_classpath = true;
                classpath = env::split_paths(value).collect();
            }
        } else {
            if input.is_some() || args[index].to_string_lossy().starts_with('-') {
                return Err(usage.into());
            }
            input = Some(PathBuf::from(&args[index]));
        }
        index += 1;
    }
    let input = input.ok_or(usage)?;
    let destination = destination.ok_or(usage)?;
    let source = fs::read_to_string(&input).map_err(|e| format!("{}: {e}", input.display()))?;
    let name = input
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid source filename")?;
    let class = korrust::compile_with_classpath(&source, name, &classpath)
        .map_err(|e| e.render(&input.display().to_string(), &source))?;
    let output = destination.join(format!("{}.class", class.internal_name));
    fs::create_dir_all(output.parent().ok_or("Invalid output directory")?)
        .map_err(|e| e.to_string())?;
    fs::write(&output, class.bytes).map_err(|e| format!("{}: {e}", output.display()))?;
    println!("{}", output.display());
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
