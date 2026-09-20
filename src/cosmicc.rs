use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

use saltwater_sia::{compile, Opt, TARGET};

const HELP: &str = "\
cosmicc - C to SIA32 compiler for Cosmic OS

Usage: cosmicc [OPTIONS] [FILE]

Compile C source through the Cosmic C frontend and Cranelift's native SIA32
backend. The output is a COSMIC-SIA code bundle, not a host executable.

Options:
  -c, --no-link        Accepted for C compiler compatibility; linking is not run.
  -o, --output PATH    Write the SIA bundle to PATH (default: a.sia).\n  -I, --include DIR    Add DIR to the header search path; may be repeated.
  -D, --define DEF     Define object macro NAME or NAME=VALUE; may be repeated.
      --target TARGET  Require `sia32-unknown-none` (the only supported target).
  -h, --help           Show this help.
  -V, --version        Show the compiler version.

FILE defaults to standard input. Use - explicitly for standard input.

Current target constraints: integer SSA code and local variables are supported;
floating-point C is rejected deliberately before backend lowering.";

fn main() {
    let mut input = None;
    let mut output = PathBuf::from("a.sia");
    let mut include_paths = Vec::new();
    let mut definitions = Vec::new();
    let mut args = env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            "-V" | "--version" => {
                println!("cosmicc {} ({TARGET})", env!("CARGO_PKG_VERSION"));
                return;
            }
            "-c" | "--no-link" => {}
            "-o" | "--output" => match args.next() {
                Some(path) => output = path.into(),
                None => usage_error("missing path after --output"),
            },
            "-I" | "--include" => match args.next() {
                Some(path) => include_paths.push(PathBuf::from(path)),
                None => usage_error("missing path after -I/--include"),
            },
            "-D" | "--define" => match args.next() {
                Some(definition) => definitions.push(definition),
                None => usage_error("missing macro after -D/--define"),
            },
            "--target" => match args.next() {
                Some(target) if target == TARGET => {}
                Some(target) => usage_error(&format!(
                    "unsupported target `{target}`; expected `{TARGET}`"
                )),
                None => usage_error("missing target after --target"),
            },
            _ if argument.starts_with("-I") && argument.len() > 2 => {
                include_paths.push(PathBuf::from(&argument[2..]));
            }
            _ if argument.starts_with("-D") && argument.len() > 2 => {
                definitions.push(argument[2..].to_owned());
            }
            _ if argument.starts_with('-') && argument != "-" => {
                usage_error(&format!("unknown option `{argument}`"));
            }
            _ => {
                if input.replace(PathBuf::from(argument)).is_some() {
                    usage_error("only one C input file is accepted");
                }
            }
        }
    }

    let source_path = input.unwrap_or_else(|| PathBuf::from("-"));
    let source = if source_path.as_os_str() == "-" {
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .unwrap_or_else(|error| fatal(&format!("failed to read standard input: {error}")));
        source
    } else {
        fs::read_to_string(&source_path).unwrap_or_else(|error| {
            fatal(&format!(
                "failed to read {}: {error}",
                source_path.display()
            ))
        })
    };

    let mut opt = Opt::default();
    opt.filename = source_path;
    opt.search_path = include_paths;
    for definition in definitions {
        let mut parts = definition.splitn(2, '=');
        let name = parts.next().expect("splitn always yields the macro name");
        let value = parts.next().unwrap_or("1");
        let value = value
            .try_into()
            .unwrap_or_else(|error: saltwater_sia::LexError| fatal(&error.to_string()));
        opt.definitions.insert(name.into(), value);
    }
    let artifact = compile(&source, opt).unwrap_or_else(|error| fatal(&error.to_string()));
    let bytes = artifact
        .to_bytes()
        .unwrap_or_else(|error| fatal(&error.to_string()));
    fs::write(&output, bytes)
        .unwrap_or_else(|error| fatal(&format!("failed to write {}: {error}", output.display())));
}

fn usage_error(message: &str) -> ! {
    eprintln!("cosmicc: {message}\nTry `cosmicc --help`.");
    process::exit(1);
}

fn fatal(message: &str) -> ! {
    eprintln!("cosmicc: error: {message}");
    process::exit(2);
}
